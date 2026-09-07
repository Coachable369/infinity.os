#[path = "../kernel/ui/mod.rs"]
mod ui;

use ui::system_layout::{
    eased_scroll_offset, SettingsTarget, SettingsWindowState, SystemLayout,
    SETTINGS_DASHBOARD_CONTENT_HEIGHT, SETTINGS_NETWORK_SECTION, SETTINGS_NODE_SECTION,
    SETTINGS_SECTION_ICON_SIZE,
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
    let window = layout.settings_window_geometry_for_section(state, SETTINGS_NODE_SECTION);
    assert!(SETTINGS_SECTION_ICON_SIZE >= 24);
    for index in 0..10 {
        let section = layout.settings_section_geometry(state, index);
        assert!(section.y >= window.navigation.y);
        assert!(section.bottom() <= window.navigation.bottom());
        let x = (section.x + section.width as i32 / 2) * 1000 / 2560;
        let y = (section.y + section.height as i32 / 2) * 1000 / 1440;
        assert_eq!(
            layout.settings_target_for_section(x, y, state, SETTINGS_NODE_SECTION),
            Some(SettingsTarget::Section(index))
        );
    }

    assert!(window.maximum_scroll > 0);
    assert_eq!(window.total_content_height, SETTINGS_DASHBOARD_CONTENT_HEIGHT);
    let top = layout.node_settings_geometry(state);
    for index in 0..5 {
        assert!(top.tabs[index].width >= (112 * layout.scale()) as u32);
        for following in (index + 1)..5 {
            assert!(!top.tabs[index].intersects(top.tabs[following]));
        }
    }
    let tab_bottom = top.tabs[..5]
        .iter()
        .map(|tab| tab.bottom())
        .max()
        .unwrap();
    assert!(top.summary.y > tab_bottom);
    assert!(!top.main.intersects(top.sidebar));
    for (index, control) in top.controls.iter().enumerate() {
        assert_eq!(control.height, (58 * layout.scale()) as u32);
        assert!(top.main.intersection(*control).width > 0);
        assert!(top.main.intersection(*control).height > 0);
        if index > 0 {
            assert!(top.controls[index - 1].bottom() < control.y);
        }
    }

    let network = layout.network_settings_geometry(state);
    for index in 0..7 {
        for following in (index + 1)..7 {
            assert!(!network.tabs[index].intersects(network.tabs[following]));
        }
    }
    let scrolled_state = SettingsWindowState {
        scroll_offset: window.maximum_scroll,
        ..state
    };
    let bottom = layout.node_settings_geometry(scrolled_state);
    assert_eq!(
        top.tabs[0].y - bottom.tabs[0].y,
        (window.maximum_scroll * layout.scale()) as i32
    );
    assert_eq!(scrolled_state.control_focus, state.control_focus);

    let thumb_x = window.scrollbar_thumb.x + window.scrollbar_thumb.width as i32 / 2;
    let thumb_y = window.scrollbar_thumb.y + window.scrollbar_thumb.height as i32 / 2;
    assert_eq!(
        layout.settings_target_for_section(
            thumb_x * 1000 / 2560,
            thumb_y * 1000 / 1440,
            state,
            SETTINGS_NODE_SECTION,
        ),
        Some(SettingsTarget::ScrollThumb)
    );
    assert_eq!(
        layout.settings_scroll_offset_for_thumb_in_section(
            1000,
            state,
            0,
            SETTINGS_NODE_SECTION,
        ),
        window.maximum_scroll
    );

    let mut eased = 0;
    while eased != window.maximum_scroll {
        let next = eased_scroll_offset(eased, window.maximum_scroll, window.maximum_scroll);
        assert!(next > eased);
        assert!(next <= window.maximum_scroll);
        eased = next;
    }
    assert_eq!(
        eased_scroll_offset(window.maximum_scroll, 0, window.maximum_scroll),
        window.maximum_scroll.saturating_sub(24)
    );

    for section in [0usize, 1, 2, 3, 4, 5, 8, 9] {
        let generic = layout.settings_window_geometry_for_section(state, section);
        assert_ne!(generic.total_content_height, SETTINGS_DASHBOARD_CONTENT_HEIGHT);
        for index in 1..state.row_count {
            let previous = layout.settings_row_geometry(state, index - 1);
            let current = layout.settings_row_geometry(state, index);
            assert!(previous.summary.bottom() <= current.summary.y || previous.detail.bottom() <= current.summary.y);
        }
    }

    let minimum = SettingsWindowState {
        width: 600,
        height: 420,
        ..state
    };
    let minimum_window = SystemLayout::new(1920, 1080)
        .settings_window_geometry_for_section(minimum, SETTINGS_NETWORK_SECTION);
    assert!(minimum_window.maximum_scroll > 0);
}
