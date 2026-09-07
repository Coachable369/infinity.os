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

use runtime::console_language::{parse, ParseOutcome};
use runtime::identity::*;
use runtime::iop::OperationId;

// ------------------------=
// FUNC: contains
// DESC: Finds an exact byte sequence without interpreting native state as text.
// ------------------=
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

// ------------------------=
// FUNC: identity_lifecycle
// DESC: Verifies first boot, authentication, isolation, capabilities, sessions, settings, and recovery.
// ------------------=
fn identity_lifecycle() {
    let mut identities = IdentitySystem::new();
    assert_eq!(identities.onboarding_state(), OnboardingState::Required);
    identities.begin_onboarding().unwrap();
    let machine = identities
        .create_machine(b"InfinityNode", 0xaa64, 7, 1)
        .unwrap();
    let renamed = identities.update_machine_name(b"HomeMesh").unwrap();
    assert_eq!(renamed.id, machine.id);

    let owner = identities
        .create_user(b"aurelius", b"Aurelius Prime", 2)
        .unwrap();
    let owner_space = identities.personal_space(owner.id).unwrap();
    assert_eq!(owner_space.owner, owner.id);
    identities
        .create_password(owner.id, b"correct horse battery", 3)
        .unwrap();
    identities.complete_onboarding().unwrap();
    let owner_session = identities
        .create_session(owner.id, b"correct horse battery", 4)
        .unwrap();
    assert_ne!(owner_session.capabilities & SESSION_IDENTITY_MANAGE, 0);
    assert_eq!(owner_session.personal_space, owner_space.space);

    let second = identities
        .create_user(b"second", b"Second User", 5)
        .unwrap();
    let second_space = identities.personal_space(second.id).unwrap();
    assert_ne!(owner_space.space, second_space.space);
    let second_credential = identities
        .create_password(second.id, b"different secret", 6)
        .unwrap();
    let second_session = identities
        .create_session(second.id, b"different secret", 7)
        .unwrap();
    assert_eq!(second_session.capabilities & SESSION_IDENTITY_MANAGE, 0);
    assert_eq!(
        identities.update_user_name(second.id, owner.id, b"Nope", false),
        Err(IdentityError::AccessDenied)
    );

    identities
        .lock_session(second_session.id, second.id)
        .unwrap();
    assert_eq!(
        identities
            .session_by_short(second_session.id.short())
            .unwrap()
            .state,
        SessionState::Locked
    );
    assert_eq!(
        identities.unlock_session(second_session.id, b"wrong secret", 8),
        Err(IdentityError::InvalidCredential)
    );
    identities
        .unlock_session(second_session.id, b"different secret", 9)
        .unwrap();
    identities
        .end_session(second_session.id, second.id)
        .unwrap();
    let closed = identities
        .session_by_short(second_session.id.short())
        .unwrap();
    assert_eq!(closed.state, SessionState::Closed);
    assert_eq!(closed.capabilities, 0);
    assert_eq!(
        identities.authenticate(second.id, b"wrong", 20),
        Err(IdentityError::InvalidCredential)
    );
    assert_eq!(
        identities.authenticate(second.id, b"wrong", 21),
        Err(IdentityError::InvalidCredential)
    );
    assert_eq!(
        identities.authenticate(second.id, b"wrong", 22),
        Err(IdentityError::InvalidCredential)
    );
    assert_eq!(
        identities.authenticate(second.id, b"different secret", 23),
        Err(IdentityError::RateLimited)
    );
    identities
        .revoke_credential(owner.id, second_credential.id, true)
        .unwrap();
    assert_eq!(
        identities.authenticate(second.id, b"different secret", 20_000),
        Err(IdentityError::InvalidCredential)
    );

    identities
        .update_user_theme(owner.id, owner.id, b"nebula-high-contrast")
        .unwrap();
    identities
        .update_user_icon_theme(owner.id, owner.id, 2)
        .unwrap();
    identities
        .update_user_accent(owner.id, owner.id, 0xd45cff)
        .unwrap();
    identities.update_primary(owner.id, 0x251f42).unwrap();
    identities
        .update_background_effects(owner.id, 64, 7)
        .unwrap();
    let primary_round_trip = IdentitySystem::decode(&identities.encode()).unwrap();
    assert_eq!(primary_round_trip.primary_rgb(), 0x251f42);
    assert_eq!(primary_round_trip.background_effects(), (64, 7));
    assert_eq!(
        identities.update_primary(StableId::zero(), 0x142f36),
        Err(IdentityError::AccessDenied)
    );
    assert_eq!(
        identities.update_background_effects(owner.id, 39, 4),
        Err(IdentityError::InvalidInput)
    );
    assert_eq!(
        identities.update_user_accent(second.id, owner.id, 0x33d69f),
        Err(IdentityError::AccessDenied)
    );
    identities
        .update_user_icon_theme(owner.id, owner.id, 3)
        .unwrap();
    assert_eq!(
        identities.update_user_icon_theme(owner.id, owner.id, 4),
        Err(IdentityError::InvalidInput)
    );
    identities
        .update_ai_profile(owner.id, owner.id, AiProviderPolicy::AskBeforeRemote)
        .unwrap();
    identities
        .update_voice_profile(owner.id, owner.id, true, VoiceActivation::PushToTalk)
        .unwrap();
    let encoded = identities.encode();
    assert!(!contains(&encoded, b"correct horse battery"));
    assert!(!contains(&encoded, b"different secret"));
    let restored = IdentitySystem::decode(&encoded).unwrap();
    assert_eq!(restored.onboarding_state(), OnboardingState::Complete);
    assert_eq!(restored.machine().unwrap().id, machine.id);
    assert_eq!(
        restored.user_profile(owner.id).unwrap().theme.as_bytes(),
        b"nebula-high-contrast"
    );
    assert_eq!(restored.user_profile(owner.id).unwrap().icon_theme, 3);
    assert_eq!(restored.background_effects(), (64, 7));
    assert_eq!(
        restored.user_profile(owner.id).unwrap().accent_rgb,
        0xd45cff
    );
    assert_eq!(
        restored.ai_profile(owner.id).unwrap().provider_policy,
        AiProviderPolicy::AskBeforeRemote
    );
    assert!(restored.voice_profile(owner.id).unwrap().enabled);
    assert!(
        restored.session_nth(0).is_none(),
        "sessions must not survive reboot"
    );
    identities
        .update_user_accent(owner.id, owner.id, 0x20bfff)
        .unwrap();
    let migrated = IdentitySystem::decode(&identities.encode()).unwrap();
    assert_eq!(
        migrated.user_profile(owner.id).unwrap().accent_rgb,
        DEFAULT_ACCENT_RGB
    );
    let mut corrupt = encoded;
    corrupt[240] ^= 0x5a;
    assert!(matches!(
        IdentitySystem::decode(&corrupt),
        Err(IdentityError::CorruptState)
    ));
    println!("PASS identity lifecycle: stable IDs, PBKDF verifier, private spaces, scoped capabilities, lock/unlock/logout, persistent profiles, and corruption recovery");
}

