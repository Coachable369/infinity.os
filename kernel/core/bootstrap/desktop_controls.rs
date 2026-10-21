//! Shared production desktop controls for native framebuffer previews.
use super::DisplayDevice;
use super::primitives::{infinity_point,UI_FONT_CELL_HEIGHT};
impl DisplayDevice {
    // ------------------------=
    // FUNC: small_infinity_mark
    // DESC: Draws the native top-bar Infinity pill without baking UI into artwork.
    // ------------------=
    pub(super) fn small_infinity_mark(&mut self, center_x: usize, center_y: usize, size: usize) {
        let width = size.max(20) as i32;
        for phase in 0..384usize {
            let (px, py) = infinity_point(phase);
            let x = center_x as i32 + px * width / 224;
            let y = center_y as i32 + py * width / 224;
            self.blend_color(x, y, 225, 247, 255, 220);
            self.blend_color(x + 1, y, 78, 200, 255, 140);
        }
    }

    // ------------------------=
    // FUNC: text_field_caret
    // DESC: Draws the shared blinking insertion caret at the active bounded text index.
    // ------------------=
    pub(super) fn text_field_caret(
        &mut self,
        text_left: usize,
        top: usize,
        height: usize,
        input: &[u8],
        focused: bool,
        kind: usize,
    ) {
        self.text_field_caret_color(text_left, top, height, input, focused, kind, (112, 221, 255));
    }

    // ------------------------=
    // FUNC: text_field_caret_color
    // DESC: Shares insertion geometry while keeping the pre-session palette separate from desktop controls.
    // ------------------=
    pub(super) fn text_field_caret_color(&mut self, text_left: usize, top: usize, height: usize,
        input: &[u8], focused: bool, kind: usize, color: (u8,u8,u8)) {
        let Some((visible, index)) = crate::ui::text_input::caret(kind) else {
            return;
        };
        if !focused || !visible {
            return;
        }
        let index = index.min(input.len());
        let x = text_left + self.ui_text_width(&input[..index], 1);
        self.fill_rect(
            x,
            top + height / 2 - 10,
            self.ui_scale().max(1),
            20,
            color.0, color.1, color.2,
        );
    }

