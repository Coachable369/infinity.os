//! Allocation-free geometry and light-front timing for the bootstrap emblem.

// ------------------------=
// FUNC: emblem_geometry
// DESC: Preserves ISO placement while moving only the installed loading emblem down 100 physical framebuffer pixels.
// ------------------=
pub(crate) fn emblem_geometry(
    screen_width: usize,
    screen_height: usize,
    source_width: usize,
    source_height: usize,
    installer: bool,
) -> (usize, usize, usize, usize) {
    let width = (screen_width * 52 / 100)
        .min((screen_height * 52 / 100) * source_width / source_height)
        .max(1);
    let height = (width * source_height / source_width).max(1);
    (
        screen_width.saturating_sub(width) / 2,
        screen_height * 16 / 100 + if installer { 0 } else { 100 },
        width,
        height,
    )
}

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
    // FUNC: installed_emblem_moves_exactly_one_hundred_physical_pixels
    // DESC: Checks ISO geometry and the installed-only translation across compact, scaled and wide framebuffers.
    // ------------------=
    #[test]
    fn installed_emblem_moves_exactly_one_hundred_physical_pixels() {
        for (width, height) in [(1024, 768), (1280, 720), (2560, 1440), (3440, 1440)] {
            let iso = emblem_geometry(width, height, 1672, 941, true);
            let installed = emblem_geometry(width, height, 1672, 941, false);
            assert_eq!(iso.1, height * 16 / 100);
            assert_eq!(installed, (iso.0, iso.1 + 100, iso.2, iso.3));
            assert_eq!(iso.2, (width * 52 / 100).min((height * 52 / 100) * 1672 / 941));
            assert_eq!(iso.3, iso.2 * 941 / 1672);
        }
    }

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
