//! Shared web compatibility settings for production and the native guest tests.

// ------------------------=
// FUNC: web_preferences
// DESC: Enables implemented lazy-loading and font APIs required by modern pages without granting new device authority.
// ------------------=
pub fn web_preferences() -> servo::Preferences {
    servo::Preferences {
        dom_intersection_observer_enabled: true,
        dom_fontface_enabled: true,
        layout_variable_fonts_enabled: true,
        ..servo::Preferences::default()
    }
}
