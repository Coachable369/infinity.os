//! Shared production template typography for installer and Eclipse configuration scenes.
use super::{DisplayDevice, FontSize};
use super::access_style as access;
use super::installer::{INSTALLER_FONT_NATIVE_SIZE_PX, INSTALLER_FONT_CELL_WIDTH, INSTALLER_FONT_CELL_HEIGHT};
use super::template_font::face as template_face;

impl DisplayDevice {
    // ------------------------=
    // FUNC: with_template_typography
    // DESC: Scopes authored typography to one compositor layer and restores it after nested paints and early returns.
    // ------------------=
    pub(super) fn with_template_typography(&mut self, element: crate::ui::installer_template::InstallerTemplateElement<'_>,
        paint: impl FnOnce(&mut Self)) {
        let previous = self.template_typography;
        self.template_typography = [element.font_family, element.text_sharpness, element.text_glow];
        paint(self);
        self.template_typography = previous;
    }

    // ------------------------=
    // FUNC: template_family
    // DESC: Resolves an explicit packaged family while preserving the scene default for legacy documents.
    // ------------------=
    fn template_family(&self, fallback: Option<u8>) -> Option<u8> {
        if self.template_typography[0] == 0 { fallback } else { Some(self.template_typography[0]) }
    }
    // ------------------------=
    // FUNC: template_text_width
    // DESC: Measures template text at its authored pixel size using the packaged installer face.
    // ------------------=
    pub(crate) fn template_text_width(&self, text: &[u8], pixels: usize, semibold: bool) -> usize {
        self.template_text_width_face(text, pixels, semibold, None)
    }

    // ------------------------=
    // FUNC: access_text_width
    // DESC: Measures Eclipse text with the immutable Inter family regardless of the signed-in desktop font preference.
    // ------------------=
    pub(super) fn access_text_width(&self, text: &[u8], pixels: usize, semibold: bool) -> usize {
        self.template_text_width_face(text, pixels, semibold, Some(1))
    }

