#![allow(dead_code)]

#[path = "../kernel/runtime/mod.rs"]
mod runtime;
#[path = "storage.rs"]
mod storage;
#[path = "../kernel/ui/mod.rs"]
mod ui;

mod console {
    #[derive(Clone, Copy)]
    pub enum ConsoleKey {
        Character(u8),
        Backspace,
        Delete,
        Left,
        Right,
        Home,
        End,
        Enter,
    }
}

// ------------------------=
// FUNC: output_text
// DESC: Provides the diagnostic sink required by the host runtime harness.
// ------------------=
fn output_text(_: &[u8]) {}

use runtime::identity::{IdentityError, IdentitySystem};
use ui::session_state::{DesktopResumeSurface, DesktopSessionLayout, WindowPlacement};

// ------------------------=
// FUNC: layout
// DESC: Creates one observable non-default desktop layout for an identity round trip.
// ------------------=
fn layout() -> DesktopSessionLayout {
    DesktopSessionLayout {
        home: WindowPlacement::new(91, 117, 714, 566, false, true),
        settings: WindowPlacement::new(135, 188, 690, 604, true, true),
        editor: WindowPlacement::new(227, 163, 548, 612, false, true),
        command: WindowPlacement::new(281, 219, 501, 477, false, false),
        task_manager: WindowPlacement::new(176, 139, 744, 633, false, true),
        desktop_item_positions: [
            [81, 141],
            [164, 177],
            [249, 213],
            [336, 251],
            [425, 289],
            [516, 329],
            [609, 371],
        ],
        focused_surface: DesktopResumeSurface::Settings,
        settings_section: 4,
        settings_expanded_row: Some(5),
        settings_scroll_offset: 73,
        input_preferences: [0xa1, 8, 2, 6, 3, 0, 0, 0],
    }
}

// ------------------------=
// FUNC: checksum
// DESC: Produces the versioned identity checksum used to exercise legacy-state migration.
// ------------------=
fn checksum(bytes: &[u8]) -> u32 {
    let mut value = 0x811c9dc5u32;
    for byte in bytes {
        value ^= *byte as u32;
        value = value.wrapping_mul(0x01000193);
    }
    value
}

// ------------------------=
// FUNC: main
// DESC: Verifies per-user desktop layouts survive identity-state reconstruction without crossing users.
// ------------------=
fn main() {
    let mut identities = IdentitySystem::new();
    identities.begin_onboarding().unwrap();
    identities
        .create_machine(b"LayoutNode", 0x8664, 1, 1)
        .unwrap();
    let owner = identities.create_user(b"owner", b"Owner", 2).unwrap();
    let guest = identities.create_user(b"guest", b"Guest", 3).unwrap();
    let expected = layout();

    assert_eq!(
        identities.update_user_desktop_layout(guest.id, owner.id, expected),
        Err(IdentityError::AccessDenied)
    );
    identities
        .update_user_desktop_layout(owner.id, owner.id, expected)
        .unwrap();
    assert_eq!(identities.user_desktop_layout(guest.id), None);

    let encoded = identities.encode();
    let restored = IdentitySystem::decode(&encoded).unwrap();
    assert_eq!(restored.user_desktop_layout(owner.id), Some(expected));
    assert_eq!(restored.user_desktop_layout(guest.id), None);
    assert!(restored.session_nth(0).is_none());

    let mut version_two = [0u8; runtime::identity::V2_IDENTITY_STATE_BYTES];
    let version_two_end = version_two.len();
    version_two[..version_two_end - 4].copy_from_slice(&encoded[..version_two_end - 4]);
    version_two[8..10].copy_from_slice(&2u16.to_le_bytes());
    version_two[10..12].copy_from_slice(&(version_two_end as u16).to_le_bytes());
    let version_two_checksum = checksum(&version_two[..version_two_end - 4]);
    version_two[version_two_end - 4..].copy_from_slice(&version_two_checksum.to_le_bytes());
    let migrated_v2 = IdentitySystem::decode(&version_two).unwrap();
    assert_eq!(migrated_v2.user_desktop_layout(owner.id), Some(expected));
    assert_eq!(
        migrated_v2
            .read_ai_memory(owner.id, owner.id)
            .unwrap()
            .name(),
        b"Infinity"
    );

    let mut legacy = [0u8; runtime::identity::LEGACY_IDENTITY_STATE_BYTES];
    let legacy_end = legacy.len();
    legacy[..legacy_end - 4].copy_from_slice(&encoded[..legacy_end - 4]);
    legacy[8..10].copy_from_slice(&1u16.to_le_bytes());
    legacy[10..12].copy_from_slice(&(legacy_end as u16).to_le_bytes());
    let legacy_checksum = checksum(&legacy[..legacy_end - 4]);
    legacy[legacy_end - 4..].copy_from_slice(&legacy_checksum.to_le_bytes());
    let migrated = IdentitySystem::decode(&legacy).unwrap();
    assert!(migrated.user_profile(owner.id).is_some());
    assert_eq!(migrated.user_desktop_layout(owner.id), None);
    println!(
        "PASS desktop layout identity: exact per-user geometry survives session reconstruction"
    );
}
