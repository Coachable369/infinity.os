//! Shared geometry and allocation-free sine samples for the existing glass header.
use crate::ui::geometry::Rect;
// ------------------------=
// FUNC: control
// DESC: Keeps voice status clear of the title and the existing window controls at every widget width.
// ------------------=
pub fn control(panel: Rect, scale: usize) -> Rect {
    let width = (146 * scale).min((panel.width as usize).saturating_sub(170 * scale));
    Rect {
        x: panel.x + (panel.width as usize).saturating_sub(66 * scale + width) as i32,
        y: panel.y + (8 * scale) as i32,
        width: width as u32,
        height: (30 * scale) as u32,
    }
}
// ------------------------=
// FUNC: sample
// DESC: Produces a bounded audio-reactive sine displacement with no rasterization or per-frame allocation.
// ------------------=
pub fn sample(x: usize, frame: usize, peak: u16) -> i32 {
    const SINE: [i32; 32] = [
        0, 25, 49, 71, 90, 106, 117, 125, 127, 125, 117, 106, 90, 71, 49, 25, 0, -25, -49, -71,
        -90, -106, -117, -125, -127, -125, -117, -106, -90, -71, -49, -25,
    ];
    let amplitude = 1 + (peak as i32).min(3000) * 6 / 3000;
    SINE[(x + frame) % 32] * amplitude / 127
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: bounded_geometry_and_motion
    // DESC: Verifies title/control gutters, sine periodicity, amplitude bounds, and actual audio response.
    // ------------------=
    fn bounded_geometry_and_motion() {
        for scale in 1..4 {
            for width in [280, 320, 400] {
                let p = Rect {
                    x: 15,
                    y: 20,
                    width: (width * scale) as u32,
                    height: 600,
                };
                let r = control(p, scale);
                assert!(r.x >= p.x + (104 * scale) as i32);
                assert_eq!(
                    r.x + r.width as i32,
                    p.x + p.width as i32 - (66 * scale) as i32
                );
            }
        }
        for x in 0..100 {
            assert!(sample(x, 0, u16::MAX).abs() <= 7);
            assert_eq!(sample(x, 0, 0), sample(x, 32, 0));
        }
        assert_eq!(sample(8, 0, 0), 1);
        assert_eq!(sample(8, 0, 3000), 7);
    }
}
