#![allow(dead_code)]

#[path = "../kernel/runtime/mod.rs"] mod runtime;
#[path = "storage.rs"] mod storage;
#[path = "../kernel/ui/mod.rs"] mod ui;
#[path = "../kernel/core/console_accounts_password.rs"] mod account_password;
#[path = "../kernel/core/console_accounts_command.rs"] mod account_command;
#[path = "../kernel/drivers/input/desktop_shortcuts.rs"] mod desktop_shortcuts;

use account_command::{prepare, AccountCommand};
use account_password::{PasswordAction, PasswordAuthority, PasswordEntry, PasswordProgress, PasswordShortcut, PASSWORD_CAPACITY};
use runtime::console_language::{parse, ConsoleLanguageError, OperationGraph, ParseOutcome, ReferenceKind, ValueType};
use runtime::identity::*;
use runtime::iop::OperationId;

// ------------------------=
// FUNC: output_text
// DESC: Supplies the host harness diagnostic sink without using log output as an oracle.
// ------------------=
fn output_text(_: &[u8]) {}

// ------------------------=
// FUNC: graph
// DESC: Extracts a typed executable graph from the production parser.
// ------------------=
fn graph(command: &[u8]) -> OperationGraph<'_> {
    match parse(command).unwrap() { ParseOutcome::Graph(graph) => graph, _ => panic!("expected operation graph") }
}

// ------------------------=
// FUNC: prepared
// DESC: Exercises the production resolver against an authenticated account read lease.
// ------------------=
fn prepared(identity: &IdentitySystem, session: StableId, input: &[u8]) -> Result<AccountCommand, IdentityError> {
    let graph = graph(input);
    assert!(graph.executable_account_operation());
    prepare(&identity.accounts(session)?, &graph.nodes[0].unwrap())
}

// ------------------------=
// FUNC: mutate
// DESC: Dispatches a parsed CLI mutation through the exact shared backend callback used by production.
// ------------------=
fn mutate(identity: &mut IdentitySystem, session: StableId, input: &[u8]) -> Result<AccountResult, IdentityError> {
    match prepared(identity, session, input)? {
        AccountCommand::Mutation(mutation) => mutation.execute(20, |request| identity.manage_accounts(session, request)),
        _ => panic!("expected account mutation"),
    }
}

// ------------------------=
// FUNC: enter_twice
// DESC: Enters matching private password fields using the production state machine.
// ------------------=
fn enter_twice(entry: &mut PasswordEntry, password: &[u8]) {
    for byte in password { assert!(entry.push(*byte)); }
    assert_eq!(entry.submit(), PasswordProgress::Confirming);
    assert_eq!(entry.masked_length(), 0);
    for byte in password { assert!(entry.push(*byte)); }
    assert_eq!(entry.submit(), PasswordProgress::Ready);
}

// ------------------------=
// FUNC: secret_command
// DESC: Resolves a password command, confirms it privately, and submits the typed request without a plaintext command argument.
// ------------------=
fn secret_command(identity: &mut IdentitySystem, session: StableId, input: &[u8], password: &[u8]) -> Result<AccountResult, IdentityError> {
    let AccountCommand::Password(action) = prepared(identity, session, input)? else { panic!("expected private password entry"); };
    let mut entry = PasswordEntry::new();
    entry.begin(session, action);
    enter_twice(&mut entry, password);
    let result = entry.finish(session, 25, |request| identity.manage_accounts(session, request));
    assert!(!entry.active());
    assert_eq!(entry.masked_length(), 0);
    result
}

