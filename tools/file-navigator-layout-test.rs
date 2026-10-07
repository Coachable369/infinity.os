#![allow(dead_code)]

#[path = "../kernel/ui/mod.rs"]
mod ui;

use ui::system_layout::{DesktopTarget, SystemLayout};

// ------------------------=
// FUNC: main
// DESC: Verifies File Navigator toolbar, app-menu, modal, and bounded context-menu hit geometry.
// ------------------=
fn main() {
    for scale in [1, 2] {
        let window = ui::geometry::Rect {
            x: 100,
            y: 100,
            width: 600,
            height: 500,
        };
        for dx in 1..=15 * scale {
            for dy in 1..=15 * scale {
                assert_eq!(
                    ui::system_layout::native_window_resize_target(
                        window,
                        ui::geometry::Point {
                            x: window.right() - dx as i32,
                            y: window.bottom() - dy as i32
                        },
                        scale
                    ),
                    Some(3)
                );
            }
        }
        let resized = ui::system_layout::resize_home_window(100, 100, 400, 400, 3, 650, 650);
        assert!(resized.2 > 400 && resized.3 > 400);
    }
    let layout = SystemLayout::new(1600, 1000);
    let (x, y, w, h) = layout.home_window_geometry_sized(80, 180, 780, 560, false);
    for index in (0..9).chain(core::iter::once(11)) {
        let row = layout.navigator_sidebar_row(ui::geometry::Rect {
            x: x as i32, y: y as i32, width: w as u32, height: h as u32,
        }, index);
        assert_eq!(layout.desktop_target_sized(
            (row.x + row.width as i32 / 2) * 1000 / 1600,
            row.y + row.height as i32 / 2,
            80, 180, 780, 560, true, false), Some(DesktopTarget::HomeSidebar(index)));
    }
    let (left, top, width, _) = layout.home_window_geometry_sized(80, 180, 780, 560, false);
    let toolbar_y = top + 34 + 18;
    let normalized_y = (toolbar_y as i32 * 1000 / 1000) as i32;
    let mut mode_actions = [false; 6];
    for physical_x in left..left + width {
        let normalized_x = (physical_x as i32 * 1000 / 1600) as i32;
        if let Some(DesktopTarget::HomeToolbar(action)) =
            layout.desktop_target_sized(normalized_x, normalized_y, 80, 180, 780, 560, true, false)
        {
            if action < mode_actions.len() {
                mode_actions[action] = true;
            }
        }
    }
    assert!(mode_actions[3]);
    assert!(mode_actions[4]);
    assert!(
        !mode_actions[5],
        "preview belongs to the View menu rather than the toolbar"
    );

    let file_label_x = ((left + 176) as i32 * 1000 / 1600) as i32;
    let title_y = ((top + 16) as i32 * 1000 / 1000) as i32;
    assert_eq!(
        layout.file_navigator_overlay_target(
            file_label_x,
            title_y,
            80,
            180,
            780,
            560,
            false,
            0,
            0,
            0,
        ),
        Some(DesktopTarget::HomeMenu(1))
    );
    let performance_label_x = ((left + 240) as i32 * 1000 / 1600) as i32;
    assert_eq!(
        layout.file_navigator_overlay_target(
            performance_label_x,
            title_y,
            80,
            180,
            780,
            560,
            false,
            0,
            0,
            0,
        ),
        Some(DesktopTarget::HomeMenu(2))
    );
    let performance_menu = layout.file_navigator_menu_geometry(left, top, 2, 4);
    let expanded_x = ((performance_menu.x + 40) * 1000 / 1600) as i32;
    let expanded_y = ((performance_menu.y + 6 + 2 * 30 + 12) * 1000 / 1000) as i32;
    assert_eq!(
        layout.file_navigator_overlay_target(
            expanded_x, expanded_y, 80, 180, 780, 560, false, 2, 4, 0,
        ),
        Some(DesktopTarget::HomeMenuItem(2))
    );
    let file_menu = layout.file_navigator_menu_geometry(left, top, 1, 4);
    let empty_trash_x = ((file_menu.x + 40) * 1000 / 1600) as i32;
    let empty_trash_y = ((file_menu.y + 6 + 2 * 30 + 12) * 1000 / 1000) as i32;
    assert_eq!(
        layout.file_navigator_overlay_target(
            empty_trash_x,
            empty_trash_y,
            80,
            180,
            780,
            560,
            false,
            1,
            4,
            0,
        ),
        Some(DesktopTarget::HomeMenuItem(2))
    );
    let dialog = layout.file_navigator_dialog_geometry(left, top, width, 560);
    let primary_x = ((dialog.x + dialog.width as i32 * 3 / 4) * 1000 / 1600) as i32;
    let primary_y = ((dialog.bottom() - 34) * 1000 / 1000) as i32;
    assert_eq!(
        layout.file_navigator_overlay_target(
            primary_x,
            primary_y,
            80,
            180,
            780,
            560,
            false,
            0,
            0,
            1,
        ),
        Some(DesktopTarget::HomeDialogAction(0))
    );

    let menu = layout.file_navigator_context_geometry(500, 500, 4);
    assert_eq!(menu.height, 124);
    let row_five_x = ((menu.x + 20) * 1000 / 1600) as i32;
    let row_five_y = ((menu.y + 6 + 28 * 4 + 10) * 1000 / 1000) as i32;
    assert_eq!(
        layout.file_navigator_context_action(500, 500, row_five_x, row_five_y, 4),
        None
    );
    assert_eq!(layout.file_navigator_context_action(500, 500, row_five_x, row_five_y, 5), Some(4));
    for rows in 1..=9 {
        let bounds = layout.file_navigator_context_geometry(990, 990, rows);
        assert_eq!(bounds.height, (12 + rows * 28) as u32);
        assert!(bounds.right() <= 1600 && bounds.bottom() <= 1000);
        for row in 0..rows {
            assert_eq!(layout.file_navigator_context_action(990, 990,
                (bounds.x + 20) * 1000 / 1600, bounds.y + 6 + row as i32 * 28 + 10, rows), Some(row));
        }
    }
    for (width, height) in [(1024, 600), (1024, 768), (2560, 1440)] {
        let layout = SystemLayout::new(width, height);
        let scale = layout.scale() as i32;
        let bounds = layout.file_navigator_context_geometry(990, 990, 9);
        assert_eq!(bounds.height, 264 * scale as u32);
        assert!(bounds.right() <= width as i32 && bounds.bottom() <= height as i32);
        for row in 0..9 {
            let x = (bounds.x + 20 * scale) * 1000 / width as i32;
            let y = (bounds.y + (6 + row * 28 + 14) * scale) * 1000 / height as i32;
            assert_eq!(layout.file_navigator_context_action(990, 990, x, y, 9), Some(row as usize));
        }
    }
    println!(
        "PASS File Navigator layout: toolbar, drop-down menus, modal actions, and context bounds"
    );
}
