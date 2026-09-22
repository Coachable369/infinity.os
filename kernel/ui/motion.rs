//! Allocation-free, elapsed-time motion shared by native spatial interactions.
//! Positions use signed logical units; time is monotonic milliseconds.

// ------------------------=
// FUNC: ease_byte
// DESC: Maps linear byte progress to a monotonic glass-surface easing curve with exact endpoints.
// ------------------=
pub fn ease_byte(progress: u8) -> u8 {
    let t = u32::from(progress);
    (t * t * (3 * 255 - 2 * t) / (255 * 255)) as u8
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Motion {
    from: i32,
    to: i32,
    start: u64,
    duration: u32,
}

impl Motion {
    // ------------------------=
    // FUNC: settled
    // DESC: Creates a stationary value without scheduling animation work.
    // ------------------=
    pub const fn settled(value: i32) -> Self {
        Self {
            from: value,
            to: value,
            start: 0,
            duration: 0,
        }
    }

    // ------------------------=
    // FUNC: value
    // DESC: Samples smoothstep motion with exact endpoints and overflow-safe fixed-point arithmetic.
    // ------------------=
    pub fn value(&self, now: u64) -> i32 {
        let elapsed = now.saturating_sub(self.start);
        if self.duration == 0 || elapsed >= u64::from(self.duration) {
            return self.to;
        }
        let t = i128::from(elapsed) * 65536 / i128::from(self.duration);
        let eased = t * t * (3 * 65536 - 2 * t) / (65536 * 65536);
        (i128::from(self.from) + (i128::from(self.to) - i128::from(self.from)) * eased / 65536)
            as i32
    }

    // ------------------------=
    // FUNC: retarget
    // DESC: Reverses or redirects from the currently displayed position without snapping or queuing transitions.
    // ------------------=
    pub fn retarget(&mut self, destination: i32, now: u64, duration: u32, reduced_motion: bool) {
        let current = self.value(now);
        *self = Self {
            from: current,
            to: destination,
            start: now,
            duration: if reduced_motion || current == destination {
                0
            } else {
                duration.min(600)
            },
        };
    }

    // ------------------------=
    // FUNC: active
    // DESC: Requests further frames only while an actual transition remains unfinished.
    // ------------------=
    pub fn active(&self, now: u64) -> bool {
        self.from != self.to && now.saturating_sub(self.start) < u64::from(self.duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: elapsed_motion_is_frame_rate_independent
    // DESC: Checks exact endpoints, monotonicity, and identical samples after skipped frames.
    // ------------------=
    fn elapsed_motion_is_frame_rate_independent() {
        let mut motion = Motion::settled(-100);
        motion.retarget(900, 100, 240, false);
        let mut previous = -100;
        for now in 100..=340 {
            let value = motion.value(now);
            assert!(value >= previous && value <= 900);
            previous = value;
        }
        assert_eq!(motion.value(220), 400);
        assert_eq!(motion.value(1000), 900);
        assert!(!motion.active(340));
    }
    #[test]
    // ------------------------=
    // FUNC: interruption_and_reduced_motion_preserve_control
    // DESC: Reverses mid-flight continuously and immediately settles reduced-motion requests.
    // ------------------=
    fn interruption_and_reduced_motion_preserve_control() {
        let mut motion = Motion::settled(0);
        motion.retarget(1000, 0, 200, false);
        let before = motion.value(70);
        motion.retarget(0, 70, 100, false);
        assert_eq!(motion.value(70), before);
        assert_eq!(motion.value(170), 0);
        motion.retarget(i32::MAX, 200, 300, true);
        assert_eq!(motion.value(200), i32::MAX);
        assert!(!motion.active(200));
        motion.retarget(i32::MIN, 300, 300, false);
        assert_eq!(motion.value(600), i32::MIN);
    }
}
