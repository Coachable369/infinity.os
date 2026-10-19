//! Behavioral face-binding test with the actual installed glyph, metric and kerning bytes.
#![allow(dead_code)]
#[path = "../kernel/runtime/font_faces.rs"]
pub(crate) mod font_faces;
mod runtime { pub(crate) use crate::font_faces; }
mod installer {
    pub const INSTALLER_FONT_ATLAS: &[u8] = include_bytes!("../assets/fonts/InfinityInstaller-Regular-24.atlas");
    pub const INSTALLER_FONT_SEMIBOLD_ATLAS: &[u8] = include_bytes!("../assets/fonts/InfinityInstaller-Semibold-24.atlas");
    pub const INSTALLER_FONT_METRICS: &[u8] = include_bytes!("../assets/fonts/InfinityInstaller-Regular-24.metrics");
    pub const INSTALLER_FONT_SEMIBOLD_METRICS: &[u8] = include_bytes!("../assets/fonts/InfinityInstaller-Semibold-24.metrics");
    pub const INSTALLER_FONT_KERN: &[u8] = include_bytes!("../assets/fonts/InfinityInstaller-Regular-24.kern");
    pub const INSTALLER_FONT_SEMIBOLD_KERN: &[u8] = include_bytes!("../assets/fonts/InfinityInstaller-Semibold-24.kern");
}
#[path = "../kernel/core/bootstrap/template_font.rs"]
mod template_font;

// ------------------------=
// FUNC: explicit_family_binds_all_three_slices_independently_of_desktop_preferences
// DESC: Exercises actual selection for both weights and every desktop family, including complete glyph coverage and matching layout metadata.
// ------------------=
#[test]
fn explicit_family_binds_all_three_slices_independently_of_desktop_preferences() {
    for selected in 1..=46 {
        font_faces::publish(selected);
        for bold in [false, true] {
            let actual = template_font::face(Some(1), bold);
            let expected = font_faces::face(1, bold);
            assert_eq!(actual, expected);
            assert!(core::ptr::eq(actual.0.as_ptr(), expected.0.as_ptr()));
            assert_eq!((actual.0.len(), actual.1.len(), actual.2.len()), (24 * 28 * 95, 95, 95 * 95));
            for glyph in 1..95 {
                assert!(actual.1[glyph] > 0);
                assert!((0..28).any(|row| actual.0[row * 24 * 95 + glyph * 24..row * 24 * 95 + (glyph + 1) * 24].iter().any(|alpha| *alpha != 0)));
            }
            assert_ne!(actual.0, font_faces::face(2, bold).0);
        }
    }
}

// ------------------------=
// FUNC: absent_family_preserves_legacy_installer_pixels_and_layout
// DESC: Verifies the optional extension leaves every byte of the historical installer family unchanged for both weights.
// ------------------=
#[test]
fn absent_family_preserves_legacy_installer_pixels_and_layout() {
    assert_eq!(template_font::face(None, false), (installer::INSTALLER_FONT_ATLAS, installer::INSTALLER_FONT_METRICS, installer::INSTALLER_FONT_KERN));
    assert_eq!(template_font::face(None, true), (installer::INSTALLER_FONT_SEMIBOLD_ATLAS, installer::INSTALLER_FONT_SEMIBOLD_METRICS, installer::INSTALLER_FONT_SEMIBOLD_KERN));
}
