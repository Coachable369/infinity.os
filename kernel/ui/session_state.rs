//! Lock-safe desktop geometry and inactivity state.

pub const DESKTOP_LAYOUT_STATE_BYTES: usize = 1536;
pub const MAX_PERSISTED_DESKTOP_LAYOUTS: usize = 8;
const DESKTOP_LAYOUT_RECORD_BYTES: usize = 177;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopResumeSurface {
    Workspace,
    Settings,
    TextEditor,
    CommandWindow,
    TaskManager,
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
    pub task_manager: WindowPlacement,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersistentDesktopLayoutStore {
    entries: [Option<([u8; 16], DesktopSessionLayout)>; MAX_PERSISTED_DESKTOP_LAYOUTS],
}

impl PersistentDesktopLayoutStore {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty user-scoped durable desktop layout collection.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            entries: [None; MAX_PERSISTED_DESKTOP_LAYOUTS],
        }
    }

    // ------------------------=
    // FUNC: layout
    // DESC: Returns the last committed desktop layout for one stable user identity.
    // ------------------=
    pub fn layout(&self, user: [u8; 16]) -> Option<DesktopSessionLayout> {
        self.entries
            .iter()
            .flatten()
            .find(|entry| entry.0 == user)
            .map(|entry| entry.1)
    }

    // ------------------------=
    // FUNC: save
    // DESC: Replaces or inserts one user's complete desktop layout atomically in memory.
    // ------------------=
    pub fn save(&mut self, user: [u8; 16], layout: DesktopSessionLayout) -> bool {
        if user == [0; 16] {
            return false;
        }
        if let Some(entry) = self
            .entries
            .iter_mut()
            .flatten()
            .find(|entry| entry.0 == user)
        {
            entry.1 = layout;
            return true;
        }
        let Some(slot) = self.entries.iter_mut().find(|entry| entry.is_none()) else {
            return false;
        };
        *slot = Some((user, layout));
        true
    }

    // ------------------------=
    // FUNC: encode
    // DESC: Encodes all user layouts into a checksummed architecture-neutral state object.
    // ------------------=
    pub fn encode(&self) -> [u8; DESKTOP_LAYOUT_STATE_BYTES] {
        let mut out = [0u8; DESKTOP_LAYOUT_STATE_BYTES];
        out[..8].copy_from_slice(b"INFDESK1");
        put_u16(&mut out, 8, 1);
        put_u16(&mut out, 10, DESKTOP_LAYOUT_STATE_BYTES as u16);
        out[12] = self.entries.iter().flatten().count() as u8;
        for (index, entry) in self.entries.iter().enumerate() {
            if let Some((user, layout)) = entry {
                let at = 16 + index * DESKTOP_LAYOUT_RECORD_BYTES;
                out[at] = 1;
                out[at + 2..at + 18].copy_from_slice(user);
                write_layout(&mut out, at + 18, *layout);
            }
        }
        let checksum = session_checksum(&out[..DESKTOP_LAYOUT_STATE_BYTES - 4]);
        put_u32(&mut out, DESKTOP_LAYOUT_STATE_BYTES - 4, checksum);
        out
    }

    // ------------------------=
    // FUNC: decode
    // DESC: Validates and restores a durable collection without accepting partial state.
    // ------------------=
    pub fn decode(input: &[u8]) -> Option<Self> {
        if input.len() != DESKTOP_LAYOUT_STATE_BYTES
            || &input[..8] != b"INFDESK1"
            || get_u16(input, 8) != 1
            || get_u16(input, 10) as usize != DESKTOP_LAYOUT_STATE_BYTES
            || get_u32(input, DESKTOP_LAYOUT_STATE_BYTES - 4)
                != session_checksum(&input[..DESKTOP_LAYOUT_STATE_BYTES - 4])
        {
            return None;
        }
        let mut store = Self::new();
        for index in 0..MAX_PERSISTED_DESKTOP_LAYOUTS {
            let at = 16 + index * DESKTOP_LAYOUT_RECORD_BYTES;
            if input[at] == 0 {
                continue;
            }
            let mut user = [0u8; 16];
            user.copy_from_slice(&input[at + 2..at + 18]);
            let layout = read_layout(input, at + 18)?;
            if !store.save(user, layout) {
                return None;
            }
        }
        (store.entries.iter().flatten().count() == input[12] as usize).then_some(store)
    }
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

