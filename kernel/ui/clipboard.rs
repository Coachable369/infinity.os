//! Capability-gated typed clipboard and drag payload foundation.

pub const MAX_CLIPBOARD_BYTES: usize = 4096;
pub const MAX_OBJECT_REFS: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClipboardKind {
    Empty,
    Utf8Text,
    ObjectRefs,
    ImageObjectRef,
}

#[derive(Clone, Copy)]
pub struct ClipboardPayload {
    pub kind: ClipboardKind,
    pub bytes: [u8; MAX_CLIPBOARD_BYTES],
    pub length: u16,
    pub source_context: u32,
    pub generation: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClipboardError {
    AccessDenied,
    TooLarge,
    InvalidType,
}

pub struct ClipboardService {
    payload: ClipboardPayload,
}

impl ClipboardService {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty typed clipboard without ambient read or write authority.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            payload: ClipboardPayload {
                kind: ClipboardKind::Empty,
                bytes: [0; MAX_CLIPBOARD_BYTES],
                length: 0,
                source_context: 0,
                generation: 0,
            },
        }
    }

    // ------------------------=
    // FUNC: write
    // DESC: Replaces clipboard content only after an external capability decision authorizes the caller.
    // ------------------=
    pub fn write(
        &mut self,
        authorized: bool,
        caller: u32,
        kind: ClipboardKind,
        bytes: &[u8],
    ) -> Result<u32, ClipboardError> {
        if !authorized {
            return Err(ClipboardError::AccessDenied);
        }
        if bytes.len() > MAX_CLIPBOARD_BYTES {
            return Err(ClipboardError::TooLarge);
        }
        if kind == ClipboardKind::Empty && !bytes.is_empty() {
            return Err(ClipboardError::InvalidType);
        }
        self.payload.bytes.fill(0);
        self.payload.bytes[..bytes.len()].copy_from_slice(bytes);
        self.payload.length = bytes.len() as u16;
        self.payload.kind = kind;
        self.payload.source_context = caller;
        self.payload.generation = self.payload.generation.wrapping_add(1);
        Ok(self.payload.generation)
    }

    // ------------------------=
    // FUNC: read
    // DESC: Returns typed clipboard metadata and content only to an authorized caller.
    // ------------------=
    pub fn read(&self, authorized: bool) -> Result<&ClipboardPayload, ClipboardError> {
        if authorized {
            Ok(&self.payload)
        } else {
            Err(ClipboardError::AccessDenied)
        }
    }
}
