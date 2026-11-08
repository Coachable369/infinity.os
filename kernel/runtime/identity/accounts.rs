//! Bounded native account management. UI/CLI callers enter through accounts() or
//! manage_accounts(); direct identity getters remain trusted kernel inspection.
//! Deletion removes identity ownership, never the external Personal Space data.
//! Its preserved stable space ID is returned for storage recovery/audit purposes;
//! recreating the same handle allocates a different identity and Personal Space.

use super::*;

pub const MAX_GROUPS: usize = 16;
pub const ACCOUNTS_STATE_BYTES: usize = 16 + MAX_GROUPS * 136;
pub const ADMINISTRATORS_GROUP_ID: StableId = StableId([
    0, 0, 0, 0, 0, 0, 0, 0, 0x47, 0x41, 0x44, 0x4d, 0x49, 0x4e, 0, 1,
]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupIdentity {
    pub id: StableId,
    pub name: ShortText,
    pub display_name: ShortText,
    pub created: u64,
    pub builtin: bool,
}

// Intentionally no Debug: borrowed passwords must never reach diagnostics.
pub enum AccountRequest<'a> {
    CreateUser { handle: &'a [u8], display_name: &'a [u8], password: &'a [u8], now: u64 },
    RenameUser { user: StableId, handle: &'a [u8], display_name: &'a [u8] },
    SetUserEnabled { user: StableId, enabled: bool },
    DeleteUser { user: StableId },
    ResetPassword { user: StableId, password: &'a [u8], now: u64 },
    CreateGroup { name: &'a [u8], display_name: &'a [u8], now: u64 },
    RenameGroup { group: StableId, name: &'a [u8], display_name: &'a [u8] },
    DeleteGroup { group: StableId },
    AddMember { group: StableId, user: StableId },
    RemoveMember { group: StableId, user: StableId },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountResult {
    User(UserIdentity),
    Group(GroupIdentity),
    Membership { group: StableId, user: StableId, present: bool },
    DeletedUser { user: StableId, preserved_personal_space: StableId },
    DeletedGroup { group: StableId },
    PasswordReset { user: StableId },
}

/// Opaque rollback state includes sessions, salts/verifiers, throttle state and IDs.
/// Never serialize or log this value; consume it on rollback and drop on commit.
pub struct AccountCheckpoint(IdentitySystem);

/// A validated active-session read lease. Its shared borrow prevents authority
/// mutation while the lease is in use; it never exposes credential material.
pub struct AccountReader<'a> {
    identity: &'a IdentitySystem,
}

// ------------------------=
// FUNC: initial_groups
// DESC: Creates the immutable built-in administrator group without consuming user identity IDs.
// ------------------=
pub(super) const fn initial_groups() -> [Option<GroupIdentity>; MAX_GROUPS] {
    let mut name = [0; 48];
    let mut display = [0; 48];
    let mut index = 0;
    while index < b"administrators".len() {
        name[index] = b"administrators"[index];
        display[index] = b"Administrators"[index];
        index += 1;
    }
    let mut groups = [None; MAX_GROUPS];
    groups[0] = Some(GroupIdentity {
        id: ADMINISTRATORS_GROUP_ID,
        name: ShortText { bytes: name, length: 14 },
        display_name: ShortText { bytes: display, length: 14 },
        created: 0,
        builtin: true,
    });
    groups
}

impl IdentitySystem {
    // ------------------------=
    // FUNC: account_storage_uncertain
    // DESC: Requires a fresh boot after an ambiguous disk commit instead of authenticating against unconfirmed state.
    // ------------------=
    pub fn account_storage_uncertain(&self) -> bool { self.storage_uncertain }

    // ------------------------=
    // FUNC: latch_account_storage_uncertain
    // DESC: Revokes every session and blocks authentication until reboot resolves an ambiguous disk commit.
    // ------------------=
    pub fn latch_account_storage_uncertain(&mut self) {
        self.storage_uncertain = true;
        self.sessions = [None; MAX_SESSIONS];
    }

    // ------------------------=
    // FUNC: accounts
    // DESC: Authenticates an active session before exposing safe account directory metadata.
    // ------------------=
    pub fn accounts(&self, session: StableId) -> Result<AccountReader<'_>, IdentityError> {
        self.account_actor(session)?;
        Ok(AccountReader { identity: self })
    }

    // ------------------------=
    // FUNC: account_checkpoint
    // DESC: Captures complete bounded identity state including transient authentication state for rollback.
    // ------------------=
    pub fn account_checkpoint(&self) -> AccountCheckpoint {
        AccountCheckpoint(self.clone())
    }

    // ------------------------=
    // FUNC: restore_account_checkpoint
    // DESC: Restores trusted rollback state while preserving any already-latched storage uncertainty.
    // ------------------=
    pub fn restore_account_checkpoint(&mut self, checkpoint: AccountCheckpoint) {
        let uncertain = self.storage_uncertain || checkpoint.0.storage_uncertain;
        *self = checkpoint.0;
        if uncertain {
            self.latch_account_storage_uncertain();
        }
    }

    // ------------------------=
    // FUNC: manage_accounts_durable
    // DESC: Persists one authenticated change and rolls back all memory state if the atomic storage callback fails.
    // ------------------=
    pub fn manage_accounts_durable(
        &mut self,
        session: StableId,
        request: AccountRequest<'_>,
        persist: impl FnOnce(&[u8], AccountResult) -> Result<(), IdentityError>,
    ) -> Result<AccountResult, IdentityError> {
        self.manage_accounts_enrolled_durable(session, request, None, persist)
    }

    // ------------------------=
    // FUNC: manage_accounts_enrolled_durable
    // DESC: Includes an optional generated SSH credential in the same rollback and persistence boundary as account creation.
    // ------------------=
    pub fn manage_accounts_enrolled_durable(
        &mut self, session: StableId, request: AccountRequest<'_>, key: Option<[u8; 32]>,
        persist: impl FnOnce(&[u8], AccountResult) -> Result<(), IdentityError>,
    ) -> Result<AccountResult, IdentityError> {
        if key.is_some() && !matches!(request, AccountRequest::CreateUser { .. }) {
            return Err(IdentityError::InvalidInput);
        }
        let checkpoint = self.account_checkpoint();
        let result = match self.manage_accounts(session, request).and_then(|result| {
            if let (Some(key), AccountResult::User(user)) = (key, result) {
                self.enroll_generated_ssh_key(user.id, key, user.created)?;
            }
            Ok(result)
        }) {
            Ok(result) => result,
            Err(error) => {
                self.restore_account_checkpoint(checkpoint);
                return Err(error);
            }
        };
        if let Err(error) = persist(&self.encode(), result) {
            self.restore_account_checkpoint(checkpoint);
            if error == IdentityError::CommitUncertain {
                self.latch_account_storage_uncertain();
            }
            return Err(error);
        }
        Ok(result)
    }

    // ------------------------=
    // FUNC: manage_accounts
    // DESC: Dispatches bounded account changes using live session authority and current administrator membership.
    // ------------------=
    pub fn manage_accounts(
        &mut self,
        session: StableId,
        request: AccountRequest<'_>,
    ) -> Result<AccountResult, IdentityError> {
        let actor = self.account_actor(session)?;
        let self_service = match &request {
            AccountRequest::RenameUser { user, .. } | AccountRequest::ResetPassword { user, .. } => {
                *user == actor.user && actor.capabilities & SESSION_MANAGE_SELF != 0
            }
            _ => false,
        };
        if !self_service
            && (actor.capabilities & SESSION_IDENTITY_MANAGE == 0 || !self.is_administrator(actor.user))
        {
            return Err(IdentityError::AccessDenied);
        }
        let result = match request {
            AccountRequest::CreateUser { handle, display_name, password, now } => {
                validate_password(password)?;
                if !self.credentials.iter().any(|credential| {
                    credential.is_none_or(|credential| credential.descriptor.state == CredentialState::Revoked)
                }) {
                    return Err(IdentityError::Full);
                }
                let user = self.create_user(handle, display_name, now)?;
                self.reclaim_revoked_credentials();
                self.create_password(user.id, password, now)?;
                AccountResult::User(user)
            }
            AccountRequest::RenameUser { user, handle, display_name } => {
                validate_handle(handle)?;
                let display_name = ShortText::new(display_name)?;
                let handle_text = ShortText::new(handle)?;
                let slot = self.account_user_slot(user)?;
                if self.users.iter().flatten().any(|candidate| {
                    candidate.id != user && eq_ascii(candidate.handle.as_bytes(), handle)
                }) {
                    return Err(IdentityError::Conflict);
                }
                let record = self.users[slot].as_mut().unwrap();
                record.handle = handle_text;
                record.display_name = display_name;
                AccountResult::User(*record)
            }
            AccountRequest::SetUserEnabled { user, enabled } => {
                let slot = self.account_user_slot(user)?;
                if !enabled {
                    if user == actor.user {
                        return Err(IdentityError::AccessDenied);
                    }
                    self.ensure_removable_administrator(user)?;
                }
                let record = self.users[slot].as_mut().unwrap();
                record.state = if enabled { UserState::Active } else { UserState::Deactivated };
                let result = *record;
                if !enabled {
                    self.remove_account_sessions(user);
                }
                AccountResult::User(result)
            }
            AccountRequest::DeleteUser { user } => {
                let slot = self.account_user_slot(user)?;
                if user == actor.user {
                    return Err(IdentityError::AccessDenied);
                }
                self.ensure_removable_administrator(user)?;
                let personal_space = self.users[slot].unwrap().personal_space_ref;
                self.erase_user_credentials(user);
                self.remove_account_sessions(user);
                self.users[slot] = None;
                self.profiles[slot] = None;
                self.ai_profiles[slot] = None;
                self.voice_profiles[slot] = None;
                self.ai_memories[slot] = AiMemory::new();
                self.ownership[slot] = None;
                for members in &mut self.memberships {
                    *members &= !(1 << slot);
                }
                // Rebuild through the existing layout API to reclaim the deleted user's slot.
                let mut layouts = PersistentDesktopLayoutStore::new();
                for survivor in self.users.iter().flatten() {
                    if let Some(layout) = self.desktop_layouts.layout(survivor.id.0) {
                        layouts.save(survivor.id.0, layout);
                    }
                }
                self.desktop_layouts = layouts;
                AccountResult::DeletedUser { user, preserved_personal_space: personal_space }
            }
            AccountRequest::ResetPassword { user, password, now } => {
                self.account_user_slot(user)?;
                validate_password(password)?;
                if !self.credentials.iter().any(|credential| {
                    credential.is_none_or(|credential| {
                        credential.descriptor.owner == user
                            || credential.descriptor.state == CredentialState::Revoked
                    })
                }) {
                    return Err(IdentityError::Full);
                }
                self.erase_user_credentials(user);
                self.reclaim_revoked_credentials();
                self.create_password(user, password, now)?;
                let initiating_session = (user == actor.user).then_some(actor);
                self.remove_account_sessions(user);
                if let Some(initiating_session) = initiating_session {
                    let slot = self.sessions.iter_mut().find(|slot| slot.is_none()).unwrap();
                    *slot = Some(initiating_session);
                }
                AccountResult::PasswordReset { user }
            }
            AccountRequest::CreateGroup { name, display_name, now } => {
                validate_handle(name)?;
                let name_text = ShortText::new(name)?;
                let display_name = ShortText::new(display_name)?;
                if self.group_by_name(name).is_some() {
                    return Err(IdentityError::Conflict);
                }
                let slot = self.groups.iter().position(Option::is_none).ok_or(IdentityError::Full)?;
                let group = GroupIdentity {
                    id: self.allocate_id(0x47), name: name_text, display_name, created: now, builtin: false,
                };
                self.groups[slot] = Some(group);
                self.memberships[slot] = 0;
                AccountResult::Group(group)
            }
            AccountRequest::RenameGroup { group, name, display_name } => {
                let slot = self.account_group_slot(group)?;
                if self.groups[slot].unwrap().builtin {
                    return Err(IdentityError::AccessDenied);
                }
                validate_handle(name)?;
                let name_text = ShortText::new(name)?;
                let display_name = ShortText::new(display_name)?;
                if self.groups.iter().flatten().any(|candidate| {
                    candidate.id != group && eq_ascii(candidate.name.as_bytes(), name)
                }) {
                    return Err(IdentityError::Conflict);
                }
                let record = self.groups[slot].as_mut().unwrap();
                record.name = name_text;
                record.display_name = display_name;
                AccountResult::Group(*record)
            }
            AccountRequest::DeleteGroup { group } => {
                let slot = self.account_group_slot(group)?;
                if self.groups[slot].unwrap().builtin {
                    return Err(IdentityError::AccessDenied);
                }
                self.groups[slot] = None;
                self.memberships[slot] = 0;
                AccountResult::DeletedGroup { group }
            }
            AccountRequest::AddMember { group, user } => {
                self.change_membership(group, user, true)?;
                AccountResult::Membership { group, user, present: true }
            }
            AccountRequest::RemoveMember { group, user } => {
                self.change_membership(group, user, false)?;
                AccountResult::Membership { group, user, present: false }
            }
        };
        self.refresh_account_session_authority();
        self.commit();
        Ok(result)
    }

    // ------------------------=
    // FUNC: account_actor
    // DESC: Resolves an exact active session and active user on this machine, rejecting user IDs as authority.
    // ------------------=
    fn account_actor(&self, id: StableId) -> Result<Session, IdentityError> {
        if self.storage_uncertain {
            return Err(IdentityError::CommitUncertain);
        }
        self.sessions.iter().flatten().find(|session| {
            session.id == id && session.state == SessionState::Active
                && self.machine.is_some_and(|machine| machine.id == session.machine)
                && self.user(session.user).is_some_and(|user| user.state == UserState::Active)
                && session.capabilities & SESSION_SETTINGS_READ != 0
                && (session.origin!=SessionOrigin::Ssh||self.console_session_authorized(session.id,session.user))
        }).copied().ok_or(IdentityError::AccessDenied)
    }

    // ------------------------=
    // FUNC: account_user_slot
    // DESC: Resolves a stable user ID to its bounded storage slot.
    // ------------------=
    fn account_user_slot(&self, user: StableId) -> Result<usize, IdentityError> {
        self.users.iter().position(|value| value.is_some_and(|value| value.id == user))
            .ok_or(IdentityError::NotFound)
    }

    // ------------------------=
    // FUNC: account_group_slot
    // DESC: Resolves a stable group ID to its bounded storage slot.
    // ------------------=
    fn account_group_slot(&self, group: StableId) -> Result<usize, IdentityError> {
        self.groups.iter().position(|value| value.is_some_and(|value| value.id == group))
            .ok_or(IdentityError::NotFound)
    }

    // ------------------------=
    // FUNC: ensure_removable_administrator
    // DESC: Prevents losing the last active administrator through membership removal, disabling or deletion.
    // ------------------=
    pub(super) fn ensure_removable_administrator(&self, user: StableId) -> Result<(), IdentityError> {
        if self.is_administrator(user) && !self.users.iter().flatten().any(|candidate| {
            candidate.id != user && self.is_administrator(candidate.id)
        }) {
            return Err(IdentityError::InvalidState);
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: change_membership
    // DESC: Validates both endpoints and applies an idempotent membership change with administrator safeguards.
    // ------------------=
    fn change_membership(&mut self, group: StableId, user: StableId, present: bool) -> Result<(), IdentityError> {
        let user_slot = self.account_user_slot(user)?;
        let group_slot = self.account_group_slot(group)?;
        if !present && group == ADMINISTRATORS_GROUP_ID {
            self.ensure_removable_administrator(user)?;
        }
        if present {
            self.memberships[group_slot] |= 1 << user_slot;
        } else {
            self.memberships[group_slot] &= !(1 << user_slot);
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: remove_account_sessions
    // DESC: Removes all authentication sessions belonging to a disabled, reset or deleted identity.
    // ------------------=
    fn remove_account_sessions(&mut self, user: StableId) {
        for slot in &mut self.sessions {
            if slot.is_some_and(|session| session.user == user) {
                *slot = None;
            }
        }
    }

    // ------------------------=
    // FUNC: erase_user_credentials
    // DESC: Clears verifier and salt material and reclaims every credential slot belonging to one user.
    // ------------------=
    fn erase_user_credentials(&mut self, user: StableId) {
        for slot in &mut self.credentials {
            if slot.is_some_and(|credential| credential.descriptor.owner == user) {
                erase_credential(slot);
            }
        }
    }

    // ------------------------=
    // FUNC: reclaim_revoked_credentials
    // DESC: Reclaims bounded revoked credential slots before account creation or password replacement.
    // ------------------=
    fn reclaim_revoked_credentials(&mut self) {
        for slot in &mut self.credentials {
            if slot.is_some_and(|credential| credential.descriptor.state == CredentialState::Revoked) {
                erase_credential(slot);
            }
        }
    }

    // ------------------------=
    // FUNC: refresh_account_session_authority
    // DESC: Applies current group authority to existing sessions immediately after account mutations.
    // ------------------=
    fn refresh_account_session_authority(&mut self) {
        for index in 0..MAX_SESSIONS {
            if let Some(session) = self.sessions[index] {
                let administrator = self.is_administrator(session.user);
                let record = self.sessions[index].as_mut().unwrap();
                record.capabilities &= !SESSION_IDENTITY_MANAGE;
                if administrator && matches!(record.state, SessionState::Active | SessionState::Locked) {
                    record.capabilities |= SESSION_IDENTITY_MANAGE;
                }
            }
        }
    }

    // ------------------------=
    // FUNC: user_by_handle
    // DESC: Resolves a user handle case-insensitively without treating it as authentication authority.
    // ------------------=
    pub fn user_by_handle(&self, handle: &[u8]) -> Option<UserIdentity> {
        self.users.iter().flatten().find(|user| eq_ascii(user.handle.as_bytes(), handle)).copied()
    }

    // ------------------------=
    // FUNC: group_count
    // DESC: Returns the number of configured groups including the built-in administrators group.
    // ------------------=
    pub fn group_count(&self) -> usize { self.groups.iter().flatten().count() }

    // ------------------------=
    // FUNC: group_nth
    // DESC: Returns one group in bounded storage order.
    // ------------------=
    pub fn group_nth(&self, index: usize) -> Option<GroupIdentity> {
        self.groups.iter().flatten().nth(index).copied()
    }

    // ------------------------=
    // FUNC: group
    // DESC: Resolves a group using its stable identity.
    // ------------------=
    pub fn group(&self, id: StableId) -> Option<GroupIdentity> {
        self.groups.iter().flatten().find(|group| group.id == id).copied()
    }

    // ------------------------=
    // FUNC: group_by_name
    // DESC: Resolves a group name using the same case-insensitive rules as user handles.
    // ------------------=
    pub fn group_by_name(&self, name: &[u8]) -> Option<GroupIdentity> {
        self.groups.iter().flatten().find(|group| eq_ascii(group.name.as_bytes(), name)).copied()
    }

    // ------------------------=
    // FUNC: is_administrator
    // DESC: Requires both active user state and explicit built-in administrator membership.
    // ------------------=
    pub fn is_administrator(&self, user: StableId) -> bool {
        self.user(user).is_some_and(|user| user.state == UserState::Active)
            && self.is_group_member(ADMINISTRATORS_GROUP_ID, user)
    }

    // ------------------------=
    // FUNC: is_group_member
    // DESC: Reads explicit membership while validating both identity endpoints.
    // ------------------=
    pub fn is_group_member(&self, group: StableId, user: StableId) -> bool {
        match (self.account_group_slot(group), self.account_user_slot(user)) {
            (Ok(group), Ok(user)) => self.memberships[group] & (1 << user) != 0,
            _ => false,
        }
    }

    // ------------------------=
    // FUNC: group_member_count
    // DESC: Counts users assigned to a group, including disabled users.
    // ------------------=
    pub fn group_member_count(&self, group: StableId) -> usize {
        self.users.iter().flatten().filter(|user| self.is_group_member(group, user.id)).count()
    }

    // ------------------------=
    // FUNC: group_member_nth
    // DESC: Returns one typed member identity in stable slot order.
    // ------------------=
    pub fn group_member_nth(&self, group: StableId, index: usize) -> Option<UserIdentity> {
        self.users.iter().flatten().filter(|user| self.is_group_member(group, user.id)).nth(index).copied()
    }

    // ------------------------=
    // FUNC: user_group_count
    // DESC: Counts all explicit groups assigned to one user.
    // ------------------=
    pub fn user_group_count(&self, user: StableId) -> usize {
        self.groups.iter().flatten().filter(|group| self.is_group_member(group.id, user)).count()
    }

    // ------------------------=
    // FUNC: user_group_nth
    // DESC: Returns one group assigned to a user in stable slot order.
    // ------------------=
    pub fn user_group_nth(&self, user: StableId, index: usize) -> Option<GroupIdentity> {
        self.groups.iter().flatten().filter(|group| self.is_group_member(group.id, user)).nth(index).copied()
    }

    // ------------------------=
    // FUNC: encode_accounts
    // DESC: Encodes versioned groups and bounded membership masks without Rust layout dependencies.
    // ------------------=
    pub(super) fn encode_accounts(&self, out: &mut [u8]) {
        out[..8].copy_from_slice(b"INFACC1\0");
        put16(out, 8, 1);
        put16(out, 10, ACCOUNTS_STATE_BYTES as u16);
        for (index, group) in self.groups.iter().enumerate() {
            if let Some(group) = group {
                let at = 16 + index * 136;
                out[at] = 1;
                out[at + 1] = group.builtin as u8;
                out[at + 2] = self.memberships[index];
                write_id(out, at + 8, group.id);
                write_text(out, at + 24, group.name);
                write_text(out, at + 73, group.display_name);
                put64(out, at + 122, group.created);
            }
        }
    }

    // ------------------------=
    // FUNC: decode_accounts
    // DESC: Restores the exact persisted group topology without reassigning administrator membership.
    // ------------------=
    pub(super) fn decode_accounts(&mut self, bytes: &[u8]) -> Result<(), IdentityError> {
        if bytes.len() != ACCOUNTS_STATE_BYTES || &bytes[..8] != b"INFACC1\0"
            || get16(bytes, 8) != 1 || get16(bytes, 10) as usize != ACCOUNTS_STATE_BYTES
        {
            return Err(IdentityError::CorruptState);
        }
        self.groups = [None; MAX_GROUPS];
        self.memberships = [0; MAX_GROUPS];
        for index in 0..MAX_GROUPS {
            let at = 16 + index * 136;
            if bytes[at] > 1 || bytes[at + 1] > 1 {
                return Err(IdentityError::CorruptState);
            }
            if bytes[at] == 0 {
                if bytes[at..at + 136].iter().any(|byte| *byte != 0) {
                    return Err(IdentityError::CorruptState);
                }
                continue;
            }
            self.groups[index] = Some(GroupIdentity {
                id: read_id(bytes, at + 8), name: read_text(bytes, at + 24)?,
                display_name: read_text(bytes, at + 73)?, created: get64(bytes, at + 122),
                builtin: bytes[at + 1] == 1,
            });
            self.memberships[index] = bytes[at + 2];
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: validate_accounts
    // DESC: Rejects duplicate identities, dangling memberships, malformed names and missing built-in authority.
    // ------------------=
    pub(super) fn validate_accounts(&self) -> Result<(), IdentityError> {
        if self.groups[0] != initial_groups()[0] {
            return Err(IdentityError::CorruptState);
        }
        let mut occupied = 0u8;
        for (index, user) in self.users.iter().enumerate() {
            if let Some(user) = user {
                if user.id.is_zero() || validate_handle(user.handle.as_bytes()).is_err()
                    || self.users[..index].iter().flatten().any(|prior| {
                        prior.id == user.id || eq_ascii(prior.handle.as_bytes(), user.handle.as_bytes())
                    })
                {
                    return Err(IdentityError::CorruptState);
                }
                occupied |= 1 << index;
            }
        }
        for (index, group) in self.groups.iter().enumerate() {
            if self.memberships[index] & !occupied != 0 {
                return Err(IdentityError::CorruptState);
            }
            if let Some(group) = group {
                if group.id.is_zero() || validate_handle(group.name.as_bytes()).is_err()
                    || (index != 0 && group.builtin)
                    || self.groups[..index].iter().flatten().any(|prior| {
                        prior.id == group.id || eq_ascii(prior.name.as_bytes(), group.name.as_bytes())
                    })
                {
                    return Err(IdentityError::CorruptState);
                }
            } else if self.memberships[index] != 0 {
                return Err(IdentityError::CorruptState);
            }
        }
        if self.users.iter().flatten().any(|user| user.state == UserState::Active)
            && !self.users.iter().flatten().any(|user| self.is_administrator(user.id))
        {
            return Err(IdentityError::CorruptState);
        }
        if self.credentials.iter().flatten().any(|credential| self.user(credential.descriptor.owner).is_none()) {
            return Err(IdentityError::CorruptState);
        }
        Ok(())
    }
}

impl AccountReader<'_> {
    // ------------------------=
    // FUNC: user_count
    // DESC: Counts directory users for an authenticated reader.
    // ------------------=
    pub fn user_count(&self) -> usize { self.identity.user_count() }

    // ------------------------=
    // FUNC: user_nth
    // DESC: Returns one safe user record for an authenticated reader.
    // ------------------=
    pub fn user_nth(&self, index: usize) -> Option<UserIdentity> { self.identity.user_nth(index) }

    // ------------------------=
    // FUNC: user
    // DESC: Resolves one user ID for an authenticated reader.
    // ------------------=
    pub fn user(&self, user: StableId) -> Option<UserIdentity> { self.identity.user(user) }

    // ------------------------=
    // FUNC: user_by_handle
    // DESC: Resolves one portable user handle for an authenticated reader.
    // ------------------=
    pub fn user_by_handle(&self, handle: &[u8]) -> Option<UserIdentity> { self.identity.user_by_handle(handle) }

    // ------------------------=
    // FUNC: group_count
    // DESC: Counts the available groups for an authenticated reader.
    // ------------------=
    pub fn group_count(&self) -> usize { self.identity.group_count() }

    // ------------------------=
    // FUNC: group_nth
    // DESC: Returns one group record for an authenticated reader.
    // ------------------=
    pub fn group_nth(&self, index: usize) -> Option<GroupIdentity> { self.identity.group_nth(index) }

    // ------------------------=
    // FUNC: group
    // DESC: Resolves one stable group ID for an authenticated reader.
    // ------------------=
    pub fn group(&self, group: StableId) -> Option<GroupIdentity> { self.identity.group(group) }

    // ------------------------=
    // FUNC: group_by_name
    // DESC: Resolves one group name for an authenticated reader.
    // ------------------=
    pub fn group_by_name(&self, name: &[u8]) -> Option<GroupIdentity> { self.identity.group_by_name(name) }

    // ------------------------=
    // FUNC: is_administrator
    // DESC: Reads current effective administrator authority for an authenticated reader.
    // ------------------=
    pub fn is_administrator(&self, user: StableId) -> bool { self.identity.is_administrator(user) }

    // ------------------------=
    // FUNC: is_group_member
    // DESC: Reads one membership relation for an authenticated reader.
    // ------------------=
    pub fn is_group_member(&self, group: StableId, user: StableId) -> bool {
        self.identity.is_group_member(group, user)
    }

    // ------------------------=
    // FUNC: group_member_count
    // DESC: Counts a group's members for an authenticated reader.
    // ------------------=
    pub fn group_member_count(&self, group: StableId) -> usize { self.identity.group_member_count(group) }

    // ------------------------=
    // FUNC: group_member_nth
    // DESC: Reads a group's member identity for an authenticated reader.
    // ------------------=
    pub fn group_member_nth(&self, group: StableId, index: usize) -> Option<UserIdentity> {
        self.identity.group_member_nth(group, index)
    }

    // ------------------------=
    // FUNC: user_group_count
    // DESC: Counts a user's groups for an authenticated reader.
    // ------------------=
    pub fn user_group_count(&self, user: StableId) -> usize { self.identity.user_group_count(user) }

    // ------------------------=
    // FUNC: user_group_nth
    // DESC: Reads a user's group identity for an authenticated reader.
    // ------------------=
    pub fn user_group_nth(&self, user: StableId, index: usize) -> Option<GroupIdentity> {
        self.identity.user_group_nth(user, index)
    }
}

// ------------------------=
// FUNC: validate_password
// DESC: Validates password bounds before any account state or credential slot is changed.
// ------------------=
fn validate_password(password: &[u8]) -> Result<(), IdentityError> {
    if !(8..=72).contains(&password.len()) { return Err(IdentityError::InvalidInput); }
    Ok(())
}

// ------------------------=
// FUNC: erase_credential
// DESC: Explicitly overwrites secret fields before reclaiming a credential slot.
// ------------------=
fn erase_credential(slot: &mut Option<Credential>) {
    if let Some(credential) = slot.as_mut() {
        for byte in credential.salt.iter_mut().chain(credential.verifier.iter_mut()) {
            // Volatile stores prevent dead-store elimination of the credential wipe.
            unsafe { core::ptr::write_volatile(byte, 0) };
        }
        credential.iterations = 0;
        credential.failures = 0;
        credential.retry_after = 0;
    }
    *slot = None;
}
