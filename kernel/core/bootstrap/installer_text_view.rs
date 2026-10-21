//! Shared production installer font assets and typography; no service or console state.
use super::{DisplayDevice, FontSize};

pub(super) const INSTALLER_FONT_NATIVE_SIZE_PX: usize = 24;
pub(super) const INSTALLER_FONT_SIZE_PX: usize = 24;
pub(super) const INSTALLER_HEADLINE_FONT_NATIVE_SIZE_PX: usize = 32;
pub(super) const INSTALLER_HEADLINE_FONT_SIZE_PX: usize = 32;
pub(super) const INSTALLER_COMPACT_FONT_NATIVE_SIZE_PX: usize = 19;
pub(super) const INSTALLER_COMPACT_FONT_SIZE_PX: usize = 19;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const INSTALLER_FONT_ATLAS: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityInstaller-Regular-24.atlas");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const INSTALLER_FONT_SEMIBOLD_ATLAS: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityInstaller-Semibold-24.atlas");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const INSTALLER_FONT_METRICS: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityInstaller-Regular-24.metrics");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const INSTALLER_FONT_SEMIBOLD_METRICS: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityInstaller-Semibold-24.metrics");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const INSTALLER_FONT_KERN: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityInstaller-Regular-24.kern");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const INSTALLER_FONT_SEMIBOLD_KERN: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityInstaller-Semibold-24.kern");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const INSTALLER_HEADLINE_FONT_ATLAS: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityInstaller-Semibold-32.atlas");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const INSTALLER_HEADLINE_FONT_METRICS: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityInstaller-Semibold-32.metrics");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const INSTALLER_HEADLINE_FONT_KERN: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityInstaller-Semibold-32.kern");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const INSTALLER_COMPACT_FONT_ATLAS: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityInstaller-Regular-19.atlas");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const INSTALLER_COMPACT_FONT_SEMIBOLD_ATLAS: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityInstaller-Semibold-19.atlas");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const INSTALLER_COMPACT_FONT_METRICS: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityInstaller-Regular-19.metrics");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const INSTALLER_COMPACT_FONT_SEMIBOLD_METRICS: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityInstaller-Semibold-19.metrics");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const INSTALLER_COMPACT_FONT_KERN: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityInstaller-Regular-19.kern");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const INSTALLER_COMPACT_FONT_SEMIBOLD_KERN: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityInstaller-Semibold-19.kern");
#[cfg(target_arch = "x86")]
pub(super) const INSTALLER_FONT_ATLAS: &[u8] = &[];
#[cfg(target_arch = "x86")]
pub(super) const INSTALLER_FONT_SEMIBOLD_ATLAS: &[u8] = &[];
#[cfg(target_arch = "x86")]
pub(super) const INSTALLER_FONT_METRICS: &[u8] = &[];
#[cfg(target_arch = "x86")]
pub(super) const INSTALLER_FONT_SEMIBOLD_METRICS: &[u8] = &[];
#[cfg(target_arch = "x86")]
pub(super) const INSTALLER_FONT_KERN: &[u8] = &[];
#[cfg(target_arch = "x86")]
pub(super) const INSTALLER_FONT_SEMIBOLD_KERN: &[u8] = &[];
#[cfg(target_arch = "x86")]
pub(super) const INSTALLER_COMPACT_FONT_ATLAS: &[u8] = &[];
#[cfg(target_arch = "x86")]
pub(super) const INSTALLER_COMPACT_FONT_SEMIBOLD_ATLAS: &[u8] = &[];
#[cfg(target_arch = "x86")]
pub(super) const INSTALLER_COMPACT_FONT_METRICS: &[u8] = &[];
#[cfg(target_arch = "x86")]
pub(super) const INSTALLER_COMPACT_FONT_SEMIBOLD_METRICS: &[u8] = &[];
#[cfg(target_arch = "x86")]
pub(super) const INSTALLER_COMPACT_FONT_KERN: &[u8] = &[];
#[cfg(target_arch = "x86")]
pub(super) const INSTALLER_COMPACT_FONT_SEMIBOLD_KERN: &[u8] = &[];
pub(super) const INSTALLER_FONT_CELL_WIDTH: usize = 24;
pub(super) const INSTALLER_FONT_CELL_HEIGHT: usize = 28;
pub(super) const INSTALLER_HEADLINE_FONT_CELL_WIDTH: usize = 32;
pub(super) const INSTALLER_HEADLINE_FONT_CELL_HEIGHT: usize = 38;
pub(super) const INSTALLER_COMPACT_FONT_CELL_WIDTH: usize = 20;
pub(super) const INSTALLER_COMPACT_FONT_CELL_HEIGHT: usize = 24;
impl DisplayDevice {
    // ------------------------=
    // FUNC: installer_text_width
    // DESC: Measures installer copy at the reference panel's fixed antialiased type size.
    // ------------------=
    pub(super) fn installer_text_width(&self, text: &[u8], semibold: bool) -> usize {
        let font_size = FontSize::new(INSTALLER_FONT_NATIVE_SIZE_PX, INSTALLER_FONT_SIZE_PX);
        let metrics = if semibold {
            INSTALLER_FONT_SEMIBOLD_METRICS
        } else {
            INSTALLER_FONT_METRICS
        };
        let kerning = if semibold {
            INSTALLER_FONT_SEMIBOLD_KERN
        } else {
            INSTALLER_FONT_KERN
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
    // FUNC: installer_text
    // DESC: Draws installer body copy with the fixed-size smooth reference typography.
    // ------------------=
    pub(super) fn installer_text(
        &mut self,
        x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
    ) {
        self.installer_text_weighted(x, y, text, red, green, blue, false);
    }

    // ------------------------=
    // FUNC: installer_text_strong
    // DESC: Draws installer headings with the fixed-size smooth semibold reference typography.
    // ------------------=
    pub(super) fn installer_text_strong(
        &mut self,
        x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
    ) {
        self.installer_text_weighted(x, y, text, red, green, blue, true);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: installer_headline_text_width
    // DESC: Measures the larger semibold face reserved for installer hero statements.
    // ------------------=
    pub(super) fn installer_headline_text_width(&self, text: &[u8]) -> usize {
        let font_size = FontSize::new(
            INSTALLER_HEADLINE_FONT_NATIVE_SIZE_PX,
            INSTALLER_HEADLINE_FONT_SIZE_PX,
        );
        let mut width = 0usize;
        let mut previous = None;
        for byte in text {
            if !(32..=126).contains(byte) {
                previous = None;
                continue;
            }
            width = Self::font_position_advance(
                width,
                font_size.scale_isize(Self::font_pair_adjustment(
                    INSTALLER_HEADLINE_FONT_KERN,
                    previous,
                    *byte,
                )),
            );
            width =
                width
                    .saturating_add(font_size.scale_usize(
                        INSTALLER_HEADLINE_FONT_METRICS[*byte as usize - 32] as usize,
                    ));
            previous = Some(*byte);
        }
        width
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: installer_headline_text
    // DESC: Alpha-rasterizes the 32px semibold installer hero face.
    // ------------------=
    pub(super) fn installer_headline_text(
        &mut self,
        mut x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
    ) {
        let font_size = FontSize::new(
            INSTALLER_HEADLINE_FONT_NATIVE_SIZE_PX,
            INSTALLER_HEADLINE_FONT_SIZE_PX,
        );
        let glyph_width = font_size
            .scale_usize(INSTALLER_HEADLINE_FONT_CELL_WIDTH)
            .max(1);
        let glyph_height = font_size
            .scale_usize(INSTALLER_HEADLINE_FONT_CELL_HEIGHT)
            .max(1);
        let mut previous = None;
        for byte in text {
            if !(32..=126).contains(byte) {
                previous = None;
                continue;
            }
            x = Self::font_position_advance(
                x,
                font_size.scale_isize(Self::font_pair_adjustment(
                    INSTALLER_HEADLINE_FONT_KERN,
                    previous,
                    *byte,
                )),
            );
            let glyph = (*byte as usize - 32) * INSTALLER_HEADLINE_FONT_CELL_WIDTH;
            for row in 0..glyph_height {
                let source_row = font_size
                    .source_index(row)
                    .min(INSTALLER_HEADLINE_FONT_CELL_HEIGHT - 1);
                for column in 0..glyph_width {
                    let source_column = font_size
                        .source_index(column)
                        .min(INSTALLER_HEADLINE_FONT_CELL_WIDTH - 1);
                    let alpha = INSTALLER_HEADLINE_FONT_ATLAS[source_row
                        * INSTALLER_HEADLINE_FONT_CELL_WIDTH
                        * 95
                        + glyph
                        + source_column];
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
            x = x.saturating_add(
                font_size
                    .scale_usize(INSTALLER_HEADLINE_FONT_METRICS[*byte as usize - 32] as usize),
            );
            previous = Some(*byte);
        }
    }

    // ------------------------=
    // FUNC: installer_text_weighted
    // DESC: Alpha-rasterizes installer text without inheriting desktop DPI multiplication.
    // ------------------=
    pub(super) fn installer_text_weighted(
        &mut self,
        mut x: usize,
        mut y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        semibold: bool,
    ) {
        let line_start = x;
        let font_size = FontSize::new(INSTALLER_FONT_NATIVE_SIZE_PX, INSTALLER_FONT_SIZE_PX);
        let atlas = if semibold {
            INSTALLER_FONT_SEMIBOLD_ATLAS
        } else {
            INSTALLER_FONT_ATLAS
        };
        let metrics = if semibold {
            INSTALLER_FONT_SEMIBOLD_METRICS
        } else {
            INSTALLER_FONT_METRICS
        };
        let kerning = if semibold {
            INSTALLER_FONT_SEMIBOLD_KERN
        } else {
            INSTALLER_FONT_KERN
        };
        let glyph_width = font_size.scale_usize(INSTALLER_FONT_CELL_WIDTH).max(1);
        let glyph_height = font_size.scale_usize(INSTALLER_FONT_CELL_HEIGHT).max(1);
        let line_advance = glyph_height.saturating_add(font_size.scale_usize(4));
        let mut previous = None;
        for byte in text {
            if *byte == b'\n' {
                x = line_start;
                y = y.saturating_add(line_advance);
                previous = None;
                continue;
            }
            if !(32..=126).contains(byte) {
                previous = None;
                continue;
            }
            x = Self::font_position_advance(
                x,
                font_size.scale_isize(Self::font_pair_adjustment(kerning, previous, *byte)),
            );
            let glyph = (*byte as usize - 32) * INSTALLER_FONT_CELL_WIDTH;
            for row in 0..glyph_height {
                let source_row = font_size
                    .source_index(row)
                    .min(INSTALLER_FONT_CELL_HEIGHT - 1);
                for column in 0..glyph_width {
                    let source_column = font_size
                        .source_index(column)
                        .min(INSTALLER_FONT_CELL_WIDTH - 1);
                    let alpha =
                        atlas[source_row * INSTALLER_FONT_CELL_WIDTH * 95 + glyph + source_column];
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
    // FUNC: installer_text_wrapped
    // DESC: Wraps installer copy into a bounded card using the reference's compact line rhythm.
    // ------------------=
    pub(super) fn installer_text_wrapped(
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
                if self.installer_text_width(&text[start..end], false) > max_width {
                    end = last_space.unwrap_or(end.saturating_sub(1).max(start + 1));
                    break;
                }
                if end == text.len() {
                    break;
                }
                end += 1;
            }
            self.installer_text(
                x,
                y + line
                    * (FontSize::new(INSTALLER_FONT_NATIVE_SIZE_PX, INSTALLER_FONT_SIZE_PX)
                        .scale_usize(INSTALLER_FONT_CELL_HEIGHT + 2)),
                &text[start..end],
                red,
                green,
                blue,
            );
            start = end;
            line += 1;
        }
    }

    // ------------------------=
    // FUNC: installer_compact_text_width
    // DESC: Measures the smaller Roboto companion used only by dense four-column information cards.
    // ------------------=
    pub(super) fn installer_compact_text_width(&self, text: &[u8], semibold: bool) -> usize {
        let font_size = FontSize::new(
            INSTALLER_COMPACT_FONT_NATIVE_SIZE_PX,
            INSTALLER_COMPACT_FONT_SIZE_PX,
        );
        let metrics = if semibold {
            INSTALLER_COMPACT_FONT_SEMIBOLD_METRICS
        } else {
            INSTALLER_COMPACT_FONT_METRICS
        };
        let kerning = if semibold {
            INSTALLER_COMPACT_FONT_SEMIBOLD_KERN
        } else {
            INSTALLER_COMPACT_FONT_KERN
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
    // FUNC: installer_compact_text
    // DESC: Draws compact card body copy with the 19px Roboto companion face.
    // ------------------=
    pub(super) fn installer_compact_text(
        &mut self,
        x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
    ) {
        self.installer_compact_text_weighted(x, y, text, red, green, blue, false);
    }

    // ------------------------=
    // FUNC: installer_compact_text_strong
    // DESC: Draws compact card labels with the 19px Roboto Medium companion face.
    // ------------------=
    pub(super) fn installer_compact_text_strong(
        &mut self,
        x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
    ) {
        self.installer_compact_text_weighted(x, y, text, red, green, blue, true);
    }

    // ------------------------=
    // FUNC: installer_compact_text_weighted
    // DESC: Alpha-rasterizes one compact Roboto weight with proportional metrics and pair kerning.
    // ------------------=
    pub(super) fn installer_compact_text_weighted(
        &mut self,
        mut x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        semibold: bool,
    ) {
        let font_size = FontSize::new(
            INSTALLER_COMPACT_FONT_NATIVE_SIZE_PX,
            INSTALLER_COMPACT_FONT_SIZE_PX,
        );
        let atlas = if semibold {
            INSTALLER_COMPACT_FONT_SEMIBOLD_ATLAS
        } else {
            INSTALLER_COMPACT_FONT_ATLAS
        };
        let metrics = if semibold {
            INSTALLER_COMPACT_FONT_SEMIBOLD_METRICS
        } else {
            INSTALLER_COMPACT_FONT_METRICS
        };
        let kerning = if semibold {
            INSTALLER_COMPACT_FONT_SEMIBOLD_KERN
        } else {
            INSTALLER_COMPACT_FONT_KERN
        };
        let glyph_width = font_size
            .scale_usize(INSTALLER_COMPACT_FONT_CELL_WIDTH)
            .max(1);
        let glyph_height = font_size
            .scale_usize(INSTALLER_COMPACT_FONT_CELL_HEIGHT)
            .max(1);
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
            let glyph = (*byte as usize - 32) * INSTALLER_COMPACT_FONT_CELL_WIDTH;
            for row in 0..glyph_height {
                let source_row = font_size
                    .source_index(row)
                    .min(INSTALLER_COMPACT_FONT_CELL_HEIGHT - 1);
                for column in 0..glyph_width {
                    let source_column = font_size
                        .source_index(column)
                        .min(INSTALLER_COMPACT_FONT_CELL_WIDTH - 1);
                    let alpha = atlas[source_row * INSTALLER_COMPACT_FONT_CELL_WIDTH * 95
                        + glyph
                        + source_column];
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
    // FUNC: installer_compact_text_wrapped
    // DESC: Wraps compact card copy without shrinking or clipping individual glyphs.
    // ------------------=
    pub(super) fn installer_compact_text_wrapped(
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
                if self.installer_compact_text_width(&text[start..end], false) > max_width {
                    end = last_space.unwrap_or(end.saturating_sub(1).max(start + 1));
                    break;
                }
                if end == text.len() {
                    break;
                }
                end += 1;
            }
            self.installer_compact_text(
                x,
                y + line
                    * FontSize::new(
                        INSTALLER_COMPACT_FONT_NATIVE_SIZE_PX,
                        INSTALLER_COMPACT_FONT_SIZE_PX,
                    )
                    .scale_usize(INSTALLER_COMPACT_FONT_CELL_HEIGHT + 1),
                &text[start..end],
                red,
                green,
                blue,
            );
            start = end;
            line += 1;
        }
    }

}
