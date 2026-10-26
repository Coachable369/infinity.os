//! Behavioral authoring proof through the production Settings decoder, layout, and pointer routing.
#![allow(dead_code)]

#[path = "../kernel/ui/mod.rs"]
mod ui;

use std::sync::OnceLock;
use ui::geometry::{Point, Rect};
use ui::installer_template::{InstallerTemplate, InstallerTemplateRect, InstallerTemplateRole as Role, SettingsAction};
use ui::system_layout::{SettingsTarget, SettingsWindowState, SystemLayout};

static FIXTURE: OnceLock<Vec<u8>> = OnceLock::new();
const ACTION_ROWS: &[(usize, usize)] = &[(0, 0), (3, 0), (3, 1), (3, 2), (3, 4), (3, 5), (3, 6), (4, 1)];

mod studio_native_render {
    // ------------------------=
    // FUNC: settings_template_bytes
    // DESC: Supplies one immutable authored fixture through the production Studio preview adapter.
    // ------------------=
    pub fn settings_template_bytes() -> Option<&'static [u8]> {
        super::FIXTURE.get().map(Vec::as_slice)
    }

    // ------------------------=
    // FUNC: asset_override
    // DESC: Keeps production palette and material assets unchanged during geometry verification.
    // ------------------=
    pub fn asset_override(_: &[u8]) -> Option<&'static [u8]> { None }

    // ------------------------=
    // FUNC: precision_art
    // DESC: Keeps the approved production surface atlas unchanged during geometry verification.
    // ------------------=
    pub fn precision_art() -> Option<&'static [u8]> { None }
}

#[derive(Clone, Copy)]
struct Record {
    section: usize,
    id: [u8; 16],
    role: u8,
    flags: usize,
    frame: usize,
    opacity: usize,
    caption: usize,
}

// ------------------------=
// FUNC: word
// DESC: Reads a bounded binary fixture field without treating rendered text as a test oracle.
// ------------------=
fn word(bytes: &[u8], cursor: &mut usize) -> usize {
    let value = u16::from_le_bytes(bytes[*cursor..*cursor + 2].try_into().unwrap());
    *cursor += 2;
    usize::from(value)
}

// ------------------------=
// FUNC: records
// DESC: Locates serialized fields in the actual compiled Settings document for isolated authoring mutations.
// ------------------=
fn records(bytes: &[u8]) -> Vec<Record> {
    let version = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
    assert!(matches!(version, 10 | 11));
    let mut cursor = 6;
    let screens = word(bytes, &mut cursor);
    let mut result = Vec::new();
    for _ in 0..screens {
        let section = usize::from(bytes[cursor]) - 1;
        cursor += 1;
        cursor += 1 + usize::from(bytes[cursor]);
        let count = word(bytes, &mut cursor);
        for _ in 0..count {
            let id = bytes[cursor..cursor + 16].try_into().unwrap();
            cursor += 16;
            cursor += 1 + usize::from(bytes[cursor]);
            let kind = cursor;
            let role = bytes[kind + 1];
            cursor += 4 + 2 + 8 + 8 + 1 + 1 + 2;
            cursor += 1 + usize::from(bytes[cursor]);
            let text_length = word(bytes, &mut cursor);
            cursor += text_length;
            cursor += 1 + usize::from(bytes[cursor]);
            cursor += 4 + 16;
            let caption = cursor;
            cursor += 1 + 4;
            result.push(Record { section, id, role, flags: kind + 3, frame: kind + 6,
                opacity: kind + 22, caption });
        }
    }
    result
}

