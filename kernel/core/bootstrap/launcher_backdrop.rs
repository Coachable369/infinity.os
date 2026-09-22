//! Frozen desktop backdrop for launcher-only presentation, invalidated on close.
use super::DisplayDevice;
const PIXELS: usize = 3840 * 2160;
static mut BACKDROP: [u32; PIXELS] = [0; PIXELS];
static mut SIZE: (usize, usize, usize) = (0, 0, 0);

// ------------------------=
// FUNC: invalidate
// DESC: Releases the cached desktop snapshot when normal desktop composition resumes.
// ------------------=
pub(super) fn invalidate() {
    unsafe {
        SIZE = (0, 0, 0);
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
