//! Retained application pixels. The trusted scheduler serializes these calls
//! and retains the NativeMemory arena until every surface is destroyed.
//! No framebuffer or kernel pixel pointer is exposed through the C ABI.
use super::{InfinityRuntime, UiOperationError, capability::CapabilityType,
    execution::SecurityIdentity, native_memory::{NativeMemory, FrameAllocation, PageTable, copy_from_user}};
use crate::ui::{geometry::{Rect, Size}, surface::{PixelFormat, SurfaceSecurityClass, MAX_SURFACES},
    window::{ContextId, SurfaceId}, compositor::SurfaceFrame};

const MAX_BYTES: usize = 16 * 1024 * 1024;
const BUDGET: usize = 64 * 1024 * 1024;
const MAX_UPLOAD: usize = 64 * 1024;

#[derive(Clone, Copy)]
struct Pixels {
    id: SurfaceId,
    owner: ContextId,
    allocation: FrameAllocation,
    bytes: usize,
    plane: usize,
    dirty: Option<(usize, usize)>,
    damage: Option<Rect>,
}

pub struct NativeSurfaces {
    entries: [Option<Pixels>; MAX_SURFACES],
    reserved: usize,
}

impl NativeSurfaces {
    // ------------------------=
    // FUNC: dispatch
    // DESC: Routes native GUI traps through the existing window, event and retained-surface services with trusted launch identity.
    // ------------------=
    pub fn dispatch<T: PageTable>(&mut self, runtime: &mut InfinityRuntime, memory: &mut NativeMemory,
        tables: &mut T, caller: SecurityIdentity, owner: ContextId, operation: u64,
        args: [u64; 6], work_area: Rect, display_area: Rect, now: u64) -> i64 {
        match operation {
            0x1100..=0x1108 => runtime.native_window_syscall(caller, owner, tables, operation,
                args, work_area, display_area, now),
            0x1110..=0x1112 => runtime.native_event_syscall(caller, tables, operation, args, now),
            0x1120..=0x1124 => self.syscall(runtime, memory, tables, caller, owner, operation, args, now),
            _ => -6,
        }
    }

    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty bounded store; pixel allocations come only from the native frame pool.
    // ------------------=
    pub const fn new() -> Self { Self { entries: [None; MAX_SURFACES], reserved: 0 } }

    // ------------------------=
    // FUNC: slot
    // DESC: Resolves an exact owned pixel allocation independently of capability validation.
    // ------------------=
    fn slot(&self, id: SurfaceId, owner: ContextId) -> Result<usize, i64> {
        let index = self.entries.iter().position(|p| p.is_some_and(|p| p.id == id)).ok_or(-2i64)?;
        if self.entries[index].unwrap().owner != owner { return Err(-1); }
        Ok(index)
    }

    // ------------------------=
    // FUNC: allocate
    // DESC: Reserves zeroed displayed and staging planes, including both planes in the physical memory budget.
    // ------------------=
    fn allocate(&self, memory: &mut NativeMemory, owner: ContextId, width: u32, height: u32)
        -> Result<(FrameAllocation, usize, usize), i64> {
        let bytes = (width as usize).checked_mul(height as usize).and_then(|v| v.checked_mul(4)).ok_or(-3i64)?;
        if width == 0 || height == 0 || width > 8192 || height > 8192 || bytes > MAX_BYTES { return Err(-3); }
        let plane = (bytes + 4095) & !4095;
        if plane * 2 > BUDGET.saturating_sub(self.reserved) { return Err(-4); }
        let allocation = memory.allocate((plane * 2 / 4096) as u32, owner.0).map_err(|_| -4i64)?;
        Ok((allocation, bytes, plane))
    }

    // ------------------------=
    // FUNC: syscall
    // DESC: Handles bounded retained-surface calls using dispatcher identity, live capabilities and checked user copies.
    // ------------------=
    pub fn syscall<T: PageTable>(&mut self, runtime: &mut InfinityRuntime, memory: &mut NativeMemory,
        tables: &T, caller: SecurityIdentity, owner: ContextId, operation: u64, args: [u64; 6], now: u64) -> i64 {
        self.execute(runtime, memory, tables, caller, owner, operation, args, now).unwrap_or_else(|error| error)
    }

