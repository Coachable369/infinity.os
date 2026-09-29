//! Monotonic frame admission for coalescing continuously animating documents.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FramePacer { period: u64, next: u64 }

impl FramePacer {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a bounded native frame cadence from a nonzero period.
    // ------------------=
    pub const fn new(period: u64) -> Self { Self { period, next: 0 } }

    // ------------------------=
    // FUNC: admit
    // DESC: Admits one frame at the cadence boundary and coalesces intervening notifications.
    // ------------------=
    pub fn admit(&mut self, now: u64) -> bool {
        if self.period == 0 || now < self.next { return false; }
        self.next = now.saturating_add(self.period);
        true
    }

    // ------------------------=
    // FUNC: reset
    // DESC: Makes the next dirty frame immediately eligible after a document or visibility change.
    // ------------------=
    pub fn reset(&mut self) { self.next = 0; }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------=
    // FUNC: continuous_damage_is_coalesced_without_losing_next_frame
    // DESC: Verifies repeated damage cannot exceed the cadence and the boundary remains eligible.
    // ------------------=
    #[test]
    fn continuous_damage_is_coalesced_without_losing_next_frame() {
        let mut pacer=FramePacer::new(50);
        assert!(pacer.admit(100));
        for now in 101..150 { assert!(!pacer.admit(now)); }
        assert!(pacer.admit(150));
        assert!(!pacer.admit(151));
        pacer.reset();
        assert!(pacer.admit(151));
    }

    // ------------------------=
    // FUNC: invalid_and_overflowing_cadence_is_safe
    // DESC: Rejects a zero cadence and saturates the next deadline at the monotonic limit.
    // ------------------=
    #[test]
    fn invalid_and_overflowing_cadence_is_safe() {
        assert!(!FramePacer::new(0).admit(1));
        let mut pacer=FramePacer::new(50);
        assert!(pacer.admit(u64::MAX-10));
        assert!(!pacer.admit(u64::MAX-1));
        assert!(pacer.admit(u64::MAX));
    }
}
