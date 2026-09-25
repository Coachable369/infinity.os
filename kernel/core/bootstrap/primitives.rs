//! Shared framebuffer pixels, blending, geometry, bitmap, icon, and font primitives.

use core::ptr::{read_volatile, write_volatile};

#[path = "soft_stroke.rs"]
mod soft_stroke;

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const UI_FONT_ATLAS: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityUI-Regular-24.atlas");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const UI_FONT_SEMIBOLD_ATLAS: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityUI-Semibold-24.atlas");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const UI_FONT_METRICS: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityUI-Regular-24.metrics");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const UI_FONT_SEMIBOLD_METRICS: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityUI-Semibold-24.metrics");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const UI_FONT_KERN: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityUI-Regular-24.kern");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const UI_FONT_SEMIBOLD_KERN: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityUI-Semibold-24.kern");
#[cfg(target_arch = "x86")]
pub(super) const UI_FONT_ATLAS: &[u8] = &[];
#[cfg(target_arch = "x86")]
pub(super) const UI_FONT_SEMIBOLD_ATLAS: &[u8] = &[];
#[cfg(target_arch = "x86")]
pub(super) const UI_FONT_METRICS: &[u8] = &[];
#[cfg(target_arch = "x86")]
pub(super) const UI_FONT_SEMIBOLD_METRICS: &[u8] = &[];
#[cfg(target_arch = "x86")]
pub(super) const UI_FONT_KERN: &[u8] = &[];
#[cfg(target_arch = "x86")]
pub(super) const UI_FONT_SEMIBOLD_KERN: &[u8] = &[];
pub(super) const UI_FONT_CELL_WIDTH: usize = 24;
pub(super) const UI_FONT_CELL_HEIGHT: usize = 28;

// ============================================================
// TYPOGRAPHY SIZE CONTROLS
// ============================================================
// These are the user-facing font-size knobs for this module.
// Change the *_FONT_SIZE_PX values to resize a font family while
// keeping the original bundled atlas as the raster source.
//
// The *_NATIVE_SIZE_PX values describe the pixel size used when
// each atlas was generated and normally should not be changed.
//
// Examples:
//   UI_FONT_SIZE_PX = 26;                 // slightly larger desktop/UI text
//   INSTALLER_FONT_SIZE_PX = 28;          // larger installer body text
//   INSTALLER_HEADLINE_FONT_SIZE_PX = 38; // larger installer headlines
//   INSTALLER_COMPACT_FONT_SIZE_PX = 21;  // larger compact installer text
//
// Existing per-call UI `scale` values are preserved. A UI call with
// scale=2 renders at UI_FONT_SIZE_PX * 2, exactly as before when the
// default UI_FONT_SIZE_PX remains 24.
pub(super) const UI_FONT_NATIVE_SIZE_PX: usize = 24;
pub(super) const UI_FONT_SIZE_PX: usize = 24;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const CURSOR_BMP: &[u8] = include_bytes!("../../../assets/boot/infinity-cursor-v1.bmp");
pub(super) const INFINITY_PATH: [(i32, i32); 97] = [
    (0, 0),
    (7, 8),
    (15, 16),
    (22, 24),
    (29, 31),
    (36, 38),
    (43, 44),
    (50, 49),
    (56, 54),
    (62, 57),
    (68, 60),
    (74, 61),
    (79, 62),
    (84, 61),
    (89, 60),
    (93, 57),
    (97, 54),
    (100, 49),
    (103, 44),
    (106, 38),
    (108, 31),
    (110, 24),
    (111, 16),
    (112, 8),
    (112, 0),
    (112, -8),
    (111, -16),
    (110, -24),
    (108, -31),
    (106, -38),
    (103, -44),
    (100, -49),
    (97, -54),
    (93, -57),
    (89, -60),
    (84, -61),
    (79, -62),
    (74, -61),
    (68, -60),
    (62, -57),
    (56, -54),
    (50, -49),
    (43, -44),
    (36, -38),
    (29, -31),
    (22, -24),
    (15, -16),
    (7, -8),
    (0, 0),
    (-7, 8),
    (-15, 16),
    (-22, 24),
    (-29, 31),
    (-36, 38),
    (-43, 44),
    (-50, 49),
    (-56, 54),
    (-62, 57),
    (-68, 60),
    (-74, 61),
    (-79, 62),
    (-84, 61),
    (-89, 60),
    (-93, 57),
    (-97, 54),
    (-100, 49),
    (-103, 44),
    (-106, 38),
    (-108, 31),
    (-110, 24),
    (-111, 16),
    (-112, 8),
    (-112, 0),
    (-112, -8),
    (-111, -16),
    (-110, -24),
    (-108, -31),
    (-106, -38),
    (-103, -44),
    (-100, -49),
    (-97, -54),
    (-93, -57),
    (-89, -60),
    (-84, -61),
    (-79, -62),
    (-74, -61),
    (-68, -60),
    (-62, -57),
    (-56, -54),
    (-50, -49),
    (-43, -44),
    (-36, -38),
    (-29, -31),
    (-22, -24),
    (-15, -16),
    (-7, -8),
    (0, 0),
];

#[derive(Clone, Copy)]
pub(super) struct FontSize {
    native_px: usize,
    pixels: usize,
}

impl FontSize {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a font scaling descriptor from native and destination pixel sizes.
    // ------------------=
    pub(super) const fn new(native_px: usize, pixels: usize) -> Self {
        Self { native_px, pixels }
    }

    // ------------------------=
    // FUNC: scale_usize
    // DESC: Scales an unsigned font metric into destination pixels.
    // ------------------=
    pub(super) fn scale_usize(self, value: usize) -> usize {
        let native = self.native_px.max(1);
        let pixels = self.pixels.max(1);
        value.saturating_mul(pixels).saturating_add(native / 2) / native
    }

    // ------------------------=
    // FUNC: scale_isize
    // DESC: Scales a signed font metric into destination pixels.
    // ------------------=
    pub(super) fn scale_isize(self, value: isize) -> isize {
        if value == 0 {
            return 0;
        }
        let magnitude = self.scale_usize(value.unsigned_abs()) as isize;
        if value < 0 {
            -magnitude
        } else {
            magnitude
        }
    }

    // ------------------------=
    // FUNC: source_index
    // DESC: Maps a destination font pixel to its native atlas coordinate.
    // ------------------=
    pub(super) fn source_index(self, destination: usize) -> usize {
        destination.saturating_mul(self.native_px.max(1)) / self.pixels.max(1)
    }
}

impl super::DisplayDevice {
    // ------------------------=
    // FUNC: soft_stroke
    // DESC: Blends a subpixel luminous stroke without allocating or touching pixels outside its bounded support.
    // ------------------=
    pub(super) fn soft_stroke(&mut self, from: (i32, i32), to: (i32, i32), scale: i32, intensity: u8) {
        let mut bounds = (usize::MAX, usize::MAX, 0usize, 0usize);
        soft_stroke::segment(from, to, scale, |x, y, alpha| {
            if x < 0
                || y < 0
                || x as usize >= self.width
                || y as usize >= self.height
                || !self.render_point_visible(x as usize, y as usize)
            {
                return;
            }
            let (x, y) = (x as usize, y as usize);
            bounds.0 = bounds.0.min(x);
            bounds.1 = bounds.1.min(y);
            bounds.2 = bounds.2.max(x + 1);
            bounds.3 = bounds.3.max(y + 1);
            let core = u16::from(alpha.saturating_sub(120));
            self.blend_color_unchecked(
                x,
                y,
                (38 + core) as u8,
                (153 + core * 3 / 4) as u8,
                255,
                (u16::from(alpha) * u16::from(intensity) / 255) as u8,
            );
        });
        if bounds.0 < bounds.2 && bounds.1 < bounds.3 {
            self.mark_dirty_rect(
                bounds.0,
                bounds.1,
                bounds.2 - bounds.0,
                bounds.3 - bounds.1,
            );
        }
    }
    // ------------------------=
    // FUNC: framebuffer_pixel
    // DESC: Reads one already-bounded raw framebuffer pixel for damage restoration.
    // ------------------=
    pub(super) fn framebuffer_pixel(&self, x: usize, y: usize) -> u32 {
        unsafe { read_volatile(self.buffer.add(y * self.stride + x)) }
    }

    // ------------------------=
    // FUNC: write_framebuffer_pixel
    // DESC: Writes one already-bounded raw framebuffer pixel for damage restoration.
    // ------------------=
    pub(super) fn write_framebuffer_pixel(&mut self, x: usize, y: usize, value: u32) {
        if !self.render_point_visible(x, y) {
            return;
        }
        unsafe { write_volatile(self.buffer.add(y * self.stride + x), value) }
        self.mark_dirty_rect(x, y, 1, 1);
    }

