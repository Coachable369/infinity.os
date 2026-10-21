//! Versioned Studio-authored semantic colors shared by both native architectures.

pub const COUNT: usize = 13;
pub const BYTES: usize = 12 + COUNT * 4;
#[used]
static TOKEN_PAYLOAD: [u8; BYTES] = *include_bytes!("../../assets/ui-design-kit/default/uikit-tokens.bin");
const DATA: &[u8; BYTES] = &TOKEN_PAYLOAD;
pub const FALLBACK: [u32; COUNT] = [0xff090f26, 0xe6101b3a, 0xf01a2e63, 0xff86c7ff,
    0xffffffff, 0xffbebebe, 0xff3e6fff, 0xff86c7ff, 0xffffffff, 0xff50d890,
    0xffffc857, 0xffff6170, 0x99000000];

// ------------------------=
// FUNC: decode
// DESC: Validates the complete bounded UIKit record before exposing any authored color.
// ------------------=
pub const fn decode(bytes: &[u8]) -> Option<[u32; COUNT]> {
    if bytes.len() != BYTES { return None; }
    let magic = b"IUIKIT01";
    let mut i = 0;
    while i < magic.len() {
        if bytes[i] != magic[i] { return None; }
        i += 1;
    }
    if bytes[8] != 1 || bytes[9] != 0 || bytes[10] != COUNT as u8 || bytes[11] != 0 {
        return None;
    }
    let mut result = [0; COUNT];
    i = 0;
    while i < COUNT {
        let at = 12 + i * 4;
        result[i] = u32::from_le_bytes([bytes[at], bytes[at+1], bytes[at+2], bytes[at+3]]);
        i += 1;
    }
    Some(result)
}

// ------------------------=
// FUNC: colors
// DESC: Resolves Studio colors with a dependency-free approved fallback for malformed records.
// ------------------=
pub const fn colors() -> [u32; COUNT] {
    match decode(DATA) { Some(colors) => colors, None => FALLBACK }
}

// ------------------------=
// FUNC: runtime_colors
// DESC: Reads the same native semantic palette while permitting a validated unsaved payload in the Studio host renderer.
// ------------------=
pub fn runtime_colors()->[u32;COUNT] {
    #[cfg(studio_native_preview)]
    if let Some(bytes)=crate::studio_native_render::asset_override(b"uikit-tokens.bin") {
        if let Some(colors)=decode(bytes) {return colors;}
    }
    colors()
}

// ------------------------=
// FUNC: primary_rgb
// DESC: Uses the authored panel color as the shared desktop primary tint until the user overrides it.
// ------------------=
pub const fn primary_rgb() -> u32 { colors()[1] & 0x00ff_ffff }
