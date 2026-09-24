#[path = "../kernel/ui/mod.rs"]
mod ui;

use ui::skin::{AccentSurface, AppearanceScope, SkinRegistry};
use ui::system_layout::{SettingsAccentTarget, SettingsWindowState, SystemLayout};

// ------------------------=
// FUNC: main
// DESC: Verifies the independent Primary and Secondary settings controls through live geometry and semantic color behavior.
// ------------------=
fn main() {
    let layout = SystemLayout::new(2560, 1440);
    let state = SettingsWindowState {
        x: 160,
        y: 210,
        width: 680,
        height: 620,
        maximized: false,
        expanded_row: None,
        scroll_offset: 0,
        control_focus: 0,
        row_count: 6,
    };
    let window = layout.settings_window_geometry(state);
    for color_row in [2usize, 3] {
        let row = layout.settings_row_geometry(state, color_row);
        assert!(row.summary.y >= window.viewport.y);
        assert!(row.summary.bottom() <= window.viewport.bottom());
    }

    let primary_state = reveal_color(layout, SettingsWindowState {
        expanded_row: Some(2),
        ..state
    }, 2);
    let primary = layout.settings_primary_geometry(primary_state);
    let primary_x = (primary.spectrum.x + primary.spectrum.width as i32 / 2) * 1000 / 2560;
    let primary_y = (primary.spectrum.y + primary.spectrum.height as i32 / 2) * 1000 / 1440;
    assert!(matches!(
        layout.settings_primary_target(primary_x, primary_y, primary_state),
        Some(SettingsAccentTarget::Spectrum { .. })
    ));
    assert_eq!(
        layout.settings_accent_target(primary_x, primary_y, primary_state),
        None
    );

    let secondary_state = reveal_color(layout, SettingsWindowState {
        expanded_row: Some(3),
        ..state
    }, 3);
    let secondary = layout.settings_accent_geometry(secondary_state);
    let secondary_x =
        (secondary.spectrum.x + secondary.spectrum.width as i32 / 2) * 1000 / 2560;
    let secondary_y =
        (secondary.spectrum.y + secondary.spectrum.height as i32 / 2) * 1000 / 1440;
    assert!(matches!(
        layout.settings_accent_target(secondary_x, secondary_y, secondary_state),
        Some(SettingsAccentTarget::Spectrum { .. })
    ));
    assert_eq!(
        layout.settings_primary_target(secondary_x, secondary_y, secondary_state),
        None
    );

    let mut skins = SkinRegistry::new();
    skins
        .set_primary(0x35233d, AppearanceScope::Machine)
        .unwrap();
    let primary_surfaces = [
        skins.accent_surface(AccentSurface::Header),
        skins.accent_surface(AccentSurface::TopBar),
        skins.accent_surface(AccentSurface::Dock),
        skins.accent_surface(AccentSurface::Widget),
    ];
    let secondary_surfaces = [
        skins.accent_surface(AccentSurface::WindowOutline),
        skins.accent_surface(AccentSurface::Focus),
        skins.accent_surface(AccentSurface::Selection),
    ];
    skins
        .set_accent(0xd45cff, AppearanceScope::User)
        .unwrap();
    assert_eq!(skins.primary_rgb(), 0x35233d);
    assert_eq!(skins.accent_rgb(), 0xd45cff);
    assert_eq!(
        primary_surfaces,
        [
            skins.accent_surface(AccentSurface::Header),
            skins.accent_surface(AccentSurface::TopBar),
            skins.accent_surface(AccentSurface::Dock),
            skins.accent_surface(AccentSurface::Widget),
        ]
    );
    assert_ne!(
        secondary_surfaces,
        [
            skins.accent_surface(AccentSurface::WindowOutline),
            skins.accent_surface(AccentSurface::Focus),
            skins.accent_surface(AccentSurface::Selection),
        ]
    );
}

// ------------------------=
// FUNC: reveal_color
// DESC: Applies the same expansion-to-viewport scroll used by Settings before testing a visible color control.
// ------------------=
fn reveal_color(layout: SystemLayout, mut state: SettingsWindowState, row: usize) -> SettingsWindowState {
    let window = layout.settings_window_geometry_for_section(state, 1);
    let detail = layout.settings_row_geometry_for_section(state, row, 1).detail;
    state.scroll_offset = (detail.bottom().saturating_sub(window.viewport.bottom()).max(0)
        as usize).div_ceil(layout.scale()).min(window.maximum_scroll);
    let detail = layout.settings_row_geometry_for_section(state, row, 1).detail;
    assert_eq!(window.viewport.intersection(detail), detail);
    state
}
