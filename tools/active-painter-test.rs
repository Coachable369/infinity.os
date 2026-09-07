#![allow(dead_code)]
#[cfg(not(baseline))]
#[path = "../kernel/core/bootstrap/primitives.rs"]
mod primitives;
#[cfg(baseline)]
#[path = "../build/primitives-before.rs"]
mod primitives;
#[path = "../kernel/ui/mod.rs"]
mod ui;
use std::time::Instant;

#[derive(Clone, Copy)]
struct Region {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
}
#[derive(Clone, Copy)]
struct DisplayDevice {
    buffer: *mut u32,
    width: usize,
    height: usize,
    stride: usize,
    format: u32,
    render_clip: Option<Region>,
    fast_motion_frame: bool,
    submissions: u64,
    recording_surface: bool,
}
type PresentRegion = Region;
#[path = "../kernel/core/bootstrap/retained_windows.rs"]
mod retained_windows;
#[path = "../kernel/core/bootstrap/window_chrome.rs"]
mod window_chrome;
impl DisplayDevice {
    // ------------------------=
    // FUNC: active_background_effects
    // DESC: Selects no backdrop blur for deterministic surface pixel fixtures.
    // ------------------=
    fn active_background_effects(&self) -> (u8, u8) {
        (100, 0)
    }

    // ------------------------=
    // FUNC: clipped_render_region
    // DESC: Supplies bounded host framebuffer geometry to the actual kernel painter.
    // ------------------=
    fn clipped_render_region(
        &self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) -> Option<Region> {
        let value = Region {
            left: left.max(self.render_clip.map_or(0, |c| c.left)),
            top: top.max(self.render_clip.map_or(0, |c| c.top)),
            right: left
                .saturating_add(width)
                .min(self.width)
                .min(self.render_clip.map_or(self.width, |c| c.right)),
            bottom: top
                .saturating_add(height)
                .min(self.height)
                .min(self.render_clip.map_or(self.height, |c| c.bottom)),
        };
        (value.left < value.right && value.top < value.bottom).then_some(value)
    }
    // ------------------------=
    // FUNC: render_point_visible
    // DESC: Checks a test framebuffer point against its damage clip.
    // ------------------=
    fn render_point_visible(&self, x: usize, y: usize) -> bool {
        x >= self.render_clip.map_or(0, |c| c.left)
            && x < self.render_clip.map_or(self.width, |c| c.right)
            && y >= self.render_clip.map_or(0, |c| c.top)
            && y < self.render_clip.map_or(self.height, |c| c.bottom)
    }
    // ------------------------=
    // FUNC: mark_dirty_rect
    // DESC: Counts actual painter damage submissions without emulating the production damage tracker.
    // ------------------=
    fn mark_dirty_rect(&mut self, _: usize, _: usize, _: usize, _: usize) {
        self.submissions += 1;
    }
    // ------------------------=
    // FUNC: installer_text
    // DESC: Rejects installer-only typography, which is outside this primitive fixture.
    // ------------------=
    fn installer_text(&mut self, _: usize, _: usize, _: &[u8], _: u8, _: u8, _: u8) {
        panic!("installer typography not in fixture");
    }
}

// ------------------------=
// FUNC: scene
// DESC: Exercises the actual glass and bitmap kernels with a full-sized panel and local damage.
// ------------------=
fn scene(display: &mut DisplayDevice) {
    display.paint_bitmap_cover_box(
        include_bytes!("../assets/desktop/infinity-default-dark-wallpaper-v2.bmp"),
        0,
        0,
        2560,
        1600,
    );
    display.fill_rect(50, 50, 2200, 1400, 2, 12, 24);
    display.fill_rounded_rect_alpha(60, 60, 2100, 1300, 18, 4, 30, 55, 173);
    display.outline_rounded_rect(60, 60, 2100, 1300, 18, 100, 180, 230);
}

