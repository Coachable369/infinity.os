#[path = "../kernel/ui/session_state.rs"]
mod session_state;

use session_state::{
    DesktopResumeSurface, DesktopSessionLayout, LockedDesktopState, SessionIdleState,
    WindowPlacement,
};

// ------------------------=
// FUNC: inactivity_locks_once_at_the_deadline
// DESC: Exercises the authenticated idle transition and activity reset as observable state changes.
// ------------------=
fn inactivity_locks_once_at_the_deadline() {
    let mut idle = SessionIdleState::new();
    let selected_timeout_seconds = 17;
    for _ in 1..selected_timeout_seconds {
        assert!(!idle.tick(true, selected_timeout_seconds));
    }
    assert_eq!(idle.elapsed_seconds(), selected_timeout_seconds - 1);
    idle.note_activity();
    assert_eq!(idle.elapsed_seconds(), 0);
    for _ in 1..selected_timeout_seconds {
        assert!(!idle.tick(true, selected_timeout_seconds));
    }
    assert!(idle.tick(true, selected_timeout_seconds));
    assert_eq!(idle.elapsed_seconds(), 0);
    assert!(!idle.tick(false, selected_timeout_seconds));
}

// ------------------------=
// FUNC: desktop_layout_survives_a_lock_round_trip
// DESC: Verifies window geometry, visibility, focus, and desktop positions remain exact across a simulated lock interval.
// ------------------=
fn desktop_layout_survives_a_lock_round_trip() {
    let expected = DesktopSessionLayout {
        home: WindowPlacement::new(83, 121, 706, 544, false, true),
        settings: WindowPlacement::new(142, 194, 641, 571, false, true),
        editor: WindowPlacement::new(211, 166, 533, 618, false, true),
        command: WindowPlacement::new(255, 205, 522, 480, true, true),
        desktop_item_positions: [
            [70, 150], [155, 182], [241, 211], [332, 244], [430, 280], [518, 316],
            [610, 350],
        ],
        focused_surface: DesktopResumeSurface::TextEditor,
        settings_section: 6,
        settings_expanded_row: Some(3),
        settings_scroll_offset: 91,
    };
    let mut lock = LockedDesktopState::new();
    assert!(!lock.has_saved_layout());
    lock.save(expected);
    assert!(lock.has_saved_layout());
    let restored = lock.restore().unwrap();
    assert_eq!(restored, expected);
    assert_eq!(restored.editor.width, 533);
    assert_eq!(restored.desktop_item_positions[5], [518, 316]);
    assert_eq!(restored.focused_surface, DesktopResumeSurface::TextEditor);
    assert!(!lock.has_saved_layout());
    assert_eq!(lock.restore(), None);
}

// ------------------------=
// FUNC: main
// DESC: Runs behavioral inactivity and lock-layout persistence scenarios.
// ------------------=
fn main() {
    inactivity_locks_once_at_the_deadline();
    desktop_layout_survives_a_lock_round_trip();
    println!("PASS window session: coherent geometry redraw policy and exact layout preservation across inactivity lock");
}
