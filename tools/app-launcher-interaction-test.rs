#[path = "../kernel/ui/app_launcher.rs"]
mod app_launcher;

use app_launcher::{LauncherRelease, LAUNCHER_APPS};

// ------------------------=
// FUNC: main
// DESC: Verifies native launcher transition, smooth-scroll, and drag-reorder state behavior.
// ------------------=
fn main() {
    app_launcher::launcher_restore_default_order();

    let settled_interaction = app_launcher::launcher_interaction_state_hash();
    app_launcher::launcher_open();
    let opening = app_launcher::launcher_presentation();
    assert!(opening.transition < 255);
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
    assert!(eased_scroll > first_scroll && eased_scroll < 180);
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
    assert_eq!(app_launcher::launcher_finish_drag(b""), LauncherRelease::Reordered);
    assert_eq!(app_launcher::launcher_visible_entry(b"", 0), Some(LAUNCHER_APPS[1]));
    assert_eq!(app_launcher::launcher_visible_entry(b"", 1), Some(third));
    assert_eq!(app_launcher::launcher_visible_entry(b"", 2), Some(first));

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
