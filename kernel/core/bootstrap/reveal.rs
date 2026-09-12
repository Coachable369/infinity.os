//! Allocation-free light-front timing for the bootstrap emblem.

// ------------------------=
// FUNC: opacity
// DESC: Returns a smooth, feathered center-out exposure with an eased start and finish.
// ------------------=
pub(crate) fn opacity(x: usize, width: usize, frame: usize, frames: usize) -> u8 {
    let t = ((frame.min(frames) as u64 * 1024) / frames.max(1) as u64) as i64;
    let eased = t * t * (3072 - 2 * t) / (1024 * 1024);
    let distance = (2 * x as i64 + 1 - width as i64).abs() * 1024 / width.max(1) as i64;
    let exposure = ((eased * 1280 / 1024 - distance) * 1024 / 256).clamp(0, 1024);
    (exposure * exposure * (3072 - 2 * exposure) * 255 / (1024 * 1024 * 1024)) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: reveal_is_symmetric_monotone_and_complete
    // DESC: Checks all sampled pixels through the entire exposure sequence.
    // ------------------=
    #[test]
    fn reveal_is_symmetric_monotone_and_complete() {
        for width in [1, 127, 128, 1920] {
            for x in 0..width {
                assert_eq!(opacity(x, width, 0, 120), 0);
                assert_eq!(opacity(x, width, 120, 120), 255);
                let mut previous = 0;
                for frame in 0..=120 {
                    let current = opacity(x, width, frame, 120);
                    assert!(current >= previous);
                    assert_eq!(current, opacity(width - 1 - x, width, frame, 120));
                    previous = current;
                }
            }
        }
    }
}
