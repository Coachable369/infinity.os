//! Production first-boot text input rendering shared by native scene and pixel proof.
use super::{DisplayDevice, FontSize};
use super::access_style as access;
use super::installer::{INSTALLER_FONT_NATIVE_SIZE_PX, INSTALLER_FONT_CELL_HEIGHT};
use crate::ui::panel_recipe::Surface as AccessSurface;

impl DisplayDevice {
    // ------------------------=
    // FUNC: onboarding_input_field
    // DESC: Draws one polished first-boot text field with placeholder, focus, and secure masking.
    // ------------------=
    pub(super) fn onboarding_input_field(
        &mut self,
        step: usize,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        input: &[u8],
        masked: bool,
        focused: bool,
        placeholder: &[u8],
    ) {
        // Focus changes can repaint this field independently of the glass card.
        // Use an opaque base so repeated bounded updates remain color-stable.
        self.access_surface(left, top, width, height, AccessSurface::Field, access::BACKGROUND,
            access::BORDER, 255, focused);
        let mut shown = [0u8; 64];
        let shown_len = input.len().min(shown.len());
        if masked {
            shown[..shown_len].fill(b'*');
        } else {
            shown[..shown_len].copy_from_slice(&input[..shown_len]);
        }
        let display = if shown_len == 0 {
            placeholder
        } else {
            &shown[..shown_len]
        };
        let color = if shown_len == 0 {
            access::SECONDARY
        } else {
            access::TEXT
        };
        let pixels = crate::ui::installer_template::InstallerTemplate::parse(
            crate::ui::installer_layout::CONFIGURATION_TEMPLATE_BYTES).ok()
            .and_then(|template| template.element(step.saturating_add(1) as u8,
                crate::ui::installer_template::InstallerTemplateRole::Input))
            .map(|element| (element.font_size as usize * self.height / 1000).max(6))
            .unwrap_or((18 * self.height / 1000).max(6));
        let text_height = FontSize::new(INSTALLER_FONT_NATIVE_SIZE_PX, pixels)
            .scale_usize(INSTALLER_FONT_CELL_HEIGHT).max(1);
        let inset = (crate::ui::system_layout::UI_GUTTER * self.height / 1000).max(4);
        let text_left = left.saturating_add(inset);
        let text_top = top + height.saturating_sub(text_height) / 2;
        let previous_clip = self.render_clip;
        self.intersect_render_clip(left + inset, top + 2,
            width.saturating_sub(inset * 2), height.saturating_sub(4));
        self.access_text(text_left, text_top, display, color.0, color.1, color.2,
            255, pixels, false);
        if focused {
            if let Some((true, index)) = crate::ui::text_input::caret(1) {
                let caret_x = text_left + self.access_text_width(&shown[..index.min(shown_len)], pixels, false);
                self.fill_rect(caret_x, text_top, self.ui_scale().max(1), text_height,
                    access::FOCUS.0, access::FOCUS.1, access::FOCUS.2);
            }
        }
        self.render_clip = previous_clip;
    }

}
