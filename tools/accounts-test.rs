#![allow(dead_code)]

#[path = "../kernel/runtime/mod.rs"]
mod runtime;
#[path = "storage.rs"]
mod storage;
#[path = "../kernel/ui/mod.rs"]
mod ui;

mod console {
    #[derive(Clone, Copy)]
    pub enum ConsoleKey { Character(u8), Backspace, Delete, Left, Right, Home, End, Enter }
}

use runtime::identity::*;
use ui::session_state::{DesktopResumeSurface, DesktopSessionLayout, WindowPlacement};

// ------------------------=
// FUNC: output_text
// DESC: Provides the host runtime diagnostic sink without making output a test oracle.
// ------------------=
fn output_text(_: &[u8]) {}

// ------------------------=
// FUNC: fixture
// DESC: Creates an onboarded administrator with a real password-authenticated session.
// ------------------=
fn fixture() -> (IdentitySystem, UserIdentity, Session) {
    let mut identity = IdentitySystem::new();
    identity.begin_onboarding().unwrap();
    identity.create_machine(b"Accounts", 0xaa64, 1, 1).unwrap();
    let owner = identity.create_user(b"owner", b"Owner", 2).unwrap();
    identity.create_password(owner.id, b"owner password", 3).unwrap();
    identity.complete_onboarding().unwrap();
    let session = identity.create_session(owner.id, b"owner password", 4).unwrap();
    (identity, owner, session)
}

// ------------------------=
// FUNC: create_user
// DESC: Creates a password-backed user through the production authenticated management API.
// ------------------=
fn create_user(identity: &mut IdentitySystem, session: StableId, handle: &[u8]) -> UserIdentity {
    match identity.manage_accounts(session, AccountRequest::CreateUser {
        handle, display_name: b"Account User", password: b"member password", now: 5,
    }).unwrap() {
        AccountResult::User(user) => user,
        _ => panic!("unexpected typed account result"),
    }
}

// ------------------------=
// FUNC: create_group
// DESC: Creates a custom group through the production authenticated management API.
// ------------------=
fn create_group(identity: &mut IdentitySystem, session: StableId, name: &[u8]) -> GroupIdentity {
    match identity.manage_accounts(session, AccountRequest::CreateGroup {
        name, display_name: b"Project Team", now: 6,
    }).unwrap() {
        AccountResult::Group(group) => group,
        _ => panic!("unexpected typed account result"),
    }
}

// ------------------------=
// FUNC: layout
// DESC: Creates an observable persisted desktop profile for deletion and slot-reuse verification.
// ------------------=
fn layout() -> DesktopSessionLayout {
    DesktopSessionLayout {
        home: WindowPlacement::new(91, 117, 714, 566, false, true),
        settings: WindowPlacement::new(135, 188, 690, 604, true, true),
        editor: WindowPlacement::new(227, 163, 548, 612, false, true),
        command: WindowPlacement::new(281, 219, 501, 477, false, false),
        task_manager: WindowPlacement::new(176, 139, 744, 633, false, true),
        desktop_item_positions: [[81, 141]; 7],
        focused_surface: DesktopResumeSurface::Settings,
        settings_section: 2, settings_expanded_row: None, settings_scroll_offset: 73,
        input_preferences: [0; 8], app_drawer_left: true,
        app_drawer_floating: [301, 201], widgets: 0x8000_0100_0123_4567,
    }
}

