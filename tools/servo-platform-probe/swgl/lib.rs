//! Focused native rasterizer prerequisite; not a browser or host GL fallback.
use gleam::gl::{self, Gl};

// ------------------------=
// FUNC: verify_clear
// DESC: Exercises upstream SWGL raster output against actual CPU-owned pixels.
// ------------------=
pub fn verify_clear() -> u64 {
    let context = swgl::Context::create();
    context.make_current();
    let mut pixels = vec![0u32; 64 * 32];
    context.init_default_framebuffer(0, 0, 64, 32, 64 * 4, pixels.as_mut_ptr().cast());
    context.clear_color(1.0, 0.0, 0.0, 1.0);
    context.clear(gl::COLOR_BUFFER_BIT);
    context.finish();
    let output = context.read_pixels(0, 0, 64, 32, gl::RGBA, gl::UNSIGNED_BYTE);
    let failure = if output.len() != 64 * 32 * 4 { u64::MAX } else { output.chunks_exact(4).enumerate()
        .find(|(_, pixel)| *pixel != [255, 0, 0, 255])
        .map(|(index, pixel)| ((index as u64 + 1) << 32) |
            u32::from_le_bytes(pixel.try_into().unwrap()) as u64).unwrap_or(0) };
    context.destroy();
    failure
}
