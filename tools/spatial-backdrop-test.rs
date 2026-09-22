//! Pixel verification of the exact native overlay backdrop implementation.
struct Region {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
}
struct DisplayDevice {
    buffer: *mut u32,
    width: usize,
    height: usize,
    stride: usize,
    render_clip: Option<(usize, usize, usize, usize)>,
}
impl DisplayDevice {
    // ------------------------=
    // FUNC: clipped_render_region
    // DESC: Supplies the same bounded clipping contract as the framebuffer facade.
    // ------------------=
    fn clipped_render_region(
        &self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) -> Option<Region> {
        let (a, b, w, h) = self.render_clip.unwrap_or((0, 0, self.width, self.height));
        let r = Region {
            left: left.max(a),
            top: top.max(b),
            right: (left + width).min(a + w).min(self.width),
            bottom: (top + height).min(b + h).min(self.height),
        };
        (r.left < r.right && r.top < r.bottom).then_some(r)
    }
}
#[path = "../kernel/core/bootstrap/launcher_backdrop.rs"]
mod backdrop;
#[test]
// ------------------------=
// FUNC: frozen_backdrop_fade_respects_damage_and_endpoints
// DESC: Asserts exact pixel results, unchanged gutters, persistent capture and safe invalidation.
// ------------------=
fn frozen_backdrop_fade_respects_damage_and_endpoints() {
    let mut pixels = [0xff102030u32; 80];
    let mut display = DisplayDevice {
        buffer: pixels.as_mut_ptr(),
        width: 8,
        height: 8,
        stride: 10,
        render_clip: None,
    };
    backdrop::capture(&display);
    pixels.fill(0xff90a0b0);
    display.render_clip = Some((2, 3, 3, 2));
    backdrop::fade(&mut display, 0);
    for y in 0..8 {
        for x in 0..10 {
            assert_eq!(
                pixels[y * 10 + x],
                if (2..5).contains(&x) && (3..5).contains(&y) {
                    0xff102030
                } else {
                    0xff90a0b0
                }
            );
        }
    }
    pixels.fill(0xff90a0b0);
    backdrop::fade(&mut display, 128);
    assert_eq!(pixels[32], 0xff506070);
    assert_eq!(pixels[31], 0xff90a0b0);
    pixels.fill(0xff90a0b0);
    backdrop::fade(&mut display, 255);
    assert_eq!(pixels, [0xff90a0b0; 80]);
    assert!(backdrop::restore(&mut display));
    assert_eq!(pixels[32], 0xff102030);
    backdrop::invalidate();
    assert!(!backdrop::restore(&mut display));
}
