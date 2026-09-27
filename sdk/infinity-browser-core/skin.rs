//! Infinity Browser sapphire skin. RGB colors are opaque recipe values; the
//! compositor owns any cached glass backing. Never tint the engine's web pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Surface {
    pub top: u32,
    pub bottom: u32,
    pub border: u32,
    pub text: u32,
}
pub const CHROME: Surface = Surface {
    top: 0x0c2945, bottom: 0x071a2e, border: 0x407c9f, text: 0xe7f2fa,
};
pub const ADDRESS: Surface = Surface {
    top: 0x041326, bottom: 0x061b30, border: 0x366887, text: 0xe7f2fa,
};
pub const ERROR: Surface = Surface {
    top: 0x392031, bottom: 0x231723, border: 0xb45c70, text: 0xffe9ee,
};
pub const FOCUS: u32 = 0x00d7ff;
pub const SECONDARY_TEXT: u32 = 0xa8c4d8;
pub const CONTENT_CLEAR: u32 = 0xf7f8fa;
pub const GUTTER: u32 = 16;
pub const GAP: u32 = 8;
pub const CONTROL_HEIGHT: u32 = 44;
pub const CORNER_RADIUS: u32 = 10;
pub const APP_ICON: &str = "infinity-browser-icon-v1-source.png";
pub const NAVIGATION_ATLAS: &str = "infinity-browser-navigation-v1-source.png";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Glyph { Back, Forward, Reload, Stop, Download, SiteControls, Menu, Go }
impl Glyph {
    // ------------------------=
    // FUNC: source
    // DESC: Selects lossless atlas cells with integer edges, including odd source dimensions.
    // ------------------=
    pub fn source(self, width: u32, height: u32) -> Option<crate::Viewport> {
        if width < 4 || height < 2 || width > i32::MAX as u32 || height > i32::MAX as u32 {
            return None;
        }
        let index = self as u32;
        let column = index % 4;
        let row = index / 4;
        let x = width as u64 * column as u64 / 4;
        let right = width as u64 * (column + 1) as u64 / 4;
        let y = height as u64 * row as u64 / 2;
        let bottom = height as u64 * (row + 1) as u64 / 2;
        Some(crate::Viewport { x: x as i32, y: y as i32,
            width: (right - x) as u32, height: (bottom - y) as u32 })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interaction { Normal, Hovered, Pressed, Disabled }
// ------------------------=
// FUNC: button
// DESC: Returns distinct high-contrast primary or secondary control recipes without per-frame allocation.
// ------------------=
pub const fn button(primary: bool, state: Interaction) -> Surface {
    if matches!(state, Interaction::Disabled) {
        return Surface { top: 0x263d52, bottom: 0x213549, border: 0x3b5265, text: 0x91a4b5 };
    }
    let (top, bottom, border) = match (primary, state) {
        (true, Interaction::Normal) => (0x09ceee, 0x00ade0, 0x69e7ff),
        (true, Interaction::Hovered) => (0x39dcf6, 0x06bfee, 0x9bf0ff),
        (true, _) => (0x009fc8, 0x0086ba, 0x32d8f5),
        (false, Interaction::Normal) => (0x0b2742, 0x06192e, 0x345b79),
        (false, Interaction::Hovered) => (0x143b58, 0x0a2842, 0x559abe),
        (false, _) => (0x061629, 0x0a2238, 0x467b9a),
    };
    Surface { top, bottom, border, text: if primary { 0x031b2c } else { 0xe7f2fa } }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: atlas_cells_cover_source_without_overlap
    // DESC: Verifies all eight glyph slices partition both even and odd sized source images.
    // ------------------=
    #[test]
    fn atlas_cells_cover_source_without_overlap() {
        for (width, height) in [(1774, 887), (2048, 1024)] {
            let mut area = 0;
            let glyphs = [Glyph::Back, Glyph::Forward, Glyph::Reload, Glyph::Stop,
                Glyph::Download, Glyph::SiteControls, Glyph::Menu, Glyph::Go];
            for glyph in glyphs {
                let cell = glyph.source(width, height).unwrap();
                area += cell.width * cell.height;
                assert!(cell.x as u32 + cell.width <= width);
                assert!(cell.y as u32 + cell.height <= height);
                for other in glyphs {
                    if glyph != other {
                        assert!(other.source(width, height).unwrap().local(cell.x, cell.y).is_none());
                    }
                }
            }
            assert_eq!(area, width * height);
        }
        assert!(Glyph::Back.source(3, 1).is_none());
        assert!(Glyph::Back.source(u32::MAX, 887).is_none());
    }
    // ------------------------=
    // FUNC: interaction_recipes_are_distinct_and_bounded
    // DESC: Checks observable style values for every control state without matching rendered prose.
    // ------------------=
    #[test]
    fn interaction_recipes_are_distinct_and_bounded() {
        for primary in [false, true] {
            let states = [Interaction::Normal, Interaction::Hovered, Interaction::Pressed, Interaction::Disabled];
            for (i, state) in states.iter().enumerate() {
                let surface = button(primary, *state);
                for value in [surface.top, surface.bottom, surface.border, surface.text] {
                    assert_eq!(value & 0xff000000, 0);
                }
                for other in &states[i + 1..] { assert_ne!(surface, button(primary, *other)); }
            }
        }
    }
}