// ------------------------=
// FUNC: main
// DESC: Verifies clipped painting against full-render pixels and reports actual painter cost and damage calls.
// ------------------=
fn main() {
    window_controls_test();
    retained_window_benchmark();
    damage_test();
    retained_surface_test();
    let mut full = vec![0x102030u32; 2560 * 1600];
    let mut partial = full.clone();
    for format in [0, 1] {
        for (extent, label) in [(32, "local"), (900, "window")] {
            full.fill(0x102030);
            partial.fill(0x102030);
            let mut display = DisplayDevice {
                buffer: full.as_mut_ptr(),
                width: 2560,
                height: 1600,
                stride: 2560,
                format,
                render_clip: Some(Region {
                    left: 0,
                    top: 0,
                    right: 2560,
                    bottom: 1600,
                }),
                fast_motion_frame: false,
                submissions: 0,
                recording_surface: false,
            };
            scene(&mut display);
            display.buffer = partial.as_mut_ptr();
            display.render_clip = Some(Region {
                left: 65,
                top: 65,
                right: 65 + extent,
                bottom: 65 + extent,
            });
            display.submissions = 0;
            let mut times = [0u128; 20];
            for time in &mut times {
                let start = Instant::now();
                scene(&mut display);
                *time = start.elapsed().as_nanos();
            }
            for y in 0..1600 {
                for x in 0..2560 {
                    assert_eq!(
                        partial[y * 2560 + x],
                        if display.render_point_visible(x, y) {
                            full[y * 2560 + x]
                        } else {
                            0x102030
                        },
                        "{x},{y}"
                    );
                }
            }
            times.sort_unstable();
            println!("{{\"fixture\":\"active_painter_{label}\",\"format\":{},\"average_ns\":{},\"p95_ns\":{},\"damage_submissions\":{}}}",format,times.iter().sum::<u128>()/20,times[18],display.submissions);
        }
    }
}

// ------------------------=
// FUNC: window_controls_test
// DESC: Verifies glyphs remain visible inside retained RGBA controls at every supported UI scale.
// ------------------=
fn window_controls_test() {
    for scale in 1..=3 {
        for index in 0..3 {
            for maximized in [false, true] {
                let size = 20 * scale;
                let mut pixels = vec![0u32; 128 * 128];
                let mut display = DisplayDevice {
                    buffer: pixels.as_mut_ptr(),
                    width: 128,
                    height: 128,
                    stride: 128,
                    format: 0,
                    render_clip: None,
                    fast_motion_frame: false,
                    submissions: 0,
                    recording_surface: true,
                };
                display.window_control(8, 8, size, index, maximized);
                let mut glyph_pixels = 0;
                for y in 8 + size / 4..8 + size * 3 / 4 + 1 {
                    for x in 8 + size / 4..8 + size * 3 / 4 + 1 {
                        let pixel = pixels[y * 128 + x];
                        if pixel & 255 > 200 && pixel >> 24 == 255 {
                            glyph_pixels += 1;
                        }
                    }
                }
                assert!(glyph_pixels >= 8 * scale);
                assert_eq!(pixels[0], 0);
            }
        }
    }
    let mut pixels = vec![0x001a0b05u32; 256 * 96];
    let mut display = DisplayDevice {
        buffer: pixels.as_mut_ptr(),
        width: 256,
        height: 96,
        stride: 256,
        format: 0,
        render_clip: None,
        fast_motion_frame: false,
        submissions: 0,
        recording_surface: false,
    };
    for (slot, (index, maximized)) in [(0, false), (1, false), (1, true), (2, false)]
        .iter()
        .enumerate()
    {
        display.window_control(16 + slot * 60, 28, 40, *index, *maximized);
    }
    let mut ppm = b"P6\n256 96\n255\n".to_vec();
    for pixel in pixels {
        ppm.extend_from_slice(&[pixel as u8, (pixel >> 8) as u8, (pixel >> 16) as u8]);
    }
    std::fs::write("build/window-controls-proof.ppm", ppm).unwrap();
}