// ------------------------=
// FUNC: write_layout
// DESC: Writes one bounded desktop layout into a fixed durable record.
// ------------------=
fn write_layout(out: &mut [u8], at: usize, layout: DesktopSessionLayout) {
    for (index, placement) in [layout.home, layout.settings, layout.editor, layout.command]
        .iter()
        .enumerate()
    {
        write_placement(out, at + index * 17, *placement);
    }
    let positions_at = at + 68;
    for (index, position) in layout.desktop_item_positions.iter().enumerate() {
        put_i32(out, positions_at + index * 8, position[0]);
        put_i32(out, positions_at + index * 8 + 4, position[1]);
    }
    out[at + 124] = match layout.focused_surface {
        DesktopResumeSurface::Workspace => 1,
        DesktopResumeSurface::Settings => 2,
        DesktopResumeSurface::TextEditor => 3,
        DesktopResumeSurface::CommandWindow => 4,
        DesktopResumeSurface::TaskManager => 5,
    };
    out[at + 125] = layout.settings_section.min(u8::MAX as usize) as u8;
    out[at + 126] = layout
        .settings_expanded_row
        .map(|row| row.min(254) as u8 + 1)
        .unwrap_or(0);
    put_u32(
        out,
        at + 128,
        layout.settings_scroll_offset.min(u32::MAX as usize) as u32,
    );
    write_placement(out, at + 133, layout.task_manager);
}

// ------------------------=
// FUNC: read_layout
// DESC: Restores one validated desktop layout from a fixed durable record.
// ------------------=
fn read_layout(input: &[u8], at: usize) -> Option<DesktopSessionLayout> {
    let home = read_placement(input, at)?;
    let settings = read_placement(input, at + 17)?;
    let editor = read_placement(input, at + 34)?;
    let command = read_placement(input, at + 51)?;
    let task_manager = read_placement(input, at + 133)?;
    let mut positions = [[0i32; 2]; 7];
    let positions_at = at + 68;
    for (index, position) in positions.iter_mut().enumerate() {
        position[0] = get_i32(input, positions_at + index * 8);
        position[1] = get_i32(input, positions_at + index * 8 + 4);
    }
    let focused_surface = match input[at + 124] {
        1 => DesktopResumeSurface::Workspace,
        2 => DesktopResumeSurface::Settings,
        3 => DesktopResumeSurface::TextEditor,
        4 => DesktopResumeSurface::CommandWindow,
        5 => DesktopResumeSurface::TaskManager,
        _ => return None,
    };
    Some(DesktopSessionLayout {
        home,
        settings,
        editor,
        command,
        task_manager,
        desktop_item_positions: positions,
        focused_surface,
        settings_section: input[at + 125] as usize,
        settings_expanded_row: input[at + 126].checked_sub(1).map(|row| row as usize),
        settings_scroll_offset: get_u32(input, at + 128) as usize,
    })
}

// ------------------------=
// FUNC: write_placement
// DESC: Encodes normalized geometry and visibility flags for one window.
// ------------------=
fn write_placement(out: &mut [u8], at: usize, placement: WindowPlacement) {
    put_i32(out, at, placement.x);
    put_i32(out, at + 4, placement.y);
    put_i32(out, at + 8, placement.width);
    put_i32(out, at + 12, placement.height);
    out[at + 16] = placement.maximized as u8 | ((placement.visible as u8) << 1);
}

// ------------------------=
// FUNC: read_placement
// DESC: Decodes one window placement and rejects invalid geometry or flag bits.
// ------------------=
fn read_placement(input: &[u8], at: usize) -> Option<WindowPlacement> {
    let flags = input[at + 16];
    let width = get_i32(input, at + 8);
    let height = get_i32(input, at + 12);
    if flags & !3 != 0 || width <= 0 || height <= 0 {
        return None;
    }
    Some(WindowPlacement::new(
        get_i32(input, at),
        get_i32(input, at + 4),
        width,
        height,
        flags & 1 != 0,
        flags & 2 != 0,
    ))
}

// ------------------------=
// FUNC: put_u16
// DESC: Writes one little-endian unsigned 16-bit field.
// ------------------=
fn put_u16(out: &mut [u8], at: usize, value: u16) {
    out[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

// ------------------------=
// FUNC: get_u16
// DESC: Reads one little-endian unsigned 16-bit field.
// ------------------=
fn get_u16(input: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([input[at], input[at + 1]])
}

// ------------------------=
// FUNC: put_u32
// DESC: Writes one little-endian unsigned 32-bit field.
// ------------------=
fn put_u32(out: &mut [u8], at: usize, value: u32) {
    out[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

// ------------------------=
// FUNC: get_u32
// DESC: Reads one little-endian unsigned 32-bit field.
// ------------------=
fn get_u32(input: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(input[at..at + 4].try_into().unwrap())
}

// ------------------------=
// FUNC: put_i32
// DESC: Writes one little-endian signed 32-bit geometry field.
// ------------------=
fn put_i32(out: &mut [u8], at: usize, value: i32) {
    out[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

// ------------------------=
// FUNC: get_i32
// DESC: Reads one little-endian signed 32-bit geometry field.
// ------------------=
fn get_i32(input: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(input[at..at + 4].try_into().unwrap())
}

// ------------------------=
// FUNC: session_checksum
// DESC: Computes the desktop-state integrity checksum over serialized bytes.
// ------------------=
fn session_checksum(bytes: &[u8]) -> u32 {
    let mut value = 0x811c9dc5u32;
    for byte in bytes {
        value ^= *byte as u32;
        value = value.wrapping_mul(0x01000193);
    }
    value
}
