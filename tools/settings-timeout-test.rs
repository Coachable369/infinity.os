#![allow(dead_code)]

#[path = "../kernel/runtime/mod.rs"]
mod runtime;
#[path = "storage.rs"]
mod storage;
#[path = "../kernel/ui/mod.rs"]
mod ui;

// ------------------------=
// FUNC: output_text
// DESC: Provides the diagnostic sink required by the host runtime harness.
// ------------------=
fn output_text(_: &[u8]) {}

use runtime::identity::{
    IdentityError, IdentitySystem, DEFAULT_NO_ACTIVITY_TIMEOUT_MINUTES,
    MAX_NO_ACTIVITY_TIMEOUT_MINUTES, MIN_NO_ACTIVITY_TIMEOUT_MINUTES,
};
use ui::session_state::SessionIdleState;
use ui::system_layout::{SettingsWindowState, SystemLayout};

// ------------------------=
// FUNC: user_timeout_round_trip
// DESC: Verifies ownership, range validation, and durable per-user timeout isolation.
// ------------------=
fn user_timeout_round_trip() {
    let mut identities = IdentitySystem::new();
    identities.begin_onboarding().unwrap();
    identities.create_machine(b"TimeoutNode", 0x8664, 1, 1).unwrap();
    let owner = identities.create_user(b"owner", b"Owner", 2).unwrap();
    let guest = identities.create_user(b"guest", b"Guest", 3).unwrap();
    assert_eq!(
        identities.user_profile(owner.id).unwrap().no_activity_timeout_minutes,
        DEFAULT_NO_ACTIVITY_TIMEOUT_MINUTES
    );
    assert_eq!(
        identities.update_user_no_activity_timeout(guest.id, owner.id, 30),
        Err(IdentityError::AccessDenied)
    );
    assert_eq!(
        identities.update_user_no_activity_timeout(owner.id, owner.id, 0),
        Err(IdentityError::InvalidInput)
    );
    identities
        .update_user_no_activity_timeout(owner.id, owner.id, 45)
        .unwrap();
    let restored = IdentitySystem::decode(&identities.encode()).unwrap();
    assert_eq!(restored.user_profile(owner.id).unwrap().no_activity_timeout_minutes, 45);
    assert_eq!(
        restored.user_profile(guest.id).unwrap().no_activity_timeout_minutes,
        DEFAULT_NO_ACTIVITY_TIMEOUT_MINUTES
    );
}

// ------------------------=
// FUNC: selected_deadline_controls_lock
// DESC: Verifies observable idle state locks at the selected deadline and resets after activity.
// ------------------=
fn selected_deadline_controls_lock() {
    let mut idle = SessionIdleState::new();
    let seconds = 3;
    assert!(!idle.tick(true, seconds));
    assert!(!idle.tick(true, seconds));
    idle.note_activity();
    assert_eq!(idle.elapsed_seconds(), 0);
    assert!(!idle.tick(true, seconds));
    assert!(!idle.tick(true, seconds));
    assert!(idle.tick(true, seconds));
}

// ------------------------=
// FUNC: slider_maps_complete_timeout_range
// DESC: Verifies the rendered slider maps both endpoints and a middle value through typed geometry.
// ------------------=
fn slider_maps_complete_timeout_range() {
    let layout = SystemLayout::new(1600, 1000);
    let state = SettingsWindowState {
        x: 160,
        y: 210,
        width: 680,
        height: 620,
        maximized: false,
        expanded_row: Some(3),
        scroll_offset: 0,
        row_count: 5,
    };
    let maximum = MAX_NO_ACTIVITY_TIMEOUT_MINUTES - MIN_NO_ACTIVITY_TIMEOUT_MINUTES;
    assert_eq!(layout.settings_slider_drag_value(0, state, 3, maximum), 0);
    assert_eq!(layout.settings_slider_drag_value(1000, state, 3, maximum), maximum);
    let geometry = layout.settings_effect_slider_geometry(state, 3, 44, maximum);
    let x = (geometry.thumb.x + (geometry.thumb.width / 2) as i32) * 1000 / 1600;
    let y = (geometry.thumb.y + (geometry.thumb.height / 2) as i32) * 1000 / 1000;
    assert_eq!(layout.settings_slider_target(x, y, state, 3, maximum), Some(44));
}

// ------------------------=
// FUNC: main
// DESC: Runs the user inactivity-timeout behavioral acceptance scenarios.
// ------------------=
fn main() {
    user_timeout_round_trip();
    selected_deadline_controls_lock();
    slider_maps_complete_timeout_range();
    println!("PASS user timeout: persistent isolation, selected idle deadline, and complete slider range");
}
