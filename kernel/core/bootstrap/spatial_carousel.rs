//! Bounded card sprites for the native Holographic carousel; no app paints in motion.
use super::*;
use crate::ui::spatial::{OverviewBounds, OverviewFrame, OVERVIEW_COUNT};
const CAPACITY: usize = 12 * 1024 * 1024;
static mut PIXELS: [u32; CAPACITY] = [0; CAPACITY];
static mut SCRATCH: [u32; 3840 * 2160] = [0; 3840 * 2160];
static mut SPRITES: [(usize, usize, usize); OVERVIEW_COUNT] = [(0, 0, 0); OVERVIEW_COUNT];
static mut KEY: Option<(usize, usize, usize, usize, u8)> = None;

// ------------------------=
// FUNC: invalidate
// DESC: Discards carousel sprites after the desktop or overlay lifetime changes.
// ------------------=
pub(super) fn invalidate() {
    unsafe {
        KEY = None;
    }
}

// ------------------------=
// FUNC: paint
// DESC: Prepares card chrome once at the larger endpoint size, then scales retained cards in animated depth order.
// ------------------=
pub(super) fn paint(
    d: &mut DisplayDevice,
    previews: &[Preview],
    from: &[OverviewBounds; OVERVIEW_COUNT],
    focus: usize,
    zoom: u8,
    progress: u8,
) -> bool {
    let key = (d.width, d.height, previews.len(), focus, zoom);
    if d.stride * d.height > 3840 * 2160 || previews.len() > OVERVIEW_COUNT {
        return false;
    }
    unsafe {
        if KEY != Some(key) {
            let mut used = 0;
            let mut sprites = [(0, 0, 0); OVERVIEW_COUNT];
            for (i, preview) in previews.iter().enumerate() {
                let target = overview_bounds(i, focus, zoom, previews.len());
                let w = (from[i].2.max(target.2) * d.width / 1000 + 24).min(d.width);
                let h = (from[i].3.max(target.3) * d.height / 1000 + 24).min(d.height);
                if w < 25 || h < 25 || used + w * h > CAPACITY {
                    return false;
                }
                let scratch = (&raw mut SCRATCH).cast::<u32>();
                for y in 0..h {
                    core::ptr::write_bytes(scratch.add(y * d.stride), 0, w);
                }
                let mut target = *d;
                target.buffer = scratch;
                target.recording_surface = true;
                target.fast_motion_frame = true;
                target.set_render_clip(0, 0, w, h);
                paint_card(&mut target, preview, (12, 12, w - 24, h - 24), i == focus);
                for y in 0..h {
                    core::ptr::copy_nonoverlapping(
                        scratch.add(y * d.stride),
                        (&raw mut PIXELS).cast::<u32>().add(used + y * w),
                        w,
                    );
                }
                sprites[i] = (used, w, h);
                used += w * h;
            }
            SPRITES = sprites;
            KEY = Some(key);
        }
        let frame = OverviewFrame::new(Some(from), focus, zoom, previews.len(), progress);
        for &i in &frame.order[..frame.count] {
            let (a, b, w, h) = frame.bounds[i];
            let (offset, sw, sh) = SPRITES[i];
            let bounds = (
                (a * d.width / 1000).saturating_sub(12),
                (b * d.height / 1000).saturating_sub(12),
                w * d.width / 1000 + 24,
                h * d.height / 1000 + 24,
            );
            d.spatial_surface(
                &(&*(&raw const PIXELS))[offset..offset + sw * sh],
                sw,
                sh,
                bounds,
            );
        }
    }
    true
}
