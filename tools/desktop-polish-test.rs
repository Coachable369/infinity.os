#![allow(dead_code)]
#[path="../kernel/ui/desktop_template.rs"] mod desktop_template;
#[path="../kernel/ui/installer_template.rs"] mod installer_template;
#[path="../kernel/ui/installer_layout.rs"] mod installer_layout;
#[path="../kernel/ui/bitmap.rs"] mod bitmap;

#[path = "../kernel/ui/desktop_polish.rs"]
mod desktop_polish;
#[path = "../kernel/ui/geometry.rs"]
mod geometry;

// ------------------------=
// FUNC: normalized
// DESC: Converts a physical test coordinate to the public normalized pointer contract.
// ------------------=
fn normalized(pixel: usize, extent: usize) -> i32 {
    (pixel.saturating_mul(1000) / extent.max(1)) as i32
}

// ------------------------=
// FUNC: exposes_exactly_ten_polishes
// DESC: Locks the requested bounded polish scope to the design-kit checklist.
// ------------------=
#[test]
fn exposes_exactly_ten_polishes() {
    assert_eq!(desktop_polish::POLISH_COUNT, 10);
    assert_eq!(desktop_polish::DOCK_ITEMS, 8);
}

// ------------------------=
// FUNC: hover_targets_respect_shell_geometry_and_gutters
// DESC: Exercises semantic hover hit regions and verifies dock gaps remain inert.
// ------------------=
#[test]
fn hover_targets_respect_shell_geometry_and_gutters() {
    let (width, height) = (1920usize, 1080usize);
    assert_eq!(
        desktop_polish::target(width, height, normalized(40, width), normalized(18, height)),
        desktop_polish::HoverTarget::Brand
    );
    assert_eq!(
        desktop_polish::target(
            width,
            height,
            normalized(width*400/1000, width),
            normalized(18, height)
        ),
        desktop_polish::HoverTarget::Help
    );
    let status_left = width*714/1000;
    assert_eq!(
        desktop_polish::target(
            width,
            height,
            normalized(status_left + 16, width),
            normalized(18, height),
        ),
        desktop_polish::HoverTarget::Status(0)
    );
    assert_eq!(
        desktop_polish::target(
            width,
            height,
            normalized(width - 54, width),
            normalized(18, height)
        ),
        desktop_polish::HoverTarget::Clock
    );
    let dock_width = width * 54 / 100;
    let dock_left = (width - dock_width) / 2;
    let gap = dock_width / desktop_polish::DOCK_ITEMS;
    let dock_top = height - 82;
    let first_icon_left = dock_left + gap / 2;
    assert_eq!(
        desktop_polish::target(
            width,
            height,
            normalized(first_icon_left + 20, width),
            normalized(dock_top + 24, height),
        ),
        desktop_polish::HoverTarget::Dock(0)
    );
    assert_eq!(
        desktop_polish::target(
            width,
            height,
            normalized(first_icon_left + 75, width),
            normalized(dock_top + 24, height),
        ),
        desktop_polish::HoverTarget::None
    );
}

// ------------------------=
// FUNC: hover_publication_changes_only_across_semantic_boundaries
// DESC: Verifies ordinary movement within one control does not request another shell repaint.
// ------------------=
#[test]
fn hover_publication_changes_only_across_semantic_boundaries() {
    let (width, height) = (1920usize, 1080usize);
    assert!(desktop_polish::publish(
        width,
        height,
        normalized(40, width),
        normalized(18, height)
    ));
    assert!(!desktop_polish::publish(
        width,
        height,
        normalized(45, width),
        normalized(18, height)
    ));
    assert!(desktop_polish::publish(
        width,
        height,
        normalized(900, width),
        normalized(500, height)
    ));
    assert!(!desktop_polish::publish(
        width,
        height,
        normalized(910, width),
        normalized(510, height)
    ));
}

// ------------------------=
// FUNC: running_indicators_follow_actual_surface_state
// DESC: Verifies launcher, Files, Settings, About, AI and Appearance indicators derive from visible state.
// ------------------=
#[test]
fn running_indicators_follow_actual_surface_state() {
    let mut visible = [false; 6];
    assert!(desktop_polish::dock_running(0, true, visible, 0, false));
    visible[0] = true;
    assert!(desktop_polish::dock_running(1, false, visible, 0, false));
    visible[4] = true;
    assert!(desktop_polish::dock_running(2, false, visible, 0, false));
    assert!(!desktop_polish::dock_running(3, false, visible, 0, false));
    assert!(desktop_polish::dock_running(3, false, visible, 9, false));
    assert!(desktop_polish::dock_running(4, false, visible, 3, false));
    assert!(desktop_polish::dock_running(5, false, visible, 1, false));
    assert!(!desktop_polish::dock_running(6, false, visible, 1, false));
    assert!(!desktop_polish::dock_running(7, false, visible, 1, false));
}