// ------------------------=
// FUNC: fixture
// DESC: Mutates one real authored control or its parent while retaining all other packaged Settings bytes and validating the result natively.
// ------------------=
fn fixture(case: &str) -> Vec<u8> {
    let mut bytes = ui::installer_layout::SETTINGS_TEMPLATE_BYTES.to_vec();
    let entries = records(&bytes);
    let action = entries.iter().find(|r| r.section == 0 && r.role == Role::SettingsRowAction as u8).unwrap();
    let parent = entries.iter().find(|r| r.section == 0 && r.role == Role::Metadata as u8).unwrap();
    let frame = match case {
        "moved" => Some([400u16, 150, 350, 350]),
        "resized" => Some([20u16, 400, 950, 550]),
        _ => None,
    };
    if let Some(frame) = frame {
        for (index, value) in frame.iter().enumerate() {
            bytes[action.frame + index * 2..action.frame + index * 2 + 2].copy_from_slice(&value.to_le_bytes());
        }
    }
    match case {
        "baseline" | "moved" | "resized" => {},
        "hidden" => bytes[action.flags] |= 2,
        "transparent" => bytes[action.opacity] = 0,
        "parent-hidden" => bytes[parent.flags] |= 2,
        "parent-transparent" => bytes[parent.opacity] = 0,
        "literal-caption" => bytes[action.caption] = 1,
        "caption-transparent" => bytes[action.caption + 4] = 0,
        _ => panic!("Unknown fixture case"),
    }
    let parsed = InstallerTemplate::parse_settings(&bytes).unwrap();
    let edited = (0..parsed.element_count(1).unwrap()).filter_map(|i| parsed.element_at(1, i))
        .find(|element| element.id == action.id).unwrap();
    if let Some(frame) = frame {
        assert_eq!(edited.frame, InstallerTemplateRect { x: frame[0], y: frame[1], width: frame[2], height: frame[3] });
    }
    bytes
}

// ------------------------=
// FUNC: state
// DESC: Creates a real resizable Settings state with the section's complete runtime row count.
// ------------------=
fn state(section: usize, row: usize) -> SettingsWindowState {
    SettingsWindowState { x: 70, y: 80, width: 850, height: 760, maximized: false,
        expanded_row: Some(row), scroll_offset: 0, control_focus: 0,
        row_count: if section == 3 { 7 } else { 5 } }
}

// ------------------------=
// FUNC: expected_bounds
// DESC: Computes the documented parent-local normalized contract independently of the production action resolver.
// ------------------=
fn expected_bounds(parent: Rect, frame: InstallerTemplateRect) -> Rect {
    Rect { x: parent.x + (u64::from(frame.x) * u64::from(parent.width) / 1000) as i32,
        y: parent.y + (u64::from(frame.y) * u64::from(parent.height) / 1000) as i32,
        width: (u64::from(frame.width) * u64::from(parent.width) / 1000) as u32,
        height: (u64::from(frame.height) * u64::from(parent.height) / 1000) as u32 }
}

// ------------------------=
// FUNC: center
// DESC: Returns an interior physical point for observable hit-testing assertions.
// ------------------=
fn center(rect: Rect) -> Point {
    Point { x: rect.x + rect.width as i32 / 2, y: rect.y + rect.height as i32 / 2 }
}

// ------------------------=
// FUNC: routed_target
// DESC: Invokes the OS's real normalized-pointer dispatcher at a physical control point.
// ------------------=
fn routed_target(layout: SystemLayout, state: SettingsWindowState, section: usize,
    point: Point, width: usize, height: usize) -> Option<SettingsTarget> {
    layout.settings_target_for_section(point.x * 1000 / width as i32,
        point.y * 1000 / height as i32, state, section)
}

