#[path = "../kernel/ui/mod.rs"]
mod ui;

use ui::system_layout::{
    SettingsTarget, SettingsWindowState, SystemLayout, SETTINGS_SECTION_ICON_SIZE,
};

// ------------------------=
// FUNC: main
// DESC: Verifies bounded Settings navigation, dashboard scrolling, and draggable scrollbar geometry.
// ------------------=
fn main() {
    let layout = SystemLayout::new(2560, 1440);
    let state = SettingsWindowState {
        x: 145,
        y: 155,
        width: 690,
        height: 500,
        maximized: false,
        expanded_row: Some(0),
        scroll_offset: 0,
        control_focus: 4,
        row_count: 8,
    };
    let window = layout.settings_window_geometry(state);
    assert!(SETTINGS_SECTION_ICON_SIZE >= 24);
    for index in 0..10 {
        let section = layout.settings_section_geometry(state, index);
        assert!(section.y >= window.navigation.y);
        assert!(section.bottom() <= window.navigation.bottom());
        let x = (section.x + section.width as i32 / 2) * 1000 / 2560;
        let y = (section.y + section.height as i32 / 2) * 1000 / 1440;
        assert_eq!(layout.settings_target(x, y, state), Some(SettingsTarget::Section(index)));
    }

    assert!(window.maximum_scroll > 0);
    let top = layout.network_settings_geometry(state);
    let scrolled_state = SettingsWindowState {
        scroll_offset: window.maximum_scroll,
        ..state
    };
    let bottom = layout.network_settings_geometry(scrolled_state);
    assert_eq!(
        top.tabs[0].y - bottom.tabs[0].y,
        (window.maximum_scroll * layout.scale()) as i32
    );
    assert_eq!(scrolled_state.control_focus, state.control_focus);

    let thumb_x = window.scrollbar_thumb.x + window.scrollbar_thumb.width as i32 / 2;
    let thumb_y = window.scrollbar_thumb.y + window.scrollbar_thumb.height as i32 / 2;
    assert_eq!(
        layout.settings_target(thumb_x * 1000 / 2560, thumb_y * 1000 / 1440, state),
        Some(SettingsTarget::ScrollThumb)
    );
    assert_eq!(
        layout.settings_scroll_offset_for_thumb(1000, state, 0),
        window.maximum_scroll
    );
}
