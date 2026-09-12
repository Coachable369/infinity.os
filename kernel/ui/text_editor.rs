//! Allocation-free document state for the installed native Text Editor.

pub const DOCUMENT_CAPACITY: usize = 16 * 1024;
pub const DOCUMENT_NAME_CAPACITY: usize = 47;
pub const DOCUMENT_PATH_CAPACITY: usize = 95;
pub const DOCUMENT_NAMESPACE: &[u8] = b"/personal/documents/";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PendingDocumentAction {
    None,
    Close,
    New,
    Open,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsavedDecision {
    Cancel,
    Discard,
    Save,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsavedTransition {
    Stay,
    Perform(PendingDocumentAction),
    Save(PendingDocumentAction),
    SaveAs(PendingDocumentAction),
}

// ------------------------=
// FUNC: resolve_unsaved_decision
// DESC: Resolves a dirty-document choice without allowing a close, replacement, or open to bypass Save, Discard, or Cancel.
// ------------------=
pub const fn resolve_unsaved_decision(
    pending: PendingDocumentAction,
    decision: UnsavedDecision,
    has_path: bool,
) -> UnsavedTransition {
    match decision {
        UnsavedDecision::Cancel => UnsavedTransition::Stay,
        UnsavedDecision::Discard => UnsavedTransition::Perform(pending),
        UnsavedDecision::Save if has_path => UnsavedTransition::Save(pending),
        UnsavedDecision::Save => UnsavedTransition::SaveAs(pending),
    }
}

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
    anchor: usize,
    undo: [Snapshot; 8],
    redo: [Snapshot; 8],
    undo_count: usize,
    redo_count: usize,
}

#[derive(Clone, Copy)]
struct Snapshot {
    bytes: [u8; DOCUMENT_CAPACITY],
    length: usize,
    cursor: usize,
    revision: u32,
}
const EMPTY_SNAPSHOT: Snapshot = Snapshot {
    bytes: [0; DOCUMENT_CAPACITY],
    length: 0,
    cursor: 0,
    revision: 0,
};

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
            anchor: 0,
            undo: [EMPTY_SNAPSHOT; 8],
            redo: [EMPTY_SNAPSHOT; 8],
            undo_count: 0,
            redo_count: 0,
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
        self.anchor = self.cursor;
    }

    // ------------------------=
    // FUNC: insert
    // DESC: Appends one printable byte or newline and records a document revision.
    // ------------------=
    pub fn insert(&mut self, byte: u8) -> bool {
        self.replace_selection(&[byte])
    }

    // ------------------------=
    // FUNC: backspace
    // DESC: Removes the last byte when present and records a document revision.
    // ------------------=
    pub fn backspace(&mut self) -> bool {
        if self.selection().is_some() {
            return self.replace_selection(b"");
        }
        self.cursor = self.cursor.min(self.length);
        if self.cursor == 0 {
            return false;
        }
        self.anchor = self.cursor - 1;
        self.replace_selection(b"")
    }

    // ------------------------=
    // FUNC: delete
    // DESC: Deletes the document byte under the caret and preserves its insertion position.
    // ------------------=
    pub fn delete(&mut self) -> bool {
        if self.selection().is_some() {
            return self.replace_selection(b"");
        }
        if self.cursor >= self.length {
            return false;
        }
        self.anchor = self.cursor + 1;
        self.replace_selection(b"")
    }

    // ------------------------=
    // FUNC: move_cursor
    // DESC: Applies standard horizontal, home, and end navigation to the document caret.
    // ------------------=
    pub fn move_cursor(&mut self, movement: i8) -> bool {
        let changed = crate::ui::text_input::move_caret(&mut self.cursor, self.length, movement);
        self.anchor = self.cursor;
        changed
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
        self.anchor = self.cursor;
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
        self.anchor = self.cursor;
        self.cursor != original
    }

    // ------------------------=
    // FUNC: clear
    // DESC: Starts a new empty saved document.
    // ------------------=
    pub fn clear(&mut self) {
        self.anchor = 0;
        self.undo_count = 0;
        self.redo_count = 0;
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
                .any(|byte| !(32..=126).contains(byte) && !matches!(*byte, b'\n' | b'\t' | b'\r'))
        {
            return false;
        }
        self.bytes[..content.len()].copy_from_slice(content);
        self.length = content.len();
        self.cursor = content.len();
        self.anchor = self.cursor;
        self.undo_count = 0;
        self.redo_count = 0;
        self.saved_length = content.len();
        self.revision = self.revision.wrapping_add(1);
        self.saved_revision = self.revision;
        true
    }

    // ------------------------=
    // FUNC: selection
    // DESC: Returns the half-open selected byte interval.
    // ------------------=
    pub fn selection(&self) -> Option<(usize, usize)> {
        (self.anchor != self.cursor)
            .then_some((self.anchor.min(self.cursor), self.anchor.max(self.cursor)))
    }
    // ------------------------=
    // FUNC: select
    // DESC: Extends an explicit bounded selection for keyboard, pointer, search, or app actions.
    // ------------------=
    pub fn select(&mut self, anchor: usize, cursor: usize) {
        self.anchor = anchor.min(self.length);
        self.cursor = cursor.min(self.length);
    }
    // ------------------------=
    // FUNC: snapshot
    // DESC: Captures an undoable document revision, not its external saved checkpoint.
    // ------------------=
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            bytes: self.bytes,
            length: self.length,
            cursor: self.cursor,
            revision: self.revision,
        }
    }
    // ------------------------=
    // FUNC: restore
    // DESC: Restores one history entry without changing the saved checkpoint.
    // ------------------=
    fn restore(&mut self, state: Snapshot) {
        self.bytes = state.bytes;
        self.length = state.length;
        self.cursor = state.cursor;
        self.anchor = state.cursor;
        self.revision = state.revision;
    }
    // ------------------------=
    // FUNC: checkpoint
    // DESC: Keeps the newest eight complete atomic edits and invalidates redo on a branch.
    // ------------------=
    fn checkpoint(&mut self) {
        if self.undo_count == self.undo.len() {
            self.undo.copy_within(1.., 0);
            self.undo_count -= 1;
        }
        self.undo[self.undo_count] = self.snapshot();
        self.undo_count += 1;
        self.redo_count = 0;
    }
    // ------------------------=
    // FUNC: undo
    // DESC: Reverses one atomic edit, including a paste or replace-all.
    // ------------------=
    pub fn undo(&mut self) -> bool {
        if self.undo_count == 0 {
            return false;
        }
        self.redo[self.redo_count] = self.snapshot();
        self.redo_count += 1;
        self.undo_count -= 1;
        self.restore(self.undo[self.undo_count]);
        true
    }
    // ------------------------=
    // FUNC: redo
    // DESC: Reapplies an undone edit without generating an extra history entry.
    // ------------------=
    pub fn redo(&mut self) -> bool {
        if self.redo_count == 0 {
            return false;
        }
        self.undo[self.undo_count] = self.snapshot();
        self.undo_count += 1;
        self.redo_count -= 1;
        self.restore(self.redo[self.redo_count]);
        true
    }
    // ------------------------=
    // FUNC: replace_selection
    // DESC: Validates a complete edit before committing; capacity errors never remove selected content.
    // ------------------=
    pub fn replace_selection(&mut self, content: &[u8]) -> bool {
        let (start, end) = self.selection().unwrap_or((self.cursor, self.cursor));
        if self.length - (end - start) + content.len() > DOCUMENT_CAPACITY
            || content
                .iter()
                .any(|b| !(32..=126).contains(b) && !matches!(*b, b'\n' | b'\t' | b'\r'))
        {
            return false;
        }
        if start == end && content.is_empty() {
            return false;
        }
        self.checkpoint();
        self.bytes
            .copy_within(end..self.length, start + content.len());
        self.bytes[start..start + content.len()].copy_from_slice(content);
        self.length = self.length - (end - start) + content.len();
        self.cursor = start + content.len();
        self.anchor = self.cursor;
        self.revision = self.revision.max(self.saved_revision).wrapping_add(1);
        true
    }
    // ------------------------=
    // FUNC: find
    // DESC: Selects the next literal match with wrapping and optional ASCII case folding.
    // ------------------=
    pub fn find(&mut self, needle: &[u8], case_sensitive: bool) -> bool {
        if needle.is_empty() || needle.len() > self.length {
            return false;
        }
        let max = self.length - needle.len() + 1;
        for step in 0..max {
            let i = (self.cursor + step) % max;
            let part = &self.bytes[i..i + needle.len()];
            if if case_sensitive {
                part == needle
            } else {
                part.eq_ignore_ascii_case(needle)
            } {
                self.select(i, i + needle.len());
                return true;
            }
        }
        false
    }
    // ------------------------=
    // FUNC: replace_all
    // DESC: Replaces non-overlapping literal matches as one undoable transaction without partial overflow edits.
    // ------------------=
    pub fn replace_all(&mut self, needle: &[u8], replacement: &[u8]) -> Option<usize> {
        if needle.is_empty() {
            return None;
        }
        let mut out = [0; DOCUMENT_CAPACITY];
        let mut n = 0;
        let mut i = 0;
        let mut count = 0;
        while i < self.length {
            let matched = self.bytes[i..self.length].starts_with(needle);
            let next = if matched {
                replacement
            } else {
                &self.bytes[i..i + 1]
            };
            if n + next.len() > out.len() {
                return None;
            }
            out[n..n + next.len()].copy_from_slice(next);
            n += next.len();
            i += if matched {
                count += 1;
                needle.len()
            } else {
                1
            };
        }
        if count > 0 {
            let old = (self.anchor, self.cursor);
            self.select(0, self.length);
            if !self.replace_selection(&out[..n]) {
                self.select(old.0, old.1);
                return None;
            }
        }
        Some(count)
    }
    // ------------------------=
    // FUNC: newline_indented
    // DESC: Inserts a newline with the current line's indentation in one undo step.
    // ------------------=
    pub fn newline_indented(&mut self) -> bool {
        let start = self.bytes[..self.cursor]
            .iter()
            .rposition(|b| *b == b'\n')
            .map_or(0, |i| i + 1);
        let mut data = [b' '; 129];
        data[0] = b'\n';
        let mut n = 1;
        for b in &self.bytes[start..self.cursor] {
            if !matches!(*b, b' ' | b'\t') || n == data.len() {
                break;
            }
            data[n] = *b;
            n += 1;
        }
        self.replace_selection(&data[..n])
    }
    // ------------------------=
    // FUNC: outdent_line
    // DESC: Removes one leading four-space or tab indentation level from the caret's current line as an undoable edit.
    // ------------------=
    pub fn outdent_line(&mut self) -> bool {
        let cursor = self.cursor.min(self.length);
        let start = self.bytes[..cursor]
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1);
        let amount = if self.bytes.get(start) == Some(&b'\t') {
            1
        } else {
            self.bytes[start..self.length]
                .iter()
                .take(4)
                .take_while(|byte| **byte == b' ')
                .count()
        };
        if amount == 0 {
            return false;
        }
        let next_cursor = cursor.saturating_sub(amount);
        self.select(start, start + amount);
        if !self.replace_selection(b"") {
            return false;
        }
        self.set_cursor(next_cursor);
        true
    }
    // ------------------------=
    // FUNC: goto_line
    // DESC: Moves to a one-based logical line, clamped to the document end.
    // ------------------=
    pub fn goto_line(&mut self, line: usize) {
        let mut at = 0;
        for _ in 1..line {
            match self.bytes[at..self.length].iter().position(|b| *b == b'\n') {
                Some(i) => at += i + 1,
                None => {
                    at = self.length;
                    break;
                }
            }
        }
        self.set_cursor(at);
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
