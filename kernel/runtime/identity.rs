//! Native identity, authentication, session, preference, and first-boot state.
//! Persistent data is encoded explicitly; Rust layout is never a disk ABI.

use crate::runtime::ai::memory::{AiMemory, AI_MEMORY_SLOT_BYTES, AI_MEMORY_STATE_BYTES};
use crate::ui::session_state::{
    DesktopSessionLayout, PersistentDesktopLayoutStore, DESKTOP_LAYOUT_STATE_BYTES,
};

pub const MAX_USERS: usize = 8;
pub const MAX_CREDENTIALS: usize = 12;
pub const MAX_SESSIONS: usize = 8;
pub const LEGACY_IDENTITY_STATE_BYTES: usize = 4096;
pub const V2_IDENTITY_STATE_BYTES: usize = LEGACY_IDENTITY_STATE_BYTES + DESKTOP_LAYOUT_STATE_BYTES;
pub const USER_AI_MEMORY_OFFSET: usize = V2_IDENTITY_STATE_BYTES - 4;
pub const IDENTITY_STATE_BYTES: usize = USER_AI_MEMORY_OFFSET + AI_MEMORY_STATE_BYTES + 4;
pub const IDENTITY_FORMAT_VERSION: u16 = 3;
pub const PASSWORD_ITERATIONS: u32 = 4096;
pub const USER_ICON_THEME_OFFSET: usize = 4056;
pub const USER_ACCENT_OFFSET: usize = 4064;
pub const SYSTEM_PRIMARY_OFFSET: usize = 4088;
pub const SYSTEM_BACKGROUND_EFFECTS_OFFSET: usize = 4091;
pub const USER_DESKTOP_LAYOUTS_OFFSET: usize = 4092;
pub const DEFAULT_ACCENT_RGB: u32 = 0x4da3ff;
pub const DEFAULT_PRIMARY_RGB: u32 = 0x0d2238;
pub const DEFAULT_BACKGROUND_OPACITY: u8 = 88;
pub const DEFAULT_BACKGROUND_BLUR: u8 = 4;
pub const DEFAULT_NO_ACTIVITY_TIMEOUT_MINUTES: u8 = 5;
pub const MIN_NO_ACTIVITY_TIMEOUT_MINUTES: u8 = 1;
pub const MAX_NO_ACTIVITY_TIMEOUT_MINUTES: u8 = 120;
const LEGACY_DEFAULT_ACCENT_RGB: u32 = 0x20bfff;

pub const SESSION_PERSONAL_READ: u64 = 1 << 0;
pub const SESSION_PERSONAL_WRITE: u64 = 1 << 1;
pub const SESSION_PROFILE_READ_SELF: u64 = 1 << 2;
pub const SESSION_PROFILE_UPDATE_SELF: u64 = 1 << 3;
pub const SESSION_SETTINGS_READ: u64 = 1 << 4;
pub const SESSION_SETTINGS_UPDATE_ALLOWED: u64 = 1 << 5;
pub const SESSION_AI_USE: u64 = 1 << 6;
pub const SESSION_VOICE_USE: u64 = 1 << 7;
pub const SESSION_DISPLAY_INPUT: u64 = 1 << 8;
pub const SESSION_MANAGE_SELF: u64 = 1 << 9;
pub const SESSION_IDENTITY_MANAGE: u64 = 1 << 10;
pub const ORDINARY_SESSION_CAPABILITIES: u64 = SESSION_PERSONAL_READ
    | SESSION_PERSONAL_WRITE
    | SESSION_PROFILE_READ_SELF
    | SESSION_PROFILE_UPDATE_SELF
    | SESSION_SETTINGS_READ
    | SESSION_SETTINGS_UPDATE_ALLOWED
    | SESSION_AI_USE
    | SESSION_VOICE_USE
    | SESSION_DISPLAY_INPUT
    | SESSION_MANAGE_SELF;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StableId(pub [u8; 16]);

impl StableId {
    // ------------------------=
    // FUNC: zero
    // DESC: Returns the reserved empty stable identity.
    // ------------------=
    pub const fn zero() -> Self {
        Self([0; 16])
    }

    // ------------------------=
    // FUNC: is_zero
    // DESC: Reports whether this identity is the reserved empty value.
    // ------------------=
    pub fn is_zero(self) -> bool {
        self.0 == [0; 16]
    }