// ------------------------=
// FUNC: parser_contract
// DESC: Verifies operation IDs, typed targets/results, discoverability, plan isolation, and rejected secret arguments.
// ------------------=
fn parser_contract() {
    let cases: &[(&[u8], OperationId, ValueType)] = &[
        (b"user create handle=avery display-name=\"Avery Chen\"", OperationId::IdentityCreate, ValueType::User),
        (b"user list", OperationId::IdentityList, ValueType::UserSet),
        (b"user read user:avery", OperationId::IdentityRead, ValueType::User),
        (b"user update user:avery handle=ava", OperationId::IdentityUpdate, ValueType::User),
        (b"user delete user:2", OperationId::IdentityDelete, ValueType::User),
        (b"user enable user:2", OperationId::IdentitySetEnabled, ValueType::User),
        (b"user disable user:2", OperationId::IdentitySetEnabled, ValueType::User),
        (b"user password user:2", OperationId::IdentityResetPassword, ValueType::User),
        (b"user groups user:2", OperationId::IdentityRead, ValueType::MembershipSet),
        (b"group create name=design", OperationId::GroupCreate, ValueType::Group),
        (b"group list", OperationId::GroupList, ValueType::GroupSet),
        (b"group read group:0", OperationId::GroupRead, ValueType::Group),
        (b"group update group:design name=product", OperationId::GroupUpdate, ValueType::Group),
        (b"group delete group:design", OperationId::GroupDelete, ValueType::Unit),
        (b"group members group:design", OperationId::GroupRead, ValueType::MembershipSet),
        (b"group add group:design user=user:avery", OperationId::GroupAddMember, ValueType::MembershipSet),
        (b"group remove group:design user=user:2", OperationId::GroupRemoveMember, ValueType::MembershipSet),
    ];
    for (input, id, value_type) in cases {
        let parsed = graph(input);
        assert_eq!(parsed.nodes[0].unwrap().schema.operation, *id);
        assert_eq!(parsed.result_type, *value_type);
        assert!(parsed.executable_account_operation());
    }
    assert_eq!(graph(b"group read group:0").nodes[0].unwrap().target.unwrap().kind, ReferenceKind::Group);
    assert!(matches!(parse(b"help group add"), Ok(ParseOutcome::Help(_, Some(schema))) if schema.operation == OperationId::GroupAddMember));
    assert!(matches!(parse(b"group"), Ok(ParseOutcome::DomainDiscovery(_))));
    assert!(matches!(parse(b"identity read"), Ok(ParseOutcome::OperationDiscovery(_))));
    let plan = graph(b"plan user disable user:2");
    assert!(plan.plan_only);
    assert!(!plan.executable_account_operation());
    assert_eq!(plan.result_type, ValueType::OperationPlan);
    for input in [b"user password user:2 password=secret".as_slice(), b"user create handle=ava display-name=Ava password=secret", b"group add group:design user=group:0", b"user read group:0", b"group read user:2"] {
        assert!(parse(input).is_err());
    }
    assert!(matches!(parse(b"group add group:design user=user:2 user=user:3"), Err(ConsoleLanguageError::DuplicateArgument)));
    assert!(account_command::sensitive_command(b"user password user:2 password=not-stored"));
    assert!(account_command::sensitive_command(b"saved = plan user create handle=ava display-name=Ava"));
    assert!(!account_command::sensitive_command(b"system status"));
}