    // ------------------------=
    // FUNC: blur_framebuffer
    // DESC: Applies a bounded block blur to the current framebuffer without allocating a second surface.
    // ------------------=
    pub(super) fn blur_framebuffer(&mut self, block_size: usize) {
        self.blur_framebuffer_region(0, 0, self.width, self.height, block_size);
    }

    // ------------------------=
    // FUNC: blur_framebuffer_region
    // DESC: Applies allocation-free block blur only beneath one semantic background rectangle.
    // ------------------=
    pub(super) fn blur_framebuffer_region(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        block_size: usize,
    ) {
        let Some(region) = self.clipped_render_region(left, top, width, height) else {
            return;
        };
        let left_bound = region.left;
        let top_bound = region.top;
        let right_bound = region.right;
        let bottom_bound = region.bottom;
        if right_bound.saturating_sub(left_bound) < 2
            || bottom_bound.saturating_sub(top_bound) < 2
            || block_size < 2
        {
            return;
        }
        let average = |first: u32, second: u32, third: u32, fourth: u32| -> u32 {
            let channel = |shift: u32| {
                (((first >> shift) & 255)
                    + ((second >> shift) & 255)
                    + ((third >> shift) & 255)
                    + ((fourth >> shift) & 255))
                    >> 2
            };
            channel(0) | channel(8) << 8 | channel(16) << 16 | channel(24) << 24
        };
        let block = block_size.min(8);
        for block_top in (top_bound..bottom_bound).step_by(block) {
            let bottom = (block_top + block - 1).min(bottom_bound - 1);
            for block_left in (left_bound..right_bound).step_by(block) {
                let right = (block_left + block - 1).min(right_bound - 1);
                let color = average(
                    unsafe { read_volatile(self.buffer.add(block_top * self.stride + block_left)) },
                    unsafe { read_volatile(self.buffer.add(block_top * self.stride + right)) },
                    unsafe { read_volatile(self.buffer.add(bottom * self.stride + block_left)) },
                    unsafe { read_volatile(self.buffer.add(bottom * self.stride + right)) },
                );
                for y in block_top..=(block_top + block - 1).min(bottom_bound - 1) {
                    for x in block_left..=(block_left + block - 1).min(right_bound - 1) {
                        unsafe {
                            write_volatile(self.buffer.add(y * self.stride + x), color);
                        }
                    }
                }
            }
        }
        self.mark_dirty_rect(
            left_bound,
            top_bound,
            right_bound - left_bound,
            bottom_bound - top_bound,
        );
    }

    // ------------------------=
    // FUNC: pixel
    // DESC: Writes one clipped RGB pixel using the firmware-provided channel format.
    // ------------------=
    pub(super) fn pixel(&mut self, x: i32, y: i32, red: u8, green: u8, blue: u8) {
        if x < 0
            || y < 0
            || x as usize >= self.width
            || y as usize >= self.height
            || !self.render_point_visible(x as usize, y as usize)
        {
            return;
        }
        self.write_rgb_unchecked(x as usize, y as usize, red, green, blue);
        self.mark_dirty_rect(x as usize, y as usize, 1, 1);
    }

    // ------------------------=
    // FUNC: write_rgb_unchecked
    // DESC: Stores an already-bounded pixel without repeating per-pixel damage bookkeeping.
    // ------------------=
    fn write_rgb_unchecked(&mut self, x: usize, y: usize, red: u8, green: u8, blue: u8) {
        let color = if self.format == 0 {
            red as u32 | (green as u32) << 8 | (blue as u32) << 16
        } else {
            blue as u32 | (green as u32) << 8 | (red as u32) << 16
        };
        let color = color
            | if self.recording_surface {
                0xff000000
            } else {
                0
            };
        unsafe {
            write_volatile(
                self.buffer.add(y as usize * self.stride + x as usize),
                color,
            );
        }
    }

    // ------------------------=
    // FUNC: blend_white
    // DESC: Alpha-blends white over one existing framebuffer pixel.
    // ------------------=
    pub(super) fn blend_white(&mut self, x: i32, y: i32, alpha: u8) {
        self.blend_color(x, y, 255, 255, 255, alpha);
    }

    // ------------------------=
    // FUNC: blend_color
    // DESC: Alpha-blends an RGB color over one existing framebuffer pixel.
    // ------------------=
    pub(super) fn blend_color(
        &mut self,
        x: i32,
        y: i32,
        target_red: u8,
        target_green: u8,
        target_blue: u8,
        alpha: u8,
    ) {
        if x < 0
            || y < 0
            || x as usize >= self.width
            || y as usize >= self.height
            || !self.render_point_visible(x as usize, y as usize)
        {
            return;
        }
        self.blend_color_unchecked(
            x as usize,
            y as usize,
            target_red,
            target_green,
            target_blue,
            alpha,
        );
        self.mark_dirty_rect(x as usize, y as usize, 1, 1);
    }

    // ------------------------=
    // FUNC: blend_color_unchecked
    // DESC: Blends one already-clipped pixel; batch callers record damage once per primitive.
    // ------------------=
    fn blend_color_unchecked(
        &mut self,
        x: usize,
        y: usize,
        target_red: u8,
        target_green: u8,
        target_blue: u8,
        alpha: u8,
    ) {
        let address = unsafe { self.buffer.add(y as usize * self.stride + x as usize) };
        let color = unsafe { read_volatile(address) };
        let (red, green, blue) = if self.format == 0 {
            (
                (color & 255) as u8,
                ((color >> 8) & 255) as u8,
                ((color >> 16) & 255) as u8,
            )
        } else {
            (
                ((color >> 16) & 255) as u8,
                ((color >> 8) & 255) as u8,
                (color & 255) as u8,
            )
        };
        let mix = |value: u8, target: u8| {
            ((value as u16 * (255 - alpha as u16) + target as u16 * alpha as u16) / 255) as u8
        };
        let red = mix(red, target_red) as u32;
        let green = mix(green, target_green) as u32;
        let blue = mix(blue, target_blue) as u32;
        let packed = if self.format == 0 {
            red | green << 8 | blue << 16
        } else {
            blue | green << 8 | red << 16
        };
        let packed = packed
            | if self.recording_surface {
                (alpha as u32 + ((color >> 24) * (255 - alpha as u32) / 255)) << 24
            } else {
                0
            };
        unsafe {
            write_volatile(address, packed);
        }
    }

    // ------------------------=
    // FUNC: ui_text_centered
    // DESC: Centers smooth proportional UI text within a fixed horizontal region.
    // ------------------=
    pub(super) fn ui_text_centered(
        &mut self,
        left: usize,
        width: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        scale: usize,
    ) {
        let text_width = self.ui_text_width(text, scale);
        self.ui_text(
            left + width.saturating_sub(text_width) / 2,
            y,
            text,
            red,
            green,
            blue,
            scale,
        );
    }

    // ------------------------=
    // FUNC: ui_text_centered_strong
    // DESC: Centers semibold UI text using the shared proportional metrics.
    // ------------------=
    pub(super) fn ui_text_centered_strong(
        &mut self,
        left: usize,
        width: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        scale: usize,
    ) {
        let text_width = self.ui_text_width_weighted(text, scale, true);
        self.ui_text_strong(
            left + width.saturating_sub(text_width) / 2,
            y,
            text,
            red,
            green,
            blue,
            scale,
        );
    }

    // ------------------------=
    // FUNC: ui_text_elided_strong
    // DESC: Keeps the normal title font while eliding overlong ASCII window titles inside their reserved menu-free slot.
    // ------------------=
    pub(super) fn ui_text_elided_strong(&mut self, x: usize, y: usize, max_width: usize,
        text: &[u8], red: u8, green: u8, blue: u8) {
        if self.ui_text_width_weighted(text, 1, true) <= max_width {
            self.ui_text_strong(x, y, text, red, green, blue, 1);
            return;
        }
        let mut shortened = [0u8; 96];
        let mut length = text.len().min(shortened.len() - 3);
        shortened[..length].copy_from_slice(&text[..length]);
        loop {
            shortened[length..length + 3].copy_from_slice(b"...");
            if self.ui_text_width_weighted(&shortened[..length + 3], 1, true) <= max_width {
                self.ui_text_strong(x, y, &shortened[..length + 3], red, green, blue, 1);
                return;
            }
            if length == 0 { return; }
            length -= 1;
        }
    }

    // ------------------------=
    // FUNC: ui_text_fit_strong
    // DESC: Draws a heading at its preferred hierarchy and safely falls back when the measured glyphs exceed the container.
    // ------------------=
    pub(super) fn ui_text_fit_strong(
        &mut self,
        x: usize,
        y: usize,
        max_width: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        preferred_scale: usize,
    ) {
        let scale = if self.ui_text_width_weighted(text, preferred_scale, true) <= max_width {
            preferred_scale
        } else {
            1
        };
        self.ui_text_strong(x, y, text, red, green, blue, scale);
    }

