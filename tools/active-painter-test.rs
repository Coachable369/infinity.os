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
struct DisplayDevice {
    buffer: *mut u32,
    width: usize,
    height: usize,
    stride: usize,
    format: u32,
    clip: Region,
    submissions: u64,
}
impl DisplayDevice {
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
            left: left.max(self.clip.left),
            top: top.max(self.clip.top),
            right: left
                .saturating_add(width)
                .min(self.width)
                .min(self.clip.right),
            bottom: top
                .saturating_add(height)
                .min(self.height)
                .min(self.clip.bottom),
        };
        (value.left < value.right && value.top < value.bottom).then_some(value)
    }
    // ------------------------=
    // FUNC: render_point_visible
    // DESC: Checks a test framebuffer point against its damage clip.
    // ------------------=
    fn render_point_visible(&self, x: usize, y: usize) -> bool {
        x >= self.clip.left && x < self.clip.right && y >= self.clip.top && y < self.clip.bottom
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
                clip: Region {
                    left: 0,
                    top: 0,
                    right: 2560,
                    bottom: 1600,
                },
                submissions: 0,
            };
            scene(&mut display);
            display.buffer = partial.as_mut_ptr();
            display.clip = Region {
                left: 65,
                top: 65,
                right: 65 + extent,
                bottom: 65 + extent,
            };
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
