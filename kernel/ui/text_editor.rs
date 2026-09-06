//! Allocation-free document state for the installed native Text Editor.

pub const DOCUMENT_CAPACITY: usize = 2048;
pub const DOCUMENT_NAME_CAPACITY: usize = 47;
pub const DOCUMENT_PATH_CAPACITY: usize = 95;
pub const DOCUMENT_NAMESPACE: &[u8] = b"/personal/documents/";

// ------------------------=
// FUNC: document_path
// DESC: Builds a bounded human-namespace reference for one native Text object without making the path its identity.
// ------------------=
pub fn document_path(name: &[u8], out: &mut [u8; DOCUMENT_PATH_CAPACITY]) -> Option<usize> {
    if name.is_empty()
        || name.len() > DOCUMENT_NAME_CAPACITY
        || DOCUMENT_NAMESPACE.len().saturating_add(name.len()) > out.len()
        || name
            .iter()
            .any(|byte| *byte == b'/' || !(32..=126).contains(byte))
    {
        return None;
    }
    let length = DOCUMENT_NAMESPACE.len() + name.len();
    out[..DOCUMENT_NAMESPACE.len()].copy_from_slice(DOCUMENT_NAMESPACE);
    out[DOCUMENT_NAMESPACE.len()..length].copy_from_slice(name);
    Some(length)
}

// ------------------------=
// FUNC: visual_line_count
// DESC: Counts wrapped visual rows for a bounded document at the current editor column width.
// ------------------=
pub fn visual_line_count(content: &[u8], columns: usize) -> usize {
    let columns = columns.max(1);
    if content.is_empty() {
        return 1;
    }
    let mut rows = 1usize;
    let mut column = 0usize;
    for byte in content {
        if *byte == b'\n' {
            rows = rows.saturating_add(1);
            column = 0;
        } else {
            column += 1;
            if column == columns {
                rows = rows.saturating_add(1);
                column = 0;
            }
        }
    }
    rows
}

// ------------------------=
// FUNC: visual_line_start
// DESC: Resolves a wrapped visual row to its first byte so rendering can scroll without copying document content.
// ------------------=
pub fn visual_line_start(content: &[u8], columns: usize, target_row: usize) -> usize {
    if target_row == 0 {
        return 0;
    }
    let columns = columns.max(1);
    let mut row = 0usize;
    let mut column = 0usize;
    for (index, byte) in content.iter().enumerate() {
        if *byte == b'\n' {
            row += 1;
            column = 0;
        } else {
            column += 1;
            if column == columns {
                row += 1;
                column = 0;
            }
        }
        if row == target_row {
            return index + 1;
        }
    }
    content.len()
}

// ------------------------=
// FUNC: visual_cursor_row
// DESC: Resolves a bounded insertion index to its wrapped visual row.
// ------------------=
pub fn visual_cursor_row(content: &[u8], columns: usize, cursor: usize) -> usize {
    visual_line_count(&content[..cursor.min(content.len())], columns).saturating_sub(1)
}

#[derive(Clone, Copy)]
pub struct TextDocument {
    bytes: [u8; DOCUMENT_CAPACITY],
    length: usize,
    cursor: usize,
    saved_length: usize,
    revision: u32,
    saved_revision: u32,
}