// ------------------------=
// FUNC: interrupted_onboarding
// DESC: Verifies first-boot progress is durable and safely resumes without partial completion.
// ------------------=
fn interrupted_onboarding() {
    let mut state = IdentitySystem::new();
    state.begin_onboarding().unwrap();
    state
        .create_machine(b"InterruptedNode", 0x8664, 1, 1)
        .unwrap();
    let persisted = state.encode();
    let resumed = IdentitySystem::decode(&persisted).unwrap();
    assert_eq!(resumed.onboarding_state(), OnboardingState::InProgress);
    assert!(resumed.machine().is_some());
    assert_eq!(resumed.user_count(), 0);

    let mut late = IdentitySystem::new();
    late.begin_onboarding().unwrap();
    late.create_machine(b"LateInterruptedNode", 0x8664, 1, 1)
        .unwrap();
    let user = late.create_user(b"recovery", b"Recovery User", 2).unwrap();
    late.create_password(user.id, b"recovery secret", 3)
        .unwrap();
    let mut resumed_late = IdentitySystem::decode(&late.encode()).unwrap();
    assert!(resumed_late.has_active_credential(user.id));
    resumed_late
        .authenticate(user.id, b"recovery secret", 4)
        .unwrap();
    resumed_late.complete_onboarding().unwrap();
    assert_eq!(resumed_late.onboarding_state(), OnboardingState::Complete);
    println!("PASS onboarding recovery: early and post-credential interruptions resume without inventing authority");
}

