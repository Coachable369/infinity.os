//! Shared allocation-free text editing and caret presentation state.

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static CARET_ACTIVE: AtomicBool = AtomicBool::new(false);
static CARET_VISIBLE: AtomicBool = AtomicBool::new(true);
static CARET_INDEX: AtomicUsize = AtomicUsize::new(0);
static CARET_KIND: AtomicUsize = AtomicUsize::new(0);
static POINTER_SHAPE: AtomicUsize = AtomicUsize::new(PointerShape::Default as usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerShape {
    Default = 0,
    Text = 1,
    ResizeNorthWestSouthEast = 2,
    ResizeNorthEastSouthWest = 3,
    ResizeVertical = 4,
    ResizeHorizontal = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextEditKey {
    Character(u8),
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
}

#[path="text_paste.rs"]
mod text_paste;
pub use text_paste::paste_ascii;

// ------------------------=
// FUNC: insert_ascii
// DESC: Inserts one printable ASCII byte at a bounded caret and shifts trailing text right.
// ------------------=
pub fn insert_ascii<const N: usize>(
    bytes: &mut [u8; N],
    length: &mut usize,
    caret: &mut usize,
    value: u8,
) -> bool {
    if !(b' '..=b'~').contains(&value) || *length >= N {
        return false;
    }
    *caret = (*caret).min(*length);
    bytes.copy_within(*caret..*length, *caret + 1);
    bytes[*caret] = value;
    *length += 1;
    *caret += 1;
    true
}

// ------------------------=
// FUNC: backspace
// DESC: Deletes the byte immediately before a bounded caret and shifts trailing text left.
// ------------------=
pub fn backspace<const N: usize>(
    bytes: &mut [u8; N],
    length: &mut usize,
    caret: &mut usize,
) -> bool {
    *caret = (*caret).min(*length);
    if *caret == 0 {
        return false;
    }
    bytes.copy_within(*caret..*length, *caret - 1);
    *length -= 1;
    *caret -= 1;
    bytes[*length] = 0;
    true
}

// ------------------------=
// FUNC: delete
// DESC: Deletes the byte under a bounded caret and retains the caret position.
// ------------------=
pub fn delete<const N: usize>(bytes: &mut [u8; N], length: &mut usize, caret: &mut usize) -> bool {
    *caret = (*caret).min(*length);
    if *caret >= *length {
        return false;
    }
    bytes.copy_within(*caret + 1..*length, *caret);
    *length -= 1;
    bytes[*length] = 0;
    true
}

// ------------------------=
// FUNC: move_caret
// DESC: Applies standard left, right, home, and end movement to a bounded caret.
// ------------------=
pub fn move_caret(caret: &mut usize, length: usize, movement: i8) -> bool {
    let previous = (*caret).min(length);
    *caret = match movement {
        -2 => 0,
        -1 => previous.saturating_sub(1),
        1 => previous.saturating_add(1).min(length),
        2 => length,
        _ => previous,
    };
    *caret != previous
}

// ------------------------=
// FUNC: caret_from_x
// DESC: Converts a click offset and fixed glyph advance into the nearest bounded insertion index.
// ------------------=
pub fn caret_from_x(offset: i32, glyph_advance: usize, length: usize) -> usize {
    if offset <= 0 || glyph_advance == 0 {
        return 0;
    }
    ((offset as usize + glyph_advance / 2) / glyph_advance).min(length)
}

// ------------------------=
// FUNC: set_presentation
// DESC: Publishes the active field caret and pointer shape for the framebuffer renderer.
// ------------------=
pub fn set_presentation(
    active: bool,
    visible: bool,
    index: usize,
    kind: usize,
    pointer_text: bool,
) {
    CARET_ACTIVE.store(active, Ordering::Relaxed);
    CARET_VISIBLE.store(visible, Ordering::Relaxed);
    CARET_INDEX.store(index, Ordering::Relaxed);
    CARET_KIND.store(kind, Ordering::Relaxed);
    POINTER_SHAPE.store(
        if pointer_text {
            PointerShape::Text
        } else {
            PointerShape::Default
        } as usize,
        Ordering::Relaxed,
    );
}

// ------------------------=
// FUNC: caret
// DESC: Returns the active caret visibility and insertion index for the current frame.
// ------------------=
pub fn caret(kind: usize) -> Option<(bool, usize)> {
    (CARET_ACTIVE.load(Ordering::Relaxed) && CARET_KIND.load(Ordering::Relaxed) == kind).then(
        || {
            (
                CARET_VISIBLE.load(Ordering::Relaxed),
                CARET_INDEX.load(Ordering::Relaxed),
            )
        },
    )
}

// ------------------------=
// FUNC: pointer_is_text
// DESC: Reports whether the pointer should render as an I-beam over editable text.
// ------------------=
pub fn pointer_is_text() -> bool {
    pointer_shape() == PointerShape::Text
}

// ------------------------=
// FUNC: set_pointer_shape
// DESC: Publishes a directional pointer shape for window-edge hover and captured resize operations.
// ------------------=
pub fn set_pointer_shape(shape: PointerShape) {
    POINTER_SHAPE.store(shape as usize, Ordering::Relaxed);
}

// ------------------------=
// FUNC: pointer_shape
// DESC: Returns the pointer shape currently requested by the active interaction surface.
// ------------------=
pub fn pointer_shape() -> PointerShape {
    match POINTER_SHAPE.load(Ordering::Relaxed) {
        1 => PointerShape::Text,
        2 => PointerShape::ResizeNorthWestSouthEast,
        3 => PointerShape::ResizeNorthEastSouthWest,
        4 => PointerShape::ResizeVertical,
        5 => PointerShape::ResizeHorizontal,
        _ => PointerShape::Default,
    }
}

// ------------------------=
// FUNC: pointer_shape_for_resize_handle
// DESC: Maps the shared window resize handle contract to its matching directional pointer.
// ------------------=
pub const fn pointer_shape_for_resize_handle(handle: usize) -> Option<PointerShape> {
    match handle {
        0 | 3 => Some(PointerShape::ResizeNorthWestSouthEast),
        1 | 2 => Some(PointerShape::ResizeNorthEastSouthWest),
        4 => Some(PointerShape::ResizeVertical),
        5 | 6 => Some(PointerShape::ResizeHorizontal),
        _ => None,
    }
}

// ------------------------=
// FUNC: presentation_hash
// DESC: Exposes caret presentation changes to bounded redraw detection without hashing secret text.
// ------------------=
pub fn presentation_hash() -> u32 {
    (CARET_ACTIVE.load(Ordering::Relaxed) as u32)
        | ((CARET_VISIBLE.load(Ordering::Relaxed) as u32) << 1)
        | ((CARET_INDEX.load(Ordering::Relaxed) as u32) << 2)
            ^ ((CARET_KIND.load(Ordering::Relaxed) as u32) << 24)
            ^ ((POINTER_SHAPE.load(Ordering::Relaxed) as u32) << 29)
}