    // ------------------------=
    // FUNC: template_text_width_face
    // DESC: Uses one explicitly selected native face for advance and kerning while retaining legacy Roboto when no family is requested.
    // ------------------=
    fn template_text_width_face(&self, text: &[u8], pixels: usize, semibold: bool, family: Option<u8>) -> usize {
        let font_size = FontSize::new(INSTALLER_FONT_NATIVE_SIZE_PX, pixels.max(1));
        let (_, metrics, kerning) = template_face(self.template_family(family), semibold);
        let mut width = 0usize;
        let mut previous = None;
        for byte in text {
            if *byte == b'\n' {
                break;
            }
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
    // FUNC: template_text
    // DESC: Alpha-rasterizes template text at an arbitrary authored size.
    // ------------------=
    pub(crate) fn template_text(
        &mut self,
        x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        opacity: u8,
        pixels: usize,
        semibold: bool,
    ) {
        self.template_text_face(x, y, text, red, green, blue, opacity, pixels, semibold, None);
    }

    // ------------------------=
    // FUNC: access_text
    // DESC: Paints Eclipse labels using the same explicit Inter face that supplies their measurement and wrapping.
    // ------------------=
    pub(super) fn access_text(&mut self, x: usize, y: usize, text: &[u8], red: u8, green: u8,
        blue: u8, opacity: u8, pixels: usize, semibold: bool) {
        self.template_text_face(x, y, text, red, green, blue, opacity, pixels, semibold, Some(1));
    }

    // ------------------------=
    // FUNC: template_text_face
    // DESC: Rasterizes an explicit native family or the unchanged legacy installer face with matching glyph metrics.
    // ------------------=
    fn template_text_face(&mut self, x: usize, y: usize, text: &[u8], red: u8,
        green: u8, blue: u8, opacity: u8, pixels: usize, semibold: bool, family: Option<u8>) {
        self.template_text_face_signed(x as i64, y as i64, text, red, green, blue, opacity, pixels, semibold, family);
    }

    // ------------------------=
    // FUNC: template_text_signed
    // DESC: Preserves glyph positions when authored controls are partially outside the display or scrolling viewport.
    // ------------------=
    pub(super) fn template_text_signed(&mut self, x: i32, y: i32, text: &[u8], red: u8,
        green: u8, blue: u8, opacity: u8, pixels: usize, semibold: bool) {
        self.template_text_face_signed(i64::from(x), i64::from(y), text, red, green, blue, opacity, pixels, semibold, None);
    }

    // ------------------------=
    // FUNC: template_text_face_signed
    // DESC: Rasterizes a native face with signed origins and compositor clipping, sharing glyph coverage and metrics with ordinary template text.
    // ------------------=
    fn template_text_face_signed(&mut self, mut x: i64, mut y: i64, text: &[u8], red: u8,
        green: u8, blue: u8, opacity: u8, pixels: usize, semibold: bool, family: Option<u8>) {
        let line_start = x;
        let font_size = FontSize::new(INSTALLER_FONT_NATIVE_SIZE_PX, pixels.max(1));
        let (atlas, metrics, kerning) = template_face(self.template_family(family), semibold);
        let glyph_width = font_size.scale_usize(INSTALLER_FONT_CELL_WIDTH).max(1);
        let glyph_height = font_size.scale_usize(INSTALLER_FONT_CELL_HEIGHT).max(1);
        let line_advance = glyph_height.saturating_add(font_size.scale_usize(4));
        let mut previous = None;
        for byte in text {
            if *byte == b'\n' {
                x = line_start;
                y = y.saturating_add(line_advance as i64);
                previous = None;
                continue;
            }
            if !(32..=126).contains(byte) {
                previous = None;
                continue;
            }
            x = x.saturating_add(font_size.scale_isize(Self::font_pair_adjustment(kerning, previous, *byte)) as i64);
            let glyph = (*byte as usize - 32) * INSTALLER_FONT_CELL_WIDTH;
            for row in 0..glyph_height {
                for column in 0..glyph_width {
                    let atlas_alpha = crate::ui::glyph_filter::coverage(atlas,
                        INSTALLER_FONT_CELL_WIDTH * 95, glyph, INSTALLER_FONT_CELL_WIDTH,
                        INSTALLER_FONT_CELL_HEIGHT, glyph_width, glyph_height, column, row);
                    // Contrast-adjust coverage, leaving zero-effect legacy pixels unchanged.
                    let sharp = i32::from(self.template_typography[1]);
                    let coverage = if atlas_alpha == 0 { 0 } else {
                        (i32::from(atlas_alpha) + (i32::from(atlas_alpha) - 128) * sharp / 100).clamp(0,255) as u8
                    };
                    let alpha = (coverage as u16 * opacity as u16 / 255) as u8;
                    if self.template_typography[2] > 0 && alpha > 0 {
                        let glow = (u32::from(alpha) * u32::from(self.template_typography[2]) / 800) as u8;
                        let distance = (pixels / 24).clamp(1, 3) as i32;
                        for (dx,dy) in [(-distance,0),(distance,0),(0,-distance),(0,distance)] {
                            self.blend_color((x + column as i64) as i32 + dx,
                                (y + row as i64) as i32 + dy, red, green, blue, glow);
                        }
                    }
                    if alpha != 0 {
                        self.blend_color(
                            (x + column as i64) as i32,
                            (y + row as i64) as i32,
                            red,
                            green,
                            blue,
                            alpha,
                        );
                    }
                }
            }
            x = x.saturating_add(font_size.scale_usize(metrics[*byte as usize - 32] as usize) as i64);
            previous = Some(*byte);
        }
    }

    // ------------------------=
    // FUNC: template_text_wrapped
    // DESC: Wraps authored template copy to its frame while preserving explicit line breaks.
    // ------------------=
    pub(super) fn template_text_wrapped(
        &mut self,
        x: usize,
        y: usize,
        max_width: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        opacity: u8,
        pixels: usize,
        semibold: bool,
    ) {
        self.template_text_wrapped_face(x, y, max_width, text, red, green, blue, opacity, pixels, semibold, None);
    }

    // ------------------------=
    // FUNC: access_text_wrapped
    // DESC: Wraps Eclipse copy with Inter advances without changing legacy installer typography.
    // ------------------=
    pub(super) fn access_text_wrapped(&mut self, x: usize, y: usize, max_width: usize,
        text: &[u8], red: u8, green: u8, blue: u8, opacity: u8, pixels: usize, semibold: bool) {
        self.template_text_wrapped_face(x, y, max_width, text, red, green, blue, opacity, pixels, semibold, Some(1));
    }

    // ------------------------=
    // FUNC: template_text_wrapped_face
    // DESC: Keeps paragraph wrapping and final rasterization on the same explicit font selection.
    // ------------------=
    fn template_text_wrapped_face(&mut self, x: usize, y: usize, max_width: usize,
        text: &[u8], red: u8, green: u8, blue: u8, opacity: u8, pixels: usize,
        semibold: bool, family: Option<u8>) {
        let mut start = 0usize;
        let mut line = 0usize;
        let line_advance = FontSize::new(INSTALLER_FONT_NATIVE_SIZE_PX, pixels.max(1))
            .scale_usize(INSTALLER_FONT_CELL_HEIGHT + 4);
        while start < text.len() {
            while start < text.len() && text[start] == b' ' {
                start += 1;
            }
            if start >= text.len() {
                break;
            }
            let explicit_end = text[start..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map(|value| start + value);
            let paragraph_end = explicit_end.unwrap_or(text.len());
            let mut end = start + 1;
            let mut last_space = None;
            while end <= paragraph_end {
                if self.template_text_width_face(&text[start..end], pixels, semibold, family) > max_width {
                    end = last_space.unwrap_or(end.saturating_sub(1).max(start + 1));
                    break;
                }
                if end < paragraph_end && text[end] == b' ' {
                    last_space = Some(end);
                }
                if end == paragraph_end {
                    break;
                }
                end += 1;
            }
            self.template_text_face(
                x,
                y + line * line_advance,
                &text[start..end],
                red,
                green,
                blue,
                opacity,
                pixels,
                semibold,
                family,
            );
            line += 1;
            start = if end == paragraph_end && explicit_end.is_some() {
                end + 1
            } else {
                end
            };
        }
    }

    // ------------------------=
    // FUNC: access_label
    // DESC: Fits antialiased pre-session typography to its responsive control without changing hit geometry.
    // ------------------=
    pub(super) fn access_label(&mut self, bounds: (usize, usize, usize, usize), text: &[u8],
        pixels: usize, color: access::Rgb, centered: bool, strong: bool) {
        let (left, top, width, height) = bounds;
        let mut pixels = pixels.max(8).min(height * 6 / 7).max(1);
        while pixels > 8 && self.access_text_width(text, pixels, strong) > width {
            pixels -= 1;
        }
        let text_width = self.access_text_width(text, pixels, strong);
        let previous = self.render_clip;
        self.intersect_render_clip(left, top, width, height);
        self.access_text(left + if centered { width.saturating_sub(text_width) / 2 } else { 0 },
            top + height.saturating_sub(pixels * 7 / 6) / 2, text,
            color.0, color.1, color.2, 255, pixels, strong);
        self.render_clip = previous;
    }

}
