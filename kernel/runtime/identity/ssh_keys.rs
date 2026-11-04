//! Native account-owned SSH keys. Authentication yields only an identity lookup.
use super::*;

impl IdentitySystem {
    // ------------------------=
    // FUNC: enroll_generated_ssh_key
    // DESC: Adds a validated key only inside trusted account-creation transactions; never grants session authority.
    // ------------------=
    pub(super) fn enroll_generated_ssh_key(&mut self, user: StableId, key: [u8; 32], now: u64) -> Result<(), IdentityError> {
        if !self.user(user).is_some_and(|u| u.state == UserState::Active) { return Err(IdentityError::NotFound); }
        let verifying = ed25519_dalek::VerifyingKey::from_bytes(&key).map_err(|_| IdentityError::InvalidCredential)?;
        if verifying.is_weak() { return Err(IdentityError::InvalidCredential); }
        let index = self.credentials.iter().position(|c| c.is_none_or(|c| c.descriptor.state == CredentialState::Revoked))
            .ok_or(IdentityError::Full)?;
        let descriptor = CredentialDescriptor { id: self.allocate_id(0x43), owner: user,
            kind: CredentialType::SshEd25519, state: CredentialState::Active, created: now, last_used: 0 };
        self.credentials[index] = Some(Credential { descriptor, salt: [0; 16], verifier: key,
            iterations: 0, failures: 0, retry_after: 0 });
        self.commit();
        Ok(())
    }

    // ------------------------=
    // FUNC: create_onboarding_user_durable
    // DESC: Creates the initial account and SSH identity atomically while setup owns the machine and no user exists.
    // ------------------=
    pub fn create_onboarding_user_durable(&mut self, handle: &[u8], name: &[u8], now: u64, key: [u8; 32],
        persist: impl FnOnce(&[u8], UserIdentity) -> Result<(), IdentityError>) -> Result<UserIdentity, IdentityError> {
        if self.user_count() != 0 || self.storage_uncertain || self.onboarding != OnboardingState::InProgress
            || self.machine.is_none() { return Err(IdentityError::AccessDenied); }
        let checkpoint = self.account_checkpoint();
        let result = (|| {
            let user = self.create_user(handle, name, now)?;
            self.enroll_generated_ssh_key(user.id, key, now)?;
            persist(&self.encode(), user)?;
            Ok(user)
        })();
        if let Err(error) = result {
            self.restore_account_checkpoint(checkpoint);
            if error == IdentityError::CommitUncertain { self.latch_account_storage_uncertain(); }
        }
        result
    }

    // ------------------------=
    // FUNC: manage_ssh_key_durable
    // DESC: Registers or revokes an account's Ed25519 key using live local-session authority and atomic identity persistence.
    // ------------------=
    pub fn manage_ssh_key_durable(
        &mut self, session: StableId, owner: StableId, key: [u8; 32],
        revoke: bool, now: u64, persist: impl FnOnce(&[u8]) -> Result<(), IdentityError>,
    ) -> Result<CredentialDescriptor, IdentityError> {
        self.accounts(session)?;
        let actor = self.sessions.iter().flatten().find(|s| s.id == session)
            .copied().ok_or(IdentityError::AccessDenied)?;
        if !(actor.user == owner && actor.capabilities & SESSION_MANAGE_SELF != 0
            || actor.capabilities & SESSION_IDENTITY_MANAGE != 0 && self.is_administrator(actor.user)) {
            return Err(IdentityError::AccessDenied);
        }
        if !self.user(owner).is_some_and(|u| u.state == UserState::Active) {
            return Err(IdentityError::NotFound);
        }
        let verifying = ed25519_dalek::VerifyingKey::from_bytes(&key).map_err(|_| IdentityError::InvalidCredential)?;
        if verifying.is_weak() { return Err(IdentityError::InvalidCredential); }
        let existing = self.credentials.iter().position(|c| c.is_some_and(|c|
            c.descriptor.owner == owner && c.descriptor.kind == CredentialType::SshEd25519
                && c.descriptor.state == CredentialState::Active && constant_time_eq(&c.verifier, &key)));
        let checkpoint = self.account_checkpoint();
        let result = if revoke {
            let index = existing.ok_or(IdentityError::NotFound)?;
            let credential = self.credentials[index].as_mut().unwrap();
            credential.descriptor.state = CredentialState::Revoked;
            credential.verifier.fill(0);
            credential.descriptor
        } else {
            if existing.is_some() { return Err(IdentityError::Conflict); }
            let index = self.credentials.iter().position(|c| c.is_none_or(|c| c.descriptor.state == CredentialState::Revoked))
                .ok_or(IdentityError::Full)?;
            let descriptor = CredentialDescriptor { id: self.allocate_id(0x43), owner,
                kind: CredentialType::SshEd25519, state: CredentialState::Active, created: now, last_used: 0 };
            self.credentials[index] = Some(Credential { descriptor, salt: [0; 16], verifier: key,
                iterations: 0, failures: 0, retry_after: 0 });
            descriptor
        };
        self.commit();
        if let Err(error) = persist(&self.encode()) {
            self.restore_account_checkpoint(checkpoint);
            if error == IdentityError::CommitUncertain { self.latch_account_storage_uncertain(); }
            return Err(error);
        }
        Ok(result)
    }