    // ------------------------=
    // FUNC: authentication_icon
    // DESC: Rasterizes the default skin's semantic authentication icons from scalable primitives.
    // ------------------=
    pub(super) fn authentication_icon(
        &mut self,
        center_x: usize,
        center_y: usize,
        kind: usize,
        size: usize,
        active: bool,
    ) {
        let role = [37usize, 0, 47, 46, 1, 17, 23, 26, 32, 34, 13, 28, 27, 17]
            .get(kind.saturating_sub(1))
            .copied();
        if let Some(role) = role {
            if active {
                let radius = size.max(8) / 4;
                self.fill_rounded_rect_alpha(
                    center_x.saturating_sub(size / 2 + 2),
                    center_y.saturating_sub(size / 2 + 2),
                    size + 4,
                    size + 4,
                    radius,
                    31,
                    142,
                    194,
                    72,
                );
            }
            if self.themed_icon(center_x, center_y, role, size.max(12)) {
                return;
            }
        }
        let color = if active {
            (139, 225, 255)
        } else {
            (226, 235, 243)
        };
        let half = size.max(8) as i32 / 2;
        let cx = center_x as i32;
        let cy = center_y as i32;
        macro_rules! stroke {
            ($x0:expr, $y0:expr, $x1:expr, $y1:expr, $red:expr, $green:expr, $blue:expr) => {
                self.icon_line($x0, $y0, $x1, $y1, ($red, $green, $blue), size)
            };
        }
        match kind {
            1 => {
                for level in 0..3i32 {
                    let span = half - level * half / 3;
                    let rise = span / 2;
                    stroke!(
                        cx - span,
                        cy - rise + level * half / 2,
                        cx,
                        cy + level * half / 3,
                        color.0,
                        color.1,
                        color.2
                    );
                    stroke!(
                        cx,
                        cy + level * half / 3,
                        cx + span,
                        cy - rise + level * half / 2,
                        color.0,
                        color.1,
                        color.2
                    );
                }
                self.icon_circle(cx, cy + half * 3 / 4, (half / 7).max(1), color, size);
            }
            2 => {
                self.icon_circle(cx, cy, half, (24, 178, 232), size);
                self.star_orb(cx, cy - half / 4, half / 3, 255, true);
                self.fill_rounded_rect_alpha(
                    (cx - half * 2 / 3).max(0) as usize,
                    cy as usize,
                    (half * 4 / 3) as usize,
                    (half * 2 / 3) as usize,
                    (half / 3).max(1) as usize,
                    73,
                    198,
                    241,
                    240,
                );
            }
            3 => {
                stroke!(
                    cx - half,
                    cy - half / 3,
                    cx,
                    cy + half / 3,
                    color.0,
                    color.1,
                    color.2
                );
                stroke!(
                    cx,
                    cy + half / 3,
                    cx + half,
                    cy - half / 3,
                    color.0,
                    color.1,
                    color.2
                );
            }
            4 => {
                stroke!(cx - half, cy, cx + half, cy, color.0, color.1, color.2);
                stroke!(
                    cx + half / 3,
                    cy - half / 2,
                    cx + half,
                    cy,
                    color.0,
                    color.1,
                    color.2
                );
                stroke!(
                    cx + half / 3,
                    cy + half / 2,
                    cx + half,
                    cy,
                    color.0,
                    color.1,
                    color.2
                );
            }
            5 => {
                self.outline_rounded_rect(
                    (cx - half * 2 / 3).max(0) as usize,
                    (cy - half / 6).max(0) as usize,
                    (half * 4 / 3) as usize,
                    half as usize,
                    (half / 6).max(1) as usize,
                    color.0,
                    color.1,
                    color.2,
                );
                self.icon_circle(cx, cy - half / 4, half / 2, color, size);
            }
            6 => {
                self.icon_circle(cx - half / 3, cy - half / 3, half / 3, color, size);
                stroke!(
                    cx - half,
                    cy + half,
                    cx - half / 3,
                    cy,
                    color.0,
                    color.1,
                    color.2
                );
                stroke!(
                    cx - half / 3,
                    cy,
                    cx + half / 3,
                    cy + half,
                    color.0,
                    color.1,
                    color.2
                );
                stroke!(
                    cx + half / 2,
                    cy - half / 4,
                    cx + half / 2,
                    cy + half / 2,
                    color.0,
                    color.1,
                    color.2
                );
                stroke!(
                    cx + half / 6,
                    cy + half / 8,
                    cx + half * 5 / 6,
                    cy + half / 8,
                    color.0,
                    color.1,
                    color.2
                );
            }
            7 => {
                self.icon_circle(cx, cy, half, color, size);
                self.icon_circle(cx, cy, half * 2 / 3, color, size);
                self.icon_circle(cx, cy, half / 3, color, size);
                stroke!(cx, cy - half, cx, cy + half, color.0, color.1, color.2);
            }
            8 => {
                self.icon_circle(cx, cy, half / 3, color, size);
                self.icon_circle(cx, cy, half * 3 / 4, color, size);
                for (dx, dy) in [(-half, 0), (half, 0), (0, -half), (0, half)] {
                    stroke!(
                        cx + dx * 2 / 3,
                        cy + dy * 2 / 3,
                        cx + dx,
                        cy + dy,
                        color.0,
                        color.1,
                        color.2
                    );
                }
            }
            9 => {
                self.icon_circle(cx, cy + half / 8, half * 3 / 4, color, size);
                stroke!(cx, cy - half, cx, cy + half / 6, color.0, color.1, color.2);
            }
            10 => {
                self.icon_circle(cx, cy, half * 3 / 4, color, size);
                stroke!(
                    cx - half,
                    cy - half / 4,
                    cx - half / 4,
                    cy - half / 2,
                    color.0,
                    color.1,
                    color.2
                );
                stroke!(
                    cx - half,
                    cy - half / 4,
                    cx - half * 3 / 4,
                    cy - half,
                    color.0,
                    color.1,
                    color.2
                );
            }
            11 => {
                stroke!(cx - half, cy, cx, cy - half / 2, color.0, color.1, color.2);
                stroke!(cx, cy - half / 2, cx + half, cy, color.0, color.1, color.2);
                stroke!(
                    cx - half / 2,
                    cy + half / 3,
                    cx,
                    cy,
                    color.0,
                    color.1,
                    color.2
                );
                stroke!(
                    cx,
                    cy,
                    cx + half / 2,
                    cy + half / 3,
                    color.0,
                    color.1,
                    color.2
                );
            }
            12 => {
                self.icon_circle(cx, cy - half * 2 / 3, half / 5, color, size);
                stroke!(cx, cy, cx, cy + half, color.0, color.1, color.2);
                stroke!(cx - half, cy, cx + half, cy, color.0, color.1, color.2);
                stroke!(
                    cx,
                    cy + half / 3,
                    cx - half * 2 / 3,
                    cy + half,
                    color.0,
                    color.1,
                    color.2
                );
                stroke!(
                    cx,
                    cy + half / 3,
                    cx + half * 2 / 3,
                    cy + half,
                    color.0,
                    color.1,
                    color.2
                );
            }
            13 => {
                stroke!(
                    cx - half,
                    cy,
                    cx - half / 3,
                    cy - half / 3,
                    color.0,
                    color.1,
                    color.2
                );
                stroke!(
                    cx - half / 3,
                    cy - half / 3,
                    cx + half / 3,
                    cy - half / 3,
                    color.0,
                    color.1,
                    color.2
                );
                stroke!(
                    cx + half / 3,
                    cy - half / 3,
                    cx + half,
                    cy,
                    color.0,
                    color.1,
                    color.2
                );
                stroke!(
                    cx + half,
                    cy,
                    cx + half / 3,
                    cy + half / 3,
                    color.0,
                    color.1,
                    color.2
                );
                stroke!(
                    cx + half / 3,
                    cy + half / 3,
                    cx - half / 3,
                    cy + half / 3,
                    color.0,
                    color.1,
                    color.2
                );
                stroke!(
                    cx - half / 3,
                    cy + half / 3,
                    cx - half,
                    cy,
                    color.0,
                    color.1,
                    color.2
                );
                self.star_orb(cx, cy, (half / 4).max(1), 255, false);
            }
            _ => {
                stroke!(cx - half, cy, cx, cy - half / 2, color.0, color.1, color.2);
                stroke!(cx, cy - half / 2, cx + half, cy, color.0, color.1, color.2);
                stroke!(cx + half, cy, cx, cy + half / 2, color.0, color.1, color.2);
                stroke!(cx, cy + half / 2, cx - half, cy, color.0, color.1, color.2);
            }
        }
    }