// ------------------------=
// FUNC: verify_case
// DESC: Verifies moved, resized, hidden, transparent, collapsed, scrolled, and dynamic-row action behavior through the actual OS layout and input paths.
// ------------------=
fn verify_case(case: &str) {
    FIXTURE.set(fixture(case)).unwrap();
    let disabled = matches!(case, "hidden" | "transparent" | "parent-hidden" | "parent-transparent");
    for (width, height) in [(1280, 800), (1920, 1080)] {
        let layout = SystemLayout::new(width, height);
        let mut hit_count = 0;
        for &(section, row) in ACTION_ROWS {
            let initial = state(section, row);
            let maximum = layout.settings_window_geometry_for_section(initial, section).maximum_scroll;
            for offset in [0, maximum / 2, maximum] {
                let state = SettingsWindowState { scroll_offset: offset, ..initial };
                let geometry = layout.settings_action_geometry(state, section, row).unwrap();
                let element = geometry.element.unwrap();
                let expected_action = match (section, row) {
                    (0, 0) => SettingsAction::RenameMachine,
                    (3, 0) => SettingsAction::CyclePolicy,
                    (3, 1) => SettingsAction::ToggleChat,
                    (3, 2) => SettingsAction::CycleModel,
                    (3, 4) | (4, 1) => SettingsAction::ToggleMicrophone,
                    (3, 5) => SettingsAction::CycleWakeWord,
                    (3, 6) => SettingsAction::ToggleSpeech,
                    _ => unreachable!(),
                };
                assert_eq!(geometry.action, expected_action);
                if section == 0 && case == "literal-caption" { assert_eq!(element.caption_mode, 1); }
                if section == 0 && case == "caption-transparent" { assert_eq!(element.caption_color[3], 0); }
                let parent = layout.settings_row_geometry_for_section(state, row, section).detail;
                let viewport = layout.settings_window_geometry_for_section(state, section).viewport;
                assert_eq!(geometry.parent_row_index, row);
                assert_eq!(geometry.parent, parent);
                assert_eq!(geometry.bounds, expected_bounds(parent, element.frame));
                assert_eq!(geometry.clip, parent.intersection(viewport));
                assert_eq!(layout.settings_element_geometry(state, section, element.id), Some(geometry.bounds));
                let active = !(section == 0 && disabled);
                let visible = geometry.bounds.intersection(geometry.clip);
                if visible.width >= 8 && visible.height >= 8 {
                    let point = center(visible);
                    assert_eq!(geometry.hit(point), active);
                    assert_eq!(routed_target(layout, state, section, point, width, height)
                        == Some(SettingsTarget::ExpandedAction), active);
                    hit_count += usize::from(active);
                }
                for point in [Point { x: geometry.bounds.x - 1, y: geometry.bounds.y },
                    Point { x: geometry.bounds.right(), y: geometry.bounds.y },
                    Point { x: geometry.bounds.x, y: geometry.bounds.bottom() }] {
                    assert!(!geometry.hit(point));
                }
                let collapsed = SettingsWindowState { expanded_row: None, ..state };
                let closed = layout.settings_action_geometry(collapsed, section, row).unwrap();
                assert!(!closed.visible);
                assert!(!closed.hit(center(geometry.bounds)));
            }
            if row > 0 {
                let shortened = SettingsWindowState { row_count: row, ..initial };
                if let Some(stale) = layout.settings_action_geometry(shortened, section, row) {
                    assert!(!stale.visible);
                    assert!(!stale.hit(center(stale.bounds)));
                }
            }
        }
        assert!(hit_count > 0);
        for (section, row) in [(0, 1), (3, 3), (4, 0), (2, 0), (9, 0)] {
            assert!(layout.settings_action_geometry(state(section, row), section, row).is_none());
        }
        verify_scroll_clip(layout, width, height);
        verify_relocated_target(layout, width, height, case);
        verify_presentation(layout);
    }
}

