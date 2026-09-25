#[path = "../kernel/ui/mod.rs"]
mod ui;

use ui::system_layout::{
    eased_scroll_offset, SettingsTarget, SettingsWindowState, SystemLayout,
    SETTINGS_DASHBOARD_CONTENT_HEIGHT, SETTINGS_NODE_CONTENT_HEIGHT, SETTINGS_NETWORK_SECTION, SETTINGS_NODE_SECTION,
    SETTINGS_SECTION_ICON_SIZE,
};

// ------------------------=
// FUNC: main
// DESC: Verifies bounded Settings navigation, dashboard scrolling, and draggable scrollbar geometry.
// ------------------=
fn main() {
    verify_card_content();
    sidebar_spacing();
    verify_authored_row_flow();
    verify_configuration_network_targets();
    verify_settings_chrome_targets();
    verify_expanded_action_targets();
    for width in [1024, 1366, 1920, 2560] {
        let layout = SystemLayout::new(width, 1440);
        let state = SettingsWindowState { x: 145, y: 155, width: 690, height: 500,
            maximized: false, expanded_row: Some(1), scroll_offset: 0, control_focus: 0, row_count: 8 };
        let geometry = layout.node_settings_geometry(state);
        assert!(!geometry.main.intersects(geometry.sidebar));
        assert!(geometry.sidebar.width >= 220 * layout.scale() as u32);
        for control in geometry.controls {
            assert_eq!(geometry.main.intersection(control), control);
        }
    }
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
        let section = layout.settings_section_geometry_for_section(state, index, SETTINGS_NODE_SECTION);
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
    let top = layout.node_settings_geometry(state);
    for index in 0..5 {
        assert!(top.tabs[index].width >= (184 * layout.scale()) as u32);
        assert_eq!(top.tabs[index].height, (44 * layout.scale()) as u32);
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
        assert!(control.height >= (12 + 28 + 4 + 28 + 8) * layout.scale() as u32);
        assert_eq!(top.main.intersection(*control), *control);
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

// ------------------------=
// FUNC: verify_card_content
// DESC: Verifies kit card content stays inside its row with no label, value or icon overlap at supported densities.
// ------------------=
fn verify_card_content() {
    for scale in [1, 2] {
        for width in [380, 600, 900] {
            let row = ui::geometry::Rect { x: 20, y: 30, width: width * scale,
                height: 72 * scale };
            let card = ui::settings_cards::content(row, scale as usize);
            for child in [card.icon, card.label, card.description, card.value] {
                assert_eq!(row.intersection(child), child);
                assert!(child.width > 0 && child.height > 0);
            }
            assert!(!card.label.intersects(card.description));
            assert!(!card.label.intersects(card.value));
            assert!(!card.description.intersects(card.value));
            assert!(!card.icon.intersects(card.label));
        }
    }
}

// ------------------------=
// FUNC: verify_settings_chrome_targets
// DESC: Checks visible window-button centers and dashboard bottom reachability for every category and display scale.
// ------------------=
fn verify_settings_chrome_targets() {
    for (width, height) in [(1024, 768), (1920, 1080), (2560, 1440)] {
        let layout = SystemLayout::new(width, height);
        let scale = layout.scale() as i32;
        for section in 0..11 {
            let state = SettingsWindowState { x: 60, y: 80, width: 850, height: 760,
                maximized: false, expanded_row: None, scroll_offset: 0, control_focus: 0, row_count: 8 };
            let geometry = layout.settings_window_geometry_for_section(state, section);
            for index in 0..3 {
                let x = geometry.window.right() - (26 + (2 - index as i32) * 25) * scale + 10 * scale;
                let y = geometry.title.y + geometry.title.height as i32 / 2;
                assert_eq!(layout.settings_target_for_section(x * 1000 / width as i32,
                    y * 1000 / height as i32, state, section), Some(SettingsTarget::WindowControl(index)));
            }
            if matches!(section, SETTINGS_NODE_SECTION | SETTINGS_NETWORK_SECTION) {
                let bottom = SettingsWindowState { scroll_offset: geometry.maximum_scroll, ..state };
                let dashboard = if section == SETTINGS_NODE_SECTION { layout.node_settings_geometry(bottom) }
                    else { layout.network_settings_geometry(bottom) };
                assert!(dashboard.sidebar.bottom() <= geometry.viewport.bottom());
                assert!(dashboard.main.bottom() <= geometry.viewport.bottom());
                assert!(geometry.viewport.bottom() - dashboard.sidebar.bottom() <= 24 * scale);
            }
        }
    }
}

// ------------------------=
// FUNC: verify_expanded_action_targets
// DESC: Checks the widened action edge and excludes invisible actions from inspection-only details.
// ------------------=
fn verify_expanded_action_targets() {
    let layout = SystemLayout::new(1920, 1440);
    for (section, actionable) in [(0, true), (1, true), (2, false), (3, true),
        (4, false), (5, false), (8, true), (9, false)] {
        let state = SettingsWindowState { x: 0, y: 0, width: 1000, height: 1000,
            maximized: true, expanded_row: Some(0), scroll_offset: 0,
            control_focus: 0, row_count: 5 };
        let detail = layout.settings_row_geometry_for_section(state, 0, section).detail;
        let x = detail.x + ui::system_layout::UI_GUTTER as i32 + 220;
        let y = detail.bottom() - 30;
        assert!(layout.settings_window_geometry_for_section(state, section).viewport.contains(
            layout.point(x * 1000 / 1920, y * 1000 / 1440)));
        assert_eq!(layout.settings_target_for_section(x * 1000 / 1920, y * 1000 / 1440, state, section),
            actionable.then_some(SettingsTarget::ExpandedAction));
    }
}

// ------------------------=
// FUNC: sidebar_spacing
// DESC: Verifies authored sidebar gutters, themed role validity, row separation and shared navigator hit geometry.
// ------------------=
fn sidebar_spacing() {
    use ui::installer_template::InstallerTemplateRole as R;
    for section in 0..11 {
        let mut bottom = 0;
        for index in 0..11 {
            let row = ui::settings_template::role_at(section, R::SettingsNavigationItem, index).unwrap().frame;
            let icon = ui::settings_template::role_at(section, R::SettingsNavigationIcon, index).unwrap().frame;
            let label = ui::settings_template::role_at(section, R::SettingsNavigationLabel, index).unwrap().frame;
            assert!(row.y >= bottom + if index == 0 {0} else {12});
            assert_eq!(icon.x-row.x, 16);
            assert_eq!(label.x-icon.x-icon.width, 12);
            assert_eq!(icon.width, icon.height);
            assert_eq!(icon.y+icon.height/2, row.y+row.height/2);
            assert!(label.x+label.width <= row.x+row.width-16);
            assert!(ui::icon_theme::settings_section_icon(index) < ui::icon_theme::ICON_ROLE_COUNT);
            bottom = row.y+row.height;
        }
    }
    for (width,height) in [(1280,720),(1920,1080),(3840,2160)] {
        let layout = SystemLayout::new(width,height);
        let s=layout.scale() as u32;
        let window = ui::geometry::Rect{x:0,y:0,width:780*s,height:560*s};
        let mut bottom=0;
        for index in 0..12 {
            let row=layout.navigator_sidebar_row(window,index);
            assert_eq!(row.x,16*s as i32);
            assert!(row.y>=bottom);
            assert!(row.bottom()<=window.bottom()-16*s as i32);
            bottom=row.bottom()+4*s as i32;
        }
    }
}

// ------------------------=
// FUNC: verify_authored_row_flow
// DESC: Exercises saved-template rows through expansion, scrolling, resize, and disclosure hit testing.
// ------------------=
fn verify_authored_row_flow() {
    use ui::installer_template::InstallerTemplateRole as Role;
    let original = ui::installer_template::InstallerTemplate::parse_settings(
        ui::installer_layout::SETTINGS_TEMPLATE_BYTES).unwrap();
    for section in 0..11 {
        let mut layers: Vec<_> = (0..original.element_count(section + 1).unwrap())
            .map(|index| original.element_at(section + 1, index).unwrap()).collect();
        layers.sort_by_key(|element| (element.z_index, element.id));
        for (index, element) in layers.iter().enumerate() {
            assert_eq!(ui::settings_template::layer_at(section as usize, index as u16), Some(*element));
        }
        assert_eq!(ui::settings_template::layer_at(section as usize, layers.len() as u16), None);
        for role in [Role::Console, Role::Content, Role::Title, Role::Body,
            Role::Metadata, Role::SettingsRowLabel, Role::SettingsRowValue, Role::SettingsDisclosure] {
            let expected: Vec<_> = (0..original.element_count(section + 1).unwrap())
                .filter_map(|index| original.element_at(section + 1, index))
                .filter(|element| element.role == role as u8).collect();
            for (index, element) in expected.iter().enumerate() {
                assert_eq!(ui::settings_template::role_at(section as usize, role, index), Some(*element));
            }
            assert_eq!(ui::settings_template::role_at(section as usize, role, expected.len()), None);
        }
    }
    for (width, height) in [(1024, 768), (1366, 768), (1920, 1080), (2048, 2048), (2560, 1440)] {
        let layout = SystemLayout::new(width, height);
        for section in [0, 1, 2, 3, 4, 5, 8, 9, 10] {
            let count = if matches!(section, 1 | 8 | 10) { 8 } else if section == 3 { 7 } else { 5 };
            let collapsed = SettingsWindowState { x: 80, y: 100, width: 850, height: 760,
                maximized: false, expanded_row: None, scroll_offset: 0, control_focus: 0, row_count: count };
            for expanded in 0..count {
                let mut state = SettingsWindowState { expanded_row: Some(expanded), ..collapsed };
                let window = layout.settings_window_geometry_for_section(state, section);
                let assistant = ui::app_assistant::geometry(window.window, layout.scale(), false);
                assert!(!window.scrollbar_track.intersects(assistant.toggle));
                let heading = layout.authored_settings_rect(state, section, Role::Body, 0).unwrap();
                assert!(window.viewport.y >= heading.bottom());
                let detail = layout.settings_row_geometry_for_section(state, expanded, section).detail;
                assert!(detail.height >= 78 * layout.scale() as u32);
                if expanded == 0 {
                    let short = SettingsWindowState { height: 420, ..state };
                    let well = layout.settings_row_geometry_for_section(short, expanded, section).detail;
                    let text_bottom = well.y + ((ui::system_layout::UI_GUTTER + 20) * layout.scale()) as i32;
                    let action_top = well.bottom() - ((ui::system_layout::UI_COMPACT_ACTION_HEIGHT + 10) * layout.scale()) as i32;
                    assert!(text_bottom + (8 * layout.scale()) as i32 <= action_top);
                }
                if expanded + 1 < count {
                    let next = layout.settings_row_geometry_for_section(state, expanded + 1, section);
                    assert!(detail.bottom() <= next.summary.y, "section {section}, expanded {expanded}");
                }
                for scroll in [0, window.maximum_scroll / 2, window.maximum_scroll] {
                    state.scroll_offset = scroll;
                    for index in 0..count {
                        let row = layout.settings_row_geometry_for_section(state, index, section).summary;
                        let base_row = layout.settings_row_geometry_for_section(collapsed, index, section).summary;
                        for role in [Role::SettingsRowLabel, Role::SettingsRowValue, Role::SettingsDisclosure] {
                            let child = layout.settings_row_element_geometry(state, section, index, role).unwrap();
                            let base_child = layout.settings_row_element_geometry(collapsed, section, index, role).unwrap();
                            assert_eq!(child.y - row.y, base_child.y - base_row.y);
                            assert_eq!(child.x - row.x, base_child.x - base_row.x);
                            assert_eq!(row.intersection(child), child);
                            if role == Role::SettingsDisclosure && window.viewport.intersection(child) == child {
                                let x = (child.x + child.width as i32 / 2) * 1000 / width as i32;
                                let y = (child.y + child.height as i32 / 2) * 1000 / height as i32;
                                assert_eq!(layout.settings_target_for_section(x, y, state, section), Some(SettingsTarget::ContentRow(index)));
                            }
                        }
                    }
                }
            }
        }
    }
}

// ------------------------=
// FUNC: verify_configuration_network_targets
// DESC: Exercises network hit targets and empty gutters at the actual authored row positions across display scales.
// ------------------=
fn verify_configuration_network_targets() {
    use ui::installer_layout::{configuration_network_row_rect, configuration_template_rect};
    use ui::installer_template::InstallerTemplateRole;
    use ui::system_layout::OnboardingTarget;
    for (width, height) in [(1024, 768), (1600, 1000), (2560, 1440)] {
        let layout = SystemLayout::new(width, height);
        let content = configuration_template_rect(6, InstallerTemplateRole::Content, width, height).unwrap();
        for index in 0..3 {
            let row = configuration_network_row_rect(index, width, height).unwrap();
            assert!(content.contains(row));
            for fraction in [1, 2, 3] {
                let x = ((row.left + row.width * fraction / 4) * 1000 / width) as i32;
                let y = ((row.top + row.height / 2) * 1000 / height) as i32;
                assert_eq!(layout.onboarding_target(6, x, y), Some(OnboardingTarget::NetworkChoice(index)));
            }
            if index < 2 {
                let next = configuration_network_row_rect(index + 1, width, height).unwrap();
                assert!(row.bottom() < next.top);
                let x = ((row.left + row.width / 2) * 1000 / width) as i32;
                let y = (((row.bottom() + next.top) / 2) * 1000 / height) as i32;
                assert_eq!(layout.onboarding_target(6, x, y), None);
            }
        }
        assert!(configuration_network_row_rect(3, width, height).is_none());
        for (role, expected) in [(InstallerTemplateRole::BackButton, OnboardingTarget::Back),
                                 (InstallerTemplateRole::PrimaryButton, OnboardingTarget::Primary)] {
            let row = configuration_template_rect(6, role, width, height).unwrap();
            assert_eq!(layout.onboarding_target(6,
                ((row.left + row.width / 2) * 1000 / width) as i32,
                ((row.top + row.height / 2) * 1000 / height) as i32), Some(expected));
        }
    }
}
