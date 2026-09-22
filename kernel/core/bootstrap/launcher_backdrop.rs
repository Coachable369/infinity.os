//! Frozen desktop backdrop for launcher-only presentation, invalidated on close.
use super::DisplayDevice;
const PIXELS: usize = 3840 * 2160;
static mut BACKDROP: [u32; PIXELS] = [0; PIXELS];
static mut SIZE: (usize, usize, usize) = (0, 0, 0);
static mut STAGE: [u32; PIXELS] = [0; PIXELS];
static mut STAGE_SIZE: (usize, usize, usize) = (0, 0, 0);

// ------------------------=
// FUNC: capture_stage
// DESC: Caches a defocused, dimmed spatial stage once per desktop change, preserving the original for dismissal.
// ------------------=
pub(super) fn capture_stage(display: &DisplayDevice) {
    unsafe {
        STAGE_SIZE = (0, 0, 0);
        if SIZE != (display.width, display.height, display.stride)
            || display.width == 0
            || display.height == 0
        {
            return;
        }
        for y in 0..display.height {
            for x in 0..display.width {
                let mut channels = [0u32; 3];
                for dy in [-8isize, 0, 8] {
                    for dx in [-8isize, 0, 8] {
                        let sx = (x as isize + dx).clamp(0, display.width as isize - 1) as usize;
                        let sy = (y as isize + dy).clamp(0, display.height as isize - 1) as usize;
                        let pixel = (*(&raw const BACKDROP))[sy * display.stride + sx];
                        for (i, sum) in channels.iter_mut().enumerate() {
                            *sum += (pixel >> (i * 8)) & 255;
                        }
                    }
                }
                let original = (*(&raw const BACKDROP))[y * display.stride + x];
                let edge = if display.width >= 256 {
                    x.saturating_sub(display.width * 4 / 100)
                        .min((display.width * 96 / 100).saturating_sub(x))
                        .min(y.saturating_sub(display.height * 7 / 100))
                        .min((display.height * 96 / 100).saturating_sub(y))
                        .saturating_mul(255)
                        / (display.width / 24).max(1)
                } else {
                    255
                }
                .min(255) as u32;
                let mut pixel = original & 0xff000000;
                for (i, sum) in channels.iter().enumerate() {
                    let source = (original >> (i * 8)) & 255;
                    pixel |= ((sum / 45 * edge + source * (255 - edge)) / 255) << (i * 8);
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
                let mut color = foreground & 0xff000000;
                for shift in [0, 8, 16] {
                    color |=
                        ((((foreground >> shift) & 255) * a + ((behind >> shift) & 255) * b + 127)
                            / 255)
                            << shift;
                }
                *pixel = color;
            }
        }
    }
}
