//! Private bounded password entry. No plaintext accessor, Clone, or Debug.

use crate::runtime::identity::{AccountRequest, IdentityError, ShortText, StableId};

pub const PASSWORD_CAPACITY: usize = 72;

pub enum PasswordAction {
    Create { handle: ShortText, display_name: ShortText },
    Reset { user: StableId },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasswordProgress { Idle, Entering, Confirming, TooShort, Mismatch, Ready }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasswordShortcut { Consume, Dispatch, Lock }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasswordAuthority { Idle, Valid, SignIn, RestartRequired }

pub struct PasswordEntry {
    action: Option<PasswordAction>,
    session: StableId,
    password: [u8; PASSWORD_CAPACITY],
    confirmation: [u8; PASSWORD_CAPACITY],
    length: usize,
    confirmation_length: usize,
    progress: PasswordProgress,
}

impl PasswordEntry {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty private entry buffer with no retained account or session.
    // ------------------=
    pub const fn new() -> Self {
        Self { action: None, session: StableId::zero(), password: [0; PASSWORD_CAPACITY], confirmation: [0; PASSWORD_CAPACITY], length: 0, confirmation_length: 0, progress: PasswordProgress::Idle }
    }

    // ------------------------=
    // FUNC: begin
    // DESC: Clears previous secret material and binds the new action to its initiating session.
    // ------------------=
    pub fn begin(&mut self, session: StableId, action: PasswordAction) {
        self.cancel();
        self.session = session;
        self.action = Some(action);
        self.progress = PasswordProgress::Entering;
    }

    // ------------------------=
    // FUNC: active
    // DESC: Reports whether the console must intercept all input ahead of history and clipboard routing.
    // ------------------=
    pub fn active(&self) -> bool { self.action.is_some() }

    // ------------------------=
    // FUNC: session
    // DESC: Returns the bound authority identity without exposing any secret bytes.
    // ------------------=
    pub fn session(&self) -> StableId { self.session }

    // ------------------------=
    // FUNC: revalidate
    // DESC: Erases secrets on revoked or changed authority and distinguishes restart-required storage failures for input routing.
    // ------------------=
    pub fn revalidate(&mut self, current: StableId, authority: Result<(), IdentityError>) -> PasswordAuthority {
        if !self.active() { return PasswordAuthority::Idle; }
        let outcome = if authority == Err(IdentityError::CommitUncertain) {
            PasswordAuthority::RestartRequired
        } else if current != self.session || authority.is_err() {
            PasswordAuthority::SignIn
        } else {
            return PasswordAuthority::Valid;
        };
        self.cancel();
        outcome
    }

    // ------------------------=
    // FUNC: shortcut
    // DESC: Erases private input before shared desktop navigation; clipboard and history chords remain consumed.
    // ------------------=
    pub fn shortcut(&mut self, code: u8, desktop: bool) -> PasswordShortcut {
        if !self.active() { return PasswordShortcut::Dispatch; }
        if code == b'L' {
            self.cancel();
            return PasswordShortcut::Lock;
        }
        if desktop && (matches!(code, b'j' | b'i' | b'K')
            || super::desktop_shortcuts::switcher_request(false, Some(code), true, true).is_some()
            || super::desktop_shortcuts::desktop_action(code, true, true).is_some()) {
            self.cancel();
            return PasswordShortcut::Dispatch;
        }
        PasswordShortcut::Consume
    }

    // ------------------------=
    // FUNC: masked_length
    // DESC: Exposes only the current field length for rendering asterisks.
    // ------------------=
    pub fn masked_length(&self) -> usize {
        if self.progress == PasswordProgress::Confirming { self.confirmation_length } else { self.length }
    }

    // ------------------------=
    // FUNC: push
    // DESC: Accepts a printable byte only while editing, without truncating an overlong password.
    // ------------------=
    pub fn push(&mut self, byte: u8) -> bool {
        if !(0x20..=0x7e).contains(&byte) { return false; }
        let (buffer, length) = match self.progress {
            PasswordProgress::Entering => (&mut self.password, &mut self.length),
            PasswordProgress::Confirming => (&mut self.confirmation, &mut self.confirmation_length),
            _ => return false,
        };
        if *length == PASSWORD_CAPACITY { return false; }
        buffer[*length] = byte;
        *length += 1;
        true
    }

    // ------------------------=
    // FUNC: backspace
    // DESC: Erases the last byte in the active secret field.
    // ------------------=
    pub fn backspace(&mut self) {
        let (buffer, length) = match self.progress {
            PasswordProgress::Entering => (&mut self.password, &mut self.length),
            PasswordProgress::Confirming => (&mut self.confirmation, &mut self.confirmation_length),
            _ => return,
        };
        if *length > 0 { *length -= 1; wipe(&mut buffer[*length..*length + 1]); }
    }

    // ------------------------=
    // FUNC: submit
    // DESC: Requires two matching bounded entries before exposing a request to the backend callback.
    // ------------------=
    pub fn submit(&mut self) -> PasswordProgress {
        match self.progress {
            PasswordProgress::Entering if self.length < 8 => PasswordProgress::TooShort,
            PasswordProgress::Entering => { self.progress = PasswordProgress::Confirming; self.progress }
            PasswordProgress::Confirming => {
                let mut difference = self.length ^ self.confirmation_length;
                for index in 0..PASSWORD_CAPACITY { difference |= (self.password[index] ^ self.confirmation[index]) as usize; }
                if difference != 0 {
                    wipe(&mut self.password); wipe(&mut self.confirmation);
                    self.length = 0; self.confirmation_length = 0;
                    self.progress = PasswordProgress::Entering;
                    PasswordProgress::Mismatch
                } else { self.progress = PasswordProgress::Ready; self.progress }
            }
            _ => self.progress,
        }
    }

    // ------------------------=
    // FUNC: finish
    // DESC: Borrows the confirmed secret for exactly one backend call, then erases it on success or failure.
    // ------------------=
    pub fn finish<T>(&mut self, session: StableId, now: u64, execute: impl FnOnce(AccountRequest<'_>) -> Result<T, IdentityError>) -> Result<T, IdentityError> {
        if self.progress != PasswordProgress::Ready || session != self.session {
            self.cancel();
            return Err(IdentityError::InvalidState);
        }
        let result = match self.action.as_ref() {
            Some(PasswordAction::Create { handle, display_name }) => execute(AccountRequest::CreateUser { handle: handle.as_bytes(), display_name: display_name.as_bytes(), password: &self.password[..self.length], now }),
            Some(PasswordAction::Reset { user }) => execute(AccountRequest::ResetPassword { user: *user, password: &self.password[..self.length], now }),
            None => Err(IdentityError::InvalidState),
        };
        self.cancel();
        result
    }

    // ------------------------=
    // FUNC: cancel
    // DESC: Volatile-erases both password buffers and clears all pending authority and account state.
    // ------------------=
    pub fn cancel(&mut self) {
        wipe(&mut self.password); wipe(&mut self.confirmation);
        self.action = None; self.session = StableId::zero();
        self.length = 0; self.confirmation_length = 0;
        self.progress = PasswordProgress::Idle;
    }
}

impl Drop for PasswordEntry {
    // ------------------------=
    // FUNC: drop
    // DESC: Erases secrets if an entry state leaves scope before explicit completion.
    // ------------------=
    fn drop(&mut self) { self.cancel(); }
}

// ------------------------=
// FUNC: wipe
// DESC: Prevents optimization from removing the erasure of a secret buffer.
// ------------------=
fn wipe(bytes: &mut [u8]) {
    for byte in bytes { unsafe { core::ptr::write_volatile(byte, 0); } }
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
}
