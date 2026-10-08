//! Executes the exact native retained-surface cache with a small software framebuffer.
#[path="../kernel/ui/motion.rs"] pub mod motion;
#[path="../kernel/ui/desktop_effects.rs"] pub mod desktop_effects;
#[derive(Clone, Copy)]
pub struct PresentRegion {
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
    recording_surface: bool,
    fast_motion_frame: bool,
    render_clip: Option<PresentRegion>,
}
mod ui {
    pub use crate::desktop_effects;
    pub mod app_launcher {pub use crate::motion;}
    pub mod input_capture {
        // ------------------------=
        // FUNC: poll
        // DESC: Leaves host-test input empty while exercising the native cache implementation.
        // ------------------=
        pub fn poll() {}
    }
}
impl DisplayDevice {
    // ------------------------=
    // FUNC: ui_scale
    // DESC: Selects native one-pixel density for deterministic pixel assertions.
    // ------------------=
    fn ui_scale(&self) -> usize {
        1
    }
    // ------------------------=
    // FUNC: active_icon_theme
    // DESC: Holds the theme constant while testing window identity isolation.
    // ------------------=
    fn active_icon_theme(&self) -> u8 {
        0
    }
    // ------------------------=
    // FUNC: active_background_effects
    // DESC: Disables unrelated background blur in the retained-surface fixture.
    // ------------------=
    fn active_background_effects(&self) -> (u8, u8) {
        (100, 0)
    }
    // ------------------------=
    // FUNC: blur_framebuffer_region
    // DESC: Rejects accidental expensive backdrop work in this opaque cache scenario.
    // ------------------=
    fn blur_framebuffer_region(&mut self, _x: usize, _y: usize, _w: usize, _h: usize, _r: usize) {
        panic!("unexpected blur");
    }
    // ------------------------=
    // FUNC: mark_dirty_rect
    // DESC: Keeps the fixture independent of scanout; exact modified pixels are asserted below.
    // ------------------=
    fn mark_dirty_rect(&mut self, _x: usize, _y: usize, _w: usize, _h: usize) {}
    // ------------------------=
    // FUNC: clipped_render_region
    // DESC: Applies the native half-open clipping contract to the small test framebuffer.
    // ------------------=
    fn clipped_render_region(
        &self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
    ) -> Option<PresentRegion> {
        let clip = self.render_clip.unwrap_or(PresentRegion {
            left: 0,
            top: 0,
            right: self.width,
            bottom: self.height,
        });
        let r = PresentRegion {
            left: x.max(clip.left),
            top: y.max(clip.top),
            right: (x + w).min(clip.right).min(self.width),
            bottom: (y + h).min(clip.bottom).min(self.height),
        };
        (r.left < r.right && r.top < r.bottom).then_some(r)
    }
}
#[path = "../kernel/core/bootstrap/retained_windows.rs"]
mod retained;
#[test]
// ------------------------=
// FUNC: carousel_surface_preserves_alpha_and_clip
// DESC: Keeps transparent cached previews visible and confines scaled composition to damage.
// ------------------=
fn carousel_surface_preserves_alpha_and_clip() {
    let mut pixels = [0u32; 16];
    let mut d = DisplayDevice {
        buffer: pixels.as_mut_ptr(),
        width: 4,
        height: 4,
        stride: 4,
        recording_surface: true,
        fast_motion_frame: true,
        render_clip: Some(PresentRegion {
            left: 1,
            top: 1,
            right: 3,
            bottom: 3,
        }),
    };
    d.spatial_surface(&[0x80800000; 4], 2, 2, (0, 0, 4, 4));
    for y in 0..4 {
        for x in 0..4 {
            assert_eq!(
                pixels[y * 4 + x],
                if (1..3).contains(&x) && (1..3).contains(&y) {
                    0x80800000
                } else {
                    0
                }
            );
        }
    }
    d.spatial_surface(&[0x80800000; 4], 2, 2, (0, 0, 4, 4));
    assert_eq!(pixels[5], 0xbfbf0000);
    d.recording_surface = false;
    d.spatial_surface(&[0xff00ff00; 4], 2, 2, (0, 0, 4, 4));
    assert_eq!(pixels[5], 0x0000ff00);
}
// ------------------------=
// FUNC: paint
// DESC: Supplies real premultiplied content to the native surface recorder.
// ------------------=
fn paint(d: &mut DisplayDevice, color: u32) {
    let r = d.render_clip.unwrap();
    for y in r.top..r.bottom {
        for x in r.left..r.right {
            unsafe {
                *d.buffer.add(y * d.stride + x) = color;
            }
        }
    }
}
#[test]
// ------------------------=
// FUNC: independent_live_surfaces_reuse_contents_and_clip_previews
// DESC: Verifies independent navigator pixels, translation reuse, invalidation, and preview damage isolation.
// ------------------=
fn independent_live_surfaces_reuse_contents_and_clip_previews() {
    let mut pixels = vec![0u32; 240 * 160];
    let mut d = DisplayDevice {
        buffer: pixels.as_mut_ptr(),
        width: 240,
        height: 160,
        stride: 240,
        recording_surface: false,
        fast_motion_frame: false,
        render_clip: None,
    };
    retained::invalidate();
    d.retained_window(6, (24, 24, 96, 64), |d| paint(d, 0xffff0000));
    d.retained_window(7, (24, 24, 96, 64), |d| paint(d, 0xff0000ff));
    d.retained_window(6, (48, 24, 96, 64), |_| {
        panic!("translation rerendered content")
    });
    pixels.fill(0x00112233);
    d.render_clip = Some(PresentRegion {
        left: 20,
        top: 20,
        right: 80,
        bottom: 70,
    });
    d.spatial_preview(6, (20, 20, 60, 50));
    assert_eq!(pixels[40 * 240 + 40], 0x00ff0000);
    assert_eq!(pixels[40 * 240 + 19], 0x00112233);
    assert_eq!(pixels[70 * 240 + 40], 0x00112233);
    d.spatial_preview(7, (20, 20, 60, 50));
    assert_eq!(pixels[40 * 240 + 40], 0x000000ff);
    // A peek reuses the same production surface, leaves other layers opaque,
    // and restores exact pixels without asking the app to paint again.
    desktop_effects::mutate(|effects| effects.peek(0,0,true));
    desktop_effects::advance(0,Some(0),true);
    desktop_effects::layer(Some(0));
    pixels.fill(0x00112233);
    d.render_clip=None;
    d.retained_window(6,(48,24,96,64),|_|panic!("peek rerendered app"));
    assert_eq!(pixels[40*240+60],0x00112233);
    desktop_effects::layer(Some(1));
    d.retained_window(7,(24,24,96,64),|_|panic!("peek rerendered background app"));
    assert_eq!(pixels[40*240+60],0x000000ff);
    desktop_effects::mutate(|effects| {effects.dismiss_peek();});
    desktop_effects::advance(1,Some(0),true);
    desktop_effects::layer(Some(0));
    d.retained_window(6,(48,24,96,64),|_|panic!("restoration rerendered app"));
    assert_eq!(pixels[40*240+60],0x00ff0000);
    desktop_effects::reset();
    d.render_clip = None;
    let mut revision = Some(1);
    assert!(retained::invalidate_revision(6, &mut revision, Some(2)));
    // Keep the last committed preview until its invalidated surface is refreshed.
    d.spatial_preview(6, (20, 20, 60, 50));
    assert_eq!(pixels[40 * 240 + 40], 0x00ff0000);
    d.retained_window(6, (48, 24, 96, 64), |d| paint(d, 0xff00ff00));
    d.spatial_preview(6, (20, 20, 60, 50));
    assert_eq!(pixels[40 * 240 + 40], 0x0000ff00);
    d.spatial_preview(7, (20, 20, 60, 50));
    assert_eq!(pixels[40 * 240 + 40], 0x000000ff);
}