// ------------------------=
// FUNC: authorization_and_crud
// DESC: Exercises authenticated directory reads, user/group CRUD, memberships and lifecycle cleanup.
// ------------------=
fn authorization_and_crud() {
    let (mut identity, owner, session) = fixture();
    let builtin = identity.group_by_name(b"ADMINISTRATORS").unwrap();
    assert_eq!(builtin.id, ADMINISTRATORS_GROUP_ID);
    assert!(identity.is_administrator(owner.id));
    assert_eq!(identity.accounts(owner.id).err(), Some(IdentityError::AccessDenied));
    assert_eq!(identity.accounts(StableId::zero()).err(), Some(IdentityError::AccessDenied));
    assert_eq!(identity.manage_accounts(owner.id, AccountRequest::DeleteUser { user: owner.id }), Err(IdentityError::AccessDenied));
    let member = create_user(&mut identity, session.id, b"member");
    let member_session = identity.create_session(member.id, b"member password", 7).unwrap();
    let directory = identity.accounts(member_session.id).unwrap();
    assert_eq!(directory.user_count(), 2);
    assert_eq!(directory.user_by_handle(b"MEMBER"), Some(member));
    assert_eq!(directory.group_count(), 1);
    assert_eq!(directory.group_member_nth(builtin.id, 0), Some(owner));
    assert_eq!(directory.user_group_nth(owner.id, 0), Some(builtin));
    assert_eq!(identity.manage_accounts(member_session.id, AccountRequest::CreateGroup {
        name: b"forbidden", display_name: b"Forbidden", now: 8,
    }), Err(IdentityError::AccessDenied));
    assert_eq!(identity.update_user_name(member.id, owner.id, b"Forged", true), Err(IdentityError::AccessDenied));
    identity.lock_session(member_session.id, member.id).unwrap();
    assert_eq!(identity.accounts(member_session.id).err(), Some(IdentityError::AccessDenied));
    assert_eq!(identity.manage_accounts(member_session.id, AccountRequest::RenameUser {
        user: member.id, handle: b"locked", display_name: b"Locked",
    }), Err(IdentityError::AccessDenied));
    identity.unlock_session(member_session.id, b"member password", 9).unwrap();
    let team = create_group(&mut identity, session.id, b"team");
    for _ in 0..2 {
        assert_eq!(identity.manage_accounts(session.id, AccountRequest::AddMember { group: team.id, user: member.id }),
            Ok(AccountResult::Membership { group: team.id, user: member.id, present: true }));
    }
    assert_eq!(identity.group_member_count(team.id), 1);
    assert_eq!(identity.user_group_count(member.id), 1);
    let renamed = identity.manage_accounts(member_session.id, AccountRequest::RenameUser {
        user: member.id, handle: b"renamed", display_name: b"Renamed Member",
    }).unwrap();
    assert!(matches!(renamed, AccountResult::User(user) if user.id == member.id && user.personal_space_ref == member.personal_space_ref));
    assert_eq!(identity.user_by_handle(b"member"), None);
    let before = identity.encode();
    assert_eq!(identity.manage_accounts(session.id, AccountRequest::RenameUser {
        user: member.id, handle: b"OWNER", display_name: b"Collision",
    }), Err(IdentityError::Conflict));
    assert_eq!(identity.encode(), before);
    assert_eq!(identity.manage_accounts(session.id, AccountRequest::CreateGroup {
        name: b"TEAM", display_name: b"Duplicate", now: 10,
    }), Err(IdentityError::Conflict));
    identity.manage_accounts(session.id, AccountRequest::RenameGroup {
        group: team.id, name: b"project", display_name: b"Project Members",
    }).unwrap();
    assert_eq!(identity.group_by_name(b"PROJECT").unwrap().id, team.id);
    assert_eq!(identity.group_by_name(b"team"), None);
    assert!(identity.is_group_member(team.id, member.id));
    identity.update_user_desktop_layout(member.id, member.id, layout()).unwrap();
    identity.manage_accounts(session.id, AccountRequest::SetUserEnabled { user: member.id, enabled: false }).unwrap();
    assert_eq!(identity.user(member.id).unwrap().state, UserState::Deactivated);
    assert!(identity.is_group_member(team.id, member.id));
    assert_eq!(identity.accounts(member_session.id).err(), Some(IdentityError::AccessDenied));
    assert_eq!(identity.authenticate(member.id, b"member password", 11), Err(IdentityError::InvalidCredential));
    identity.manage_accounts(session.id, AccountRequest::SetUserEnabled { user: member.id, enabled: true }).unwrap();
    let member_session = identity.create_session(member.id, b"member password", 12).unwrap();
    identity.manage_accounts(session.id, AccountRequest::RemoveMember { group: team.id, user: member.id }).unwrap();
    assert_eq!(identity.group_member_count(team.id), 0);
    identity.manage_accounts(session.id, AccountRequest::AddMember { group: team.id, user: member.id }).unwrap();
    assert_eq!(identity.manage_accounts(session.id, AccountRequest::DeleteUser { user: member.id }),
        Ok(AccountResult::DeletedUser { user: member.id, preserved_personal_space: member.personal_space_ref }));
    assert_eq!(identity.user(member.id), None);
    assert_eq!(identity.user_profile(member.id), None);
    assert_eq!(identity.ai_profile(member.id), None);
    assert_eq!(identity.voice_profile(member.id), None);
    assert_eq!(identity.personal_space(member.id), None);
    assert_eq!(identity.user_desktop_layout(member.id), None);
    assert_eq!(identity.group_member_count(team.id), 0);
    assert!(!identity.has_active_credential(member.id));
    assert!(identity.session_by_short(member_session.id.short()).is_none());
    let replacement = create_user(&mut identity, session.id, b"renamed");
    assert_ne!(replacement.id, member.id);
    assert_ne!(replacement.personal_space_ref, member.personal_space_ref);
    assert!(!identity.is_group_member(team.id, replacement.id));
    assert_eq!(identity.user_desktop_layout(replacement.id), None);
    identity.update_user_desktop_layout(replacement.id, replacement.id, layout()).unwrap();
    assert_eq!(identity.manage_accounts(session.id, AccountRequest::DeleteGroup { group: team.id }),
        Ok(AccountResult::DeletedGroup { group: team.id }));
    assert_eq!(identity.group(team.id), None);
    assert_eq!(identity.user_group_count(replacement.id), 0);
    let restored = IdentitySystem::decode(&identity.encode()).unwrap();
    assert_eq!(restored.user(member.id), None);
    assert_eq!(restored.user_profile(member.id), None);
    assert_eq!(restored.user_desktop_layout(member.id), None);
    assert_eq!(restored.user_desktop_layout(replacement.id), Some(layout()));
}

