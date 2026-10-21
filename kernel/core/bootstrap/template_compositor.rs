//! Runtime compositor for the exact layer model saved by Installer Studio.

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
use super::desktop::ONBOARDING_BMP;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
use super::installer::{
    INSTALLER_MASTHEAD_BMP, INSTALLER_PROGRESS_HERO_BMP, INSTALLER_WELCOME_MASTHEAD_BMP,
};
use super::*;
use super::installer::{INSTALLER_FONT_CELL_HEIGHT, INSTALLER_FONT_NATIVE_SIZE_PX};
use crate::ui::installer_layout::{
    scale_template_rect, CONFIGURATION_TEMPLATE_BYTES, INSTALLER_TEMPLATE_BYTES,
};
use crate::ui::installer_template::{
    template_image_uses_aspect_fill, InstallerTemplate, InstallerTemplateElement,
    InstallerTemplateRole,
};

impl DisplayDevice {
    // ------------------------=
    // FUNC: installer_template_screen
    // DESC: Paints every visible installer layer in the editor's deterministic stacking order.
    // ------------------=
    pub(super) fn installer_template_screen(&mut self, screen: u8) -> bool {
        self.template_screen(INSTALLER_TEMPLATE_BYTES, screen, None, true, false, None, None)
    }

    // ------------------------=
    // FUNC: installer_template_screen_with_details
    // DESC: Substitutes live data at its saved layer position instead of drawing a second fixed-position overlay.
    // ------------------=
    pub(super) fn installer_template_screen_with_details(&mut self, screen: u8, details: &[u8]) -> bool {
        self.template_screen(INSTALLER_TEMPLATE_BYTES, screen, None, true, false, Some(details), None)
    }

    // ------------------------=
    // FUNC: installer_template_progress_screen
    // DESC: Composes saved Installing layers in order while substituting only the live progress value and status.
    // ------------------=
    pub(super) fn installer_template_progress_screen(&mut self, percent: usize, label: &[u8]) -> bool {
        self.template_screen(INSTALLER_TEMPLATE_BYTES, 8, None, true, false, None, Some((percent, label)))
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
            true,
            None,
            None,
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
            false,
            None,
            None,
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
            false,
            None,
            None,
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
        skip_live_input: bool,
        live_details: Option<&[u8]>,
        progress: Option<(usize, &[u8])>,
    ) -> bool {
        let Ok(template) = InstallerTemplate::parse(bytes) else {
            return false;
        };
        let Some(count) = template.element_count(screen) else {
            return false;
        };
        let mut progress_index=0usize;
        for index in 0..count {
            let Some(element) = template.layer_at(screen, index) else {
                return false;
            };
            if element.hidden || element.opacity == 0 {
                continue;
            }
            if full_scene
                && skip_live_input
                && element.role == InstallerTemplateRole::Input as u8
            {
                continue;
            }
            let is_navigation = element.role == InstallerTemplateRole::BackButton as u8
                || element.role == InstallerTemplateRole::PrimaryButton as u8
                || element.role == InstallerTemplateRole::Footer as u8;
            // Configuration live content paints these once with the actual
            // navigation focus; avoid accumulating their translucent edges.
            if full_scene && bytes.as_ptr() == CONFIGURATION_TEMPLATE_BYTES.as_ptr()
                && is_navigation { continue; }
            if full_scene || is_navigation {
                if element.kind == 6 {
                    let (percent, label) = progress.unwrap_or((0, element.text));
                    self.template_progress_control(
                        scale_template_rect(element.frame, self.width, self.height),
                        element.fill, element.border, element.opacity, element.corner_radius,
                        percent, label,
                    );
                    continue;
                }
                if element.role == InstallerTemplateRole::LiveDetails as u8 {
                    if let Some(text) = live_details {
                        self.template_element(InstallerTemplateElement { text, ..element }, None, navigation);
                    }
                    continue;
                }
                let image = if element.kind == 2 {
                    template.asset(element.image_asset)
                } else {
                    None
                };
                if bytes.as_ptr() == CONFIGURATION_TEMPLATE_BYTES.as_ptr()
                    && matches!(element.kind, 1 | 4 | 5)
                {
                    let progress=(progress_index,screen.saturating_sub(1) as usize);
                    if element.role==InstallerTemplateRole::ProgressSegment as u8 {progress_index+=1;}
                    self.access_template_control(element, navigation, progress);
                } else if bytes.as_ptr() == CONFIGURATION_TEMPLATE_BYTES.as_ptr()
                    && element.kind == 3
                {
                    self.configuration_template_text_layer(element);
                } else {
                    self.template_element(element, image, navigation);
                }
            }
        }
        true
    }

