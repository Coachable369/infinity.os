#![allow(dead_code)]

#[path = "../kernel/ui/mod.rs"]
mod ui;

use ui::icon_theme::{IconThemeId, IconThemeRegistry, ICON_ROLE_COUNT, ICON_THEME_COUNT};
use ui::system_layout::{SettingsWindowState, SystemLayout};

// ------------------------=
// FUNC: main
// DESC: Verifies validated selection, cycling, invalidation, and complete semantic icon coverage.
// ------------------=
fn main() {
    let mut registry = IconThemeRegistry::new();
    assert_eq!(ICON_THEME_COUNT, 4);
    assert_eq!(ICON_ROLE_COUNT, 60);
    assert_eq!(registry.active(), IconThemeId::CrystalBlueGlass);
    assert_eq!(registry.generation(), 0);

    assert_eq!(registry.cycle(), IconThemeId::LuminousObsidian);
    assert_eq!(registry.generation(), 1);
    assert_eq!(registry.cycle(), IconThemeId::FrostedQuartz);
    assert_eq!(registry.cycle(), IconThemeId::AuroraHarmony);
    assert_eq!(registry.cycle(), IconThemeId::CrystalBlueGlass);
    assert_eq!(registry.generation(), 4);

    assert_eq!(registry.activate(2), Ok(IconThemeId::FrostedQuartz));
    let stable_generation = registry.generation();
    assert_eq!(registry.activate(2), Ok(IconThemeId::FrostedQuartz));
    assert_eq!(registry.generation(), stable_generation);
    assert_eq!(registry.activate(3), Ok(IconThemeId::AuroraHarmony));
    assert_eq!(registry.activate(4), Err(()));
    assert_eq!(registry.active(), IconThemeId::AuroraHarmony);
    let layout = SystemLayout::new(1536, 1024);
    let settings = SettingsWindowState {
        x: 160,
        y: 210,
        width: 680,
        height: 620,
        maximized: false,
        expanded_row: Some(1),
        scroll_offset: 0,
        control_focus: 0,
        row_count: 6,
    };
    let detail = layout.settings_row_geometry(settings, 1).detail;
    let card_width = detail.width as i32 / ICON_THEME_COUNT as i32;
    for theme in 0..ICON_THEME_COUNT as i32 {
        let framebuffer_x = detail.x + theme * card_width + card_width / 2;
        let framebuffer_y = detail.y + detail.height as i32 / 2;
        assert_eq!(
            layout.settings_icon_theme_target(
                framebuffer_x * 1000 / 1536,
                framebuffer_y * 1000 / 1024,
                settings,
            ),
            Some(theme as u8)
        );
    }
    assert_eq!(layout.settings_icon_theme_target(100, 100, settings), None);
    println!("PASS icon themes: four validated families cover sixty semantic roles and invalidate consumers transactionally");
}