    // ------------------------=
    // FUNC: ui_text_wrapped
    // DESC: Wraps proportional UI copy at word boundaries without allocating or drawing beyond the supplied width.
    // ------------------=
    pub(super) fn ui_text_wrapped(
        &mut self,
        x: usize,
        y: usize,
        max_width: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        max_lines: usize,
    ) {
        let mut start = 0usize;
        let mut line = 0usize;
        while start < text.len() && line < max_lines {
            while start < text.len()
                && matches!(text[start], b' ' | b'\t' | b'\r' | b'\n')
            {
                start += 1;
            }
            if start >= text.len() {
                break;
            }
            let end = self.ui_text_wrap_line_end(text, start, max_width);
            self.ui_text(
                x,
                y + line * (UI_FONT_CELL_HEIGHT + 4) * self.ui_scale(),
                &text[start..end],
                red,
                green,
                blue,
                1,
            );
            start = end;
            if start < text.len() && text[start] == b' ' {
                start += 1;
            }
            line += 1;
        }
    }

    // ------------------------=
    // FUNC: ui_text_wrapped_line_count
    // DESC: Measures the number of proportional word-wrapped lines without allocating or drawing.
    // ------------------=
    pub(super) fn ui_text_wrapped_line_count(
        &self,
        max_width: usize,
        text: &[u8],
        max_lines: usize,
    ) -> usize {
        let mut start = 0usize;
        let mut lines = 0usize;
        while start < text.len() && lines < max_lines {
            while start < text.len()
                && matches!(text[start], b' ' | b'\t' | b'\r' | b'\n')
            {
                start += 1;
            }
            if start >= text.len() {
                break;
            }
            let end = self.ui_text_wrap_line_end(text, start, max_width);
            start = end;
            if start < text.len() && text[start] == b' ' {
                start += 1;
            }
            lines += 1;
        }
        lines
    }

    // ------------------------=
    // FUNC: ui_text_wrapped_compact_clipped
    // DESC: Draws a compact wrapped line stream inside a vertical viewport while collapsing duplicate blank response rows.
    // ------------------=
    pub(super) fn ui_text_wrapped_compact_clipped(
        &mut self,
        x: usize,
        y: i32,
        max_width: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        line_height: usize,
        clip_top: usize,
        clip_bottom: usize,
    ) {
        let mut start = 0usize;
        let mut line = 0usize;
        while start < text.len() {
            while start < text.len()
                && matches!(text[start], b' ' | b'\t' | b'\r' | b'\n')
            {
                start += 1;
            }
            if start >= text.len() {
                break;
            }
            let end = self.ui_text_wrap_line_end(text, start, max_width);
            let line_y = y.saturating_add((line.saturating_mul(line_height)) as i32);
            if line_y.saturating_add(UI_FONT_CELL_HEIGHT as i32) > clip_top as i32
                && line_y < clip_bottom as i32
                && line_y >= 0
            {
                self.ui_text(
                    x,
                    line_y as usize,
                    &text[start..end],
                    red,
                    green,
                    blue,
                    1,
                );
            }
            start = end;
            if start < text.len() && text[start] == b' ' {
                start += 1;
            }
            line += 1;
        }
    }

    // ------------------------=
    // FUNC: ui_text_wrap_line_end
    // DESC: Finds the final byte for one measured word-wrapped proportional text line.
    // ------------------=
    fn ui_text_wrap_line_end(&self, text: &[u8], start: usize, max_width: usize) -> usize {
        let mut end = start + 1;
        let mut last_space = None;
        while end <= text.len() {
            if end < text.len() && matches!(text[end], b'\r' | b'\n') {
                return end;
            }
            if end < text.len() && text[end] == b' ' {
                last_space = Some(end);
            }
            if self.ui_text_width(&text[start..end], 1) > max_width {
                return last_space.unwrap_or(end.saturating_sub(1).max(start + 1));
            }
            if end == text.len() {
                break;
            }
            end += 1;
        }
        end
    }

    // ------------------------=
    // FUNC: ui_text_width
    // DESC: Measures smooth UI copy using the same proportional glyph advances used by the rasterizer.
    // ------------------=
    pub(super) fn ui_text_width(&self, text: &[u8], scale: usize) -> usize {
        self.ui_text_width_weighted(text, scale, false)
    }

    // ------------------------=
    // FUNC: ui_text_width_weighted
    // DESC: Measures Roboto UI text with the same proportional advances and pair kerning used for drawing.
    // ------------------=
    pub(super) fn ui_text_width_weighted(
        &self,
        text: &[u8],
        scale: usize,
        semibold: bool,
    ) -> usize {
        let scale = self.ui_effective_text_scale(scale);
        let font_size = FontSize::new(
            UI_FONT_NATIVE_SIZE_PX,
            UI_FONT_SIZE_PX.saturating_mul(scale),
        );
        let metrics = if semibold {
            UI_FONT_SEMIBOLD_METRICS
        } else {
            UI_FONT_METRICS
        };
        let kerning = if semibold {
            UI_FONT_SEMIBOLD_KERN
        } else {
            UI_FONT_KERN
        };
        let mut width = 0usize;
        let mut previous = None;
        for byte in text {
            if !(32..=126).contains(byte) {
                previous = None;
                continue;
            }
            width = Self::font_position_advance(
                width,
                font_size.scale_isize(Self::font_pair_adjustment(kerning, previous, *byte)),
            );
            width =
                width.saturating_add(font_size.scale_usize(metrics[*byte as usize - 32] as usize));
            previous = Some(*byte);
        }
        width
    }

    // ------------------------=
    // FUNC: ui_effective_text_scale
    // DESC: Selects a legible anti-aliased text scale for the active display while preserving title hierarchy.
    // ------------------=
    pub(super) fn ui_effective_text_scale(&self, requested: usize) -> usize {
        requested.max(1).min(3)
    }

    // ------------------------=
    // FUNC: font_pair_adjustment
    // DESC: Decodes one signed Roboto pair-kerning adjustment from a compact lookup table.
    // ------------------=
    pub(super) fn font_pair_adjustment(table: &[u8], previous: Option<u8>, current: u8) -> isize {
        let Some(previous) = previous else {
            return 0;
        };
        if !(32..=126).contains(&previous) || !(32..=126).contains(&current) {
            return 0;
        }
        table[(previous as usize - 32) * 95 + current as usize - 32] as isize - 128
    }

    // ------------------------=
    // FUNC: font_position_advance
    // DESC: Applies a signed kerning delta without allowing a text coordinate to underflow.
    // ------------------=
    pub(super) fn font_position_advance(position: usize, adjustment: isize) -> usize {
        if adjustment < 0 {
            position.saturating_sub(adjustment.unsigned_abs())
        } else {
            position.saturating_add(adjustment as usize)
        }
    }

    // ------------------------=
    // FUNC: ui_text
    // DESC: Alpha-rasterizes bundled Roboto Regular for smooth bootstrap and desktop typography.
    // ------------------=
    pub(super) fn ui_text(
        &mut self,
        x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        scale: usize,
    ) {
        self.ui_text_weighted(x, y, text, red, green, blue, scale, false);
    }

    // ------------------------=
    // FUNC: ui_text_strong
    // DESC: Renders headings and controls with bundled Roboto Medium.
    // ------------------=
    pub(super) fn ui_text_strong(
        &mut self,
        x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        scale: usize,
    ) {
        self.ui_text_weighted(x, y, text, red, green, blue, scale, true);
    }

    // ------------------------=
    // FUNC: ui_text_weighted
    // DESC: Alpha-rasterizes one of the bundled proportional UI font weights.
    // ------------------=
    pub(super) fn ui_text_weighted(
        &mut self,
        x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        scale: usize,
        semibold: bool,
    ) {
        self.ui_text_shaded(x, y, text, red, green, blue, scale, semibold, None);
    }

    // ------------------------=
    // FUNC: ui_text_shaded
    // DESC: Uses the shared glyph rasterizer with an optional horizontal grayscale tint.
    // ------------------=
    pub(super) fn ui_text_shaded(
        &mut self, x: usize, y: usize, text: &[u8], red: u8, green: u8,
        blue: u8, scale: usize, semibold: bool, shade: Option<(usize, usize)>,
    ) {
        let scale = self.ui_effective_text_scale(scale);
        let font_size = FontSize::new(
            UI_FONT_NATIVE_SIZE_PX,
            UI_FONT_SIZE_PX.saturating_mul(scale),
        );
        self.ui_text_raster(x,y,text,red,green,blue,font_size,semibold,shade);
    }