// ------------------------=
// FUNC: account_dispatch
// DESC: Exercises CLI user/group CRUD, membership, credentials, authority failures, and stale target rejection against native state.
// ------------------=
fn account_dispatch() {
    let mut identity = IdentitySystem::new();
    identity.begin_onboarding().unwrap();
    identity.create_machine(b"CLI Test", 0xaa64, 1, 1).unwrap();
    let owner = identity.create_user(b"owner", b"Owner", 2).unwrap();
    identity.create_password(owner.id, b"owner-password", 3).unwrap();
    identity.complete_onboarding().unwrap();
    let session = identity.create_session(owner.id, b"owner-password", 4).unwrap().id;
    let created = secret_command(&mut identity, session, b"user create handle=avery display-name=\"Avery Chen\"", b"new-password").unwrap();
    let AccountResult::User(user) = created else { panic!("expected created user"); };
    assert_eq!(identity.user_count(), 2);
    let read_numeric = format!("user read user:{}", user.id.short());
    for input in [b"user read user:avery".as_slice(), read_numeric.as_bytes()] {
        assert!(matches!(prepared(&identity, session, input).unwrap(), AccountCommand::User { user: selected, .. } if selected.id == user.id));
    }
    assert!(matches!(prepared(&identity, session, b"group read group:0").unwrap(), AccountCommand::Group { group, .. } if group.id == ADMINISTRATORS_GROUP_ID));
    assert!(matches!(prepared(&identity, session, b"user list").unwrap(), AccountCommand::Users { count: 2, .. }));
    mutate(&mut identity, session, b"user update user:avery handle=ava display-name=\"Ava Chen\"").unwrap();
    assert_eq!(identity.user_by_handle(b"ava").unwrap().id, user.id);
    assert!(matches!(prepared(&identity, session, b"user read user:avery"), Err(IdentityError::NotFound)));
    assert!(matches!(prepared(&identity, session, b"identity read user:999999"), Err(IdentityError::NotFound)));
    assert!(matches!(prepared(&identity, session, b"user read user:99999999999999999999999999999"), Err(IdentityError::InvalidInput)));
    assert!(matches!(prepared(&identity, session, b"user update user:ava"), Err(IdentityError::InvalidInput)));

    let result = mutate(&mut identity, session, b"group create name=design display-name=\"Design Team\"").unwrap();
    let AccountResult::Group(group) = result else { panic!("expected created group"); };
    mutate(&mut identity, session, b"group add group:design user=user:ava").unwrap();
    assert!(identity.is_group_member(group.id, user.id));
    assert!(matches!(prepared(&identity, session, b"user groups user:ava").unwrap(), AccountCommand::Groups { count: 1, .. }));
    assert!(matches!(prepared(&identity, session, b"group members group:design").unwrap(), AccountCommand::Group { count: 1, .. }));
    let add_admin = format!("group add group:0 user=user:{}", user.id.short());
    mutate(&mut identity, session, add_admin.as_bytes()).unwrap();
    assert!(identity.is_administrator(user.id));
    mutate(&mut identity, session, b"group remove group:administrators user=user:ava").unwrap();
    assert!(!identity.is_administrator(user.id));
    mutate(&mut identity, session, b"group update group:design name=product").unwrap();
    assert_eq!(identity.group_by_name(b"product").unwrap().id, group.id);
    assert!(matches!(prepared(&identity, session, b"group read group:design"), Err(IdentityError::NotFound)));
    mutate(&mut identity, session, b"group remove group:product user=user:ava").unwrap();
    assert!(!identity.is_group_member(group.id, user.id));

    let ordinary = identity.create_session(user.id, b"new-password", 30).unwrap().id;
    assert_eq!(mutate(&mut identity, ordinary, b"group create name=forbidden"), Err(IdentityError::AccessDenied));
    let mut pending_password = PasswordEntry::new();
    pending_password.begin(ordinary, PasswordAction::Reset { user: user.id });
    for byte in b"pending-secret" { assert!(pending_password.push(*byte)); }
    assert_eq!(pending_password.revalidate(ordinary, identity.accounts(ordinary).map(|_| ())), PasswordAuthority::Valid);
    mutate(&mut identity, session, b"user disable user:ava").unwrap();
    assert_eq!(identity.user(user.id).unwrap().state, UserState::Deactivated);
    assert!(identity.accounts(ordinary).is_err());
    assert_eq!(pending_password.revalidate(ordinary, identity.accounts(ordinary).map(|_| ())), PasswordAuthority::SignIn);
    assert!(!pending_password.active());
    assert_eq!(pending_password.masked_length(), 0);
    let mut dispatched = false;
    assert!(pending_password.finish(ordinary, 31, |_| { dispatched = true; Ok(()) }).is_err());
    assert!(!dispatched);
    mutate(&mut identity, session, b"user enable user:ava").unwrap();
    assert_eq!(identity.user(user.id).unwrap().state, UserState::Active);
    secret_command(&mut identity, session, b"user password user:ava", b"changed-password").unwrap();
    assert!(identity.create_session(user.id, b"changed-password", 40).is_ok());
    assert!(identity.create_session(user.id, b"new-password", 41).is_err());
    mutate(&mut identity, session, b"group delete group:product").unwrap();
    assert!(identity.group(group.id).is_none());
    mutate(&mut identity, session, b"user delete user:ava").unwrap();
    assert!(identity.user(user.id).is_none());
    assert!(matches!(prepared(&identity, session, read_numeric.as_bytes()), Err(IdentityError::NotFound)));
    assert_eq!(identity.user(owner.id).unwrap().handle.as_bytes(), b"owner");
    identity.lock_session(session, owner.id).unwrap();
    assert!(prepared(&identity, session, b"user list").is_err());
}

// ------------------------=
// FUNC: password_transitions
// DESC: Verifies confirmation, mismatch recovery, bounds, cancellation, one-shot dispatch, and session binding without reading secret buffers.
// ------------------=
fn password_transitions() {
    let session = StableId([1; 16]);
    let user = StableId([2; 16]);
    let mut entry = PasswordEntry::new();
    entry.begin(session, PasswordAction::Reset { user });
    assert_eq!(entry.submit(), PasswordProgress::TooShort);
    assert!(!entry.push(b'\n'));
    for byte in b"password1" { assert!(entry.push(*byte)); }
    entry.backspace();
    assert_eq!(entry.masked_length(), 8);
    assert_eq!(entry.submit(), PasswordProgress::Confirming);
    for byte in b"different" { assert!(entry.push(*byte)); }
    assert_eq!(entry.submit(), PasswordProgress::Mismatch);
    assert_eq!(entry.masked_length(), 0);
    enter_twice(&mut entry, b"replacement");
    let mut calls = 0;
    let result = entry.finish(session, 99, |request| {
        calls += 1;
        match request { AccountRequest::ResetPassword { user: target, password, now } => { assert_eq!(target, user); assert_eq!(password, b"replacement"); assert_eq!(now, 99); }, _ => panic!("wrong request") }
        Err::<(), _>(IdentityError::AccessDenied)
    });
    assert_eq!(result, Err(IdentityError::AccessDenied));
    assert_eq!(calls, 1);
    assert!(!entry.active());
    assert_eq!(entry.masked_length(), 0);
    assert!(entry.finish(session, 100, |_| { calls += 1; Ok(()) }).is_err());
    assert_eq!(calls, 1);

    entry.begin(session, PasswordAction::Reset { user });
    for _ in 0..PASSWORD_CAPACITY { assert!(entry.push(b'x')); }
    assert!(!entry.push(b'y'));
    assert_eq!(entry.masked_length(), PASSWORD_CAPACITY);
    entry.cancel();
    assert!(!entry.active());
    assert_eq!(entry.masked_length(), 0);
    assert_eq!(entry.session(), StableId::zero());
    entry.begin(session, PasswordAction::Reset { user });
    enter_twice(&mut entry, b"new-password");
    assert!(entry.finish(StableId([3; 16]), 100, |_| { calls += 1; Ok(()) }).is_err());
    assert_eq!(calls, 1);
    assert!(!entry.active());
}

