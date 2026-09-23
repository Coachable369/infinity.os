#[path = "../kernel/ui/app_launcher.rs"]
mod app_launcher;

use app_launcher::{LauncherRelease, LAUNCHER_APPS};

// ------------------------=
// FUNC: main
// DESC: Verifies native launcher transition, smooth-scroll, and drag-reorder state behavior.
// ------------------=
fn main() {
    shortcut_drag_and_persistence();
    for section in [6, 7, 8, 9] {
        assert_eq!(
            LAUNCHER_APPS
                .iter()
                .filter(|entry| entry.action == app_launcher::LauncherAction::Settings(section))
                .count(),
            1
        );
    }
    app_launcher::launcher_restore_default_order();

    let settled_interaction = app_launcher::launcher_interaction_state_hash();
    app_launcher::launcher_open();
    let opening = app_launcher::launcher_presentation();
    assert!(opening.transition < 255);
    app_launcher::launcher_animation_advance(240, 0);
    assert_eq!(
        app_launcher::launcher_presentation().transition,
        opening.transition
    );
    assert_eq!(
        app_launcher::launcher_interaction_state_hash(),
        settled_interaction
    );
    let mut previous_transition = opening.transition;
    for _ in 0..16 {
        let tick = app_launcher::launcher_animation_tick(240);
        let presentation = app_launcher::launcher_presentation();
        assert!(presentation.transition >= previous_transition);
        assert_eq!(
            app_launcher::launcher_interaction_state_hash(),
            settled_interaction
        );
        previous_transition = presentation.transition;
        if !tick.changed {
            break;
        }
    }
    assert_eq!(app_launcher::launcher_presentation().transition, 255);

    assert!(app_launcher::launcher_scroll_to(180, 240));
    let first_scroll = app_launcher::launcher_presentation().scroll;
    let _ = app_launcher::launcher_animation_tick(240);
    let eased_scroll = app_launcher::launcher_presentation().scroll;
    assert_eq!(first_scroll, 180);
    assert_eq!(eased_scroll, 180);
    for _ in 0..32 {
        let _ = app_launcher::launcher_animation_tick(240);
    }
    assert_eq!(app_launcher::launcher_presentation().scroll, 180);

    let first = app_launcher::launcher_visible_entry(b"", 0).unwrap();
    let third = app_launcher::launcher_visible_entry(b"", 2).unwrap();
    app_launcher::launcher_begin_drag(0, 100, 100);
    assert!(app_launcher::launcher_update_drag(Some(2), 180, 140));
    let dragging = app_launcher::launcher_presentation();
    assert!(dragging.drag_moved);
    assert_eq!(app_launcher::launcher_display_slot(1, dragging), 0);
    assert_eq!(
        app_launcher::launcher_finish_drag(b""),
        LauncherRelease::Reordered
    );
    assert_eq!(
        app_launcher::launcher_visible_entry(b"", 0),
        Some(LAUNCHER_APPS[1])
    );
    assert_eq!(app_launcher::launcher_visible_entry(b"", 1), Some(third));
    assert_eq!(app_launcher::launcher_visible_entry(b"", 2), Some(first));
    // Cross rows in both directions, then return horizontally left.
    for (source, target) in [(2, 14), (14, 6), (6, 7), (7, 0)] {
        let moved = app_launcher::launcher_visible_entry(b"", source).unwrap();
        app_launcher::launcher_begin_drag(source, 500, 500);
        app_launcher::launcher_update_drag(Some(target), 700, 700);
        assert_eq!(
            app_launcher::launcher_finish_drag(b""),
            LauncherRelease::Reordered
        );
        assert_eq!(
            app_launcher::launcher_visible_entry(b"", target),
            Some(moved)
        );
        let mut ids = app_launcher::launcher_presentation().order;
        ids.sort();
        assert_eq!(ids, core::array::from_fn(|i| i as u8));
    }

    app_launcher::launcher_begin_close();
    let mut closed = false;
    for _ in 0..16 {
        let tick = app_launcher::launcher_animation_tick(240);
        closed |= tick.closed;
        if closed {
            break;
        }
    }
    assert!(closed);
    assert_eq!(app_launcher::launcher_presentation().transition, 0);

    app_launcher::launcher_restore_default_order();
    println!("App launcher interaction behavior: PASS");
}
#[path = "../kernel/ui/geometry.rs"]
pub mod geometry;
mod ui {
    pub use crate::geometry;
}

// ------------------------=
// FUNC: shortcut_drag_and_persistence
// DESC: Exercises copy semantics, click slop, two-axis capture, bounded damage and corrupt-record rejection.
// ------------------=
fn shortcut_drag_and_persistence() {
    use app_launcher::shortcuts::{self, State};
    let path=shortcuts::user_path([0;16]);
    assert!(path.iter().all(|b|*b!=0));
    for id in 1..=255u8 { assert_ne!(path,shortcuts::user_path([id;16])); }
    let mut s = State::new();
    s.begin(6, 100, 200, false);
    s.motion(103, 201);
    assert!(!s.drag.unwrap().3);
    s.motion(400, 500);
    assert!(s.drag.unwrap().3);
    s.place(6, 400, 500);
    s.drag = None;
    let order = app_launcher::launcher_presentation().order;
    let encoded = s.encode(order);
    let (restored, restored_order) = State::decode(&encoded).unwrap();
    assert_eq!(restored.positions, s.positions);
    assert_eq!(restored.drag, None);
    assert_eq!(restored_order, order);
    assert_eq!(
        app_launcher::launcher_visible_count(b""),
        LAUNCHER_APPS.len()
    );
    assert_eq!(s.positions[6], [400, 500]);
    let mut corrupt = encoded;
    corrupt[30] ^= 1;
    assert!(State::decode(&corrupt).is_none());
    let mut invalid = order;
    invalid[0] = invalid[1];
    assert!(State::decode(&s.encode(invalid)).is_none());
    assert!(State::decode(&encoded[..83]).is_none());
    shortcuts::publish(s);
    let _ = shortcuts::take_damage(1920, 1080);
    s.begin(6, 400, 500, true);
    s.motion(410, 510);
    assert_eq!(s.stationary_positions()[6], [0, 0]);
    assert_eq!(s.positions[6], [400, 500]);
    let mut cancelled = s;
    cancelled.drag = None;
    assert_eq!(cancelled.stationary_positions()[6], [400, 500]);
    shortcuts::publish(s);
    let (_, _, w, h) = shortcuts::take_damage(1920, 1080).unwrap();
    assert!(w * h < 1920 * 1080 / 8);
    assert!(shortcuts::take_damage(1920, 1080).is_none());
    shortcuts::publish(State::new());
    let _ = shortcuts::take_changed();
}