    // ------------------------=
    // FUNC: ui_text_raster
    // DESC: Rasterizes the existing native font at an explicit pixel size without changing global typography.
    // ------------------=
    pub(super) fn ui_text_raster(&mut self, mut x: usize, y: usize, text: &[u8], red: u8, green: u8,
        blue: u8, font_size: FontSize, semibold: bool, shade: Option<(usize,usize)>) {
        let atlas = if semibold {
            UI_FONT_SEMIBOLD_ATLAS
        } else {
            UI_FONT_ATLAS
        };
        let metrics = if semibold {
            UI_FONT_SEMIBOLD_METRICS
        } else {
            UI_FONT_METRICS
        };
        let kerning = if semibold {
            UI_FONT_SEMIBOLD_KERN
        } else {
            UI_FONT_KERN
        };
        let glyph_width = font_size.scale_usize(UI_FONT_CELL_WIDTH).max(1);
        let glyph_height = font_size.scale_usize(UI_FONT_CELL_HEIGHT).max(1);
        let mut previous = None;
        for byte in text {
            if !(32..=126).contains(byte) {
                previous = None;
                continue;
            }
            x = Self::font_position_advance(
                x,
                font_size.scale_isize(Self::font_pair_adjustment(kerning, previous, *byte)),
            );
            let glyph = (*byte as usize - 32) * UI_FONT_CELL_WIDTH;
            for row in 0..glyph_height {
                let source_row = font_size.source_index(row).min(UI_FONT_CELL_HEIGHT - 1);
                for column in 0..glyph_width {
                    let source_column = font_size.source_index(column).min(UI_FONT_CELL_WIDTH - 1);
                    let alpha = atlas[source_row * UI_FONT_CELL_WIDTH * 95 + glyph + source_column];
                    if alpha != 0 {
                        let (red, green, blue) = if let Some((offset, period)) = shade {
                            let gray = crate::ui::thinking::grayscale(x + column, offset, period);
                            (gray, gray, gray)
                        } else { (red, green, blue) };
                        self.blend_color(
                            (x + column) as i32,
                            (y + row) as i32,
                            red,
                            green,
                            blue,
                            alpha,
                        );
                    }
                }
            }
            x = x.saturating_add(font_size.scale_usize(metrics[*byte as usize - 32] as usize));
            previous = Some(*byte);
        }
    }

    // ------------------------=
    // FUNC: pointer_cursor
    // DESC: Draws the scaled cursor artwork at a clipped screen position.
    // ------------------=
    pub(super) fn pointer_cursor(&mut self, cursor_x: i32, cursor_y: i32) {
        let x = self.width as i32 * cursor_x / 1000;
        let y = self.height as i32 * cursor_y / 1000;
        let prefs = crate::ui::input_preferences::current();
        let scale = crate::ui::cursor::shape_scale(prefs, self.ui_scale());
        match crate::ui::text_input::pointer_shape() {
            crate::ui::text_input::PointerShape::Text => {
                self.pointer_text_cursor(x, y, scale);
                return;
            }
            crate::ui::text_input::PointerShape::ResizeNorthWestSouthEast
            | crate::ui::text_input::PointerShape::ResizeNorthEastSouthWest
            | crate::ui::text_input::PointerShape::ResizeVertical
            | crate::ui::text_input::PointerShape::ResizeHorizontal => {
                self.pointer_resize_cursor(x, y, scale, crate::ui::text_input::pointer_shape());
                return;
            }
            crate::ui::text_input::PointerShape::Default => {}
        }
        let bounds = crate::ui::cursor::bounds(x,y,self.ui_scale(),prefs,false);
        self.cursor_sprite(prefs.cursor_style as usize,bounds.x,bounds.y,bounds.width as usize);
    }

    // ------------------------=
    // FUNC: cursor_sprite
    // DESC: Paints straight-alpha cursor art identically in the gallery and at the live pointer hotspot.
    // ------------------=
    pub(super) fn cursor_sprite(&mut self, style: usize, x: i32, y: i32, size: usize) {
        for py in 0..size {for px in 0..size {
            let p=crate::ui::cursor::sample(style,px,py,size);
            self.blend_color(x+px as i32,y+py as i32,p[0],p[1],p[2],p[3]);
        }}
    }

    // ------------------------=
    // FUNC: pointer_resize_cursor
    // DESC: Draws a high-contrast double-headed pointer matching the hovered resize direction.
    // ------------------=
    fn pointer_resize_cursor(
        &mut self,
        x: i32,
        y: i32,
        scale: usize,
        shape: crate::ui::text_input::PointerShape,
    ) {
        let scale = scale.max(1) as i32;
        let (start_x, start_y, end_x, end_y) = match shape {
            crate::ui::text_input::PointerShape::ResizeHorizontal => (2, 14, 26, 14),
            crate::ui::text_input::PointerShape::ResizeVertical => (14, 2, 14, 26),
            crate::ui::text_input::PointerShape::ResizeNorthWestSouthEast => (3, 3, 25, 25),
            _ => (25, 3, 3, 25),
        };
        let start_x = x + start_x * scale;
        let start_y = y + start_y * scale;
        let end_x = x + end_x * scale;
        let end_y = y + end_y * scale;
        self.icon_line(
            start_x + scale,
            start_y + scale,
            end_x + scale,
            end_y + scale,
            (2, 8, 18),
            28 * scale as usize,
        );
        self.icon_line(
            start_x,
            start_y,
            end_x,
            end_y,
            (225, 247, 255),
            28 * scale as usize,
        );
        let arrow = 6 * scale;
        match shape {
            crate::ui::text_input::PointerShape::ResizeHorizontal => {
                self.icon_line(
                    start_x,
                    start_y,
                    start_x + arrow,
                    start_y - arrow,
                    (73, 199, 255),
                    28 * scale as usize,
                );
                self.icon_line(
                    start_x,
                    start_y,
                    start_x + arrow,
                    start_y + arrow,
                    (73, 199, 255),
                    28 * scale as usize,
                );
                self.icon_line(
                    end_x,
                    end_y,
                    end_x - arrow,
                    end_y - arrow,
                    (73, 199, 255),
                    28 * scale as usize,
                );
                self.icon_line(
                    end_x,
                    end_y,
                    end_x - arrow,
                    end_y + arrow,
                    (73, 199, 255),
                    28 * scale as usize,
                );
            }
            crate::ui::text_input::PointerShape::ResizeVertical => {
                self.icon_line(
                    start_x,
                    start_y,
                    start_x - arrow,
                    start_y + arrow,
                    (73, 199, 255),
                    28 * scale as usize,
                );
                self.icon_line(
                    start_x,
                    start_y,
                    start_x + arrow,
                    start_y + arrow,
                    (73, 199, 255),
                    28 * scale as usize,
                );
                self.icon_line(
                    end_x,
                    end_y,
                    end_x - arrow,
                    end_y - arrow,
                    (73, 199, 255),
                    28 * scale as usize,
                );
                self.icon_line(
                    end_x,
                    end_y,
                    end_x + arrow,
                    end_y - arrow,
                    (73, 199, 255),
                    28 * scale as usize,
                );
            }
            crate::ui::text_input::PointerShape::ResizeNorthWestSouthEast => {
                self.icon_line(
                    start_x,
                    start_y,
                    start_x + arrow,
                    start_y,
                    (73, 199, 255),
                    28 * scale as usize,
                );
                self.icon_line(
                    start_x,
                    start_y,
                    start_x,
                    start_y + arrow,
                    (73, 199, 255),
                    28 * scale as usize,
                );
                self.icon_line(
                    end_x,
                    end_y,
                    end_x - arrow,
                    end_y,
                    (73, 199, 255),
                    28 * scale as usize,
                );
                self.icon_line(
                    end_x,
                    end_y,
                    end_x,
                    end_y - arrow,
                    (73, 199, 255),
                    28 * scale as usize,
                );
            }
            _ => {
                self.icon_line(
                    start_x,
                    start_y,
                    start_x - arrow,
                    start_y,
                    (73, 199, 255),
                    28 * scale as usize,
                );
                self.icon_line(
                    start_x,
                    start_y,
                    start_x,
                    start_y + arrow,
                    (73, 199, 255),
                    28 * scale as usize,
                );
                self.icon_line(
                    end_x,
                    end_y,
                    end_x + arrow,
                    end_y,
                    (73, 199, 255),
                    28 * scale as usize,
                );
                self.icon_line(
                    end_x,
                    end_y,
                    end_x,
                    end_y - arrow,
                    (73, 199, 255),
                    28 * scale as usize,
                );
            }
        }
    }