// ------------------------=
// FUNC: administrator_safety
// DESC: Checks built-in immutability, last-admin protection and immediate session authority updates.
// ------------------=
fn administrator_safety() {
    let (mut identity, owner, session) = fixture();
    let admin = ADMINISTRATORS_GROUP_ID;
    assert_eq!(identity.manage_accounts(session.id, AccountRequest::DeleteUser { user: owner.id }), Err(IdentityError::AccessDenied));
    assert_eq!(identity.manage_accounts(session.id, AccountRequest::SetUserEnabled { user: owner.id, enabled: false }), Err(IdentityError::AccessDenied));
    assert_eq!(identity.manage_accounts(session.id, AccountRequest::RemoveMember { group: admin, user: owner.id }), Err(IdentityError::InvalidState));
    assert_eq!(identity.manage_accounts(session.id, AccountRequest::DeleteGroup { group: admin }), Err(IdentityError::AccessDenied));
    assert_eq!(identity.manage_accounts(session.id, AccountRequest::RenameGroup {
        group: admin, name: b"owners", display_name: b"Owners",
    }), Err(IdentityError::AccessDenied));
    let second = create_user(&mut identity, session.id, b"second");
    let second_session = identity.create_session(second.id, b"member password", 10).unwrap();
    identity.manage_accounts(session.id, AccountRequest::AddMember { group: admin, user: second.id }).unwrap();
    assert_ne!(identity.session_by_short(second_session.id.short()).unwrap().capabilities & SESSION_IDENTITY_MANAGE, 0);
    identity.manage_accounts(session.id, AccountRequest::RemoveMember { group: admin, user: owner.id }).unwrap();
    assert!(!identity.is_administrator(owner.id));
    assert_eq!(identity.session_by_short(session.id.short()).unwrap().capabilities & SESSION_IDENTITY_MANAGE, 0);
    assert_eq!(identity.manage_accounts(session.id, AccountRequest::DeleteUser { user: second.id }), Err(IdentityError::AccessDenied));
    assert_eq!(identity.manage_accounts(second_session.id, AccountRequest::RemoveMember { group: admin, user: second.id }), Err(IdentityError::InvalidState));
    let mut restored = IdentitySystem::decode(&identity.encode()).unwrap();
    assert!(!restored.is_administrator(owner.id));
    assert!(restored.is_administrator(second.id));
    assert_eq!(restored.create_session(owner.id, b"owner password", 11).unwrap().capabilities & SESSION_IDENTITY_MANAGE, 0);
    identity.manage_accounts(second_session.id, AccountRequest::DeleteUser { user: owner.id }).unwrap();
    assert!(identity.is_administrator(second.id));
    assert!(identity.session_by_short(session.id.short()).is_none());
}

