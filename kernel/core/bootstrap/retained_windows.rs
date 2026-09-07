//! Native desktop-owned premultiplied window surfaces. Applications never receive
//! either the presentation buffer or the physical framebuffer pointer.
use super::{DisplayDevice, PresentRegion};
const PIXELS: usize = 2560 * 1600;
const SCRATCH_PIXELS: usize = 3840 * 2160;
const SLOTS: usize = 5;

#[derive(Clone, Copy)]
struct CachedWindow {
    valid: bool,
    width: usize,
    height: usize,
    inset: (usize, usize),
    pixels: [u32; PIXELS],
}
static mut WINDOWS: [CachedWindow; SLOTS] = [CachedWindow {
    valid: false,
    width: 0,
    height: 0,
    inset: (0, 0),
    pixels: [0; PIXELS],
}; SLOTS];
static mut SCRATCH: [u32; SCRATCH_PIXELS] = [0; SCRATCH_PIXELS];

// ------------------------=
// FUNC: invalidate
// DESC: Invalidates native surfaces after content or appearance changes, not translations.
// ------------------=
pub(super) fn invalidate() {
    unsafe {
        let windows = &mut *(&raw mut WINDOWS);
        for window in windows {
            window.valid = false;
        }
    }
}

// ------------------------=
// FUNC: over
// DESC: Composes a cached premultiplied pixel over an opaque destination in either channel order.
// ------------------=
pub(super) fn over(source: u32, destination: u32) -> u32 {
    let inverse = 255 - (source >> 24);
    let channel = |shift: u32| {
        (((source >> shift) & 255u32) + ((destination >> shift) & 255) * inverse / 255).min(255)
    };
    channel(0) | channel(8) << 8 | channel(16) << 16
}

impl DisplayDevice {
    // ------------------------=
    // FUNC: retained_window
    // DESC: Rasterizes a changed window once into a private surface, then clips and composites cached pixels.
    // ------------------=
    pub(super) fn retained_window(
        &mut self,
        slot: usize,
        bounds: (usize, usize, usize, usize),
        paint: impl FnOnce(&mut Self),
    ) {
        if self.recording_surface || slot >= SLOTS || self.stride * self.height > SCRATCH_PIXELS {
            paint(self);
            return;
        }
        let (left, top, width, height) = bounds;
        let padding = 16 * self.ui_scale();
        let inset = (left.min(padding), top.min(padding));
        let left = left.saturating_sub(padding);
        let top = top.saturating_sub(padding);
        let width = width
            .saturating_add(padding * 2)
            .min(self.width.saturating_sub(left));
        let height = height
            .saturating_add(padding * 2)
            .min(self.height.saturating_sub(top));
        if width == 0 || height == 0 {
            return;
        }
        if width.saturating_mul(height) > PIXELS {
            paint(self);
            return;
        }
        let Some(region) = self.clipped_render_region(left, top, width, height) else {
            return;
        };
        unsafe {
            let cache = &mut (*(&raw mut WINDOWS))[slot];
            if !cache.valid
                || cache.width != width
                || cache.height != height
                || cache.inset != inset
            {
                let scratch = (&raw mut SCRATCH).cast::<u32>();
                for y in top..top + height {
                    core::ptr::write_bytes(scratch.add(y * self.stride + left), 0, width);
                }
                let mut target = *self;
                target.buffer = scratch;
                target.recording_surface = true;
                target.fast_motion_frame = true;
                target.render_clip = Some(PresentRegion {
                    left,
                    top,
                    right: left + width,
                    bottom: top + height,
                });
                paint(&mut target);
                for y in 0..height {
                    core::ptr::copy_nonoverlapping(
                        scratch.add((top + y) * self.stride + left),
                        cache.pixels.as_mut_ptr().add(y * width),
                        width,
                    );
                }
                cache.width = width;
                cache.height = height;
                cache.inset = inset;
                cache.valid = true;
            }
            let (opacity, blur) = self.active_background_effects();
            if blur >= 2 && opacity < 100 && !self.fast_motion_frame {
                self.blur_framebuffer_region(bounds.0, bounds.1, bounds.2, bounds.3, blur as usize);
            }
            for y in region.top..region.bottom {
                for x in region.left..region.right {
                    let source = cache.pixels[(y - top) * width + x - left];
                    if source >> 24 == 0 {
                        continue;
                    }
                    let destination = self.buffer.add(y * self.stride + x);
                    let pixel = if source >> 24 == 255 {
                        source & 0xffffff
                    } else {
                        over(source, core::ptr::read_volatile(destination))
                    };
                    core::ptr::write_volatile(destination, pixel);
                }
            }
        }
        self.mark_dirty_rect(
            region.left,
            region.top,
            region.right - region.left,
            region.bottom - region.top,
        );
    }
}