// ------------------------=
// FUNC: console_contract
// DESC: Verifies every Milestone 7 human command resolves to a typed operation schema.
// ------------------=
fn console_contract() {
    for command in [
        b"user create handle=test display-name=Test".as_slice(),
        b"user list",
        b"user read user:1",
        b"user update user:1 display-name=Prime",
        b"user delete user:2",
        b"identity read user:1",
        b"machine read",
        b"machine update name=InfinityNode",
        b"credential create user=user:1 type=password",
        b"credential list",
        b"session list",
        b"session read session:1",
        b"session lock",
        b"session delete session:1",
        b"personal-space read user:1",
        b"ai-profile read user:1",
        b"ai-profile update user:1 provider-policy=local-only",
        b"voice-profile read user:1",
        b"voice-profile update user:1 enabled=true activation=push-to-talk",
        b"settings read",
        b"settings update name=appearance.theme value=cosmic-dark",
        b"skin list",
        b"skin inspect name=infinity.default.dark",
        b"appearance read",
        b"appearance set-skin name=skin value=infinity.default.dark",
        b"ui tree",
        b"ui focus",
        b"window list",
        b"clipboard read",
        b"clipboard write name=text value=hello",
    ] {
        assert!(
            matches!(parse(command), Ok(ParseOutcome::Graph(_))),
            "unregistered command: {}",
            String::from_utf8_lossy(command)
        );
    }
    println!("PASS GUI/CLI contract: identity, machine, credential, session, Personal Space, AI, voice, and settings use typed operations");
}

// ------------------------=
// FUNC: service_registry
// DESC: Verifies dependency-ordered identity services publish the expected IOP providers.
// ------------------=
fn service_registry() {
    let mut system = runtime::InfinityRuntime::new(false);
    system.define_bootstrap().unwrap();
    system.start_all(0);
    for operation in [
        OperationId::IdentityRead,
        OperationId::AuthenticationVerify,
        OperationId::SessionCreate,
        OperationId::SettingsUpdate,
        OperationId::OnboardingAdvance,
        OperationId::ShellOpen,
        OperationId::FontList,
        OperationId::FontOpen,
        OperationId::SkinList,
        OperationId::AppearanceSetSkin,
        OperationId::UiInspectTree,
        OperationId::WindowList,
        OperationId::ClipboardRead,
        OperationId::ClipboardWrite,
    ] {
        assert!(system.services.provider(operation as u32).is_some());
    }
    println!("PASS service registry: Identity, Authentication, Session, Settings, Onboarding, and Shell operations are discoverable");
}

// ------------------------=
// FUNC: default_font_catalog
// DESC: Verifies the complete default type library uses stable IDs and installed System-space resources.
// ------------------=
fn default_font_catalog() {
    let system = runtime::InfinityRuntime::new(false);
    assert_eq!(system.fonts.count(), 46);
    for index in 0..46 {
        let font = system.fonts.nth(index).expect("missing default font");
        assert_eq!(font.id as usize, index + 1);
        assert!(font.resource.starts_with(b"System/Fonts/"));
        assert!(font.resource.ends_with(b".ttf"));
        assert_eq!(system.fonts.by_id(font.id), Some(font));
    }
    println!(
        "PASS font catalog: 46 typed default families resolve to installed System-space resources"
    );
}

// ------------------------=
// FUNC: main
// DESC: Runs the complete Milestone 7 host acceptance suite.
// ------------------=
fn main() {
    identity_lifecycle();
    interrupted_onboarding();
    console_contract();
    service_registry();
    default_font_catalog();
    println!("PASS Milestone 7 native identity and first-boot foundation");
}
