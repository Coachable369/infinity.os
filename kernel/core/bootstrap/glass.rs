//! Shared native glass recipe, used directly by the desktop and pixel regression harness.
use super::*;
impl DisplayDevice {
    // ------------------------=
    // FUNC: glass_panel
    // DESC: Builds a layered translucent panel with restrained shadow, highlight, and cyan edge treatment.
    // ------------------=
    pub(in super::super) fn glass_panel(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        strong: bool,
    ) {
        let (panel_r, panel_g, panel_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Widget);
        let (outline_r, outline_g, outline_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
        self.glass_panel_with_palette(
            left,
            top,
            width,
            height,
            strong,
            (panel_r, panel_g, panel_b),
            (outline_r, outline_g, outline_b),
        );
    }

    // ------------------------=
    // FUNC: glass_panel_with_palette
    // DESC: Draws the shared layered glass recipe using one caller-selected surface and edge palette.
    // ------------------=
    pub(super) fn glass_panel_with_palette(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        strong: bool,
        panel: (u8, u8, u8),
        outline: (u8, u8, u8),
    ) {
        let radius = (width.min(height) / 12).clamp(8, 18);
        let (opacity, blur) = self.active_background_effects();
        if blur >= 2 && opacity < 100 && !self.fast_motion_frame {
            self.blur_framebuffer_region(left, top, width, height, blur as usize);
        }
        if self.skin_visual_mode() == 1 {
            self.fill_rounded_rect_alpha(
                left,
                top,
                width,
                height,
                radius,
                248,
                251,
                255,
                ((if strong { 244u16 } else { 226u16 }) * u16::from(opacity) / 100) as u8,
            );
            self.outline_rounded_rect(left, top, width, height, radius, 122, 145, 166);
            return;
        }
        if self.skin_visual_mode() == 2 {
            self.fill_rounded_rect_alpha(
                left,
                top,
                width,
                height,
                radius,
                8,
                8,
                8,
                (255u16 * u16::from(opacity) / 100) as u8,
            );
            self.outline_rounded_rect(left, top, width, height, radius, 255, 255, 255);
            return;
        }
        for inset in (1..=5usize).rev() {
            let (shadow_red, shadow_green, shadow_blue, shadow_alpha) = if strong {
                (5, 29, 42, 12)
            } else {
                (0, 4, 10, 18)
            };
            self.fill_rounded_rect_alpha(
                left.saturating_add(inset * 2),
                top.saturating_add(inset * 2),
                width,
                height,
                radius,
                shadow_red,
                shadow_green,
                shadow_blue,
                shadow_alpha,
            );
        }
        self.fill_rounded_rect_alpha(
            left,
            top,
            width,
            height,
            radius,
            panel.0,
            panel.1,
            panel.2,
            ((if strong { 232u16 } else { 204u16 }) * u16::from(opacity) / 100) as u8,
        );
        self.outline_rounded_rect(
            left, top, width, height, radius, outline.0, outline.1, outline.2,
        );
        if width > 4 && height > 4 {
            let divisor = if strong { 3 } else { 5 };
            let inner_edge = (
                outline.0 / divisor,
                outline.1 / divisor,
                outline.2 / divisor,
            );
            self.outline_rounded_rect(
                left + 2,
                top + 2,
                width - 4,
                height - 4,
                radius.saturating_sub(2),
                inner_edge.0,
                inner_edge.1,
                inner_edge.2,
            );
        }
    }
}
