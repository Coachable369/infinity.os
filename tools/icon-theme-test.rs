#![allow(dead_code)]

#[path = "../kernel/ui/geometry.rs"]
mod geometry;
#[path = "../kernel/ui/icon_theme.rs"]
mod icon_theme;
#[path = "../kernel/ui/system_layout.rs"]
mod system_layout;

use icon_theme::{IconThemeId, IconThemeRegistry, ICON_ROLE_COUNT, ICON_THEME_COUNT};
use system_layout::SystemLayout;

// ------------------------=
// FUNC: main
// DESC: Verifies validated selection, cycling, invalidation, and complete semantic icon coverage.
// ------------------=
fn main() {
    let mut registry = IconThemeRegistry::new();
    assert_eq!(ICON_THEME_COUNT, 3);
    assert_eq!(ICON_ROLE_COUNT, 60);
    assert_eq!(registry.active(), IconThemeId::CrystalBlueGlass);
    assert_eq!(registry.generation(), 0);

    assert_eq!(registry.cycle(), IconThemeId::LuminousObsidian);
    assert_eq!(registry.generation(), 1);
    assert_eq!(registry.cycle(), IconThemeId::FrostedQuartz);
    assert_eq!(registry.cycle(), IconThemeId::CrystalBlueGlass);
    assert_eq!(registry.generation(), 3);

    assert_eq!(registry.activate(2), Ok(IconThemeId::FrostedQuartz));
    let stable_generation = registry.generation();
    assert_eq!(registry.activate(2), Ok(IconThemeId::FrostedQuartz));
    assert_eq!(registry.generation(), stable_generation);
    assert_eq!(registry.activate(3), Err(()));
    assert_eq!(registry.active(), IconThemeId::FrostedQuartz);
    let layout = SystemLayout::new(1536, 1024);
    assert_eq!(layout.settings_icon_theme_target(449, 703, false), Some(0));
    assert_eq!(layout.settings_icon_theme_target(598, 703, false), Some(1));
    assert_eq!(layout.settings_icon_theme_target(746, 703, false), Some(2));
    assert_eq!(layout.settings_icon_theme_target(100, 100, false), None);
    println!("PASS icon themes: three validated families cover sixty semantic roles and invalidate consumers transactionally");
}