    // ------------------------=
    // FUNC: configuration_template_text_layer
    // DESC: Uses the authored Inter typography and fits only single-line-height titles so responsive aspect ratios cannot hide their final words.
    // ------------------=
    fn configuration_template_text_layer(&mut self, element: InstallerTemplateElement<'_>) {
        let rect = scale_template_rect(element.frame, self.width, self.height);
        let strong = element.role == InstallerTemplateRole::Title as u8;
        let mut pixels = (element.font_size as usize * self.height / 1000).max(6);
        let line_height = FontSize::new(INSTALLER_FONT_NATIVE_SIZE_PX, pixels)
            .scale_usize(INSTALLER_FONT_CELL_HEIGHT);
        let line_advance = FontSize::new(INSTALLER_FONT_NATIVE_SIZE_PX, pixels)
            .scale_usize(INSTALLER_FONT_CELL_HEIGHT + 4);
        if strong && !element.text.contains(&b'\n') && rect.height < line_height + line_advance {
            while pixels > 6 && self.access_text_width(element.text, pixels, strong) > rect.width {
                pixels -= 1;
            }
        }
        let previous_clip = self.render_clip;
        self.intersect_render_clip(rect.left, rect.top, rect.width, rect.height);
        self.access_text_wrapped(rect.left, rect.top, rect.width, element.text,
            element.fill[0], element.fill[1], element.fill[2],
            (element.fill[3] as u16 * element.opacity as u16 / 100) as u8, pixels, strong);
        self.render_clip = previous_clip;
    }

    // ------------------------=
    // FUNC: access_template_control
    // DESC: Renders saved configuration controls with the approved Access gradient, focus easing and subpixel edges.
    // ------------------=
    pub(super) fn access_template_control(&mut self, element: InstallerTemplateElement<'_>,
        navigation: Option<(usize, bool, Option<(i32, i32, bool)>)>, progress:(usize,usize)) {
        use super::access_style as palette;
        use crate::ui::panel_recipe::Surface;
        let rect=scale_template_rect(element.frame,self.width,self.height);
        let focused=navigation.is_some_and(|(focus,_,_)|
            (element.role==InstallerTemplateRole::BackButton as u8 && focus==0)
            || (element.role==InstallerTemplateRole::PrimaryButton as u8 && focus==1));
        let fill=(element.fill[0],element.fill[1],element.fill[2]);
        let border=(element.border[0],element.border[1],element.border[2]);
        let opacity=(element.fill[3] as u16*element.opacity as u16/100) as u8;
        let border_opacity=(element.border[3] as u16*element.opacity as u16/100) as u8;
        let text_opacity=(255u16*element.opacity as u16/100) as u8;
        if element.role==InstallerTemplateRole::ProgressSegment as u8 {
            let size=rect.height.min(rect.width);
            let left=rect.left+(rect.width-size)/2;
            let current=progress.0<=progress.1;
            self.access_surface_alpha(left,rect.top,size,size,
                if current {Surface::Current}else{Surface::Inactive},fill,border,
                opacity,border_opacity,false);
            if progress.0<progress.1 {self.access_glyph(left+size/2,rect.top+size/2,15,size*3/4,false);}
            return;
        }
        let kind=if element.kind==5 {
            if element.role==InstallerTemplateRole::PrimaryButton as u8 {Surface::Primary}else{Surface::Secondary}
        } else if element.role==InstallerTemplateRole::Input as u8 {Surface::Field}else{Surface::Panel};
        if element.role==InstallerTemplateRole::Console as u8 {
            self.access_frost_panel(rect.left,rect.top,rect.width,rect.height,fill,border,opacity,border_opacity);
        } else {
            self.access_surface_alpha(rect.left,rect.top,rect.width,rect.height,kind,fill,border,
                opacity,border_opacity,focused);
        }
        if element.kind==5 {
            let font=(element.font_size as usize*self.height/1000).max(6);
            let width=self.access_text_width(element.text,font,true);
            let previous=self.render_clip;
            self.intersect_render_clip(rect.left,rect.top,rect.width,rect.height);
            self.access_text(rect.left+rect.width.saturating_sub(width)/2,
                rect.top+rect.height.saturating_sub(font*7/6)/2,
                element.text,palette::TEXT.0,palette::TEXT.1,palette::TEXT.2,text_opacity,font,true);
            self.render_clip=previous;
        }
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
        self.template_element_in_rect(element, image, navigation, rect);
    }

