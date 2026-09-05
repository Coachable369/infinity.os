//! Bounded retained pixel-surface metadata and execution-context ownership.

use super::geometry::{Rect, Size};
use super::window::{ContextId, SurfaceId, WindowId};

pub const MAX_SURFACES: usize = 32;
pub const DEFAULT_SURFACE_BUDGET_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_SURFACE_EDGE: u32 = 8192;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PixelFormat {
    Xrgb8888,
    Argb8888,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum SurfaceSecurityClass {
    Application,
    System,
    Trusted,
    Cursor,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SurfaceDescriptor {
    pub id: SurfaceId,
    pub owner: ContextId,
    pub size: Size,
    pub stride_pixels: u32,
    pub format: PixelFormat,
    pub security_class: SurfaceSecurityClass,
    pub byte_length: u64,
    pub content_generation: u64,
    pub visible: bool,
    pub opacity: u8,
    pub pending_damage: Option<Rect>,
    pub associated_window: Option<WindowId>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SurfaceError {
    Full,
    Unknown,
    AccessDenied,
    InvalidGeometry,
    BudgetExceeded,
    StaleGeneration,
    InUse,
}

pub struct SurfaceRegistry {
    entries: [Option<SurfaceDescriptor>; MAX_SURFACES],
    next_id: u32,
    byte_budget: u64,
    bytes_reserved: u64,
}

impl SurfaceRegistry {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty surface registry with a strict total memory reservation budget.
    // ------------------=
    pub const fn new(byte_budget: u64) -> Self {
        Self {
            entries: [None; MAX_SURFACES],
            next_id: 1,
            byte_budget,
            bytes_reserved: 0,
        }
    }

    // ------------------------=
    // FUNC: create
    // DESC: Reserves a retained surface for one owner while preventing untrusted privileged surfaces.
    // ------------------=
    pub fn create(
        &mut self,
        owner: ContextId,
        size: Size,
        format: PixelFormat,
        security_class: SurfaceSecurityClass,
        privileged: bool,
    ) -> Result<SurfaceId, SurfaceError> {
        if security_class >= SurfaceSecurityClass::Trusted && !privileged {
            return Err(SurfaceError::AccessDenied);
        }
        let byte_length = surface_byte_length(size)?;
        if byte_length > self.byte_budget.saturating_sub(self.bytes_reserved) {
            return Err(SurfaceError::BudgetExceeded);
        }
        let slot = self
            .entries
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(SurfaceError::Full)?;
        let id = SurfaceId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1).max(1);
        *slot = Some(SurfaceDescriptor {
            id,
            owner,
            size,
            stride_pixels: size.width,
            format,
            security_class,
            byte_length,
            content_generation: 0,
            visible: true,
            opacity: 255,
            pending_damage: None,
            associated_window: None,
        });
        self.bytes_reserved = self.bytes_reserved.saturating_add(byte_length);
        Ok(id)
    }

    // ------------------------=
    // FUNC: inspect
    // DESC: Returns immutable surface metadata without exposing another context's pixel memory.
    // ------------------=
    pub fn inspect(&self, id: SurfaceId) -> Option<&SurfaceDescriptor> {
        self.entries
            .iter()
            .flatten()
            .find(|surface| surface.id == id)
    }

    // ------------------------=
    // FUNC: publish
    // DESC: Advances a surface generation only for its owner and rejects replayed generations.
    // ------------------=
    pub fn publish(
        &mut self,
        owner: ContextId,
        id: SurfaceId,
        generation: u64,
    ) -> Result<(), SurfaceError> {
        let surface = self
            .entries
            .iter_mut()
            .flatten()
            .find(|surface| surface.id == id)
            .ok_or(SurfaceError::Unknown)?;
        if surface.owner != owner {
            return Err(SurfaceError::AccessDenied);
        }
        if generation <= surface.content_generation {
            return Err(SurfaceError::StaleGeneration);
        }
        surface.content_generation = generation;
        surface.pending_damage = None;
        Ok(())
    }

    // ------------------------=
    // FUNC: mark_damage
    // DESC: Records a conservative owner-scoped surface-local damage union for the next commit.
    // ------------------=
    pub fn mark_damage(
        &mut self,
        owner: ContextId,
        id: SurfaceId,
        rect: Rect,
    ) -> Result<Rect, SurfaceError> {
        let surface = self
            .entries
            .iter_mut()
            .flatten()
            .find(|surface| surface.id == id)
            .ok_or(SurfaceError::Unknown)?;
        if surface.owner != owner {
            return Err(SurfaceError::AccessDenied);
        }
        let clipped = rect.intersection(Rect {
            x: 0,
            y: 0,
            width: surface.size.width,
            height: surface.size.height,
        });
        if clipped.width == 0 || clipped.height == 0 {
            return Err(SurfaceError::InvalidGeometry);
        }
        surface.pending_damage = Some(match surface.pending_damage {
            Some(current) => current.union(clipped),
            None => clipped,
        });
        Ok(surface.pending_damage.unwrap())
    }

    // ------------------------=
    // FUNC: set_visibility
    // DESC: Changes whether an owned retained surface participates in composition.
    // ------------------=
    pub fn set_visibility(
        &mut self,
        owner: ContextId,
        id: SurfaceId,
        visible: bool,
    ) -> Result<(), SurfaceError> {
        let surface = self.owned_mut(owner, id)?;
        surface.visible = visible;
        Ok(())
    }

    // ------------------------=
    // FUNC: set_opacity
    // DESC: Updates an owned surface's global opacity without exposing its backing pixels.
    // ------------------=
    pub fn set_opacity(
        &mut self,
        owner: ContextId,
        id: SurfaceId,
        opacity: u8,
    ) -> Result<(), SurfaceError> {
        let surface = self.owned_mut(owner, id)?;
        surface.opacity = opacity;
        Ok(())
    }

    // ------------------------=
    // FUNC: associate_window
    // DESC: Records the owner-authorized Window Server association independently of screen position.
    // ------------------=
    pub fn associate_window(
        &mut self,
        owner: ContextId,
        id: SurfaceId,
        window: Option<WindowId>,
    ) -> Result<(), SurfaceError> {
        let surface = self.owned_mut(owner, id)?;
        surface.associated_window = window;
        Ok(())
    }

    // ------------------------=
    // FUNC: resize
    // DESC: Atomically updates an owned surface reservation after geometry and total-budget validation.
    // ------------------=
    pub fn resize(
        &mut self,
        owner: ContextId,
        id: SurfaceId,
        size: Size,
    ) -> Result<SurfaceDescriptor, SurfaceError> {
        let byte_length = surface_byte_length(size)?;
        let index = self
            .entries
            .iter()
            .position(|entry| entry.map(|surface| surface.id) == Some(id))
            .ok_or(SurfaceError::Unknown)?;
        let current = self.entries[index].ok_or(SurfaceError::Unknown)?;
        if current.owner != owner {
            return Err(SurfaceError::AccessDenied);
        }
        let available = self
            .byte_budget
            .saturating_sub(self.bytes_reserved)
            .saturating_add(current.byte_length);
        if byte_length > available {
            return Err(SurfaceError::BudgetExceeded);
        }
        let mut resized = current;
        resized.size = size;
        resized.stride_pixels = size.width;
        resized.byte_length = byte_length;
        resized.content_generation = current.content_generation.wrapping_add(1);
        resized.pending_damage = Some(Rect {
            x: 0,
            y: 0,
            width: size.width,
            height: size.height,
        });
        self.bytes_reserved = self
            .bytes_reserved
            .saturating_sub(current.byte_length)
            .saturating_add(byte_length);
        self.entries[index] = Some(resized);
        Ok(resized)
    }

    // ------------------------=
    // FUNC: destroy
    // DESC: Releases an owned surface reservation without disturbing unrelated contexts.
    // ------------------=
    pub fn destroy(&mut self, owner: ContextId, id: SurfaceId) -> Result<(), SurfaceError> {
        let slot = self
            .entries
            .iter_mut()
            .find(|entry| entry.map(|surface| surface.id) == Some(id))
            .ok_or(SurfaceError::Unknown)?;
        let surface = slot.ok_or(SurfaceError::Unknown)?;
        if surface.owner != owner {
            return Err(SurfaceError::AccessDenied);
        }
        self.bytes_reserved = self.bytes_reserved.saturating_sub(surface.byte_length);
        *slot = None;
        Ok(())
    }

    // ------------------------=
    // FUNC: context_failed
    // DESC: Reclaims all surface reservations owned by a failed execution context.
    // ------------------=
    pub fn context_failed(&mut self, owner: ContextId) -> usize {
        let mut removed = 0;
        for entry in &mut self.entries {
            if entry.map(|surface| surface.owner) == Some(owner) {
                self.bytes_reserved = self
                    .bytes_reserved
                    .saturating_sub(entry.unwrap().byte_length);
                *entry = None;
                removed += 1;
            }
        }
        removed
    }

    // ------------------------=
    // FUNC: bytes_reserved
    // DESC: Reports current retained-surface memory reservations for resource governance.
    // ------------------=
    pub const fn bytes_reserved(&self) -> u64 {
        self.bytes_reserved
    }

    // ------------------------=
    // FUNC: count
    // DESC: Reports the number of live retained surfaces.
    // ------------------=
    pub fn count(&self) -> usize {
        self.entries.iter().flatten().count()
    }

    // ------------------------=
    // FUNC: owned_mut
    // DESC: Resolves mutable retained-surface metadata only for its owning execution context.
    // ------------------=
    fn owned_mut(
        &mut self,
        owner: ContextId,
        id: SurfaceId,
    ) -> Result<&mut SurfaceDescriptor, SurfaceError> {
        let surface = self
            .entries
            .iter_mut()
            .flatten()
            .find(|surface| surface.id == id)
            .ok_or(SurfaceError::Unknown)?;
        if surface.owner != owner {
            return Err(SurfaceError::AccessDenied);
        }
        Ok(surface)
    }
}

// ------------------------=
// FUNC: surface_byte_length
// DESC: Validates bounded geometry and calculates packed 32-bit surface storage without overflow.
// ------------------=
fn surface_byte_length(size: Size) -> Result<u64, SurfaceError> {
    if size.width == 0
        || size.height == 0
        || size.width > MAX_SURFACE_EDGE
        || size.height > MAX_SURFACE_EDGE
    {
        return Err(SurfaceError::InvalidGeometry);
    }
    u64::from(size.width)
        .checked_mul(u64::from(size.height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(SurfaceError::InvalidGeometry)
}