// ------------------------=
// FUNC: password_reclaim_and_rollback
// DESC: Verifies repeated reset reclamation, secret replacement, session retention and full failed-write rollback.
// ------------------=
fn password_reclaim_and_rollback() {
    let (mut identity, owner, session) = fixture();
    let member = create_user(&mut identity, session.id, b"member");
    let member_session = identity.create_session(member.id, b"member password", 6).unwrap();
    let member_other = identity.create_session(member.id, b"member password", 7).unwrap();
    identity.manage_accounts(member_session.id, AccountRequest::ResetPassword {
        user: member.id, password: b"own new password", now: 8,
    }).unwrap();
    assert!(identity.accounts(member_session.id).is_ok());
    assert_eq!(identity.accounts(member_other.id).err(), Some(IdentityError::AccessDenied));
    assert_eq!(identity.authenticate(member.id, b"member password", 9), Err(IdentityError::InvalidCredential));
    identity.authenticate(member.id, b"own new password", 10).unwrap();
    for round in 0..(MAX_CREDENTIALS * 2) {
        identity.manage_accounts(session.id, AccountRequest::ResetPassword {
            user: member.id, password: b"reset password", now: 20 + round as u64,
        }).unwrap();
    }
    assert_eq!(identity.credential_nth(2), None);
    assert_eq!(identity.accounts(member_session.id).err(), Some(IdentityError::AccessDenied));
    identity.authenticate(member.id, b"reset password", 100).unwrap();
    let before = identity.encode();
    assert_eq!(identity.manage_accounts(session.id, AccountRequest::ResetPassword {
        user: member.id, password: b"short", now: 101,
    }), Err(IdentityError::InvalidInput));
    assert_eq!(identity.encode(), before);
    for now in 102..105 {
        assert_eq!(identity.authenticate(member.id, b"wrong", now), Err(IdentityError::InvalidCredential));
    }
    let before = identity.encode();
    let session_before = identity.session_by_short(session.id.short());
    let result = identity.manage_accounts_durable(session.id, AccountRequest::ResetPassword {
        user: member.id, password: b"uncommitted password", now: 105,
    }, |bytes, result| {
        assert_eq!(result, AccountResult::PasswordReset { user: member.id });
        let mut persisted = IdentitySystem::decode(bytes).unwrap();
        persisted.authenticate(member.id, b"uncommitted password", 106).unwrap();
        Err(IdentityError::PersonalSpaceUnavailable)
    });
    assert_eq!(result, Err(IdentityError::PersonalSpaceUnavailable));
    assert_eq!(identity.encode(), before);
    assert_eq!(identity.session_by_short(session.id.short()), session_before);
    assert_eq!(identity.authenticate(member.id, b"reset password", 107), Err(IdentityError::RateLimited));
    identity.authenticate(member.id, b"reset password", 10_000).unwrap();
    let member_session = identity.create_session(member.id, b"reset password", 10_001).unwrap();
    let before = identity.encode();
    let failed_delete = identity.manage_accounts_durable(session.id, AccountRequest::DeleteUser { user: member.id }, |_, _| {
        Err(IdentityError::PersonalSpaceUnavailable)
    });
    assert_eq!(failed_delete, Err(IdentityError::PersonalSpaceUnavailable));
    assert_eq!(identity.encode(), before);
    assert!(identity.accounts(member_session.id).is_ok());
    let other_owner_session = identity.create_session(owner.id, b"owner password", 10_002).unwrap();
    identity.manage_accounts(session.id, AccountRequest::ResetPassword {
        user: owner.id, password: b"owner changed password", now: 10_003,
    }).unwrap();
    assert!(identity.accounts(session.id).is_ok());
    assert_eq!(identity.accounts(other_owner_session.id).err(), Some(IdentityError::AccessDenied));
    let mut callback_called = false;
    assert_eq!(identity.manage_accounts_durable(member_session.id, AccountRequest::DeleteUser { user: owner.id }, |_, _| {
        callback_called = true;
        Ok(())
    }), Err(IdentityError::AccessDenied));
    assert!(!callback_called);
}

