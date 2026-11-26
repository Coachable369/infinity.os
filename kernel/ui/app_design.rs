//! Studio-authored application titles; stable runtime identities remain unchanged.
const DESIGNS: &[u8; 1536] = include_bytes!("../../assets/ui-design-kit/default/app-design.bin");

// ------------------------=
// FUNC: title
// DESC: Resolves a bounded authored display title without changing app dispatch identity.
// ------------------=
pub fn title(app: usize, fallback: &'static [u8]) -> &'static [u8] {
    if app >= 12 { return fallback; }
    let start = app * 128;
    if DESIGNS[start + 6] == 0 { return fallback; }
    let length = DESIGNS[start] as usize;
    if length == 0 || length > 96 { return fallback; }
    &DESIGNS[start + 16..start + 16 + length]
}
