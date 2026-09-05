//! Initial boot emblem, progress, BBS menu, and console presentation.

use super::*;

impl super::DisplayDevice {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: emblem_reveal_band
    // DESC: Reveals one vertical animation band of the infinity emblem.
    // ------------------=
    pub(super) fn emblem_reveal_band(&mut self, step: usize, total: usize) {
        if EMBLEM_BMP.len() < 138 || &EMBLEM_BMP[0..2] != b"BM" {
            return;
        }
        let source_width = le32(EMBLEM_BMP, 18) as usize;
        let source_height = (le32(EMBLEM_BMP, 22) as i32).unsigned_abs() as usize;
        let target_width =
            (self.width * 52 / 100).min((self.height * 70 / 100) * source_width / source_height);
        let half = target_width / 2;
        let start = half * step / total;
        let end = half * (step + 1) / total;
        self.emblem_range(half.saturating_sub(end), half.saturating_sub(start), 255);
        self.emblem_range(half + start, (half + end).min(target_width), 255);
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
                let address = unsafe { self.buffer.add((top + y) * self.stride + left + x) };
                let current = unsafe { read_volatile(address) };
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
    pub(super) fn startup_bbs(&mut self, command: &[u8], cursor_x: i32, cursor_y: i32, pressed: bool) {
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

        let graffiti: [&[u8]; 6] = [
            b".___ _______  ___________.___ _______  .___________________.___.________    _________",
            b"|   |\\      \\ \\_   _____/|   |\\      \\ |   \\__    ___/\\__  |   |\\_____  \\  /   _____/",
            b"|   |/   |   \\ |    __)  |   |/   |   \\|   | |    |    /   |   | /   |   \\ \\_____  \\",
            b"|   /    |    \\|     \\   |   /    |    \\   | |    |    \\____   |/    |    \\/        \\",
            b"|___\\____|__  /\\___  /   |___\\____|__  /___| |____|    / ______|\\_______  /_______  /",
            b"            \\/     \\/                \\/                \\/               \\/        \\/",
        ];
        for (row, line) in graffiti.iter().enumerate() {
            let (red, green, blue) = if row == 0 || row == 5 {
                (143, 215, 255)
            } else if row & 1 == 0 {
                (248, 251, 255)
            } else {
                (214, 233, 249)
            };
            // Keep the wide FIGlet banner at its native cell size; ordinary
            // interface text below it uses the larger readable UI scale.
            self.text_scaled(
                text_left,
                top + 13 * scale + row * 11,
                line,
                red,
                green,
                blue,
                1,
                false,
            );
        }
        self.text(
            text_left,
            top + 82 * scale,
            b"--[ Welcome to infinityOS ] -- SYSTEM READY --",
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
            b"TAB / ARROWS: MOVE     ENTER: SELECT     TYPE: install | repair | console",
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
            (b"[1]  INSTALL INFINITYOS", b"Begin the installation", 735),
            (
                b"[2]  REPAIR AN EXISTING INSTALLATION",
                b"Inspect and repair",
                785,
            ),
            (b"[3]  OPEN SYSTEM CONSOLE", b"Enter system console", 835),
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
