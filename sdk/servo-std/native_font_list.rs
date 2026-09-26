//! Minimal packaged font catalog. No fontconfig, directory scans or host paths.
use fonts_traits::{FontIdentifier, FontTemplate, FontTemplateDescriptor, LocalFontIdentifier, LowercaseFontFamilyName};
use style::values::computed::font::GenericFontFamily;
use crate::FallbackFontSelectionOptions;

// ------------------------=
// FUNC: for_each_available_family
// DESC: Enumerates the three actual fonts embedded in the native engine package.
// ------------------=
pub(crate) fn for_each_available_family<F: FnMut(String)>(mut callback: F) {
    for name in ["Fira Sans", "EB Garamond", "IBM Plex Mono"] { callback(name.into()); }
}
// ------------------------=
// FUNC: for_each_variation
// DESC: Exposes the regular face; bold and italic synthesis remain with the existing font engine.
// ------------------=
pub(crate) fn for_each_variation<F: FnMut(FontTemplate)>(family_name: &str, mut callback: F) {
    let id = if family_name.eq_ignore_ascii_case("Fira Sans") { LocalFontIdentifier::Sans }
        else if family_name.eq_ignore_ascii_case("EB Garamond") { LocalFontIdentifier::Serif }
        else if family_name.eq_ignore_ascii_case("IBM Plex Mono") { LocalFontIdentifier::Mono }
        else { return; };
    callback(FontTemplate::new(FontIdentifier::Local(id), FontTemplateDescriptor::default(), None));
}
// ------------------------=
// FUNC: fallback_font_families
// DESC: Offers only the packaged families; comprehensive script and emoji coverage is not claimed.
// ------------------=
pub fn fallback_font_families(_: FallbackFontSelectionOptions) -> Vec<&'static str> {
    vec!["Fira Sans", "EB Garamond", "IBM Plex Mono"]
}
// ------------------------=
// FUNC: default_system_generic_font_family
// DESC: Maps CSS generic families to actual packaged typefaces.
// ------------------=
pub(crate) fn default_system_generic_font_family(generic: GenericFontFamily) -> LowercaseFontFamilyName {
    let name = match generic {
        GenericFontFamily::Monospace => "IBM Plex Mono",
        GenericFontFamily::None | GenericFontFamily::Serif | GenericFontFamily::Cursive | GenericFontFamily::Fantasy => "EB Garamond",
        _ => "Fira Sans",
    };
    LowercaseFontFamilyName::from(name)
}
