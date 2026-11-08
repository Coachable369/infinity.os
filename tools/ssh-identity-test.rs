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
    generated_account_capacity();
    password_authentication();
    remote_authority_parity();
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

// ------------------------=
// FUNC: generated_account_capacity
// DESC: Keeps all eight account slots usable with password plus generated SSH credentials, including cold decoding and rollback.
// ------------------=
fn generated_account_capacity() {
    let mut identity = IdentitySystem::new();
    identity.begin_onboarding().unwrap();
    identity.create_machine(b"Capacity", 0xaa64, 1, 1).unwrap();
    let key = ed25519_dalek::SigningKey::from_bytes(&[51; 32]).verifying_key().to_bytes();
    assert_eq!(identity.create_onboarding_user_durable(b"owner", b"Owner", 2, key, |_, _| Err(IdentityError::InvalidState)), Err(IdentityError::InvalidState));
    assert_eq!(identity.user_count(), 0);
    let owner = identity.create_onboarding_user_durable(b"owner", b"Owner", 2, key, |_, _| Ok(())).unwrap();
    assert!(!identity.has_password_credential(owner.id));
    identity.create_password(owner.id, b"owner password", 3).unwrap();
    identity.complete_onboarding().unwrap();
    let session = identity.create_session(owner.id, b"owner password", 4).unwrap();
    let mut old = identity.encode()[..V5_IDENTITY_STATE_BYTES].to_vec();
    old[8..10].copy_from_slice(&5u16.to_le_bytes());
    old[10..12].copy_from_slice(&(V5_IDENTITY_STATE_BYTES as u16).to_le_bytes());
    let end = old.len() - 4;
    let mut checksum = 0x811c9dc5u32;
    for byte in &old[..end] { checksum = (checksum ^ *byte as u32).wrapping_mul(0x01000193); }
    old[end..].copy_from_slice(&checksum.to_le_bytes());
    let restored = IdentitySystem::decode(&old).unwrap();
    assert_eq!(restored.ssh_key_owner(b"owner", &key), Some(owner.id));
    for index in 1..MAX_USERS {
        let name = [b'u', b'0' + index as u8];
        let key = ed25519_dalek::SigningKey::from_bytes(&[51 + index as u8; 32]).verifying_key().to_bytes();
        let AccountResult::User(user) = identity.manage_accounts_enrolled_durable(session.id,
            AccountRequest::CreateUser { handle: &name, display_name: &name, password: b"member password", now: 5 },
            Some(key), |_, _| Ok(())).unwrap() else { panic!() };
        let mut restored = IdentitySystem::decode(&identity.encode()).unwrap();
        assert_eq!(restored.ssh_key_owner(&name, &key), Some(user.id));
        restored.authenticate(user.id, b"member password", 6).unwrap();
    }
    assert_eq!(identity.user_count(), MAX_USERS);
    assert_eq!((0..MAX_CREDENTIALS).filter_map(|i| identity.credential_nth(i)).count(), MAX_USERS * 2);
}

// ------------------------=
// FUNC: password_authentication
// DESC: Verifies account-password throttling, independent remote permission parity, persistence and revocation.
// ------------------=
fn password_authentication() {
    let mut identity = IdentitySystem::new();
    identity.begin_onboarding().unwrap();
    identity.create_machine(b"Password SSH", 0xaa64, 1, 1).unwrap();
    let user = identity.create_user(b"Jonathan", b"Jonathan", 2).unwrap();
    let credential = identity.create_password(user.id, b"native password", 3).unwrap();
    identity.complete_onboarding().unwrap();
    let mut identity = IdentitySystem::decode(&identity.encode()).unwrap();
    assert!(identity.authenticate_ssh_password(b"missing", b"native password", 10).is_err());
    for now in 10..13 {
        assert_eq!(identity.authenticate_ssh_password(b"Jonathan", b"incorrect", now), Err(IdentityError::InvalidCredential));
    }
    assert_eq!(identity.authenticate_ssh_password(b"Jonathan", b"native password", 13), Err(IdentityError::RateLimited));
    let local=identity.create_session(user.id, b"native password", 10).unwrap();
    let proof = identity.authenticate_ssh_password(b"Jonathan", b"native password", 2012).unwrap();
    assert_eq!(proof, (user.id, credential.id));
    let remote = identity.create_ssh_password_session(proof.0, proof.1, 3).unwrap();
    assert_eq!(remote.origin, SessionOrigin::Ssh);
    assert_eq!(remote.capabilities,local.capabilities);
    assert_eq!(remote.remote_key, None);
    assert_eq!(remote.remote_password, Some(credential.id));
    assert!(identity.console_session_authorized(remote.id, user.id));
    assert!(identity.accounts(remote.id).is_ok());
    identity.manage_accounts(local.id, AccountRequest::ResetPassword { user: user.id, password: b"changed password", now: 2014 }).unwrap();
    assert!(!identity.ssh_password_active(user.id, credential.id));
    assert!(!identity.console_session_authorized(remote.id, user.id));
    assert!(identity.authenticate_ssh_password(b"Jonathan", b"native password", 2015).is_err());
    assert!(identity.authenticate_ssh_password(b"Jonathan", b"changed password", 2016).is_ok());
    assert!(identity.accounts(remote.id).is_err());
}

