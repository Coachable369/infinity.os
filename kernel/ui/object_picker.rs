//! Bounded Pool namespace browser shared by editor input and presentation.
use super::geometry::Rect;
pub const PATH: usize = 95;
pub const LIMIT: usize = 64;
pub struct Geometry {
    pub sheet: Rect,
    pub name: Rect,
    pub location: Rect,
    pub parent: Rect,
    pub list: Rect,
    pub rows: usize,
    pub row_height: usize,
    pub previous: Rect,
    pub next: Rect,
    pub cancel: Rect,
    pub accept: Rect,
}
// ------------------------=
// FUNC: geometry
// DESC: Shares bounded native sheet, field, paging and action geometry between painting and hit testing.
// ------------------=
pub fn geometry(content: Rect, scale: usize, save: bool) -> Geometry {
    let width = (620 * scale).min((content.width as usize).saturating_sub(24 * scale));
    let height = (450 * scale).min((content.height as usize).saturating_sub(16 * scale));
    let x = content.x + ((content.width as usize - width) / 2) as i32;
    let y = content.y + ((content.height as usize - height) / 2) as i32;
    let rect = |dx: usize, dy: usize, w: usize, h: usize| Rect {
        x: x + dx as i32,
        y: y + dy as i32,
        width: w as u32,
        height: h as u32,
    };
    let inset = 24 * scale;
    let inner = width.saturating_sub(2 * inset);
    let list_y = if save { 160 * scale } else { 108 * scale };
    let row_height = 30 * scale;
    let rows = height
        .saturating_sub(list_y + 124 * scale)
        .checked_div(row_height)
        .unwrap_or(0)
        .clamp(1, 6);
    let actions = height.saturating_sub(48 * scale);
    let half = inner.saturating_sub(12 * scale) / 2;
    Geometry {
        sheet: rect(0, 0, width, height),
        name: rect(inset, 54 * scale, inner, 36 * scale),
        location: rect(
            inset,
            if save { 106 * scale } else { 54 * scale },
            inner.saturating_sub(56 * scale),
            36 * scale,
        ),
        parent: rect(
            width.saturating_sub(inset + 48 * scale),
            if save { 106 * scale } else { 54 * scale },
            48 * scale,
            36 * scale,
        ),
        list: rect(inset, list_y, inner, rows * row_height),
        rows,
        row_height,
        previous: rect(
            inset,
            actions.saturating_sub(72 * scale),
            66 * scale,
            32 * scale,
        ),
        next: rect(
            inset + 78 * scale,
            actions.saturating_sub(72 * scale),
            66 * scale,
            32 * scale,
        ),
        cancel: rect(inset, actions, half, 36 * scale),
        accept: rect(inset + half + 12 * scale, actions, half, 36 * scale),
    }
}
#[derive(Clone, Copy)]
pub struct Entry {
    pub path: [u8; PATH],
    pub len: usize,
    pub folder: bool,
}
impl Entry {
    // ------------------------=
    // FUNC: empty
    // DESC: Creates an unused browser entry.
    // ------------------=
    pub const fn empty() -> Self {
        Self {
            path: [0; PATH],
            len: 0,
            folder: false,
        }
    }
    // ------------------------=
    // FUNC: bytes
    // DESC: Reads the native namespace reference, not a rendered label.
    // ------------------=
    pub fn bytes(&self) -> &[u8] {
        &self.path[..self.len]
    }
}
#[derive(Clone, Copy)]
pub struct Picker {
    pub location: Entry,
    pub name: Entry,
    pub entries: [Entry; LIMIT],
    pub count: usize,
    pub selected: usize,
    pub field: u8,
    pub error: u8,
}
impl Picker {
    // ------------------------=
    // FUNC: state_hash
    // DESC: Invalidates picker pixels only when browsing, focus, errors, or names change.
    // ------------------=
    pub fn state_hash(&self) -> u32 {
        let mut hash = self.count as u32
            ^ (self.selected as u32) << 8
            ^ (self.field as u32) << 16
            ^ (self.error as u32) << 24;
        for byte in self.location.bytes().iter().chain(self.name.bytes()) {
            hash = hash.wrapping_mul(16777619) ^ u32::from(*byte);
        }
        hash
    }
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty root browser with bounded storage.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            location: Entry::empty(),
            name: Entry::empty(),
            entries: [Entry::empty(); LIMIT],
            count: 0,
            selected: 0,
            field: 0,
            error: 0,
        }
    }
    // ------------------------=
    // FUNC: set_location
    // DESC: Validates an absolute Pool namespace, rejects traversal, and normalizes trailing separators.
    // ------------------=
    pub fn set_location(&mut self, path: &[u8]) -> bool {
        if path.first() != Some(&b'/')
            || path.len() > PATH
            || path.iter().any(|b| !(32..=126).contains(b))
        {
            return false;
        }
        let end = if path.len() > 1 && path.last() == Some(&b'/') {
            path.len() - 1
        } else {
            path.len()
        };
        if path[1..end]
            .split(|b| *b == b'/')
            .any(|s| s == b"." || s == b".." || (s.is_empty() && end > 1))
        {
            return false;
        }
        self.location.path[..end].copy_from_slice(&path[..end]);
        self.location.len = end;
        self.count = 0;
        self.selected = 0;
        self.error = 0;
        true
    }
    // ------------------------=
    // FUNC: parent
    // DESC: Navigates one namespace level without escaping the Pool root.
    // ------------------=
    pub fn parent(&mut self) {
        let path = self.location;
        let end = path
            .bytes()
            .iter()
            .rposition(|b| *b == b'/')
            .unwrap_or(0)
            .max(1);
        self.set_location(&path.path[..end]);
    }
    // ------------------------=
    // FUNC: add
    // DESC: Projects both explicit and inferred folders from real namespace references, deduplicating direct children.
    // ------------------=
    pub fn add(&mut self, path: &[u8], is_folder: bool) {
        let parent = self.location.bytes();
        if !path.starts_with(parent) || path.len() <= parent.len() {
            return;
        }
        let start = if parent == b"/" {
            1
        } else {
            if path[parent.len()] != b'/' {
                return;
            }
            parent.len() + 1
        };
        if start >= path.len() {
            return;
        }
        let split = path[start..].iter().position(|b| *b == b'/');
        let end = split.map(|n| start + n).unwrap_or(path.len());
        if end > PATH
            || self.entries[..self.count]
                .iter()
                .any(|e| e.bytes() == &path[..end])
        {
            return;
        }
        if self.count == LIMIT {
            self.error = 5;
            return;
        }
        let mut e = Entry::empty();
        e.path[..end].copy_from_slice(&path[..end]);
        e.len = end;
        e.folder = is_folder || split.is_some();
        self.entries[self.count] = e;
        self.count += 1;
    }
    // ------------------------=
    // FUNC: destination
    // DESC: Builds a separate filename and selected location into a safe bounded object reference.
    // ------------------=
    pub fn destination(&self, name: &[u8]) -> Option<Entry> {
        if name.is_empty()
            || name.len() > 47
            || name == b"."
            || name == b".."
            || name.iter().any(|b| *b == b'/' || !(32..=126).contains(b))
        {
            return None;
        }
        let parent = self.location.bytes();
        let slash = usize::from(parent != b"/");
        let len = parent.len() + slash + name.len();
        if parent.is_empty() || len > PATH {
            return None;
        }
        let mut e = Entry::empty();
        e.path[..parent.len()].copy_from_slice(parent);
        if slash == 1 {
            e.path[parent.len()] = b'/';
        }
        e.path[parent.len() + slash..len].copy_from_slice(name);
        e.len = len;
        Some(e)
    }
}
static mut PRESENTATION: Picker = Picker::new();
// ------------------------=
// FUNC: publish
// DESC: Publishes the single UI-thread browser snapshot for native rendering.
// ------------------=
pub fn publish(picker: Picker) {
    unsafe {
        PRESENTATION = picker;
    }
}
// ------------------------=
// FUNC: presentation
// DESC: Reads an immutable copy of the UI-thread browser snapshot.
// ------------------=
pub fn presentation() -> Picker {
    unsafe { *(&raw const PRESENTATION) }
}