// ------------------------=
// FUNC: verify_scroll_clip
// DESC: Sweeps real scroll positions until an authored action is partially clipped, proving clipped pixels cannot receive pointer actions.
// ------------------=
fn verify_scroll_clip(layout: SystemLayout, width: usize, height: usize) {
    let initial = SettingsWindowState { height: 500, ..state(3, 0) };
    let maximum = layout.settings_window_geometry_for_section(initial, 3).maximum_scroll;
    let mut partial = false;
    let mut fully_clipped = false;
    for offset in 0..=maximum {
        let state = SettingsWindowState { scroll_offset: offset, ..initial };
        let geometry = layout.settings_action_geometry(state, 3, 0).unwrap();
        let shown = geometry.bounds.intersection(geometry.clip);
        if shown.height == 0 || shown.width == 0 {
            assert!(!geometry.hit(center(geometry.bounds)));
            fully_clipped = true;
        } else if shown.height < geometry.bounds.height && shown.height >= 8 {
            let inside = center(shown);
            assert!(geometry.hit(inside));
            assert_eq!(routed_target(layout, state, 3, inside, width, height), Some(SettingsTarget::ExpandedAction));
            let outside = Point { x: inside.x, y: if shown.y > geometry.bounds.y {
                shown.y - 1
            } else { shown.bottom() } };
            assert!(geometry.bounds.contains(outside));
            assert!(!geometry.hit(outside));
            partial = true;
        }
    }
    assert!(partial);
    assert!(fully_clipped);
}

// ------------------------=
// FUNC: verify_relocated_target
// DESC: Verifies a moved authored control disables the previous pointer area rather than retaining a second fixed-coordinate action.
// ------------------=
fn verify_relocated_target(layout: SystemLayout, width: usize, height: usize, case: &str) {
    if case != "moved" { return; }
    let state = state(0, 0);
    let geometry = layout.settings_action_geometry(state, 0, 0).unwrap();
    let source = InstallerTemplate::parse_settings(ui::installer_layout::SETTINGS_TEMPLATE_BYTES).unwrap();
    let original = source.element(1, Role::SettingsRowAction).unwrap();
    let old = expected_bounds(geometry.parent, original.frame).intersection(geometry.clip);
    let point = center(old);
    assert!(old.width >= 8 && old.height >= 8);
    assert!(!geometry.bounds.contains(point));
    assert!(!geometry.hit(point));
    assert_ne!(routed_target(layout, state, 0, point, width, height), Some(SettingsTarget::ExpandedAction));
}

// ------------------------=
// FUNC: verify_presentation
// DESC: Verifies semantic hover, press, title-control transfer, occlusion and bounded old/new damage using the production presentation and stacking state machines.
// ------------------=
fn verify_presentation(layout: SystemLayout) {
    use ui::app_launcher::window_controls as controls;
    let state=state(3,0);
    let action=layout.settings_action_geometry(state,3,0).unwrap();
    let shown=action.bounds.intersection(action.clip);
    assert!(shown.width>=8&&shown.height>=8);
    let point=center(shown);
    assert!(action.hit(point));
    controls::publish(None);let _=controls::take_damage();
    let hover=controls::Highlight{rect:action.bounds,pressed:false};
    assert!(controls::publish(Some(hover)));
    assert_eq!(controls::state(action.bounds),1);
    assert_eq!(controls::take_damage(),[None,Some(action.bounds)]);
    assert!(action.hit(Point{x:point.x+1,..point}));
    assert!(!controls::publish(Some(hover)));
    assert_eq!(controls::take_damage(),[None,None]);
    assert!(controls::publish(Some(controls::Highlight{pressed:true,..hover})));
    assert_eq!(controls::state(action.bounds),2);
    assert_eq!(controls::take_damage(),[Some(action.bounds),Some(action.bounds)]);
    let window=layout.settings_window_geometry_for_section(state,3);
    let title=controls::bounds(window.window,window.title.height,layout.scale())[0];
    assert!(controls::publish(Some(controls::Highlight{rect:title,pressed:false})));
    assert_eq!(controls::state(action.bounds),0);
    assert_eq!(controls::state(title),1);
    assert_eq!(controls::take_damage(),[Some(action.bounds),Some(title)]);

    let mut stack=ui::desktop_stack::Stack::new();
    let mut windows=[Rect::default();ui::desktop_stack::SURFACES];
    windows[4]=window.window;windows[2]=action.bounds;
    let mut visible=[false;ui::desktop_stack::SURFACES];visible[4]=true;visible[2]=true;
    stack.sync(visible,4,3);
    assert_eq!(stack.hit(windows,point),Some(4));
    controls::publish(Some(hover));let _=controls::take_damage();
    stack.sync(visible,2,3);
    assert_eq!(stack.hit(windows,point),Some(2));
    let uncovered=(stack.hit(windows,point)==Some(4)&&action.hit(point)).then_some(hover);
    assert!(controls::publish(uncovered));
    assert_eq!(controls::state(action.bounds),0);
    assert_eq!(controls::take_damage(),[Some(action.bounds),None]);
    assert_eq!(controls::take_damage(),[None,None]);
}

