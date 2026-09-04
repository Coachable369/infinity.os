//! Window/surface ownership, z-order policy, modal state, and crash cleanup.

use super::geometry::{Point, Rect};

pub const MAX_WINDOWS: usize = 16;
pub const MAX_DISPLAYS: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WindowId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SurfaceId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ContextId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum ZOrderClass {
    Desktop = 0,
    Normal = 1,
    Floating = 2,
    Menu = 3,
    Modal = 4,
    Trusted = 5,
    Cursor = 6,
}

#[derive(Clone, Copy)]
pub struct Window {
    pub id: WindowId,
    pub owner: ContextId,
    pub surface: SurfaceId,
    pub bounds: Rect,
    pub z_class: ZOrderClass,
    pub visible: bool,
    pub modal_parent: Option<WindowId>,
    pub accepts_input: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WindowError {
    Full,
    Duplicate,
    Unknown,
    AccessDenied,
    InvalidBounds,
    ModalBlocked,
}

pub struct WindowServer {
    windows: [Option<Window>; MAX_WINDOWS],
    focused: Option<WindowId>,
    pointer_capture: Option<(WindowId, ContextId)>,
    next_id: u32,
}

impl WindowServer {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty capability-gated window registry.
    // ------------------=
    pub const fn new() -> Self {
        Self { windows: [None; MAX_WINDOWS], focused: None, pointer_capture: None, next_id: 1 }
    }

    // ------------------------=
    // FUNC: create
    // DESC: Creates a surface-backed window owned by exactly one execution context.
    // ------------------=
    pub fn create(&mut self, owner: ContextId, surface: SurfaceId, bounds: Rect, z_class: ZOrderClass) -> Result<WindowId, WindowError> {
        if bounds.width == 0 || bounds.height == 0 {
            return Err(WindowError::InvalidBounds);
        }
        let slot = self.windows.iter_mut().find(|entry| entry.is_none()).ok_or(WindowError::Full)?;
        let id = WindowId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1).max(1);
        *slot = Some(Window { id, owner, surface, bounds, z_class, visible: true, modal_parent: None, accepts_input: true });
        if self.focused.is_none() && z_class != ZOrderClass::Desktop {
            self.focused = Some(id);
        }
        Ok(id)
    }

    // ------------------------=
    // FUNC: inspect
    // DESC: Resolves immutable window metadata without exposing surface memory.
    // ------------------=
    pub fn inspect(&self, id: WindowId) -> Option<&Window> {
        self.windows.iter().flatten().find(|window| window.id == id)
    }

    // ------------------------=
    // FUNC: mutate
    // DESC: Resolves a mutable window only when the requesting execution context owns it.
    // ------------------=
    pub fn mutate(&mut self, caller: ContextId, id: WindowId) -> Result<&mut Window, WindowError> {
        let window = self.windows.iter_mut().flatten().find(|window| window.id == id).ok_or(WindowError::Unknown)?;
        if window.owner != caller {
            return Err(WindowError::AccessDenied);
        }
        Ok(window)
    }

    // ------------------------=
    // FUNC: move_window
    // DESC: Moves an owned window while constraining its title bar and body to the usable work area.
    // ------------------=
    pub fn move_window(&mut self, caller: ContextId, id: WindowId, requested: Point, work_area: Rect) -> Result<Rect, WindowError> {
        let window = self.mutate(caller, id)?;
        let maximum_x = work_area
            .x
            .saturating_add(work_area.width.saturating_sub(window.bounds.width) as i32);
        let maximum_y = work_area
            .y
            .saturating_add(work_area.height.saturating_sub(window.bounds.height) as i32);
        window.bounds.x = requested.x.clamp(work_area.x, maximum_x.max(work_area.x));
        window.bounds.y = requested.y.clamp(work_area.y, maximum_y.max(work_area.y));
        Ok(window.bounds)
    }

    // ------------------------=
    // FUNC: resize_window
    // DESC: Resizes an owned window within minimum dimensions and the current display work area.
    // ------------------=
    pub fn resize_window(&mut self, caller: ContextId, id: WindowId, requested_width: u32, requested_height: u32, work_area: Rect) -> Result<Rect, WindowError> {
        const MINIMUM_WIDTH: u32 = 240;
        const MINIMUM_HEIGHT: u32 = 160;
        let window = self.mutate(caller, id)?;
        let available_width = work_area
            .right()
            .saturating_sub(window.bounds.x)
            .max(0) as u32;
        let available_height = work_area
            .bottom()
            .saturating_sub(window.bounds.y)
            .max(0) as u32;
        if available_width < MINIMUM_WIDTH || available_height < MINIMUM_HEIGHT {
            return Err(WindowError::InvalidBounds);
        }
        window.bounds.width = requested_width.clamp(MINIMUM_WIDTH, available_width);
        window.bounds.height = requested_height.clamp(MINIMUM_HEIGHT, available_height);
        Ok(window.bounds)
    }