// ------------------------=
// FUNC: checksum
// DESC: Computes the binary identity checksum for historical-version and structural-corruption fixtures.
// ------------------=
fn checksum(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c9dc5u32, |sum, byte| (sum ^ *byte as u32).wrapping_mul(0x01000193))
}

// ------------------------=
// FUNC: repair_checksum
// DESC: Repairs only the artifact checksum so malformed structured account records reach decoder validation.
// ------------------=
fn repair_checksum(bytes: &mut [u8]) {
    let end = bytes.len() - 4;
    let value = checksum(&bytes[..end]);
    bytes[end..].copy_from_slice(&value.to_le_bytes());
}

// ------------------------=
// FUNC: migration_and_corruption
// DESC: Checks all historical versions migrate the former active owner and v4 preserves explicit authority.
// ------------------=
fn migration_and_corruption() {
    let (mut identity, owner, session) = fixture();
    let member = create_user(&mut identity, session.id, b"member");
    let group = create_group(&mut identity, session.id, b"team");
    identity.manage_accounts(session.id, AccountRequest::AddMember { group: group.id, user: member.id }).unwrap();
    identity.update_user_desktop_layout(member.id, member.id, layout()).unwrap();
    let encoded = identity.encode();
    assert_eq!(encoded.len(), 12_960);
    let mut version_four = encoded;
    version_four[8..10].copy_from_slice(&4u16.to_le_bytes());
    repair_checksum(&mut version_four);
    let migrated = IdentitySystem::decode(&version_four).unwrap();
    assert_eq!(migrated.group(group.id), Some(group));
    assert!(migrated.is_group_member(group.id, member.id));
    assert!(migrated.is_administrator(owner.id));
    assert!(!migrated.is_administrator(member.id));
    for (version, length) in [(1u16, LEGACY_IDENTITY_STATE_BYTES), (2, V2_IDENTITY_STATE_BYTES), (3, V3_IDENTITY_STATE_BYTES)] {
        let mut bytes = encoded[..length].to_vec();
        bytes[8..10].copy_from_slice(&version.to_le_bytes());
        bytes[10..12].copy_from_slice(&(length as u16).to_le_bytes());
        repair_checksum(&mut bytes);
        let mut restored = IdentitySystem::decode(&bytes).unwrap();
        assert!(restored.is_administrator(owner.id));
        assert!(!restored.is_administrator(member.id));
        assert_eq!(restored.group_count(), 1);
        assert_eq!(restored.user(member.id), Some(member));
        assert_eq!(restored.user_desktop_layout(member.id), if version >= 2 { Some(layout()) } else { None });
        restored.authenticate(owner.id, b"owner password", 20).unwrap();
        restored.authenticate(member.id, b"member password", 20).unwrap();
        // A disabled first slot historically handed authority to the first active user.
        bytes[160 + 1] = UserState::Deactivated as u8;
        repair_checksum(&mut bytes);
        let restored = IdentitySystem::decode(&bytes).unwrap();
        assert!(!restored.is_administrator(owner.id));
        assert!(restored.is_administrator(member.id));
    }
    let restored = IdentitySystem::decode(&encoded).unwrap();
    assert_eq!(restored.group(group.id), Some(group));
    assert!(restored.is_group_member(group.id, member.id));
    assert!(restored.session_nth(0).is_none());
    let mut invalid = encoded;
    invalid[ACCOUNTS_STATE_OFFSET + 16 + 2] = 0x80;
    repair_checksum(&mut invalid);
    assert!(matches!(IdentitySystem::decode(&invalid), Err(IdentityError::CorruptState)));
    let mut invalid = encoded;
    invalid[ACCOUNTS_STATE_OFFSET + 16 + 2] = 0;
    repair_checksum(&mut invalid);
    assert!(matches!(IdentitySystem::decode(&invalid), Err(IdentityError::CorruptState)));
    let mut invalid = encoded;
    invalid[ACCOUNTS_STATE_OFFSET + 16 + 136 + 1] = 1;
    repair_checksum(&mut invalid);
    assert!(matches!(IdentitySystem::decode(&invalid), Err(IdentityError::CorruptState)));
}

