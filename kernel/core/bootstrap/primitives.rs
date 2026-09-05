//! Shared framebuffer pixels, blending, geometry, bitmap, icon, and font primitives.

use core::ptr::{read_volatile, write_volatile};

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
        unsafe { write_volatile(self.buffer.add(y * self.stride + x), value) }
        self.mark_dirty_rect(x, y, 1, 1);
    }

    // ------------------------=
    // FUNC: pixel
    // DESC: Writes one clipped RGB pixel using the firmware-provided channel format.
    // ------------------=
    pub(super) fn pixel(&mut self, x: i32, y: i32, red: u8, green: u8, blue: u8) {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return;
        }
        let color = if self.format == 0 {
            red as u32 | (green as u32) << 8 | (blue as u32) << 16
        } else {
            blue as u32 | (green as u32) << 8 | (red as u32) << 16
        };
        unsafe {
            write_volatile(
                self.buffer.add(y as usize * self.stride + x as usize),
                color,
            );
        }
        self.mark_dirty_rect(x as usize, y as usize, 1, 1);
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
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return;
        }
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
        self.pixel(
            x,
            y,
            mix(red, target_red),
            mix(green, target_green),
            mix(blue, target_blue),
        );
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
            while start < text.len() && text[start] == b' ' {
                start += 1;
            }
            if start >= text.len() {
                break;
            }
            let mut end = start + 1;
            let mut last_space = None;
            while end <= text.len() {
                if end < text.len() && text[end] == b' ' {
                    last_space = Some(end);
                }
                if self.ui_text_width(&text[start..end], 1) > max_width {
                    end = last_space.unwrap_or(end.saturating_sub(1).max(start + 1));
                    break;
                }
                if end == text.len() {
                    break;
                }
                end += 1;
            }
            self.ui_text(
                x,
                y + line * (UI_FONT_CELL_HEIGHT + 4) * self.ui_scale(),
                &text[start..end],
                red,
                green,
                blue,
                1,
            );
            start = end.saturating_add((end < text.len() && text[end] == b' ') as usize);
            line += 1;
        }
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
        mut x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        scale: usize,
        semibold: bool,
    ) {
        let scale = self.ui_effective_text_scale(scale);
        let font_size = FontSize::new(
            UI_FONT_NATIVE_SIZE_PX,
            UI_FONT_SIZE_PX.saturating_mul(scale),
        );
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
        let scale = self.ui_scale();
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        {
            let offset = le32(CURSOR_BMP, 10) as usize;
            let source_width = le32(CURSOR_BMP, 18) as usize;
            let signed_height = le32(CURSOR_BMP, 22) as i32;
            let source_height = signed_height.unsigned_abs() as usize;
            let size = 28 * scale;
            for py in 0..size {
                for px in 0..size {
                    let sx = px * source_width / size;
                    let sy = py * source_height / size;
                    let source_y = if signed_height < 0 {
                        sy
                    } else {
                        source_height - 1 - sy
                    };
                    let index = offset + (source_y * source_width + sx) * 4;
                    if index + 3 >= CURSOR_BMP.len() {
                        return;
                    }
                    let alpha = CURSOR_BMP[index + 3];
                    if alpha > 2 {
                        self.blend_color(
                            x + px as i32,
                            y + py as i32,
                            CURSOR_BMP[index + 2],
                            CURSOR_BMP[index + 1],
                            CURSOR_BMP[index],
                            alpha,
                        );
                    }
                }
            }
        }
        #[cfg(target_arch = "x86")]
        for row in 0..14 * scale as i32 {
            for column in 0..=(row / 2) {
                self.pixel(x + column + 2, y + row + 3, 20, 24, 32);
                self.pixel(x + column, y + row, 248, 250, 255);
            }
        }
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
        for y in top..(top + height).min(self.height) {
            for x in left..(left + width).min(self.width) {
                self.pixel(x as i32, y as i32, red, green, blue);
            }
        }
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
        for y in top..(top + height).min(self.height) {
            for x in left..(left + width).min(self.width) {
                self.blend_color(x as i32, y as i32, red, green, blue, alpha);
            }
        }
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
        let right = left.saturating_add(width).min(self.width);
        let bottom = top.saturating_add(height).min(self.height);
        let radius_squared = (radius * radius) as i64;
        for y in top..bottom {
            for x in left..right {
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
                    self.blend_color(x as i32, y as i32, red, green, blue, alpha);
                }
            }
        }
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
        let right = left.saturating_add(width).min(self.width);
        let bottom = top.saturating_add(height).min(self.height);
        for y in top..bottom {
            for x in left..right {
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
        for y in top..(top + height).min(self.height) {
            let logical_y = crop_y + y * crop_height / self.height;
            let source_y = if signed_height < 0 {
                logical_y
            } else {
                source_height - 1 - logical_y
            };
            for x in left..(left + width).min(self.width) {
                let source_x = crop_x + x * crop_width / self.width;
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
        for y in 0..height.min(self.height.saturating_sub(top)) {
            let sy = y * source_height / height;
            let source_y = if signed_height < 0 {
                sy
            } else {
                source_height - 1 - sy
            };
            for x in 0..width.min(self.width.saturating_sub(left)) {
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
        for y in 0..height.min(self.height.saturating_sub(top)) {
            let sy = y * source_height / height;
            let source_y = if signed_height < 0 {
                sy
            } else {
                source_height - 1 - sy
            };
            for x in 0..width.min(self.width.saturating_sub(left)) {
                let sx = x * source_width / width;
                let index = offset + source_y * row_bytes + sx * 4;
                if index + 3 >= bitmap.len() {
                    return;
                }
                let alpha = bitmap[index + 3];
                if alpha != 0 {
                    self.blend_color(
                        (left + x) as i32,
                        (top + y) as i32,
                        bitmap[index + 2],
                        bitmap[index + 1],
                        bitmap[index],
                        alpha,
                    );
                }
            }
        }
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
        for y in 0..size.min(self.height.saturating_sub(top)) {
            let atlas_y = cell_y + y * cell_height / size;
            let source_y = if signed_height < 0 {
                atlas_y
            } else {
                source_height - 1 - atlas_y
            };
            for x in 0..size.min(self.width.saturating_sub(left)) {
                let source_x = cell_x + x * cell_width / size;
                let index = offset + source_y * row_bytes + source_x * 4;
                if index + 3 >= bitmap.len() {
                    return false;
                }
                let alpha = bitmap[index + 3];
                if alpha != 0 {
                    self.blend_color(
                        (left + x) as i32,
                        (top + y) as i32,
                        bitmap[index + 2],
                        bitmap[index + 1],
                        bitmap[index],
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
        for y in 0..height.min(self.height.saturating_sub(top)) {
            let sy = crop_y + y * crop_height / height;
            let source_y = if signed_height < 0 {
                sy
            } else {
                source_height - 1 - sy
            };
            for x in 0..width.min(self.width.saturating_sub(left)) {
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