    // ------------------------=
    // FUNC: short
    // DESC: Returns the compact runtime projection while preserving the full stable identity internally.
    // ------------------=
    pub fn short(self) -> u64 {
        u64::from_le_bytes(self.0[..8].try_into().unwrap())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ShortText {
    bytes: [u8; 48],
    length: u8,
}

impl ShortText {
    // ------------------------=
    // FUNC: empty
    // DESC: Creates an empty bounded UTF-8-compatible text value.
    // ------------------=
    pub const fn empty() -> Self {
        Self {
            bytes: [0; 48],
            length: 0,
        }
    }

    // ------------------------=
    // FUNC: new
    // DESC: Validates and copies a bounded text value without allocation.
    // ------------------=
    pub fn new(value: &[u8]) -> Result<Self, IdentityError> {
        if value.is_empty() || value.len() > 48 || value.iter().any(|b| *b < 0x20) {
            return Err(IdentityError::InvalidInput);
        }
        let mut result = Self::empty();
        result.bytes[..value.len()].copy_from_slice(value);
        result.length = value.len() as u8;
        Ok(result)
    }

    // ------------------------=
    // FUNC: as_bytes
    // DESC: Returns the initialized bytes of this bounded text value.
    // ------------------=
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.length as usize]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UserState {
    Active = 1,
    Deactivated = 2,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CredentialType {
    Password = 1,
    Pin = 2,
    Hardware = 3,
    Biometric = 4,
    Passkey = 5,
    Recovery = 6,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CredentialState {
    Active = 1,
    Revoked = 2,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SessionState {
    Created = 1,
    Authenticating = 2,
    Active = 3,
    Locked = 4,
    Closing = 5,
    Closed = 6,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OnboardingState {
    Required = 1,
    InProgress = 2,
    Complete = 3,
    RecoveryRequired = 4,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AiProviderPolicy {
    LocalOnly = 1,
    PreferLocal = 2,
    AskBeforeRemote = 3,
    RemoteAllowed = 4,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VoiceActivation {
    Disabled = 1,
    PushToTalk = 2,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingScope {
    System = 1,
    Machine = 2,
    User = 3,
    Session = 4,
    Application = 5,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MachineIdentity {
    pub id: StableId,
    pub display_name: ShortText,
    pub architecture: u32,
    pub installation_id: StableId,
    pub system_generation: u64,
    pub created: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UserIdentity {
    pub id: StableId,
    pub display_name: ShortText,
    pub handle: ShortText,
    pub created: u64,
    pub state: UserState,
    pub profile_ref: StableId,
    pub personal_space_ref: StableId,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CredentialDescriptor {
    pub id: StableId,
    pub owner: StableId,
    pub kind: CredentialType,
    pub state: CredentialState,
    pub created: u64,
    pub last_used: u64,
}

#[derive(Clone, Copy)]
struct Credential {
    descriptor: CredentialDescriptor,
    salt: [u8; 16],
    verifier: [u8; 32],
    iterations: u32,
    failures: u8,
    retry_after: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UserProfile {
    pub user: StableId,
    pub language: ShortText,
    pub region: ShortText,
    pub theme: ShortText,
    pub icon_theme: u8,
    pub accent_rgb: u32,
    pub no_activity_timeout_minutes: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AiProfile {
    pub user: StableId,
    pub provider_policy: AiProviderPolicy,
    pub remote_processing: bool,
    pub chat_enabled: bool,
    pub chat_model_index: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VoiceProfile {
    pub user: StableId,
    pub enabled: bool,
    pub activation: VoiceActivation,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PersonalSpaceOwnership {
    pub owner: StableId,
    pub space: StableId,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Session {
    pub id: StableId,
    pub user: StableId,
    pub machine: StableId,
    pub created: u64,
    pub state: SessionState,
    pub capabilities: u64,
    pub personal_space: StableId,
    pub ai_context: StableId,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IdentityError {
    InvalidInput,
    Full,
    NotFound,
    Conflict,
    AccessDenied,
    InvalidCredential,
    RateLimited,
    InvalidState,
    CorruptState,
    PersonalSpaceUnavailable,
}

pub struct IdentitySystem {
    machine: Option<MachineIdentity>,
    users: [Option<UserIdentity>; MAX_USERS],
    credentials: [Option<Credential>; MAX_CREDENTIALS],
    profiles: [Option<UserProfile>; MAX_USERS],
    ai_profiles: [Option<AiProfile>; MAX_USERS],
    voice_profiles: [Option<VoiceProfile>; MAX_USERS],
    ownership: [Option<PersonalSpaceOwnership>; MAX_USERS],
    sessions: [Option<Session>; MAX_SESSIONS],
    onboarding: OnboardingState,
    next_id: u64,
    generation: u64,
    primary_rgb: u32,
    background_opacity: u8,
    background_blur: u8,
    desktop_layouts: PersistentDesktopLayoutStore,
    ai_memories: [AiMemory; MAX_USERS],
}

impl IdentitySystem {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty machine whose durable onboarding state is Required.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            machine: None,
            users: [None; MAX_USERS],
            credentials: [None; MAX_CREDENTIALS],
            profiles: [None; MAX_USERS],
            ai_profiles: [None; MAX_USERS],
            voice_profiles: [None; MAX_USERS],
            ownership: [None; MAX_USERS],
            sessions: [None; MAX_SESSIONS],
            onboarding: OnboardingState::Required,
            next_id: 1,
            generation: 0,
            primary_rgb: DEFAULT_PRIMARY_RGB,
            background_opacity: DEFAULT_BACKGROUND_OPACITY,
            background_blur: DEFAULT_BACKGROUND_BLUR,
            desktop_layouts: PersistentDesktopLayoutStore::new(),
            ai_memories: [AiMemory::new(); MAX_USERS],
        }
    }

    // ------------------------=
    // FUNC: onboarding_state
    // DESC: Returns the authoritative first-boot lifecycle state.
    // ------------------=
    pub fn onboarding_state(&self) -> OnboardingState {
        self.onboarding
    }

    // ------------------------=
    // FUNC: begin_onboarding
    // DESC: Marks first-boot setup in progress so interruption is recoverable.
    // ------------------=
    pub fn begin_onboarding(&mut self) -> Result<(), IdentityError> {
        if !matches!(
            self.onboarding,
            OnboardingState::Required
                | OnboardingState::RecoveryRequired
                | OnboardingState::InProgress
        ) {
            return Err(IdentityError::InvalidState);
        }
        self.onboarding = OnboardingState::InProgress;
        self.commit();
        Ok(())
    }

    // ------------------------=
    // FUNC: create_machine
    // DESC: Establishes stable machine identity independently from its editable display name.
    // ------------------=
    pub fn create_machine(
        &mut self,
        name: &[u8],
        architecture: u32,
        generation: u64,
        now: u64,
    ) -> Result<MachineIdentity, IdentityError> {
        if self.machine.is_some() {
            return Err(IdentityError::Conflict);
        }
        let machine = MachineIdentity {
            id: self.allocate_id(0x4d),
            display_name: ShortText::new(name)?,
            architecture,
            installation_id: self.allocate_id(0x49),
            system_generation: generation,
            created: now,
        };
        self.machine = Some(machine);
        self.commit();
        Ok(machine)
    }

    // ------------------------=
    // FUNC: machine
    // DESC: Reads the stable machine identity and mutable presentation metadata.
    // ------------------=
    pub fn machine(&self) -> Option<MachineIdentity> {
        self.machine
    }

    // ------------------------=
    // FUNC: update_machine_name
    // DESC: Renames the machine without changing its stable identity.
    // ------------------=
    pub fn update_machine_name(&mut self, name: &[u8]) -> Result<MachineIdentity, IdentityError> {
        let text = ShortText::new(name)?;
        let machine = self.machine.as_mut().ok_or(IdentityError::NotFound)?;
        machine.display_name = text;
        let result = *machine;
        self.commit();
        Ok(result)
    }

    // ------------------------=
    // FUNC: primary_rgb
    // DESC: Returns the durable machine-wide primary theme color.
    // ------------------=
    pub const fn primary_rgb(&self) -> u32 {
        self.primary_rgb
    }

    // ------------------------=
    // FUNC: update_primary
    // DESC: Persists a validated primary theme color for OS-wide glass surfaces.
    // ------------------=
    pub fn update_primary(
        &mut self,
        actor: StableId,
        primary_rgb: u32,
    ) -> Result<u32, IdentityError> {
        if !self
            .users
            .iter()
            .flatten()
            .any(|user| user.id == actor && user.state == UserState::Active)
        {
            return Err(IdentityError::AccessDenied);
        }
        if primary_rgb == 0 || primary_rgb > 0x00ff_ffff {
            return Err(IdentityError::InvalidInput);
        }
        self.primary_rgb = primary_rgb;
        self.commit();
        Ok(primary_rgb)
    }

    // ------------------------=
    // FUNC: background_effects
    // DESC: Returns the durable machine-wide background opacity percentage and blur radius.
    // ------------------=
    pub const fn background_effects(&self) -> (u8, u8) {
        (self.background_opacity, self.background_blur)
    }

    // ------------------------=
    // FUNC: update_background_effects
    // DESC: Persists validated background-only glass opacity and blur values for the active machine.
    // ------------------=
    pub fn update_background_effects(
        &mut self,
        actor: StableId,
        opacity: u8,
        blur: u8,
    ) -> Result<(u8, u8), IdentityError> {
        if !self
            .users
            .iter()
            .flatten()
            .any(|user| user.id == actor && user.state == UserState::Active)
        {
            return Err(IdentityError::AccessDenied);
        }
        if !(40..=100).contains(&opacity) || (opacity - 40) % 4 != 0 || blur > 8 {
            return Err(IdentityError::InvalidInput);
        }
        self.background_opacity = opacity;
        self.background_blur = blur;
        self.commit();
        Ok((opacity, blur))
    }

    // ------------------------=
    // FUNC: create_user
    // DESC: Atomically creates a native user, profiles, and isolated Personal Space ownership.
    // ------------------=
    pub fn create_user(
        &mut self,
        handle: &[u8],
        display_name: &[u8],
        now: u64,
    ) -> Result<UserIdentity, IdentityError> {
        validate_handle(handle)?;
        if self
            .users
            .iter()
            .flatten()
            .any(|u| eq_ascii(u.handle.as_bytes(), handle))
        {
            return Err(IdentityError::Conflict);
        }
        let slot = self
            .users
            .iter()
            .position(Option::is_none)
            .ok_or(IdentityError::Full)?;
        let user_id = self.allocate_id(0x55);
        let profile = self.allocate_id(0x50);
        let personal = self.allocate_id(0x53);
        let user = UserIdentity {
            id: user_id,
            display_name: ShortText::new(display_name)?,
            handle: ShortText::new(handle)?,
            created: now,
            state: UserState::Active,
            profile_ref: profile,
            personal_space_ref: personal,
        };
        self.users[slot] = Some(user);
        self.profiles[slot] = Some(UserProfile {
            user: user_id,
            language: ShortText::new(b"English (US)")?,
            region: ShortText::new(b"United States")?,
            theme: ShortText::new(b"Cosmic Dark")?,
            icon_theme: 0,
            accent_rgb: DEFAULT_ACCENT_RGB,
            no_activity_timeout_minutes: DEFAULT_NO_ACTIVITY_TIMEOUT_MINUTES,
        });
        self.ai_profiles[slot] = Some(AiProfile {
            user: user_id,
            provider_policy: AiProviderPolicy::LocalOnly,
            remote_processing: false,
            chat_enabled: true,
            chat_model_index: 0,
        });
        self.ai_memories[slot] = AiMemory::new();
        self.voice_profiles[slot] = Some(VoiceProfile {
            user: user_id,
            enabled: false,
            activation: VoiceActivation::Disabled,
        });
        self.ownership[slot] = Some(PersonalSpaceOwnership {
            owner: user_id,
            space: personal,
        });
        self.commit();
        Ok(user)
    }

    // ------------------------=
    // FUNC: user_count
    // DESC: Counts configured user identities, including deactivated audit identities.
    // ------------------=
    pub fn user_count(&self) -> usize {
        self.users.iter().flatten().count()
    }

    // ------------------------=
    // FUNC: user_nth
    // DESC: Returns one user for typed Identity.List results.
    // ------------------=
    pub fn user_nth(&self, index: usize) -> Option<UserIdentity> {
        self.users.iter().flatten().nth(index).copied()
    }

    // ------------------------=
    // FUNC: user
    // DESC: Reads a user by stable identity.
    // ------------------=
    pub fn user(&self, id: StableId) -> Option<UserIdentity> {
        self.users.iter().flatten().find(|u| u.id == id).copied()
    }

    // ------------------------=
    // FUNC: user_by_short
    // DESC: Resolves a human console ID projection to its full stable user identity.
    // ------------------=
    pub fn user_by_short(&self, value: u64) -> Option<UserIdentity> {
        self.users
            .iter()
            .flatten()
            .find(|user| user.id.short() == value)
            .copied()
    }

    // ------------------------=
    // FUNC: update_user_name
    // DESC: Updates allowed profile presentation data after self-or-admin authority validation.
    // ------------------=
    pub fn update_user_name(
        &mut self,
        actor: StableId,
        id: StableId,
        display_name: &[u8],
        administrator: bool,
    ) -> Result<UserIdentity, IdentityError> {
        if actor != id && !administrator {
            return Err(IdentityError::AccessDenied);
        }
        let text = ShortText::new(display_name)?;
        let user = self
            .users
            .iter_mut()
            .flatten()
            .find(|u| u.id == id)
            .ok_or(IdentityError::NotFound)?;
        user.display_name = text;
        let result = *user;
        self.commit();
        Ok(result)
    }

    // ------------------------=
    // FUNC: deactivate_user
    // DESC: Deactivates an identity without deleting its persistent audit identity or Personal Space.
    // ------------------=
    pub fn deactivate_user(
        &mut self,
        actor: StableId,
        id: StableId,
        administrator: bool,
    ) -> Result<(), IdentityError> {
        if actor == id || !administrator {
            return Err(IdentityError::AccessDenied);
        }
        let user = self
            .users
            .iter_mut()
            .flatten()
            .find(|u| u.id == id)
            .ok_or(IdentityError::NotFound)?;
        user.state = UserState::Deactivated;
        for session in self.sessions.iter_mut().flatten().filter(|s| s.user == id) {
            session.state = SessionState::Closed;
            session.capabilities = 0;
        }
        self.commit();
        Ok(())
    }

    // ------------------------=
    // FUNC: create_password
    // DESC: Derives and stores a salted PBKDF2-HMAC-SHA256 verifier without retaining plaintext.
    // ------------------=
    pub fn create_password(
        &mut self,
        owner: StableId,
        password: &[u8],
        now: u64,
    ) -> Result<CredentialDescriptor, IdentityError> {
        if password.len() < 8 || password.len() > 72 || self.user(owner).is_none() {
            return Err(IdentityError::InvalidInput);
        }
        let slot = self
            .credentials
            .iter()
            .position(Option::is_none)
            .ok_or(IdentityError::Full)?;
        let id = self.allocate_id(0x43);
        let mut salt = id.0;
        for (index, byte) in owner.0.iter().enumerate() {
            salt[index] ^= *byte;
        }
        let descriptor = CredentialDescriptor {
            id,
            owner,
            kind: CredentialType::Password,
            state: CredentialState::Active,
            created: now,
            last_used: 0,
        };
        self.credentials[slot] = Some(Credential {
            descriptor,
            salt,
            verifier: pbkdf2_hmac_sha256(password, &salt, PASSWORD_ITERATIONS),
            iterations: PASSWORD_ITERATIONS,
            failures: 0,
            retry_after: 0,
        });
        self.commit();
        Ok(descriptor)
    }

    // ------------------------=
    // FUNC: credential_descriptor
    // DESC: Returns safe credential metadata and never exposes verifier or salt bytes.
    // ------------------=
    pub fn credential_descriptor(&self, id: StableId) -> Option<CredentialDescriptor> {
        self.credentials
            .iter()
            .flatten()
            .find(|c| c.descriptor.id == id)
            .map(|c| c.descriptor)
    }

    // ------------------------=
    // FUNC: credential_nth
    // DESC: Returns safe credential metadata without exposing verifier material.
    // ------------------=
    pub fn credential_nth(&self, index: usize) -> Option<CredentialDescriptor> {
        self.credentials
            .iter()
            .flatten()
            .nth(index)
            .map(|value| value.descriptor)
    }

    // ------------------------=
    // FUNC: has_active_credential
    // DESC: Reports whether a user already owns an active credential without exposing secret material.
    // ------------------=
    pub fn has_active_credential(&self, owner: StableId) -> bool {
        self.credentials.iter().flatten().any(|credential| {
            credential.descriptor.owner == owner
                && credential.descriptor.state == CredentialState::Active
        })
    }

    // ------------------------=
    // FUNC: revoke_credential
    // DESC: Revokes credential authority while retaining its non-secret audit descriptor.
    // ------------------=
    pub fn revoke_credential(
        &mut self,
        actor: StableId,
        id: StableId,
        administrator: bool,
    ) -> Result<(), IdentityError> {
        let credential = self
            .credentials
            .iter_mut()
            .flatten()
            .find(|credential| credential.descriptor.id == id)
            .ok_or(IdentityError::NotFound)?;
        if credential.descriptor.owner != actor && !administrator {
            return Err(IdentityError::AccessDenied);
        }
        credential.descriptor.state = CredentialState::Revoked;
        credential.verifier.fill(0);
        self.commit();
        Ok(())
    }

    // ------------------------=
    // FUNC: authenticate
    // DESC: Verifies a credential in constant time and rate-limits repeated failures.
    // ------------------=
    pub fn authenticate(
        &mut self,
        user: StableId,
        secret: &[u8],
        now: u64,
    ) -> Result<(), IdentityError> {
        let credential = self
            .credentials
            .iter_mut()
            .flatten()
            .find(|c| c.descriptor.owner == user && c.descriptor.state == CredentialState::Active)
            .ok_or(IdentityError::InvalidCredential)?;
        if now < credential.retry_after {
            return Err(IdentityError::RateLimited);
        }
        let candidate = pbkdf2_hmac_sha256(secret, &credential.salt, credential.iterations);
        if !constant_time_eq(&candidate, &credential.verifier) {
            credential.failures = credential.failures.saturating_add(1);
            if credential.failures >= 3 {
                credential.retry_after =
                    now.saturating_add(2000u64 << (credential.failures - 3).min(5));
            }
            return Err(IdentityError::InvalidCredential);
        }
        credential.failures = 0;
        credential.retry_after = 0;
        credential.descriptor.last_used = now;
        self.commit();
        Ok(())
    }

    // ------------------------=
    // FUNC: create_session
    // DESC: Creates an authenticated user session with least-authority user capabilities.
    // ------------------=
    pub fn create_session(
        &mut self,
        user: StableId,
        secret: &[u8],
        now: u64,
    ) -> Result<Session, IdentityError> {
        self.authenticate(user, secret, now)?;
        let machine = self.machine.ok_or(IdentityError::InvalidState)?;
        let identity = self
            .user(user)
            .filter(|u| u.state == UserState::Active)
            .ok_or(IdentityError::NotFound)?;
        let slot = self
            .sessions
            .iter()
            .position(Option::is_none)
            .ok_or(IdentityError::Full)?;
        let is_owner = self
            .users
            .iter()
            .flatten()
            .find(|candidate| candidate.state == UserState::Active)
            .map(|candidate| candidate.id == user)
            .unwrap_or(false);
        let session = Session {
            id: self.allocate_id(0x4e),
            user,
            machine: machine.id,
            created: now,
            state: SessionState::Active,
            capabilities: ORDINARY_SESSION_CAPABILITIES
                | if is_owner { SESSION_IDENTITY_MANAGE } else { 0 },
            personal_space: identity.personal_space_ref,
            ai_context: self.allocate_id(0x41),
        };
        self.sessions[slot] = Some(session);
        Ok(session)
    }

    // ------------------------=
    // FUNC: session_nth
    // DESC: Returns one session, retaining closed entries for bounded audit history.
    // ------------------=
    pub fn session_nth(&self, index: usize) -> Option<Session> {
        self.sessions.iter().flatten().nth(index).copied()
    }

    // ------------------------=
    // FUNC: session_by_short
    // DESC: Resolves a human session ID projection without using it as security authority.
    // ------------------=
    pub fn session_by_short(&self, value: u64) -> Option<Session> {
        self.sessions
            .iter()
            .flatten()
            .find(|session| session.id.short() == value)
            .copied()
    }

    // ------------------------=
    // FUNC: personal_space
    // DESC: Reads one user's explicit Personal Space ownership relationship.
    // ------------------=
    pub fn personal_space(&self, user: StableId) -> Option<PersonalSpaceOwnership> {
        self.ownership
            .iter()
            .flatten()
            .find(|ownership| ownership.owner == user)
            .copied()
    }

    // ------------------------=
    // FUNC: user_profile
    // DESC: Reads one user's authoritative typed preference profile.
    // ------------------=
    pub fn user_profile(&self, user: StableId) -> Option<UserProfile> {
        self.profiles
            .iter()
            .flatten()
            .find(|profile| profile.user == user)
            .copied()
    }

    // ------------------------=
    // FUNC: user_desktop_layout
    // DESC: Reads one user's last durable desktop window and object placement state.
    // ------------------=
    pub fn user_desktop_layout(&self, user: StableId) -> Option<DesktopSessionLayout> {
        self.desktop_layouts.layout(user.0)
    }

    // ------------------------=
    // FUNC: update_user_desktop_layout
    // DESC: Updates desktop placement state only for the authenticated owning user.
    // ------------------=
    pub fn update_user_desktop_layout(
        &mut self,
        actor: StableId,
        user: StableId,
        layout: DesktopSessionLayout,
    ) -> Result<(), IdentityError> {
        if actor != user {
            return Err(IdentityError::AccessDenied);
        }
        if !self.users.iter().flatten().any(|candidate| candidate.id == user) {
            return Err(IdentityError::NotFound);
        }
        if !self.desktop_layouts.save(user.0, layout) {
            return Err(IdentityError::Full);
        }
        self.commit();
        Ok(())
    }

    // ------------------------=
    // FUNC: update_user_theme
    // DESC: Updates the user-scoped appearance preference without changing system policy.
    // ------------------=
    pub fn update_user_theme(
        &mut self,
        actor: StableId,
        user: StableId,
        theme: &[u8],
    ) -> Result<UserProfile, IdentityError> {
        if actor != user {
            return Err(IdentityError::AccessDenied);
        }
        let value = ShortText::new(theme)?;
        let profile = self
            .profiles
            .iter_mut()
            .flatten()
            .find(|profile| profile.user == user)
            .ok_or(IdentityError::NotFound)?;
        profile.theme = value;
        let result = *profile;
        self.commit();
        Ok(result)
    }

    // ------------------------=
    // FUNC: update_user_icon_theme
    // DESC: Persists one validated user-scoped desktop icon family selection.
    // ------------------=
    pub fn update_user_icon_theme(
        &mut self,
        actor: StableId,
        user: StableId,
        icon_theme: u8,
    ) -> Result<UserProfile, IdentityError> {
        if actor != user {
            return Err(IdentityError::AccessDenied);
        }
        if icon_theme >= crate::ui::icon_theme::ICON_THEME_COUNT {
            return Err(IdentityError::InvalidInput);
        }
        let profile = self
            .profiles
            .iter_mut()
            .flatten()
            .find(|profile| profile.user == user)
            .ok_or(IdentityError::NotFound)?;
        profile.icon_theme = icon_theme;
        let result = *profile;
        self.commit();
        Ok(result)
    }

    // ------------------------=
    // FUNC: update_user_accent
    // DESC: Persists one validated user-scoped RGB accent for every semantic UI surface.
    // ------------------=
    pub fn update_user_accent(
        &mut self,
        actor: StableId,
        user: StableId,
        accent_rgb: u32,
    ) -> Result<UserProfile, IdentityError> {
        if actor != user {
            return Err(IdentityError::AccessDenied);
        }
        if accent_rgb == 0 || accent_rgb > 0x00ff_ffff {
            return Err(IdentityError::InvalidInput);
        }
        let profile = self
            .profiles
            .iter_mut()
            .flatten()
            .find(|profile| profile.user == user)
            .ok_or(IdentityError::NotFound)?;
        profile.accent_rgb = accent_rgb;
        let result = *profile;
        self.commit();
        Ok(result)
    }

    // ------------------------=
    // FUNC: update_user_no_activity_timeout
    // DESC: Persists the owning user's bounded inactivity-lock deadline in whole minutes.
    // ------------------=
    pub fn update_user_no_activity_timeout(
        &mut self,
        actor: StableId,
        user: StableId,
        minutes: u8,
    ) -> Result<UserProfile, IdentityError> {
        if actor != user {
            return Err(IdentityError::AccessDenied);
        }
        if !(MIN_NO_ACTIVITY_TIMEOUT_MINUTES..=MAX_NO_ACTIVITY_TIMEOUT_MINUTES).contains(&minutes) {
            return Err(IdentityError::InvalidInput);
        }
        let profile = self
            .profiles
            .iter_mut()
            .flatten()
            .find(|profile| profile.user == user)
            .ok_or(IdentityError::NotFound)?;
        profile.no_activity_timeout_minutes = minutes;
        let result = *profile;
        self.commit();
        Ok(result)
    }

    // ------------------------=
    // FUNC: lock_session
    // DESC: Locks an active session and makes its UI inaccessible without ending it.
    // ------------------=
    pub fn lock_session(&mut self, id: StableId, actor: StableId) -> Result<(), IdentityError> {
        let session = self
            .sessions
            .iter_mut()
            .flatten()
            .find(|s| s.id == id)
            .ok_or(IdentityError::NotFound)?;
        if session.user != actor || session.state != SessionState::Active {
            return Err(IdentityError::AccessDenied);
        }
        session.state = SessionState::Locked;
        Ok(())
    }

    // ------------------------=
    // FUNC: unlock_session
    // DESC: Reauthenticates and resumes the same locked session.
    // ------------------=
    pub fn unlock_session(
        &mut self,
        id: StableId,
        secret: &[u8],
        now: u64,
    ) -> Result<(), IdentityError> {
        let user = self
            .sessions
            .iter()
            .flatten()
            .find(|s| s.id == id && s.state == SessionState::Locked)
            .map(|s| s.user)
            .ok_or(IdentityError::InvalidState)?;
        self.authenticate(user, secret, now)?;
        self.sessions
            .iter_mut()
            .flatten()
            .find(|s| s.id == id)
            .unwrap()
            .state = SessionState::Active;
        Ok(())
    }

    // ------------------------=
    // FUNC: end_session
    // DESC: Terminates a session and revokes all user-scoped capabilities while preserving its audit entry.
    // ------------------=
    pub fn end_session(&mut self, id: StableId, actor: StableId) -> Result<(), IdentityError> {
        let session = self
            .sessions
            .iter_mut()
            .flatten()
            .find(|s| s.id == id)
            .ok_or(IdentityError::NotFound)?;
        if session.user != actor {
            return Err(IdentityError::AccessDenied);
        }
        session.state = SessionState::Closing;
        session.capabilities = 0;
        session.state = SessionState::Closed;
        Ok(())
    }

    // ------------------------=
    // FUNC: ai_profile
    // DESC: Reads one user's scoped AI policy.
    // ------------------=
    pub fn ai_profile(&self, user: StableId) -> Option<AiProfile> {
        self.ai_profiles
            .iter()
            .flatten()
            .find(|p| p.user == user)
            .copied()
    }

    // ------------------------=
    // FUNC: update_ai_profile
    // DESC: Updates the authoritative user-scoped AI provider policy.
    // ------------------=
    pub fn update_ai_profile(
        &mut self,
        actor: StableId,
        user: StableId,
        policy: AiProviderPolicy,
    ) -> Result<AiProfile, IdentityError> {
        if actor != user {
            return Err(IdentityError::AccessDenied);
        }
        let profile = self
            .ai_profiles
            .iter_mut()
            .flatten()
            .find(|p| p.user == user)
            .ok_or(IdentityError::NotFound)?;
        profile.provider_policy = policy;
        profile.remote_processing = policy == AiProviderPolicy::RemoteAllowed;
        let result = *profile;
        self.commit();
        Ok(result)
    }

    // ------------------------=
    // FUNC: update_ai_chat_preferences
    // DESC: Persists desktop chat enablement and one installed model selection for the owning user.
    // ------------------=
    pub fn update_ai_chat_preferences(
        &mut self,
        actor: StableId,
        user: StableId,
        enabled: bool,
        model_index: u8,
    ) -> Result<AiProfile, IdentityError> {
        if actor != user || model_index as usize >= crate::runtime::ai::chat::CHAT_MODELS.len() {
            return Err(IdentityError::AccessDenied);
        }
        let profile = self
            .ai_profiles
            .iter_mut()
            .flatten()
            .find(|profile| profile.user == user)
            .ok_or(IdentityError::NotFound)?;
        profile.chat_enabled = enabled;
        profile.chat_model_index = model_index;
        let result = *profile;
        self.commit();
        Ok(result)
    }

    // ------------------------=
    // FUNC: read_ai_memory
    // DESC: Reads durable semantic AI memory only for its owning user.
    // ------------------=
    pub fn read_ai_memory(
        &self,
        actor: StableId,
        user: StableId,
    ) -> Result<AiMemory, IdentityError> {
        if actor != user {
            return Err(IdentityError::AccessDenied);
        }
        let slot = self
            .users
            .iter()
            .position(|candidate| candidate.map(|value| value.id) == Some(user))
            .ok_or(IdentityError::NotFound)?;
        Ok(self.ai_memories[slot])
    }

    // ------------------------=
    // FUNC: update_ai_memory
    // DESC: Replaces the owning user's bounded durable semantic AI memory.
    // ------------------=
    pub fn update_ai_memory(
        &mut self,
        actor: StableId,
        user: StableId,
        memory: AiMemory,
    ) -> Result<AiMemory, IdentityError> {
        if actor != user {
            return Err(IdentityError::AccessDenied);
        }
        let slot = self
            .users
            .iter()
            .position(|candidate| candidate.map(|value| value.id) == Some(user))
            .ok_or(IdentityError::NotFound)?;
        self.ai_memories[slot] = memory;
        self.commit();
        Ok(memory)
    }

    // ------------------------=
    // FUNC: voice_profile
    // DESC: Reads one user's scoped voice and microphone preference.
    // ------------------=
    pub fn voice_profile(&self, user: StableId) -> Option<VoiceProfile> {
        self.voice_profiles
            .iter()
            .flatten()
            .find(|p| p.user == user)
            .copied()
    }

    // ------------------------=
    // FUNC: update_voice_profile
    // DESC: Updates explicit voice enablement without granting implicit microphone access.
    // ------------------=
    pub fn update_voice_profile(
        &mut self,
        actor: StableId,
        user: StableId,
        enabled: bool,
        activation: VoiceActivation,
    ) -> Result<VoiceProfile, IdentityError> {
        if actor != user || (enabled && activation == VoiceActivation::Disabled) {
            return Err(IdentityError::AccessDenied);
        }
        let profile = self
            .voice_profiles
            .iter_mut()
            .flatten()
            .find(|p| p.user == user)
            .ok_or(IdentityError::NotFound)?;
        profile.enabled = enabled;
        profile.activation = if enabled {
            activation
        } else {
            VoiceActivation::Disabled
        };
        let result = *profile;
        self.commit();
        Ok(result)
    }

    // ------------------------=
    // FUNC: complete_onboarding
    // DESC: Commits setup only after machine, user, credential, ownership, and profiles all exist.
    // ------------------=
    pub fn complete_onboarding(&mut self) -> Result<(), IdentityError> {
        let ready = self.machine.is_some()
            && self
                .users
                .iter()
                .flatten()
                .any(|u| u.state == UserState::Active)
            && self
                .credentials
                .iter()
                .flatten()
                .any(|c| c.descriptor.state == CredentialState::Active)
            && self.ownership.iter().flatten().next().is_some();
        if !ready {
            self.onboarding = OnboardingState::RecoveryRequired;
            return Err(IdentityError::InvalidState);
        }
        self.onboarding = OnboardingState::Complete;
        self.commit();
        Ok(())
    }

    // ------------------------=
    // FUNC: encode
    // DESC: Serializes versioned architecture-neutral durable identity state with a checksum.
    // ------------------=
    pub fn encode(&self) -> [u8; IDENTITY_STATE_BYTES] {
        let mut out = [0u8; IDENTITY_STATE_BYTES];
        out[..8].copy_from_slice(b"INFIDN1\0");
        put16(&mut out, 8, IDENTITY_FORMAT_VERSION);
        put16(&mut out, 10, IDENTITY_STATE_BYTES as u16);
        out[12] = self.onboarding as u8;
        out[13] = self.user_count() as u8;
        put64(&mut out, 16, self.next_id);
        put64(&mut out, 24, self.generation);
        if let Some(machine) = self.machine {
            out[32] = 1;
            write_id(&mut out, 40, machine.id);
            write_text(&mut out, 56, machine.display_name);
            put32(&mut out, 105, machine.architecture);
            write_id(&mut out, 109, machine.installation_id);
            put64(&mut out, 125, machine.system_generation);
            put64(&mut out, 133, machine.created);
        }
        for (index, user) in self.users.iter().enumerate() {
            if let Some(user) = user {
                write_user(
                    &mut out,
                    160 + index * 160,
                    *user,
                    self.profiles[index],
                    self.ai_profiles[index],
                    self.voice_profiles[index],
                );
            }
        }
        for (index, credential) in self.credentials.iter().enumerate() {
            if let Some(value) = credential {
                write_credential(&mut out, 1440 + index * 120, *value);
            }
        }
        for (index, profile) in self.profiles.iter().enumerate() {
            if let Some(value) = profile {
                write_profile(&mut out, 2880 + index * 147, *value);
                out[USER_ICON_THEME_OFFSET + index] = value.icon_theme;
                let accent_at = USER_ACCENT_OFFSET + index * 3;
                out[accent_at] = ((value.accent_rgb >> 16) & 0xff) as u8;
                out[accent_at + 1] = ((value.accent_rgb >> 8) & 0xff) as u8;
                out[accent_at + 2] = (value.accent_rgb & 0xff) as u8;
            }
        }
        out[SYSTEM_PRIMARY_OFFSET] = ((self.primary_rgb >> 16) & 0xff) as u8;
        out[SYSTEM_PRIMARY_OFFSET + 1] = ((self.primary_rgb >> 8) & 0xff) as u8;
        out[SYSTEM_PRIMARY_OFFSET + 2] = (self.primary_rgb & 0xff) as u8;
        out[SYSTEM_BACKGROUND_EFFECTS_OFFSET] =
            encode_background_effects(self.background_opacity, self.background_blur);
        out[USER_DESKTOP_LAYOUTS_OFFSET
            ..USER_DESKTOP_LAYOUTS_OFFSET + DESKTOP_LAYOUT_STATE_BYTES]
            .copy_from_slice(&self.desktop_layouts.encode());
        out[USER_AI_MEMORY_OFFSET..USER_AI_MEMORY_OFFSET + 8].copy_from_slice(b"INFAIM1\0");
        out[USER_AI_MEMORY_OFFSET + 8..USER_AI_MEMORY_OFFSET + 10]
            .copy_from_slice(&1u16.to_le_bytes());
        for index in 0..MAX_USERS {
            let at = USER_AI_MEMORY_OFFSET + 16 + index * AI_MEMORY_SLOT_BYTES;
            let _ = self.ai_memories[index].encode_into(&mut out[at..at + AI_MEMORY_SLOT_BYTES]);
        }
        let checksum = checksum32(&out[..IDENTITY_STATE_BYTES - 4]);
        put32(&mut out, IDENTITY_STATE_BYTES - 4, checksum);
        out
    }

    // ------------------------=
    // FUNC: decode
    // DESC: Validates and restores durable identity state without restoring transient sessions.
    // ------------------=
    pub fn decode(bytes: &[u8]) -> Result<Self, IdentityError> {
        if !matches!(
            bytes.len(),
            LEGACY_IDENTITY_STATE_BYTES | V2_IDENTITY_STATE_BYTES | IDENTITY_STATE_BYTES
        ) {
            return Err(IdentityError::CorruptState);
        }
        let version = get16(bytes, 8);
        let valid_shape = (version == 1 && bytes.len() == LEGACY_IDENTITY_STATE_BYTES)
            || (version == 2 && bytes.len() == V2_IDENTITY_STATE_BYTES)
            || (version == IDENTITY_FORMAT_VERSION && bytes.len() == IDENTITY_STATE_BYTES);
        if &bytes[..8] != b"INFIDN1\0"
            || !valid_shape
            || get16(bytes, 10) as usize != bytes.len()
            || get32(bytes, bytes.len() - 4) != checksum32(&bytes[..bytes.len() - 4])
        {
            return Err(IdentityError::CorruptState);
        }
        let mut state = Self::new();
        state.onboarding = onboarding_from(bytes[12])?;
        state.next_id = get64(bytes, 16).max(1);
        state.generation = get64(bytes, 24);
        let stored_primary = ((bytes[SYSTEM_PRIMARY_OFFSET] as u32) << 16)
            | ((bytes[SYSTEM_PRIMARY_OFFSET + 1] as u32) << 8)
            | bytes[SYSTEM_PRIMARY_OFFSET + 2] as u32;
        state.primary_rgb = if stored_primary == 0 {
            DEFAULT_PRIMARY_RGB
        } else {
            stored_primary
        };
        let (opacity, blur) = decode_background_effects(bytes[SYSTEM_BACKGROUND_EFFECTS_OFFSET]);
        state.background_opacity = opacity;
        state.background_blur = blur;
        if bytes[32] == 1 {
            state.machine = Some(MachineIdentity {
                id: read_id(bytes, 40),
                display_name: read_text(bytes, 56)?,
                architecture: get32(bytes, 105),
                installation_id: read_id(bytes, 109),
                system_generation: get64(bytes, 125),
                created: get64(bytes, 133),
            });
        }
        for index in 0..MAX_USERS {
            let offset = 160 + index * 160;
            if bytes[offset] != 0 {
                let user = read_user(bytes, offset)?;
                state.users[index] = Some(user.0);
                let profile_offset = 2880 + index * 147;
                state.profiles[index] = Some(if bytes[profile_offset] == 0 {
                    user.1
                } else {
                    read_profile(bytes, profile_offset, user.0.id)?
                });
                let icon_theme = bytes[USER_ICON_THEME_OFFSET + index];
                if icon_theme >= crate::ui::icon_theme::ICON_THEME_COUNT {
                    return Err(IdentityError::CorruptState);
                }
                if let Some(profile) = state.profiles[index].as_mut() {
                    profile.no_activity_timeout_minutes = user.1.no_activity_timeout_minutes;
                    profile.icon_theme = icon_theme;
                    let accent_at = USER_ACCENT_OFFSET + index * 3;
                    let stored_accent = ((bytes[accent_at] as u32) << 16)
                        | ((bytes[accent_at + 1] as u32) << 8)
                        | bytes[accent_at + 2] as u32;
                    profile.accent_rgb =
                        if stored_accent == 0 || stored_accent == LEGACY_DEFAULT_ACCENT_RGB {
                            DEFAULT_ACCENT_RGB
                        } else {
                            stored_accent
                        };
                }
                state.ai_profiles[index] = Some(user.2);
                state.voice_profiles[index] = Some(user.3);
                state.ownership[index] = Some(PersonalSpaceOwnership {
                    owner: user.0.id,
                    space: user.0.personal_space_ref,
                });
            }
        }
        for index in 0..MAX_CREDENTIALS {
            let offset = 1440 + index * 120;
            if bytes[offset] != 0 {
                state.credentials[index] = Some(read_credential(bytes, offset)?);
            }
        }
        if version >= 2 {
            state.desktop_layouts = PersistentDesktopLayoutStore::decode(
                &bytes[USER_DESKTOP_LAYOUTS_OFFSET
                    ..USER_DESKTOP_LAYOUTS_OFFSET + DESKTOP_LAYOUT_STATE_BYTES],
            )
            .ok_or(IdentityError::CorruptState)?;
        }
        if version >= 3 {
            if &bytes[USER_AI_MEMORY_OFFSET..USER_AI_MEMORY_OFFSET + 8] != b"INFAIM1\0"
                || get16(bytes, USER_AI_MEMORY_OFFSET + 8) != 1
            {
                return Err(IdentityError::CorruptState);
            }
            for index in 0..MAX_USERS {
                if state.users[index].is_none() {
                    continue;
                }
                let at = USER_AI_MEMORY_OFFSET + 16 + index * AI_MEMORY_SLOT_BYTES;
                state.ai_memories[index] =
                    AiMemory::decode_from(&bytes[at..at + AI_MEMORY_SLOT_BYTES])
                        .ok_or(IdentityError::CorruptState)?;
            }
        }
        Ok(state)
    }

    // ------------------------=
    // FUNC: generation
    // DESC: Returns the authoritative state generation used by GUI refresh and IEF events.
    // ------------------=
    pub fn generation(&self) -> u64 {
        self.generation
    }

    // ------------------------=
    // FUNC: commit
    // DESC: Advances authoritative state before any observer notification is emitted.
    // ------------------=
    fn commit(&mut self) {
        self.generation = self.generation.saturating_add(1);
    }

    // ------------------------=
    // FUNC: allocate_id
    // DESC: Allocates a stable typed 128-bit identity independent of display names and paths.
    // ------------------=
    fn allocate_id(&mut self, domain: u8) -> StableId {
        let value = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let mut id = [0u8; 16];
        id[..8].copy_from_slice(&value.to_le_bytes());
        id[8] = domain;
        id[9..16].copy_from_slice(&mix64(value ^ domain as u64).to_le_bytes()[..7]);
        StableId(id)
    }
}

// ------------------------=
// FUNC: validate_handle
// DESC: Validates a portable human handle without treating it as identity or authority.
// ------------------=
fn validate_handle(value: &[u8]) -> Result<(), IdentityError> {
    if value.len() < 2
        || value.len() > 32
        || !value[0].is_ascii_alphabetic()
        || !value
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(*b, b'-' | b'_'))
    {
        return Err(IdentityError::InvalidInput);
    }
    Ok(())
}

// ------------------------=
// FUNC: eq_ascii
// DESC: Compares ASCII identifiers without case sensitivity.
// ------------------=
fn eq_ascii(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(a, b)| a.to_ascii_lowercase() == b.to_ascii_lowercase())
}

// ------------------------=
// FUNC: constant_time_eq
// DESC: Compares verifier bytes without data-dependent early exit.
// ------------------=
fn constant_time_eq(left: &[u8; 32], right: &[u8; 32]) -> bool {
    let mut difference = 0u8;
    for index in 0..32 {
        difference |= left[index] ^ right[index];
    }
    difference == 0
}

// ------------------------=
// FUNC: pbkdf2_hmac_sha256
// DESC: Derives a 256-bit password verifier using PBKDF2-HMAC-SHA256.
// ------------------=
fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut input = [0u8; 68];
    input[..salt.len()].copy_from_slice(salt);
    input[salt.len()..salt.len() + 4].copy_from_slice(&1u32.to_be_bytes());
    let mut block = hmac_sha256(password, &input[..salt.len() + 4]);
    let mut out = block;
    for _ in 1..iterations {
        block = hmac_sha256(password, &block);
        for index in 0..32 {
            out[index] ^= block[index];
        }
    }
    out
}

// ------------------------=
// FUNC: hmac_sha256
// DESC: Computes HMAC-SHA256 for bounded credential derivation inputs.
// ------------------=
fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut normalized = [0u8; 64];
    if key.len() > 64 {
        normalized[..32].copy_from_slice(&sha256(key));
    } else {
        normalized[..key.len()].copy_from_slice(key);
    }
    let mut inner = [0u8; 160];
    let mut outer = [0u8; 96];
    for index in 0..64 {
        inner[index] = normalized[index] ^ 0x36;
        outer[index] = normalized[index] ^ 0x5c;
    }
    inner[64..64 + message.len()].copy_from_slice(message);
    let first = sha256(&inner[..64 + message.len()]);
    outer[64..96].copy_from_slice(&first);
    sha256(&outer)
}

// ------------------------=
// FUNC: sha256
// DESC: Computes SHA-256 without heap allocation for native credential verification.
// ------------------=
fn sha256(message: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h = [
        0x6a09e667u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    let total = ((message.len() + 9 + 63) / 64) * 64;
    for chunk in 0..total / 64 {
        let mut block = [0u8; 64];
        for index in 0..64 {
            let absolute = chunk * 64 + index;
            block[index] = if absolute < message.len() {
                message[absolute]
            } else if absolute == message.len() {
                0x80
            } else {
                0
            };
        }
        if chunk == total / 64 - 1 {
            block[56..64].copy_from_slice(&((message.len() as u64) * 8).to_be_bytes());
        }
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut z) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = z
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            z = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(z);
    }
    let mut out = [0u8; 32];
    for i in 0..8 {
        out[i * 4..i * 4 + 4].copy_from_slice(&h[i].to_be_bytes());
    }
    out
}

// ------------------------=
// FUNC: mix64
// DESC: Mixes a monotonic identity counter into non-sequential stable ID bytes.
// ------------------=
fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58476d1ce4e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

// ------------------------=
// FUNC: encode_background_effects
// DESC: Packs opacity and blur into the final backward-compatible preference byte.
// ------------------=
fn encode_background_effects(opacity: u8, blur: u8) -> u8 {
    let opacity_level = opacity.saturating_sub(40) / 4;
    (opacity_level << 4) | blur.min(8).saturating_add(1)
}

// ------------------------=
// FUNC: decode_background_effects
// DESC: Restores packed glass effects while mapping legacy zero bytes to the frosted defaults.
// ------------------=
fn decode_background_effects(packed: u8) -> (u8, u8) {
    if packed == 0 {
        return (DEFAULT_BACKGROUND_OPACITY, DEFAULT_BACKGROUND_BLUR);
    }
    let opacity = 40 + (packed >> 4).min(15) * 4;
    (opacity, (packed & 0x0f).saturating_sub(1).min(8))
}

// ------------------------=
// FUNC: checksum32
// DESC: Computes the identity-state integrity checksum.
// ------------------=
fn checksum32(bytes: &[u8]) -> u32 {
    let mut value = 0x811c9dc5u32;
    for byte in bytes {
        value ^= *byte as u32;
        value = value.wrapping_mul(0x01000193);
    }
    value
}

// ------------------------=
// FUNC: put16
// DESC: Writes an explicit little-endian 16-bit field.
// ------------------=
fn put16(out: &mut [u8], at: usize, value: u16) {
    out[at..at + 2].copy_from_slice(&value.to_le_bytes())
}
// ------------------------=
// FUNC: put32
// DESC: Writes an explicit little-endian 32-bit field.
// ------------------=
fn put32(out: &mut [u8], at: usize, value: u32) {
    out[at..at + 4].copy_from_slice(&value.to_le_bytes())
}
// ------------------------=
// FUNC: put64
// DESC: Writes an explicit little-endian 64-bit field.
// ------------------=
fn put64(out: &mut [u8], at: usize, value: u64) {
    out[at..at + 8].copy_from_slice(&value.to_le_bytes())
}
// ------------------------=
// FUNC: get16
// DESC: Reads an explicit little-endian 16-bit field.
// ------------------=
fn get16(input: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([input[at], input[at + 1]])
}
// ------------------------=
// FUNC: get32
// DESC: Reads an explicit little-endian 32-bit field.
// ------------------=
fn get32(input: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([input[at], input[at + 1], input[at + 2], input[at + 3]])
}
// ------------------------=
// FUNC: get64
// DESC: Reads an explicit little-endian 64-bit field.
// ------------------=
fn get64(input: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(input[at..at + 8].try_into().unwrap())
}
// ------------------------=
// FUNC: write_id
// DESC: Writes a stable 128-bit identity field.
// ------------------=
fn write_id(out: &mut [u8], at: usize, id: StableId) {
    out[at..at + 16].copy_from_slice(&id.0)
}
// ------------------------=
// FUNC: read_id
// DESC: Reads a stable 128-bit identity field.
// ------------------=
fn read_id(input: &[u8], at: usize) -> StableId {
    let mut id = [0u8; 16];
    id.copy_from_slice(&input[at..at + 16]);
    StableId(id)
}
// ------------------------=
// FUNC: write_text
// DESC: Writes one bounded text field with its explicit length.
// ------------------=
fn write_text(out: &mut [u8], at: usize, text: ShortText) {
    out[at] = text.length;
    out[at + 1..at + 49].copy_from_slice(&text.bytes)
}
// ------------------------=
// FUNC: read_text
// DESC: Validates and reads one bounded text field.
// ------------------=
fn read_text(input: &[u8], at: usize) -> Result<ShortText, IdentityError> {
    let length = input[at] as usize;
    if length > 48 {
        return Err(IdentityError::CorruptState);
    }
    let mut bytes = [0u8; 48];
    bytes.copy_from_slice(&input[at + 1..at + 49]);
    Ok(ShortText {
        bytes,
        length: length as u8,
    })
}

// ------------------------=
// FUNC: write_profile
// DESC: Persists the scoped language, region, and appearance preferences for one user.
// ------------------=
fn write_profile(out: &mut [u8], at: usize, profile: UserProfile) {
    write_text(out, at, profile.language);
    write_text(out, at + 49, profile.region);
    write_text(out, at + 98, profile.theme);
}

// ------------------------=
// FUNC: read_profile
// DESC: Restores a typed user preference profile from the versioned identity object.
// ------------------=
fn read_profile(input: &[u8], at: usize, user: StableId) -> Result<UserProfile, IdentityError> {
    Ok(UserProfile {
        user,
        language: read_text(input, at)?,
        region: read_text(input, at + 49)?,
        theme: read_text(input, at + 98)?,
        icon_theme: 0,
        accent_rgb: DEFAULT_ACCENT_RGB,
        no_activity_timeout_minutes: DEFAULT_NO_ACTIVITY_TIMEOUT_MINUTES,
    })
}

// ------------------------=
// FUNC: write_user
// DESC: Writes a user and its scoped profiles into one durable record.
// ------------------=
fn write_user(
    out: &mut [u8],
    at: usize,
    user: UserIdentity,
    profile: Option<UserProfile>,
    ai: Option<AiProfile>,
    voice: Option<VoiceProfile>,
) {
    out[at] = 1;
    out[at + 1] = user.state as u8;
    write_id(out, at + 2, user.id);
    write_text(out, at + 18, user.display_name);
    write_text(out, at + 67, user.handle);
    put64(out, at + 116, user.created);
    write_id(out, at + 124, user.profile_ref);
    write_id(out, at + 140, user.personal_space_ref);
    out[at + 156] = ai.map(|v| v.provider_policy as u8).unwrap_or(1);
    out[at + 157] = ai
        .map(|value| {
            0x80 | (value.remote_processing as u8)
                | ((value.chat_enabled as u8) << 1)
                | ((value.chat_model_index.min(15)) << 2)
        })
        .unwrap_or(0x82);
    let timeout_minutes = profile
        .map(|value| value.no_activity_timeout_minutes)
        .unwrap_or(DEFAULT_NO_ACTIVITY_TIMEOUT_MINUTES)
        .clamp(
            MIN_NO_ACTIVITY_TIMEOUT_MINUTES,
            MAX_NO_ACTIVITY_TIMEOUT_MINUTES,
        );
    out[at + 158] = voice.map(|v| v.enabled as u8).unwrap_or(0) | (timeout_minutes << 1);
    out[at + 159] = voice.map(|v| v.activation as u8).unwrap_or(1)
}

// ------------------------=
// FUNC: read_user
// DESC: Reads and validates a durable user and its scoped profiles.
// ------------------=
fn read_user(
    input: &[u8],
    at: usize,
) -> Result<(UserIdentity, UserProfile, AiProfile, VoiceProfile), IdentityError> {
    let id = read_id(input, at + 2);
    let user = UserIdentity {
        id,
        display_name: read_text(input, at + 18)?,
        handle: read_text(input, at + 67)?,
        created: get64(input, at + 116),
        state: if input[at + 1] == 1 {
            UserState::Active
        } else {
            UserState::Deactivated
        },
        profile_ref: read_id(input, at + 124),
        personal_space_ref: read_id(input, at + 140),
    };
    let profile = UserProfile {
        user: id,
        language: ShortText::new(b"English (US)")?,
        region: ShortText::new(b"United States")?,
        theme: ShortText::new(b"Cosmic Dark")?,
        icon_theme: 0,
        accent_rgb: DEFAULT_ACCENT_RGB,
        no_activity_timeout_minutes: match input[at + 158] >> 1 {
            0 => DEFAULT_NO_ACTIVITY_TIMEOUT_MINUTES,
            value if value <= MAX_NO_ACTIVITY_TIMEOUT_MINUTES => value,
            _ => return Err(IdentityError::CorruptState),
        },
    };
    let ai_preferences = input[at + 157];
    let ai_preferences_versioned = ai_preferences & 0x80 != 0;
    let ai = AiProfile {
        user: id,
        provider_policy: match input[at + 156] {
            1 => AiProviderPolicy::LocalOnly,
            2 => AiProviderPolicy::PreferLocal,
            3 => AiProviderPolicy::AskBeforeRemote,
            4 => AiProviderPolicy::RemoteAllowed,
            _ => return Err(IdentityError::CorruptState),
        },
        remote_processing: ai_preferences & 1 != 0,
        chat_enabled: !ai_preferences_versioned || ai_preferences & 2 != 0,
        chat_model_index: if ai_preferences_versioned {
            ((ai_preferences >> 2) & 0x0f).min(
                crate::runtime::ai::chat::CHAT_MODELS
                    .len()
                    .saturating_sub(1) as u8,
            )
        } else {
            0
        },
    };
    let voice = VoiceProfile {
        user: id,
        enabled: input[at + 158] != 0,
        activation: if input[at + 159] == 2 {
            VoiceActivation::PushToTalk
        } else {
            VoiceActivation::Disabled
        },
    };
    Ok((user, profile, ai, voice))
}

// ------------------------=
// FUNC: write_credential
// DESC: Writes credential metadata and verifier without any plaintext secret.
// ------------------=
fn write_credential(out: &mut [u8], at: usize, value: Credential) {
    out[at] = 1;
    out[at + 1] = value.descriptor.kind as u8;
    out[at + 2] = value.descriptor.state as u8;
    write_id(out, at + 4, value.descriptor.id);
    write_id(out, at + 20, value.descriptor.owner);
    put64(out, at + 36, value.descriptor.created);
    put64(out, at + 44, value.descriptor.last_used);
    out[at + 52..at + 68].copy_from_slice(&value.salt);
    out[at + 68..at + 100].copy_from_slice(&value.verifier);
    put32(out, at + 100, value.iterations)
}

// ------------------------=
// FUNC: read_credential
// DESC: Reads and validates a credential verifier record.
// ------------------=
fn read_credential(input: &[u8], at: usize) -> Result<Credential, IdentityError> {
    let mut salt = [0u8; 16];
    salt.copy_from_slice(&input[at + 52..at + 68]);
    let mut verifier = [0u8; 32];
    verifier.copy_from_slice(&input[at + 68..at + 100]);
    let iterations = get32(input, at + 100);
    if iterations < 1000 {
        return Err(IdentityError::CorruptState);
    }
    Ok(Credential {
        descriptor: CredentialDescriptor {
            id: read_id(input, at + 4),
            owner: read_id(input, at + 20),
            kind: if input[at + 1] == 1 {
                CredentialType::Password
            } else {
                CredentialType::Pin
            },
            state: if input[at + 2] == 1 {
                CredentialState::Active
            } else {
                CredentialState::Revoked
            },
            created: get64(input, at + 36),
            last_used: get64(input, at + 44),
        },
        salt,
        verifier,
        iterations,
        failures: 0,
        retry_after: 0,
    })
}

// ------------------------=
// FUNC: onboarding_from
// DESC: Validates the persisted first-boot lifecycle value.
// ------------------=
fn onboarding_from(value: u8) -> Result<OnboardingState, IdentityError> {
    match value {
        1 => Ok(OnboardingState::Required),
        2 => Ok(OnboardingState::InProgress),
        3 => Ok(OnboardingState::Complete),
        4 => Ok(OnboardingState::RecoveryRequired),
        _ => Err(IdentityError::CorruptState),
    }
}