    // ------------------------=
    // FUNC: focus
    // DESC: Gives input focus to an eligible window while respecting the active modal chain.
    // ------------------=
    pub fn focus(&mut self, id: WindowId) -> Result<(), WindowError> {
        let candidate = *self.inspect(id).ok_or(WindowError::Unknown)?;
        if !candidate.visible || !candidate.accepts_input {
            return Err(WindowError::AccessDenied);
        }
        if let Some(modal) = self.top_modal() {
            if modal.id != id && modal.modal_parent != Some(id) {
                return Err(WindowError::ModalBlocked);
            }
        }
        self.focused = Some(id);
        Ok(())
    }

    // ------------------------=
    // FUNC: hit_test
    // DESC: Selects the highest eligible surface at a global pointer coordinate.
    // ------------------=
    pub fn hit_test(&self, point: Point) -> Option<WindowId> {
        let modal = self.top_modal().map(|window| window.id);
        let mut best = None;
        let mut best_z = 0u8;
        for window in self.windows.iter().flatten() {
            if !window.visible || !window.accepts_input || !window.bounds.contains(point) {
                continue;
            }
            if modal.is_some() && modal != Some(window.id) {
                continue;
            }
            if best.is_none() || window.z_class as u8 >= best_z {
                best = Some(window.id);
                best_z = window.z_class as u8;
            }
        }
        best
    }

    // ------------------------=
    // FUNC: capture_pointer
    // DESC: Grants pointer capture only to the execution context that owns the target window.
    // ------------------=
    pub fn capture_pointer(&mut self, caller: ContextId, id: WindowId) -> Result<(), WindowError> {
        let window = *self.inspect(id).ok_or(WindowError::Unknown)?;
        if window.owner != caller {
            return Err(WindowError::AccessDenied);
        }
        self.pointer_capture = Some((id, caller));
        Ok(())
    }

    // ------------------------=
    // FUNC: release_pointer
    // DESC: Releases capture only for its owning context and prevents cross-context interference.
    // ------------------=
    pub fn release_pointer(&mut self, caller: ContextId) -> Result<(), WindowError> {
        if self.pointer_capture.map(|entry| entry.1) != Some(caller) {
            return Err(WindowError::AccessDenied);
        }
        self.pointer_capture = None;
        Ok(())
    }

    // ------------------------=
    // FUNC: context_failed
    // DESC: Removes every crashed-context surface and repairs focus and pointer capture.
    // ------------------=
    pub fn context_failed(&mut self, owner: ContextId) -> usize {
        let mut removed = 0;
        for entry in &mut self.windows {
            if entry.map(|window| window.owner) == Some(owner) {
                if self.focused == entry.map(|window| window.id) {
                    self.focused = None;
                }
                *entry = None;
                removed += 1;
            }
        }
        if self.pointer_capture.map(|entry| entry.1) == Some(owner) {
            self.pointer_capture = None;
        }
        removed
    }

    // ------------------------=
    // FUNC: focused
    // DESC: Returns the window currently receiving keyboard events.
    // ------------------=
    pub const fn focused(&self) -> Option<WindowId> {
        self.focused
    }

    // ------------------------=
    // FUNC: top_modal
    // DESC: Resolves the active highest modal window for focus trapping.
    // ------------------=
    fn top_modal(&self) -> Option<&Window> {
        self.windows.iter().flatten().find(|window| window.visible && window.z_class == ZOrderClass::Modal)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DisplayRotation {
    Zero,
    Ninety,
    OneEighty,
    TwoSeventy,
}

#[derive(Clone, Copy)]
pub struct DisplayDescriptor {
    pub id: u32,
    pub bounds: Rect,
    pub work_area: Rect,
    pub scale: super::geometry::Scale,
    pub rotation: DisplayRotation,
    pub primary: bool,
}

// ------------------------=
// FUNC: place_dialog
// DESC: Centers a dialog in the display work area with safe edge gutters.
// ------------------=
pub fn place_dialog(display: DisplayDescriptor, requested_width: u32, requested_height: u32) -> Rect {
    let gutter = display.scale.pixels(24);
    let width = requested_width.min(display.work_area.width.saturating_sub(gutter * 2));
    let height = requested_height.min(display.work_area.height.saturating_sub(gutter * 2));
    Rect {
        x: display.work_area.x + (display.work_area.width.saturating_sub(width) / 2) as i32,
        y: display.work_area.y + (display.work_area.height.saturating_sub(height) / 2) as i32,
        width,
        height,
    }
}
