//! Small, deterministic animation state; no allocations or inference work.

#[derive(Default)]
pub struct ThinkingAnimation {
    pub frame: u64,
    active: bool,
}

impl ThinkingAnimation {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an inactive animation without allocation.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            frame: 0,
            active: false,
        }
    }
    // ------------------------=
    // FUNC: advance
    // DESC: Requests at most thirty header updates per second while generation is visible.
    // ------------------=
    pub fn advance(&mut self, active: bool, now_ns: u64) -> bool {
        let frame = now_ns / 33_333_333;
        let changed = active && (!self.active || frame != self.frame);
        self.active = active;
        self.frame = frame;
        changed
    }
}

// ------------------------=
// FUNC: grayscale
// DESC: Produces a smooth periodic grayscale highlight with a readable dim floor.
// ------------------=
pub fn grayscale(position: usize, offset: usize, period: usize) -> u8 {
    let period = period.max(2);
    let distance = (position + offset) % period;
    let triangle = distance.min(period - distance);
    (128 + triangle * 127 / (period / 2)) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: animation_lifecycle
    // DESC: Verifies frame throttling, cancellation, restart, and grayscale motion.
    // ------------------=
    #[test]
    fn animation_lifecycle() {
        let mut animation = ThinkingAnimation::default();
        assert!(!animation.advance(false, 0));
        assert!(animation.advance(true, 0));
        assert!(!animation.advance(true, 10_000_000));
        assert!(animation.advance(true, 34_000_000));
        assert!(!animation.advance(false, 68_000_000));
        assert!(!animation.advance(false, 99_000_000));
        assert!(animation.advance(true, 99_000_000));
        assert_ne!(grayscale(0, 0, 64), grayscale(0, 32, 64));
        for x in 0..128 {
            assert_eq!(grayscale(x, 0, 64), grayscale(x, 64, 64));
        }
    }
}
