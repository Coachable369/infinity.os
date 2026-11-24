//! Automatic two-factor sign-in with bounded retry cooldown and password-entry suppression.
use super::identity::StableId;
pub struct Gate { account: Option<StableId>, entered: u64, attempted: bool, password: bool }
impl Gate {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a disarmed gate until a login account becomes visible.
    // ------------------=
    pub const fn new() -> Self { Self { account: None, entered: 0, attempted: false, password: false } }
    // ------------------------=
    // FUNC: retry
    // DESC: Rearms a completed recoverable attempt once, with a 2-second cooldown; never clears password suppression.
    // ------------------=
    pub fn retry(&mut self, now: u64) {
        if self.attempted { self.attempted = false; self.entered = now.saturating_add(1250); }
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Starts after settling or retry cooldown; password entry suppresses capture for this visit.
    // ------------------=
    pub fn poll(&mut self, account: Option<StableId>, enabled: bool, password_started: bool, now: u64) -> bool {
        if self.account != account { self.account = account; self.entered = now; self.attempted = false; self.password = false; }
        if account.is_none() { return false; }
        if password_started { self.password = true; }
        if !enabled || self.password || self.attempted || now.saturating_sub(self.entered) < 750 { return false; }
        self.attempted = true;
        true
    }
}
