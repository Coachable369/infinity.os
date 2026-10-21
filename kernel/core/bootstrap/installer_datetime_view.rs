//! Live installer date/time painter shared by native rendering and framebuffer verification.
use super::DisplayDevice;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
use super::installer::DATE_TIME_WORLD_BMP;

impl DisplayDevice {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: installer_date_time_panel
    // DESC: Draws the functional date, time, and typed time-zone installer screen.
    // ------------------=
    pub(super) fn installer_date_time_panel(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        date_time: crate::storage::DateTimeConfiguration,
        active_part: usize,
        focus: usize,
        cursor_x: i32,
        cursor_y: i32,
        pressed: bool,
    ) {
        let scale = self.ui_scale();
        use super::access_style as palette;
        use crate::ui::{geometry::Rect, panel_recipe::{Material, Surface}};
        // restore_installer_panel has already restored the wallpaper for this
        // content repaint, so blur cannot accumulate over previous controls.
        self.access_frost_panel(left, top, width, height,
            palette::SURFACE, palette::BORDER, 232, 255);
        let material = Material { fill: palette::SURFACE, edge: palette::BORDER,
            opacity: 212, edge_opacity: 255, focus: 0, pressed: false, scale };

        let inset = width * 20 / 1000;
        let header_y = top + height * 27 / 1000;
        let screen_title = crate::ui::installer_layout::installer_template_text(
            5,
            crate::ui::installer_template::InstallerTemplateRole::Title,
        )
        .unwrap_or(b"DATE & TIME");
        self.installer_text_strong(left + inset, header_y, screen_title, 220, 230, 241);
        let section = b"GUIDED SETUP";
        let section_width = self.installer_text_width(section, true);
        self.installer_text_strong(
            left + width.saturating_sub(inset + section_width),
            header_y,
            section,
            52,
            198,
            246,
        );
        let divider_y = top + height * 86 / 1000;
        self.fill_rect(
            left + inset,
            divider_y,
            width.saturating_sub(inset * 2),
            scale,
            20,
            88,
            124,
        );

        let form_left = self.width * 10 / 100;
        let form_top = self.height * 405 / 1000;
        let form_width = self.width * 38 / 100;
        let form_height = self.height * 365 / 1000;
        self.precision_surface(Rect { x: form_left as i32, y: form_top as i32,
            width: form_width as u32, height: form_height as u32 }, Surface::Panel, material);
        self.installer_headline_text(
            form_left + 20 * scale,
            form_top + 20 * scale,
            b"SET YOUR LOCAL TIME",
            244,
            249,
            254,
        );
        let body = crate::ui::installer_layout::installer_template_text(
            5,
            crate::ui::installer_template::InstallerTemplateRole::Body,
        )
        .unwrap_or(b"Used by your clock, calendar, events, and activity history.");
        self.installer_text_wrapped(
            form_left + 20 * scale,
            form_top + 58 * scale,
            form_width.saturating_sub(40 * scale),
            body,
            172,
            195,
            215,
            2,
        );

        let mut date = *b"0000-00-00";
        date[0] = b'0' + ((date_time.year / 1000) % 10) as u8;
        date[1] = b'0' + ((date_time.year / 100) % 10) as u8;
        date[2] = b'0' + ((date_time.year / 10) % 10) as u8;
        date[3] = b'0' + (date_time.year % 10) as u8;
        Self::installer_two_digits(&mut date, 5, date_time.month);
        Self::installer_two_digits(&mut date, 8, date_time.day);
        let mut time = *b"00:00";
        Self::installer_two_digits(&mut time, 0, date_time.hour);
        Self::installer_two_digits(&mut time, 3, date_time.minute);
        let fields: [(&[u8], &[u8], usize, usize); 3] = [
            (b"DATE", &date, 465, 2),
            (b"TIME", &time, 555, 3),
            (
                b"TIME ZONE",
                Self::installer_time_zone_label(date_time.time_zone_id),
                645,
                4,
            ),
        ];
        for (label, value, normalized_y, field_focus) in fields {
            let field_top = self.height * normalized_y / 1000;
            let field_height = self.height * 70 / 1000;
            let focused = focus == field_focus;
            let hovered = (100..=475).contains(&cursor_x)
                && (normalized_y as i32..=(normalized_y + 70) as i32).contains(&cursor_y);
            self.precision_surface(Rect { x: (self.width * 10 / 100) as i32,
                y: field_top as i32, width: (self.width * 38 / 100) as u32,
                height: field_height as u32 }, Surface::Field,
                Material { fill: palette::BACKGROUND,
                    opacity: if hovered && pressed { 250 } else { 226 },
                    focus: if focused || hovered { 255 } else { 0 },
                    pressed: hovered && pressed, ..material });
            self.installer_compact_text_strong(
                self.width * 115 / 1000,
                field_top + 8 * scale,
                label,
                60,
                199,
                244,
            );
            self.installer_text(
                self.width * 17 / 100,
                field_top + 29 * scale,
                value,
                232,
                241,
                250,
            );
            self.installer_text_strong(
                self.width * 125 / 1000,
                field_top + 28 * scale,
                b"-",
                77,
                203,
                246,
            );
            self.installer_text_strong(
                self.width * 44 / 100,
                field_top + 28 * scale,
                b"+",
                77,
                203,
                246,
            );
        }

        let active_y = if focus == 2 {
            465
        } else if focus == 3 {
            555
        } else {
            645
        };
        if focus == 2 || focus == 3 {
            let (part_x, part_width) = if focus == 2 {
                match active_part.min(2) {
                    0 => (170, 82),
                    1 => (270, 43),
                    _ => (330, 43),
                }
            } else if active_part == 4 {
                (305, 48)
            } else {
                (220, 48)
            };
            self.fill_rect(
                self.width * part_x / 1000,
                self.height * (active_y + 57) / 1000,
                self.width * part_width / 1000,
                2 * scale,
                63,
                202,
                246,
            );
        }

        let art_left = self.width * 50 / 100;
        let art_top = self.height * 405 / 1000;
        let art_width = self.width * 40 / 100;
        let art_height = self.height * 365 / 1000;
        self.precision_surface(Rect { x: art_left as i32, y: art_top as i32,
            width: art_width as u32, height: art_height as u32 }, Surface::Panel,
            Material { opacity: 226, edge_opacity: 0, ..material });
        let map_width = art_width.saturating_sub(8 * scale);
        let map_height = (map_width / 2).min(art_height.saturating_sub(8 * scale));
        let map_left = art_left + (art_width.saturating_sub(map_width)) / 2;
        let map_top = art_top + (art_height.saturating_sub(map_height)) / 2;
        self.paint_bitmap_fit_rect(
            DATE_TIME_WORLD_BMP,
            map_left,
            map_top,
            map_width,
            map_height,
        );
        let (longitude, latitude) =
            Self::installer_time_zone_map_coordinates(date_time.time_zone_id);
        let marker_x = map_left as i32 + (longitude as i32 + 180) * map_width as i32 / 360;
        let marker_y = map_top as i32 + (90 - latitude as i32) * map_height as i32 / 180;
        let band_width = (map_width / 24).max(4 * scale);
        let band_left = (marker_x - band_width as i32 / 2)
            .max(map_left as i32)
            .min((map_left + map_width.saturating_sub(band_width)) as i32)
            as usize;
        self.fill_rect_alpha(band_left, map_top, band_width, map_height, 32, 173, 238, 54);
        self.fill_rect(band_left, map_top, scale, map_height, 63, 196, 241);
        self.fill_rect(
            band_left + band_width.saturating_sub(scale),
            map_top,
            scale,
            map_height,
            63,
            196,
            241,
        );
        self.fill_rect(
            map_left,
            marker_y.max(map_top as i32) as usize,
            map_width,
            scale,
            45,
            139,
            190,
        );
        self.star_orb(marker_x, marker_y, 4 * scale as i32, 245, true);
        self.fill_rect_alpha(map_left, map_top, map_width, 45 * scale,
            palette::BACKGROUND.0, palette::BACKGROUND.1, palette::BACKGROUND.2, 196);
        self.fill_rect_alpha(
            map_left,
            map_top + map_height.saturating_sub(38 * scale),
            map_width,
            38 * scale,
            palette::BACKGROUND.0,
            palette::BACKGROUND.1,
            palette::BACKGROUND.2,
            208,
        );
        self.precision_surface(Rect { x: art_left as i32, y: art_top as i32,
            width: art_width as u32, height: art_height as u32 }, Surface::Panel,
            Material { opacity: 0, ..material });
        self.installer_text_strong(
            art_left + 22 * scale,
            art_top + 20 * scale,
            b"SELECT YOUR TIME ZONE",
            64,
            202,
            247,
        );
        self.installer_text(
            art_left + 22 * scale,
            art_top + art_height.saturating_sub(31 * scale),
            Self::installer_time_zone_label(date_time.time_zone_id),
            184,
            204,
            221,
        );
        let zone_label = Self::installer_time_zone_label(date_time.time_zone_id);
        let offset = &zone_label[..zone_label.len().min(9)];
        let offset_width = self.installer_text_width(offset, false) + 18 * scale;
        let offset_left = art_left + art_width.saturating_sub(22 * scale + offset_width);
        let offset_top = art_top + art_height.saturating_sub(38 * scale);
        self.precision_surface(Rect { x: offset_left as i32, y: offset_top as i32,
            width: offset_width as u32, height: (27 * scale) as u32 }, Surface::Field,
            Material { fill: palette::BACKGROUND, opacity: 255, ..material });
        self.installer_text(
            offset_left + 9 * scale,
            offset_top + 6 * scale,
            offset,
            156,
            232,
            255,
        );
        self.installer_text(
            left + width.saturating_sub(inset + self.installer_text_width(b"05", false)),
            self.height * 80 / 100,
            b"05",
            105,
            194,
            232,
        );
    }

