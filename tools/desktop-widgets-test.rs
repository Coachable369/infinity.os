#![allow(dead_code)]
#[path = "../kernel/ui/desktop_widgets.rs"]
mod desktop_widgets;
#[path = "../kernel/ui/geometry.rs"]
mod geometry;
use desktop_widgets::State;
use geometry::Point;
#[test]
// ------------------------=
// FUNC: drag_snap_visibility_and_persistence
// DESC: Tests production pointer capture, release snapping, bounded layout and durable hidden state.
// ------------------=
fn drag_snap_visibility_and_persistence() {
    for (w, h, s) in [(800, 600, 1), (1920, 1080, 1), (3840, 2160, 2)] {
        let mut state = State::new();
        for id in 0..2 {
            let r = state.rect(id, w, h, s, false);
            state.drag = Some((id, 10, 10));
            state.move_pointer(Point { x: 110, y: 110 }, true, w, h, s, false);
            assert!(state.drag.is_some());
            assert_ne!(r, state.rect(id, w, h, s, false));
            state.move_pointer(
                Point {
                    x: 10 + (8 * s) as i32,
                    y: 110,
                },
                false,
                w,
                h,
                s,
                false,
            );
            let r = state.rect(id, w, h, s, false);
            assert!(state.drag.is_none());
            assert!((r.x - (8 * s) as i32).abs() <= 4);
            assert!(r.x >= 0 && r.y >= 0 && r.right() <= w as i32 && r.bottom() <= h as i32);
        }
        state.visible = 0;
        state.menu = Some(Point {
            x: w as i32,
            y: h as i32,
        });
        let menu = state.menu_rect(w, h, s).unwrap();
        assert!(menu.right() <= w as i32 && menu.bottom() <= h as i32);
        let restored = State::decode(state.encode());
        assert_eq!(restored.positions, state.positions);
        assert_eq!(restored.visible, 0);
        assert_eq!(restored.menu, None);
        assert_eq!(State::decode(0), State::new());
    }
}
#[test]
// ------------------------=
// FUNC: damage_is_bounded_and_idle_is_clean
// DESC: Proves header capture alone does not repaint and a small move damages only that widget's coverage.
// ------------------=
fn damage_is_bounded_and_idle_is_clean() {
    use desktop_widgets::*;
    publish(State::new());
    let _ = take_damage(1920, 1080, 1);
    assert_eq!(take_damage(1920, 1080, 1), None);
    let mut state = current();
    state.drag = Some((0, 10, 10));
    publish(state);
    assert_eq!(take_damage(1920, 1080, 1), None);
    let old = state.rect(0, 1920, 1080, 1, false);
    state.move_pointer(
        Point {
            x: old.x + 20,
            y: old.y + 20,
        },
        true,
        1920,
        1080,
        1,
        false,
    );
    publish(state);
    let damage = take_damage(1920, 1080, 1).unwrap();
    assert!((damage.width as usize * damage.height as usize) < 1920 * 1080 / 4);
    assert!(damage.contains(Point { x: old.x, y: old.y }));
    assert_eq!(take_damage(1920, 1080, 1), None);
}
