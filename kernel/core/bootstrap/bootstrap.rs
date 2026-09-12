//! Splash artwork, emblem animation, boot progress, terminal, and startup options.

use super::installer::INSTALLER_FONT_CELL_HEIGHT;
use super::*;
use crate::boot_info::BootInfo;

#[path = "reveal.rs"]
mod reveal;

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const SPLASH_BMP: &[u8] =
    include_bytes!("../../../assets/boot/infinity-eclipse-header-v1.bmp");
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const CONSOLE_BACKGROUND_BMP: &[u8] =
    include_bytes!("../../../assets/boot/infinity-console-background-v1.bmp");
#[cfg(not(feature = "installer"))]
pub(super) const CONSOLE_BACKGROUND_BMP: &[u8] = &[];

// Startup BBS / FIGlet banner sizing.
// The source bitmap glyphs are 8x8. This renderer resamples them to the
// requested pixel height, so values such as 12px are supported directly.
pub(super) const FIGLET_FONT_HEIGHT_PX: usize = 15;
pub(super) const FIGLET_LINE_HEIGHT_PX: usize = 17;

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const EMBLEM_BMP: &[u8] = include_bytes!("../../../assets/boot/infinity-emblem-v2.bmp");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const BOOT_EMBLEM_TOP_PERCENT: usize = 23;

// Installer-only calibration: the ISO reveal pulse follows the visible ribbon
// centerline in the composited bootstrap artwork, which sits 50 pixels below
// the original mathematical path origin.
pub(super) const BOOT_PARTICLE_Y_OFFSET: i32 = 50;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const CONSOLE_EMBLEM_TOP_PERCENT: usize = 18;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const CONSOLE_EMBLEM_WIDTH_PERCENT: usize = 44;