    // ------------------------=
    // FUNC: installer_two_digits
    // DESC: Writes one zero-padded two-digit value into a fixed installer label.
    // ------------------=
    pub(super) fn installer_two_digits(output: &mut [u8], offset: usize, value: u8) {
        output[offset] = b'0' + (value / 10) % 10;
        output[offset + 1] = b'0' + value % 10;
    }

    // ------------------------=
    // FUNC: installer_time_zone_label
    // DESC: Projects a stable time-zone ID into its human-readable installer label.
    // ------------------=
    pub(super) fn installer_time_zone_label(id: u16) -> &'static [u8] {
        match id {
            1 => b"UTC-08:00  Pacific",
            2 => b"UTC-07:00  Mountain",
            3 => b"UTC-06:00  Central",
            4 => b"UTC-05:00  Eastern",
            5 => b"UTC-04:00  Atlantic",
            7 => b"UTC+01:00  Central Europe",
            8 => b"UTC+05:30  India",
            9 => b"UTC+08:00  Singapore",
            10 => b"UTC+09:00  Japan",
            11 => b"UTC+10:00  Eastern Australia",
            _ => b"UTC+00:00  Universal",
        }
    }

    // ------------------------=
    // FUNC: installer_time_zone_map_coordinates
    // DESC: Returns a representative longitude and latitude for a typed time-zone marker.
    // ------------------=
    pub(super) fn installer_time_zone_map_coordinates(id: u16) -> (i16, i16) {
        match id {
            1 => (-122, 37),
            2 => (-111, 40),
            3 => (-95, 40),
            4 => (-74, 40),
            5 => (-63, 45),
            7 => (10, 50),
            8 => (78, 22),
            9 => (104, 1),
            10 => (139, 36),
            11 => (151, -33),
            _ => (0, 51),
        }
    }

}
