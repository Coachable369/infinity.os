//! Frozen desktop backdrop for launcher-only presentation, invalidated on close.
use super::DisplayDevice;
#[path = "spatial_surface.rs"]
mod spatial_surface;
const PIXELS: usize = 3840 * 2160;
static mut BACKDROP: [u32; PIXELS] = [0; PIXELS];
static mut SIZE: (usize, usize, usize) = (0, 0, 0);
static mut STAGE: [u32; PIXELS] = [0; PIXELS];
static mut STAGE_SIZE: (usize, usize, usize) = (0, 0, 0);

// ------------------------=
// FUNC: compose_spatial
// DESC: Fuses retained-scene translation and desktop fading without intermediate framebuffer passes.
// ------------------=
pub(super) fn compose_spatial(
    display: &mut DisplayDevice,
    scene: &[u32],
    lift: usize,
    opacity: u8,
    isolated: bool,
) -> bool {
    unsafe {
        if SIZE != (display.width, display.height, display.stride)
            || (isolated && STAGE_SIZE != SIZE)
        {
            return false;
        }
        let Some(region) = display.clipped_render_region(0, 0, display.width, display.height)
        else {
            return false;
        };
        spatial_surface::compose(
            scene,
            if isolated {
                &*(&raw const STAGE)
            } else {
                &*(&raw const BACKDROP)
            },
            core::slice::from_raw_parts_mut(display.buffer, display.stride * display.height),
            display.stride,
            (region.left, region.top, region.right, region.bottom),
            lift,
            opacity,
        )
    }
}

// ------------------------=
// FUNC: column_sum
// DESC: Sums three vertical taps in independent 16-bit lanes without channel overflow.
// ------------------=
fn column_sum(source: *const u32, stride: usize, x: usize, rows: [usize; 3]) -> u64 {
    let mut sum = 0;
    for y in rows {
        let p = unsafe { *source.add(y * stride + x) } as u64;
        sum += (p & 255) | ((p & 0xff00) << 8) | ((p & 0xff0000) << 16);
    }
    sum
}

// ------------------------=
// FUNC: capture_stage
// DESC: Caches a defocused, dimmed spatial stage once per desktop change, preserving the original for dismissal.
// ------------------=
pub(super) fn capture_stage(display: &DisplayDevice) {
    capture_stage_pixels(display, false);
}
// ------------------------=
// FUNC: capture_clean_stage
// DESC: Retains wallpaper-only pixels while preserving the desktop snapshot for closing.
// ------------------=
pub(super) fn capture_clean_stage(display: &DisplayDevice) {
    capture_stage_pixels(display, true);
}
// ------------------------=
// FUNC: capture_stage_pixels
// DESC: Builds an edge-to-edge soft stage from the original desktop or isolated wallpaper.
// ------------------=
fn capture_stage_pixels(display: &DisplayDevice, clean: bool) {
    unsafe {
        STAGE_SIZE = (0, 0, 0);
        if SIZE != (display.width, display.height, display.stride)
            || display.width == 0
            || display.height == 0
        {
            return;
        }
        let source = if clean {
            display.buffer as *const u32
        } else {
            (&raw const BACKDROP).cast::<u32>()
        };
        for y in 0..display.height {
            let rows = [y.saturating_sub(8), y, (y + 8).min(display.height - 1)];
            // The next pixel reuses sixteen columns. Only one new three-tap
            // column is loaded; the 17-slot ring also handles clamped edges.
            let mut columns = [0u64; 17];
            for x in 0..=8.min(display.width - 1) {
                columns[x] = column_sum(source, display.stride, x, rows);
            }
            for x in 0..display.width {
                let right = (x + 8).min(display.width - 1);
                if x != 0 {
                    columns[right % 17] = column_sum(source, display.stride, right, rows);
                }
                let sums =
                    columns[x.saturating_sub(8) % 17] + columns[x % 17] + columns[right % 17];
                let original = *source.add(y * display.stride + x);
                let mut pixel = original & 0xff000000;
                for i in 0..3 {
                    let sum = ((sums >> (i * 16)) & 0xffff) as u32;
                    pixel |= (sum / 45) << (i * 8);
                }
                (*(&raw mut STAGE))[y * display.stride + x] = pixel;
            }
        }
        STAGE_SIZE = SIZE;
    }
}

