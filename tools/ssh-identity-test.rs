#![allow(dead_code)]
#[path = "../kernel/runtime/mod.rs"] mod runtime;
#[path = "storage.rs"] mod storage;
#[path = "../kernel/ui/mod.rs"] mod ui;
mod console {
    #[derive(Clone, Copy)]
    pub enum ConsoleKey { Character(u8), Backspace, Delete, Left, Right, Home, End, Enter }
}
use runtime::identity::*;

// ------------------------=
// FUNC: output_text
// DESC: Keeps host diagnostics separate from typed acceptance assertions.
// ------------------=
fn output_text(_: &[u8]) {}
// ------------------------=
// FUNC: main
// DESC: Verifies durable native SSH credentials, authorization, rollback, revocation and zero local-session creation.
// ------------------=
fn main() {
    let mut identity = IdentitySystem::new();
    identity.begin_onboarding().unwrap();
    identity.create_machine(b"SSH acceptance", 0xaa64, 1, 1).unwrap();
    let owner = identity.create_user(b"owner", b"Owner", 2).unwrap();
    identity.create_password(owner.id, b"owner password", 3).unwrap();
    identity.complete_onboarding().unwrap();
    let session = identity.create_session(owner.id, b"owner password", 4).unwrap();
    let key = ed25519_dalek::SigningKey::from_bytes(&[9; 32]).verifying_key().to_bytes();
    assert_eq!(identity.manage_ssh_key_durable(StableId::zero(), owner.id, key, false, 5, |_| Ok(())), Err(IdentityError::AccessDenied));
    assert_eq!(identity.manage_ssh_key_durable(session.id, owner.id, key, false, 5, |_| Err(IdentityError::InvalidState)), Err(IdentityError::InvalidState));
    assert_eq!(identity.ssh_key_owner(b"owner", &key), None);
    let credential = identity.manage_ssh_key_durable(session.id, owner.id, key, false, 6, |_| Ok(())).unwrap();
    assert_eq!(credential.kind, CredentialType::SshEd25519);
    let encoded = identity.encode();
    let mut restored = IdentitySystem::decode(&encoded).unwrap();
    for _ in 0..100 {
        assert_eq!(restored.ssh_key_owner(b"owner", &key), Some(owner.id));
        assert!(restored.ssh_key_active(owner.id, &key));
    }
    assert!((0..MAX_SESSIONS).all(|index| restored.session_nth(index).is_none()));
    assert_eq!(restored.ssh_key_owner(b"unknown", &key), None);
    assert_eq!(restored.ssh_key_owner(b"owner", &[8; 32]), None);
    assert_eq!(restored.authenticate(owner.id, &key, 10), Err(IdentityError::InvalidCredential));
    let fresh = restored.create_session(owner.id, b"owner password", 11).unwrap();
    assert_eq!(restored.manage_ssh_key_durable(fresh.id, owner.id, key, true, 12, |_| Err(IdentityError::InvalidState)), Err(IdentityError::InvalidState));
    assert!(restored.ssh_key_active(owner.id, &key));
    restored.manage_ssh_key_durable(fresh.id, owner.id, key, true, 13, |_| Ok(())).unwrap();
    assert!(!restored.ssh_key_active(owner.id, &key));
    assert!(!IdentitySystem::decode(&restored.encode()).unwrap().ssh_key_active(owner.id, &key));
    let member = match restored.manage_accounts(fresh.id, AccountRequest::CreateUser {
        handle: b"member", display_name: b"Member", password: b"member password", now: 14,
    }).unwrap() { AccountResult::User(user) => user, _ => panic!() };
    let member_session = restored.create_session(member.id, b"member password", 15).unwrap();
    assert_eq!(restored.manage_ssh_key_durable(member_session.id, owner.id, key, false, 16, |_| Ok(())), Err(IdentityError::AccessDenied));
    restored.manage_ssh_key_durable(member_session.id, member.id, key, false, 17, |_| Ok(())).unwrap();
    assert!(restored.ssh_key_active(member.id, &key));
    restored.manage_accounts(fresh.id, AccountRequest::SetUserEnabled { user: member.id, enabled: false }).unwrap();
    assert!(!restored.ssh_key_active(member.id, &key));
    restored.manage_accounts(fresh.id, AccountRequest::SetUserEnabled { user: member.id, enabled: true }).unwrap();
    assert!(restored.ssh_key_active(member.id, &key));
    restored.manage_accounts(fresh.id, AccountRequest::DeleteUser { user: member.id }).unwrap();
    assert!(!restored.ssh_key_active(member.id, &key));
    assert_eq!(restored.manage_ssh_key_durable(fresh.id, owner.id, key, false, 18, |_| Err(IdentityError::CommitUncertain)), Err(IdentityError::CommitUncertain));
    assert!(restored.account_storage_uncertain());
    assert_eq!(restored.ssh_key_owner(b"owner", &key), None);
}
