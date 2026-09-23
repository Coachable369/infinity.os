//! Bounded translation of a retained spatial scene; no layout or rasterization.
// ------------------------=
// FUNC: translate
// DESC: Copies clipped scene rows at a vertical offset while leaving exposed backdrop and stride padding untouched.
// ------------------=
pub fn translate(
    source: &[u32],
    destination: &mut [u32],
    stride: usize,
    bounds: (usize, usize, usize, usize),
    lift: usize,
) -> bool {
    let (left, top, right, bottom) = bounds;
    if stride == 0
        || left >= right
        || right > stride
        || top >= bottom
        || bottom
            .checked_mul(stride)
            .is_none_or(|n| n > source.len() || n > destination.len())
    {
        return false;
    }
    for y in top.saturating_add(lift)..bottom {
        let dst = y * stride + left;
        let src = (y - lift) * stride + left;
        destination[dst..dst + right - left].copy_from_slice(&source[src..src + right - left]);
    }
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: translation_preserves_exposed_pixels_and_padding
    // DESC: Verifies all transition offsets, endpoint identity and rejected malformed bounds.
    // ------------------=
    fn translation_preserves_exposed_pixels_and_padding() {
        let source: Vec<u32> = (0..120).collect();
        for lift in 0..12 {
            let mut destination = vec![999; 120];
            assert!(translate(&source, &mut destination, 12, (2, 2, 9, 9), lift));
            for y in 0..10 {
                for x in 0..12 {
                    assert_eq!(
                        destination[y * 12 + x],
                        if (2..9).contains(&x) && y >= 2 + lift && y < 9 {
                            source[(y - lift) * 12 + x]
                        } else {
                            999
                        }
                    );
                }
            }
        }
        let mut destination = vec![999; 120];
        assert!(!translate(&source, &mut destination, 12, (0, 0, 13, 9), 0));
        assert!(!translate(&source, &mut destination, 12, (0, 0, 12, 11), 0));
        assert_eq!(destination, vec![999; 120]);
    }
}
