//! Allocation-free subpixel coverage for narrow luminous strokes.

// ------------------------=
// FUNC: segment
// DESC: Emits a half-open antialiased segment in Q8 coordinates, with a bright core and a smooth bounded halo.
// ------------------=
pub(super) fn segment(
    from: (i32, i32),
    to: (i32, i32),
    scale: i32,
    mut pixel: impl FnMut(i32, i32, u8),
) {
    let steep = (to.1 - from.1).abs() > (to.0 - from.0).abs();
    let (mut a, mut b) = if steep {
        ((from.1, from.0), (to.1, to.0))
    } else {
        (from, to)
    };
    if a.0 > b.0 {
        core::mem::swap(&mut a, &mut b);
    }
    let extent = i64::from(b.0) - i64::from(a.0);
    if extent == 0 {
        return;
    }
    let scale = scale.clamp(1, 3);
    for major in (a.0 + 255).div_euclid(256)..(b.0 + 255).div_euclid(256) {
        let minor = i64::from(a.1)
            + (i64::from(major) * 256 - i64::from(a.0)) * i64::from(b.1 - a.1) / extent;
        let center = (minor as i32).div_euclid(256);
        for offset in -4 * scale..=4 * scale {
            let row = center + offset;
            let distance = (i64::from(row) * 256 - minor).unsigned_abs() as i32 / scale;
            let alpha = if distance < 192 {
                235 - distance * 95 / 192
            } else if distance < 448 {
                140 - (distance - 192) * 102 / 256
            } else if distance < 1024 {
                let rest = 1024 - distance;
                38 * rest * rest / (576 * 576)
            } else {
                0
            };
            if alpha > 0 {
                let (x, y) = if steep { (row, major) } else { (major, row) };
                pixel(x, y, alpha as u8);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: coverage_is_subpixel_bounded_and_join_safe
    // DESC: Checks fractional coverage, reverse-direction equivalence and nonduplicated adjacent segment pixels.
    // ------------------=
    fn coverage_is_subpixel_bounded_and_join_safe() {
        let mut forward = std::collections::BTreeMap::new();
        segment((0, 128), (2560, 128), 1, |x, y, a| {
            assert!(x >= 0 && x < 10 && y.abs() <= 4);
            forward.insert((x, y), a);
        });
        assert_eq!(forward[&(5, 0)], forward[&(5, 1)]);
        assert!(forward[&(5, 0)] > forward[&(5, 2)]);
        let mut reverse = std::collections::BTreeMap::new();
        segment((2560, 128), (0, 128), 1, |x, y, a| {
            reverse.insert((x, y), a);
        });
        assert_eq!(forward, reverse);
        let mut split = std::collections::BTreeMap::new();
        for (a, b) in [((0, 128), (1280, 128)), ((1280, 128), (2560, 128))] {
            segment(a, b, 1, |x, y, c| {
                assert!(split.insert((x, y), c).is_none());
            });
        }
        assert_eq!(forward, split);
    }
}