    // ------------------------=
    // FUNC: ssh_key_owner
    // DESC: Authorizes a public-key offer against an active native account without creating any session or capability.
    // ------------------=
    pub fn ssh_key_owner(&self, name: &[u8], key: &[u8; 32]) -> Option<StableId> {
        if self.storage_uncertain { return None; }
        let user = self.user_by_handle(name)?;
        self.ssh_key_active(user.id, key).then_some(user.id)
    }

    // ------------------------=
    // FUNC: ssh_key_active
    // DESC: Rechecks a stable user's account and credential state for each live SSH transport.
    // ------------------=
    pub fn ssh_key_active(&self, user: StableId, key: &[u8; 32]) -> bool {
        !self.storage_uncertain && self.user(user).is_some_and(|u| u.state == UserState::Active)
            && self.credentials.iter().flatten().any(|c| c.descriptor.owner == user
                && c.descriptor.kind == CredentialType::SshEd25519
                && c.descriptor.state == CredentialState::Active && constant_time_eq(&c.verifier, key))
    }
}

impl IdentitySystem {
    // ------------------------=
    // FUNC: create_ssh_session
    // DESC: Creates a bounded remote-origin query session only for the key already verified by the SSH transport.
    // ------------------=
    pub(crate) fn create_ssh_session(&mut self, user: StableId, key: [u8; 32], now: u64) -> Result<Session, IdentityError> {
        if !self.ssh_key_active(user, &key) { return Err(IdentityError::AccessDenied); }
        if self.sessions.iter().flatten().filter(|s| s.origin == SessionOrigin::Ssh && s.state != SessionState::Closed).count() >= 4 {
            return Err(IdentityError::Full);
        }
        let slot = self.sessions.iter().position(|s| s.is_none_or(|s| s.state == SessionState::Closed)).ok_or(IdentityError::Full)?;
        let machine = self.machine.ok_or(IdentityError::InvalidState)?.id;
        let session = Session { origin: SessionOrigin::Ssh, remote_key: Some(key), id: self.allocate_id(0x4e), user, machine,
            created: now, state: SessionState::Active, capabilities: SESSION_REMOTE_QUERY,
            personal_space: StableId::zero(), ai_context: StableId::zero() };
        self.sessions[slot] = Some(session);
        Ok(session)
    }
    // ------------------------=
    // FUNC: console_session_authorized
    // DESC: Rechecks native identity, origin and the independent remote ceiling before every command or output delivery.
    // ------------------=
    pub fn console_session_authorized(&self, session: StableId, user: StableId) -> bool {
        if self.storage_uncertain || !self.user(user).is_some_and(|u| u.state == UserState::Active) { return false; }
        self.sessions.iter().flatten().any(|s| s.id == session && s.user == user && s.state == SessionState::Active
            && match s.origin { SessionOrigin::Local => s.capabilities & SESSION_DISPLAY_INPUT != 0,
                SessionOrigin::Ssh => s.capabilities == SESSION_REMOTE_QUERY && s.remote_key.is_some_and(|key| self.ssh_key_active(user, &key)) })
    }
}
