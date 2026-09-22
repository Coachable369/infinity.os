//! Native desktop-owned premultiplied window surfaces. Applications never receive
//! either the presentation buffer or the physical framebuffer pointer.
use super::{DisplayDevice, PresentRegion};
const PIXELS: usize = 2560 * 1600;
const SCRATCH_PIXELS: usize = 3840 * 2160;
// Six ordinary shell slots plus one persistent surface per navigator instance.
const SLOTS: usize = 12;

// ------------------------=
// FUNC: invalidate_revision
// DESC: Invalidates only the visible owner's cached window when its committed projection changes; unchanged or hidden observations do not repaint.
// ------------------=
pub(super) fn invalidate_revision(
    slot: usize,
    previous: &mut Option<u64>,
    current: Option<u64>,
) -> bool {
    if *previous == current {
        return false;
    }
    *previous = current;
    if current.is_none() || slot >= SLOTS {
        return false;
    }
    unsafe {
        let cache = &mut (*(&raw mut WINDOWS))[slot];
        cache.valid = false;
        cache.damage = None;
    }
    true
}

#[derive(Clone, Copy)]
struct CachedWindow {
    valid: bool,
    has_pixels: bool,
    icon_theme: u8,
    width: usize,
    height: usize,
    inset: (usize, usize),
    damage: Option<PresentRegion>,
    pixels: [u32; PIXELS],
}
static mut WINDOWS: [CachedWindow; SLOTS] = [CachedWindow {
    valid: false,
    has_pixels: false,
    // Invalid entries never consult their theme. Keep the initializer zero so
    // the bounded pixel cache occupies BSS, not bytes in the installed kernel.
    icon_theme: 0,
    width: 0,
    height: 0,
    inset: (0, 0),
    damage: None,
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
            window.damage = None;
        }
    }
}

// ------------------------=
// FUNC: invalidate_region
// DESC: Invalidates a bounded region of one unchanged window surface without discarding unrelated cached windows.
// ------------------=
pub(super) fn invalidate_region(slot: usize, region: PresentRegion) {
    if slot >= SLOTS || region.left >= region.right || region.top >= region.bottom {
        return;
    }
    unsafe {
        let cache = &mut (*(&raw mut WINDOWS))[slot];
        if !cache.valid {
            return;
        }
        cache.damage = Some(match cache.damage {
            Some(previous) => PresentRegion {
                left: previous.left.min(region.left),
                top: previous.top.min(region.top),
                right: previous.right.max(region.right),
                bottom: previous.bottom.max(region.bottom),
            },
            None => region,
        });
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
    // FUNC: spatial_preview
    // DESC: Samples a real retained window with bilinear premultiplied filtering without calling its renderer.
    // ------------------=
    pub(super) fn spatial_preview(&mut self, slot: usize, bounds: (usize, usize, usize, usize)) {
        if slot >= SLOTS {
            return;
        }
        let (left, top, width, height) = bounds;
        if width == 0 || height == 0 {
            return;
        }
        unsafe {
            let cache = &(*(&raw const WINDOWS))[slot];
            if !cache.has_pixels || cache.width == 0 || cache.height == 0 {
                return;
            }
            // Preserve aspect ratio; no stretched fonts in overview thumbnails.
            let factor = (width * 65536 / cache.width).min(height * 65536 / cache.height);
            let w = (cache.width * factor / 65536).max(1);
            let h = (cache.height * factor / 65536).max(1);
            let left = left + (width - w) / 2;
            let top = top + (height - h) / 2;
            let Some(region) = self.clipped_render_region(left, top, w, h) else {
                return;
            };
            for y in region.top..region.bottom {
                let sy = ((y - top) * 256 * cache.height / h).min((cache.height - 1) * 256);
                for x in region.left..region.right {
                    let sx = ((x - left) * 256 * cache.width / w).min((cache.width - 1) * 256);
                    let (x0, y0) = (sx / 256, sy / 256);
                    let (fx, fy) = (sx % 256, sy % 256);
                    let (x1, y1) = (
                        (x0 + 1).min(cache.width - 1),
                        (y0 + 1).min(cache.height - 1),
                    );
                    let mut sum = [0usize; 4];
                    for (px, py, weight) in [
                        (x0, y0, (256 - fx) * (256 - fy)),
                        (x1, y0, fx * (256 - fy)),
                        (x0, y1, (256 - fx) * fy),
                        (x1, y1, fx * fy),
                    ] {
                        let pixel = cache.pixels[py * cache.width + px];
                        for (i, value) in sum.iter_mut().enumerate() {
                            *value += ((pixel >> (8 * i)) & 255) as usize * weight;
                        }
                    }
                    let mut pixel = 0u32;
                    for (i, value) in sum.iter().enumerate() {
                        pixel |= ((value / 65536) as u32) << (8 * i);
                    }
                    let out = self.buffer.add(y * self.stride + x);
                    *out = over(pixel, *out);
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
        let icon_theme = self.active_icon_theme();
        unsafe {
            let cache = &mut (*(&raw mut WINDOWS))[slot];
            let rebuild = !cache.valid
                || cache.icon_theme != icon_theme
                || cache.width != width
                || cache.height != height
                || cache.inset != inset;
            if rebuild || cache.damage.is_some() {
                let changed = if rebuild {
                    PresentRegion {
                        left,
                        top,
                        right: left + width,
                        bottom: top + height,
                    }
                } else {
                    let damage = cache.damage.unwrap();
                    PresentRegion {
                        left: damage.left.clamp(left, left + width),
                        top: damage.top.clamp(top, top + height),
                        right: damage.right.clamp(left, left + width),
                        bottom: damage.bottom.clamp(top, top + height),
                    }
                };
                let scratch = (&raw mut SCRATCH).cast::<u32>();
                for y in changed.top..changed.bottom {
                    core::ptr::write_bytes(
                        scratch.add(y * self.stride + changed.left),
                        0,
                        changed.right.saturating_sub(changed.left),
                    );
                }
                let mut target = *self;
                target.buffer = scratch;
                target.recording_surface = true;
                target.fast_motion_frame = true;
                target.render_clip = Some(changed);
                paint(&mut target);
                for y in changed.top..changed.bottom {
                    core::ptr::copy_nonoverlapping(
                        scratch.add(y * self.stride + changed.left),
                        cache
                            .pixels
                            .as_mut_ptr()
                            .add((y - top) * width + changed.left.saturating_sub(left)),
                        changed.right.saturating_sub(changed.left),
                    );
                }
                cache.width = width;
                cache.height = height;
                cache.inset = inset;
                cache.valid = true;
                cache.has_pixels = true;
                cache.icon_theme = icon_theme;
                cache.damage = None;
            }
            let (opacity, blur) = self.active_background_effects();
            if slot != 5 && blur >= 2 && opacity < 100 && !self.fast_motion_frame {
                self.blur_framebuffer_region(bounds.0, bounds.1, bounds.2, bounds.3, blur as usize);
            }
            for y in region.top..region.bottom {
                if y & 31 == 0 {
                    crate::ui::input_capture::poll();
                }
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
