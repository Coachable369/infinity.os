//! Bounded page-layout zoom shared by native engine sessions.
pub const LEVELS: [u16; 13] = [50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200, 250, 300];

// ------------------------=
// FUNC: next
// DESC: Moves one standard zoom step, clamps at either end, or resets to actual size.
// ------------------=
pub fn next(current: u16, direction: i32) -> u16 {
    if direction > 0 { LEVELS.iter().copied().find(|&v| v > current).unwrap_or(300) }
    else if direction < 0 { LEVELS.iter().rev().copied().find(|&v| v < current).unwrap_or(50) }
    else { 100 }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: zoom_steps_bound_and_reset
    // DESC: Exercises every step in both directions and repeated boundary operations.
    // ------------------=
    #[test]
    fn zoom_steps_bound_and_reset() {
        for pair in LEVELS.windows(2) {
            assert_eq!(next(pair[0], 1), pair[1]);
            assert_eq!(next(pair[1], -1), pair[0]);
        }
        assert_eq!(next(50,-1),50);
        assert_eq!(next(300,1),300);
        for level in LEVELS { assert_eq!(next(level,0),100); }
    }
}