    // ------------------------=
    // FUNC: template_element_in_rect
    // DESC: Paints an authored layer inside an explicitly mapped runtime rectangle.
    // ------------------=
    pub(super) fn template_element_in_rect(
        &mut self,
        element: InstallerTemplateElement<'_>,
        image: Option<&[u8]>,
        navigation: Option<(usize, bool, Option<(i32, i32, bool)>)>,
        rect: crate::ui::installer_layout::InstallerRect,
    ) {
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
                template_image_uses_aspect_fill(element.role),
            ),
            3 => self.template_text_layer(element, rect),
            5 => self.template_button_layer(element, rect, navigation),
            6 => self.template_progress_control(
                rect,
                element.fill,
                element.border,
                element.opacity,
                element.corner_radius,
                42,
                element.text,
            ),
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
        if element.role == InstallerTemplateRole::ProgressSegment as u8 {
            let bar_height = (4 * self.height / 1000).max(2).min(rect.height);
            self.fill_rounded_rect_alpha(
                rect.left,
                rect.top + rect.height.saturating_sub(bar_height) / 2,
                rect.width,
                bar_height,
                radius.min(bar_height / 2),
                element.fill[0],
                element.fill[1],
                element.fill[2],
                fill_alpha,
            );
            return;
        }
        let focused = navigation.is_some_and(|(focus, _, _)| {
            element.role == InstallerTemplateRole::Input as u8 && focus >= 2
        });
        let is_control = element.role == InstallerTemplateRole::Input as u8
            || element.role == InstallerTemplateRole::DateField as u8
            || element.role == InstallerTemplateRole::TimeField as u8
            || element.role == InstallerTemplateRole::TimeZoneSelector as u8
            || element.role == InstallerTemplateRole::OffsetBadge as u8;
        if element.role==InstallerTemplateRole::Console as u8 {
            self.access_frost_panel(rect.left,rect.top,rect.width,rect.height,
                (element.fill[0],element.fill[1],element.fill[2]),
                (element.border[0],element.border[1],element.border[2]),fill_alpha,border_alpha);
            return;
        }
        self.precision_surface(crate::ui::geometry::Rect{x:rect.left as i32,y:rect.top as i32,
            width:rect.width as u32,height:rect.height as u32},
            if is_control {crate::ui::panel_recipe::Surface::Field}else{crate::ui::panel_recipe::Surface::Panel},
            crate::ui::panel_recipe::Material{fill:(element.fill[0],element.fill[1],element.fill[2]),
                edge:(element.border[0],element.border[1],element.border[2]),opacity:fill_alpha,
                edge_opacity:border_alpha,focus:if focused {255}else{0},pressed:false,scale:self.ui_scale()});
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
    pub(super) fn template_image(
        &mut self,
        name: &[u8],
        packaged: Option<&[u8]>,
        rect: crate::ui::installer_layout::InstallerRect,
        crop: [u8; 4],
        opacity: u8,
        aspect_fill: bool,
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
                include_bytes!("../../../assets/desktop/infinity-onboarding-wallpaper-v1.bmp")
            } else if name.ends_with(b"eclipse-access-wallpaper-v1.png") {
                ONBOARDING_BMP
            } else if name.ends_with(b"infinity-eclipse-header-v1.bmp") {
                super::bootstrap::SPLASH_BMP
            } else if name.ends_with(b"infinity-installer-progress-hero-v1.png") {
                INSTALLER_PROGRESS_HERO_BMP
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
                    aspect_fill,
                );
            }
        }
    }

    // ------------------------=
    // FUNC: template_progress_control
    // DESC: Draws an authored UIKit progress control with a live label and percentage.
    // ------------------=
    pub(super) fn template_progress_control(
        &mut self,
        rect: crate::ui::installer_layout::InstallerRect,
        fill: [u8; 4],
        border: [u8; 4],
        opacity: u8,
        _corner_radius: u8,
        percent: usize,
        label: &[u8],
    ) {
        let previous_clip = self.render_clip;
        self.intersect_render_clip(rect.left, rect.top, rect.width, rect.height);
        let alpha = |value: u8| (value as u16 * opacity as u16 / 100) as u8;
        let inset = (18 * self.height / 1000).max(1).min(rect.width / 4);
        let pixels = (18 * self.height / 1000).max(1);
        let gap = 14 * self.height / 1000;
        let track_height = (16 * self.height / 1000).max(1);
        let line_height = pixels * 28 / 24;
        let group_height = line_height + gap + track_height;
        let top = rect.top + rect.height.saturating_sub(group_height) / 2;
        let track_top = top + line_height + gap;
        let track_width = rect.width.saturating_sub(inset * 2);
        use crate::ui::{geometry::Rect, panel_recipe::{Material,Surface}};
        let material=Material{fill:super::access_style::SURFACE,edge:super::access_style::BORDER,
            opacity:alpha(240),edge_opacity:alpha(border[3]),focus:0,pressed:false,scale:self.ui_scale()};
        self.precision_surface(Rect{x:rect.left as i32,y:rect.top as i32,
            width:rect.width as u32,height:rect.height as u32},Surface::Panel,material);
        let mut percent_text = [b'0'; 4];
        let value = percent.min(100);
        let digits = if value == 100 {
            percent_text[..3].copy_from_slice(b"100");
            3
        } else if value >= 10 {
            percent_text[0] += (value / 10) as u8;
            percent_text[1] += (value % 10) as u8;
            2
        } else {
            percent_text[0] += value as u8;
            1
        };
        percent_text[digits] = b'%';
        let percent_slice = &percent_text[..digits + 1];
        let percent_width = self.template_text_width(percent_slice, pixels, true);
        self.template_text(rect.left + rect.width.saturating_sub(inset + percent_width), top,
            percent_slice, fill[0], fill[1], fill[2], alpha(fill[3]), pixels, true);
        self.intersect_render_clip(rect.left + inset, top,
            track_width.saturating_sub(percent_width + inset), line_height);
        self.template_text(rect.left + inset, top, label, 241, 245, 250, alpha(255), pixels, true);
        self.render_clip = previous_clip;
        self.intersect_render_clip(rect.left, rect.top, rect.width, rect.height);
        let track=Rect{x:(rect.left+inset) as i32,y:track_top as i32,
            width:track_width as u32,height:track_height as u32};
        self.precision_surface(track,Surface::Field,Material{fill:super::access_style::BACKGROUND,opacity:alpha(199),..material});
        self.precision_surface(Rect{width:(track_width*value/100) as u32,..track},Surface::Primary,
            Material{fill:super::access_style::PRIMARY,opacity:alpha(fill[3]),..material});
        self.render_clip = previous_clip;
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
        let previous_clip = self.render_clip;
        self.intersect_render_clip(rect.left, rect.top, rect.width, rect.height);
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
        self.render_clip = previous_clip;
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
        let primary = element.role == InstallerTemplateRole::PrimaryButton as u8;
        use crate::ui::{geometry::Rect,panel_recipe::{Material,Surface}};
        self.precision_surface(Rect{x:rect.left as i32,y:rect.top as i32,
            width:rect.width as u32,height:rect.height as u32},
            if primary {Surface::Primary}else{Surface::Secondary},Material{
                fill:(element.fill[0],element.fill[1],element.fill[2]),
                edge:(element.border[0],element.border[1],element.border[2]),
                opacity:(element.fill[3] as u16*opacity/100) as u8,
                edge_opacity:(element.border[3] as u16*opacity/100) as u8,
                focus:if focused||hovered {super::access_style::focus_amount()}else{0},
                pressed:depressed,scale:self.ui_scale()});
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
