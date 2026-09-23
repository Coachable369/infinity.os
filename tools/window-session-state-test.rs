#[path = "../kernel/ui/session_state.rs"]
mod session_state;

use session_state::{
    DesktopResumeSurface, DesktopSessionLayout, LockedDesktopState, PersistentDesktopLayoutStore,
    SessionIdleState, WindowPlacement,
};

// ------------------------=
// FUNC: example_layout
// DESC: Produces a non-default layout for lock and durable-session round trips.
// ------------------=
fn example_layout() -> DesktopSessionLayout {
    DesktopSessionLayout {
        home: WindowPlacement::new(83, 121, 706, 544, false, true),
        settings: WindowPlacement::new(142, 194, 641, 571, false, true),
        editor: WindowPlacement::new(211, 166, 533, 618, false, true),
        command: WindowPlacement::new(255, 205, 522, 480, true, true),
        task_manager: WindowPlacement::new(188, 146, 720, 610, false, true),
        desktop_item_positions: [
            [70, 150],
            [155, 182],
            [241, 211],
            [332, 244],
            [430, 280],
            [518, 316],
            [610, 350],
        ],
        focused_surface: DesktopResumeSurface::TextEditor,
        settings_section: 6,
        settings_expanded_row: Some(3),
        settings_scroll_offset: 91,
        input_preferences: [0xa1, 7, 8, 3, 2, 3, 0, 0],
        app_drawer_left: true,
        app_drawer_floating: [301, 201],
    }
}

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
    let expected = example_layout();
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
// FUNC: desktop_layout_survives_a_durable_session_round_trip
// DESC: Verifies separate users recover exact window geometry after encoding and reconstructing state.
// ------------------=
fn desktop_layout_survives_a_durable_session_round_trip() {
    let first_user = [0x11; 16];
    let second_user = [0x22; 16];
    let expected = example_layout();
    let mut second_layout = expected;
    second_layout.home = WindowPlacement::new(25, 75, 900, 700, true, false);
    second_layout.focused_surface = DesktopResumeSurface::Settings;

    let mut store = PersistentDesktopLayoutStore::new();
    assert!(store.save(first_user, expected));
    assert!(store.save(second_user, second_layout));
    let encoded = store.encode();
    let restored = PersistentDesktopLayoutStore::decode(&encoded).unwrap();
    assert_eq!(restored.layout(first_user), Some(expected));
    assert_eq!(restored.layout(second_user), Some(second_layout));
    assert_eq!(restored.layout([0x33; 16]), None);

    let mut corrupt = encoded;
    corrupt[114] ^= 0x40;
    assert_eq!(PersistentDesktopLayoutStore::decode(&corrupt), None);
}

// ------------------------=
// FUNC: main
// DESC: Runs behavioral inactivity and lock-layout persistence scenarios.
// ------------------=
fn main() {
    inactivity_locks_once_at_the_deadline();
    desktop_layout_survives_a_lock_round_trip();
    desktop_layout_survives_a_durable_session_round_trip();
    println!("PASS window session: exact per-user layouts survive lock and durable session reconstruction");
}