// ------------------------=
// FUNC: bounded_capacity
// DESC: Verifies bounded group/user capacity, failed-input atomicity and reclaimed user/group/credential slots.
// ------------------=
fn bounded_capacity() {
    let (mut identity, owner, session) = fixture();
    let before = identity.encode();
    assert_eq!(identity.manage_accounts(session.id, AccountRequest::CreateUser {
        handle: b"valid", display_name: b"", password: b"valid password", now: 10,
    }), Err(IdentityError::InvalidInput));
    assert_eq!(identity.encode(), before);
    assert_eq!(identity.manage_accounts(session.id, AccountRequest::AddMember {
        group: ADMINISTRATORS_GROUP_ID, user: StableId::zero(),
    }), Err(IdentityError::NotFound));
    for index in 1..MAX_USERS {
        let handle = [b'u', b'0' + index as u8];
        create_user(&mut identity, session.id, &handle);
    }
    let before = identity.encode();
    assert_eq!(identity.manage_accounts(session.id, AccountRequest::CreateUser {
        handle: b"overflow", display_name: b"Overflow", password: b"valid password", now: 11,
    }), Err(IdentityError::Full));
    assert_eq!(identity.encode(), before);
    for index in 1..MAX_GROUPS {
        let name = [b'g', b'a' + index as u8];
        create_group(&mut identity, session.id, &name);
    }
    let before = identity.encode();
    assert_eq!(identity.manage_accounts(session.id, AccountRequest::CreateGroup {
        name: b"overflow", display_name: b"Overflow", now: 12,
    }), Err(IdentityError::Full));
    assert_eq!(identity.encode(), before);
    let reclaimed_group = identity.group_nth(1).unwrap();
    identity.manage_accounts(session.id, AccountRequest::DeleteGroup { group: reclaimed_group.id }).unwrap();
    let replacement = create_group(&mut identity, session.id, reclaimed_group.name.as_bytes());
    assert_ne!(replacement.id, reclaimed_group.id);
    let deleted = identity.user_nth(1).unwrap();
    identity.manage_accounts(session.id, AccountRequest::DeleteUser { user: deleted.id }).unwrap();
    create_user(&mut identity, session.id, deleted.handle.as_bytes());
    while identity.credential_nth(MAX_CREDENTIALS - 1).is_none() {
        identity.create_password(owner.id, b"additional password", 13).unwrap();
    }
    identity.manage_accounts(session.id, AccountRequest::ResetPassword {
        user: owner.id, password: b"reclaimed password", now: 14,
    }).unwrap();
    assert!(identity.accounts(session.id).is_ok());
    assert_eq!(identity.credential_nth(MAX_USERS), None);
    for now in 20..40 {
        let transient = identity.create_session(owner.id, b"reclaimed password", now).unwrap();
        identity.end_session(transient.id, owner.id).unwrap();
    }
    assert!(IdentitySystem::decode(&identity.encode()).is_ok());
}

