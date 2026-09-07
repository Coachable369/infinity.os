//! Trusted UI and secure-input boundaries that skins cannot override.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrustedSurface {
    Authentication,
    Lock,
    CapabilityConsent,
    DestructiveConfirmation,
    NodePairing,
    Recovery,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SecureInputLease {
    pub owner_context: u32,
    pub surface: TrustedSurface,
    pub expires_at: u64,
    pub generation: u32,
}

#[derive(Clone, Copy)]
pub struct TrustedWindowToken {
    owner_context: u32,
    generation: u32,
}

impl TrustedWindowToken {
    // ------------------------=
    // FUNC: authorizes
    // DESC: Confirms that a non-forgeable trusted-window grant belongs to the requested context.
    // ------------------=
    pub(super) const fn authorizes(self, owner_context: u32) -> bool {
        self.owner_context == owner_context && self.generation != 0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrustedUiError {
    Busy,
    AccessDenied,
    Expired,
}

pub struct TrustedUiManager {
    secure_input: Option<SecureInputLease>,
    generation: u32,
}

impl TrustedUiManager {
    // ------------------------=
    // FUNC: new
    // DESC: Creates the reserved trusted-overlay owner with no active secure-input lease.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            secure_input: None,
            generation: 1,
        }
    }

    // ------------------------=
    // FUNC: acquire_secure_input
    // DESC: Grants exclusive credential input to one trusted surface for a bounded lease.
    // ------------------=
    pub fn acquire_secure_input(
        &mut self,
        trusted: bool,
        owner_context: u32,
        surface: TrustedSurface,
        expires_at: u64,
    ) -> Result<SecureInputLease, TrustedUiError> {
        if !trusted {
            return Err(TrustedUiError::AccessDenied);
        }
        if self.secure_input.is_some() {
            return Err(TrustedUiError::Busy);
        }
        self.generation = self.generation.wrapping_add(1);
        let lease = SecureInputLease {
            owner_context,
            surface,
            expires_at,
            generation: self.generation,
        };
        self.secure_input = Some(lease);
        Ok(lease)
    }

    // ------------------------=
    // FUNC: release_secure_input
    // DESC: Releases secure input only when the caller presents the active lease generation.
    // ------------------=
    pub fn release_secure_input(&mut self, lease: SecureInputLease) -> Result<(), TrustedUiError> {
        if self.secure_input != Some(lease) {
            return Err(TrustedUiError::AccessDenied);
        }
        self.secure_input = None;
        Ok(())
    }

    // ------------------------=
    // FUNC: authorize_trusted_window
    // DESC: Derives a scoped trusted-z-order token only from the currently active secure-input lease.
    // ------------------=
    pub fn authorize_trusted_window(
        &self,
        lease: SecureInputLease,
    ) -> Result<TrustedWindowToken, TrustedUiError> {
        if self.secure_input != Some(lease) {
            return Err(TrustedUiError::AccessDenied);
        }
        Ok(TrustedWindowToken {
            owner_context: lease.owner_context,
            generation: lease.generation,
        })
    }

    // ------------------------=
    // FUNC: expire
    // DESC: Removes an expired secure-input lease without restarting its caller.
    // ------------------=
    pub fn expire(&mut self, now: u64) -> bool {
        if self
            .secure_input
            .map(|lease| now >= lease.expires_at)
            .unwrap_or(false)
        {
            self.secure_input = None;
            true
        } else {
            false
        }
    }

    // ------------------------=
    // FUNC: secure_input_owner
    // DESC: Returns the exclusive trusted input owner while its lease remains active at the supplied time.
    // ------------------=
    pub fn secure_input_owner(&self, now: u64) -> Option<u32> {
        self.secure_input
            .filter(|lease| now < lease.expires_at)
            .map(|lease| lease.owner_context)
    }
}