impl TextDocument {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty saved native text document.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            bytes: [0; DOCUMENT_CAPACITY],
            length: 0,
            cursor: 0,
            saved_length: 0,
            revision: 0,
            saved_revision: 0,
        }
    }

    // ------------------------=
    // FUNC: bytes
    // DESC: Returns the current bounded ASCII document projection.
    // ------------------=
    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..self.length]
    }

    // ------------------------=
    // FUNC: cursor
    // DESC: Returns the current bounded document insertion position.
    // ------------------=
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    // ------------------------=
    // FUNC: set_cursor
    // DESC: Places the document caret at one bounded pointer-derived byte index.
    // ------------------=
    pub fn set_cursor(&mut self, index: usize) {
        self.cursor = index.min(self.length);
    }

    // ------------------------=
    // FUNC: insert
    // DESC: Appends one printable byte or newline and records a document revision.
    // ------------------=
    pub fn insert(&mut self, byte: u8) -> bool {
        if self.length >= DOCUMENT_CAPACITY || (!(32..=126).contains(&byte) && byte != b'\n') {
            return false;
        }
        self.cursor = self.cursor.min(self.length);
        self.bytes
            .copy_within(self.cursor..self.length, self.cursor + 1);
        self.bytes[self.cursor] = byte;
        self.length += 1;
        self.cursor += 1;
        self.revision = self.revision.wrapping_add(1);
        true
    }

    // ------------------------=
    // FUNC: backspace
    // DESC: Removes the last byte when present and records a document revision.
    // ------------------=
    pub fn backspace(&mut self) -> bool {
        self.cursor = self.cursor.min(self.length);
        if self.cursor == 0 {
            return false;
        }
        self.bytes
            .copy_within(self.cursor..self.length, self.cursor - 1);
        self.length -= 1;
        self.cursor -= 1;
        self.bytes[self.length] = 0;
        self.revision = self.revision.wrapping_add(1);
        true
    }

    // ------------------------=
    // FUNC: delete
    // DESC: Deletes the document byte under the caret and preserves its insertion position.
    // ------------------=
    pub fn delete(&mut self) -> bool {
        if self.cursor >= self.length {
            return false;
        }
        self.bytes
            .copy_within(self.cursor + 1..self.length, self.cursor);
        self.length -= 1;
        self.bytes[self.length] = 0;
        self.revision = self.revision.wrapping_add(1);
        true
    }

    // ------------------------=
    // FUNC: move_cursor
    // DESC: Applies standard horizontal, home, and end navigation to the document caret.
    // ------------------=
    pub fn move_cursor(&mut self, movement: i8) -> bool {
        crate::ui::text_input::move_caret(&mut self.cursor, self.length, movement)
    }

    // ------------------------=
    // FUNC: move_cursor_to_line_edge
    // DESC: Moves the document caret to the beginning or end of its current explicit line.
    // ------------------=
    pub fn move_cursor_to_line_edge(&mut self, end: bool) -> bool {
        let original = self.cursor.min(self.length);
        self.cursor = if end {
            self.bytes[original..self.length]
                .iter()
                .position(|byte| *byte == b'\n')
                .map(|offset| original + offset)
                .unwrap_or(self.length)
        } else {
            self.bytes[..original]
                .iter()
                .rposition(|byte| *byte == b'\n')
                .map(|index| index + 1)
                .unwrap_or(0)
        };
        self.cursor != original
    }

    // ------------------------=
    // FUNC: move_cursor_vertical
    // DESC: Moves the caret to the nearest column on the preceding or following explicit text line.
    // ------------------=
    pub fn move_cursor_vertical(&mut self, previous: bool) -> bool {
        let original = self.cursor.min(self.length);
        let line_start = self.bytes[..original]
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        let column = original - line_start;
        if previous {
            if line_start == 0 {
                return false;
            }
            let prior_end = line_start - 1;
            let prior_start = self.bytes[..prior_end]
                .iter()
                .rposition(|byte| *byte == b'\n')
                .map(|index| index + 1)
                .unwrap_or(0);
            self.cursor = (prior_start + column).min(prior_end);
        } else {
            let Some(relative_end) = self.bytes[original..self.length]
                .iter()
                .position(|byte| *byte == b'\n')
            else {
                return false;
            };
            let next_start = original + relative_end + 1;
            let next_end = self.bytes[next_start..self.length]
                .iter()
                .position(|byte| *byte == b'\n')
                .map(|offset| next_start + offset)
                .unwrap_or(self.length);
            self.cursor = (next_start + column).min(next_end);
        }
        self.cursor != original
    }

    // ------------------------=
    // FUNC: clear
    // DESC: Starts a new empty saved document.
    // ------------------=
    pub fn clear(&mut self) {
        self.length = 0;
        self.cursor = 0;
        self.saved_length = 0;
        self.revision = self.revision.wrapping_add(1);
        self.saved_revision = self.revision;
    }

    // ------------------------=
    // FUNC: open
    // DESC: Loads bounded persisted document bytes as the current saved revision.
    // ------------------=
    pub fn open(&mut self, content: &[u8]) -> bool {
        if content.len() > DOCUMENT_CAPACITY
            || content
                .iter()
                .any(|byte| !(32..=126).contains(byte) && *byte != b'\n')
        {
            return false;
        }
        self.bytes[..content.len()].copy_from_slice(content);
        self.length = content.len();
        self.cursor = content.len();
        self.saved_length = content.len();
        self.revision = self.revision.wrapping_add(1);
        self.saved_revision = self.revision;
        true
    }

    // ------------------------=
    // FUNC: save
    // DESC: Commits the current in-session document revision.
    // ------------------=
    pub fn save(&mut self) {
        self.saved_length = self.length;
        self.saved_revision = self.revision;
    }

    // ------------------------=
    // FUNC: is_saved
    // DESC: Reports whether content and revision match the last explicit save.
    // ------------------=
    pub const fn is_saved(&self) -> bool {
        self.length == self.saved_length && self.revision == self.saved_revision
    }
}

impl Default for TextDocument {
    // ------------------------=
    // FUNC: default
    // DESC: Returns the empty native document default.
    // ------------------=
    fn default() -> Self {
        Self::new()
    }
}
