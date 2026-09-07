//! Runtime compositor for the exact layer model saved by Installer Studio.

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
use super::desktop::ONBOARDING_BMP;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
use super::installer::{INSTALLER_MASTHEAD_BMP, INSTALLER_WELCOME_MASTHEAD_BMP};
use super::*;
use crate::ui::installer_layout::{
    scale_template_rect, CONFIGURATION_TEMPLATE_BYTES, INSTALLER_TEMPLATE_BYTES,
};
use crate::ui::installer_template::{
    InstallerTemplate, InstallerTemplateElement, InstallerTemplateRole,
};

impl DisplayDevice {
    // ------------------------=
    // FUNC: installer_template_screen
    // DESC: Paints every visible installer layer in the editor's deterministic stacking order.
    // ------------------=
    pub(super) fn installer_template_screen(&mut self, screen: u8) -> bool {
        self.template_screen(INSTALLER_TEMPLATE_BYTES, screen, None, true)
    }

    // ------------------------=
    // FUNC: configuration_template_screen
    // DESC: Paints every visible first-boot configuration layer exactly from its saved template.
    // ------------------=
    pub(super) fn configuration_template_screen(&mut self, step: usize) -> bool {
        self.template_screen(
            CONFIGURATION_TEMPLATE_BYTES,
            step.saturating_add(1) as u8,
            None,
            true,
        )
    }

    // ------------------------=
    // FUNC: installer_template_navigation
    // DESC: Repaints authored navigation layers while applying current focus and availability state.
    // ------------------=
    pub(super) fn installer_template_navigation(
        &mut self,
        screen: u8,
        focus: usize,
        has_primary: bool,
        cursor_x: i32,
        cursor_y: i32,
        pressed: bool,
    ) -> bool {
        self.template_screen(
            INSTALLER_TEMPLATE_BYTES,
            screen,
            Some((focus, has_primary, Some((cursor_x, cursor_y, pressed)))),
            false,
        )
    }

    // ------------------------=
    // FUNC: configuration_template_navigation
    // DESC: Repaints authored first-boot actions with their current keyboard or pointer focus.
    // ------------------=
    pub(super) fn configuration_template_navigation(&mut self, step: usize, focus: usize) -> bool {
        self.template_screen(
            CONFIGURATION_TEMPLATE_BYTES,
            step.saturating_add(1) as u8,
            Some((focus, true, None)),
            false,
        )
    }

    // ------------------------=
    // FUNC: template_screen
    // DESC: Resolves and paints either a complete template scene or only its interactive navigation layers.
    // ------------------=
    fn template_screen(
        &mut self,
        bytes: &'static [u8],
        screen: u8,
        navigation: Option<(usize, bool, Option<(i32, i32, bool)>)>,
        full_scene: bool,
    ) -> bool {
        let Ok(template) = InstallerTemplate::parse(bytes) else {
            return false;
        };
        let Some(count) = template.element_count(screen) else {
            return false;
        };
        for index in 0..count {
            let Some(element) = template.layer_at(screen, index) else {
                return false;
            };
            if element.hidden {
                continue;
            }
            let is_navigation = element.role == InstallerTemplateRole::BackButton as u8
                || element.role == InstallerTemplateRole::PrimaryButton as u8
                || element.role == InstallerTemplateRole::Footer as u8;
            if full_scene || is_navigation {
                let image = if element.kind == 2 {
                    template.asset(element.image_asset)
                } else {
                    None
                };
                self.template_element(element, image, navigation);
            }
        }
        true
    }

    // ------------------------=
    // FUNC: template_element
    // DESC: Converts one saved layer into framebuffer pixels using its authored geometry and appearance.
    // ------------------=
    fn template_element(
        &mut self,
        element: InstallerTemplateElement<'_>,
        image: Option<&[u8]>,
        navigation: Option<(usize, bool, Option<(i32, i32, bool)>)>,
    ) {
        if element.role == InstallerTemplateRole::PrimaryButton as u8
            && navigation.is_some_and(|(_, available, _)| !available)
        {
            return;
        }
        let rect = scale_template_rect(element.frame, self.width, self.height);
        let opacity = element.opacity as u16;
        let fill_alpha = (element.fill[3] as u16 * opacity / 100) as u8;
        let border_alpha = (element.border[3] as u16 * opacity / 100) as u8;
        let radius = (element.corner_radius as usize * self.height / 1000).max(1);
        match element.kind {
            1 | 4 => self.template_surface_layer(
                element,
                rect,
                radius,
                fill_alpha,
                border_alpha,
                navigation,
            ),
            2 => self.template_image(
                element.image_asset,
                image,
                rect,
                element.crop,
                (255u16 * element.opacity as u16 / 100) as u8,
            ),
            3 => self.template_text_layer(element, rect),
            5 => self.template_button_layer(element, rect, navigation),
            _ => {}
        }
    }

