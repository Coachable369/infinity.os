//! Native desktop-owned premultiplied window surfaces. Applications never receive
//! either the presentation buffer or the physical framebuffer pointer.
use super::{DisplayDevice, PresentRegion};
const PIXELS: usize = 2560 * 1600;
const SCRATCH_PIXELS: usize = 3840 * 2160;
const SLOTS: usize = 6;

// ------------------------=
// FUNC: invalidate_revision
// DESC: Invalidates only the visible owner's cached window when its committed projection changes; unchanged or hidden observations do not repaint.
// ------------------=
pub(super) fn invalidate_revision(slot:usize,previous:&mut Option<u64>,current:Option<u64>)->bool {
    if *previous==current {return false;}
    *previous=current;
    if current.is_none() || slot>=SLOTS {return false;}
    unsafe {let cache=&mut (*(&raw mut WINDOWS))[slot];cache.valid=false;cache.damage=None;}
    true
}

#[derive(Clone, Copy)]
struct CachedWindow {
    valid: bool,
    width: usize,
    height: usize,
    inset: (usize, usize),
    damage: Option<PresentRegion>,
    pixels: [u32; PIXELS],
}
static mut WINDOWS: [CachedWindow; SLOTS] = [CachedWindow {
    valid: false,
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
    if slot >= SLOTS || region.left >= region.right || region.top >= region.bottom { return; }
    unsafe {
        let cache = &mut (*(&raw mut WINDOWS))[slot];
        if !cache.valid { return; }
        cache.damage = Some(match cache.damage {
            Some(previous) => PresentRegion { left: previous.left.min(region.left),
                top: previous.top.min(region.top), right: previous.right.max(region.right),
                bottom: previous.bottom.max(region.bottom) },
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
            let rebuild = !cache.valid
                || cache.width != width
                || cache.height != height
                || cache.inset != inset;
            if rebuild || cache.damage.is_some() {
                let changed = if rebuild { PresentRegion { left, top, right: left + width, bottom: top + height } }
                    else { let damage = cache.damage.unwrap(); PresentRegion {
                        left: damage.left.clamp(left, left + width), top: damage.top.clamp(top, top + height),
                        right: damage.right.clamp(left, left + width), bottom: damage.bottom.clamp(top, top + height),
                    } };
                let scratch = (&raw mut SCRATCH).cast::<u32>();
                for y in changed.top..changed.bottom {
                    core::ptr::write_bytes(scratch.add(y * self.stride + changed.left), 0, changed.right.saturating_sub(changed.left));
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
                        cache.pixels.as_mut_ptr().add((y - top) * width + changed.left.saturating_sub(left)),
                        changed.right.saturating_sub(changed.left),
                    );
                }
                cache.width = width;
                cache.height = height;
                cache.inset = inset;
                cache.valid = true;
                cache.damage = None;
            }
            let (opacity, blur) = self.active_background_effects();
            if slot != 5 && blur >= 2 && opacity < 100 && !self.fast_motion_frame {
                self.blur_framebuffer_region(bounds.0, bounds.1, bounds.2, bounds.3, blur as usize);
            }
            for y in region.top..region.bottom {
                if y & 31 == 0 { crate::ui::input_capture::poll(); }
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
