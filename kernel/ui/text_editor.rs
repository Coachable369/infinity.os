//! Allocation-free document state for the installed native Text Editor.

pub const DOCUMENT_CAPACITY: usize = 2048;

#[derive(Clone, Copy)]
pub struct TextDocument {
    bytes: [u8; DOCUMENT_CAPACITY],
    length: usize,
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
    // FUNC: insert
    // DESC: Appends one printable byte or newline and records a document revision.
    // ------------------=
    pub fn insert(&mut self, byte: u8) -> bool {
        if self.length >= DOCUMENT_CAPACITY || (!(32..=126).contains(&byte) && byte != b'\n') {
            return false;
        }
        self.bytes[self.length] = byte;
        self.length += 1;
        self.revision = self.revision.wrapping_add(1);
        true
    }

    // ------------------------=
    // FUNC: backspace
    // DESC: Removes the last byte when present and records a document revision.
    // ------------------=
    pub fn backspace(&mut self) -> bool {
        if self.length == 0 {
            return false;
        }
        self.length -= 1;
        self.revision = self.revision.wrapping_add(1);
        true
    }

    // ------------------------=
    // FUNC: clear
    // DESC: Starts a new empty saved document.
    // ------------------=
    pub fn clear(&mut self) {
        self.length = 0;
        self.saved_length = 0;
        self.revision = self.revision.wrapping_add(1);
        self.saved_revision = self.revision;
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
