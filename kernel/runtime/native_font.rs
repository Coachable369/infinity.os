//! Versioned, read-only FontKit glyphs from the installed native font library.
//! No framebuffer access, host font service, allocation or implicit font download.
use super::font_faces;

pub const VERSION: u32 = 1;
pub const MAX_COVERAGE: usize = 96 * 96;
pub const METRIC_BYTES: usize = 40;

// ------------------------=
// FUNC: glyph
// DESC: Rasterizes a supported native glyph with bilinear alpha and matching metrics, rejecting unknown styles and unsupported characters explicitly.
// ------------------=
pub fn glyph(request: [u32; 4], coverage: &mut [u8]) -> Result<[i32; 10], i64> {
    let [version, scalar, height, flags] = request;
    if version != VERSION || !(8..=96).contains(&height) { return Err(-22); }
    if flags & !3 != 0 || !(32..=126).contains(&scalar) { return Err(-88); }
    let height = height as usize;
    let width = (24 * height + 27) / 28;
    let count = width * height;
    if coverage.len() < count { return Err(-34); }
    let (atlas, advances, _) = font_faces::face(if flags & 1 != 0 { 8 } else { 1 }, flags & 2 != 0);
    let offset = (scalar as usize - 32) * 24;
    for y in 0..height {
        // Pixel-center sampling preserves the original atlas exactly at28px.
        let fy = (((2 * y + 1) * 28 * 256 / (2 * height)) as i32 - 128).clamp(0, 27 * 256);
        let sy = fy as usize / 256; let ty = fy as usize % 256;
        for x in 0..width {
            let fx = (((2 * x + 1) * 24 * 256 / (2 * width)) as i32 - 128).clamp(0, 23 * 256);
            let sx = fx as usize / 256; let tx = fx as usize % 256;
            let sample = |row: usize, column: usize| atlas[row * 24 * 95 + offset + column] as usize;
            let upper = sample(sy, sx) * (256 - tx) + sample(sy, (sx + 1).min(23)) * tx;
            let lower = sample((sy + 1).min(27), sx) * (256 - tx) + sample((sy + 1).min(27), (sx + 1).min(23)) * tx;
            coverage[y * width + x] = ((upper * (256 - ty) + lower * ty + 32768) / 65536) as u8;
        }
    }
    // The native pack normalizes glyph ink into a common cell. Its flat-bottom
    // capital H anchors that cell's baseline, including each installed weight.
    let baseline = (0..28).rev().find(|row| (0..24).any(|x|
        atlas[row * 24 * 95 + (b'H' - 32) as usize * 24 + x] >= 128)).unwrap_or(23) + 1;
    let ascent = (baseline * height + 14) / 28;
    let advance = (advances[scalar as usize - 32] as usize * height + 14) / 28;
    Ok([VERSION as i32, width as i32, height as i32, 0, ascent as i32,
        advance as i32, ascent as i32, height.saturating_sub(ascent) as i32,
        (height + (4 * height + 27) / 28) as i32, count as i32])
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: installed_pixels_and_metrics_are_preserved
    // DESC: Checks byte-exact native-size coverage, alpha antialiasing, consistent advance and bounded rejected requests.
    // ------------------=
    #[test]
    fn installed_pixels_and_metrics_are_preserved() {
        let mut coverage = [0; MAX_COVERAGE];
        for flags in 0..4 {
            let (atlas, advances, _) = font_faces::face(if flags & 1 == 0 { 1 } else { 8 }, flags & 2 != 0);
            for scalar in 32..=126 {
                let metrics = glyph([1, scalar, 28, flags], &mut coverage).unwrap();
                assert_eq!(&metrics[1..3], &[24, 28]);
                assert_eq!(metrics[5], advances[scalar as usize - 32] as i32);
                assert_eq!(metrics[6] + metrics[7], 28);
                for y in 0..28 { for x in 0..24 {
                    assert_eq!(coverage[y * 24 + x], atlas[y * 24 * 95 + (scalar as usize - 32) * 24 + x]);
                }}
            }
        }
        for height in [8, 13, 19, 32, 67, 96] {
            let m = glyph([1, b'A' as u32, height, 0], &mut coverage).unwrap();
            assert_eq!(m[9], m[1] * m[2]);
            assert!(coverage[..m[9] as usize].iter().any(|a| *a > 0 && *a < 255));
        }
        for request in [[0,65,28,0], [1,65,97,0], [1,0x2603,28,0], [1,65,28,4]] {
            coverage.fill(93);
            assert!(glyph(request, &mut coverage).is_err());
            assert!(coverage.iter().all(|a| *a == 93));
        }
        assert_eq!(glyph([1,65,96,0], &mut [0;32]), Err(-34));
    }
}