    // ------------------------=
    // FUNC: execute
    // DESC: Applies surface mutations atomically; failed uploads and allocations preserve displayed pixels and metadata.
    // ------------------=
    fn execute<T: PageTable>(&mut self, runtime: &mut InfinityRuntime, memory: &mut NativeMemory,
        tables: &T, caller: SecurityIdentity, owner: ContextId, operation: u64, args: [u64; 6], now: u64)
        -> Result<i64, i64> {
        if !(0x1120..=0x1124).contains(&operation) { return Err(-6); }
        if operation == 0x1120 {
            let width = u32::try_from(args[1]).map_err(|_| -3i64)?;
            let height = u32::try_from(args[2]).map_err(|_| -3i64)?;
            let format = match args[3] { 0 => PixelFormat::Xrgb8888, 1 => PixelFormat::Argb8888, _ => return Err(-3) };
            authorize(runtime, caller, args[0], CapabilityType::SurfaceCreate, super::SERVICE_WINDOW_SERVER as u64, now)?;
            let index = self.entries.iter().position(Option::is_none).ok_or(-4i64)?;
            let (allocation, bytes, plane) = self.allocate(memory, owner, width, height)?;
            let id = match runtime.create_surface(caller, owner, args[0], Size { width, height }, format, SurfaceSecurityClass::Application, now) {
                Ok(id) => id,
                Err(error) => { let _ = memory.release(allocation); return Err(abi_error(error)); }
            };
            self.entries[index] = Some(Pixels { id, owner, allocation, bytes, plane, dirty: None, damage: None });
            self.reserved += plane * 2;
            return Ok(id.0 as i64);
        }
        let id = SurfaceId(u32::try_from(args[1]).map_err(|_| -3i64)?);
        let kind = match operation { 0x1123 => CapabilityType::SurfaceResize,
            0x1124 => CapabilityType::SurfaceDestroy, _ => CapabilityType::SurfacePublish };
        authorize(runtime, caller, args[0], kind, id.0 as u64, now)?;
        let index = self.slot(id, owner)?;
        let mut pixels = self.entries[index].unwrap();
        let descriptor = runtime.ui.surfaces.inspect(id).copied().ok_or(-2i64)?;
        if descriptor.owner != owner || descriptor.byte_length != pixels.bytes as u64 { return Err(-3); }
        match operation {
            0x1121 => {
                let offset = usize::try_from(args[3]).map_err(|_| -3i64)?;
                let length = usize::try_from(args[4]).map_err(|_| -3i64)?;
                let end = offset.checked_add(length).ok_or(-3i64)?;
                if length == 0 || length > MAX_UPLOAD || (offset | length) & 3 != 0 || end > pixels.bytes { return Err(-3); }
                let destination = unsafe { core::slice::from_raw_parts_mut(
                    (pixels.allocation.physical_address as usize + pixels.plane + offset) as *mut u8, length) };
                copy_from_user(tables, args[2], destination).map_err(|_| -5i64)?;
                pixels.dirty = Some(match pixels.dirty { Some((first, last)) => (first.min(offset), last.max(end)), None => (offset, end) });
            }
            0x1122 => {
                if args[2] <= descriptor.content_generation { return Err(-3); }
                if let Some((first, end)) = pixels.dirty {
                    unsafe { core::ptr::copy_nonoverlapping(
                        (pixels.allocation.physical_address as usize + pixels.plane + first) as *const u8,
                        (pixels.allocation.physical_address as usize + first) as *mut u8, end - first); }
                    let stride = descriptor.stride_pixels as usize * 4;
                    let top = first / stride;
                    let bottom = (end - 1) / stride;
                    let damage = if top == bottom {
                        Rect { x: ((first % stride) / 4) as i32, y: top as i32, width: ((end - first) / 4) as u32, height: 1 }
                    } else { Rect { x: 0, y: top as i32, width: descriptor.size.width, height: (bottom - top + 1) as u32 } };
                    pixels.damage = Some(pixels.damage.map_or(damage, |old| old.union(damage)));
                    damage_window(runtime, id, damage);
                }
                runtime.publish_surface(caller, owner, args[0], id, args[2], now).map_err(abi_error)?;
                pixels.dirty = None;
            }
            0x1123 => {
                let width = u32::try_from(args[2]).map_err(|_| -3i64)?;
                let height = u32::try_from(args[3]).map_err(|_| -3i64)?;
                let (allocation, bytes, plane) = self.allocate(memory, owner, width, height)?;
                if let Err(error) = runtime.resize_surface(caller, owner, args[0], id, Size { width, height }, now) {
                    let _ = memory.release(allocation); return Err(abi_error(error));
                }
                // Preserve committed overlap, never unpublished staging data.
                for row in 0..height.min(descriptor.size.height) as usize {
                    let count = width.min(descriptor.size.width) as usize * 4;
                    unsafe {
                        let source = (pixels.allocation.physical_address as usize + row * descriptor.stride_pixels as usize * 4) as *const u8;
                        let destination = (allocation.physical_address as usize + row * width as usize * 4) as *mut u8;
                        core::ptr::copy_nonoverlapping(source, destination, count);
                        core::ptr::copy_nonoverlapping(source, destination.add(plane), count);
                    }
                }
                memory.release(pixels.allocation).map_err(|_| -3i64)?;
                self.reserved = self.reserved - pixels.plane * 2 + plane * 2;
                pixels.allocation = allocation; pixels.bytes = bytes; pixels.plane = plane;
                pixels.dirty = None;
                pixels.damage = Some(Rect { x: 0, y: 0, width, height });
                damage_window(runtime, id, pixels.damage.unwrap());
            }
            0x1124 => {
                runtime.destroy_surface(caller, owner, args[0], id, now).map_err(abi_error)?;
                memory.release(pixels.allocation).map_err(|_| -3i64)?;
                self.reserved -= pixels.plane * 2;
                self.entries[index] = None;
                return Ok(0);
            }
            _ => return Err(-6),
        }
        self.entries[index] = Some(pixels);
        Ok(0)
    }