// ------------------------=
// FUNC: retained_window_benchmark
// DESC: Measures the actual native cached-window composition path and asserts no application painting during translations.
// ------------------=
fn retained_window_benchmark() {
    retained_windows::invalidate();
    let mut pixels = vec![0x102030u32; 2560 * 1600];
    let mut display = DisplayDevice {
        buffer: pixels.as_mut_ptr(),
        width: 2560,
        height: 1600,
        stride: 2560,
        format: 0,
        render_clip: None,
        fast_motion_frame: true,
        submissions: 0,
        recording_surface: false,
    };
    let mut painted = 0;
    display.retained_window(1, (80, 80, 1280, 900), |target| {
        painted += 1;
        scene(target);
    });
    let mut times = [0u128; 100];
    for (index, elapsed) in times.iter_mut().enumerate() {
        let start = Instant::now();
        display.retained_window(1, (80 + index, 80, 1280, 900), |_| {
            painted += 1;
        });
        *elapsed = start.elapsed().as_nanos();
    }
    assert_eq!(painted, 1);
    times.sort_unstable();
    println!("{{\"fixture\":\"native_cached_window\",\"frames\":100,\"application_paints\":{},\"average_ns\":{},\"p95_ns\":{}}}",
        painted, times.iter().sum::<u128>() / 100, times[94]);
}

// ------------------------=
// FUNC: damage_test
// DESC: Verifies real presentation damage covers sparse input without full-screen overflow collapse.
// ------------------=
fn damage_test() {
    use ui::present_damage::{insert, Region};
    let mut regions = [Region::default(); 8];
    let mut count = 0;
    for index in 0..9 {
        let x = 10 + index * 250;
        let y = 10 + index * 150;
        insert(
            &mut regions,
            &mut count,
            Region {
                left: x,
                top: y,
                right: x + 8,
                bottom: y + 8,
            },
        );
    }
    assert_eq!(count, 8);
    assert!(regions[..count].iter().map(|r| r.area()).sum::<usize>() < 2560 * 1600 / 20);
    for index in 0..9 {
        let x = 10 + index * 250;
        let y = 10 + index * 150;
        assert!(regions[..count].iter().any(|r| r.contains(Region {
            left: x,
            top: y,
            right: x + 8,
            bottom: y + 8
        })));
    }
    for x in 100..200 {
        count = 0;
        for left in [x, x + 3] {
            insert(
                &mut regions,
                &mut count,
                Region {
                    left,
                    top: 100,
                    right: left + 56,
                    bottom: 156,
                },
            );
        }
        assert!(regions[..count].iter().map(|r| r.area()).sum::<usize>() <= 59 * 56);
    }
}

// ------------------------=
// FUNC: retained_surface_test
// DESC: Verifies translation reuses pixels, preserves transparency on changed backdrops, and invalidation repaints once.
// ------------------=
fn retained_surface_test() {
    retained_windows::invalidate();
    let mut pixels = vec![0x302010u32; 320 * 200];
    let mut display = DisplayDevice {
        buffer: pixels.as_mut_ptr(),
        width: 320,
        height: 200,
        stride: 320,
        format: 0,
        render_clip: None,
        fast_motion_frame: false,
        submissions: 0,
        recording_surface: false,
    };
    let mut painted = 0;
    display.retained_window(0, (40, 40, 40, 40), |target| {
        painted += 1;
        target.fill_rect_alpha(40, 40, 40, 40, 80, 100, 120, 128);
        target.fill_rect(50, 50, 10, 10, 1, 2, 3);
    });
    assert_eq!(painted, 1);
    assert_eq!(pixels[50 * 320 + 50], 0x030201);
    pixels.fill(0x908070);
    display.retained_window(0, (90, 60, 40, 40), |_| {
        painted += 1;
    });
    assert_eq!(
        painted, 1,
        "Translation must never call the application painter"
    );
    assert_eq!(pixels[70 * 320 + 100], 0x030201);
    assert_eq!(pixels[40 * 320 + 40], 0x908070);
    let expected = retained_windows::over(128 << 24 | 60 << 16 | 50 << 8 | 40, 0x908070);
    assert_eq!(pixels[60 * 320 + 90], expected);
    display.render_clip = Some(Region {
        left: 100,
        top: 70,
        right: 102,
        bottom: 72,
    });
    pixels.fill(0xabcdef);
    display.retained_window(0, (90, 60, 40, 40), |_| {
        panic!("valid cache must be reused")
    });
    for y in 0..200 {
        for x in 0..320 {
            assert_eq!(
                pixels[y * 320 + x],
                if (100..102).contains(&x) && (70..72).contains(&y) {
                    0x030201
                } else {
                    0xabcdef
                }
            );
        }
    }
    retained_windows::invalidate();
    display.retained_window(0, (90, 60, 40, 40), |_| {
        painted += 1;
    });
    assert_eq!(painted, 2);
}
