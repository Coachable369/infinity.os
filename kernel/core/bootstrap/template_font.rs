//! Explicit pre-session font selection; legacy installer callers keep their original face.

use super::installer::*;

// ------------------------=
// FUNC: face
// DESC: Returns matching glyph, advance and kerning data for an explicit FontKit family or the unchanged legacy installer font.
// ------------------=
pub(super) fn face(family: Option<u8>, semibold: bool) -> (&'static [u8], &'static [u8], &'static [u8]) {
    if let Some(family) = family {
        return crate::runtime::font_faces::face(family, semibold);
    }
    if semibold {
        (INSTALLER_FONT_SEMIBOLD_ATLAS, INSTALLER_FONT_SEMIBOLD_METRICS, INSTALLER_FONT_SEMIBOLD_KERN)
    } else {
        (INSTALLER_FONT_ATLAS, INSTALLER_FONT_METRICS, INSTALLER_FONT_KERN)
    }
}
