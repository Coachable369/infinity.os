//! Installer artwork, typography, storage screens, cards, controls, and composition.

use super::*;

pub(super) use super::installer_text_view::*;
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const INSTALLER_BMP: &[u8] =
    include_bytes!("../../../assets/boot/infinity-installer-background-v2.bmp");
#[cfg(not(feature = "installer"))]
pub(super) const INSTALLER_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const INSTALLER_MASTHEAD_BMP: &[u8] =
    include_bytes!("../../../assets/boot/infinity-installer-masthead-v2.bmp");
#[cfg(not(feature = "installer"))]
pub(super) const INSTALLER_MASTHEAD_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const INSTALLER_WELCOME_MASTHEAD_BMP: &[u8] =
    include_bytes!("../../../assets/boot/infinity-installer-masthead-v1.bmp");
#[cfg(not(feature = "installer"))]
pub(super) const INSTALLER_WELCOME_MASTHEAD_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const INSTALLER_MESH_HERO_BMP: &[u8] =
    include_bytes!("../../../assets/boot/infinity-installer-mesh-hero-v1.bmp");
#[cfg(not(feature = "installer"))]
pub(super) const INSTALLER_MESH_HERO_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const INSTALLER_MESH_OVERVIEW_BMP: &[u8] =
    include_bytes!("../../../assets/boot/infinity-installer-mesh-overview-v1.bmp");
#[cfg(not(feature = "installer"))]
pub(super) const INSTALLER_MESH_OVERVIEW_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const INSTALLER_MESH_DIAGRAM_BMP: &[u8] =
    include_bytes!("../../../assets/boot/infinity-installer-mesh-diagram-v1.bmp");
#[cfg(not(feature = "installer"))]
pub(super) const INSTALLER_MESH_DIAGRAM_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const INSTALLER_ACTIVATION_BMP: &[u8] =
    include_bytes!("../../../assets/boot/infinity-installer-activation-v2.bmp");
#[cfg(not(feature = "installer"))]
pub(super) const INSTALLER_ACTIVATION_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const INSTALLER_PROGRESS_HERO_BMP: &[u8] =
    include_bytes!("../../../assets/boot/infinity-installer-progress-hero-v1.bmp");
#[cfg(not(feature = "installer"))]
pub(super) const INSTALLER_PROGRESS_HERO_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const STORAGE_HIERARCHY_BMP: &[u8] =
    include_bytes!("../../../assets/boot/infinity-storage-hierarchy-v3.bmp");
#[cfg(not(feature = "installer"))]
pub(super) const STORAGE_HIERARCHY_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const DISK_DISCOVERY_VISION_BMP: &[u8] =
    include_bytes!("../../../assets/boot/infinity-disk-discovery-vision-v1.bmp");
#[cfg(not(feature = "installer"))]
pub(super) const DISK_DISCOVERY_VISION_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const STORAGE_DEVICE_BMP: &[u8] =
    include_bytes!("../../../assets/boot/infinity-storage-device-v1.bmp");
#[cfg(not(feature = "installer"))]
pub(super) const STORAGE_DEVICE_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const DATE_TIME_WORLD_BMP: &[u8] =
    include_bytes!("../../../assets/boot/infinity-time-zone-map-v1.bmp");
#[cfg(not(feature = "installer"))]
pub(super) const DATE_TIME_WORLD_BMP: &[u8] = &[];

