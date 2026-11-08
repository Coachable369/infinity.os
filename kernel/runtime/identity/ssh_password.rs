//! Password SSH authentication uses native credentials and an independent native user session.
use super::*;

impl IdentitySystem {
    // ------------------------=
    // FUNC: authenticate_ssh_password
    // DESC: Verifies only account passwords with constant-time comparison and account-scoped monotonic SSH throttling.
    // ------------------=
    pub fn authenticate_ssh_password(&mut self, name: &[u8], secret: &[u8], now: u64) -> Result<(StableId, StableId), IdentityError> {
        if self.storage_uncertain { return Err(IdentityError::CommitUncertain); }
        if secret.is_empty() || secret.len() > 72 { return Err(IdentityError::InvalidCredential); }
        let user = self.user_by_handle(name).filter(|u| u.state == UserState::Active)
            .ok_or(IdentityError::InvalidCredential)?.id;
        let slot = self.users.iter().position(|u| u.is_some_and(|u| u.id == user)).ok_or(IdentityError::InvalidCredential)?;
        let throttle = &mut self.ssh_password_throttle[slot];
        if now < throttle.1 { return Err(IdentityError::RateLimited); }
        let credential = self.credentials.iter_mut().flatten().find(|c| c.descriptor.owner == user
            && c.descriptor.kind == CredentialType::Password && c.descriptor.state == CredentialState::Active)
            .ok_or(IdentityError::InvalidCredential)?;
        let mut candidate = pbkdf2_hmac_sha256(secret, &credential.salt, credential.iterations);
        let valid = constant_time_eq(&candidate, &credential.verifier);
        zeroize::Zeroize::zeroize(&mut candidate);
        if !valid {
            throttle.0 = throttle.0.saturating_add(1);
            if throttle.0 >= 3 {
                throttle.1 = now.saturating_add(2000u64 << (throttle.0 - 3).min(5));
            }
            return Err(IdentityError::InvalidCredential);
        }
        *throttle = (0, 0);
        Ok((user, credential.descriptor.id))
    }

    // ------------------------=
    // FUNC: ssh_password_active
    // DESC: Revokes remote password sessions when their account or exact authenticated credential is revoked.
    // ------------------=
    pub fn ssh_password_active(&self, user: StableId, credential: StableId) -> bool {
        !self.storage_uncertain && self.user(user).is_some_and(|u| u.state == UserState::Active)
            && self.credentials.iter().flatten().any(|c| c.descriptor.id == credential
                && c.descriptor.owner == user && c.descriptor.kind == CredentialType::Password
                && c.descriptor.state == CredentialState::Active)
    }

    // ------------------------=
    // FUNC: create_ssh_password_session
    // DESC: Creates the same account-derived remote authority and personal scopes as verified SSH key login.
    // ------------------=
    pub(crate) fn create_ssh_password_session(&mut self, user: StableId, credential: StableId, now: u64) -> Result<Session, IdentityError> {
        if !self.ssh_password_active(user, credential) { return Err(IdentityError::AccessDenied); }
        if self.sessions.iter().flatten().filter(|s| s.origin == SessionOrigin::Ssh && s.state != SessionState::Closed).count() >= 4 {
            return Err(IdentityError::Full);
        }
        let slot = self.sessions.iter().position(|s| s.is_none_or(|s| s.state == SessionState::Closed)).ok_or(IdentityError::Full)?;
        let machine = self.machine.ok_or(IdentityError::InvalidState)?.id;
        let personal_space=self.user(user).ok_or(IdentityError::NotFound)?.personal_space_ref;
        let session = Session { origin: SessionOrigin::Ssh, remote_key: None, remote_password: Some(credential),
            id: self.allocate_id(0x4e), user, machine, created: now, state: SessionState::Active,
            capabilities: self.console_capabilities(user), personal_space, ai_context: self.allocate_id(0x41) };
        self.sessions[slot] = Some(session);
        Ok(session)
    }
}