    // ------------------------=
    // FUNC: template_surface_layer
    // DESC: Paints UIKit glass depth for panels, consoles, fields, selectors, and badges from authored colors.
    // ------------------=
    fn template_surface_layer(
        &mut self,
        element: InstallerTemplateElement<'_>,
        rect: crate::ui::installer_layout::InstallerRect,
        radius: usize,
        fill_alpha: u8,
        border_alpha: u8,
        navigation: Option<(usize, bool, Option<(i32, i32, bool)>)>,
    ) {
        let focused = navigation.is_some_and(|(focus, _, _)| {
            element.role == InstallerTemplateRole::Input as u8 && focus >= 2
        });
        let is_control = element.role == InstallerTemplateRole::Input as u8
            || element.role == InstallerTemplateRole::DateField as u8
            || element.role == InstallerTemplateRole::TimeField as u8
            || element.role == InstallerTemplateRole::TimeZoneSelector as u8
            || element.role == InstallerTemplateRole::OffsetBadge as u8;
        self.fill_rounded_rect_alpha(
            rect.left,
            rect.top,
            rect.width,
            rect.height,
            radius,
            element.fill[0],
            element.fill[1],
            element.fill[2],
            fill_alpha,
        );
        let lift = if is_control { (7, 21, 28) } else { (6, 14, 21) };
        self.fill_rounded_rect_alpha(
            rect.left.saturating_add(2),
            rect.top.saturating_add(2),
            rect.width.saturating_sub(4),
            rect.height / 2,
            radius.saturating_sub(2),
            element.fill[0].saturating_add(lift.0),
            element.fill[1].saturating_add(lift.1),
            element.fill[2].saturating_add(lift.2),
            if focused { 132 } else { 84 },
        );
        let border = if focused {
            [156, 232, 255]
        } else {
            [element.border[0], element.border[1], element.border[2]]
        };
        self.outline_rounded_rect_alpha(
            rect.left,
            rect.top,
            rect.width,
            rect.height,
            radius,
            border[0],
            border[1],
            border[2],
            border_alpha,
        );
        if is_control && !element.text.is_empty() {
            let font_px = (element.font_size as usize * self.height / 1000).max(6);
            let text_height = (font_px * 7 / 6).max(1);
            self.template_text(
                rect.left + 16 * self.ui_scale().max(1),
                rect.top + rect.height.saturating_sub(text_height) / 2,
                element.text,
                if element.role == InstallerTemplateRole::Input as u8 {
                    119
                } else {
                    241
                },
                if element.role == InstallerTemplateRole::Input as u8 {
                    133
                } else {
                    245
                },
                if element.role == InstallerTemplateRole::Input as u8 {
                    149
                } else {
                    250
                },
                255,
                font_px,
                false,
            );
        }
    }

    // ------------------------=
    // FUNC: template_image
    // DESC: Resolves packaged editor image assets and paints them into their authored frame.
    // ------------------=
    fn template_image(
        &mut self,
        name: &[u8],
        packaged: Option<&[u8]>,
        rect: crate::ui::installer_layout::InstallerRect,
        crop: [u8; 4],
        opacity: u8,
    ) {
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        {
            let bitmap = if let Some(bitmap) = packaged {
                bitmap
            } else if name.ends_with(b"infinity-installer-masthead-v2.png") {
                INSTALLER_MASTHEAD_BMP
            } else if name.ends_with(b"infinity-installer-masthead-v1.png") {
                INSTALLER_WELCOME_MASTHEAD_BMP
            } else if name.ends_with(b"infinity-onboarding-wallpaper-v1.png") {
                ONBOARDING_BMP
            } else {
                &[]
            };
            if !bitmap.is_empty() {
                self.paint_bitmap_template_rect(
                    bitmap,
                    rect.left,
                    rect.top,
                    rect.width,
                    rect.height,
                    crop,
                    opacity,
                );
            }
        }
    }

