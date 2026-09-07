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
