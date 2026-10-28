//! Immutable native faces: no host font service, allocation or decoding during paint.
use core::sync::atomic::{AtomicU8, Ordering};
static SELECTED: AtomicU8 = AtomicU8::new(2);
const ATLAS: usize = 24 * 28 * 95;
const RECORD: usize = ATLAS + 95 + 95 * 95;
const DATA: &[u8; RECORD * 46 * 2] = include_bytes!("../../assets/fonts/InfinityUI-library.bin");

// ------------------------=
// FUNC: publish
// DESC: Switches a validated installed face atomically; invalid IDs always use the immutable fallback.
// ------------------=
pub fn publish(id: u8) {
    SELECTED.store(
        if (1..=46).contains(&id) { id } else { 2 },
        Ordering::Release,
    );
}
// ------------------------=
// FUNC: selected
// DESC: Reads the native UI family's stable ID without locking the runtime during painting.
// ------------------=
pub fn selected() -> u8 {
    SELECTED.load(Ordering::Acquire)
}
// ------------------------=
// FUNC: face
// DESC: Returns matching glyph pixels, advance metrics and kerning for a single family and weight.
// ------------------=
pub fn face(id: u8, bold: bool) -> (&'static [u8], &'static [u8], &'static [u8]) {
    let id = if (1..=46).contains(&id) { id } else { 2 };
    let start = ((id as usize - 1) * 2 + usize::from(bold)) * RECORD;
    let record = &DATA[start..start + RECORD];
    (
        &record[..ATLAS],
        &record[ATLAS..ATLAS + 95],
        &record[ATLAS + 95..],
    )
}
