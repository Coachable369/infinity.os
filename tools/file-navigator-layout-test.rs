#![allow(dead_code)]

#[path = "../kernel/ui/mod.rs"]
mod ui;

use ui::system_layout::{DesktopTarget, SystemLayout};

// ------------------------=
// FUNC: main
// DESC: Verifies File Navigator exposes only List and Grid toolbar modes and a four-row context menu.
// ------------------=
fn main() {
    let layout = SystemLayout::new(1600, 1000);
    let (left, top, width, _) = layout.home_window_geometry_sized(80, 180, 780, 560, false);
    let toolbar_y = top + 34 + 18;
    let normalized_y = (toolbar_y as i32 * 1000 / 1000) as i32;
    let mut mode_actions = [false; 6];
    for physical_x in left..left + width {
        let normalized_x = (physical_x as i32 * 1000 / 1600) as i32;
        if let Some(DesktopTarget::HomeToolbar(action)) = layout.desktop_target_sized(
            normalized_x,
            normalized_y,
            80,
            180,
            780,
            560,
            true,
            false,
        ) {
            if action < mode_actions.len() {
                mode_actions[action] = true;
            }
        }
    }
    assert!(mode_actions[3]);
    assert!(mode_actions[4]);
    assert!(!mode_actions[5], "removed Inspector control must not retain a hit target");

    let menu = layout.file_navigator_context_geometry(500, 500);
    assert_eq!(menu.height, 120);
    let row_five_x = ((menu.x + 20) * 1000 / 1600) as i32;
    let row_five_y = ((menu.y + 6 + 28 * 4 + 10) * 1000 / 1000) as i32;
    assert_eq!(
        layout.file_navigator_context_action(500, 500, row_five_x, row_five_y),
        None
    );
    println!("PASS File Navigator layout: full-width content with no Inspector controls");
}
