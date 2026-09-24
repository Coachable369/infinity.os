//! Wall-clock-bounded inference scheduling, independent of firmware and models.
pub const BUDGET_NS: u64 = 2_000_000;
const CLOCKED_LIMIT: usize = 16_384;
const FALLBACK_LIMIT: usize = 256;

pub struct PumpBudget {
    started: Option<u64>,
    last: Option<u64>,
    polls: usize,
}
impl PumpBudget {
    // ------------------------=
    // FUNC: new
    // DESC: Starts an allocation-free deadline with a finite stalled-clock guard.
    // ------------------=
    pub fn new(now: Option<u64>) -> Self {
        Self { started: now, last: now, polls: 0 }
    }
    // ------------------------=
    // FUNC: next
    // DESC: Allows a service slice only inside the time budget, retaining the legacy cap when no clock exists.
    // ------------------=
    pub fn next(&mut self, now: Option<u64>) -> bool {
        let limit = match (self.started, now) {
            (Some(start), Some(current)) => {
                if current < self.last.unwrap_or(start) || current.saturating_sub(start) >= BUDGET_NS {
                    return false;
                }
                self.last = Some(current);
                CLOCKED_LIMIT
            }
            (None, None) => FALLBACK_LIMIT,
            _ => return false,
        };
        if self.polls >= limit { return false; }
        self.polls += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: fast_worker_polls_use_available_deadline
    // DESC: Verifies fast polls can cross the old cap but cannot cross the two millisecond deadline.
    // ------------------=
    #[test]
    fn fast_worker_polls_use_available_deadline() {
        let mut budget = PumpBudget::new(Some(100));
        for i in 0..2000 { assert!(budget.next(Some(100 + i * 1000))); }
        assert!(!budget.next(Some(100 + BUDGET_NS)));
    }
    // ------------------------=
    // FUNC: broken_clocks_cannot_spin_forever
    // DESC: Verifies absent, stalled, disappearing, and regressing clocks terminate safely.
    // ------------------=
    #[test]
    fn broken_clocks_cannot_spin_forever() {
        let mut absent = PumpBudget::new(None);
        for _ in 0..FALLBACK_LIMIT { assert!(absent.next(None)); }
        assert!(!absent.next(None));
        let mut stalled = PumpBudget::new(Some(9));
        for _ in 0..CLOCKED_LIMIT { assert!(stalled.next(Some(9))); }
        assert!(!stalled.next(Some(9)));
        let mut lost = PumpBudget::new(Some(9));
        assert!(!lost.next(None));
        let mut backwards = PumpBudget::new(Some(9));
        assert!(backwards.next(Some(10)));
        assert!(!backwards.next(Some(9)));
    }
}