    // ------------------------=
    // FUNC: pointer_text_cursor
    // DESC: Draws a high-contrast I-beam pointer centered over editable text controls.
    // ------------------=
    fn pointer_text_cursor(&mut self, x: i32, y: i32, scale: usize) {
        let scale = scale.max(1) as i32;
        let center_x = x + 7 * scale;
        let top = y + 2 * scale;
        let bottom = y + 22 * scale;
        for offset in -1..=1 {
            self.icon_line(
                center_x + offset,
                top,
                center_x + offset,
                bottom,
                (5, 14, 24),
                28 * scale as usize,
            );
        }
        self.icon_line(
            center_x - 5 * scale,
            top,
            center_x + 5 * scale,
            top,
            (5, 14, 24),
            28 * scale as usize,
        );
        self.icon_line(
            center_x - 5 * scale,
            bottom,
            center_x + 5 * scale,
            bottom,
            (5, 14, 24),
            28 * scale as usize,
        );
        self.icon_line(
            center_x,
            top,
            center_x,
            bottom,
            (226, 247, 255),
            28 * scale as usize,
        );
        self.icon_line(
            center_x - 4 * scale,
            top,
            center_x + 4 * scale,
            top,
            (226, 247, 255),
            28 * scale as usize,
        );
        self.icon_line(
            center_x - 4 * scale,
            bottom,
            center_x + 4 * scale,
            bottom,
            (226, 247, 255),
            28 * scale as usize,
        );
    }