impl super::DisplayDevice {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_background
    // DESC: Scales and paints the boot artwork across the complete display.
    // ------------------=
    pub(super) fn paint_background(&mut self) {
        self.paint_background_rect(0, 0, self.width, self.height);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_background_rect
    // DESC: Restores a clipped display rectangle from the scaled boot artwork.
    // ------------------=
    pub(super) fn paint_background_rect(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        self.paint_bitmap_cover_rect(SPLASH_BMP, left, top, width, height);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_console_background
    // DESC: Paints the distinct installed-system scene after bootstrap completes.
    // ------------------=
    pub(super) fn paint_console_background(&mut self) {
        self.paint_bitmap_cover_rect(CONSOLE_BACKGROUND_BMP, 0, 0, self.width, self.height);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_console_background_rect
    // DESC: Restores a damaged console rectangle from the installed-system artwork.
    // ------------------=
    pub(super) fn paint_console_background_rect(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        self.paint_bitmap_cover_rect(CONSOLE_BACKGROUND_BMP, left, top, width, height);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: infinity_screen_position
    // DESC: Converts an infinity-path phase into scaled screen coordinates.
    // ------------------=
    pub(super) fn infinity_screen_position(&self, phase: usize, bootstrap: bool) -> (i32, i32) {
        let (left, top, width, height) = if bootstrap {
            let source_width = le32(EMBLEM_BMP, 18) as usize;
            let source_height = (le32(EMBLEM_BMP, 22) as i32).unsigned_abs() as usize;
            let width = (self.width * 52 / 100)
                .min((self.height * 70 / 100) * source_width / source_height);
            let height = width * source_height / source_width;
            (
                (self.width - width) / 2,
                self.height * BOOT_EMBLEM_TOP_PERCENT / 100,
                width,
                height,
            )
        } else {
            self.console_emblem_geometry()
        };
        let (px, py) = infinity_point(phase);
        let center_x = left as i32 + width as i32 / 2;
        let center_y = top as i32 + height as i32 / 2;
        (
            center_x + px * width as i32 / 224,
            center_y + py * height as i32 / 124,
        )
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: console_emblem_geometry
    // DESC: Reserves a resolution-independent emblem rectangle above the Console panel.
    // ------------------=
    pub(super) fn console_emblem_geometry(&self) -> (usize, usize, usize, usize) {
        let source_width = le32(EMBLEM_BMP, 18) as usize;
        let source_height = (le32(EMBLEM_BMP, 22) as i32).unsigned_abs() as usize;
        let panel_top = self.height * 68 / 100;
        let top = self.height * CONSOLE_EMBLEM_TOP_PERCENT / 100;
        let safe_height = panel_top.saturating_sub(top + self.height * 4 / 100);
        let width_from_height = safe_height * source_width / source_height;
        let target_width = (self.width * CONSOLE_EMBLEM_WIDTH_PERCENT / 100)
            .min(width_from_height)
            .max(1);
        let target_height = target_width * source_height / source_width;
        (
            (self.width - target_width) / 2,
            top,
            target_width,
            target_height,
        )
    }
}

impl super::DisplayDevice {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: emblem_reveal_band
    // DESC: Advances a feathered light front, repainting only columns whose exposure changed.
    // ------------------=
    pub(super) fn emblem_reveal_band(&mut self, step: usize, total: usize) {
        if EMBLEM_BMP.len() < 138 || &EMBLEM_BMP[0..2] != b"BM" {
            return;
        }
        let source_width = le32(EMBLEM_BMP, 18) as usize;
        let source_height = (le32(EMBLEM_BMP, 22) as i32).unsigned_abs() as usize;
        let target_width =
            (self.width * 52 / 100).min((self.height * 70 / 100) * source_width / source_height);
        let target_height = target_width * source_height / source_width;
        let left = (self.width - target_width) / 2;
        let top = self.height * BOOT_EMBLEM_TOP_PERCENT / 100;
        for x in 0..target_width {
            let before = reveal::opacity(x, target_width, step, total);
            let after = reveal::opacity(x, target_width, step + 1, total);
            if before == after {
                continue;
            }
            self.paint_background_rect(left + x, top, 1, target_height);
            self.emblem_range(x, x + 1, after);
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: emblem_range
    // DESC: Blends an opacity-controlled horizontal range of the infinity emblem.
    // ------------------=
    pub(super) fn emblem_range(&mut self, first_x: usize, last_x: usize, opacity: u8) {
        let source_width = le32(EMBLEM_BMP, 18) as usize;
        let source_height = (le32(EMBLEM_BMP, 22) as i32).unsigned_abs() as usize;
        let target_width =
            (self.width * 52 / 100).min((self.height * 70 / 100) * source_width / source_height);
        let target_height = target_width * source_height / source_width;
        self.emblem_rect(first_x, 0, last_x, target_height, opacity);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: emblem_rect
    // DESC: Blends a clipped rectangular region of the infinity emblem artwork.
    // ------------------=
    pub(super) fn emblem_rect(
        &mut self,
        first_x: usize,
        first_y: usize,
        last_x: usize,
        last_y: usize,
        opacity: u8,
    ) {
        if EMBLEM_BMP.len() < 138 || &EMBLEM_BMP[0..2] != b"BM" || le16(EMBLEM_BMP, 28) != 32 {
            return;
        }
        let source_width = le32(EMBLEM_BMP, 18) as usize;
        let source_height = (le32(EMBLEM_BMP, 22) as i32).unsigned_abs() as usize;
        let target_width =
            (self.width * 52 / 100).min((self.height * 70 / 100) * source_width / source_height);
        let target_height = target_width * source_height / source_width;
        let left = (self.width - target_width) / 2;
        let top = self.height * BOOT_EMBLEM_TOP_PERCENT / 100;
        self.emblem_rect_at(
            left,
            top,
            target_width,
            target_height,
            first_x,
            first_y,
            last_x,
            last_y,
            opacity,
        );
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: emblem_rect_at
    // DESC: Blends a clipped emblem region into an explicitly bounded destination rectangle.
    // ------------------=
    pub(super) fn emblem_rect_at(
        &mut self,
        left: usize,
        top: usize,
        target_width: usize,
        target_height: usize,
        first_x: usize,
        first_y: usize,
        last_x: usize,
        last_y: usize,
        opacity: u8,
    ) {
        if EMBLEM_BMP.len() < 138 || &EMBLEM_BMP[0..2] != b"BM" || le16(EMBLEM_BMP, 28) != 32 {
            return;
        }
        let offset = le32(EMBLEM_BMP, 10) as usize;
        let source_width = le32(EMBLEM_BMP, 18) as usize;
        let signed_height = le32(EMBLEM_BMP, 22) as i32;
        let source_height = signed_height.unsigned_abs() as usize;
        for y in first_y..last_y.min(target_height) {
            let sy = y * source_height / target_height;
            let source_y = if signed_height < 0 {
                sy
            } else {
                source_height - 1 - sy
            };
            for x in first_x..last_x.min(target_width) {
                let sx = x * source_width / target_width;
                let index = offset + (source_y * source_width + sx) * 4;
                if index + 3 >= EMBLEM_BMP.len() {
                    return;
                }
                let alpha = ((EMBLEM_BMP[index + 3] as u16 * opacity as u16) / 255) as u8;
                if alpha < 3 {
                    continue;
                }
                let blue = EMBLEM_BMP[index] as u16;
                let green = EMBLEM_BMP[index + 1] as u16;
                let red = EMBLEM_BMP[index + 2] as u16;
                let a = alpha as u16;
                let current = self.framebuffer_pixel(left + x, top + y);
                let (cr, cg, cb) = if self.format == 0 {
                    (
                        (current & 255) as u16,
                        ((current >> 8) & 255) as u16,
                        ((current >> 16) & 255) as u16,
                    )
                } else {
                    (
                        ((current >> 16) & 255) as u16,
                        ((current >> 8) & 255) as u16,
                        (current & 255) as u16,
                    )
                };
                self.pixel(
                    (left + x) as i32,
                    (top + y) as i32,
                    ((red * a + cr * (255 - a)) / 255) as u8,
                    ((green * a + cg * (255 - a)) / 255) as u8,
                    ((blue * a + cb * (255 - a)) / 255) as u8,
                );
            }
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: finish_emblem
    // DESC: Draws the complete infinity emblem after its reveal animation finishes.
    // ------------------=
    pub(super) fn finish_emblem(&mut self) {
        let source_width = le32(EMBLEM_BMP, 18) as usize;
        let source_height = (le32(EMBLEM_BMP, 22) as i32).unsigned_abs() as usize;
        let target_width =
            (self.width * 52 / 100).min((self.height * 70 / 100) * source_width / source_height);
        let target_height = target_width * source_height / source_width;
        let left = (self.width - target_width) / 2;
        let top = self.height * BOOT_EMBLEM_TOP_PERCENT / 100;
        let padding = self.ui_scale() * 32;
        self.paint_background_rect(
            left.saturating_sub(padding),
            top.saturating_sub(padding),
            target_width + padding * 2,
            target_height + padding * 2,
        );
        self.emblem_range(0, target_width, 255);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: finish_console_emblem
    // DESC: Draws a complete uncropped emblem over the installed-system background.
    // ------------------=
    pub(super) fn finish_console_emblem(&mut self) {
        let (left, top, target_width, target_height) = self.console_emblem_geometry();
        let padding = self.ui_scale() * 32;
        self.paint_console_background_rect(
            left.saturating_sub(padding),
            top.saturating_sub(padding),
            target_width + padding * 2,
            target_height + padding * 2,
        );
        self.emblem_rect_at(
            left,
            top,
            target_width,
            target_height,
            0,
            0,
            target_width,
            target_height,
            255,
        );
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: restore_emblem_particle
    // DESC: Restores artwork beneath a moving emblem particle before its next frame.
    // ------------------=
    pub(super) fn restore_emblem_particle(&mut self, x: i32, y: i32, radius: i32, bootstrap: bool) {
        let (emblem_left, emblem_top, target_width, target_height) = if bootstrap {
            let source_width = le32(EMBLEM_BMP, 18) as usize;
            let source_height = (le32(EMBLEM_BMP, 22) as i32).unsigned_abs() as usize;
            let target_width = (self.width * 52 / 100)
                .min((self.height * 70 / 100) * source_width / source_height);
            let target_height = target_width * source_height / source_width;
            (
                (self.width - target_width) / 2,
                self.height * BOOT_EMBLEM_TOP_PERCENT / 100,
                target_width,
                target_height,
            )
        } else {
            self.console_emblem_geometry()
        };
        let left = (x - radius - 2).max(0) as usize;
        let top = (y - radius - 2).max(0) as usize;
        let size = (radius * 2 + 5) as usize;
        if bootstrap {
            self.paint_background_rect(left, top, size, size);
        } else {
            self.paint_console_background_rect(left, top, size, size);
        }
        let local_left = left.saturating_sub(emblem_left);
        let local_top = top.saturating_sub(emblem_top);
        let local_right = (left + size).saturating_sub(emblem_left).min(target_width);
        let local_bottom = (top + size).saturating_sub(emblem_top).min(target_height);
        if local_left < local_right && local_top < local_bottom {
            self.emblem_rect_at(
                emblem_left,
                emblem_top,
                target_width,
                target_height,
                local_left,
                local_top,
                local_right,
                local_bottom,
                255,
            );
        }
    }

    // ------------------------=
    // FUNC: infinity_pulse
    // DESC: Draws the luminous pulse and trail along the infinity-shaped path.
    // ------------------=
    pub(super) fn infinity_pulse(&mut self, head: usize) {
        let scale_x = (self.width.min(self.height * 16 / 9) as i32 / 400).max(1);
        let scale_y = (scale_x * 2 / 3).max(1);
        let center_x = self.width as i32 / 2;
        let center_y = self.height as i32 * 37 / 100 + BOOT_PARTICLE_Y_OFFSET;
        let (head_x, head_y) = infinity_point(head);
        self.glow(
            center_x + head_x * scale_x,
            center_y + head_y * scale_y,
            scale_x * 2,
            80,
        );
        self.glow(
            center_x + head_x * scale_x,
            center_y + head_y * scale_y,
            (scale_x / 2).max(2),
            235,
        );
        for age in (2..14usize).rev() {
            let index = (head + 384 - age * 3) % 384;
            let (px, py) = infinity_point(index);
            let strength = 18 + ((14 - age) * 6) as u8;
            self.glow(
                center_x + px * scale_x,
                center_y + py * scale_y,
                (scale_x / 2).max(2),
                strength,
            );
            if age % 4 == 0 {
                let jitter_x = (((index * 17 + age * 11) % 9) as i32 - 4) * scale_x;
                let jitter_y = (((index * 7 + age * 13) % 7) as i32 - 3) * scale_y;
                self.glow(
                    center_x + px * scale_x + jitter_x,
                    center_y + py * scale_y + jitter_y,
                    (scale_x / 3).max(1),
                    strength.saturating_add(55),
                );
            }
        }
    }

    // ------------------------=
    // FUNC: glow
    // DESC: Draws a soft white radial glow centered at the requested point.
    // ------------------=
    pub(super) fn glow(&mut self, x: i32, y: i32, radius: i32, intensity: u8) {
        self.glow_color(x, y, radius, 255, 255, 255, intensity);
    }

    // ------------------------=
    // FUNC: glow_color
    // DESC: Draws a soft colored radial glow with distance-based opacity.
    // ------------------=
    pub(super) fn glow_color(
        &mut self,
        x: i32,
        y: i32,
        radius: i32,
        red: u8,
        green: u8,
        blue: u8,
        intensity: u8,
    ) {
        for oy in -radius..=radius {
            for ox in -radius..=radius {
                let distance = ox * ox + oy * oy;
                if distance <= radius * radius {
                    let alpha = ((radius * radius - distance) as u32 * intensity as u32
                        / (radius * radius).max(1) as u32) as u8;
                    self.blend_color(x + ox, y + oy, red, green, blue, alpha);
                }
            }
        }
    }

    // ------------------------=
    // FUNC: star_orb
    // DESC: Draws a star-like orbiting particle and optional bright trail head.
    // ------------------=
    pub(super) fn star_orb(&mut self, x: i32, y: i32, radius: i32, intensity: u8, head: bool) {
        self.glow_color(x, y, radius * 2, 108, 176, 255, intensity / 3);
        self.glow_color(x, y, radius, 220, 238, 255, intensity);
        self.glow_color(x, y, (radius / 3).max(1), 255, 255, 255, 255);
        if head {
            for distance in 1..=radius * 3 {
                let alpha = (180i32.saturating_sub(distance * 150 / (radius * 3).max(1))) as u8;
                self.blend_white(x + distance, y, alpha);
                self.blend_white(x - distance, y, alpha);
                self.blend_white(x, y + distance, alpha);
                self.blend_white(x, y - distance, alpha);
                if distance <= radius * 2 {
                    self.blend_color(x + distance, y + distance, 176, 216, 255, alpha / 2);
                    self.blend_color(x - distance, y - distance, 176, 216, 255, alpha / 2);
                    self.blend_color(x + distance, y - distance, 176, 216, 255, alpha / 2);
                    self.blend_color(x - distance, y + distance, 176, 216, 255, alpha / 2);
                }
            }
        }
    }

    // ------------------------=
    // FUNC: progress
    // DESC: Draws the loading label and bounded progress bar for the boot sequence.
    // ------------------=
    pub(super) fn progress(&mut self, percent: usize, label: &[u8]) {
        let ui_scale = self.ui_scale();
        let width = (self.width * 58 / 100).max(240);
        let height = 28usize * ui_scale;
        let left = (self.width - width) / 2;
        let top = self.height * 82 / 100;
        for y in 0..height {
            for x in 0..width {
                let border = x < 2 || y < 2 || x >= width - 2 || y >= height - 2;
                let filled = x < (width - 4) * percent / 100 + 2;
                let (r, g, b) = if border {
                    (184, 184, 188)
                } else if filled {
                    (112, 112, 118)
                } else {
                    (30, 30, 34)
                };
                self.pixel((left + x) as i32, (top + y) as i32, r, g, b);
            }
        }
        let text_width = self.installer_text_width(label, false);
        self.text(
            (self.width.saturating_sub(text_width)) / 2,
            top + height.saturating_sub(INSTALLER_FONT_CELL_HEIGHT) / 2,
            label,
            235,
            235,
            238,
        );
    }

    // ------------------------=
    // FUNC: terminal_box
    // DESC: Draws the seven-row console panel, command input, and selection state.
    // ------------------=
    pub(super) fn terminal_box(
        &mut self,
        lines: &[[u8; 96]; 6],
        lengths: &[usize; 6],
        line_count: usize,
        prompt: &[u8],
        command: &[u8],
        _split_menu: bool,
    ) {
        let ui_scale = self.ui_scale();
        let width = (self.width * 58 / 100)
            .max(420)
            .min(self.width.saturating_sub(24));
        let row_height = INSTALLER_FONT_CELL_HEIGHT + 4;
        let height = row_height * 7 + 16 * ui_scale;
        let left = (self.width - width) / 2;
        let top = self.height * 68 / 100;
        for y in 0..height {
            for x in 0..width {
                let border = x < 2 || y < 2 || x >= width - 2 || y >= height - 2;
                let (r, g, b) = if border {
                    (166, 170, 180)
                } else {
                    (13, 15, 20)
                };
                self.pixel((left + x) as i32, (top + y) as i32, r, g, b);
            }
        }
        for row in 0..line_count.min(6) {
            let (red, green, blue) = (216, 222, 232);
            self.text(
                left + 14 * ui_scale,
                top + 8 * ui_scale + row * row_height,
                &lines[row][..lengths[row]],
                red,
                green,
                blue,
            );
        }
        if !prompt.is_empty() && line_count < 7 {
            let y = top + 8 * ui_scale + line_count * row_height;
            self.text(left + 14 * ui_scale, y, prompt, 238, 241, 247);
            let prompt_width = self.installer_text_width(prompt, false);
            let command_width = self.installer_text_width(command, false);
            self.text(
                left + 14 * ui_scale + prompt_width,
                y,
                command,
                255,
                255,
                255,
            );
            self.text(
                left + 14 * ui_scale + prompt_width + command_width,
                y,
                b"_",
                255,
                255,
                255,
            );
        }
    }

    // ------------------------=
    // FUNC: startup_bbs
    // DESC: Composes the BBS-style startup console over the boot artwork.
    // ------------------=
    pub(super) fn startup_bbs(
        &mut self,
        command: &[u8],
        cursor_x: i32,
        cursor_y: i32,
        pressed: bool,
    ) {
        let scale = self.ui_scale();
        let left = self.width * 13 / 100;
        let top = self.height * 58 / 100;
        let width = self.width * 74 / 100;
        let height = self.height * 39 / 100;
        self.fill_rect(left, top, width, height, 7, 11, 18);
        self.outline_rect(left, top, width, height, 142, 164, 190);
        self.outline_rect(
            left + 4 * scale,
            top + 4 * scale,
            width.saturating_sub(8 * scale),
            height.saturating_sub(8 * scale),
            42,
            68,
            94,
        );
        let text_left = left + 22 * scale;

        // ASCII - Grafiti Header
        let graffiti: [&[u8]; 6] = [
            br#"  .___           _____ .__         .__   __              ________     _________ "#,
            br#"  |   |  ____  _/ ____\|__|  ____  |__|_/  |_  ___.__.   \_____  \   /   _____/ "#,
            br#"  |   | /    \ \   __\ |  | /    \ |  |\   __\<   |  |    /   |   \  \_____  \  "#,
            br#"  |   ||   |  \ |  |   |  ||   |  \|  | |  |   \___  |   /    |    \ /        \ "#,
            br#"  |___||___|  / |__|   |__||___|  /|__| |__|   / ____| /\_______  //_______  /  "#,
            br#"            \/                  \/             \/      \/        \/         \/  "#,
        ];

        for (row, line) in graffiti.iter().enumerate() {
            let (red, green, blue) = if row == 0 || row == 5 {
                (143, 215, 255)
            } else if row & 1 == 0 {
                (248, 251, 255)
            } else {
                (214, 233, 249)
            };

            // Render the wide FIGlet banner at an explicit, alterable pixel
            // height instead of being restricted to 8px/16px integer scaling.
            self.text_scaled_to_height(
                text_left,
                top + 13 * scale + row * FIGLET_LINE_HEIGHT_PX,
                line,
                red,
                green,
                blue,
                FIGLET_FONT_HEIGHT_PX,
                false,
            );
        }
        self.text(
            text_left,
            top.saturating_add(82 * scale).saturating_sub(22),
            b"     --[ Welcome to infinityOS ]: \n\n",
            112,
            181,
            224,
        );
        self.startup_option_rows(cursor_x, cursor_y, pressed);
        let prompt_y = self.height * 88 / 100;
        let prompt_x = self.width * 18 / 100;
        self.infinity_prompt_mark(prompt_x, prompt_y);
        let prompt_label = b" -> ";
        let prompt_label_x = prompt_x + 21 * scale;
        self.text(prompt_label_x, prompt_y, prompt_label, 216, 229, 242);
        let command_x = prompt_label_x + self.installer_text_width(prompt_label, false);
        self.text(command_x, prompt_y, command, 248, 250, 255);
        self.text(
            command_x + self.installer_text_width(command, false),
            prompt_y,
            b"_",
            248,
            250,
            255,
        );
        self.text(
            self.width * 18 / 100,
            self.height * 93 / 100,
            b"TAB / ARROWS: MOVE     ENTER: SELECT     CMDS: install | repair | console",
            121,
            158,
            191,
        );
    }

    // ------------------------=
    // FUNC: infinity_prompt_mark
    // DESC: Draws the compact infinity mark used beside the command prompt.
    // ------------------=
    pub(super) fn infinity_prompt_mark(&mut self, left: usize, top: usize) {
        let scale = self.ui_scale();
        for &(path_x, path_y) in &INFINITY_PATH {
            let x = left + ((path_x + 112) as usize * 18 * scale / 224);
            let y = top + ((62 - path_y) as usize * 8 * scale / 124);
            self.fill_rect(x, y, scale.max(1), scale.max(1), 216, 229, 242);
        }
    }

    // ------------------------=
    // FUNC: startup_option_rows
    // DESC: Draws and highlights the mouse-selectable startup actions.
    // ------------------=
    pub(super) fn startup_option_rows(&mut self, cursor_x: i32, cursor_y: i32, pressed: bool) {
        let scale = self.ui_scale();
        let options: [(&[u8], &[u8], i32); 3] = [
            (
                b"[1]  INSTALL INFINITYOS",
                b"<-- Begin the installation",
                735,
            ),
            (
                b"[2]  REPAIR AN EXISTING INSTALLATION",
                b"<-- Inspect and repair",
                785,
            ),
            (
                b"[3]  OPEN SYSTEM CONSOLE",
                b"<-- Command Line System console",
                835,
            ),
        ];
        for (title, subtitle, center) in options {
            let row_top = self.height * (center as usize - 23) / 1000;
            let row_height = self.height * 46 / 1000;
            let selected = (cursor_y - center).abs() <= 25;
            let hovered = selected && (180..=820).contains(&cursor_x);
            let (r, g, b) = if hovered && pressed {
                (32, 54, 76)
            } else if selected {
                (20, 39, 58)
            } else {
                (10, 17, 27)
            };
            self.fill_rect(
                self.width * 18 / 100,
                row_top,
                self.width * 64 / 100,
                row_height,
                r,
                g,
                b,
            );
            if selected {
                self.fill_rect(
                    self.width * 18 / 100,
                    row_top,
                    3 * scale,
                    row_height,
                    201,
                    225,
                    246,
                );
            }
            let color = if selected {
                (248, 251, 255)
            } else {
                (207, 220, 234)
            };
            self.text(
                self.width * 20 / 100,
                row_top + 4 * scale,
                title,
                color.0,
                color.1,
                color.2,
            );
            self.text(
                self.width * 49 / 100,
                row_top + 4 * scale,
                subtitle,
                104,
                151,
                190,
            );
        }
    }
}

// ------------------------=
// FUNC: note_pointer_activity
// DESC: Marks recent pointer motion so bootstrap effects yield the current frame to interactive input.
// ------------------=
pub fn note_pointer_activity() {
    unsafe {
        POINTER_ACTIVITY_PENDING = true;
        // Keep the software-rendered particle pass out of the short interval
        // between positioning the pointer and pressing a button. VirtualBox's
        // firmware pointer exposes only its current button state; if a costly
        // frame spans both press and release, the click cannot be recovered.
        // Thirty 60 Hz deadlines provide a 500 ms interaction window while
        // leaving the animation completely smooth whenever the pointer rests.
        POINTER_ACTIVITY_GRACE_TICKS = 30;
    }
}

// ------------------------=
// FUNC: activate_console
// DESC: Transfers the display device into the persistent interactive console surface.
// ------------------=
fn activate_console(display: DisplayDevice) {
    unsafe {
        CONSOLE = Some(ConsoleSurface {
            display,
            particle_phase: 0,
            particles_initialized: false,
            bootstrap_scene: true,
            split_layout: false,
            layout_initialized: false,
            cursor_x: 0,
            cursor_y: 0,
            pointer_pressed: false,
            installer_scene: false,
            last_installer_screen: 0,
            last_installer_focus: usize::MAX,
            last_installer_choice: usize::MAX,
            last_installer_date_time: crate::storage::DateTimeConfiguration::utc_default(),
            last_installer_date_time_part: usize::MAX,
            installer_progress: 0,
            installer_animation_phase: 0,
            last_system_screen: 0,
            last_system_step: usize::MAX,
            last_system_focus: usize::MAX,
            last_system_menu: usize::MAX,
            last_icon_theme: u8::MAX,
            last_accent_rgb: u32::MAX,
            last_primary_rgb: u32::MAX,
            last_background_opacity: u8::MAX,
            last_background_blur: u8::MAX,
            last_system_content: 0,
            last_system_static_content: 0,
            last_launcher_state: 0,
            last_launcher_interaction_state: 0,
            last_launcher_transition: 0,
            last_system_validation_error: false,
            last_home_window_x: i32::MIN,
            last_home_window_y: i32::MIN,
            last_home_window_width: i32::MIN,
            last_home_window_height: i32::MIN,
            last_home_window_visible: false,
            last_home_window_maximized: false,
            last_home_location: usize::MAX,
            last_home_selected_item: None,
            last_home_dragging_item: None,
            last_home_note_location: usize::MAX,
            last_file_navigator_state: None,
            last_desktop_items: 0,
            last_desktop_item_positions: [[i32::MIN; 2]; 7],
            last_system_clock: crate::storage::DateTimeConfiguration::utc_default(),
            last_network_settings: None,
            last_node_settings: None,
            last_pool_settings_revision: None,
            last_settings_window: crate::ui::system_layout::SettingsWindowState {
                x: 160,
                y: 210,
                width: 680,
                height: 620,
                maximized: false,
                expanded_row: None,
                scroll_offset: 0,
                control_focus: 0,
                row_count: 8,
            },
            last_app_window_x: 0,
            last_app_window_y: 0,
            last_app_window_width: 0,
            last_app_window_height: 0,
            last_app_window_maximized: false,
            last_editor_saved: true,
            system_ui_active: false,
            cursor_saved: false,
            cursor_left: 0,
            cursor_top: 0,
            cursor_width: 0,
            cursor_height: 0,
            cursor_backing: [0; 128 * 128],
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            menu_saved: false,
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            menu_left: 0,
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            menu_top: 0,
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            menu_width: 0,
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            menu_height: 0,
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            menu_backing: [0; 640 * 800],
        });
    }
}

impl ConsoleSurface {
    // ------------------------=
    // FUNC: restore_cursor
    // DESC: Restores the saved pixels underneath the cursor from the previous frame.
    // ------------------=
    pub(super) fn restore_cursor(&mut self) {
        if !self.cursor_saved {
            return;
        }
        for y in 0..self.cursor_height {
            for x in 0..self.cursor_width {
                self.display.write_framebuffer_pixel(
                    self.cursor_left + x,
                    self.cursor_top + y,
                    self.cursor_backing[y * self.cursor_width + x],
                );
            }
        }
        self.cursor_saved = false;
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: restore_menu
    // DESC: Restores the exact desktop pixels saved beneath the active top-level menu.
    // ------------------=
    pub(super) fn restore_menu(&mut self) {
        if !self.menu_saved {
            return;
        }
        for y in 0..self.menu_height {
            for x in 0..self.menu_width {
                self.display.write_framebuffer_pixel(
                    self.menu_left + x,
                    self.menu_top + y,
                    self.menu_backing[y * self.menu_width + x],
                );
            }
        }
        self.menu_saved = false;
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: save_menu
    // DESC: Captures the bounded desktop region underneath one translucent top-level menu.
    // ------------------=
    pub(super) fn save_menu(&mut self, menu_kind: usize) {
        let layout =
            crate::ui::system_layout::SystemLayout::new(self.display.width, self.display.height);
        let (left, top, width, height) = layout.system_menu_damage_geometry(menu_kind);
        let width = width.min(640);
        let height = height.min(800);
        for y in 0..height {
            for x in 0..width {
                self.menu_backing[y * width + x] =
                    self.display.framebuffer_pixel(left + x, top + y);
            }
        }
        self.menu_left = left;
        self.menu_top = top;
        self.menu_width = width;
        self.menu_height = height;
        self.menu_saved = true;
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: present_desktop_menu
    // DESC: Opens, updates, switches, or closes a menu using only its saved damage region.
    // ------------------=
    pub(super) fn present_desktop_menu(
        &mut self,
        previous_screen: u8,
        previous_menu: usize,
        screen: u8,
        menu_kind: usize,
        focus: usize,
        clock: crate::storage::DateTimeConfiguration,
    ) {
        if previous_screen == 3 {
            self.restore_menu();
        }
        if screen == 3 {
            self.save_menu(menu_kind);
            let scale = self.display.ui_scale().max(1);
            self.display.system_menu_panel(menu_kind, focus, scale);
        }
        if previous_screen != screen || previous_menu != menu_kind {
            self.display
                .system_top_bar((screen == 3).then_some(menu_kind), clock);
        }
    }

    // ------------------------=
    // FUNC: save_and_draw_cursor
    // DESC: Saves pixels beneath the cursor and then draws the cursor artwork.
    // ------------------=
    pub(super) fn save_and_draw_cursor(&mut self, cursor_x: i32, cursor_y: i32) {
        let left = (self.display.width as i32 * cursor_x / 1000).max(0) as usize;
        let top = (self.display.height as i32 * cursor_y / 1000).max(0) as usize;
        let size = (28 * self.display.ui_scale()).min(128);
        let width = size.min(self.display.width.saturating_sub(left));
        let height = size.min(self.display.height.saturating_sub(top));
        for y in 0..height {
            for x in 0..width {
                self.cursor_backing[y * width + x] =
                    self.display.framebuffer_pixel(left + x, top + y);
            }
        }
        self.cursor_left = left;
        self.cursor_top = top;
        self.cursor_width = width;
        self.cursor_height = height;
        self.cursor_saved = width != 0 && height != 0;
        self.display.pointer_cursor(cursor_x, cursor_y);
    }

    // ------------------------=
    // FUNC: prepare_layout
    // DESC: Recomputes scaled geometry when the screen or display mode changes.
    // ------------------=
    pub(super) fn prepare_layout(&mut self, split: bool, installer: bool) {
        // The startup BBS intentionally masks the lower portion of the large
        // bootstrap emblem. Once the user enters Console or Repair, replace
        // that composition with the dedicated console scene and its bounded
        // emblem before drawing the shorter terminal panel. Reusing the boot
        // emblem here exposed its masked edge and produced the chopped image.
        if self.bootstrap_scene && !installer && !split && self.split_layout {
            self.particles_initialized = false;
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            {
                self.display.paint_console_background();
                self.display.finish_console_emblem();
            }
            self.bootstrap_scene = false;
            self.layout_initialized = false;
        }
        if self.installer_scene != installer {
            self.particles_initialized = false;
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            if installer {
                self.display.paint_installer_background();
                self.display.paint_installer_masthead();
                self.bootstrap_scene = false;
            } else {
                self.display.paint_console_background();
                self.display.finish_console_emblem();
            }
            #[cfg(target_arch = "x86")]
            self.display
                .fill_rect(0, 0, self.display.width, self.display.height, 0, 0, 0);
            self.installer_scene = installer;
            self.layout_initialized = false;
        }
        if self.layout_initialized && self.split_layout == split {
            return;
        }
        let left = self.display.width * 3 / 100;
        // Invalidate the complete union of the startup BBS and installer
        // surfaces.  The startup panel begins higher and ends lower than the
        // ordinary console; a smaller rectangle leaves its frame behind when
        // changing modes.
        let top = self.display.height * 57 / 100;
        let width = self.display.width * 95 / 100;
        let height = self.display.height * 42 / 100;
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        if installer {
            self.display
                .paint_installer_background_rect(left, top, width, height);
        } else if self.bootstrap_scene {
            self.display.paint_background_rect(left, top, width, height);
        } else {
            self.display
                .paint_console_background_rect(left, top, width, height);
        }
        #[cfg(target_arch = "x86")]
        self.display.fill_rect(left, top, width, height, 0, 0, 0);
        self.split_layout = split;
        self.layout_initialized = true;
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: animate
    // DESC: Advances and redraws one frame of console particle animation.
    // ------------------=
    pub(super) fn animate(&mut self) {
        if self.installer_scene || self.system_ui_active {
            return;
        }
        let scale = self.display.ui_scale() as i32;
        // The startup BBS begins above the ordinary Console panel. Animation
        // restoration must use the active panel edge, otherwise it restores
        // emblem/background pixels over the opaque BBS and creates the dark
        // scallops visible along its top border.
        let panel_clip = if self.split_layout {
            self.display.height * 58 / 100
        } else {
            self.display.height * 68 / 100
        } as i32;
        let animation_clip = panel_clip - (self.display.height * 2 / 100) as i32;
        // Three comet-like streams share the emblem's exact 384-step path.
        // Each head pulls a tapered five-particle wake through both loops and
        // the center crossover; this continues independently of boot progress.
        if self.particles_initialized {
            for stream in 0..3usize {
                for trail in 0..6usize {
                    let old_phase = (self.particle_phase + stream * 128 + 384 - trail * 5) % 384;
                    let (x, y) = self
                        .display
                        .infinity_screen_position(old_phase, self.bootstrap_scene);
                    let radius = ((7 - trail as i32) * scale / 2).max(scale);
                    let extent = if trail == 0 { radius * 3 } else { radius * 2 };
                    if y + extent + 2 < animation_clip {
                        self.display
                            .restore_emblem_particle(x, y, extent, self.bootstrap_scene);
                    }
                }
            }
        }
        self.particle_phase = (self.particle_phase + 2) % 384;
        for stream in 0..3usize {
            for trail in 0..6usize {
                let phase = (self.particle_phase + stream * 128 + 384 - trail * 5) % 384;
                let (x, y) = self
                    .display
                    .infinity_screen_position(phase, self.bootstrap_scene);
                let radius = ((7 - trail as i32) * scale / 2).max(scale);
                let intensity = (230usize.saturating_sub(trail * 31)) as u8;
                let extent = if trail == 0 { radius * 3 } else { radius * 2 };
                if y + extent + 2 < animation_clip {
                    self.display.star_orb(x, y, radius, intensity, trail == 0);
                }
            }
        }
        self.particles_initialized = true;
    }

    #[cfg(target_arch = "x86")]
    // ------------------------=
    // FUNC: animate
    // DESC: Provides a no-op animation implementation when graphics are unavailable.
    // ------------------=
    pub(super) fn animate(&mut self) {}
}

// ------------------------=
// FUNC: console_present
// DESC: Presents console or installer state while minimizing damaged-region redraws.
// ------------------=
pub fn console_present(
    lines: &[[u8; 96]; 6],
    lengths: &[usize; 6],
    line_count: usize,
    prompt: &[u8],
    command: &[u8],
    split_menu: bool,
    installer_screen: u8,
    installer_focus: usize,
    installer_choice: usize,
    installer_date_time: crate::storage::DateTimeConfiguration,
    installer_date_time_part: usize,
    installer_has_primary: bool,
    storage_device: Option<crate::storage::StorageDevice>,
    cursor_x: i32,
    cursor_y: i32,
    pointer_pressed: bool,
) {
    unsafe {
        let slot = &raw mut CONSOLE;
        if let Some(console) = (*slot).as_mut() {
            console.system_ui_active = false;
            let full_redraw = !console.layout_initialized
                || console.split_layout != split_menu
                || console.installer_scene != (installer_screen != 0);
            let screen_changed = console.last_installer_screen != installer_screen;
            let focus_changed = console.last_installer_focus != installer_focus;
            let choice_changed = console.last_installer_choice != installer_choice;
            let date_time_changed = console.last_installer_date_time != installer_date_time;
            let date_time_part_changed =
                console.last_installer_date_time_part != installer_date_time_part;
            let old_cursor_y = console.cursor_y;
            let pressed_changed = console.pointer_pressed != pointer_pressed;
            let pointer_changed =
                console.cursor_x != cursor_x || console.cursor_y != cursor_y || pressed_changed;
            let startup_row = |y: i32| -> u8 {
                if (710..760).contains(&y) {
                    0
                } else if (760..810).contains(&y) {
                    1
                } else if (810..860).contains(&y) {
                    2
                } else {
                    3
                }
            };
            let startup_row_changed = startup_row(old_cursor_y) != startup_row(cursor_y);
            // Restore only the pixels under the old cursor. Repainting the
            // full photographic background for each key or HID report caused
            // the visible flash seen in VirtualBox.
            console.restore_cursor();
            console.cursor_x = cursor_x;
            console.cursor_y = cursor_y;
            console.pointer_pressed = pointer_pressed;
            console.prepare_layout(split_menu, installer_screen != 0);
            if installer_screen != 0 {
                let content_redraw = full_redraw
                    || screen_changed
                    || (installer_screen == 5
                        && (choice_changed
                            || date_time_changed
                            || date_time_part_changed
                            || (focus_changed && !pointer_changed)));
                if content_redraw {
                    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
                    console.display.restore_installer_panel(installer_screen);
                    console.display.installer_panel(
                        lines,
                        lengths,
                        line_count,
                        prompt,
                        command,
                        installer_screen,
                        storage_device,
                        installer_date_time,
                        installer_date_time_part,
                        installer_focus,
                        cursor_x,
                        cursor_y,
                        pointer_pressed,
                    );
                }
                if installer_screen == 8 && content_redraw {
                    console.installer_progress = 0;
                    console.installer_animation_phase = 0;
                    console
                        .display
                        .installer_progress_frame(0, b"PREPARING INSTALLATION", 0);
                }
                if installer_screen == 7 && (content_redraw || focus_changed || pressed_changed) {
                    console.display.installer_confirmation_popup(
                        installer_focus,
                        cursor_x,
                        cursor_y,
                        pointer_pressed,
                    );
                }
                if full_redraw || screen_changed || focus_changed || pressed_changed {
                    console.display.installer_navigation(
                        installer_screen,
                        installer_focus,
                        installer_has_primary,
                        cursor_x,
                        cursor_y,
                        pointer_pressed,
                        full_redraw || screen_changed,
                    );
                }
            } else if split_menu {
                // Text edits still repaint the BBS, but ordinary mouse motion
                // only restores and redraws the small cursor sprite. Repaint
                // option rows when their hover state actually changes.
                if full_redraw || !pointer_changed {
                    console
                        .display
                        .startup_bbs(command, cursor_x, cursor_y, pointer_pressed);
                } else if startup_row_changed || pressed_changed {
                    console
                        .display
                        .startup_option_rows(cursor_x, cursor_y, pointer_pressed);
                }
            } else {
                console
                    .display
                    .terminal_box(lines, lengths, line_count, prompt, command, false);
            }
            if installer_screen != 0 || split_menu {
                console.save_and_draw_cursor(cursor_x, cursor_y);
            }
            console.last_installer_screen = installer_screen;
            console.last_installer_focus = installer_focus;
            console.last_installer_choice = installer_choice;
            console.last_installer_date_time = installer_date_time;
            console.last_installer_date_time_part = installer_date_time_part;
            console.display.present_damage();
        }
    }
}

// ------------------------=
// FUNC: animation_tick
// DESC: Advances the active console animation on an input-driver timer tick.
// ------------------=
pub fn animation_tick() {
    crate::drivers::input::keyboard_tick();
    if !animation_due() {
        return;
    }
    if crate::console::ui_animation_tick() {
        return;
    }
    unsafe {
        SYSTEM_CLOCK_TICKS = SYSTEM_CLOCK_TICKS.saturating_add(1);
        if SYSTEM_CLOCK_TICKS >= 60 {
            SYSTEM_CLOCK_TICKS = 0;
            crate::console::clock_tick();
        }
    }
    // Keep the full-quality 60 Hz particle path while idle. A pointer update
    // owns its current presentation deadline completely; effects resume at the
    // first idle deadline (normally within 16 ms). On the software-rendered ARM
    // bootstrap this prevents any expensive glow frame from landing between a
    // HID sample and its cursor presentation. Installer and desktop paths are
    // unchanged.
    let defer_for_pointer = unsafe {
        let active = POINTER_ACTIVITY_PENDING;
        POINTER_ACTIVITY_PENDING = false;
        if POINTER_ACTIVITY_GRACE_TICKS != 0 {
            POINTER_ACTIVITY_GRACE_TICKS -= 1;
            true
        } else {
            active
        }
    };
    if defer_for_pointer {
        return;
    }
    unsafe {
        let slot = &raw mut CONSOLE;
        if let Some(console) = (*slot).as_mut() {
            if console.system_ui_active && matches!(console.last_system_screen, 5 | 6) {
                console.restore_cursor();
                console.particle_phase = (console.particle_phase + 2) % 384;
                console
                    .display
                    .system_login_animation(console.particle_phase);
                console.save_and_draw_cursor(console.cursor_x, console.cursor_y);
                console.display.present_damage();
                return;
            }
            // Animation and cursor share the front buffer. Restore the cursor
            // before updating particles, then recapture the finished pixels;
            // otherwise cursor motion writes stale emblem pixels back and
            // produces flashing trails.
            let cursor_over_animation = !console.installer_scene
                && console.split_layout
                && (180..=820).contains(&console.cursor_x)
                && (180..=560).contains(&console.cursor_y);
            if cursor_over_animation {
                console.restore_cursor();
            }
            console.animate();
            if cursor_over_animation {
                console.save_and_draw_cursor(console.cursor_x, console.cursor_y);
            }
            console.display.present_damage();
        }
    }
}

#[cfg(target_arch = "aarch64")]
// ------------------------=
// FUNC: animation_due
// DESC: Uses the x86 time-stamp counter to decide when the next frame is due.
// ------------------=
fn animation_due() -> bool {
    static mut NEXT: u64 = 0;
    let now: u64;
    let frequency: u64;
    unsafe {
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) now);
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) frequency);
        if now < NEXT {
            return false;
        }
        NEXT = now.saturating_add((frequency / 60).max(1));
    }
    true
}

#[cfg(target_arch = "x86_64")]
// ------------------------=
// FUNC: animation_due
// DESC: Uses the AArch64 virtual counter to decide when the next frame is due.
// ------------------=
fn animation_due() -> bool {
    static mut NEXT: u64 = 0;
    let now = unsafe { core::arch::x86_64::_rdtsc() };
    unsafe {
        if now < NEXT {
            return false;
        }
        let leaf = core::arch::x86_64::__cpuid(0x16).eax as u64;
        let frequency = leaf.max(1000) * 1_000_000;
        NEXT = now.saturating_add(frequency / 60);
    }
    true
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: animation_due
// DESC: Provides a conservative frame cadence on targets without a hardware timer path.
// ------------------=
fn animation_due() -> bool {
    false
}

#[cfg(target_arch = "aarch64")]
// ------------------------=
// FUNC: wait_frame
// DESC: Uses the AArch64 architectural counter for stable animation timing independent of CPU speed.
// ------------------=
pub(super) fn wait_frame(milliseconds: u64) {
    let start: u64;
    let frequency: u64;
    unsafe {
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) start);
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) frequency);
    }
    let target = start.saturating_add(frequency.saturating_mul(milliseconds) / 1000);
    loop {
        let now: u64;
        unsafe {
            core::arch::asm!("mrs {}, cntvct_el0", out(reg) now);
        }
        if now >= target {
            break;
        }
        core::hint::spin_loop();
    }
}

#[cfg(target_arch = "x86_64")]
// ------------------------=
// FUNC: wait_frame
// DESC: Uses the x86 time-stamp counter for stable animation timing independent of CPU speed.
// ------------------=
pub(super) fn wait_frame(milliseconds: u64) {
    let megahertz = core::arch::x86_64::__cpuid(0x16).eax as u64;
    let megahertz = megahertz.max(1000);
    let start = unsafe { core::arch::x86_64::_rdtsc() };
    let target = start.saturating_add(megahertz.saturating_mul(1000).saturating_mul(milliseconds));
    while unsafe { core::arch::x86_64::_rdtsc() } < target {
        core::hint::spin_loop();
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: show_splash
// DESC: Runs the graphical boot splash and hands the framebuffer to the console.
// ------------------=
pub fn show_splash(info: &BootInfo) -> bool {
    let Some(mut display) = DisplayDevice::from_boot_info(info) else {
        return false;
    };
    display.mark_dirty_rect(0, 0, display.width, display.height);
    display.paint_background();
    let stages: [(usize, &[u8]); 4] = [
        (25, b"GRAPHICS SURFACE READY"),
        (50, b"KEYBOARD DRIVER PROBE"),
        (75, b"MOUSE DRIVER PROBE"),
        (100, b"INFINITYOS READY"),
    ];
    for (stage_index, (stage, label)) in stages.into_iter().enumerate() {
        for frame in 0..30 {
            let sequence = stage_index * 30 + frame;
            display.emblem_reveal_band(sequence, 120);
            display.progress(stage.saturating_sub(24) + frame * 24 / 30, label);
            display.present_damage();
            wait_frame(20);
        }
    }
    // Keep the completed bootstrap composition intact for the startup menu.
    // The BBS panel owns its lower rectangle, while particle restoration above
    // it now reads from the same boot artwork and full-size emblem geometry.
    display.finish_emblem();
    display.force_full_present();
    activate_console(display);
    true
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: show_splash
// DESC: Reports that no graphical splash is available on unsupported targets.
// ------------------=
pub fn show_splash(_info: &BootInfo) -> bool {
    false
}
