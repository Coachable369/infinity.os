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

// ------------------------=
// FUNC: settle_position
// DESC: Samples a finite damped-spring keyframe track with one small overshoot and exact rest, independent of frame rate.
// ------------------=
pub fn settle_position(from: i32, to: i32, progress: u8) -> i32 {
    const KEYS: [(i64, i64); 6] = [
        (0, 0),
        (100, 218),
        (160, 273),
        (205, 250),
        (235, 257),
        (255, 255),
    ];
    let t = i64::from(progress);
    let pair = KEYS.windows(2).find(|pair| t <= pair[1].0).unwrap();
    let (a, b) = (pair[0], pair[1]);
    let q = (t - a.0) * 65536 / (b.0 - a.0);
    let eased = q * q * (3 * 65536 - 2 * q) / (65536 * 65536);
    let weight = a.1 + (b.1 - a.1) * eased / 65536;
    (i64::from(from) + (i64::from(to) - i64::from(from)) * weight / 255)
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Motion {
    from: i32,
    to: i32,
    start: u64,
    duration: u32,
}

#[derive(Clone, Copy)]
pub struct DeferredSelection {
    pending: Option<(usize, usize)>,
}
impl DeferredSelection {
    // ------------------------=
    // FUNC: new
    // DESC: Starts without any delayed workspace mutation.
    // ------------------=
    pub const fn new() -> Self {
        Self { pending: None }
    }
    // ------------------------=
    // FUNC: request
    // DESC: Replaces the pending destination without executing it during animation.
    // ------------------=
    pub fn request(&mut self, tab: usize, index: usize) {
        self.pending = Some((tab, index));
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Discards a pending mutation and reports whether cancellation occurred.
    // ------------------=
    pub fn cancel(&mut self) -> bool {
        self.pending.take().is_some()
    }
    // ------------------------=
    // FUNC: finish
    // DESC: Delivers the destination exactly once, only at the closed endpoint.
    // ------------------=
    pub fn finish(&mut self, progress: i32) -> Option<(usize, usize)> {
        if progress == 0 {
            self.pending.take()
        } else {
            None
        }
    }
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
    // FUNC: spring_track_settles_exactly_without_unbounded_motion
    // DESC: Verifies bounded overshoot, reverse travel, endpoint rest, and integer overflow safety.
    // ------------------=
    fn spring_track_settles_exactly_without_unbounded_motion() {
        assert_eq!(settle_position(100, 500, 0), 100);
        assert_eq!(settle_position(100, 500, 255), 500);
        assert!(settle_position(100, 500, 160) > 500);
        for t in 0..=255 {
            assert!((100..=529).contains(&settle_position(100, 500, t)));
            assert!((71..=500).contains(&settle_position(500, 100, t)));
            let _ = settle_position(i32::MIN, i32::MAX, t);
        }
    }
    #[test]
    // ------------------------=
    // FUNC: deferred_selection_is_cancellable_and_exactly_once
    // DESC: Verifies no early mutation, interruption, replacement and single delivery.
    // ------------------=
    fn deferred_selection_is_cancellable_and_exactly_once() {
        let mut selection = DeferredSelection::new();
        selection.request(1, 2);
        assert_eq!(selection.finish(128), None);
        assert!(selection.cancel());
        assert_eq!(selection.finish(0), None);
        selection.request(1, 2);
        selection.request(1, 3);
        assert_eq!(selection.finish(1), None);
        assert_eq!(selection.finish(0), Some((1, 3)));
        assert_eq!(selection.finish(0), None);
    }
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