    // ------------------------=
    // FUNC: frame
    // DESC: Borrows committed pixels for the existing compositor without allocating, scaling or rerendering application content.
    // ------------------=
    pub fn frame<'a>(&'a self, runtime: &InfinityRuntime, id: SurfaceId) -> Option<SurfaceFrame<'a>> {
        let pixels = self.entries.iter().flatten().find(|p| p.id == id)?;
        let descriptor = *runtime.ui.surfaces.inspect(id)?;
        if descriptor.owner != pixels.owner || descriptor.byte_length != pixels.bytes as u64 { return None; }
        let window = runtime.ui.windows.inspect(descriptor.associated_window?)?;
        Some(SurfaceFrame { descriptor,
            pixels: unsafe { core::slice::from_raw_parts(pixels.allocation.physical_address as *const u32, pixels.bytes / 4) },
            bounds: window.bounds, opacity: descriptor.opacity, z_class: window.z_class,
            visible: descriptor.visible && window.visible && window.state != crate::ui::window::WindowState::Minimized })
    }

    // ------------------------=
    // FUNC: take_damage
    // DESC: Drains only committed surface-local damage for translation into the existing compositor damage tracker.
    // ------------------=
    pub fn take_damage(&mut self, id: SurfaceId) -> Option<Rect> {
        self.entries.iter_mut().flatten().find(|p| p.id == id)?.damage.take()
    }

    // ------------------------=
    // FUNC: context_failed
    // DESC: Releases owned windows, surface metadata and pixel frames after the trusted scheduler has stopped the process.
    // ------------------=
    pub fn context_failed(&mut self, runtime: &mut InfinityRuntime, memory: &mut NativeMemory, owner: ContextId) -> usize {
        for index in 0..runtime.ui.windows.count() {
            if let Some(window) = runtime.ui.windows.ordered_window(index) {
                if window.owner == owner && window.visible { runtime.ui.damage.add(window.bounds); }
            }
        }
        runtime.ui.context_failed(owner);
        let mut released = 0;
        for slot in &mut self.entries {
            if let Some(pixels) = slot.filter(|p| p.owner == owner) {
                if memory.release(pixels.allocation).is_ok() {
                    self.reserved -= pixels.plane * 2; *slot = None; released += 1;
                }
            }
        }
        released
    }
}

// ------------------------=
// FUNC: damage_window
// DESC: Translates committed pixels into bounded compositor damage; transformed windows invalidate only their own bounds.
// ------------------=
fn damage_window(runtime: &mut InfinityRuntime, id: SurfaceId, local: Rect) {
    let Some(surface) = runtime.ui.surfaces.inspect(id) else { return; };
    let Some(window) = surface.associated_window.and_then(|id| runtime.ui.windows.inspect(id)) else { return; };
    if !window.visible || !surface.visible { return; }
    let damage = if window.bounds.width == surface.size.width && window.bounds.height == surface.size.height {
        Rect { x: window.bounds.x.saturating_add(local.x), y: window.bounds.y.saturating_add(local.y),
            width: local.width, height: local.height }.intersection(window.bounds)
    } else { window.bounds };
    runtime.ui.damage.add(damage);
}

// ------------------------=
// FUNC: authorize
// DESC: Revalidates live surface authority before touching private application pixel storage.
// ------------------=
fn authorize(runtime: &InfinityRuntime, caller: SecurityIdentity, capability: u64, kind: CapabilityType, scope: u64, now: u64) -> Result<(), i64> {
    runtime.capabilities.validate(capability, caller, kind, scope, 1, 0, now).map_err(|_| -1)
}

// ------------------------=
// FUNC: abi_error
// DESC: Maps typed UI failures into stable C surface errors.
// ------------------=
fn abi_error(error: UiOperationError) -> i64 {
    use crate::ui::surface::SurfaceError;
    match error {
        UiOperationError::AccessDenied | UiOperationError::Surface(SurfaceError::AccessDenied) => -1,
        UiOperationError::Surface(SurfaceError::Unknown) => -2,
        UiOperationError::Surface(SurfaceError::Full | SurfaceError::BudgetExceeded) => -4,
        _ => -3,
    }
}
