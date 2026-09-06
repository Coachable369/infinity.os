//! Lock-safe desktop geometry and inactivity state.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopResumeSurface {
    Workspace,
    Settings,
    TextEditor,
    CommandWindow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowPlacement {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
    pub visible: bool,
}

impl WindowPlacement {
    // ------------------------=
    // FUNC: new
    // DESC: Captures one normalized window placement without changing its geometry.
    // ------------------=
    pub const fn new(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        maximized: bool,
        visible: bool,
    ) -> Self {
        Self {
            x,
            y,
            width,
            height,
            maximized,
            visible,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DesktopSessionLayout {
    pub home: WindowPlacement,
    pub settings: WindowPlacement,
    pub editor: WindowPlacement,
    pub command: WindowPlacement,
    pub desktop_item_positions: [[i32; 2]; 7],
    pub focused_surface: DesktopResumeSurface,
    pub settings_section: usize,
    pub settings_expanded_row: Option<usize>,
    pub settings_scroll_offset: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LockedDesktopState {
    saved: Option<DesktopSessionLayout>,
}

impl LockedDesktopState {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty lock boundary with no stale desktop snapshot.
    // ------------------=
    pub const fn new() -> Self {
        Self { saved: None }
    }

    // ------------------------=
    // FUNC: save
    // DESC: Atomically retains the latest desktop layout before the lock surface replaces it.
    // ------------------=
    pub fn save(&mut self, layout: DesktopSessionLayout) {
        self.saved = Some(layout);
    }

    // ------------------------=
    // FUNC: restore
    // DESC: Returns the retained layout exactly once after successful authentication.
    // ------------------=
    pub fn restore(&mut self) -> Option<DesktopSessionLayout> {
        self.saved.take()
    }

    // ------------------------=
    // FUNC: has_saved_layout
    // DESC: Reports whether a locked session currently owns a restorable desktop layout.
    // ------------------=
    pub const fn has_saved_layout(self) -> bool {
        self.saved.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionIdleState {
    seconds: u32,
}

impl SessionIdleState {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an activity timer with no accumulated idle time.
    // ------------------=
    pub const fn new() -> Self {
        Self { seconds: 0 }
    }

    // ------------------------=
    // FUNC: note_activity
    // DESC: Resets the inactivity interval after authenticated keyboard or pointer input.
    // ------------------=
    pub fn note_activity(&mut self) {
        self.seconds = 0;
    }

    // ------------------------=
    // FUNC: tick
    // DESC: Advances one elapsed second and requests a lock exactly at the selected user deadline.
    // ------------------=
    pub fn tick(&mut self, authenticated_desktop_visible: bool, timeout_seconds: u32) -> bool {
        if !authenticated_desktop_visible {
            self.seconds = 0;
            return false;
        }
        self.seconds = self.seconds.saturating_add(1);
        if self.seconds >= timeout_seconds.max(1) {
            self.seconds = 0;
            true
        } else {
            false
        }
    }

    // ------------------------=
    // FUNC: elapsed_seconds
    // DESC: Exposes structured inactivity state for diagnostics and behavioral verification.
    // ------------------=
    pub const fn elapsed_seconds(self) -> u32 {
        self.seconds
    }
}