    // ------------------------=
    // FUNC: template_text_layer
    // DESC: Paints authored text, color, size, opacity, wrapping, and clipping within its saved frame.
    // ------------------=
    fn template_text_layer(
        &mut self,
        element: InstallerTemplateElement<'_>,
        rect: crate::ui::installer_layout::InstallerRect,
    ) {
        let alpha = (element.fill[3] as u16 * element.opacity as u16 / 100) as u8;
        self.set_render_clip(rect.left, rect.top, rect.width, rect.height);
        self.template_text_wrapped(
            rect.left,
            rect.top,
            rect.width,
            element.text,
            element.fill[0],
            element.fill[1],
            element.fill[2],
            alpha,
            element.font_size as usize * self.height / 1000,
            element.role == InstallerTemplateRole::Title as u8,
        );
        self.clear_render_clip();
    }

    // ------------------------=
    // FUNC: template_button_layer
    // DESC: Paints an authored button and overlays its live focus state without changing saved styling.
    // ------------------=
    fn template_button_layer(
        &mut self,
        element: InstallerTemplateElement<'_>,
        rect: crate::ui::installer_layout::InstallerRect,
        navigation: Option<(usize, bool, Option<(i32, i32, bool)>)>,
    ) {
        let focused = navigation.is_some_and(|(focus, _, _)| {
            (element.role == InstallerTemplateRole::BackButton as u8 && focus == 0)
                || (element.role == InstallerTemplateRole::PrimaryButton as u8 && focus == 1)
        });
        let pointer = navigation.and_then(|(_, _, pointer)| pointer);
        let hovered = pointer.is_some_and(|(x, y, _)| {
            x >= element.frame.x as i32
                && x <= (element.frame.x + element.frame.width) as i32
                && y >= element.frame.y as i32
                && y <= (element.frame.y + element.frame.height) as i32
        });
        let depressed = pointer.is_some_and(|(_, _, is_pressed)| is_pressed) && hovered;
        let opacity = element.opacity as u16;
        let radius = (element.corner_radius as usize * self.height / 1000).max(1);
        let primary = element.role == InstallerTemplateRole::PrimaryButton as u8;
        let lift = if primary {
            if focused || hovered {
                (30, 82, 104)
            } else {
                (17, 57, 75)
            }
        } else if focused || hovered {
            (24, 55, 75)
        } else {
            (12, 25, 36)
        };
        let darken = if depressed { (3, 8, 11) } else { (0, 0, 0) };
        let base = [
            element.fill[0].saturating_sub(darken.0),
            element.fill[1].saturating_sub(darken.1),
            element.fill[2].saturating_sub(darken.2),
        ];
        self.fill_rounded_rect_alpha(
            rect.left,
            rect.top,
            rect.width,
            rect.height,
            radius,
            base[0],
            base[1],
            base[2],
            (element.fill[3] as u16 * opacity / 100) as u8,
        );
        self.fill_rounded_rect_alpha(
            rect.left.saturating_add(2),
            rect.top.saturating_add(2),
            rect.width.saturating_sub(4),
            rect.height / 2,
            radius.saturating_sub(2),
            base[0].saturating_add(lift.0),
            base[1].saturating_add(lift.1),
            base[2].saturating_add(lift.2),
            if focused {
                148
            } else if hovered {
                118
            } else {
                82
            },
        );
        let border = if focused {
            [156, 232, 255]
        } else if hovered {
            [32, 191, 255]
        } else {
            [element.border[0], element.border[1], element.border[2]]
        };
        self.outline_rounded_rect_alpha(
            rect.left,
            rect.top,
            rect.width,
            rect.height,
            radius,
            border[0],
            border[1],
            border[2],
            (element.border[3] as u16 * opacity / 100) as u8,
        );
        let font_px = (element.font_size as usize * self.height / 1000).max(6);
        let text_width = self.template_text_width(element.text, font_px, true);
        let text_height = (font_px * 7 / 6).max(1);
        self.template_text(
            rect.left + rect.width.saturating_sub(text_width) / 2,
            rect.top
                + rect.height.saturating_sub(text_height) / 2
                + if depressed {
                    2 * self.ui_scale().max(1)
                } else {
                    0
                },
            element.text,
            255,
            255,
            255,
            (255u16 * opacity / 100) as u8,
            font_px,
            true,
        );
    }
}
