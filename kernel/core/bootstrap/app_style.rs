//! Compact native-app typography and surfaces from the editor IDesign kit.
use crate::ui::geometry::Rect;
pub(super) const TEXT: (u8, u8, u8) = (255, 255, 255);
pub(super) const MUTED: (u8, u8, u8) = (159, 176, 200);
pub(super) const CYAN: (u8, u8, u8) = (34, 211, 238);
pub(super) const FONT: &[u8] = include_bytes!("../../../assets/fonts/InfinityUI-Regular-16.atlas");
pub(super) const MEDIUM: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityUI-Semibold-16.atlas");
const METRICS: &[u8] = include_bytes!("../../../assets/fonts/InfinityUI-Regular-16.metrics");
const STRONG_METRICS: &[u8] =
    include_bytes!("../../../assets/fonts/InfinityUI-Semibold-16.metrics");
const KERN: &[u8] = include_bytes!("../../../assets/fonts/InfinityUI-Regular-16.kern");
const STRONG_KERN: &[u8] = include_bytes!("../../../assets/fonts/InfinityUI-Semibold-16.kern");
impl super::DisplayDevice {
    // ------------------------=
    // FUNC: app_text_width
    // DESC: Measures compact app typography with the same advances and kerning used for painting.
    // ------------------=
    pub(super) fn app_text_width(&self, text: &[u8], strong: bool, scale: usize) -> usize {
        let metrics = if strong { STRONG_METRICS } else { METRICS };
        let kern = if strong { STRONG_KERN } else { KERN };
        let mut width = 0;
        let mut previous = None;
        for &c in text {
            if !(32..=126).contains(&c) {
                continue;
            }
            width = Self::font_position_advance(
                width,
                Self::font_pair_adjustment(kern, previous, c) * scale as isize,
            );
            width += metrics[c as usize - 32] as usize * scale;
            previous = Some(c);
        }
        width
    }
    // ------------------------=
    // FUNC: app_text
    // DESC: Paints the kit's antialiased 16 px regular and medium fonts without changing unrelated system typography.
    // ------------------=
    pub(super) fn app_text(
        &mut self,
        mut x: usize,
        y: usize,
        text: &[u8],
        color: (u8, u8, u8),
        strong: bool,
        scale: usize,
    ) {
        let atlas = if strong { MEDIUM } else { FONT };
        let metrics = if strong { STRONG_METRICS } else { METRICS };
        let kern = if strong { STRONG_KERN } else { KERN };
        let mut previous = None;
        for &c in text {
            if !(32..=126).contains(&c) {
                continue;
            }
            x = Self::font_position_advance(
                x,
                Self::font_pair_adjustment(kern, previous, c) * scale as isize,
            );
            for row in 0..24 * scale {
                for col in 0..20 * scale {
                    let alpha = atlas[row / scale * 20 * 95 + (c as usize - 32) * 20 + col / scale];
                    if alpha != 0 {
                        self.blend_color(
                            (x + col) as i32,
                            (y + row) as i32,
                            color.0,
                            color.1,
                            color.2,
                            alpha,
                        );
                    }
                }
            }
            x += metrics[c as usize - 32] as usize * scale;
            previous = Some(c);
        }
    }
    // ------------------------=
    // FUNC: app_label
    // DESC: Fits one readable line inside a real control's gutter and clips overflow to its bounds.
    // ------------------=
    pub(super) fn app_label(
        &mut self,
        r: Rect,
        text: &[u8],
        color: (u8, u8, u8),
        strong: bool,
        s: usize,
    ) {
        let clip = self.render_clip;
        self.intersect_render_clip(
            r.x.max(0) as usize,
            r.y.max(0) as usize,
            r.width as usize,
            r.height as usize,
        );
        let mut n = text.len();
        while n > 0 && self.app_text_width(&text[..n], strong, s) > r.width as usize {
            n -= 1;
        }
        self.app_text(
            r.x.max(0) as usize,
            r.y.max(0) as usize + (r.height as usize).saturating_sub(24 * s) / 2,
            &text[..n],
            color,
            strong,
            s,
        );
        self.render_clip = clip;
    }
    // ------------------------=
    // FUNC: app_card
    // DESC: Draws the reference glass recipe with a curved blue top light and a soft one-pixel border.
    // ------------------=
    pub(super) fn app_card(
        &mut self,
        r: Rect,
        color: (u8, u8, u8),
        border: (u8, u8, u8),
        s: usize,
    ) {
        self.fill_rounded_rect_alpha(
            r.x.max(0) as usize,
            r.y.max(0) as usize,
            r.width as usize,
            r.height as usize,
            8 * s,
            color.0,
            color.1,
            color.2,
            255,
        );
        let radius = (8 * s).min(r.width as usize / 2).min(r.height as usize / 2);
        let light_height = (72 * s).min(r.height as usize);
        for row in 1..light_height {
            let edge_row = row.min((r.height as usize).saturating_sub(row + 1));
            let inset = if edge_row < radius {
                let dy = radius - edge_row;
                let mut dx = radius;
                while dx * dx + dy * dy > radius * radius {
                    dx -= 1;
                }
                radius - dx
            } else {
                1
            };
            let strength = (light_height - row) * 12 / light_height.max(1);
            self.fill_rect(
                r.x.max(0) as usize + inset,
                r.y.max(0) as usize + row,
                (r.width as usize).saturating_sub(inset * 2),
                1,
                color.0.saturating_add((strength / 3) as u8),
                color.1.saturating_add((strength * 2 / 3) as u8),
                color.2.saturating_add(strength as u8),
            );
        }
        self.outline_rounded_rect(
            r.x.max(0) as usize,
            r.y.max(0) as usize,
            r.width as usize,
            r.height as usize,
            8 * s,
            border.0,
            border.1,
            border.2,
        );
    }
    // ------------------------=
    // FUNC: app_symbol
    // DESC: Draws the kit's small functional line icons using the native vector primitive family.
    // ------------------=
    pub(super) fn app_symbol(&mut self, r: Rect, symbol: u8, color: (u8, u8, u8), s: usize) {
        let x = r.x + r.width as i32 / 2;
        let y = r.y + r.height as i32 / 2;
        let k = s as i32;
        let lines: &[(i32, i32, i32, i32)] = match symbol {
            b'x' => &[(-5, -5, 5, 5), (-5, 5, 5, -5)],
            b'+' => &[(-5, 0, 5, 0), (0, -5, 0, 5)],
            b'^' => &[(0, 7, 0, -7), (-5, -2, 0, -7), (0, -7, 5, -2)],
            b'-' => &[(-5, 0, 5, 0)],
            b'm' => &[
                (-5, -5, 5, -5),
                (5, -5, 5, 5),
                (5, 5, -5, 5),
                (-5, 5, -5, -5),
            ],
            b'd' => &[
                (-5, -7, 1, -7),
                (1, -7, 5, -3),
                (5, -3, 5, 7),
                (5, 7, -5, 7),
                (-5, 7, -5, -7),
                (1, -7, 1, -3),
                (1, -3, 5, -3),
                (-2, 1, 2, 1),
                (-2, 4, 2, 4),
            ],
            _ => &[],
        };
        for &(a, b, c, d) in lines {
            for offset in 0..s as i32 {
                self.line(
                    x + a * k + offset,
                    y + b * k,
                    x + c * k + offset,
                    y + d * k,
                    color.0,
                    color.1,
                    color.2,
                );
            }
        }
    }
    // ------------------------=
    // FUNC: app_ai_mark
    // DESC: Extends the native icon family with the reference's luminous four-point assistant mark.
    // ------------------=
    pub(super) fn app_ai_mark(&mut self, r: Rect) {
        let cx = r.x + r.width as i32 / 2;
        let cy = r.y + r.height as i32 / 2;
        let radius = (r.width.min(r.height) as i32 / 2).max(1);
        for y in -radius..=radius {
            for x in -radius..=radius {
                let distance = x * x + y * y;
                if distance < radius * radius {
                    let glow = ((radius * radius - distance) * 65 / (radius * radius)) as u8;
                    self.blend_color(cx + x, cy + y, 34, 211, 238, glow);
                }
                let ax = x.abs();
                let ay = y.abs();
                let arm = (radius - ax - ay).max(0);
                if arm > 0 && ax * ay * 7 < radius * radius / 2 {
                    self.blend_color(
                        cx + x,
                        cy + y,
                        150,
                        239,
                        255,
                        (arm * 255 / (radius / 3).max(1)).min(255) as u8,
                    );
                }
            }
        }
    }
    // ------------------------=
    // FUNC: app_button
    // DESC: Centers an enabled action in a consistently padded 32 px kit button.
    // ------------------=
    pub(super) fn app_button(&mut self, r: Rect, label: &[u8], primary: bool, s: usize) {
        self.app_card(
            r,
            if primary { CYAN } else { (15, 27, 46) },
            if primary {
                (81, 225, 249)
            } else {
                (33, 58, 85)
            },
            s,
        );
        let w = self.app_text_width(label, true, s);
        let x = r.x + (r.width as usize).saturating_sub(w) as i32 / 2;
        self.app_label(
            Rect {
                x,
                width: w as u32,
                ..r
            },
            label,
            if primary { (5, 25, 40) } else { TEXT },
            true,
            s,
        );
    }
}
