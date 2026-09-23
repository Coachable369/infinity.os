//! Bounded translation of a retained spatial scene; no layout or rasterization.
// ------------------------=
// FUNC: compose
// DESC: Translates and fades a retained scene in one pass, preserving clipped borders and stride padding.
// ------------------=
pub fn compose(
    source: &[u32],
    backdrop: &[u32],
    destination: &mut [u32],
    stride: usize,
    bounds: (usize, usize, usize, usize),
    lift: usize,
    opacity: u8,
) -> bool {
    let (left, top, right, bottom) = bounds;
    if stride == 0
        || left >= right
        || right > stride
        || top >= bottom
        || bottom
            .checked_mul(stride)
            .is_none_or(|n| n > source.len() || n > backdrop.len() || n > destination.len())
    {
        return false;
    }
    let split = top.saturating_add(lift).min(bottom);
    let a = u32::from(opacity);
    let b = 255 - a;
    for y in top..bottom {
        let start = y * stride + left;
        let end = y * stride + right;
        let output = &mut destination[start..end];
        let behind = &backdrop[start..end];
        if y < split {
            output.copy_from_slice(behind);
        } else {
            let offset = (y - lift) * stride + left;
            let foreground = &source[offset..offset + right - left];
            if opacity == 255 {
                output.copy_from_slice(foreground);
            } else {
                for ((out, &fg), &bg) in output.iter_mut().zip(foreground).zip(behind) {
                    let lanes = (fg & 0x00ff00ff) * a + (bg & 0x00ff00ff) * b + 0x007f007f;
                    let rb = ((lanes + 0x00010001 + ((lanes >> 8) & 0x00ff00ff)) >> 8) & 0x00ff00ff;
                    let green = (((fg >> 8) & 255) * a + ((bg >> 8) & 255) * b + 127) / 255;
                    *out = (fg & 0xff000000) | rb | (green << 8);
                }
            }
        }
    }
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: fused_composition_matches_reference
    // DESC: Checks every opacity and lift against independent channel arithmetic, including clipping and malformed buffers.
    // ------------------=
    fn fused_composition_matches_reference() {
        let source: Vec<u32> = (0u32..120).map(|i| i.wrapping_mul(0x17327919)).collect();
        let backdrop: Vec<u32> = source.iter().map(|p| p ^ 0xffffffff).collect();
        for opacity in 0..=255u8 {
            for lift in 0..12 {
                let mut output = vec![999; 120];
                assert!(compose(
                    &source,
                    &backdrop,
                    &mut output,
                    12,
                    (2, 2, 9, 9),
                    lift,
                    opacity
                ));
                for y in 0..10 {
                    for x in 0..12 {
                        let i = y * 12 + x;
                        let mut expected = 999;
                        if (2..9).contains(&x) && (2..9).contains(&y) {
                            let bg = backdrop[i];
                            let fg = if y >= 2 + lift {
                                source[(y - lift) * 12 + x]
                            } else {
                                bg
                            };
                            expected = fg & 0xff000000;
                            for shift in [0, 8, 16] {
                                expected |= ((((fg >> shift) & 255) * u32::from(opacity)
                                    + ((bg >> shift) & 255) * (255 - u32::from(opacity))
                                    + 127)
                                    / 255)
                                    << shift;
                            }
                        }
                        assert_eq!(output[i], expected);
                    }
                }
            }
        }
        let mut output = vec![999; 120];
        for bounds in [(0, 0, 13, 9), (0, 0, 12, 11), (4, 0, 2, 5)] {
            assert!(!compose(
                &source,
                &backdrop,
                &mut output,
                12,
                bounds,
                0,
                128
            ));
        }
        assert_eq!(output, vec![999; 120]);
    }
}
