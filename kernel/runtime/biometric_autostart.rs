//! One automatic two-factor attempt per login visit; retries remain explicit.
use super::identity::StableId;
pub struct Gate { account: Option<StableId>, entered: u64, attempted: bool }
impl Gate {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a disarmed gate until a login account becomes visible.
    // ------------------=
    pub const fn new() -> Self { Self { account: None, entered: 0, attempted: false } }
    // ------------------------=
    // FUNC: poll
    // DESC: Starts once after a short settling period; password entry suppresses capture for this visit.
    // ------------------=
    pub fn poll(&mut self, account: Option<StableId>, enabled: bool, password_started: bool, now: u64) -> bool {
        if self.account != account { self.account = account; self.entered = now; self.attempted = false; }
        if account.is_none() { return false; }
        if password_started { self.attempted = true; }
        if !enabled || self.attempted || now.saturating_sub(self.entered) < 750 { return false; }
        self.attempted = true;
        true
    }
}