// ------------------------=
// FUNC: remote_authority_parity
// DESC: Exercises administrator and member SSH rights, independent scopes, live role changes, local-session isolation and credential revocation.
// ------------------=
fn remote_authority_parity(){
    let mut identity=IdentitySystem::new();identity.begin_onboarding().unwrap();
    identity.create_machine(b"Remote parity",0xaa64,1,1).unwrap();
    let owner=identity.create_user(b"admin",b"Admin",2).unwrap();identity.create_password(owner.id,b"admin password",3).unwrap();identity.complete_onboarding().unwrap();
    let local=identity.create_session(owner.id,b"admin password",4).unwrap();
    let key=ed25519_dalek::SigningKey::from_bytes(&[31;32]).verifying_key().to_bytes();
    identity.manage_ssh_key_durable(local.id,owner.id,key,false,5,|_|Ok(())).unwrap();
    let admin=identity.create_ssh_session(owner.id,key,6).unwrap();
    let AccountResult::User(member)=identity.manage_accounts(admin.id,AccountRequest::CreateUser{handle:b"member",display_name:b"Member",password:b"member password",now:7}).unwrap()else{panic!()};
    let member_local=identity.create_session(member.id,b"member password",8).unwrap();
    let member_key=ed25519_dalek::SigningKey::from_bytes(&[32;32]).verifying_key().to_bytes();
    identity.manage_ssh_key_durable(member_local.id,member.id,member_key,false,9,|_|Ok(())).unwrap();
    let remote=identity.create_ssh_session(member.id,member_key,10).unwrap();
    let proof=identity.authenticate_ssh_password(b"member",b"member password",11).unwrap();
    let password=identity.create_ssh_password_session(proof.0,proof.1,12).unwrap();
    assert_eq!(remote.origin,SessionOrigin::Ssh);assert_eq!(remote.capabilities,member_local.capabilities);
    assert_eq!(remote.personal_space,member.personal_space_ref);assert_ne!(remote.personal_space,admin.personal_space);
    assert_ne!(remote.ai_context,member_local.ai_context);assert_ne!(remote.ai_context,password.ai_context);
    assert_ne!(admin.capabilities&SESSION_IDENTITY_MANAGE,0);assert_eq!(remote.capabilities&SESSION_IDENTITY_MANAGE,0);
    assert!(identity.accounts(remote.id).is_ok());assert!(!identity.console_session_authorized(admin.id,member.id));
    assert_eq!(identity.manage_accounts(remote.id,AccountRequest::CreateGroup{name:b"denied",display_name:b"Denied",now:13}),Err(IdentityError::AccessDenied));
    identity.end_session(local.id,owner.id).unwrap();identity.lock_session(member_local.id,member.id).unwrap();
    assert!(identity.console_session_authorized(admin.id,owner.id));assert!(identity.console_session_authorized(remote.id,member.id));
    identity.manage_accounts(admin.id,AccountRequest::AddMember{group:ADMINISTRATORS_GROUP_ID,user:member.id}).unwrap();
    assert_ne!(identity.session_by_short(remote.id.short()).unwrap().capabilities&SESSION_IDENTITY_MANAGE,0);
    assert!(identity.manage_accounts(remote.id,AccountRequest::CreateGroup{name:b"allowed",display_name:b"Allowed",now:14}).is_ok());
    identity.manage_accounts(admin.id,AccountRequest::RemoveMember{group:ADMINISTRATORS_GROUP_ID,user:member.id}).unwrap();
    assert_eq!(identity.session_by_short(remote.id.short()).unwrap().capabilities&SESSION_IDENTITY_MANAGE,0);
    assert_eq!(identity.manage_accounts(remote.id,AccountRequest::CreateGroup{name:b"deniedagain",display_name:b"Denied again",now:15}),Err(IdentityError::AccessDenied));
    identity.manage_ssh_key_durable(admin.id,member.id,member_key,true,16,|_|Ok(())).unwrap();
    assert!(!identity.console_session_authorized(remote.id,member.id));assert!(identity.accounts(remote.id).is_err());
    assert!(identity.console_session_authorized(password.id,member.id));
    identity.manage_accounts(admin.id,AccountRequest::ResetPassword{user:member.id,password:b"changed password",now:17}).unwrap();
    assert!(!identity.console_session_authorized(password.id,member.id));assert!(identity.accounts(password.id).is_err());
    assert!(identity.console_session_authorized(admin.id,owner.id));
}
