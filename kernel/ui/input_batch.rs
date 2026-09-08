//! Coalesces presentation, never input events, during one bounded device drain.
pub struct PresentationBatch { active: bool, pending: bool }
impl PresentationBatch {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an idle presentation gate without an input queue.
    // ------------------=
    pub const fn new() -> Self { Self { active: false, pending: false } }
    // ------------------------=
    // FUNC: enter
    // DESC: Returns ownership only to the outer drain so nested actions cannot flush it prematurely.
    // ------------------=
    pub fn enter(&mut self) -> bool { let owner = !self.active; self.active = true; owner }
    // ------------------------=
    // FUNC: request
    // DESC: Records deferred damage while active; outside a drain presentation remains immediate.
    // ------------------=
    pub fn request(&mut self) -> bool {
        if self.active { self.pending = true; false } else { true }
    }
    // ------------------------=
    // FUNC: leave
    // DESC: Flushes exactly once for the outer drain, preserving all preceding state mutations.
    // ------------------=
    pub fn leave(&mut self, owner: bool) -> bool {
        if !owner { return false; }
        self.active = false; let pending = self.pending; self.pending = false; pending
    }
}

// ------------------------=
// FUNC: bounded_input_drain_preserves_every_action
// DESC: Verifies ten state transitions precede one presentation, nesting cannot flush, and idle input stays immediate.
// ------------------=
#[test]
fn bounded_input_drain_preserves_every_action() {
    let mut gate = PresentationBatch::new(); let mut accepted = 0;
    assert!(gate.request()); let owner = gate.enter();
    for _ in 0..10 { accepted += 1; assert!(!gate.request()); }
    let nested = gate.enter(); assert!(!gate.leave(nested)); assert_eq!(accepted, 10);
    assert!(gate.leave(owner)); assert!(!gate.leave(true)); assert!(gate.request());
}