// ------------------------=
// FUNC: set_frame
// DESC: Mutates normalized geometry in a binary child fixture without replacing its typed identity or parent binding.
// ------------------=
fn set_frame(bytes: &mut [u8], record: Record, frame: [u16; 4]) {
    for (index, coordinate) in frame.into_iter().enumerate() {
        bytes[record.frame + index * 2..record.frame + index * 2 + 2]
            .copy_from_slice(&coordinate.to_le_bytes());
    }
}

// ------------------------=
// FUNC: verify_children
// DESC: Exercises moved row children and slider endpoints through native geometry and clipped pointer dispatch at two desktop sizes.
// ------------------=
fn verify_children(case: &str) {
    let mut bytes = ui::installer_layout::SETTINGS_TEMPLATE_BYTES.to_vec();
    let entries = records(&bytes);
    let track = *entries.iter().find(|r| r.section == 4 && r.role == 37).unwrap();
    let thumb = *entries.iter().find(|r| r.section == 4 && r.role == 38).unwrap();
    let parent = *entries.iter().filter(|r| r.section == 4 && r.role == Role::Metadata as u8).nth(3).unwrap();
    let moved = case == "child-moved";
    if moved {
        set_frame(&mut bytes, track, [430, 700, 500, 70]);
        set_frame(&mut bytes, thumb, [0, 620, 140, 260]);
    }
    match case {
        "child-baseline" | "child-moved" => {},
        "child-track-hidden" => bytes[track.flags] |= 2,
        "child-thumb-hidden" => bytes[thumb.flags] |= 2,
        "child-parent-hidden" => bytes[parent.flags] |= 2,
        "child-track-transparent" => bytes[track.opacity] = 0,
        "child-thumb-transparent" => bytes[thumb.opacity] = 0,
        "child-parent-transparent" => bytes[parent.opacity] = 0,
        _ => panic!("Unknown child fixture"),
    }
    InstallerTemplate::parse_settings(&bytes).unwrap();
    FIXTURE.set(bytes).unwrap();
    let parsed = InstallerTemplate::parse_settings(FIXTURE.get().unwrap()).unwrap();
    let track_element = parsed.element(5, Role::SettingsSliderTrack).unwrap();
    let thumb_element = parsed.element(5, Role::SettingsSliderThumb).unwrap();
    let disabled = !matches!(case, "child-baseline" | "child-moved");
    for (width, height) in [(1280, 800), (1920, 1080)] {
        let layout = SystemLayout::new(width, height);
        let mut state = state(4, 3);
        state.scroll_offset = layout.settings_reveal_scroll(state, 4);
        let detail = layout.settings_row_geometry_for_section(state, 3, 4).detail;
        let viewport = layout.settings_window_geometry_for_section(state, 4).viewport;
        let maximum = 59;
        for value in [0, 30, maximum] {
            let geometry = layout.settings_effect_slider_geometry_for_section(state, 3, value, maximum, 4);
            assert_eq!(geometry.track, expected_bounds(detail, track_element.frame));
            let thumb_width = geometry.track.width * u32::from(thumb_element.frame.width) / 1000;
            assert_eq!(geometry.thumb.width, thumb_width);
            assert_eq!(geometry.thumb.x, geometry.track.x +
                (geometry.track.width.saturating_sub(thumb_width) * u32::from(value) / u32::from(maximum)) as i32);
            assert_eq!(geometry.thumb.y, detail.y + (detail.height * u32::from(thumb_element.frame.y) / 1000) as i32);
            let point = center(geometry.thumb);
            let nx = point.x * 1000 / width as i32;
            let ny = point.y * 1000 / height as i32;
            let target = layout.settings_slider_target_for_section(nx, ny, state, 3, maximum, 4);
            if disabled || !detail.intersection(viewport).contains(layout.point(nx, ny)) {
                assert_eq!(target, None);
            } else {
                // Normalized desktop pointer rounding is at most two physical pixels.
                assert!(target.is_some_and(|actual| actual.abs_diff(value) <= 1));
            }
        }
        assert_eq!(layout.settings_slider_drag_value_for_section(-1000, state, 3, maximum, 4), 0);
        assert_eq!(layout.settings_slider_drag_value_for_section(2000, state, 3, maximum, 4), maximum);
        let collapsed = SettingsWindowState { expanded_row: None, ..state };
        assert_eq!(layout.settings_slider_target_for_section(500, 500, collapsed, 3, maximum, 4), None);
        let zero = layout.settings_effect_slider_geometry_for_section(state, 3, 0, maximum, 4);
        for point in [Point { x: zero.track.x - 5, y: center(zero.thumb).y },
            Point { x: zero.track.right() + 5, y: center(zero.thumb).y },
            Point { x: center(zero.track).x, y: viewport.y - 5 }] {
            assert_eq!(layout.settings_slider_target_for_section(point.x * 1000 / width as i32,
                point.y * 1000 / height as i32, state, 3, maximum, 4), None);
        }
        let sections = [(0, 5), (2, 5), (3, 7), (4, 5), (5, 5), (9, 5), (10, 8)];
        for (section, row_count) in sections {
            for row in 0..row_count {
                let mut state = SettingsWindowState { row_count, ..self::state(section, row) };
                state.scroll_offset = layout.settings_reveal_scroll(state, section);
                let rows: Vec<_> = entries.iter().filter(|r| r.section == section && r.role == Role::Metadata as u8).collect();
                for role in [34, 35, 36] {
                    let child = (0..parsed.element_count(section as u8 + 1).unwrap())
                        .filter_map(|index| parsed.element_at(section as u8 + 1, index))
                        .find(|element| element.role == role && element.parent_id == rows[row].id).unwrap();
                    let geometry = layout.settings_row_geometry_for_section(state, row, section);
                    let parent = if role == 34 { geometry.summary } else { geometry.detail };
                    assert_eq!(layout.settings_element_geometry(state, section, child.id), Some(expected_bounds(parent, child.frame)));
                    let closed = SettingsWindowState { expanded_row: None, ..state };
                    if role != 34 { assert_eq!(layout.settings_element_geometry(closed, section, child.id), None); }
                }
            }
        }
    }
}

// ------------------------=
// FUNC: main
// DESC: Runs each immutable authored fixture in a separate process so native template caching remains identical to production.
// ------------------=
fn main() {
    if let Some(case) = std::env::args().nth(1) {
        if case.starts_with("child-") { verify_children(&case); } else { verify_case(&case); }
        return;
    }
    for case in ["baseline", "moved", "resized", "hidden", "transparent", "parent-hidden", "parent-transparent", "literal-caption", "caption-transparent",
        "child-baseline", "child-moved", "child-track-hidden", "child-thumb-hidden", "child-parent-hidden",
        "child-track-transparent", "child-thumb-transparent", "child-parent-transparent"] {
        let status = std::process::Command::new(std::env::current_exe().unwrap()).arg(case).status().unwrap();
        assert!(status.success(), "Settings action fixture failed: {case}");
    }
}