// ------------------------=
// FUNC: uncertain_commit_latch
// DESC: Checks the shared legacy/durable failure latch revokes sessions and survives stale checkpoint rollback.
// ------------------=
fn uncertain_commit_latch() {
    let (mut identity, owner, session) = fixture();
    let locked = identity.create_session(owner.id, b"owner password", 10).unwrap();
    identity.lock_session(locked.id, owner.id).unwrap();
    let before = identity.encode();
    let legacy_rollback = identity.account_checkpoint();
    identity.latch_account_storage_uncertain();
    identity.latch_account_storage_uncertain();
    assert!(identity.account_storage_uncertain());
    assert!(identity.session_nth(0).is_none());
    assert_eq!(identity.encode(), before);
    identity.restore_account_checkpoint(legacy_rollback);
    assert!(identity.account_storage_uncertain());
    assert!(identity.session_nth(0).is_none());
    assert_eq!(identity.accounts(session.id).err(), Some(IdentityError::CommitUncertain));
    assert_eq!(identity.authenticate(owner.id, b"owner password", 11), Err(IdentityError::CommitUncertain));
    assert_eq!(identity.create_session(owner.id, b"owner password", 12), Err(IdentityError::CommitUncertain));
    let mut callback_called = false;
    assert_eq!(identity.manage_accounts_durable(session.id, AccountRequest::RenameUser {
        user: owner.id, handle: b"blocked", display_name: b"Blocked",
    }, |_, _| {
        callback_called = true;
        Ok(())
    }), Err(IdentityError::CommitUncertain));
    assert!(!callback_called);
    assert!(identity.account_storage_uncertain());
    assert_eq!(identity.encode(), before);

    let (mut durable, owner, session) = fixture();
    let before = durable.encode();
    let older_checkpoint = durable.account_checkpoint();
    assert_eq!(durable.manage_accounts_durable(session.id, AccountRequest::RenameUser {
        user: owner.id, handle: b"changed", display_name: b"Changed",
    }, |_, _| Err(IdentityError::CommitUncertain)), Err(IdentityError::CommitUncertain));
    assert!(durable.account_storage_uncertain());
    assert!(durable.session_nth(0).is_none());
    assert_eq!(durable.encode(), before);
    durable.restore_account_checkpoint(older_checkpoint);
    assert!(durable.account_storage_uncertain());
    assert!(durable.session_nth(0).is_none());
    assert_eq!(durable.accounts(session.id).err(), Some(IdentityError::CommitUncertain));
    // Restoring a checkpoint that already carries the latch must also fail closed.
    let mut fresh = IdentitySystem::new();
    fresh.restore_account_checkpoint(durable.account_checkpoint());
    assert!(fresh.account_storage_uncertain());
    assert_eq!(fresh.authenticate(owner.id, b"owner password", 13), Err(IdentityError::CommitUncertain));
}

// ------------------------=
// FUNC: main
// DESC: Executes focused behavioral account-management tests against the production native identity model.
// ------------------=
fn main() {
    authorization_and_crud();
    administrator_safety();
    password_reclaim_and_rollback();
    migration_and_corruption();
    bounded_capacity();
    uncertain_commit_latch();
    println!("PASS account management behavioral suite");
}