    // ------------------------=
    // FUNC: polished_button
    // DESC: Draws the OS glass button artwork with shared focus lighting and unchanged action geometry.
    // ------------------=
    pub(super) fn polished_button(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        label: &[u8],
        primary: bool,
        focused: bool,
    ) {
        self.app_button_surface(crate::ui::geometry::Rect{x:left as i32,y:top as i32,
            width:width as u32,height:height as u32},primary,if focused {255}else{0},false,self.ui_scale());
        self.ui_text_centered_strong(left, width,
            top + height.saturating_sub(UI_FONT_CELL_HEIGHT) / 2,
            label, 242, 248, 252, 1);
        if primary {
            self.authentication_icon(
                left + width.saturating_sub(height / 2),
                top + height / 2,
                4,
                height / 3,
                focused,
            );
        }
    }

    #[cfg(all(
        not(feature = "installer"),
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))]
    // ------------------------=
    // FUNC: themed_icon
    // DESC: Renders one semantic role from the active complete icon family.
    // ------------------=
    pub(super) fn themed_icon(
        &mut self,
        center_x: usize,
        center_y: usize,
        role: usize,
        size: usize,
    ) -> bool {
        if role >= crate::ui::icon_theme::ICON_ROLE_COUNT {
            return false;
        }
        let action = role >= 45;
        let bitmap = match (self.active_icon_theme(), action) {
            (1, false) => super::desktop::LUMINOUS_OBSIDIAN_BASE_BMP,
            (1, true) => super::desktop::LUMINOUS_OBSIDIAN_ACTIONS_BMP,
            (2, false) => super::desktop::FROSTED_QUARTZ_BASE_BMP,
            (2, true) => super::desktop::FROSTED_QUARTZ_ACTIONS_BMP,
            (3, false) => super::desktop::AURORA_HARMONY_BASE_BMP,
            (3, true) => super::desktop::AURORA_HARMONY_ACTIONS_BMP,
            (_, false) => super::desktop::CRYSTAL_BLUE_GLASS_BASE_BMP,
            (_, true) => super::desktop::CRYSTAL_BLUE_GLASS_ACTIONS_BMP,
        };
        self.paint_bitmap_alpha_atlas_cell(
            bitmap,
            5,
            if action { 3 } else { 9 },
            if action { role - 45 } else { role },
            center_x.saturating_sub(size / 2),
            center_y.saturating_sub(size / 2),
            size,
        )
    }

}
