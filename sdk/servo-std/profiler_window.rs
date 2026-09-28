// ------------------------=
// FUNC: recent_start
// DESC: Bounds the profiler history window at the monotonic clock's origin.
// ------------------=
pub fn recent_start(now: u64, window: u64) -> u64 {
    now.saturating_sub(window)
}

#[cfg(test)]
mod tests {
    // ------------------------=
    // FUNC: first_frames_and_later_history
    // DESC: Covers native fast startup, the window boundary, and subsequent frames.
    // ------------------=
    #[test]
    fn first_frames_and_later_history() {
        let second = 1_000_000_000;
        for now in [0, 1, 16_666_667, second - 1, second] {
            assert_eq!(super::recent_start(now, second), 0);
        }
        assert_eq!(super::recent_start(second + 1, second), 1);
        assert_eq!(super::recent_start(2 * second, second), second);
        assert_eq!(super::recent_start(u64::MAX, second), u64::MAX - second);
    }
}
