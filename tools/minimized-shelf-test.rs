#![allow(dead_code)]
#[path = "../kernel/ui/geometry.rs"]
pub mod geometry;
mod ui {
    pub use crate::geometry;
}
#[path = "../kernel/ui/minimized_shelf.rs"]
mod shelf;
use geometry::Point;
use shelf::*;
#[test]
// ------------------------=
// FUNC: drawer_anchors_and_tracks_drag_without_losing_apps
// DESC: Verifies both edges, inward menus, and live drag geometry at different display sizes.
// ------------------=
fn drawer_anchors_and_tracks_drag_without_losing_apps() {
    for (w, h) in [(800, 600), (1920, 1080), (3840, 2160)] {
        let mut state = State::new();
        state.set(EDITOR, true);
        state.menu = Some(EDITOR);
        let right = Geometry::new(w, h, state);
        assert!(right.menu.right() < right.rail.x);
        state.left = true;
        let left = Geometry::new(w, h, state);
        assert!(left.menu.x > left.rail.right());
        assert_eq!(left.rail.x, (w / 100) as i32);
        state.drag = Some((400, 10));
        let moving = Geometry::new(w, h, state);
        assert_eq!(moving.rail.x, (w * 400 / 1000) as i32);
        assert_eq!(state.item(0), Some(EDITOR));
        state.drag = None;
        assert_eq!(Geometry::new(w, h, state).rail, left.rail);
    }
}

#[test]
// ------------------------=
// FUNC: lifecycle_keeps_exact_minimized_identities
// DESC: Verifies minimize, restore, close, and menu dismissal without manufacturing closed apps.
// ------------------=
fn lifecycle_keeps_exact_minimized_identities() {
    let mut state = State::new();
    assert_eq!(state.count(), 0);
    state.set(2, true);
    state.set(EDITOR, true);
    state.set(COMMAND, true);
    assert_eq!(state.item(0), Some(2));
    assert_eq!(state.item(1), Some(COMMAND));
    assert_eq!(state.item(2), Some(EDITOR));
    state.menu = Some(EDITOR);
    state.row = Some(2);
    state.set(EDITOR, false);
    assert_eq!(state.menu, None);
    assert_eq!(state.row, None);
    assert_eq!(state.count(), 2);
    state.set(2, false);
    assert_eq!(state.item(0), Some(COMMAND));
    state.set(COMMAND, false);
    assert_eq!(state.count(), 0);
    state.set(COUNT, true);
    assert_eq!(state.count(), 0);
}

#[test]
// ------------------------=
// FUNC: shelf_overflow_geometry_and_menu_actions_are_reachable
// DESC: Tests representative resolutions, exact targets, bounded overflow, and all three action rows.
// ------------------=
fn shelf_overflow_geometry_and_menu_actions_are_reachable() {
    for (width, height) in [
        (800, 600),
        (1024, 768),
        (1440, 900),
        (1920, 1080),
        (2560, 1440),
        (3440, 1440),
    ] {
        let mut state = State::new();
        for id in 0..COUNT {
            state.set(id, true);
        }
        let g = Geometry::new(width, height, state);
        assert!(g.rail.height > g.rail.width);
        assert!(g.rail.right() < width as i32);
        assert!(g.rail.bottom() < height as i32 * 85 / 100);
        for offset in 0..COUNT {
            state.scroll(1, g.capacity);
            assert!(state.offset <= state.count().saturating_sub(g.capacity));
            for row in 0..g.capacity {
                let tile = g.tile_rect(row);
                assert!(tile.bottom() <= g.rail.bottom());
                let point = Point {
                    x: tile.x + tile.width as i32 / 2,
                    y: tile.y + tile.height as i32 / 2,
                };
                assert_eq!(g.hit(state, point), state.item(state.offset + row));
            }
            state.menu = state.item(state.offset);
            let menu = Geometry::new(width, height, state);
            assert!(menu.menu.right() < g.rail.x);
            assert!(menu.menu.bottom() <= height as i32);
            for action in 0..3 {
                assert_eq!(
                    menu.menu_row(Point {
                        x: menu.menu.x + 20,
                        y: menu.menu.y + (action as i32 + 1) * menu.row_height as i32 + 5
                    }),
                    Some(action)
                );
            }
            assert!(
                menu.damage(state).width * menu.damage(state).height < (width * height / 3) as u32,
                "resolution {width}x{height}, offset {offset}"
            );
        }
        state.scroll(-100, g.capacity);
        assert_eq!(state.offset, 0);
    }
}

#[test]
// ------------------------=
// FUNC: unchanged_pointer_state_has_no_scene_damage
// DESC: Tests the production dirty-state gate and bounded coverage for opening and dismissing a menu.
// ------------------=
fn unchanged_pointer_state_has_no_scene_damage() {
    publish(State::new());
    let _ = take_damage(1920, 1080);
    assert_eq!(take_damage(1920, 1080), None);
    set(EDITOR, true);
    let damage = take_damage(1920, 1080).unwrap();
    assert!(damage.width < 1920 / 8);
    assert_eq!(take_damage(1920, 1080), None);
    let mut state = current();
    state.menu = Some(EDITOR);
    publish(state);
    let menu = Geometry::new(1920, 1080, state).menu;
    assert!(take_damage(1920, 1080).unwrap().intersection(menu) == menu);
    state.menu = None;
    publish(state);
    assert!(take_damage(1920, 1080).unwrap().intersection(menu) == menu);
    assert_eq!(take_damage(1920, 1080), None);
}
