//! Window/surface ownership, z-order policy, modal state, and crash cleanup.

use super::geometry::{Point, Rect};
use super::trusted::TrustedWindowToken;

pub const MAX_WINDOWS: usize = 16;
pub const MAX_DISPLAYS: usize = 4;
pub const MAX_WINDOW_EVENTS: usize = 32;

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Window {
    pub id: WindowId,
    pub owner: ContextId,
    pub surface: SurfaceId,
    pub bounds: Rect,
    pub previous_bounds: Rect,
    pub restore_bounds: Rect,
    pub minimum_width: u32,
    pub minimum_height: u32,
    pub z_class: ZOrderClass,
    pub state: WindowState,
    pub visible: bool,
    pub modal_parent: Option<WindowId>,
    pub accepts_input: bool,
}

impl Window {
    // ------------------------=
    // FUNC: assistant_geometry
    // DESC: Makes the assistant a standard normal-window affordance, excluding privileged secure input and non-app surfaces.
    // ------------------=
    pub fn assistant_geometry(
        &self,
        scale: usize,
        expanded: bool,
    ) -> Option<super::app_assistant::Geometry> {
        (self.visible && matches!(self.z_class, ZOrderClass::Normal | ZOrderClass::Floating))
            .then(|| super::app_assistant::geometry(self.bounds, scale, expanded))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WindowError {
    Full,
    Duplicate,
    Unknown,
    AccessDenied,
    InvalidBounds,
    ModalBlocked,
    PrivilegedZOrder,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WindowTransition {
    pub old_bounds: Rect,
    pub new_bounds: Rect,
    pub state: WindowState,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WindowEventKind {
    Created,
    Destroyed,
    Focused,
    Moved,
    Resized,
    StateChanged,
    CaptureChanged,
    ContextFailed,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WindowEvent {
    pub sequence: u64,
    pub kind: WindowEventKind,
    pub window: WindowId,
    pub owner: ContextId,
    pub old_bounds: Rect,
    pub new_bounds: Rect,
}

impl WindowEvent {
    pub const ENCODED_BYTES: usize = 52;

    // ------------------------=
    // FUNC: encode_v1
    // DESC: Encodes a versioned machine-readable geometry transition without exposing pixels or titles.
    // ------------------=
    pub fn encode_v1(&self, out: &mut [u8; Self::ENCODED_BYTES]) {
        out.fill(0);
        out[0] = 1;
        out[1] = self.kind as u8;
        out[4..8].copy_from_slice(&self.window.0.to_le_bytes());
        out[8..12].copy_from_slice(&self.owner.0.to_le_bytes());
        out[12..20].copy_from_slice(&self.sequence.to_le_bytes());
        encode_rect(self.old_bounds, &mut out[20..36]);
        encode_rect(self.new_bounds, &mut out[36..52]);
    }
}

// ------------------------=
// FUNC: encode_rect
// DESC: Encodes signed position and unsigned extent fields in fixed little-endian order.
// ------------------=
fn encode_rect(rect: Rect, out: &mut [u8]) {
    out[0..4].copy_from_slice(&rect.x.to_le_bytes());
    out[4..8].copy_from_slice(&rect.y.to_le_bytes());
    out[8..12].copy_from_slice(&rect.width.to_le_bytes());
    out[12..16].copy_from_slice(&rect.height.to_le_bytes());
}

pub struct WindowServer {
    windows: [Option<Window>; MAX_WINDOWS],
    focused: Option<WindowId>,
    pointer_capture: Option<(WindowId, ContextId)>,
    next_id: u32,
    events: [Option<WindowEvent>; MAX_WINDOW_EVENTS],
    event_head: usize,
    event_len: usize,
    event_sequence: u64,
    dropped_events: u32,
}

#[derive(Clone, Copy)]
pub struct WindowServerCheckpoint {
    windows: [Option<Window>; MAX_WINDOWS],
    focused: Option<WindowId>,
    next_id: u32,
}

impl WindowServer {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty capability-gated window registry.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            windows: [None; MAX_WINDOWS],
            focused: None,
            pointer_capture: None,
            next_id: 1,
            events: [None; MAX_WINDOW_EVENTS],
            event_head: 0,
            event_len: 0,
            event_sequence: 0,
            dropped_events: 0,
        }
    }

    // ------------------------=
    // FUNC: checkpoint
    // DESC: Captures bounded authoritative window metadata without transient input capture or pixels.
    // ------------------=
    pub fn checkpoint(&self) -> WindowServerCheckpoint {
        WindowServerCheckpoint {
            windows: self.windows,
            focused: self.focused,
            next_id: self.next_id,
        }
    }

    // ------------------------=
    // FUNC: recover
    // DESC: Reconstructs non-trusted window metadata while clearing transient capture and event queues.
    // ------------------=
    pub fn recover(checkpoint: WindowServerCheckpoint) -> Self {
        let mut windows = checkpoint.windows;
        for entry in &mut windows {
            if matches!(
                entry.map(|window| window.z_class),
                Some(ZOrderClass::Trusted | ZOrderClass::Cursor)
            ) {
                *entry = None;
            }
        }
        let focused = checkpoint.focused.filter(|id| {
            windows
                .iter()
                .flatten()
                .any(|window| window.id == *id && window.visible && window.accepts_input)
        });
        Self {
            windows,
            focused,
            pointer_capture: None,
            next_id: checkpoint.next_id,
            events: [None; MAX_WINDOW_EVENTS],
            event_head: 0,
            event_len: 0,
            event_sequence: 0,
            dropped_events: 0,
        }
    }

    // ------------------------=
    // FUNC: create
    // DESC: Creates a surface-backed window owned by exactly one execution context.
    // ------------------=
    pub fn create(
        &mut self,
        owner: ContextId,
        surface: SurfaceId,
        bounds: Rect,
        z_class: ZOrderClass,
    ) -> Result<WindowId, WindowError> {
        if matches!(z_class, ZOrderClass::Trusted | ZOrderClass::Cursor) {
            return Err(WindowError::PrivilegedZOrder);
        }
        self.create_internal(owner, surface, bounds, z_class)
    }

    // ------------------------=
    // FUNC: create_trusted
    // DESC: Creates a trusted overlay only when secure-input policy issued a scoped owner token.
    // ------------------=
    pub fn create_trusted(
        &mut self,
        token: TrustedWindowToken,
        owner: ContextId,
        surface: SurfaceId,
        bounds: Rect,
    ) -> Result<WindowId, WindowError> {
        if !token.authorizes(owner.0) {
            return Err(WindowError::AccessDenied);
        }
        self.create_internal(owner, surface, bounds, ZOrderClass::Trusted)
    }

    // ------------------------=
    // FUNC: create_internal
    // DESC: Inserts validated window metadata into the bounded retained window registry.
    // ------------------=
    fn create_internal(
        &mut self,
        owner: ContextId,
        surface: SurfaceId,
        bounds: Rect,
        z_class: ZOrderClass,
    ) -> Result<WindowId, WindowError> {
        if bounds.width == 0 || bounds.height == 0 {
            return Err(WindowError::InvalidBounds);
        }
        let slot = self
            .windows
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(WindowError::Full)?;
        let id = WindowId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1).max(1);
        *slot = Some(Window {
            id,
            owner,
            surface,
            bounds,
            previous_bounds: bounds,
            restore_bounds: bounds,
            minimum_width: 240,
            minimum_height: 160,
            z_class,
            state: WindowState::Normal,
            visible: true,
            modal_parent: None,
            accepts_input: true,
        });
        if self.focused.is_none() && z_class != ZOrderClass::Desktop {
            self.focused = Some(id);
        }
        self.emit(WindowEventKind::Created, id, owner, bounds, bounds);
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
        let window = self
            .windows
            .iter_mut()
            .flatten()
            .find(|window| window.id == id)
            .ok_or(WindowError::Unknown)?;
        if window.owner != caller {
            return Err(WindowError::AccessDenied);
        }
        Ok(window)
    }

    // ------------------------=
    // FUNC: move_window
    // DESC: Moves an owned window while constraining its title bar and body to the usable work area.
    // ------------------=
    pub fn move_window(
        &mut self,
        caller: ContextId,
        id: WindowId,
        requested: Point,
        work_area: Rect,
    ) -> Result<Rect, WindowError> {
        let (old_bounds, new_bounds, owner) = {
            let window = self.mutate(caller, id)?;
            if window.state != WindowState::Normal {
                return Err(WindowError::InvalidBounds);
            }
            let maximum_x = work_area
                .x
                .saturating_add(work_area.width.saturating_sub(window.bounds.width) as i32);
            let maximum_y = work_area
                .y
                .saturating_add(work_area.height.saturating_sub(window.bounds.height) as i32);
            let old_bounds = window.bounds;
            window.previous_bounds = old_bounds;
            window.bounds.x = requested.x.clamp(work_area.x, maximum_x.max(work_area.x));
            window.bounds.y = requested.y.clamp(work_area.y, maximum_y.max(work_area.y));
            (old_bounds, window.bounds, window.owner)
        };
        self.emit(WindowEventKind::Moved, id, owner, old_bounds, new_bounds);
        Ok(new_bounds)
    }

    // ------------------------=
    // FUNC: resize_window
    // DESC: Resizes an owned window within minimum dimensions and the current display work area.
    // ------------------=
    pub fn resize_window(
        &mut self,
        caller: ContextId,
        id: WindowId,
        requested_width: u32,
        requested_height: u32,
        work_area: Rect,
    ) -> Result<Rect, WindowError> {
        let (old_bounds, new_bounds, owner) = {
            let window = self.mutate(caller, id)?;
            let available_width = work_area.right().saturating_sub(window.bounds.x).max(0) as u32;
            let available_height = work_area.bottom().saturating_sub(window.bounds.y).max(0) as u32;
            if window.state != WindowState::Normal
                || available_width < window.minimum_width
                || available_height < window.minimum_height
            {
                return Err(WindowError::InvalidBounds);
            }
            let old_bounds = window.bounds;
            window.previous_bounds = old_bounds;
            window.bounds.width = requested_width.clamp(window.minimum_width, available_width);
            window.bounds.height = requested_height.clamp(window.minimum_height, available_height);
            (old_bounds, window.bounds, window.owner)
        };
        self.emit(WindowEventKind::Resized, id, owner, old_bounds, new_bounds);
        Ok(new_bounds)
    }

    // ------------------------=
    // FUNC: set_state
    // DESC: Applies minimize, maximize, fullscreen, and restore policy while preserving reconstructable bounds.
    // ------------------=
    pub fn set_state(
        &mut self,
        caller: ContextId,
        id: WindowId,
        state: WindowState,
        work_area: Rect,
        display_area: Rect,
    ) -> Result<WindowTransition, WindowError> {
        let (old_bounds, new_bounds, visible, owner) = {
            let window = self.mutate(caller, id)?;
            let old_bounds = window.bounds;
            if window.state == WindowState::Normal && state != WindowState::Normal {
                window.restore_bounds = window.bounds;
            }
            window.previous_bounds = old_bounds;
            window.state = state;
            match state {
                WindowState::Normal => {
                    window.bounds = window.restore_bounds;
                    window.visible = true;
                    window.accepts_input = true;
                }
                WindowState::Minimized => {
                    window.visible = false;
                    window.accepts_input = false;
                }
                WindowState::Maximized => {
                    window.bounds = work_area;
                    window.visible = true;
                    window.accepts_input = true;
                }
                WindowState::Fullscreen => {
                    window.bounds = display_area;
                    window.visible = true;
                    window.accepts_input = true;
                }
            }
            (old_bounds, window.bounds, window.visible, window.owner)
        };
        if !visible && self.focused == Some(id) {
            self.focused = None;
        }
        self.emit(
            WindowEventKind::StateChanged,
            id,
            owner,
            old_bounds,
            new_bounds,
        );
        Ok(WindowTransition {
            old_bounds,
            new_bounds,
            state,
        })
    }

    // ------------------------=
    // FUNC: close
    // DESC: Closes an owned window and repairs focus and pointer capture without ambient authority.
    // ------------------=
    pub fn close(&mut self, caller: ContextId, id: WindowId) -> Result<Window, WindowError> {
        let slot = self
            .windows
            .iter_mut()
            .find(|entry| entry.map(|window| window.id) == Some(id))
            .ok_or(WindowError::Unknown)?;
        let window = slot.ok_or(WindowError::Unknown)?;
        if window.owner != caller {
            return Err(WindowError::AccessDenied);
        }
        *slot = None;
        if self.focused == Some(id) {
            self.focused = None;
        }
        if self.pointer_capture.map(|capture| capture.0) == Some(id) {
            self.pointer_capture = None;
        }
        self.emit(
            WindowEventKind::Destroyed,
            id,
            caller,
            window.bounds,
            window.bounds,
        );
        Ok(window)
    }

    // ------------------------=
    // FUNC: set_modal_parent
    // DESC: Establishes an owned modal relationship while preventing cross-context modal spoofing.
    // ------------------=
    pub fn set_modal_parent(
        &mut self,
        caller: ContextId,
        child: WindowId,
        parent: WindowId,
    ) -> Result<(), WindowError> {
        let parent_window = *self.inspect(parent).ok_or(WindowError::Unknown)?;
        if parent_window.owner != caller {
            return Err(WindowError::AccessDenied);
        }
        let child_window = self.mutate(caller, child)?;
        if child_window.z_class != ZOrderClass::Modal {
            return Err(WindowError::InvalidBounds);
        }
        child_window.modal_parent = Some(parent);
        Ok(())
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
        self.emit(
            WindowEventKind::Focused,
            id,
            candidate.owner,
            candidate.bounds,
            candidate.bounds,
        );
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
        self.emit(
            WindowEventKind::CaptureChanged,
            id,
            caller,
            window.bounds,
            window.bounds,
        );
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
        let (window, owner) = self.pointer_capture.unwrap();
        let bounds = self
            .inspect(window)
            .map(|entry| entry.bounds)
            .unwrap_or_default();
        self.pointer_capture = None;
        self.emit(
            WindowEventKind::CaptureChanged,
            window,
            owner,
            bounds,
            bounds,
        );
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
        if removed != 0 {
            self.emit(
                WindowEventKind::ContextFailed,
                WindowId(0),
                owner,
                Rect::default(),
                Rect::default(),
            );
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
    // FUNC: count
    // DESC: Returns the number of live windows for typed inspection and restart reconstruction.
    // ------------------=
    pub fn count(&self) -> usize {
        self.windows.iter().flatten().count()
    }

    // ------------------------=
    // FUNC: uses_surface
    // DESC: Reports whether a live window still references a retained surface before reclamation.
    // ------------------=
    pub fn uses_surface(&self, surface: SurfaceId) -> bool {
        self.windows
            .iter()
            .flatten()
            .any(|window| window.surface == surface)
    }

    // ------------------------=
    // FUNC: transition_damage
    // DESC: Returns the old and new retained bounds that must be composed for the latest geometry transition.
    // ------------------=
    pub fn transition_damage(&self, id: WindowId) -> Option<(Rect, Rect)> {
        self.inspect(id)
            .map(|window| (window.previous_bounds, window.bounds))
    }

    // ------------------------=
    // FUNC: pointer_target
    // DESC: Routes pointer input to the capture owner when active or otherwise performs policy hit testing.
    // ------------------=
    pub fn pointer_target(&self, point: Point) -> Option<WindowId> {
        self.pointer_capture
            .map(|capture| capture.0)
            .or_else(|| self.hit_test(point))
    }

    // ------------------------=
    // FUNC: next_event
    // DESC: Removes the oldest typed window lifecycle event from the bounded bridge queue.
    // ------------------=
    pub fn next_event(&mut self) -> Option<WindowEvent> {
        if self.event_len == 0 {
            return None;
        }
        let event = self.events[self.event_head].take();
        self.event_head = (self.event_head + 1) % MAX_WINDOW_EVENTS;
        self.event_len -= 1;
        event
    }

    // ------------------------=
    // FUNC: dropped_event_count
    // DESC: Reports typed window events coalesced by bounded-queue pressure.
    // ------------------=
    pub const fn dropped_event_count(&self) -> u32 {
        self.dropped_events
    }

    // ------------------------=
    // FUNC: emit
    // DESC: Enqueues a typed lifecycle event and drops the oldest record under bounded pressure.
    // ------------------=
    fn emit(
        &mut self,
        kind: WindowEventKind,
        window: WindowId,
        owner: ContextId,
        old_bounds: Rect,
        new_bounds: Rect,
    ) {
        if self.event_len == MAX_WINDOW_EVENTS {
            self.events[self.event_head] = None;
            self.event_head = (self.event_head + 1) % MAX_WINDOW_EVENTS;
            self.event_len -= 1;
            self.dropped_events = self.dropped_events.saturating_add(1);
        }
        self.event_sequence = self.event_sequence.wrapping_add(1);
        let index = (self.event_head + self.event_len) % MAX_WINDOW_EVENTS;
        self.events[index] = Some(WindowEvent {
            sequence: self.event_sequence,
            kind,
            window,
            owner,
            old_bounds,
            new_bounds,
        });
        self.event_len += 1;
    }

    // ------------------------=
    // FUNC: top_modal
    // DESC: Resolves the active highest modal window for focus trapping.
    // ------------------=
    fn top_modal(&self) -> Option<&Window> {
        self.windows
            .iter()
            .flatten()
            .find(|window| window.visible && window.z_class == ZOrderClass::Modal)
    }
}
