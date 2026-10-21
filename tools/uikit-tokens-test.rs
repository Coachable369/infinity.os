#[path = "../kernel/ui/kit_tokens.rs"]
mod kit_tokens;
#[path = "../kernel/ui/geometry.rs"]
mod geometry;
#[path = "../kernel/ui/skin.rs"]
mod skin;
pub const INFINITY_UI_ABI_VERSION: u16 = 1;

// ------------------------=
// FUNC: main
// DESC: Exercises the bounded token decoder, malformed fallback contract and actual default skin color resolution.
// ------------------=
fn main() {
    let mut bytes = [0u8; kit_tokens::BYTES];
    bytes[..8].copy_from_slice(b"IUIKIT01");
    bytes[8..10].copy_from_slice(&1u16.to_le_bytes());
    bytes[10..12].copy_from_slice(&13u16.to_le_bytes());
    for index in 0..13 {
        bytes[12 + index * 4..16 + index * 4].copy_from_slice(&(0xff102030u32 + index as u32).to_le_bytes());
    }
    let colors = kit_tokens::decode(&bytes).unwrap();
    for index in 0..13 { assert_eq!(colors[index], 0xff102030 + index as u32); }
    assert!(kit_tokens::decode(&bytes[..63]).is_none());
    for at in [0, 7, 8, 9, 10, 11] {
        let mut invalid = bytes;
        invalid[at] ^= 0x80;
        assert!(kit_tokens::decode(&invalid).is_none());
    }
    let actual = skin::default_dark_skin();
    for (index, color) in actual.tokens.colors.iter().enumerate() {
        assert_eq!(color.0, kit_tokens::colors()[index]);
    }
    assert_eq!(skin::SkinRegistry::new().primary_rgb(), kit_tokens::colors()[1] & 0xffffff);
}