// ------------------------=
// FUNC: password_shortcut_transitions
// DESC: Exercises actual native switcher codes, private clipboard/history isolation, and one-shot secret erasure before lifecycle dispatch.
// ------------------=
fn password_shortcut_transitions() {
    let session = StableId([1; 16]);
    let user = StableId([2; 16]);
    for code in [desktop_shortcuts::SPATIAL_NEXT, desktop_shortcuts::SPATIAL_PREVIOUS, b'j', b'i', b'K', b'N', b'L'] {
        let mut entry = PasswordEntry::new();
        entry.begin(session, PasswordAction::Reset { user });
        for byte in b"first-secret" { assert!(entry.push(*byte)); }
        assert_eq!(entry.submit(), PasswordProgress::Confirming);
        for byte in b"second-secret" { assert!(entry.push(*byte)); }
        assert_eq!(entry.shortcut(code, true), if code == b'L' { PasswordShortcut::Lock } else { PasswordShortcut::Dispatch });
        assert!(!entry.active());
        assert_eq!(entry.session(), StableId::zero());
        assert_eq!(entry.masked_length(), 0);
        let mut dispatched = false;
        assert!(entry.finish(session, 1, |_| { dispatched = true; Ok(()) }).is_err());
        assert!(!dispatched);
    }
    let mut entry = PasswordEntry::new();
    entry.begin(session, PasswordAction::Reset { user });
    assert!(entry.push(b'x'));
    for code in [b'c', b'x', b'v', b'C', b'V', b'r', b'a'] {
        assert_eq!(entry.shortcut(code, true), PasswordShortcut::Consume);
        assert!(entry.active());
        assert_eq!(entry.session(), session);
        assert_eq!(entry.masked_length(), 1);
    }
    assert_eq!(entry.shortcut(desktop_shortcuts::SPATIAL_NEXT, false), PasswordShortcut::Consume);
    assert!(entry.active());
    assert_eq!(entry.shortcut(b'L', false), PasswordShortcut::Lock);
    assert!(!entry.active());
}

// ------------------------=
// FUNC: password_authority_transitions
// DESC: Verifies the production guard's routing outcomes and secret erasure on session replacement or uncertain storage.
// ------------------=
fn password_authority_transitions() {
    let session = StableId([1; 16]);
    let user = StableId([2; 16]);
    for (current, authority, expected) in [
        (StableId([3; 16]), Ok(()), PasswordAuthority::SignIn),
        (session, Err(IdentityError::AccessDenied), PasswordAuthority::SignIn),
        (session, Err(IdentityError::CommitUncertain), PasswordAuthority::RestartRequired),
    ] {
        let mut entry = PasswordEntry::new();
        entry.begin(session, PasswordAction::Reset { user });
        enter_twice(&mut entry, b"confirmed-secret");
        assert_eq!(entry.revalidate(current, authority), expected);
        assert!(!entry.active());
        assert_eq!(entry.masked_length(), 0);
        assert_eq!(entry.session(), StableId::zero());
        let mut dispatched = false;
        assert!(entry.finish(session, 1, |_| { dispatched = true; Ok(()) }).is_err());
        assert!(!dispatched);
        assert_eq!(entry.revalidate(current, authority), PasswordAuthority::Idle);
    }
}

// ------------------------=
// FUNC: main
// DESC: Runs focused behavioral parser, dispatch, and secret-state acceptance through the repository harness.
// ------------------=
fn main() { parser_contract(); account_dispatch(); password_transitions(); password_shortcut_transitions(); password_authority_transitions(); }