// ------------------------=
// FUNC: restore_stage
// DESC: Restores only damaged spatial-stage rows without repeating blur, tint, or application rendering.
// ------------------=
pub(super) fn restore_stage(display: &mut DisplayDevice) -> bool {
    unsafe {
        if STAGE_SIZE != (display.width, display.height, display.stride) {
            return false;
        }
        if let Some(region) = display.clipped_render_region(0, 0, display.width, display.height) {
            for y in region.top..region.bottom {
                let offset = y * display.stride + region.left;
                core::ptr::copy_nonoverlapping(
                    (&raw const STAGE).cast::<u32>().add(offset),
                    display.buffer.add(offset),
                    region.right - region.left,
                );
            }
        }
    }
    true
}

// ------------------------=
// FUNC: invalidate
// DESC: Releases the cached desktop snapshot when normal desktop composition resumes.
// ------------------=
pub(super) fn invalidate() {
    unsafe {
        SIZE = (0, 0, 0);
        STAGE_SIZE = (0, 0, 0);
    }
}

// ------------------------=
// FUNC: capture
// DESC: Saves a completely composed backdrop once, never capturing a partially painted frame.
// ------------------=
pub(super) fn capture(display: &DisplayDevice) {
    if display.render_clip.is_some() || display.stride * display.height > PIXELS {
        return;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(
            display.buffer,
            (&raw mut BACKDROP).cast::<u32>(),
            display.stride * display.height,
        );
        SIZE = (display.width, display.height, display.stride);
    }
}

// ------------------------=
// FUNC: restore
// DESC: Restores only damaged backdrop rows without repainting applications, icons, wallpaper or glass.
// ------------------=
pub(super) fn restore(display: &mut DisplayDevice) -> bool {
    unsafe {
        if SIZE != (display.width, display.height, display.stride) {
            return false;
        }
        if let Some(region) = display.clipped_render_region(0, 0, display.width, display.height) {
            for y in region.top..region.bottom {
                let offset = y * display.stride + region.left;
                core::ptr::copy_nonoverlapping(
                    (&raw const BACKDROP).cast::<u32>().add(offset),
                    display.buffer.add(offset),
                    region.right - region.left,
                );
            }
        }
    }
    true
}

// ------------------------=
// FUNC: fade
// DESC: Blends only the active damaged overlay against its retained desktop for interruption-safe reveal and dismissal.
// ------------------=
pub(super) fn fade(display: &mut DisplayDevice, opacity: u8) {
    if opacity == 255 {
        return;
    }
    unsafe {
        if SIZE != (display.width, display.height, display.stride) {
            return;
        }
        let Some(region) = display.clipped_render_region(0, 0, display.width, display.height)
        else {
            return;
        };
        let a = u32::from(opacity);
        let b = 255 - a;
        for y in region.top..region.bottom {
            for x in region.left..region.right {
                let offset = y * display.stride + x;
                let pixel = display.buffer.add(offset);
                let behind = (*(&raw const BACKDROP))[offset];
                let foreground = *pixel;
                // Two independent 16-bit lanes; the rounded numerator never
                // exceeds 65152 so neither multiply nor division carries lanes.
                let lanes = (foreground & 0x00ff00ff) * a + (behind & 0x00ff00ff) * b + 0x007f007f;
                let rb = ((lanes + 0x00010001 + ((lanes >> 8) & 0x00ff00ff)) >> 8) & 0x00ff00ff;
                let green = (((foreground >> 8) & 255) * a + ((behind >> 8) & 255) * b + 127) / 255;
                *pixel = (foreground & 0xff000000) | rb | (green << 8);
            }
        }
    }
}