impl super::DisplayDevice {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_installer_background
    // DESC: Scales and paints the installer artwork across the complete display.
    // ------------------=
    pub(super) fn paint_installer_background(&mut self) {
        self.paint_bitmap_cover_rect(INSTALLER_BMP, 0, 0, self.width, self.height);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_installer_masthead
    // DESC: Centers the generated InfinityOS brand masthead above every setup step.
    // ------------------=
    pub(super) fn paint_installer_masthead(&mut self) {
        let masthead =
            crate::ui::installer_layout::installer_wizard_layout(1, self.width, self.height)
                .masthead;
        self.paint_bitmap_fit_rect(
            INSTALLER_MASTHEAD_BMP,
            masthead.left,
            masthead.top,
            masthead.width,
            masthead.height,
        );
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_installer_welcome_masthead
    // DESC: Draws the taller Welcome to InfinityOS brand lockup used only by setup step one.
    // ------------------=
    pub(super) fn paint_installer_welcome_masthead(&mut self) {
        let left = self.width * 29 / 100;
        let top = self.height * 2 / 100;
        let width = self.width * 42 / 100;
        let height = self.height * 30 / 100;
        self.paint_bitmap_fit_rect(INSTALLER_WELCOME_MASTHEAD_BMP, left, top, width, height);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_installer_background_rect
    // DESC: Restores a clipped display rectangle from the installer artwork.
    // ------------------=
    pub(super) fn paint_installer_background_rect(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        self.paint_bitmap_cover_rect(INSTALLER_BMP, left, top, width, height);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: restore_installer_panel
    // DESC: Restores a clean installer scene before drawing the active wizard step.
    // ------------------=
    pub(super) fn restore_installer_panel(&mut self, screen: u8) {
        // A step transition is infrequent; restore the complete scene so the
        // invariant frame and masthead never inherit pixels from another step.
        self.paint_installer_background();
        if crate::ui::installer_template::InstallerTemplate::parse(
                crate::ui::installer_layout::INSTALLER_TEMPLATE_BYTES,
            ).is_ok_and(|template| template.element_count(screen).is_some())
        {
            return;
        }
        self.paint_installer_masthead();
    }


    // ------------------------=
    // FUNC: installer_navigation
    // DESC: Draws installer Back and Continue controls with pointer and focus states.
    // ------------------=
    pub(super) fn installer_navigation(
        &mut self,
        screen: u8,
        focus: usize,
        has_primary: bool,
        cursor_x: i32,
        cursor_y: i32,
        pressed: bool,
        redraw_foundation: bool,
    ) {
        if screen != 7
            && self.installer_template_navigation(
                screen,
                focus,
                has_primary,
                cursor_x,
                cursor_y,
                pressed,
            )
        {
            return;
        }
        let modal_or_progress = screen == 7 || screen == 8;
        let layout =
            crate::ui::installer_layout::installer_wizard_layout(screen, self.width, self.height);
        // Rebuild the shared navigation rail when a step changes so labels do
        // not accumulate over the translucent photographic background.
        if redraw_foundation {
            // One continuous footer rail matches the reference and prevents
            // the three formerly separate foundations from looking clipped.
            self.fill_rect_alpha(
                layout.navigation_rail.left,
                layout.navigation_rail.top,
                layout.navigation_rail.width,
                layout.navigation_rail.height,
                7,
                12,
                19,
                232,
            );
            self.fill_rect(
                layout.footer_rail.left,
                layout.navigation_rail.top,
                layout.footer_rail.width,
                self.ui_scale(),
                34,
                92,
                128,
            );
        }
        let (primary, subtitle): (&[u8], &[u8]) = match screen {
            1 => (b"BEGIN SETUP", b"See how InfinityPool works"),
            2 => (b"FIND DISKS", b"Look for a usable disk"),
            3 => (b"USE THIS DISK", b"Open its summary"),
            4 => (b"CONTINUE", b"Set date and time"),
            5 => (b"REVIEW PLAN", b"Save date and time"),
            6 => (b"CONTINUE", b"Go to final confirmation"),
            9 => (b"REBOOT", b"Start the installed system"),
            _ => (b"RETURN", b"Return to the startup screen"),
        };
        let has_back = screen != 9;
        if has_back && !modal_or_progress {
            let (back, back_subtitle): (&[u8], &[u8]) = if screen == 1 {
                (b"NOT NOW", b"Return to startup")
            } else {
                (b"BACK", b"Previous screen")
            };
            self.installer_button_rect(
                layout.back_button,
                back,
                back_subtitle,
                false,
                focus == 0,
                cursor_x,
                cursor_y,
                pressed,
            );
        }
        if has_primary && !modal_or_progress {
            self.installer_button_rect(
                layout.primary_button,
                primary,
                subtitle,
                true,
                focus == 1,
                cursor_x,
                cursor_y,
                pressed,
            );
        }
        if redraw_foundation {
            let hint: &[u8] = if screen == 7 {
                b"TAB / ARROWS: CHOOSE    ENTER: CONFIRM    ESC: CANCEL"
            } else if screen == 8 {
                crate::ui::installer_layout::installer_template_text(
                    8,
                    crate::ui::installer_template::InstallerTemplateRole::Footer,
                )
                .unwrap_or(b"INSTALLATION IN PROGRESS    PLEASE KEEP THIS DEVICE POWERED")
            } else if screen == 5 {
                b"LEFT / RIGHT: FIELD    UP / DOWN: CHANGE    TAB: MOVE    ENTER: SELECT"
            } else {
                b"TAB / ARROWS: MOVE FOCUS    ENTER: SELECT    ESC: CANCEL    F1: HELP"
            };
            let hint_width = self.installer_text_width(hint, false);
            self.installer_text(
                self.width.saturating_sub(hint_width) / 2,
                layout.footer_rail.top + self.height * 35 / 1000,
                hint,
                156,
                174,
                202,
            );
        }
    }

    // ------------------------=
    // FUNC: installer_activation_art
    // DESC: Draws the reference progress hero or compact completion activation artwork.
    // ------------------=
    pub(super) fn installer_activation_art(&mut self, compact: bool) {
        #[cfg(target_arch = "x86")]
        let _ = compact;

        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        {
            if compact {
                let left = self.width * 52 / 100;
                let top = self.height * 40 / 100;
                let width = self.width * 35 / 100;
                let height = (width * 9 / 16).min(self.height * 34 / 100);
                self.paint_bitmap_fit_rect(INSTALLER_ACTIVATION_BMP, left, top, width, height);
                self.outline_rect(left, top, width, height, 48, 118, 164);
            } else {
                if let Some(element) = crate::ui::installer_layout::installer_template_element(
                    8,
                    crate::ui::installer_template::InstallerTemplateRole::ProgressHero,
                ) {
                    let rect = crate::ui::installer_layout::scale_template_rect(
                        element.frame,
                        self.width,
                        self.height,
                    );
                    let template = crate::ui::installer_template::InstallerTemplate::parse(
                        crate::ui::installer_layout::INSTALLER_TEMPLATE_BYTES,
                    )
                    .ok();
                    self.template_image(
                        element.image_asset,
                        template.and_then(|value| value.asset(element.image_asset)),
                        rect,
                        element.crop,
                        (255u16 * element.opacity as u16 / 100) as u8,
                        false,
                    );
                } else {
                    let left = self.width * 17 / 100;
                    let top = self.height * 38 / 100;
                    let width = self.width * 66 / 100;
                    let height = self.height * 26 / 100;
                    self.paint_bitmap_fit_rect_inset(
                        INSTALLER_PROGRESS_HERO_BMP,
                        left,
                        top,
                        width,
                        height,
                        3,
                    );
                }
            }
        }
    }

    // ------------------------=
    // FUNC: installer_progress_frame
    // DESC: Draws one verified installation-progress frame over the activation artwork.
    // ------------------=
    pub(super) fn installer_progress_frame(&mut self, percent: usize, label: &[u8], phase: usize) {
        if let Ok(template) = crate::ui::installer_template::InstallerTemplate::parse(
            crate::ui::installer_layout::INSTALLER_TEMPLATE_BYTES,
        ) {
            if let Some(element) = template.element(8, crate::ui::installer_template::InstallerTemplateRole::ProgressBar) {
                if !element.hidden {
                    let rect = crate::ui::installer_layout::scale_template_rect(element.frame, self.width, self.height);
                    let previous_clip = self.render_clip;
                    self.intersect_render_clip(rect.left, rect.top, rect.width, rect.height);
                    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
                    self.paint_installer_background();
                    self.installer_template_progress_screen(percent, label);
                    self.render_clip = previous_clip;
                }
            }
            return;
        }
        let scale = self.ui_scale();
        let element = crate::ui::installer_layout::installer_template_element(
            8,
            crate::ui::installer_template::InstallerTemplateRole::ProgressBar,
        );
        let rect = element
            .map(|value| {
                crate::ui::installer_layout::scale_template_rect(
                    value.frame,
                    self.width,
                    self.height,
                )
            })
            .unwrap_or(crate::ui::installer_layout::InstallerRect {
                left: self.width * 11 / 100,
                top: self.height * 67 / 100,
                width: self.width * 78 / 100,
                height: self.height * 19 / 100,
            });
        self.template_progress_control(
            rect,
            element.map(|value| value.fill).unwrap_or([174, 219, 247, 255]),
            element.map(|value| value.border).unwrap_or([53, 165, 220, 255]),
            element.map(|value| value.opacity).unwrap_or(100),
            element.map(|value| value.corner_radius).unwrap_or(15),
            element.map(|value| value.surface_material).unwrap_or(0),
            element.map(|value| value.corner_style).unwrap_or(0),
            element.map(|value| value.font_size).unwrap_or(18),
            percent,
            label,
        );

        let (path_x, path_y) = infinity_point(phase);
        let orb_x = self.width as i32 / 2 + path_x * (self.width as i32 / 650).max(1);
        let orb_y = self.height as i32 * 50 / 100 + path_y * (self.height as i32 / 900).max(1);
        self.star_orb(orb_x, orb_y, 4 * scale as i32, 244, true);
    }

    // ------------------------=
    // FUNC: installer_countdown_frame
    // DESC: Draws a cinematic, bounded reboot countdown frame after installation succeeds.
    // ------------------=
    pub(super) fn installer_countdown_frame(
        &mut self,
        remaining: usize,
        frame: usize,
        phase: usize,
    ) {
        let scale = self.ui_scale();
        let left = self.width * 31 / 100;
        let top = self.height * 48 / 100;
        let width = self.width * 38 / 100;
        let height = self.height * 27 / 100;
        // Each animated frame samples the original wallpaper, never the
        // preceding translucent countdown, its number, or its moving particles.
        let previous_clip = self.render_clip;
        self.intersect_render_clip(left, top, width, height);
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        self.paint_installer_background();
        self.access_frost_panel(left, top, width, height,
            super::access_style::SURFACE, super::access_style::BORDER, 240, 255);
        self.render_clip = previous_clip;

        let title = if remaining == 0 {
            b"RESTARTING NOW".as_slice()
        } else {
            b"BOOTING INSTALLED INFINITYOS".as_slice()
        };
        let title_width = self.installer_text_width(title, false);
        self.text(
            left + width.saturating_sub(title_width) / 2,
            top + 22 * scale,
            title,
            228,
            241,
            251,
        );

        let number = [b'0' + remaining.min(9) as u8];
        self.text_scaled(
            left + width / 2 - 24 * scale,
            top + 66 * scale,
            &number,
            246,
            250,
            255,
            6 * scale,
            true,
        );

        for index in 0..96usize {
            let (px, py) = INFINITY_PATH[index];
            let x = left as i32 + width as i32 / 2 + px * scale as i32;
            let y = top as i32 + height as i32 * 72 / 100 + py * scale as i32 / 3;
            let active = index <= frame * 96 / 30;
            self.pixel(
                x,
                y,
                if active { 206 } else { 36 },
                if active { 232 } else { 67 },
                if active { 249 } else { 88 },
            );
            self.pixel(
                x + 1,
                y,
                if active { 206 } else { 36 },
                if active { 232 } else { 67 },
                if active { 249 } else { 88 },
            );
        }
        let (head_x, head_y) = infinity_point(phase);
        self.star_orb(
            left as i32 + width as i32 / 2 + head_x * scale as i32,
            top as i32 + height as i32 * 72 / 100 + head_y * scale as i32 / 3,
            4 * scale as i32,
            248,
            true,
        );
    }

    // ------------------------=
    // FUNC: installer_panel
    // DESC: Composes the active installer step, copy, and status into its main panel.
    // ------------------=
    pub(super) fn installer_panel(
        &mut self,
        lines: &[[u8; 96]; 6],
        lengths: &[usize; 6],
        line_count: usize,
        prompt: &[u8],
        command: &[u8],
        screen: u8,
        storage_device: Option<crate::storage::StorageDevice>,
        date_time: crate::storage::DateTimeConfiguration,
        date_time_part: usize,
        focus: usize,
        cursor_x: i32,
        cursor_y: i32,
        pressed: bool,
    ) {
        // Live date/time values use the same authored fields as Studio; input
        // hit testing is derived from those same roles in installer_layout.
        if screen==5 && self.template_scene_data_with_date_time(
            crate::ui::installer_layout::INSTALLER_TEMPLATE_BYTES,screen,
            Some((focus,true,Some((cursor_x,cursor_y,pressed)))),true,false,None,None,false,false,
            Some((date_time,date_time_part))) {return;}
        if screen!=5 {
            let (details, length) = crate::ui::installer_template::installer_live_details(
                lines, lengths, line_count, prompt, command,
            );
            if self.installer_template_screen_with_details(screen, &details[..length]) {
                return;
            }
        }
        let scale = self.ui_scale();
        let frame =
            crate::ui::installer_layout::installer_wizard_layout(screen, self.width, self.height);
        let left = frame.panel.left;
        let top = frame.panel.top;
        let width = frame.panel.width;
        let height = frame.panel.height;
        if screen == 1 {
            self.installer_welcome_panel(left, top, width, height);
            return;
        }
        if screen == 2 {
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            {
                self.installer_pool_panel(left, top, width, height);
                return;
            }
        }
        if screen == 3 {
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            {
                self.installer_disk_discovery_panel(left, top, width, height, storage_device);
                return;
            }
        }
        if screen == 5 {
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            {
                self.installer_date_time_panel(
                    left,
                    top,
                    width,
                    height,
                    date_time,
                    date_time_part,
                    focus,
                    cursor_x,
                    cursor_y,
                    pressed,
                );
                return;
            }
        }
        self.fill_rect_alpha(left, top, width, height, 7, 12, 19, 220);
        self.outline_rect(left, top, width, height, 55, 150, 210);
        self.outline_rect(
            left + 4 * scale,
            top + 4 * scale,
            width.saturating_sub(8 * scale),
            height.saturating_sub(8 * scale),
            25,
            88,
            128,
        );
        let fallback_title: &[u8] = match screen {
            1 => b"WELCOME TO INFINITYOS",
            2 => b"HOW INFINITY POOL WORKS",
            3 => b"CHOOSE INSTALLATION DISK",
            4 => b"REVIEW SELECTED DISK",
            5 => b"DATE & TIME",
            6 | 7 => b"REVIEW INSTALLATION",
            8 => b"INSTALLING INFINITYOS",
            9 => b"INSTALLATION COMPLETE",
            10 => b"INSTALLATION NEEDS ATTENTION",
            11 => b"INFINITYOS SETUP HELP",
            _ => b"INFINITYOS GUIDED SETUP",
        };
        let title = crate::ui::installer_layout::installer_template_text(
            screen,
            crate::ui::installer_template::InstallerTemplateRole::Title,
        )
        .unwrap_or(fallback_title);
        let inset = width * 20 / 1000;
        let header_y = top + height * 27 / 1000;
        self.installer_text_strong(left + inset, header_y, title, 220, 230, 241);
        let section: &[u8] = if screen == 11 {
            b"HELP / F1 TO RETURN"
        } else {
            b"GUIDED SETUP"
        };
        let section_width = self.installer_text_width(section, true);
        self.installer_text_strong(
            left + width.saturating_sub(inset + section_width),
            header_y,
            section,
            52,
            198,
            246,
        );
        self.fill_rect(
            left + inset,
            top + height * 86 / 1000,
            width.saturating_sub(inset * 2),
            scale,
            20,
            88,
            124,
        );
        self.installer_generic_content_frame(screen, frame.content, line_count);
        if screen == 1 {
            self.installer_welcome_top();
        }
        if screen == 8 {
            self.installer_activation_art(false);
        } else if screen == 9 {
            self.installer_activation_art(true);
        }
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        if screen == 1 || screen == 2 {
            let graphic_top_percent = if screen == 1 { 47 } else { 39 };
            let graphic_right_percent = if screen == 1 { 91 } else { 89 };
            let graphic_top = self.height * graphic_top_percent / 100;
            let graphic_width = if screen == 1 {
                self.width * 49 / 100
            } else {
                (self.width * 47 / 100).min((self.height * 40 / 100) * 16 / 9)
            };
            let graphic_left =
                (self.width * graphic_right_percent / 100).saturating_sub(graphic_width);
            // The welcome artwork lives behind the four feature cards and
            // must stop before the buttons. Deriving its height from pixel
            // width made it overflow on widescreen displays because width and
            // height percentages do not share the same physical scale.
            let graphic_height = if screen == 1 {
                (self.height * 79 / 100).saturating_sub(graphic_top)
            } else {
                graphic_width * 9 / 16
            };
            let bitmap = if screen == 1 {
                INSTALLER_MESH_HERO_BMP
            } else {
                STORAGE_HIERARCHY_BMP
            };
            if screen == 1 {
                self.paint_bitmap_cover_box(
                    bitmap,
                    graphic_left,
                    graphic_top,
                    graphic_width,
                    graphic_height,
                );
            } else {
                self.paint_bitmap_fit_rect(
                    bitmap,
                    graphic_left,
                    graphic_top,
                    graphic_width,
                    graphic_height,
                );
            }
            self.outline_rect(
                graphic_left,
                graphic_top,
                graphic_width,
                graphic_height,
                43,
                76,
                104,
            );
        }
        let authored_body = crate::ui::installer_layout::installer_template_text(
            screen,
            crate::ui::installer_template::InstallerTemplateRole::Body,
        );
        for row in 0..line_count.min(6) {
            if screen == 8 {
                break;
            }
            let y = if row == 0 {
                frame.content.top + 12 * scale
            } else {
                frame.content.top + 50 * scale + (row - 1) * 24 * scale
            };
            let color = if row == 0 {
                (244, 248, 253)
            } else {
                (216, 226, 237)
            };
            if row == 0 {
                self.installer_text_strong(
                    frame.content.left + 18 * scale,
                    y,
                    authored_body.unwrap_or(&lines[row][..lengths[row]]),
                    color.0,
                    color.1,
                    color.2,
                );
            } else {
                self.installer_text(
                    frame.content.left + 18 * scale,
                    y,
                    &lines[row][..lengths[row]],
                    color.0,
                    color.1,
                    color.2,
                );
            }
        }
        if screen == 1 {
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            self.installer_mesh_overview();
            self.installer_mesh_feature_overlay();
        }
        if !prompt.is_empty() && line_count < 7 {
            let y = top + (48 + line_count * 22) * scale;
            self.text(left + 24 * scale, y, prompt, 235, 241, 248);
            let prompt_width = self.installer_text_width(prompt, false);
            let command_width = self.installer_text_width(command, false);
            self.text(left + 24 * scale + prompt_width, y, command, 255, 255, 255);
            self.text(
                left + 24 * scale + prompt_width + command_width,
                y,
                b"_",
                255,
                255,
                255,
            );
        }
        if screen != 11 {
            let mut step = [b'0', b'0'];
            let visible_screen = match screen {
                7 => 6,
                8 => 7,
                9 | 10 => 8,
                _ => screen,
            };
            step[0] = b'0' + (visible_screen / 10);
            step[1] = b'0' + (visible_screen % 10);
            let step_x = left + width.saturating_sub(44 * scale);
            let step_y = self.height * 80 / 100;
            self.text(step_x, step_y, &step, 105, 154, 193);
        }
    }

    // ------------------------=
    // FUNC: installer_generic_content_frame
    // DESC: Gives text-led installer states a balanced glass content surface inside the shared safe area.
    // ------------------=
    pub(super) fn installer_generic_content_frame(
        &mut self,
        screen: u8,
        content: crate::ui::installer_layout::InstallerRect,
        line_count: usize,
    ) {
        if screen == 8 {
            return;
        }
        let scale = self.ui_scale();
        let (left, width) = if screen == 9 {
            (content.left, content.width * 46 / 100)
        } else {
            (content.left, content.width)
        };
        let height = if screen == 9 {
            content.height * 74 / 100
        } else {
            let copy_height = (line_count.max(3) * 21 * scale + 54 * scale)
                .min(content.height.saturating_sub(12 * scale));
            copy_height.max(content.height * 58 / 100)
        };
        self.fill_rounded_rect_alpha(left, content.top, width, height, 12 * scale, 2, 13, 24, 222);
        self.fill_rounded_rect_alpha(
            left + 2 * scale,
            content.top + 2 * scale,
            width.saturating_sub(4 * scale),
            height * 28 / 100,
            10 * scale,
            21,
            58,
            84,
            58,
        );
        let border = if screen == 10 {
            (174, 96, 86)
        } else {
            (39, 119, 158)
        };
        self.outline_rounded_rect(
            left,
            content.top,
            width,
            height,
            12 * scale,
            border.0,
            border.1,
            border.2,
        );
        self.installer_corner_accents(left, content.top, width, height);
        self.fill_rect(
            left + 18 * scale,
            content.top + 38 * scale,
            width.saturating_sub(36 * scale),
            scale,
            border.0,
            border.1,
            border.2,
        );
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: installer_pool_panel
    // DESC: Composes the approved second-step Infinity Pool explanation and topology.
    // ------------------=
    pub(super) fn installer_pool_panel(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        let scale = self.ui_scale();
        let edge = (width / 900).clamp(1, 3);
        let inset = width * 20 / 1000;
        self.fill_rect_alpha(left, top, width, height, 2, 10, 18, 232);
        self.fill_rect_alpha(left, top, width, height / 3, 4, 20, 33, 42);
        self.outline_rect(left, top, width, height, 16, 137, 190);
        self.outline_rect(
            left + 5 * edge,
            top + 5 * edge,
            width.saturating_sub(10 * edge),
            height.saturating_sub(10 * edge),
            20,
            73,
            104,
        );
        self.installer_corner_accents(left, top, width, height);

        let header_y = top + height * 27 / 1000;
        let screen_title = crate::ui::installer_layout::installer_template_text(
            2,
            crate::ui::installer_template::InstallerTemplateRole::Title,
        )
        .unwrap_or(b"HOW INFINITY POOL WORKS");
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
            edge,
            20,
            88,
            124,
        );

        let content_top = top + height * 115 / 1000;
        let content_bottom = self.height * 80 / 100;
        let content_height = content_bottom.saturating_sub(content_top);
        let left_panel = left + width * 18 / 1000;
        let left_width = width * 445 / 1000;
        let gap = width * 14 / 1000;
        let graphic_left = left_panel + left_width + gap;
        let graphic_width = (left + width).saturating_sub(graphic_left + inset);

        self.fill_rect_alpha(
            left_panel,
            content_top,
            left_width,
            content_height,
            2,
            11,
            20,
            202,
        );
        self.outline_rounded_rect(
            left_panel,
            content_top,
            left_width,
            content_height,
            12 * scale,
            19,
            85,
            119,
        );
        self.installer_corner_accents(left_panel, content_top, left_width, content_height);

        let copy_x = left_panel + width * 25 / 1000;
        let headline_y = content_top + height * 39 / 1000;
        self.installer_headline_text(copy_x, headline_y, b"ONE DISK BECOMES", 246, 249, 253);
        let second_prefix = b"PART OF THE ";
        let second_y = headline_y + INSTALLER_HEADLINE_FONT_CELL_HEIGHT + 2;
        self.installer_headline_text(copy_x, second_y, second_prefix, 246, 249, 253);
        let prefix_width = self.installer_headline_text_width(second_prefix);
        self.installer_headline_text(
            copy_x + prefix_width,
            second_y,
            b"INFINITY POOL.",
            49,
            202,
            247,
        );
        let copy_rule_y = second_y + INSTALLER_HEADLINE_FONT_CELL_HEIGHT + 5;
        self.fill_rect(
            copy_x,
            copy_rule_y,
            left_width.saturating_sub(width * 50 / 1000),
            edge,
            24,
            80,
            111,
        );

        let body_y = copy_rule_y + 15;
        let body = crate::ui::installer_layout::installer_template_text(
            2,
            crate::ui::installer_template::InstallerTemplateRole::Body,
        )
        .unwrap_or(b"The Infinity Pool organizes your disk into four protected areas that keep your data safe, isolated, and easy to recover.");
        self.installer_text_wrapped(
            copy_x,
            body_y,
            left_width.saturating_sub(width * 50 / 1000),
            body,
            193,
            207,
            222,
            3,
        );
        let taxonomy_y = body_y + 3 * 26 + 2;
        self.installer_text_strong(
            copy_x,
            taxonomy_y,
            b"SYSTEM | PERSONAL | APPLICATIONS | RECOVERY",
            48,
            201,
            246,
        );

        let cards_top = taxonomy_y + INSTALLER_FONT_CELL_HEIGHT + 5;
        let cards_bottom = content_bottom.saturating_sub(10 * scale);
        let cards_height = cards_bottom.saturating_sub(cards_top);
        self.installer_pool_space_cards(
            copy_x,
            cards_top,
            left_width.saturating_sub(width * 50 / 1000),
            cards_height,
        );

        self.fill_rect_alpha(
            graphic_left,
            content_top,
            graphic_width,
            content_height,
            1,
            10,
            19,
            220,
        );
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        self.paint_bitmap_fit_rect(
            STORAGE_HIERARCHY_BMP,
            graphic_left,
            content_top,
            graphic_width,
            content_height,
        );
        self.outline_rounded_rect(
            graphic_left,
            content_top,
            graphic_width,
            content_height,
            12 * scale,
            62,
            142,
            184,
        );
        self.installer_corner_accents(graphic_left, content_top, graphic_width, content_height);

        let pool_label = b"INFINITY POOL";
        let pool_label_width = self.installer_text_width(pool_label, true);
        self.installer_text_strong(
            graphic_left + graphic_width.saturating_sub(pool_label_width) / 2,
            content_top + content_height * 29 / 100,
            pool_label,
            54,
            203,
            247,
        );
        let labels: [&[u8]; 4] = [b"SYSTEM", b"PERSONAL", b"APPLICATIONS", b"RECOVERY"];
        for (index, label) in labels.iter().enumerate() {
            let center_x = graphic_left + graphic_width * (13 + index * 25) / 100;
            let label_width = self.installer_compact_text_width(label, true);
            self.installer_compact_text_strong(
                center_x.saturating_sub(label_width / 2),
                content_top + content_height * 88 / 100,
                label,
                54,
                203,
                247,
            );
        }

        self.installer_text(
            left + width.saturating_sub(inset + self.installer_text_width(b"02", false)),
            self.height * 80 / 100,
            b"02",
            105,
            194,
            232,
        );
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: installer_pool_space_cards
    // DESC: Draws the four equal System, Personal, Applications, and Recovery cards.
    // ------------------=
    pub(super) fn installer_pool_space_cards(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        let items: [(&[u8], &[u8], usize); 4] = [
            (b"SYSTEM", b"Core operating system and boot files.", 0),
            (b"PERSONAL", b"Your files, settings, and preferences.", 1),
            (b"APPLICATIONS", b"Installed apps and their data.", 2),
            (b"RECOVERY", b"Backups, restore tools and repair data.", 3),
        ];
        let gap = (width / 55).max(8);
        let card_width = width.saturating_sub(gap * 3) / 4;
        for (index, (title, body, icon)) in items.iter().enumerate() {
            let x = left + index * (card_width + gap);
            self.fill_rect_alpha(x, top, card_width, height, 2, 14, 24, 220);
            self.outline_rounded_rect(x, top, card_width, height, 10, 24, 111, 150);
            self.installer_corner_accents(x, top, card_width, height);
            let icon_size = (height * 38 / 100).clamp(36, 64);
            self.installer_pool_space_icon(
                x + card_width / 2,
                top + height * 27 / 100,
                *icon,
                icon_size,
            );

            let title_width = self.installer_compact_text_width(title, true);
            self.installer_compact_text_strong(
                x + card_width.saturating_sub(title_width) / 2,
                top + height * 55 / 100,
                title,
                50,
                201,
                246,
            );
            self.installer_compact_text_wrapped(
                x + 8,
                top + height * 69 / 100,
                card_width.saturating_sub(16),
                body,
                196,
                210,
                224,
                3,
            );
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: installer_pool_space_icon
    // DESC: Draws one compact vector icon for an Infinity Pool protected area.
    // ------------------=
    pub(super) fn installer_pool_space_icon(
        &mut self,
        center_x: usize,
        center_y: usize,
        kind: usize,
        size: usize,
    ) {
        let cx = center_x as i32;
        let cy = center_y as i32;
        let half = size as i32 / 2;
        let color = (47, 202, 247);
        self.icon_circle(cx, cy, half + 7, (21, 101, 139), size);
        self.icon_circle(cx, cy, half + 2, (24, 130, 174), size);
        match kind {
            0 => {
                self.line(
                    cx - half / 2,
                    cy - half / 2,
                    cx,
                    cy - half * 3 / 4,
                    color.0,
                    color.1,
                    color.2,
                );
                self.line(
                    cx,
                    cy - half * 3 / 4,
                    cx + half / 2,
                    cy - half / 2,
                    color.0,
                    color.1,
                    color.2,
                );
                self.line(
                    cx - half / 2,
                    cy - half / 2,
                    cx - half / 2,
                    cy + half / 5,
                    color.0,
                    color.1,
                    color.2,
                );
                self.line(
                    cx + half / 2,
                    cy - half / 2,
                    cx + half / 2,
                    cy + half / 5,
                    color.0,
                    color.1,
                    color.2,
                );
                self.line(
                    cx - half / 2,
                    cy + half / 5,
                    cx,
                    cy + half * 3 / 4,
                    color.0,
                    color.1,
                    color.2,
                );
                self.line(
                    cx + half / 2,
                    cy + half / 5,
                    cx,
                    cy + half * 3 / 4,
                    color.0,
                    color.1,
                    color.2,
                );
            }
            1 => {
                self.icon_circle(cx, cy - half / 3, half / 3, color, size);
                self.line(
                    cx - half / 2,
                    cy + half * 2 / 3,
                    cx - half / 2,
                    cy + half / 3,
                    color.0,
                    color.1,
                    color.2,
                );
                self.line(
                    cx - half / 2,
                    cy + half / 3,
                    cx,
                    cy + half / 8,
                    color.0,
                    color.1,
                    color.2,
                );
                self.line(
                    cx + half / 2,
                    cy + half / 3,
                    cx,
                    cy + half / 8,
                    color.0,
                    color.1,
                    color.2,
                );
                self.line(
                    cx + half / 2,
                    cy + half / 3,
                    cx + half / 2,
                    cy + half * 2 / 3,
                    color.0,
                    color.1,
                    color.2,
                );
            }
            2 => {
                let cell = (half * 2 / 3).max(4) as usize;
                for (dx, dy) in [
                    (-half * 2 / 3, -half * 2 / 3),
                    (half / 6, -half * 2 / 3),
                    (-half * 2 / 3, half / 6),
                    (half / 6, half / 6),
                ] {
                    self.outline_rounded_rect(
                        (cx + dx) as usize,
                        (cy + dy) as usize,
                        cell,
                        cell,
                        2,
                        color.0,
                        color.1,
                        color.2,
                    );
                }
            }
            _ => {
                self.icon_circle(cx, cy, half * 2 / 3, color, size);
                self.line(
                    cx - half * 3 / 4,
                    cy - half / 3,
                    cx - half / 3,
                    cy - half / 2,
                    color.0,
                    color.1,
                    color.2,
                );
                self.line(
                    cx - half * 3 / 4,
                    cy - half / 3,
                    cx - half * 2 / 3,
                    cy + half / 8,
                    color.0,
                    color.1,
                    color.2,
                );
            }
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: installer_disk_discovery_panel
    // DESC: Composes the live disk-discovery step from detected device facts and approved artwork.
    // ------------------=
    pub(super) fn installer_disk_discovery_panel(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        storage_device: Option<crate::storage::StorageDevice>,
    ) {
        let scale = self.ui_scale();
        let edge = (width / 900).clamp(1, 3);
        let inset = width * 20 / 1000;
        self.fill_rect_alpha(left, top, width, height, 2, 10, 18, 232);
        self.fill_rect_alpha(left, top, width, height / 3, 4, 20, 33, 42);
        self.outline_rect(left, top, width, height, 16, 137, 190);
        self.outline_rect(
            left + 5 * edge,
            top + 5 * edge,
            width.saturating_sub(10 * edge),
            height.saturating_sub(10 * edge),
            20,
            73,
            104,
        );
        self.installer_corner_accents(left, top, width, height);

        let header_y = top + height * 27 / 1000;
        let screen_title = crate::ui::installer_layout::installer_template_text(
            3,
            crate::ui::installer_template::InstallerTemplateRole::Title,
        )
        .unwrap_or(b"DISKS DISCOVERED");
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
            edge,
            20,
            88,
            124,
        );

        let content_top = top + height * 112 / 1000;
        let content_bottom = self.height * 80 / 100;
        let content_height = content_bottom.saturating_sub(content_top);
        let left_panel = left + width * 18 / 1000;
        let left_width = width * 465 / 1000;
        let gap = width * 12 / 1000;
        let right_panel = left_panel + left_width + gap;
        let right_width = (left + width).saturating_sub(right_panel + inset);

        self.fill_rect_alpha(
            left_panel,
            content_top,
            left_width,
            content_height,
            1,
            10,
            19,
            225,
        );
        self.outline_rounded_rect(
            left_panel,
            content_top,
            left_width,
            content_height,
            9 * scale,
            22,
            105,
            143,
        );
        self.installer_corner_accents(left_panel, content_top, left_width, content_height);

        let panel_inset = width * 10 / 1000;
        let available_y = content_top + 9 * scale;
        self.installer_text_strong(
            left_panel + panel_inset,
            available_y,
            b"AVAILABLE DISKS",
            45,
            207,
            249,
        );
        let found = if storage_device.is_some() {
            b"1 disk found".as_slice()
        } else {
            b"0 disks found".as_slice()
        };
        let found_width = self.installer_compact_text_width(found, true);
        self.installer_compact_text_strong(
            left_panel + left_width.saturating_sub(panel_inset + found_width),
            available_y + 3,
            found,
            45,
            207,
            249,
        );
        let body = crate::ui::installer_layout::installer_template_text(
            3,
            crate::ui::installer_template::InstallerTemplateRole::Body,
        )
        .unwrap_or(b"Select a disk to add to the Infinity Pool.");
        self.installer_compact_text(
            left_panel + panel_inset,
            available_y + INSTALLER_FONT_CELL_HEIGHT,
            body,
            183,
            207,
            232,
        );

        let card_top = content_top + content_height * 21 / 100;
        let card_height = content_height * 52 / 100;
        let card_left = left_panel + 2;
        let card_width = left_width.saturating_sub(4);
        if let Some(device) = storage_device {
            self.fill_rect_alpha(card_left, card_top, card_width, card_height, 3, 24, 38, 238);
            self.fill_rect_alpha(
                card_left,
                card_top,
                card_width,
                card_height / 2,
                20,
                67,
                92,
                44,
            );
            self.outline_rounded_rect(
                card_left,
                card_top,
                card_width,
                card_height,
                8 * scale,
                48,
                214,
                248,
            );
            self.fill_rect(card_left, card_top, 3 * scale, card_height, 116, 232, 255);

            let image_left = card_left + panel_inset;
            let image_top = card_top + 6 * scale;
            let image_width = card_width * 22 / 100;
            let image_height = card_height.saturating_sub(12 * scale);
            self.paint_bitmap_fit_rect(
                STORAGE_DEVICE_BMP,
                image_left,
                image_top,
                image_width,
                image_height,
            );

            let detail_x = image_left + image_width + panel_inset;
            let title_y = card_top + 7 * scale;
            self.installer_text_strong(detail_x, title_y, b"Disk 1", 230, 240, 250);
            let kind = Self::installer_disk_kind(&device);
            self.installer_compact_text(
                detail_x + self.installer_text_width(b"Disk 1", true) + 10,
                title_y + 3,
                kind,
                130,
                197,
                238,
            );

            let badge = b"RECOMMENDED";
            let badge_width = self.installer_compact_text_width(badge, true) + 16;
            let badge_left = card_left + card_width.saturating_sub(badge_width + panel_inset);
            self.fill_rounded_rect_alpha(
                badge_left,
                title_y,
                badge_width,
                INSTALLER_COMPACT_FONT_CELL_HEIGHT,
                5,
                19,
                72,
                99,
                245,
            );
            self.outline_rounded_rect(
                badge_left,
                title_y,
                badge_width,
                INSTALLER_COMPACT_FONT_CELL_HEIGHT,
                5,
                79,
                198,
                238,
            );
            self.installer_compact_text_strong(badge_left + 8, title_y, badge, 189, 225, 248);

            self.installer_compact_text(detail_x, title_y + 30, device.model(), 126, 192, 234);
            let (capacity, capacity_length) = Self::installer_capacity_label(device.capacity_mib());
            self.installer_compact_text(
                detail_x,
                title_y + 55,
                &capacity[..capacity_length],
                126,
                192,
                234,
            );
            let capacity_width =
                self.installer_compact_text_width(&capacity[..capacity_length], false);
            self.installer_compact_text(
                detail_x + capacity_width + 12,
                title_y + 55,
                device.bus,
                126,
                192,
                234,
            );
            let state = if device.has_gpt {
                b"Existing partitions detected".as_slice()
            } else {
                b"Empty disk - ready for InfinityOS".as_slice()
            };
            self.installer_compact_text(
                detail_x,
                card_top + card_height.saturating_sub(47),
                state,
                189,
                211,
                229,
            );
            let bar_left = detail_x;
            let bar_top = card_top + card_height.saturating_sub(18);
            let bar_width = (card_left + card_width).saturating_sub(panel_inset + bar_left);
            self.fill_rounded_rect_alpha(bar_left, bar_top, bar_width, 9, 4, 4, 9, 18, 255);
            self.outline_rounded_rect(bar_left, bar_top, bar_width, 9, 5, 62, 119, 151);
            self.fill_rounded_rect_alpha(
                bar_left + 1,
                bar_top + 1,
                bar_width.saturating_sub(2),
                7,
                4,
                161,
                207,
                235,
                235,
            );
        } else {
            self.fill_rect_alpha(card_left, card_top, card_width, card_height, 3, 17, 28, 225);
            self.outline_rounded_rect(card_left, card_top, card_width, card_height, 8, 34, 86, 118);
            self.installer_text_strong(
                card_left + panel_inset,
                card_top + card_height / 3,
                b"NO USABLE DISK FOUND",
                225,
                235,
                244,
            );
            self.installer_compact_text(
                card_left + panel_inset,
                card_top + card_height / 3 + 34,
                b"Attach a writable disk, then return and scan again.",
                145,
                177,
                205,
            );
        }

        let note_top = content_top + content_height * 77 / 100;
        self.icon_circle(
            (left_panel + panel_inset + 14) as i32,
            (note_top + 17) as i32,
            13,
            (70, 207, 250),
            28,
        );
        self.installer_compact_text_strong(
            left_panel + panel_inset + 10,
            note_top + 5,
            b"i",
            170,
            228,
            251,
        );
        self.installer_compact_text(
            left_panel + panel_inset + 38,
            note_top,
            b"You can add more disks later to grow your Infinity Pool.",
            145,
            201,
            235,
        );
        self.installer_compact_text(
            left_panel + panel_inset + 38,
            note_top + 24,
            b"All data on the selected disk will be used by InfinityOS.",
            145,
            201,
            235,
        );

        self.paint_bitmap_cover_box(
            DISK_DISCOVERY_VISION_BMP,
            right_panel,
            content_top,
            right_width,
            content_height,
        );
        self.fill_rect_alpha(
            right_panel,
            content_top,
            right_width * 47 / 100,
            content_height,
            0,
            7,
            15,
            62,
        );
        self.outline_rounded_rect(
            right_panel,
            content_top,
            right_width,
            content_height,
            9 * scale,
            52,
            112,
            147,
        );
        self.installer_corner_accents(right_panel, content_top, right_width, content_height);
        let vision_x = right_panel + right_width * 4 / 100;
        let vision_y = content_top + content_height * 6 / 100;
        self.installer_text_strong(vision_x, vision_y, b"MORE THAN STORAGE", 215, 232, 247);
        self.installer_text_strong(
            vision_x,
            vision_y + INSTALLER_FONT_CELL_HEIGHT,
            b"A STRONGER TOMORROW",
            215,
            232,
            247,
        );
        self.installer_compact_text(
            vision_x,
            vision_y + INSTALLER_FONT_CELL_HEIGHT * 2 + 7,
            b"EACH DISK BECOMES PART OF ONE SECURE POOL.",
            150,
            201,
            238,
        );
        self.installer_compact_text(
            vision_x,
            vision_y + INSTALLER_FONT_CELL_HEIGHT * 2 + 31,
            b"YOUR DATA. YOUR SYSTEM. BUILT TO GROW.",
            150,
            201,
            238,
        );
        let benefits_top = vision_y + INSTALLER_FONT_CELL_HEIGHT * 2 + 63;
        let benefits: [(&[u8], &[u8]); 4] = [
            (b"UNIFIED STORAGE", b"Multiple disks. One pool."),
            (b"BUILT FOR RESILIENCE", b"Your data stays protected."),
            (b"SCALES WITH YOU", b"Add disks anytime."),
            (b"FREEDOM TO CREATE", b"More space for what matters."),
        ];
        for (index, (title, subtitle)) in benefits.iter().enumerate() {
            let row_y = benefits_top + index * 45;
            self.installer_discovery_benefit_icon(vision_x + 17, row_y + 16, index, 30);
            self.installer_compact_text_strong(vision_x + 42, row_y, title, 164, 215, 246);
            self.installer_compact_text(vision_x + 42, row_y + 22, subtitle, 87, 177, 229);
        }
        let promise = b"PEOPLE + IDEAS + DATA";
        let promise_width = self.installer_compact_text_width(promise, true);
        self.installer_compact_text_strong(
            right_panel + right_width.saturating_sub(promise_width + 25),
            content_top + content_height.saturating_sub(30),
            promise,
            128,
            199,
            237,
        );

        let page = b"03";
        self.installer_text(
            left + width.saturating_sub(inset + self.installer_text_width(page, false)),
            self.height * 80 / 100,
            page,
            105,
            194,
            232,
        );
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: installer_discovery_benefit_icon
    // DESC: Draws one semantic vector icon for the disk-discovery vision panel.
    // ------------------=
    pub(super) fn installer_discovery_benefit_icon(
        &mut self,
        center_x: usize,
        center_y: usize,
        kind: usize,
        size: usize,
    ) {
        let half = size as i32 / 2;
        let cx = center_x as i32;
        let cy = center_y as i32;
        let color = (58, 210, 250);
        self.outline_rounded_rect(
            (cx - half - 4).max(0) as usize,
            (cy - half - 4).max(0) as usize,
            size + 8,
            size + 8,
            5,
            color.0,
            color.1,
            color.2,
        );
        match kind {
            0 => {
                for offset in [-half / 2, 0, half / 2] {
                    self.outline_rounded_rect(
                        (cx - half / 2).max(0) as usize,
                        (cy + offset - 3).max(0) as usize,
                        half.max(6) as usize,
                        6,
                        3,
                        color.0,
                        color.1,
                        color.2,
                    );
                }
            }
            1 => {
                self.line(
                    cx - half / 2,
                    cy - half / 2,
                    cx,
                    cy - half * 3 / 4,
                    color.0,
                    color.1,
                    color.2,
                );
                self.line(
                    cx,
                    cy - half * 3 / 4,
                    cx + half / 2,
                    cy - half / 2,
                    color.0,
                    color.1,
                    color.2,
                );
                self.line(
                    cx - half / 2,
                    cy - half / 2,
                    cx - half / 2,
                    cy + half / 5,
                    color.0,
                    color.1,
                    color.2,
                );
                self.line(
                    cx + half / 2,
                    cy - half / 2,
                    cx + half / 2,
                    cy + half / 5,
                    color.0,
                    color.1,
                    color.2,
                );
                self.line(
                    cx - half / 2,
                    cy + half / 5,
                    cx,
                    cy + half * 3 / 4,
                    color.0,
                    color.1,
                    color.2,
                );
                self.line(
                    cx + half / 2,
                    cy + half / 5,
                    cx,
                    cy + half * 3 / 4,
                    color.0,
                    color.1,
                    color.2,
                );
            }
            2 => {
                for (index, bar) in [half / 3, half * 2 / 3, half].iter().enumerate() {
                    self.fill_rect(
                        (cx - half + index as i32 * half * 2 / 3).max(0) as usize,
                        (cy + half - *bar).max(0) as usize,
                        (half / 3).max(3) as usize,
                        (*bar).max(3) as usize,
                        color.0,
                        color.1,
                        color.2,
                    );
                }
            }
            _ => {
                self.icon_circle(cx - half / 3, cy, half / 2, color, size);
                self.icon_circle(cx + half / 3, cy, half / 2, color, size);
                self.line(
                    cx - half * 2 / 3,
                    cy - half / 3,
                    cx + half * 2 / 3,
                    cy + half / 3,
                    color.0,
                    color.1,
                    color.2,
                );
                self.line(
                    cx - half * 2 / 3,
                    cy + half / 3,
                    cx + half * 2 / 3,
                    cy - half / 3,
                    color.0,
                    color.1,
                    color.2,
                );
            }
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: installer_disk_kind
    // DESC: Maps discovered hardware traits to a concise user-facing device class.
    // ------------------=
    pub(super) fn installer_disk_kind(device: &crate::storage::StorageDevice) -> &'static [u8] {
        if device.removable {
            b"(REMOVABLE)"
        } else if device.bus.windows(4).any(|window| window == b"UEFI") {
            b"(VIRTUAL DISK)"
        } else {
            b"(ATA STORAGE)"
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: installer_capacity_label
    // DESC: Formats a detected disk capacity without floating point or fabricated free-space data.
    // ------------------=
    pub(super) fn installer_capacity_label(capacity_mib: u64) -> ([u8; 24], usize) {
        let mut label = [0u8; 24];
        let mut length = 0usize;
        if capacity_mib >= 1024 * 1024 {
            let tenths = capacity_mib.saturating_mul(10) / (1024 * 1024);
            Self::installer_append_decimal(&mut label, &mut length, tenths / 10);
            if length + 2 < label.len() {
                label[length] = b'.';
                label[length + 1] = b'0' + (tenths % 10) as u8;
                length += 2;
            }
            Self::installer_append_label(&mut label, &mut length, b" TB");
        } else if capacity_mib >= 1024 {
            let tenths = capacity_mib.saturating_mul(10) / 1024;
            Self::installer_append_decimal(&mut label, &mut length, tenths / 10);
            if length + 2 < label.len() {
                label[length] = b'.';
                label[length + 1] = b'0' + (tenths % 10) as u8;
                length += 2;
            }
            Self::installer_append_label(&mut label, &mut length, b" GB");
        } else {
            Self::installer_append_decimal(&mut label, &mut length, capacity_mib);
            Self::installer_append_label(&mut label, &mut length, b" MB");
        }
        (label, length)
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: installer_append_decimal
    // DESC: Appends one unsigned decimal value into a fixed installer label buffer.
    // ------------------=
    pub(super) fn installer_append_decimal(buffer: &mut [u8], length: &mut usize, mut value: u64) {
        let mut digits = [0u8; 20];
        let mut count = 0usize;
        loop {
            digits[count] = b'0' + (value % 10) as u8;
            count += 1;
            value /= 10;
            if value == 0 {
                break;
            }
        }
        while count > 0 && *length < buffer.len() {
            count -= 1;
            buffer[*length] = digits[count];
            *length += 1;
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: installer_append_label
    // DESC: Appends static units or punctuation into a fixed installer label buffer.
    // ------------------=
    pub(super) fn installer_append_label(buffer: &mut [u8], length: &mut usize, suffix: &[u8]) {
        for byte in suffix {
            if *length >= buffer.len() {
                break;
            }
            buffer[*length] = *byte;
            *length += 1;
        }
    }

    // ------------------------=
    // FUNC: installer_welcome_panel
    // DESC: Composes the first installer screen to the approved reference geometry and content.
    // ------------------=
    pub(super) fn installer_welcome_panel(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        let edge = (width / 900).clamp(1, 3);
        let inset = width * 18 / 1000;
        self.fill_rect_alpha(left, top, width, height, 2, 10, 18, 226);
        self.fill_rect_alpha(left, top, width, height / 3, 4, 20, 33, 52);
        self.outline_rect(left, top, width, height, 16, 137, 190);
        self.outline_rect(
            left + 5 * edge,
            top + 5 * edge,
            width.saturating_sub(10 * edge),
            height.saturating_sub(10 * edge),
            20,
            73,
            104,
        );
        self.installer_corner_accents(left, top, width, height);

        let header_y = top + height * 24 / 1000;
        let screen_title = crate::ui::installer_layout::installer_template_text(
            1,
            crate::ui::installer_template::InstallerTemplateRole::Title,
        )
        .unwrap_or(b"WELCOME TO INFINITYOS");
        self.installer_text_strong(left + inset, header_y, screen_title, 220, 230, 241);
        let section = b"GUIDED SETUP";
        let section_width = self.installer_text_width(section, true);
        let section_x = left + width.saturating_sub(inset + section_width + 30);
        self.installer_text_strong(section_x, header_y, section, 52, 198, 246);
        self.installer_mesh_icon(
            left + width.saturating_sub(inset + 10),
            header_y + UI_FONT_CELL_HEIGHT / 2,
            2,
            14,
        );
        let divider_y = top + height * 82 / 1000;
        self.fill_rect(
            left + inset,
            divider_y,
            width.saturating_sub(inset * 2),
            edge,
            20,
            88,
            124,
        );

        let content_top = top + height * 112 / 1000;
        let left_column = left + width * 15 / 1000;
        let left_width = width * 380 / 1000;
        let hero_left = left + width * 410 / 1000;
        let hero_width = width * 575 / 1000;
        let callout_height = height * 375 / 1000;
        self.installer_welcome_callout(left_column, content_top, left_width, callout_height);

        let hero_image_height = height * 430 / 1000;
        self.paint_bitmap_cover_box(
            INSTALLER_MESH_HERO_BMP,
            hero_left,
            content_top,
            hero_width,
            hero_image_height,
        );
        self.outline_rect(
            hero_left,
            content_top,
            hero_width,
            hero_image_height,
            20,
            76,
            108,
        );

        // Keep the lower feature row within the shared content safe area so it
        // never competes with the invariant navigation controls.
        let cards_top = top + height * 525 / 1000;
        let cards_height = height * 200 / 1000;
        self.installer_mesh_overview_row(left_column, cards_top, left_width, cards_height);
        self.installer_mesh_feature_row(hero_left, cards_top, hero_width, cards_height);
    }

    // ------------------------=
    // FUNC: installer_corner_accents
    // DESC: Draws the short cyan corner brackets used by the approved setup frame.
    // ------------------=
    pub(super) fn installer_corner_accents(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        let length = (width / 110).clamp(10, 24) as i32;
        let inset = (width / 1000).clamp(2, 4) as i32;
        let color = (32, 173, 222);
        let x0 = left as i32 + inset;
        let y0 = top as i32 + inset;
        let x1 = (left + width) as i32 - inset - 1;
        let y1 = (top + height) as i32 - inset - 1;
        for (x, y, dx, dy) in [
            (x0, y0, 1, 1),
            (x1, y0, -1, 1),
            (x0, y1, 1, -1),
            (x1, y1, -1, -1),
        ] {
            self.line(x, y, x + dx * length, y, color.0, color.1, color.2);
            self.line(x, y, x, y + dy * length, color.0, color.1, color.2);
        }
    }

    // ------------------------=
    // FUNC: installer_welcome_callout
    // DESC: Draws the exact welcome message and approval promise in the upper-left panel.
    // ------------------=
    pub(super) fn installer_welcome_callout(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        self.fill_rect_alpha(left, top, width, height, 1, 10, 19, 194);
        self.outline_rect(left, top, width, height, 18, 83, 119);
        self.installer_corner_accents(left, top, width, height);
        self.installer_text_strong(left + 42, top + 15, b"WELCOME TO INFINITYOS", 50, 203, 246);
        self.line(
            (left + 18) as i32,
            (top + 26) as i32,
            (left + 29) as i32,
            (top + 26) as i32,
            48,
            200,
            244,
        );
        self.line(
            (left + 24) as i32,
            (top + 20) as i32,
            (left + 30) as i32,
            (top + 26) as i32,
            48,
            200,
            244,
        );
        self.line(
            (left + 24) as i32,
            (top + 32) as i32,
            (left + 30) as i32,
            (top + 26) as i32,
            48,
            200,
            244,
        );
        let body_x = left + 43;
        let body_y = top + 55;
        let body = crate::ui::installer_layout::installer_template_text(
            1,
            crate::ui::installer_template::InstallerTemplateRole::Body,
        )
        .unwrap_or(b"InfinityOS is a distributed operating system. One simple home for your system, your apps, and everything you create. We'll guide you through every choice. Nothing changes until you approve it.");
        self.installer_text_wrapped(
            body_x,
            body_y,
            width.saturating_sub(86),
            body,
            193,
            207,
            222,
            6,
        );
        self.installer_text(
            body_x,
            top + height.saturating_sub(39),
            b"You're in control. Always.",
            51,
            202,
            245,
        );
    }

    // ------------------------=
    // FUNC: installer_mesh_overview_row
    // DESC: Draws the generated panoramic mesh-compute diagram beneath its centered section label.
    // ------------------=
    pub(super) fn installer_mesh_overview_row(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        let label = b"MESH COMPUTING OVERVIEW";
        let label_width = self.installer_compact_text_width(label, true);
        let label_x = left + width.saturating_sub(label_width) / 2;
        let label_y = top.saturating_sub(31);
        self.fill_rect(
            left,
            label_y + 10,
            label_x.saturating_sub(left + 14),
            1,
            23,
            91,
            128,
        );
        self.fill_rect(
            label_x + label_width + 14,
            label_y + 10,
            left + width.saturating_sub(label_x + label_width + 14),
            1,
            23,
            91,
            128,
        );
        self.installer_compact_text_strong(label_x, label_y, label, 63, 181, 226);
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        self.paint_bitmap_fit_rect(INSTALLER_MESH_DIAGRAM_BMP, left, top, width, height);
        #[cfg(target_arch = "x86")]
        self.fill_rect_alpha(left, top, width, height, 2, 10, 18, 226);
        self.outline_rect(left, top, width, height, 27, 91, 130);
    }

    // ------------------------=
    // FUNC: installer_mesh_feature_row
    // DESC: Places the four approved mesh principles as one aligned strip beneath the hero.
    // ------------------=
    pub(super) fn installer_mesh_feature_row(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        let features: [(&[u8], &[u8]); 4] = [
            (
                b"DISTRIBUTED BY DESIGN",
                b"Computes share power and resources across a secure mesh of devices.",
            ),
            (
                b"RESILIENT & ADAPTIVE",
                b"If one node goes offline, the mesh adapts and continues seamlessly.",
            ),
            (
                b"YOURS TO CONTROL",
                b"Every device in the mesh is yours. Your data. Your rules.",
            ),
            (
                b"BUILT FOR TOMORROW",
                b"Scale from one device to thousands. InfinityOS grows with you.",
            ),
        ];
        let card_width = width / 4;
        for (index, (title, body)) in features.iter().enumerate() {
            let x = left + index * card_width;
            let actual_width = if index == 3 {
                width.saturating_sub(card_width * 3)
            } else {
                card_width
            };
            self.fill_rect_alpha(x, top, actual_width, height, 2, 12, 21, 220);
            self.outline_rect(x, top, actual_width, height, 20, 82, 116);
            let marker = [b'0', b'1' + index as u8];
            self.outline_rect(x + 17, top + 17, 29, 25, 35, 141, 185);
            self.installer_compact_text_strong(x + 22, top + 20, &marker, 52, 203, 244);
            self.installer_compact_text_strong(x + 56, top + 20, title, 53, 196, 238);
            self.installer_compact_text_wrapped(
                x + 20,
                top + 62,
                actual_width.saturating_sub(40),
                body,
                183,
                200,
                218,
                4,
            );
        }
    }

    // ------------------------=
    // FUNC: installer_mesh_icon
    // DESC: Draws one crisp vector icon for the mesh overview cards without baked UI pixels.
    // ------------------=
    pub(super) fn installer_mesh_icon(
        &mut self,
        center_x: usize,
        center_y: usize,
        kind: usize,
        size: usize,
    ) {
        let cx = center_x as i32;
        let cy = center_y as i32;
        let half = size.max(12) as i32 / 2;
        let color = (48, 202, 246);
        match kind {
            0 => {
                self.outline_rect(
                    (cx - half * 2 / 3).max(0) as usize,
                    (cy - half * 2 / 3).max(0) as usize,
                    (half * 4 / 3) as usize,
                    (half * 4 / 3) as usize,
                    color.0,
                    color.1,
                    color.2,
                );
                for offset in [-half, half] {
                    self.line(
                        cx + offset,
                        cy - half / 2,
                        cx + offset * 2 / 3,
                        cy - half / 2,
                        color.0,
                        color.1,
                        color.2,
                    );
                    self.line(
                        cx + offset,
                        cy + half / 2,
                        cx + offset * 2 / 3,
                        cy + half / 2,
                        color.0,
                        color.1,
                        color.2,
                    );
                }
                self.star_orb(cx, cy, (half / 4).max(2), 250, false);
            }
            1 => {
                self.outline_rounded_rect(
                    (cx - half * 2 / 3).max(0) as usize,
                    cy.max(0) as usize,
                    (half * 4 / 3) as usize,
                    half as usize,
                    3,
                    color.0,
                    color.1,
                    color.2,
                );
                self.icon_circle(cx, cy - half / 4, half / 2, color, size);
                self.line(
                    cx,
                    cy + half / 4,
                    cx,
                    cy + half / 2,
                    color.0,
                    color.1,
                    color.2,
                );
            }
            2 => {
                let nodes = [
                    (cx, cy - half),
                    (cx - half, cy + half / 2),
                    (cx + half, cy + half / 2),
                    (cx, cy),
                ];
                for (x, y) in nodes {
                    self.icon_circle(x, y, (half / 5).max(2), color, size);
                }
                self.line(cx, cy, cx, cy - half, color.0, color.1, color.2);
                self.line(cx, cy, cx - half, cy + half / 2, color.0, color.1, color.2);
                self.line(cx, cy, cx + half, cy + half / 2, color.0, color.1, color.2);
            }
            _ => {
                self.outline_rect(
                    (cx - half).max(0) as usize,
                    (cy - half).max(0) as usize,
                    (half * 2) as usize,
                    (half * 2) as usize,
                    color.0,
                    color.1,
                    color.2,
                );
                self.line(cx - half, cy, cx + half, cy, color.0, color.1, color.2);
                self.line(cx, cy - half, cx, cy + half, color.0, color.1, color.2);
                self.star_orb(cx, cy, (half / 5).max(2), 250, false);
            }
        }
    }

    // ------------------------=
    // FUNC: installer_welcome_top
    // DESC: Draws the centered first-screen tagline beneath the InfinityOS masthead.
    // ------------------=
    pub(super) fn installer_welcome_top(&mut self) {
        let tagline = b"BOUNDLESS BY DESIGN. CONNECTED BY CHOICE.";
        let tagline_width = self.installer_text_width(tagline, false);
        self.text(
            self.width.saturating_sub(tagline_width) / 2,
            self.height * 34 / 100,
            tagline,
            80,
            175,
            229,
        );
    }

    // ------------------------=
    // FUNC: installer_mesh_feature_overlay
    // DESC: Places the four mesh-computing principles inside the unified welcome panel.
    // ------------------=
    pub(super) fn installer_mesh_feature_overlay(&mut self) {
        let features: [(&[u8], &[u8], &[u8], &[u8]); 4] = [
            (
                b"DISTRIBUTED BY DESIGN",
                b"Computes share power and",
                b"resources across a secure",
                b"mesh of devices.",
            ),
            (
                b"RESILIENT & ADAPTIVE",
                b"If one node goes offline,",
                b"the mesh adapts and",
                b"continues.",
            ),
            (
                b"YOURS TO CONTROL",
                b"Every device in the mesh",
                b"is yours. Your data.",
                b"Your rules.",
            ),
            (
                b"BUILT FOR TOMORROW",
                b"Scale from one device to",
                b"thousands. InfinityOS",
                b"grows with you.",
            ),
        ];
        let area_left = self.width * 42 / 100;
        let area_top = self.height * 62 / 100;
        let area_width = self.width * 49 / 100;
        let gap = self.width / 200;
        let card_width = area_width.saturating_sub(gap) / 2;
        let card_height = self.height * 8 / 100;
        let row_gap = self.height / 200;
        for (index, (title, first, second, third)) in features.iter().enumerate() {
            let column = index % 2;
            let row = index / 2;
            let x = area_left + column * (card_width + gap);
            let y = area_top + row * (card_height + row_gap);
            self.installer_feature_card(
                index,
                x,
                y,
                card_width,
                card_height,
                *title,
                *first,
                *second,
                *third,
            );
        }
    }

    // ------------------------=
    // FUNC: installer_feature_card
    // DESC: Draws one translucent mesh principle card over the welcome illustration.
    // ------------------=
    pub(super) fn installer_feature_card(
        &mut self,
        index: usize,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        title: &[u8],
        first: &[u8],
        second: &[u8],
        third: &[u8],
    ) {
        let scale = self.ui_scale();
        // These four cards intentionally use a fixed two-pixel glyph scale.
        // Do not tie their legibility to framebuffer or card dimensions: the
        // previous adaptive fallback made the copy tiny on the ARM display.
        let font_scale = 2;
        self.fill_rect_alpha(x, y, width, height, 4, 13, 23, 210);
        self.outline_rect(x, y, width, height, 29, 91, 130);
        self.fill_rect(x, y, 2 * scale, height, 72, 192, 242);
        let mut marker = [b'0', b'1'];
        marker[1] = b'1' + index as u8;
        self.text_scaled(
            x + 5 * font_scale,
            y + 6 * font_scale,
            &marker,
            76,
            204,
            250,
            font_scale,
            true,
        );
        let title_x = x + 25 * font_scale;
        let body_x = x + 5 * font_scale;
        self.text_scaled(
            title_x,
            y + 6 * font_scale,
            title,
            78,
            201,
            245,
            font_scale,
            true,
        );
        self.text_scaled(
            body_x,
            y + 23 * font_scale,
            first,
            185,
            205,
            225,
            font_scale,
            false,
        );
        self.text_scaled(
            body_x,
            y + 35 * font_scale,
            second,
            185,
            205,
            225,
            font_scale,
            false,
        );
        self.text_scaled(
            body_x,
            y + 47 * font_scale,
            third,
            185,
            205,
            225,
            font_scale,
            false,
        );
    }

    // ------------------------=
    // FUNC: installer_mesh_node
    // DESC: Draws one reusable glowing device node for the mesh-status topology.
    // ------------------=
    pub(super) fn installer_mesh_node(&mut self, center_x: usize, center_y: usize, kind: usize) {
        let scale = self.ui_scale();
        let size = 12 * scale;
        let left = center_x.saturating_sub(size / 2);
        let top = center_y.saturating_sub(size / 2);
        self.outline_rect(left, top, size, size, 49, 145, 204);
        self.outline_rect(
            left + 2 * scale,
            top + 2 * scale,
            size.saturating_sub(4 * scale),
            size.saturating_sub(4 * scale),
            118,
            217,
            255,
        );
        if kind % 3 == 0 {
            self.fill_rect(
                left + 3 * scale,
                top + 8 * scale,
                6 * scale,
                scale,
                105,
                211,
                255,
            );
        } else if kind % 3 == 1 {
            self.line(
                (left + 3 * scale) as i32,
                (top + 3 * scale) as i32,
                (left + 9 * scale) as i32,
                (top + 9 * scale) as i32,
                105,
                211,
                255,
            );
        } else {
            self.outline_rect(
                left + 4 * scale,
                top + 2 * scale,
                4 * scale,
                8 * scale,
                105,
                211,
                255,
            );
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: installer_mesh_overview
    // DESC: Composes the generated planetary mesh card and its exact explanatory labels.
    // ------------------=
    pub(super) fn installer_mesh_overview(&mut self) {
        let scale = self.ui_scale();
        let font_scale = 2;
        let left = self.width * 8 / 100;
        let top = self.height * 61 / 100;
        let width = self.width * 33 / 100;
        let height = self.height * 18 / 100;
        self.paint_bitmap_fit_rect(INSTALLER_MESH_OVERVIEW_BMP, left, top, width, height);
        self.outline_rect(left, top, width, height, 31, 83, 119);
        self.text_scaled(
            left + 9 * scale,
            top + 8 * scale,
            b"MESH COMPUTING OVERVIEW",
            75,
            203,
            250,
            font_scale,
            true,
        );
        let items: [(&[u8], &[u8], &[u8], &[u8]); 4] = [
            (b"SHARE RESOURCES", b"CPU  GPU", b"STORAGE", b"BANDWIDTH"),
            (b"SYNCHRONIZE DATA", b"FAST  SECURE", b"PRIVATE", b""),
            (b"WORK TOGETHER", b"ONE SYSTEM", b"MANY DEVICES", b""),
            (b"STAY IN CONTROL", b"YOU DECIDE", b"WHAT'S SHARED", b""),
        ];
        let horizontal_padding = width * 3 / 100;
        let horizontal_gap = width * 3 / 100;
        let content_top = top + height * 24 / 100;
        let content_height = height * 72 / 100;
        let vertical_gap = height * 3 / 100;
        let cell_width = width.saturating_sub(horizontal_padding * 2 + horizontal_gap) / 2;
        let cell_height = content_height.saturating_sub(vertical_gap) / 2;
        for (index, (title, first_detail, second_detail, third_detail)) in items.iter().enumerate()
        {
            let column = index % 2;
            let row = index / 2;
            let cell_left = left + horizontal_padding + column * (cell_width + horizontal_gap);
            let cell_top = content_top + row * (cell_height + vertical_gap);
            self.fill_rect_alpha(cell_left, cell_top, cell_width, cell_height, 3, 13, 23, 164);
            self.outline_rect(cell_left, cell_top, cell_width, cell_height, 27, 91, 130);
            self.fill_rect(cell_left, cell_top, 2 * scale, cell_height, 67, 184, 232);
            let text_x = cell_left + 5 * font_scale;
            let text_y = cell_top + 4 * font_scale;
            self.text_scaled(text_x, text_y, title, 72, 197, 242, font_scale, true);
            let details = [*first_detail, *second_detail, *third_detail];
            for (detail_index, detail) in details.iter().enumerate() {
                if detail.is_empty() {
                    continue;
                }
                self.text_scaled(
                    text_x,
                    text_y + (11 + detail_index * 9) * font_scale,
                    detail,
                    178,
                    202,
                    224,
                    font_scale,
                    false,
                );
            }
        }
    }


}

// Read-only debugger progress; no installer authority or input channel.
#[used]
#[no_mangle]
static mut INFINITY_INSTALL_PROGRESS: [u64; 4] = [0x494e46494e535431, 0, 0, 0];

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: installer_progress_update
// DESC: Presents measured installation progress without inventing intermediate completion or delaying disk work.
// ------------------=
pub fn installer_progress_update(percent: u8, label: &[u8]) {
    unsafe {
        let slot = &raw mut CONSOLE;
        if let Some(console) = (*slot).as_mut() {
            console.restore_cursor();
            let target = (percent as usize).min(100);
            console.installer_animation_phase = (console.installer_animation_phase + 5) % 384;
            console.display.installer_progress_frame(target, label, console.installer_animation_phase);
            console.installer_progress = target;
            console.save_and_draw_cursor(console.cursor_x, console.cursor_y);
            console.display.present_damage();
            let state = &raw mut INFINITY_INSTALL_PROGRESS;
            let sequence = (*state)[1].wrapping_add(2) & !1;
            core::ptr::write_volatile(state.cast::<u64>().add(1), sequence | 1);
            core::ptr::write_volatile(state.cast::<u64>().add(2), target as u64);
            core::ptr::write_volatile(state.cast::<u64>().add(3), sequence);
            core::ptr::write_volatile(state.cast::<u64>().add(1), sequence);
        }
    }
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: installer_progress_update
// DESC: Provides a text-only fallback when the architecture has no graphical installer surface.
// ------------------=
pub fn installer_progress_update(_percent: u8, _label: &[u8]) {}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: installer_reboot_countdown
// DESC: Presents a three-second animated transition before firmware reset.
// ------------------=
pub fn installer_reboot_countdown() {
    unsafe {
        let slot = &raw mut CONSOLE;
        if let Some(console) = (*slot).as_mut() {
            console.restore_cursor();
            for remaining in (1..=3usize).rev() {
                for frame in 0..30usize {
                    console.installer_animation_phase =
                        (console.installer_animation_phase + 4) % 384;
                    console.display.installer_countdown_frame(
                        remaining,
                        frame,
                        console.installer_animation_phase,
                    );
                    console.display.present_damage();
                    super::bootstrap::wait_frame(33);
                }
            }
            console
                .display
                .installer_countdown_frame(0, 30, console.installer_animation_phase);
            console.display.present_damage();
            super::bootstrap::wait_frame(220);
        }
    }
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: installer_reboot_countdown
// DESC: Provides an immediate text-mode transition on architectures without graphical countdown support.
// ------------------=
pub fn installer_reboot_countdown() {}