    // ------------------------=
    // FUNC: fill_rect
    // DESC: Fills a clipped rectangle with a solid RGB color.
    // ------------------=
    pub(super) fn fill_rect(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        red: u8,
        green: u8,
        blue: u8,
    ) {
        let Some(region) = self.clipped_render_region(left, top, width, height) else {
            return;
        };
        let color = if self.format == 0 {
            red as u32 | (green as u32) << 8 | (blue as u32) << 16
        } else {
            blue as u32 | (green as u32) << 8 | (red as u32) << 16
        };
        let color = color
            | if self.recording_surface {
                0xff000000
            } else {
                0
            };
        for y in region.top..region.bottom {
            for x in region.left..region.right {
                unsafe {
                    write_volatile(self.buffer.add(y * self.stride + x), color);
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

    // ------------------------=
    // FUNC: fill_rect_alpha
    // DESC: Applies a translucent RGB fill while retaining the scene beneath a panel.
    // ------------------=
    pub(super) fn fill_rect_alpha(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        red: u8,
        green: u8,
        blue: u8,
        alpha: u8,
    ) {
        let Some(region) = self.clipped_render_region(left, top, width, height) else {
            return;
        };
        for y in region.top..region.bottom {
            for x in region.left..region.right {
                self.blend_color_unchecked(x, y, red, green, blue, alpha);
            }
        }
        self.mark_dirty_rect(
            region.left,
            region.top,
            region.right - region.left,
            region.bottom - region.top,
        );
    }

    // ------------------------=
    // FUNC: fill_rounded_rect_alpha
    // DESC: Applies a clipped translucent fill with geometrically smooth rounded corners.
    // ------------------=
    pub(super) fn fill_rounded_rect_alpha(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        radius: usize,
        red: u8,
        green: u8,
        blue: u8,
        alpha: u8,
    ) {
        let radius = radius.min(width / 2).min(height / 2).max(1);
        let Some(region) = self.clipped_render_region(left, top, width, height) else {
            return;
        };
        let radius_squared = (radius * radius) as i64;
        for y in region.top..region.bottom {
            if y & 31 == 0 { crate::ui::input_capture::poll(); }
            for x in region.left..region.right {
                let dx = if x < left + radius {
                    left + radius - x
                } else if x >= left + width.saturating_sub(radius) {
                    x.saturating_sub(left + width.saturating_sub(radius) - 1)
                } else {
                    0
                };
                let dy = if y < top + radius {
                    top + radius - y
                } else if y >= top + height.saturating_sub(radius) {
                    y.saturating_sub(top + height.saturating_sub(radius) - 1)
                } else {
                    0
                };
                if dx == 0 || dy == 0 || (dx * dx + dy * dy) as i64 <= radius_squared {
                    self.blend_color_unchecked(x, y, red, green, blue, alpha);
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

    // ------------------------=
    // FUNC: outline_rounded_rect
    // DESC: Draws a one-pixel rounded border while preserving transparent panel corners.
    // ------------------=
    pub(super) fn outline_rounded_rect(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        radius: usize,
        red: u8,
        green: u8,
        blue: u8,
    ) {
        let radius = radius.min(width / 2).min(height / 2).max(1);
        let outer = (radius * radius) as i64;
        let inner_radius = radius.saturating_sub(1);
        let inner = (inner_radius * inner_radius) as i64;
        let Some(region) = self.clipped_render_region(left, top, width, height) else {
            return;
        };
        for y in region.top..region.bottom {
            for x in region.left..region.right {
                let edge = x == left || y == top || x + 1 == left + width || y + 1 == top + height;
                let dx = if x < left + radius {
                    left + radius - x
                } else if x >= left + width.saturating_sub(radius) {
                    x.saturating_sub(left + width.saturating_sub(radius) - 1)
                } else {
                    0
                };
                let dy = if y < top + radius {
                    top + radius - y
                } else if y >= top + height.saturating_sub(radius) {
                    y.saturating_sub(top + height.saturating_sub(radius) - 1)
                } else {
                    0
                };
                let corner_distance = (dx * dx + dy * dy) as i64;
                let corner_edge =
                    dx > 0 && dy > 0 && corner_distance <= outer && corner_distance >= inner;
                if (edge && (dx == 0 || dy == 0)) || corner_edge {
                    self.pixel(x as i32, y as i32, red, green, blue);
                }
            }
        }
    }

    // ------------------------=
    // FUNC: outline_rounded_rect_alpha
    // DESC: Draws a translucent one-pixel rounded border for template-authored appearance.
    // ------------------=
    pub(super) fn outline_rounded_rect_alpha(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        radius: usize,
        red: u8,
        green: u8,
        blue: u8,
        alpha: u8,
    ) {
        let radius = radius.min(width / 2).min(height / 2).max(1);
        let outer = (radius * radius) as i64;
        let inner_radius = radius.saturating_sub(1);
        let inner = (inner_radius * inner_radius) as i64;
        let Some(region) = self.clipped_render_region(left, top, width, height) else {
            return;
        };
        for y in region.top..region.bottom {
            for x in region.left..region.right {
                let edge = x == left || y == top || x + 1 == left + width || y + 1 == top + height;
                let dx = if x < left + radius {
                    left + radius - x
                } else if x >= left + width.saturating_sub(radius) {
                    x.saturating_sub(left + width.saturating_sub(radius) - 1)
                } else {
                    0
                };
                let dy = if y < top + radius {
                    top + radius - y
                } else if y >= top + height.saturating_sub(radius) {
                    y.saturating_sub(top + height.saturating_sub(radius) - 1)
                } else {
                    0
                };
                let distance = (dx * dx + dy * dy) as i64;
                if (edge && (dx == 0 || dy == 0))
                    || (dx > 0 && dy > 0 && distance <= outer && distance >= inner)
                {
                    self.blend_color(x as i32, y as i32, red, green, blue, alpha);
                }
            }
        }
    }

    // ------------------------=
    // FUNC: outline_rect
    // DESC: Draws a one-pixel rectangular border in the requested RGB color.
    // ------------------=
    pub(super) fn outline_rect(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        red: u8,
        green: u8,
        blue: u8,
    ) {
        for x in left..(left + width).min(self.width) {
            self.pixel(x as i32, top as i32, red, green, blue);
            self.pixel(
                x as i32,
                (top + height.saturating_sub(1)) as i32,
                red,
                green,
                blue,
            );
        }
        for y in top..(top + height).min(self.height) {
            self.pixel(left as i32, y as i32, red, green, blue);
            self.pixel(
                (left + width.saturating_sub(1)) as i32,
                y as i32,
                red,
                green,
                blue,
            );
        }
    }

    // ------------------------=
    // FUNC: line
    // DESC: Rasterizes a clipped straight line for installer icons and accents.
    // ------------------=
    pub(super) fn line(
        &mut self,
        mut x0: i32,
        mut y0: i32,
        x1: i32,
        y1: i32,
        red: u8,
        green: u8,
        blue: u8,
    ) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        loop {
            self.pixel(x0, y0, red, green, blue);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let twice = error * 2;
            if twice >= dy {
                error += dy;
                x0 += sx;
            }
            if twice <= dx {
                error += dx;
                y0 += sy;
            }
        }
    }

    // ------------------------=
    // FUNC: icon_line
    // DESC: Draws a scale-aware anti-aliased-looking icon stroke that remains legible after VM display scaling.
    // ------------------=
    pub(super) fn icon_line(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        color: (u8, u8, u8),
        icon_size: usize,
    ) {
        let radius = (icon_size / 18).clamp(1, 3) as i32;
        for offset in -radius..=radius {
            self.line(x0 + offset, y0, x1 + offset, y1, color.0, color.1, color.2);
            self.line(x0, y0 + offset, x1, y1 + offset, color.0, color.1, color.2);
        }
    }

    // ------------------------=
    // FUNC: icon_circle
    // DESC: Rasterizes a scale-aware circular vector stroke for semantic skin icons.
    // ------------------=
    pub(super) fn icon_circle(
        &mut self,
        center_x: i32,
        center_y: i32,
        radius: i32,
        color: (u8, u8, u8),
        icon_size: usize,
    ) {
        let mut x = radius.max(1);
        let mut y = 0i32;
        let mut error = 1 - x;
        let weight = (icon_size / 20).clamp(1, 2) as i32;
        while x >= y {
            for offset in -weight..=weight {
                for (px, py) in [
                    (x, y),
                    (y, x),
                    (-y, x),
                    (-x, y),
                    (-x, -y),
                    (-y, -x),
                    (y, -x),
                    (x, -y),
                ] {
                    self.blend_color(
                        center_x + px + offset,
                        center_y + py,
                        color.0,
                        color.1,
                        color.2,
                        235,
                    );
                }
            }
            y += 1;
            if error < 0 {
                error += 2 * y + 1;
            } else {
                x -= 1;
                error += 2 * (y - x) + 1;
            }
        }
    }

    // ------------------------=
    // FUNC: text
    // DESC: Draws text using the standard interface scale and bitmap font.
    // ------------------=
    pub(super) fn text(&mut self, x: usize, y: usize, text: &[u8], r: u8, g: u8, b: u8) {
        self.installer_text(x, y, text, r, g, b);
    }

    // ------------------------=
    // FUNC: text_scaled
    // DESC: Rasterizes glyphs at an explicit integer scale with newline support.
    // ------------------=
    pub(super) fn text_scaled(
        &mut self,
        mut x: usize,
        y: usize,
        text: &[u8],
        r: u8,
        g: u8,
        b: u8,
        scale: usize,
        embolden: bool,
    ) {
        for &character in text {
            let glyph = glyph(character);
            for (row, bits) in glyph.iter().enumerate() {
                for column in 0..8 {
                    if bits & (1 << (7 - column)) != 0 {
                        for py in 0..scale {
                            for px in 0..scale {
                                self.pixel(
                                    (x + column * scale + px) as i32,
                                    (y + row * scale + py) as i32,
                                    r,
                                    g,
                                    b,
                                );
                            }
                        }
                        // At compact resolutions a one-pixel stem becomes faint
                        // after VirtualBox scales the guest surface. Give 1x text
                        // a second horizontal sample without changing its metrics.
                        if embolden && scale == 1 {
                            self.pixel((x + column + 1) as i32, (y + row) as i32, r, g, b);
                        }
                    }
                }
            }
            x += 9 * scale;
        }
    }

    // ------------------------=
    // FUNC: text_scaled_to_height
    // DESC: Resamples the legacy 8x8 bitmap font to an exact pixel height.
    // ------------------=
    pub(super) fn text_scaled_to_height(
        &mut self,
        mut x: usize,
        y: usize,
        text: &[u8],
        r: u8,
        g: u8,
        b: u8,
        height_px: usize,
        embolden: bool,
    ) {
        let height_px = height_px.max(1);
        // Preserve the source font's square 8x8 aspect ratio.
        let width_px = height_px;
        // Source glyph advance is 9 pixels (8 drawn + 1 spacing). Scale that
        // proportionally and round up so adjacent glyphs never collide.
        let advance_px = (9usize.saturating_mul(height_px).saturating_add(7)) / 8;

        for &character in text {
            let glyph = glyph(character);
            for destination_y in 0..height_px {
                let source_y = destination_y.saturating_mul(8) / height_px;
                let bits = glyph[source_y.min(7)];
                for destination_x in 0..width_px {
                    let source_x = destination_x.saturating_mul(8) / width_px;
                    if bits & (1 << (7 - source_x.min(7))) != 0 {
                        self.pixel(
                            (x + destination_x) as i32,
                            (y + destination_y) as i32,
                            r,
                            g,
                            b,
                        );
                        if embolden {
                            self.pixel(
                                (x + destination_x + 1) as i32,
                                (y + destination_y) as i32,
                                r,
                                g,
                                b,
                            );
                        }
                    }
                }
            }
            x = x.saturating_add(advance_px);
        }
    }

    // The bundled atlas already contains 16-pixel anti-aliased glyphs. Keep its
    // native scale through ordinary 720p, 1080p, and square HiDPI VM modes so
    // text metrics remain proportional to the component geometry. Scale the
    // complete UI only on true 1440p-or-larger surfaces.
    // ------------------------=
    // FUNC: ui_scale
    // DESC: Selects a readable integer UI scale from the current display resolution.
    // ------------------=
    pub(super) fn ui_scale(&self) -> usize {
        if self.width >= 2560 && self.height >= 1440 {
            2
        } else {
            1
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_bitmap_cover_rect
    // DESC: Samples a bitmap with cover scaling and paints only the requested rectangle.
    // ------------------=
    pub(super) fn paint_bitmap_cover_rect(
        &mut self,
        bitmap: &[u8],
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        if bitmap.len() < 54 || &bitmap[0..2] != b"BM" {
            return;
        }
        let offset = le32(bitmap, 10) as usize;
        let source_width = le32(bitmap, 18) as usize;
        let signed_height = le32(bitmap, 22) as i32;
        let source_height = signed_height.unsigned_abs() as usize;
        if le16(bitmap, 28) != 24 || source_width == 0 || source_height == 0 {
            return;
        }
        let row_bytes = (source_width * 3 + 3) & !3;
        let (crop_x, crop_y, crop_width, crop_height) =
            if source_width * self.height > source_height * self.width {
                let width = source_height * self.width / self.height;
                ((source_width - width) / 2, 0, width, source_height)
            } else {
                let height = source_width * self.height / self.width;
                (0, (source_height - height) / 2, source_width, height)
            };
        let Some(region) = self.clipped_render_region(left, top, width, height) else {
            return;
        };
        for y in region.top..region.bottom {
            let logical_y = crop_y + y * crop_height / self.height;
            let source_y = if signed_height < 0 {
                logical_y
            } else {
                source_height - 1 - logical_y
            };
            for x in region.left..region.right {
                let source_x = crop_x + x * crop_width / self.width;
                let index = offset + source_y * row_bytes + source_x * 3;
                if index + 2 >= bitmap.len() {
                    return;
                }
                self.write_rgb_unchecked(
                    x,
                    y,
                    bitmap[index + 2],
                    bitmap[index + 1],
                    bitmap[index],
                );
            }
        }
        self.mark_dirty_rect(
            region.left,
            region.top,
            region.right - region.left,
            region.bottom - region.top,
        );
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_bitmap_stretch_rect
    // DESC: Restores a clipped rectangle from a bitmap stretched across the complete framebuffer.
    // ------------------=
    pub(super) fn paint_bitmap_stretch_rect(
        &mut self,
        bitmap: &[u8],
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        if bitmap.len() < 54 || &bitmap[0..2] != b"BM" || le16(bitmap, 28) != 24 {
            return;
        }
        let offset = le32(bitmap, 10) as usize;
        let source_width = le32(bitmap, 18) as usize;
        let signed_height = le32(bitmap, 22) as i32;
        let source_height = signed_height.unsigned_abs() as usize;
        if source_width == 0 || source_height == 0 || self.width == 0 || self.height == 0 {
            return;
        }
        let row_bytes = (source_width * 3 + 3) & !3;
        for y in top..(top + height).min(self.height) {
            let logical_y = y * source_height / self.height;
            let source_y = if signed_height < 0 {
                logical_y
            } else {
                source_height - 1 - logical_y
            };
            for x in left..(left + width).min(self.width) {
                let source_x = x * source_width / self.width;
                let index = offset + source_y * row_bytes + source_x * 3;
                if index + 2 >= bitmap.len() {
                    return;
                }
                self.pixel(
                    x as i32,
                    y as i32,
                    bitmap[index + 2],
                    bitmap[index + 1],
                    bitmap[index],
                );
            }
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_bitmap_fit_rect
    // DESC: Samples a bitmap with aspect-fit scaling inside the requested rectangle.
    // ------------------=
    pub(super) fn paint_bitmap_fit_rect(
        &mut self,
        bitmap: &[u8],
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        if bitmap.len() < 54 || &bitmap[0..2] != b"BM" || le16(bitmap, 28) != 24 {
            return;
        }
        let offset = le32(bitmap, 10) as usize;
        let source_width = le32(bitmap, 18) as usize;
        let signed_height = le32(bitmap, 22) as i32;
        let source_height = signed_height.unsigned_abs() as usize;
        if source_width == 0 || source_height == 0 || width == 0 || height == 0 {
            return;
        }
        let row_bytes = (source_width * 3 + 3) & !3;
        let Some(region) = self.clipped_render_region(left, top, width, height) else {
            return;
        };
        for y in region.top - top..region.bottom - top {
            let sy = y * source_height / height;
            let source_y = if signed_height < 0 {
                sy
            } else {
                source_height - 1 - sy
            };
            for x in region.left - left..region.right - left {
                let sx = x * source_width / width;
                let index = offset + source_y * row_bytes + sx * 3;
                if index + 2 >= bitmap.len() {
                    return;
                }
                self.pixel(
                    (left + x) as i32,
                    (top + y) as i32,
                    bitmap[index + 2],
                    bitmap[index + 1],
                    bitmap[index],
                );
            }
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_bitmap_template_rect
    // DESC: Paints a cropped template image with role-selected aspect fit and authored layer opacity.
    // ------------------=
    pub(super) fn paint_bitmap_template_rect(
        &mut self,
        bitmap: &[u8],
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        crop: [u8; 4],
        opacity: u8,
        aspect_fill: bool,
    ) {
        let Some(bitmap) = crate::ui::bitmap::RuntimeBitmap::parse(bitmap) else {
            return;
        };
        if width == 0 || height == 0 {
            return;
        }
        let Some(placement) = bitmap.placement(width, height, crop, aspect_fill) else {
            return;
        };
        let draw_left = left.saturating_add(placement.destination_left);
        let draw_top = top.saturating_add(placement.destination_top);
        for y in 0..placement
            .destination_height
            .min(self.height.saturating_sub(draw_top))
        {
            let source_y =
                placement.source_top + y * placement.source_height / placement.destination_height;
            for x in 0..placement
                .destination_width
                .min(self.width.saturating_sub(draw_left))
            {
                let source_x = placement.source_left
                    + x * placement.source_width / placement.destination_width;
                let Some([red, green, blue, source_alpha]) = bitmap.rgba(source_x, source_y) else {
                    return;
                };
                let alpha = (source_alpha as u16 * opacity as u16 / 255) as u8;
                self.blend_color(
                    (draw_left + x) as i32,
                    (draw_top + y) as i32,
                    red,
                    green,
                    blue,
                    alpha,
                );
            }
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_bitmap_fit_rect_inset
    // DESC: Paints a bitmap while excluding an equal source-edge inset from all four sides.
    // ------------------=
    pub(super) fn paint_bitmap_fit_rect_inset(
        &mut self,
        bitmap: &[u8],
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        source_inset: usize,
    ) {
        if bitmap.len() < 54 || &bitmap[0..2] != b"BM" || le16(bitmap, 28) != 24 {
            return;
        }
        let offset = le32(bitmap, 10) as usize;
        let source_width = le32(bitmap, 18) as usize;
        let signed_height = le32(bitmap, 22) as i32;
        let source_height = signed_height.unsigned_abs() as usize;
        let inset = source_inset
            .min(source_width.saturating_sub(1) / 2)
            .min(source_height.saturating_sub(1) / 2);
        let sampled_width = source_width.saturating_sub(inset * 2);
        let sampled_height = source_height.saturating_sub(inset * 2);
        if sampled_width == 0 || sampled_height == 0 || width == 0 || height == 0 {
            return;
        }
        let row_bytes = (source_width * 3 + 3) & !3;
        let Some(region) = self.clipped_render_region(left, top, width, height) else {
            return;
        };
        for y in region.top - top..region.bottom - top {
            let sy = inset + y * sampled_height / height;
            let source_y = if signed_height < 0 {
                sy
            } else {
                source_height - 1 - sy
            };
            for x in region.left - left..region.right - left {
                let sx = inset + x * sampled_width / width;
                let index = offset + source_y * row_bytes + sx * 3;
                if index + 2 >= bitmap.len() {
                    return;
                }
                self.pixel(
                    (left + x) as i32,
                    (top + y) as i32,
                    bitmap[index + 2],
                    bitmap[index + 1],
                    bitmap[index],
                );
            }
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_bitmap_alpha_fit_rect
    // DESC: Aspect-fits a 32-bit BGRA bitmap and alpha-blends it over the existing framebuffer.
    // ------------------=
    pub(super) fn paint_bitmap_alpha_fit_rect(
        &mut self,
        bitmap: &[u8],
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        self.paint_bitmap_alpha_fit_rect_opacity(bitmap, left, top, width, height, 255);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_bitmap_alpha_fit_rect_opacity
    // DESC: Alpha-blends a 32-bit BGRA bitmap through one additional bounded layer opacity.
    // ------------------=
    pub(super) fn paint_bitmap_alpha_fit_rect_opacity(
        &mut self,
        bitmap: &[u8],
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        opacity: u8,
    ) {
        if bitmap.len() < 54 || &bitmap[0..2] != b"BM" || le16(bitmap, 28) != 32 {
            return;
        }
        let offset = le32(bitmap, 10) as usize;
        let source_width = le32(bitmap, 18) as usize;
        let signed_height = le32(bitmap, 22) as i32;
        let source_height = signed_height.unsigned_abs() as usize;
        if source_width == 0 || source_height == 0 || width == 0 || height == 0 {
            return;
        }
        let row_bytes = source_width * 4;
        let Some(region) = self.clipped_render_region(left, top, width, height) else {
            return;
        };
        for y in region.top - top..region.bottom - top {
            let sy = y * source_height / height;
            let source_y = if signed_height < 0 {
                sy
            } else {
                source_height - 1 - sy
            };
            for x in region.left - left..region.right - left {
                let sx = x * source_width / width;
                let index = offset + source_y * row_bytes + sx * 4;
                if index + 3 >= bitmap.len() {
                    return;
                }
                let alpha = (bitmap[index + 3] as u16 * opacity as u16 / 255) as u8;
                if alpha != 0 {
                    self.blend_color_unchecked(
                        left + x,
                        top + y,
                        bitmap[index + 2],
                        bitmap[index + 1],
                        bitmap[index],
                        alpha,
                    );
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

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_bitmap_alpha_atlas_cell
    // DESC: Alpha-blends one semantic cell from a uniformly divided 32-bit BGRA icon atlas.
    // ------------------=
    pub(super) fn paint_bitmap_alpha_atlas_cell(
        &mut self,
        bitmap: &[u8],
        columns: usize,
        rows: usize,
        cell: usize,
        left: usize,
        top: usize,
        size: usize,
    ) -> bool {
        if bitmap.len() < 54
            || &bitmap[0..2] != b"BM"
            || le16(bitmap, 28) != 32
            || columns == 0
            || rows == 0
            || cell >= columns.saturating_mul(rows)
            || size == 0
        {
            return false;
        }
        let offset = le32(bitmap, 10) as usize;
        let source_width = le32(bitmap, 18) as usize;
        let signed_height = le32(bitmap, 22) as i32;
        let source_height = signed_height.unsigned_abs() as usize;
        let cell_width = source_width / columns;
        let cell_height = source_height / rows;
        if cell_width == 0 || cell_height == 0 {
            return false;
        }
        let cell_x = (cell % columns) * cell_width;
        let cell_y = (cell / columns) * cell_height;
        let row_bytes = source_width * 4;
        // Average premultiplied samples when reducing an icon; transparent RGB
        // must not introduce dark fringes. Work is bounded to 16 samples/pixel.
        let samples_x = (cell_width / size).clamp(1, 4);
        let samples_y = (cell_height / size).clamp(1, 4);
        for y in 0..size.min(self.height.saturating_sub(top)) {
            for x in 0..size.min(self.width.saturating_sub(left)) {
                let mut channels = [0u32; 3];
                let mut coverage = 0u32;
                for sy in 0..samples_y {
                    let atlas_y = cell_y + ((y * samples_y * 2 + sy * 2 + 1)
                        * cell_height / (size * samples_y * 2)).min(cell_height - 1);
                    let source_y = if signed_height < 0 { atlas_y } else { source_height - 1 - atlas_y };
                    for sx in 0..samples_x {
                        let source_x = cell_x + ((x * samples_x * 2 + sx * 2 + 1)
                            * cell_width / (size * samples_x * 2)).min(cell_width - 1);
                        let index = offset + source_y * row_bytes + source_x * 4;
                        if index + 3 >= bitmap.len() { return false; }
                        let alpha = bitmap[index + 3] as u32;
                        coverage += alpha;
                        for channel in 0..3 {
                            channels[channel] += bitmap[index + channel] as u32 * alpha;
                        }
                    }
                }
                let alpha = (coverage / (samples_x * samples_y) as u32) as u8;
                if alpha != 0 {
                    self.blend_color(
                        (left + x) as i32,
                        (top + y) as i32,
                        (channels[2] / coverage) as u8,
                        (channels[1] / coverage) as u8,
                        (channels[0] / coverage) as u8,
                        alpha,
                    );
                }
            }
        }
        true
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_bitmap_cover_box
    // DESC: Aspect-crops a bitmap into a fixed destination box without painting beyond its bounds.
    // ------------------=
    pub(super) fn paint_bitmap_cover_box(
        &mut self,
        bitmap: &[u8],
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        if bitmap.len() < 54 || &bitmap[0..2] != b"BM" || le16(bitmap, 28) != 24 {
            return;
        }
        let offset = le32(bitmap, 10) as usize;
        let source_width = le32(bitmap, 18) as usize;
        let signed_height = le32(bitmap, 22) as i32;
        let source_height = signed_height.unsigned_abs() as usize;
        if source_width == 0 || source_height == 0 || width == 0 || height == 0 {
            return;
        }
        let row_bytes = (source_width * 3 + 3) & !3;
        let (crop_x, crop_y, crop_width, crop_height) =
            if source_width * height > source_height * width {
                let cropped_width = source_height * width / height;
                (
                    (source_width.saturating_sub(cropped_width)) / 2,
                    0,
                    cropped_width,
                    source_height,
                )
            } else {
                let cropped_height = source_width * height / width;
                (
                    0,
                    (source_height.saturating_sub(cropped_height)) / 2,
                    source_width,
                    cropped_height,
                )
            };
        let Some(region) = self.clipped_render_region(left, top, width, height) else {
            return;
        };
        for y in region.top - top..region.bottom - top {
            let sy = crop_y + y * crop_height / height;
            let source_y = if signed_height < 0 {
                sy
            } else {
                source_height - 1 - sy
            };
            for x in region.left - left..region.right - left {
                let sx = crop_x + x * crop_width / width;
                let index = offset + source_y * row_bytes + sx * 3;
                if index + 2 >= bitmap.len() {
                    return;
                }
                self.pixel(
                    (left + x) as i32,
                    (top + y) as i32,
                    bitmap[index + 2],
                    bitmap[index + 1],
                    bitmap[index],
                );
            }
        }
    }

    #[cfg(target_arch = "x86")]
    // ------------------------=
    // FUNC: paint_bitmap_cover_box
    // DESC: Supplies a bounded dark hero fallback when legacy x86 omits high-resolution bitmap decoding.
    // ------------------=
    pub(super) fn paint_bitmap_cover_box(
        &mut self,
        _bitmap: &[u8],
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        self.fill_rect(left, top, width, height, 2, 10, 18);
    }
}

// ------------------------=
// FUNC: infinity_point
// DESC: Looks up a normalized point along the closed infinity animation path.
// ------------------=
pub(super) fn infinity_point(phase: usize) -> (i32, i32) {
    let step = (phase / 4) % 96;
    let fraction = (phase % 4) as i32;
    let (ax, ay) = INFINITY_PATH[step];
    let (bx, by) = INFINITY_PATH[step + 1];
    (ax + (bx - ax) * fraction / 4, ay + (by - ay) * fraction / 4)
}
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: le16
// DESC: Decodes a little-endian 16-bit integer from bitmap bytes.
// ------------------=
pub(super) fn le16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: le32
// DESC: Decodes a little-endian 32-bit integer from bitmap bytes.
// ------------------=
pub(super) fn le32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

// ------------------------=
// FUNC: glyph
// DESC: Returns the eight-row bitmap glyph used by the built-in ASCII font.
// ------------------=
pub(super) fn glyph(c: u8) -> [u8; 8] {
    match c {
        b'A' => [0x18, 0x24, 0x42, 0x7e, 0x42, 0x42, 0x42, 0],
        b'B' => [0x7c, 0x42, 0x42, 0x7c, 0x42, 0x42, 0x7c, 0],
        b'C' => [0x3c, 0x42, 0x40, 0x40, 0x40, 0x42, 0x3c, 0],
        b'D' => [0x78, 0x44, 0x42, 0x42, 0x42, 0x44, 0x78, 0],
        b'E' => [0x7e, 0x40, 0x40, 0x7c, 0x40, 0x40, 0x7e, 0],
        b'F' => [0x7e, 0x40, 0x40, 0x7c, 0x40, 0x40, 0x40, 0],
        b'G' => [0x3c, 0x42, 0x40, 0x4e, 0x42, 0x42, 0x3c, 0],
        b'H' => [0x42, 0x42, 0x42, 0x7e, 0x42, 0x42, 0x42, 0],
        b'I' => [0x7e, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7e, 0],
        b'J' => [0x0e, 0x04, 0x04, 0x04, 0x44, 0x44, 0x38, 0],
        b'K' => [0x42, 0x44, 0x48, 0x70, 0x48, 0x44, 0x42, 0],
        b'L' => [0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x7e, 0],
        b'M' => [0x42, 0x66, 0x5a, 0x5a, 0x42, 0x42, 0x42, 0],
        b'N' => [0x42, 0x62, 0x52, 0x4a, 0x46, 0x42, 0x42, 0],
        b'O' => [0x3c, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3c, 0],
        b'P' => [0x7c, 0x42, 0x42, 0x7c, 0x40, 0x40, 0x40, 0],
        b'Q' => [0x3c, 0x42, 0x42, 0x42, 0x4a, 0x44, 0x3a, 0],
        b'R' => [0x7c, 0x42, 0x42, 0x7c, 0x48, 0x44, 0x42, 0],
        b'S' => [0x3c, 0x42, 0x40, 0x3c, 0x02, 0x42, 0x3c, 0],
        b'T' => [0x7e, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0],
        b'U' => [0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3c, 0],
        b'V' => [0x42, 0x42, 0x42, 0x42, 0x24, 0x24, 0x18, 0],
        b'W' => [0x42, 0x42, 0x42, 0x5a, 0x5a, 0x66, 0x42, 0],
        b'X' => [0x42, 0x24, 0x18, 0x18, 0x18, 0x24, 0x42, 0],
        b'Y' => [0x42, 0x24, 0x18, 0x18, 0x18, 0x18, 0x18, 0],
        b'Z' => [0x7e, 0x04, 0x08, 0x10, 0x20, 0x40, 0x7e, 0],
        b'0' => [0x3c, 0x42, 0x46, 0x4a, 0x52, 0x62, 0x3c, 0],
        b'1' => [0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x7e, 0],
        b'2' => [0x3c, 0x42, 0x02, 0x0c, 0x30, 0x40, 0x7e, 0],
        b'3' => [0x7c, 0x02, 0x02, 0x3c, 0x02, 0x02, 0x7c, 0],
        b'4' => [0x0c, 0x14, 0x24, 0x44, 0x7e, 0x04, 0x04, 0],
        b'5' => [0x7e, 0x40, 0x40, 0x7c, 0x02, 0x02, 0x7c, 0],
        b'6' => [0x3c, 0x40, 0x40, 0x7c, 0x42, 0x42, 0x3c, 0],
        b'7' => [0x7e, 0x02, 0x04, 0x08, 0x10, 0x10, 0x10, 0],
        b'8' => [0x3c, 0x42, 0x42, 0x3c, 0x42, 0x42, 0x3c, 0],
        b'9' => [0x3c, 0x42, 0x42, 0x3e, 0x02, 0x02, 0x3c, 0],
        b' ' => [0; 8],
        b'.' => [0, 0, 0, 0, 0, 0x18, 0x18, 0],
        b'-' => [0, 0, 0, 0x7e, 0, 0, 0, 0],
        b'=' => [0, 0, 0x7e, 0, 0x7e, 0, 0, 0],
        b'+' => [0, 0x18, 0x18, 0x7e, 0x18, 0x18, 0, 0],
        b'>' => [0x40, 0x20, 0x10, 0x08, 0x10, 0x20, 0x40, 0],
        b'_' => [0, 0, 0, 0, 0, 0, 0x7e, 0],
        b'!' => [0x18, 0x18, 0x18, 0x18, 0x18, 0, 0x18, 0],
        b':' => [0, 0x18, 0x18, 0, 0x18, 0x18, 0, 0],
        b'[' => [0x3c, 0x20, 0x20, 0x20, 0x20, 0x20, 0x3c, 0],
        b']' => [0x3c, 0x04, 0x04, 0x04, 0x04, 0x04, 0x3c, 0],
        b'/' => [0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0, 0],
        b'\\' => [0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0, 0],
        b'|' => [0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0],
        b'<' => [0x04, 0x08, 0x10, 0x20, 0x10, 0x08, 0x04, 0],
        b',' => [0, 0, 0, 0, 0, 0x18, 0x18, 0x10],
        b'\'' => [0x18, 0x18, 0x10, 0, 0, 0, 0, 0],
        b'?' => [0x3c, 0x42, 0x02, 0x0c, 0x10, 0, 0x10, 0],
        b'(' => [0x0c, 0x10, 0x20, 0x20, 0x20, 0x10, 0x0c, 0],
        b')' => [0x30, 0x08, 0x04, 0x04, 0x04, 0x08, 0x30, 0],
        b'#' => [0x24, 0x24, 0x7e, 0x24, 0x7e, 0x24, 0x24, 0],
        b'*' => [0, 0x42, 0x24, 0x7e, 0x24, 0x42, 0, 0],
        b'a'..=b'z' => glyph(c - 32),
        _ => [0x7e, 0x42, 0x5a, 0x5a, 0x42, 0x42, 0x7e, 0],
    }
}
