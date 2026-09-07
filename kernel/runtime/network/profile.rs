//! Persistent operational connectivity profiles and atomic activation staging.

use super::types::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProfileKind {
    Standard,
    Restricted,
    Offline,
    Operations,
    Developer,
    Custom,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NetworkProfile {
    pub id: ProfileId,
    pub kind: ProfileKind,
    pub name_code: u32,
    pub interfaces_enabled: bool,
    pub dynamic_addressing: bool,
    pub resolver_enabled: bool,
    pub default_route_enabled: bool,
    pub local_discovery_enabled: bool,
    pub internet_allowed: bool,
    pub inbound_listeners_allowed: bool,
    pub audit_decisions: bool,
    pub protected: bool,
}

pub struct ProfileManager {
    profiles: [Option<NetworkProfile>; MAX_PROFILES],
    active: ProfileId,
    last_known_good: ProfileId,
    next_id: ProfileId,
    generation: u64,
}

impl ProfileManager {
    // ------------------------=
    // FUNC: new
    // DESC: Creates the profile store before built-in profiles are installed.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            profiles: [None; MAX_PROFILES],
            active: 0,
            last_known_good: 0,
            next_id: 6,
            generation: 0,
        }
    }

    // ------------------------=
    // FUNC: install_builtins
    // DESC: Installs deterministic Standard, Restricted, Offline, Operations, and Developer profiles.
    // ------------------=
    pub fn install_builtins(&mut self) {
        self.profiles[0] = Some(Self::builtin(
            1,
            ProfileKind::Standard,
            true,
            true,
            true,
            true,
            true,
            true,
            false,
            false,
            false,
        ));
        self.profiles[1] = Some(Self::builtin(
            2,
            ProfileKind::Restricted,
            true,
            true,
            true,
            true,
            false,
            false,
            false,
            true,
            true,
        ));
        self.profiles[2] = Some(Self::builtin(
            3,
            ProfileKind::Offline,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            true,
            false,
        ));
        self.profiles[3] = Some(Self::builtin(
            4,
            ProfileKind::Operations,
            true,
            false,
            true,
            true,
            true,
            false,
            true,
            true,
            true,
        ));
        self.profiles[4] = Some(Self::builtin(
            5,
            ProfileKind::Developer,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            false,
        ));
        if self.active == 0 {
            self.active = 1;
            self.last_known_good = 1;
            self.generation = 1;
        }
    }

    // ------------------------=
    // FUNC: builtin
    // DESC: Constructs one built-in profile with explicit policy values.
    // ------------------=
    const fn builtin(
        id: ProfileId,
        kind: ProfileKind,
        interfaces_enabled: bool,
        dynamic_addressing: bool,
        resolver_enabled: bool,
        default_route_enabled: bool,
        local_discovery_enabled: bool,
        internet_allowed: bool,
        inbound_listeners_allowed: bool,
        audit_decisions: bool,
        protected: bool,
    ) -> NetworkProfile {
        NetworkProfile {
            id,
            kind,
            name_code: id,
            interfaces_enabled,
            dynamic_addressing,
            resolver_enabled,
            default_route_enabled,
            local_discovery_enabled,
            internet_allowed,
            inbound_listeners_allowed,
            audit_decisions,
            protected,
        }
    }

    // ------------------------=
    // FUNC: create
    // DESC: Adds a validated custom profile to the bounded persistent collection.
    // ------------------=
    pub fn create(&mut self, mut profile: NetworkProfile) -> Result<ProfileId, NetworkError> {
        let slot = self
            .profiles
            .iter()
            .position(Option::is_none)
            .ok_or(NetworkError::ResourceLimitExceeded)?;
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(6);
        profile.id = id;
        profile.kind = ProfileKind::Custom;
        self.validate(profile)?;
        self.profiles[slot] = Some(profile);
        Ok(id)
    }

    // ------------------------=
    // FUNC: update
    // DESC: Updates a custom profile after validating coherent connectivity policy.
    // ------------------=
    pub fn update(
        &mut self,
        id: ProfileId,
        mut profile: NetworkProfile,
    ) -> Result<(), NetworkError> {
        if id <= 5 {
            return Err(NetworkError::AccessDenied);
        }
        self.validate(profile)?;
        let slot = self
            .profiles
            .iter()
            .position(|v| v.map(|p| p.id) == Some(id))
            .ok_or(NetworkError::InvalidProfile)?;
        profile.id = id;
        profile.kind = ProfileKind::Custom;
        self.profiles[slot] = Some(profile);
        Ok(())
    }

    // ------------------------=
    // FUNC: delete
    // DESC: Deletes an inactive custom profile while protecting built-ins and active state.
    // ------------------=
    pub fn delete(&mut self, id: ProfileId) -> Result<(), NetworkError> {
        if id <= 5 || id == self.active {
            return Err(NetworkError::AccessDenied);
        }
        let slot = self
            .profiles
            .iter()
            .position(|v| v.map(|p| p.id) == Some(id))
            .ok_or(NetworkError::InvalidProfile)?;
        self.profiles[slot] = None;
        Ok(())
    }

    // ------------------------=
    // FUNC: validate
    // DESC: Rejects internally contradictory profiles before any state is staged.
    // ------------------=
    pub fn validate(&self, profile: NetworkProfile) -> Result<(), NetworkError> {
        if !profile.interfaces_enabled
            && (profile.dynamic_addressing
                || profile.default_route_enabled
                || profile.resolver_enabled
                || profile.local_discovery_enabled
                || profile.internet_allowed
                || profile.inbound_listeners_allowed)
        {
            return Err(NetworkError::InvalidProfile);
        }
        if profile.internet_allowed && !profile.default_route_enabled {
            return Err(NetworkError::InvalidProfile);
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: stage
    // DESC: Validates and returns a profile without mutating active configuration.
    // ------------------=
    pub fn stage(&self, id: ProfileId) -> Result<NetworkProfile, NetworkError> {
        let profile = *self.read(id).ok_or(NetworkError::InvalidProfile)?;
        self.validate(profile)?;
        Ok(profile)
    }

    // ------------------------=
    // FUNC: commit
    // DESC: Atomically commits a previously staged and verified profile.
    // ------------------=
    pub fn commit(&mut self, profile: NetworkProfile) -> Result<u64, NetworkError> {
        self.validate(profile)?;
        self.last_known_good = self.active;
        self.active = profile.id;
        self.generation = self.generation.saturating_add(1);
        Ok(self.generation)
    }

    // ------------------------=
    // FUNC: rollback
    // DESC: Restores the last known-good profile after activation failure.
    // ------------------=
    pub fn rollback(&mut self) {
        if self.last_known_good != 0 {
            self.active = self.last_known_good;
            self.generation = self.generation.saturating_add(1);
        }
    }

    // ------------------------=
    // FUNC: active
    // DESC: Returns the committed active profile.
    // ------------------=
    pub fn active(&self) -> Option<&NetworkProfile> {
        self.read(self.active)
    }

    // ------------------------=
    // FUNC: active_id
    // DESC: Returns the stable active profile identifier.
    // ------------------=
    pub const fn active_id(&self) -> ProfileId {
        self.active
    }

    // ------------------------=
    // FUNC: generation
    // DESC: Returns the monotonic profile-commit generation.
    // ------------------=
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    // ------------------------=
    // FUNC: restore_active
    // DESC: Restores a persisted active profile after validating it against installed profiles.
    // ------------------=
    pub fn restore_active(&mut self, id: ProfileId, generation: u64) -> Result<(), NetworkError> {
        let profile = *self.read(id).ok_or(NetworkError::InvalidProfile)?;
        self.validate(profile)?;
        self.active = id;
        self.last_known_good = id;
        self.generation = generation.max(1);
        Ok(())
    }

    // ------------------------=
    // FUNC: read
    // DESC: Resolves a profile by stable identity.
    // ------------------=
    pub fn read(&self, id: ProfileId) -> Option<&NetworkProfile> {
        self.profiles.iter().flatten().find(|p| p.id == id)
    }

    // ------------------------=
    // FUNC: count
    // DESC: Returns the number of available profiles.
    // ------------------=
    pub fn count(&self) -> usize {
        self.profiles.iter().flatten().count()
    }

    // ------------------------=
    // FUNC: nth
    // DESC: Returns one profile for typed discovery.
    // ------------------=
    pub fn nth(&self, index: usize) -> Option<&NetworkProfile> {
        self.profiles.iter().flatten().nth(index)
    }
}
