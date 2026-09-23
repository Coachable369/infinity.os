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
    backdrop::capture_stage(&display);
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
    pixels.fill(0xff90a0b0);
    assert!(backdrop::restore_stage(&mut display));
    assert_eq!(pixels[32], 0xff030609);
    assert_eq!(pixels[31], 0xff90a0b0);
    assert_eq!(pixels[39], 0xff90a0b0);
    assert!(backdrop::restore(&mut display));
    assert_eq!(pixels[32], 0xff102030);
    let scene = [0xff90a0b0; 80];
    pixels.fill(0xff334455);
    assert!(backdrop::compose_spatial(
        &mut display,
        &scene,
        1,
        128,
        false
    ));
    assert_eq!(pixels[32], 0xff102030);
    assert_eq!(pixels[42], 0xff506070);
    assert_eq!(pixels[41], 0xff334455);
    assert_eq!(pixels[49], 0xff334455);
    // Isolated overlays use wallpaper pixels even at the first reveal frame,
    // while closing can still recover every original desktop pixel.
    display.render_clip = None;
    pixels.fill(0xff80a0c0);
    backdrop::capture_clean_stage(&display);
    assert!(backdrop::restore_stage(&mut display));
    let clean = pixels;
    pixels.fill(0xffef0101);
    assert!(backdrop::compose_spatial(&mut display, &scene, 0, 0, true));
    for y in 0..8 {
        for x in 0..8 {
            assert_eq!(pixels[y * 10 + x], clean[y * 10 + x]);
        }
    }
    assert!(backdrop::restore(&mut display));
    assert_eq!(pixels[0], 0xff102030);
    backdrop::invalidate();
    assert!(!backdrop::restore(&mut display));
    assert!(!backdrop::restore_stage(&mut display));
    let unchanged = pixels;
    assert!(!backdrop::compose_spatial(
        &mut display,
        &scene,
        0,
        128,
        false
    ));
    assert_eq!(pixels, unchanged);
    reference_equivalence();
}

// ------------------------=
// FUNC: reference_equivalence
// DESC: Compares native optimized pixels with the original scalar equations across padded, narrow and vignetted surfaces and every fade opacity.
// ------------------=
fn reference_equivalence() {
    for width in [1usize, 8, 17, 37, 257, 513] {
        let height = 31;
        let stride = width + 3;
        let source: Vec<u32> = (0..stride * height)
            .map(|i| (i as u32).wrapping_mul(0x9e3779b9).rotate_left(11))
            .collect();
        let mut pixels = source.clone();
        let mut display = DisplayDevice {
            buffer: pixels.as_mut_ptr(),
            width,
            height,
            stride,
            render_clip: None,
        };
        backdrop::capture(&display);
        backdrop::capture_stage(&display);
        pixels.fill(0xabcddcba);
        assert!(backdrop::restore_stage(&mut display));
        for y in 0..height {
            for x in 0..stride {
                if x >= width {
                    assert_eq!(pixels[y * stride + x], 0xabcddcba);
                    continue;
                }
                let original = source[y * stride + x];
                let mut sums = [0u32; 3];
                for dy in [-8isize, 0, 8] {
                    for dx in [-8isize, 0, 8] {
                        let sx = (x as isize + dx).clamp(0, width as isize - 1) as usize;
                        let sy = (y as isize + dy).clamp(0, height as isize - 1) as usize;
                        for i in 0..3 {
                            sums[i] += (source[sy * stride + sx] >> (i * 8)) & 255;
                        }
                    }
                }
                let edge = if width >= 256 {
                    x.saturating_sub(width * 4 / 100)
                        .min((width * 96 / 100).saturating_sub(x))
                        .min(y.saturating_sub(height * 7 / 100))
                        .min((height * 96 / 100).saturating_sub(y))
                        .saturating_mul(255)
                        / (width / 24).max(1)
                } else {
                    255
                }
                .min(255) as u32;
                let mut expected = original & 0xff000000;
                for i in 0..3 {
                    expected |= ((sums[i] / 45 * edge
                        + ((original >> (i * 8)) & 255) * (255 - edge))
                        / 255)
                        << (i * 8);
                }
                assert_eq!(pixels[y * stride + x], expected, "stage at {width}:{x},{y}");
            }
        }
        let foreground: Vec<u32> = source
            .iter()
            .map(|p| p.rotate_left(9) ^ 0x89abcdef)
            .collect();
        display.render_clip = Some((0, 2, width, height - 4));
        for alpha in 0u32..=255 {
            pixels.copy_from_slice(&foreground);
            backdrop::fade(&mut display, alpha as u8);
            for y in 0..height {
                for x in 0..stride {
                    let offset = y * stride + x;
                    let front = foreground[offset];
                    let mut expected = front;
                    if x < width && (2..height - 2).contains(&y) {
                        expected &= 0xff000000;
                        for shift in [0, 8, 16] {
                            expected |= ((((front >> shift) & 255) * alpha
                                + ((source[offset] >> shift) & 255) * (255 - alpha)
                                + 127)
                                / 255)
                                << shift;
                        }
                    }
                    assert_eq!(pixels[offset], expected, "fade {alpha} at {width}:{x},{y}");
                }
            }
        }
        backdrop::invalidate();
    }
}

#[test]
#[ignore]
// ------------------------=
// FUNC: backdrop_costs_at_desktop_resolution
// DESC: Measures the actual native backdrop stages separately without including allocation or scanout.
// ------------------=
fn backdrop_costs_at_desktop_resolution() {
    let mut pixels = vec![0xff426891u32; 2560 * 1440];
    let mut display = DisplayDevice {
        buffer: pixels.as_mut_ptr(),
        width: 2560,
        height: 1440,
        stride: 2560,
        render_clip: None,
    };
    backdrop::capture(&display);
    for _ in 0..5 {
        let start = std::time::Instant::now();
        backdrop::capture_stage(&display);
        let capture = start.elapsed();
        let start = std::time::Instant::now();
        backdrop::restore(&mut display);
        backdrop::restore_stage(&mut display);
        let restore = start.elapsed();
        let start = std::time::Instant::now();
        backdrop::fade(&mut display, std::hint::black_box(128));
        let fade = start.elapsed();
        std::hint::black_box(&pixels);
        println!(
            "capture_us={} restore_us={} fade_us={}",
            capture.as_micros(),
            restore.as_micros(),
            fade.as_micros()
        );
    }
    backdrop::invalidate();
}
