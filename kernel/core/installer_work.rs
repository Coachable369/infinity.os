//! One-shot handoff from explicit confirmation to disk work after presentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Work { Idle, Pending, Running }
impl Work {
    // ------------------------=
    // FUNC: confirm
    // DESC: Queues one authorized confirmation without executing or duplicating an active operation.
    // ------------------=
    pub fn confirm(&mut self) -> bool {
        if *self != Self::Idle { return false; }
        *self = Self::Pending;
        true
    }
    // ------------------------=
    // FUNC: begin
    // DESC: Starts only once and only after the input presentation transaction has completed.
    // ------------------=
    pub fn begin(&mut self, presentation_active: bool) -> bool {
        if presentation_active || *self != Self::Pending { return false; }
        *self = Self::Running;
        true
    }
    // ------------------------=
    // FUNC: busy
    // DESC: Gates all destructive/key/button actions while allowing cursor motion during the committed installation.
    // ------------------=
    pub fn busy(&self) -> bool { *self != Self::Idle }
    // ------------------------=
    // FUNC: finish
    // DESC: Releases the one-shot handoff only after storage returns a definitive outcome.
    // ------------------=
    pub fn finish(&mut self) { *self = Self::Idle; }
}
