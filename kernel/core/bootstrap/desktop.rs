//! Authentication, onboarding, desktop shell, menus, settings, windows, and file management.

use super::*;

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const AUTHENTICATION_BMP: &[u8] =
    include_bytes!("../../../assets/desktop/infinity-default-dark-wallpaper-v2.bmp");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const DESKTOP_BMP: &[u8] =
    include_bytes!("../../../assets/desktop/infinity-shell-wallpaper-v3.bmp");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const ONBOARDING_BMP: &[u8] =
    include_bytes!("../../../assets/desktop/infinity-onboarding-wallpaper-v1.bmp");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const TOP_BAR_INFINITY_BMP: &[u8] =
    include_bytes!("../../../assets/desktop/infinity-topbar-icon-v2.bmp");
#[cfg(all(
    not(feature = "installer"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const CRYSTAL_BLUE_GLASS_BASE_BMP: &[u8] =
    include_bytes!("../../../assets/icons/runtime/crystal-blue-glass-base.bmp");
#[cfg(all(
    not(feature = "installer"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const CRYSTAL_BLUE_GLASS_ACTIONS_BMP: &[u8] =
    include_bytes!("../../../assets/icons/runtime/crystal-blue-glass-actions.bmp");
#[cfg(all(
    not(feature = "installer"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const LUMINOUS_OBSIDIAN_BASE_BMP: &[u8] =
    include_bytes!("../../../assets/icons/runtime/luminous-obsidian-base.bmp");
#[cfg(all(
    not(feature = "installer"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const LUMINOUS_OBSIDIAN_ACTIONS_BMP: &[u8] =
    include_bytes!("../../../assets/icons/runtime/luminous-obsidian-actions.bmp");
#[cfg(all(
    not(feature = "installer"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const FROSTED_QUARTZ_BASE_BMP: &[u8] =
    include_bytes!("../../../assets/icons/runtime/frosted-quartz-base.bmp");
#[cfg(all(
    not(feature = "installer"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const FROSTED_QUARTZ_ACTIONS_BMP: &[u8] =
    include_bytes!("../../../assets/icons/runtime/frosted-quartz-actions.bmp");
#[cfg(all(
    not(feature = "installer"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const CRYSTAL_BLUE_GLASS_LAUNCHER_BMP: &[u8] =
    include_bytes!("../../../assets/icons/runtime/crystal-blue-glass-launcher-256.bmp");
#[cfg(all(
    not(feature = "installer"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const LUMINOUS_OBSIDIAN_LAUNCHER_BMP: &[u8] =
    include_bytes!("../../../assets/icons/runtime/luminous-obsidian-launcher-256.bmp");
#[cfg(all(
    not(feature = "installer"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const FROSTED_QUARTZ_LAUNCHER_BMP: &[u8] =
    include_bytes!("../../../assets/icons/runtime/frosted-quartz-launcher-256.bmp");

impl super::DisplayDevice {
    // ------------------------=
    // FUNC: active_accent_surface
    // DESC: Resolves one user-selected semantic accent surface into framebuffer channels.
    // ------------------=
    pub(super) fn active_accent_surface(
        &self,
        surface: crate::ui::skin::AccentSurface,
    ) -> (u8, u8, u8) {
        crate::runtime::with_runtime(|runtime| runtime.ui.skins.accent_surface(surface).channels())
            .unwrap_or((32, 191, 255))
    }

    // ------------------------=
    // FUNC: active_accent_rgb
    // DESC: Returns the authenticated user's current portable RGB accent.
    // ------------------=
    pub(super) fn active_accent_rgb(&self) -> u32 {
        crate::runtime::with_runtime(|runtime| runtime.ui.skins.accent_rgb())
            .unwrap_or(crate::runtime::identity::DEFAULT_ACCENT_RGB)
    }

    // ------------------------=
    // FUNC: active_primary_rgb
    // DESC: Returns the current portable RGB primary surface color.
    // ------------------=
    pub(super) fn active_primary_rgb(&self) -> u32 {
        crate::runtime::with_runtime(|runtime| runtime.ui.skins.primary_rgb())
            .unwrap_or(crate::runtime::identity::DEFAULT_PRIMARY_RGB)
    }

    // ------------------------=
    // FUNC: active_background_effects
    // DESC: Returns the active background-only opacity percentage and blur radius.
    // ------------------=
    pub(super) fn active_background_effects(&self) -> (u8, u8) {
        crate::runtime::with_runtime(|runtime| runtime.ui.skins.background_effects()).unwrap_or((
            crate::runtime::identity::DEFAULT_BACKGROUND_OPACITY,
            crate::runtime::identity::DEFAULT_BACKGROUND_BLUR,
        ))
    }

    // ------------------------=
    // FUNC: active_background_alpha
    // DESC: Scales one panel fill alpha without changing outlines, controls, text, or focus treatments.
    // ------------------=
    pub(super) fn active_background_alpha(&self, base: u8) -> u8 {
        crate::runtime::with_runtime(|runtime| runtime.ui.skins.background_alpha(base))
            .unwrap_or(base)
    }

    // ------------------------=
    // FUNC: active_icon_theme
    // DESC: Reads the current user-scoped icon family for surface-independent semantic rendering.
    // ------------------=
    pub(super) fn active_icon_theme(&self) -> u8 {
        crate::runtime::with_runtime(|runtime| runtime.ui.icons.active() as u8).unwrap_or(0)
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
            (1, false) => LUMINOUS_OBSIDIAN_BASE_BMP,
            (1, true) => LUMINOUS_OBSIDIAN_ACTIONS_BMP,
            (2, false) => FROSTED_QUARTZ_BASE_BMP,
            (2, true) => FROSTED_QUARTZ_ACTIONS_BMP,
            (_, false) => CRYSTAL_BLUE_GLASS_BASE_BMP,
            (_, true) => CRYSTAL_BLUE_GLASS_ACTIONS_BMP,
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

    #[cfg(all(
        not(feature = "installer"),
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))]
    // ------------------------=
    // FUNC: launcher_icon
    // DESC: Renders a launcher role from its compact 256px-per-cell installed-theme atlas.
    // ------------------=
    pub(super) fn launcher_icon(
        &mut self,
        center_x: usize,
        center_y: usize,
        role: usize,
        size: usize,
    ) -> bool {
        let roles = [0usize, 1, 4, 8, 9, 10, 12, 19, 23, 25, 26, 28, 32, 49];
        let Some(cell) = roles.iter().position(|candidate| *candidate == role) else {
            return self.themed_icon(center_x, center_y, role, size);
        };
        let bitmap = match self.active_icon_theme() {
            1 => LUMINOUS_OBSIDIAN_LAUNCHER_BMP,
            2 => FROSTED_QUARTZ_LAUNCHER_BMP,
            _ => CRYSTAL_BLUE_GLASS_LAUNCHER_BMP,
        };
        self.paint_bitmap_alpha_atlas_cell(
            bitmap,
            4,
            4,
            cell,
            center_x.saturating_sub(size / 2),
            center_y.saturating_sub(size / 2),
            size,
        )
    }

    #[cfg(all(
        not(feature = "installer"),
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))]
    // ------------------------=
    // FUNC: icon_theme_preview
    // DESC: Renders the same semantic sample from a requested family for the Themes and Skins chooser.
    // ------------------=
    pub(super) fn icon_theme_preview(
        &mut self,
        theme: u8,
        center_x: usize,
        center_y: usize,
        size: usize,
    ) -> bool {
        let bitmap = match theme {
            1 => LUMINOUS_OBSIDIAN_BASE_BMP,
            2 => FROSTED_QUARTZ_BASE_BMP,
            _ => CRYSTAL_BLUE_GLASS_BASE_BMP,
        };
        self.paint_bitmap_alpha_atlas_cell(
            bitmap,
            5,
            9,
            0,
            center_x.saturating_sub(size / 2),
            center_y.saturating_sub(size / 2),
            size,
        )
    }

    #[cfg(any(feature = "installer", target_arch = "x86"))]
    // ------------------------=
    // FUNC: themed_icon
    // DESC: Leaves semantic icon rendering to vector fallbacks on the legacy text architecture.
    // ------------------=
    pub(super) fn themed_icon(
        &mut self,
        _center_x: usize,
        _center_y: usize,
        _role: usize,
        _size: usize,
    ) -> bool {
        false
    }

    #[cfg(any(feature = "installer", target_arch = "x86"))]
    // ------------------------=
    // FUNC: launcher_icon
    // DESC: Uses the existing vector fallback where installed 256px launcher atlases are unavailable.
    // ------------------=
    pub(super) fn launcher_icon(
        &mut self,
        center_x: usize,
        center_y: usize,
        role: usize,
        size: usize,
    ) -> bool {
        self.themed_icon(center_x, center_y, role, size)
    }

    #[cfg(any(feature = "installer", target_arch = "x86"))]
    // ------------------------=
    // FUNC: icon_theme_preview
    // DESC: Omits raster theme previews on the legacy text architecture.
    // ------------------=
    pub(super) fn icon_theme_preview(
        &mut self,
        _theme: u8,
        _center_x: usize,
        _center_y: usize,
        _size: usize,
    ) -> bool {
        false
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_first_boot_background
    // DESC: Paints the dedicated premium onboarding environment with left-side card contrast.
    // ------------------=
    pub(super) fn paint_first_boot_background(&mut self) {
        self.paint_bitmap_cover_rect(ONBOARDING_BMP, 0, 0, self.width, self.height);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_desktop_background
    // DESC: Paints the high-resolution authenticated-session wallpaper without baked interface elements.
    // ------------------=
    pub(super) fn paint_desktop_background(&mut self) {
        match self.skin_visual_mode() {
            1 => self.fill_rect(0, 0, self.width, self.height, 232, 237, 243),
            2 => self.fill_rect(0, 0, self.width, self.height, 0, 0, 0),
            _ => self.paint_bitmap_cover_rect(DESKTOP_BMP, 0, 0, self.width, self.height),
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_authentication_background
    // DESC: Paints the dedicated gold-standard sign-in scene independently from the desktop wallpaper.
    // ------------------=
    pub(super) fn paint_authentication_background(&mut self) {
        match self.skin_visual_mode() {
            1 => self.fill_rect(0, 0, self.width, self.height, 232, 237, 243),
            2 => self.fill_rect(0, 0, self.width, self.height, 0, 0, 0),
            _ => self.paint_bitmap_cover_rect(AUTHENTICATION_BMP, 0, 0, self.width, self.height),
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_authentication_background_rect
    // DESC: Restores a bounded authentication-wallpaper region beneath animated login particles.
    // ------------------=
    pub(super) fn paint_authentication_background_rect(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        match self.skin_visual_mode() {
            1 => self.fill_rect(left, top, width, height, 232, 237, 243),
            2 => self.fill_rect(left, top, width, height, 0, 0, 0),
            _ => self.paint_bitmap_cover_rect(AUTHENTICATION_BMP, left, top, width, height),
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_desktop_background_rect
    // DESC: Restores a bounded desktop region for tear-free animated login effects.
    // ------------------=
    pub(super) fn paint_desktop_background_rect(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        match self.skin_visual_mode() {
            1 => self.fill_rect(left, top, width, height, 232, 237, 243),
            2 => self.fill_rect(left, top, width, height, 0, 0, 0),
            _ => self.paint_bitmap_cover_rect(DESKTOP_BMP, left, top, width, height),
        }
    }

    // ------------------------=
    // FUNC: skin_visual_mode
    // DESC: Maps the active transactional skin identity to the bootstrap renderer's migration adapter.
    // ------------------=
    pub(super) fn skin_visual_mode(&self) -> u8 {
        crate::runtime::with_runtime(|runtime| {
            let id = runtime.ui.skins.active().id;
            if id == crate::ui::skin::SkinId::from_bytes(b"infinity.diagnostic.light") {
                1
            } else if id == crate::ui::skin::SkinId::from_bytes(b"infinity.safe") {
                2
            } else {
                0
            }
        })
        .unwrap_or(0)
    }

    #[cfg(target_arch = "x86")]
    // ------------------------=
    // FUNC: paint_desktop_background
    // DESC: Provides a dark legacy fallback when the high-resolution desktop surface is unavailable.
    // ------------------=
    pub(super) fn paint_desktop_background(&mut self) {
        self.fill_rect(0, 0, self.width, self.height, 1, 7, 15);
    }

    #[cfg(target_arch = "x86")]
    // ------------------------=
    // FUNC: paint_authentication_background
    // DESC: Provides the legacy text-compatible sign-in canvas when high-resolution artwork is unavailable.
    // ------------------=
    pub(super) fn paint_authentication_background(&mut self) {
        self.fill_rect(0, 0, self.width, self.height, 1, 7, 15);
    }

    #[cfg(target_arch = "x86")]
    // ------------------------=
    // FUNC: paint_authentication_background_rect
    // DESC: Restores a bounded legacy authentication region beneath animated particles.
    // ------------------=
    pub(super) fn paint_authentication_background_rect(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        self.fill_rect(left, top, width, height, 1, 7, 15);
    }

    #[cfg(target_arch = "x86")]
    // ------------------------=
    // FUNC: paint_desktop_background_rect
    // DESC: Restores a bounded legacy desktop region without bitmap resources.
    // ------------------=
    pub(super) fn paint_desktop_background_rect(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        self.fill_rect(left, top, width, height, 1, 7, 15);
    }

    #[cfg(target_arch = "x86")]
    // ------------------------=
    // FUNC: paint_first_boot_background
    // DESC: Provides the legacy x86 text-compatible fallback for first-boot surfaces.
    // ------------------=
    pub(super) fn paint_first_boot_background(&mut self) {
        self.fill_rect(0, 0, self.width, self.height, 1, 7, 15);
    }

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
    // FUNC: top_bar_infinity_icon
    // DESC: Draws the compact generated illuminated InfinityOS mark used by the desktop top bar.
    // ------------------=
    pub(super) fn top_bar_infinity_icon(&mut self, center_x: usize, center_y: usize, width: usize) {
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        {
            let height = width * 7 / 15;
            self.paint_bitmap_alpha_fit_rect(
                TOP_BAR_INFINITY_BMP,
                center_x.saturating_sub(width / 2),
                center_y.saturating_sub(height / 2),
                width,
                height,
            );
        }
        #[cfg(target_arch = "x86")]
        self.small_infinity_mark(center_x, center_y, width);
    }
}

impl super::DisplayDevice {
    // ------------------------=
    // FUNC: authentication_frame
    // DESC: Renders the default skin authentication scene to the supplied gold-standard geometry and copy.
    // ------------------=
    pub(super) fn authentication_frame(
        &mut self,
        locked: bool,
        selected_user: usize,
        input: &[u8],
        focus: usize,
        paint_header: bool,
    ) {
        let fit = (self.width.saturating_mul(1000) / 1536)
            .min(self.height.saturating_mul(1000) / 1024)
            .max(1);
        let content_width = 1536usize.saturating_mul(fit) / 1000;
        let content_height = 1024usize.saturating_mul(fit) / 1000;
        let offset_x = self.width.saturating_sub(content_width) / 2;
        let offset_y = self.height.saturating_sub(content_height) / 2;
        let sx = |value: usize| offset_x + value.saturating_mul(fit) / 1000;
        let sy = |value: usize| offset_y + value.saturating_mul(fit) / 1000;
        let sw = |value: usize| value.saturating_mul(fit) / 1000;

        if paint_header {
            let top_height = sw(64).max(38);
            self.fill_rect_alpha(0, 0, self.width, top_height, 1, 6, 13, 238);
            self.outline_rect(0, top_height.saturating_sub(1), self.width, 1, 15, 31, 48);
            self.small_infinity_mark(sx(57), top_height / 2, sw(48));
            self.ui_text(
                sx(92),
                top_height / 2 - UI_FONT_CELL_HEIGHT / 2,
                b"WELCOME TO I N F I N I T Y O S",
                239,
                244,
                249,
                1,
            );
            self.ui_text(
                sx(1302),
                top_height / 2 - UI_FONT_CELL_HEIGHT / 2,
                b"MACHINE NAME",
                229,
                235,
                241,
                1,
            );
            self.authentication_icon(sx(1497), top_height / 2, 1, sw(21), false);
        }

        let card_x = sx(54);
        let card_y = sy(123);
        let card_w = sw(521);
        let card_h = sw(754);
        self.glass_panel(card_x, card_y, card_w, card_h, true);
        self.small_infinity_mark(card_x + card_w / 2, card_y + sw(91), sw(122));
        self.ui_text_centered(
            card_x,
            card_w,
            card_y + sw(148),
            if locked {
                b"Welcome Back"
            } else {
                b"Welcome to InfinityOS"
            },
            244,
            247,
            251,
            2,
        );
        self.ui_text_centered(
            card_x,
            card_w,
            card_y + sw(198),
            b"Secure. Limitless.",
            188,
            198,
            211,
            1,
        );

        let inner_x = card_x + sw(49);
        let inner_w = card_w.saturating_sub(sw(98));
        let user_y = card_y + sw(253);
        self.fill_rounded_rect_alpha(
            inner_x,
            user_y,
            inner_w,
            sw(86),
            sw(13),
            2,
            10,
            20,
            if focus == 0 { 248 } else { 224 },
        );
        self.outline_rounded_rect(
            inner_x,
            user_y,
            inner_w,
            sw(86),
            sw(13),
            if focus == 0 { 37 } else { 44 },
            if focus == 0 { 183 } else { 66 },
            if focus == 0 { 234 } else { 89 },
        );
        self.authentication_icon(inner_x + sw(45), user_y + sw(43), 2, sw(50), focus == 0);
        let user = crate::runtime::with_runtime(|runtime| runtime.identity.user_nth(selected_user))
            .flatten();
        if let Some(value) = user {
            self.ui_text(
                inner_x + sw(91),
                user_y + sw(25),
                value.display_name.as_bytes(),
                237,
                242,
                248,
                1,
            );
        } else {
            self.ui_text(
                inner_x + sw(91),
                user_y + sw(25),
                b"Aurelius",
                237,
                242,
                248,
                1,
            );
        }
        self.ui_text(
            inner_x + sw(91),
            user_y + sw(53),
            b"Last signed in 2 minutes ago",
            151,
            162,
            177,
            1,
        );
        self.authentication_icon(
            inner_x + inner_w - sw(28),
            user_y + sw(43),
            3,
            sw(15),
            focus == 0,
        );

        let password_y = card_y + sw(356);
        self.authentication_password_field(inner_x, password_y, inner_w, sw(57), input, focus == 1);

        let sign_y = card_y + sw(439);
        self.fill_rounded_rect_alpha(
            inner_x,
            sign_y,
            inner_w,
            sw(crate::ui::system_layout::UI_HERO_ACTION_HEIGHT),
            sw(13),
            7,
            48,
            79,
            if focus == 2 { 248 } else { 226 },
        );
        self.outline_rounded_rect(
            inner_x,
            sign_y,
            inner_w,
            sw(crate::ui::system_layout::UI_HERO_ACTION_HEIGHT),
            sw(13),
            34,
            182,
            235,
        );
        self.ui_text_centered(
            inner_x,
            inner_w,
            sign_y + sw(18),
            if locked { b"Unlock" } else { b"Sign In" },
            247,
            250,
            253,
            1,
        );
        self.authentication_icon(
            inner_x + inner_w - sw(30),
            sign_y + sw(28),
            4,
            sw(18),
            focus == 2,
        );

        let separator_y = card_y + sw(527);
        self.fill_rect(inner_x, separator_y, inner_w, 1, 18, 31, 45);
        self.fill_rect_alpha(
            inner_x + inner_w / 2 - sw(23),
            separator_y - sw(10),
            sw(46),
            sw(20),
            4,
            12,
            22,
            255,
        );
        self.ui_text_centered(
            inner_x + inner_w / 2 - sw(23),
            sw(46),
            separator_y - sw(8),
            b"or",
            157,
            168,
            183,
            1,
        );

        let key_y = card_y + sw(557);
        self.fill_rounded_rect_alpha(
            inner_x,
            key_y,
            inner_w,
            sw(crate::ui::system_layout::UI_HERO_ACTION_HEIGHT),
            sw(13),
            5,
            16,
            28,
            if focus == 3 { 248 } else { 224 },
        );
        self.outline_rounded_rect(
            inner_x,
            key_y,
            inner_w,
            sw(crate::ui::system_layout::UI_HERO_ACTION_HEIGHT),
            sw(13),
            if focus == 3 { 34 } else { 37 },
            if focus == 3 { 182 } else { 59 },
            if focus == 3 { 235 } else { 79 },
        );
        self.ui_text_centered(
            inner_x,
            inner_w,
            key_y + sw(17),
            b"Sign In with Security Key",
            235,
            240,
            246,
            1,
        );
        self.authentication_icon(
            inner_x + inner_w - sw(31),
            key_y + sw(27),
            5,
            sw(17),
            focus == 3,
        );

        let actions_y = card_y + sw(661);
        let third = inner_w / 3;
        let action_labels: [&[u8]; 3] = [b"Add User", b"Forgot Password?", b"Options"];
        for (index, label) in action_labels.iter().enumerate() {
            let center = inner_x + third * index + third / 2;
            self.authentication_icon(center, actions_y, 6 + index, sw(27), focus == index + 4);
            self.ui_text_centered(
                inner_x + third * index,
                third,
                actions_y + sw(31),
                label,
                if focus == index + 4 { 255 } else { 209 },
                if focus == index + 4 { 255 } else { 217 },
                if focus == index + 4 { 255 } else { 226 },
                1,
            );
        }

        let tray_x = sx(529);
        let tray_y = sy(914);
        let tray_w = sw(478);
        let tray_h = sw(90);
        self.glass_panel(tray_x, tray_y, tray_w, tray_h, false);
        let utility_labels: [&[u8]; 4] = [b"Shut Down", b"Restart", b"Network", b"Accessibility"];
        for (index, label) in utility_labels.iter().enumerate() {
            let cell = tray_w / 4;
            let center = tray_x + cell * index + cell / 2;
            self.authentication_icon(
                center,
                tray_y + sw(31),
                9 + index,
                sw(25),
                focus == index + 7,
            );
            self.ui_text_centered(
                tray_x + cell * index,
                cell,
                tray_y + sw(57),
                label,
                if focus == index + 7 { 255 } else { 221 },
                if focus == index + 7 { 255 } else { 228 },
                if focus == index + 7 { 255 } else { 236 },
                1,
            );
        }
    }

    // ------------------------=
    // FUNC: authentication_focus_controls
    // DESC: Rebuilds only the login card and utility tray after pointer hover changes focus.
    // ------------------=
    pub(super) fn authentication_focus_controls(
        &mut self,
        locked: bool,
        selected_user: usize,
        input: &[u8],
        focus: usize,
    ) {
        let fit = (self.width.saturating_mul(1000) / 1536)
            .min(self.height.saturating_mul(1000) / 1024)
            .max(1);
        let content_width = 1536usize.saturating_mul(fit) / 1000;
        let content_height = 1024usize.saturating_mul(fit) / 1000;
        let offset_x = self.width.saturating_sub(content_width) / 2;
        let offset_y = self.height.saturating_sub(content_height) / 2;
        let sx = |value: usize| offset_x + value.saturating_mul(fit) / 1000;
        let sy = |value: usize| offset_y + value.saturating_mul(fit) / 1000;
        let sw = |value: usize| value.saturating_mul(fit) / 1000;
        let card_x = sx(54);
        let card_y = sy(123);
        let card_w = sw(521);
        let card_h = sw(754);
        let tray_x = sx(529);
        let tray_y = sy(914);
        let tray_w = sw(478);
        let tray_h = sw(90);

        self.paint_authentication_background_rect(card_x, card_y, card_w, card_h);
        self.paint_authentication_background_rect(tray_x, tray_y, tray_w, tray_h);
        self.authentication_frame(locked, selected_user, input, focus, false);
    }

    // ------------------------=
    // FUNC: authentication_password_field
    // DESC: Draws the secure password control without retaining or displaying plaintext pixels.
    // ------------------=
    pub(super) fn authentication_password_field(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        input: &[u8],
        focused: bool,
    ) {
        self.fill_rounded_rect_alpha(x, y, width, height, height / 4, 2, 9, 18, 255);
        self.outline_rounded_rect(
            x,
            y,
            width,
            height,
            height / 4,
            if focused { 34 } else { 37 },
            if focused { 182 } else { 58 },
            if focused { 235 } else { 78 },
        );
        self.authentication_icon(x + height / 2, y + height / 2, 5, height / 3, focused);
        if input.is_empty() {
            self.ui_text(
                x + height,
                y + height / 2 - 8,
                b"Enter your password",
                115,
                126,
                142,
                1,
            );
        } else {
            let mut masked = [0u8; 64];
            let length = input.len().min(masked.len());
            masked[..length].fill(b'*');
            self.ui_text(
                x + height,
                y + height / 2 - 8,
                &masked[..length],
                238,
                244,
                249,
                1,
            );
        }
        self.authentication_icon(
            x + width - height / 2,
            y + height / 2,
            13,
            height / 3,
            focused,
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
        let role = [37usize, 0, 47, 46, 1, 17, 23, 26, 32, 34, 13, 28, 27]
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
    // FUNC: system_top_bar
    // DESC: Draws the shared quiet glass top bar with interactive menus and honest device status.
    // ------------------=
    pub(super) fn system_top_bar(
        &mut self,
        active_menu: Option<usize>,
        clock: crate::storage::DateTimeConfiguration,
    ) -> usize {
        let scale = self.ui_scale().max(1);
        let height = (38 * scale).min(self.height / 14).max(34 * scale);
        let rail_inset = 2 * scale;
        let rail_width = self.width.saturating_sub(rail_inset * 2);
        let rail_height = height.saturating_sub(rail_inset * 2);
        let content_y = height / 2;
        let text_y = content_y.saturating_sub(UI_FONT_CELL_HEIGHT / 2);
        let (top_r, top_g, top_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::TopBar);
        let (outline_r, outline_g, outline_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
        let (selection_r, selection_g, selection_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Selection);
        let backdrop_alpha = self.active_background_alpha(176);
        let rail_alpha = self.active_background_alpha(238);
        let highlight_alpha = self.active_background_alpha(88);
        self.fill_rect_alpha(0, 0, self.width, height, 0, 2, 7, backdrop_alpha);
        self.fill_rounded_rect_alpha(
            rail_inset,
            rail_inset,
            rail_width,
            rail_height,
            7 * scale,
            top_r,
            top_g,
            top_b,
            rail_alpha,
        );
        self.fill_rounded_rect_alpha(
            rail_inset,
            rail_inset,
            rail_width,
            rail_height / 2,
            7 * scale,
            18,
            39,
            59,
            highlight_alpha,
        );
        self.fill_rounded_rect_alpha(
            self.width / 3,
            rail_inset,
            self.width / 2,
            rail_height,
            7 * scale,
            selection_r,
            selection_g,
            selection_b,
            42,
        );
        self.outline_rounded_rect(
            rail_inset,
            rail_inset,
            rail_width,
            rail_height,
            7 * scale,
            outline_r,
            outline_g,
            outline_b,
        );
        self.fill_rect_alpha(
            9 * scale,
            height.saturating_sub(2 * scale),
            self.width.saturating_sub(18 * scale),
            1,
            outline_r,
            outline_g,
            outline_b,
            150,
        );

        let brand_width = (280 * scale).min(self.width / 3);
        if active_menu == Some(0) {
            self.fill_rounded_rect_alpha(
                8 * scale,
                4 * scale,
                brand_width,
                height.saturating_sub(8 * scale),
                7 * scale,
                selection_r,
                selection_g,
                selection_b,
                188,
            );
        }
        self.top_bar_infinity_icon(36 * scale, content_y, 68 * scale);
        self.ui_text(76 * scale, text_y, b"I N F I N I T Y O S", 221, 229, 239, 1);

        let help_x = 610 * scale;
        if active_menu == Some(5) {
            let active_width = self.ui_text_width(b"Help", 1) + 18 * scale;
            self.fill_rounded_rect_alpha(
                help_x.saturating_sub(9 * scale),
                4 * scale,
                active_width,
                height.saturating_sub(8 * scale),
                7 * scale,
                selection_r,
                selection_g,
                selection_b,
                210,
            );
        }
        self.ui_text(help_x, text_y, b"Help", 213, 222, 231, 1);

        let icon_size = 18 * scale;
        let status_y = content_y;
        let status_width = 32 * scale;
        let clock_width = 104 * scale;
        let status_left = self
            .width
            .saturating_sub(7 * status_width + clock_width + 10 * scale);
        let private_width = self.ui_text_width(b"LOCAL  |  PRIVATE", 1);
        self.ui_text(
            status_left.saturating_sub(private_width + 18 * scale),
            text_y,
            b"LOCAL  |  PRIVATE",
            211,
            221,
            231,
            1,
        );
        for (index, kind) in [0usize, 1, 2, 3, 13, 4, 5].iter().enumerate() {
            let center = status_left + index * status_width + status_width / 2;
            self.system_status_icon(center, status_y, *kind, icon_size);
        }
        self.paint_system_top_bar_clock_well(clock, height, scale);
        height
    }

    // ------------------------=
    // FUNC: paint_system_top_bar_clock_well
    // DESC: Draws the live clock as an inset segment of the continuous top-bar glass rail.
    // ------------------=
    pub(super) fn paint_system_top_bar_clock_well(
        &mut self,
        clock: crate::storage::DateTimeConfiguration,
        height: usize,
        scale: usize,
    ) {
        let segment_width = (104 * scale).min(self.width);
        let segment_left = self.width.saturating_sub(segment_width + 4 * scale);
        let separator_left = segment_left.saturating_sub(3 * scale);
        let separator_top = 8 * scale;
        let separator_height = height.saturating_sub(16 * scale);
        let (outline_r, outline_g, outline_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
        self.fill_rect_alpha(
            separator_left,
            separator_top,
            scale,
            separator_height,
            outline_r,
            outline_g,
            outline_b,
            104,
        );
        self.fill_rect_alpha(
            separator_left + scale,
            separator_top,
            scale,
            separator_height,
            0,
            3,
            8,
            92,
        );

        let mut time = *b"00:00:00";
        time[0] = b'0' + clock.hour / 10;
        time[1] = b'0' + clock.hour % 10;
        time[3] = b'0' + clock.minute / 10;
        time[4] = b'0' + clock.minute % 10;
        time[6] = b'0' + clock.second / 10;
        time[7] = b'0' + clock.second % 10;
        let time_width = self.ui_text_width(&time, 1);
        self.ui_text_strong(
            segment_left + segment_width.saturating_sub(time_width) / 2,
            height / 2 - UI_FONT_CELL_HEIGHT / 2,
            &time,
            226,
            237,
            246,
            1,
        );
    }

    // ------------------------=
    // FUNC: system_top_bar_clock
    // DESC: Restores and repaints only the desktop clock segment when wall time advances.
    // ------------------=
    pub(super) fn system_top_bar_clock(&mut self, clock: crate::storage::DateTimeConfiguration) {
        let scale = self.ui_scale().max(1);
        let height = (38 * scale).min(self.height / 14).max(34 * scale);
        let repaint_width = (116 * scale).min(self.width);
        let repaint_left = self.width.saturating_sub(repaint_width);
        let repaint_top = 3 * scale;
        let repaint_height = height.saturating_sub(6 * scale);
        let repaint_right = self.width.saturating_sub(4 * scale);
        let interior_width = repaint_right.saturating_sub(repaint_left);
        let (top_r, top_g, top_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::TopBar);
        self.paint_desktop_background_rect(
            repaint_left,
            repaint_top,
            interior_width,
            repaint_height,
        );
        self.fill_rect_alpha(
            repaint_left,
            repaint_top,
            interior_width,
            repaint_height,
            top_r,
            top_g,
            top_b,
            238,
        );
        self.fill_rect_alpha(
            repaint_left,
            repaint_top,
            interior_width,
            height / 2 - repaint_top,
            18,
            39,
            59,
            88,
        );
        self.paint_system_top_bar_clock_well(clock, height, scale);
    }

    // ------------------------=
    // FUNC: system_identity_bar
    // DESC: Draws the minimal trusted first-boot bar without presenting desktop commands before a session exists.
    // ------------------=
    pub(super) fn system_identity_bar(&mut self) -> usize {
        let scale = self.ui_scale().max(1);
        let height = (46 * scale).min(self.height / 12).max(40);
        self.fill_rect_alpha(0, 0, self.width, height, 0, 4, 10, 232);
        self.fill_rect_alpha(0, 0, self.width, height / 2, 14, 26, 39, 54);
        self.fill_rect_alpha(0, height.saturating_sub(1), self.width, 1, 50, 70, 87, 170);
        self.small_infinity_mark(35 * scale, height / 2, 48 * scale);
        self.ui_text_strong(
            64 * scale,
            height / 2 - 10 * scale,
            b"INFINITYOS",
            241,
            246,
            250,
            1,
        );
        let status = b"LOCAL  |  PRIVATE";
        let status_width = self.ui_text_width(status, 1);
        let wifi_center = self.width.saturating_sub(29 * scale);
        self.ui_text(
            wifi_center.saturating_sub(status_width + 36 * scale),
            height / 2 - 10 * scale,
            status,
            211,
            221,
            231,
            1,
        );
        self.system_status_icon(wifi_center, height / 2, 1, 18 * scale);
        height
    }

    // ------------------------=
    // FUNC: system_menu_panel
    // DESC: Draws a native vector-backed desktop menu for the active top-bar domain.
    // ------------------=
    pub(super) fn system_menu_panel(&mut self, menu_kind: usize, focus: usize, scale: usize) {
        let (anchor, width, items): (usize, usize, &[&[u8]]) = match menu_kind {
            1 => (
                298,
                248,
                &[
                    b"Open Console",
                    b"Open Personal Space",
                    b"New Project...",
                    b"System Settings...",
                    b"Close Home Window",
                ],
            ),
            2 => (
                358,
                230,
                &[b"Undo", b"Redo", b"Cut", b"Copy", b"Paste", b"Select All"],
            ),
            3 => (
                418,
                238,
                &[
                    b"Show Home Window",
                    b"Center Home Window",
                    b"Icon View",
                    b"Refresh",
                    b"Appearance...",
                ],
            ),
            4 => (
                480,
                242,
                &[
                    b"Minimize Home",
                    b"Restore Home",
                    b"Center Window",
                    b"System Settings...",
                ],
            ),
            5 => (
                608,
                252,
                &[
                    b"InfinityOS Help",
                    b"Keyboard Shortcuts",
                    b"System Status",
                    b"About InfinityOS",
                ],
            ),
            _ => (
                16,
                268,
                &[
                    b"About InfinityOS",
                    b"System Settings...",
                    b"Users & Accounts...",
                    b"AI & Voice...",
                    b"Network...",
                    b"Privacy & Security...",
                    b"Lock Screen",
                    b"Log Out...",
                    b"Restart...",
                    b"Shut Down...",
                ],
            ),
        };
        let menu_x = (anchor * scale).min(self.width.saturating_sub(width * scale + 8));
        let menu_y = (38 * scale).min(self.height / 14).max(34 * scale) + 6 * scale;
        let menu_w = (width * scale).min(self.width.saturating_sub(menu_x + 8));
        let menu_h = (22 + items.len() * 34) * scale;
        let (selection_r, selection_g, selection_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Selection);
        self.glass_panel(menu_x, menu_y, menu_w, menu_h, true);
        self.fill_rounded_rect_alpha(
            menu_x + 2 * scale,
            menu_y + 2 * scale,
            menu_w.saturating_sub(4 * scale),
            20 * scale,
            10 * scale,
            18,
            35,
            50,
            118,
        );
        for (index, item) in items.iter().enumerate() {
            let row_y = menu_y + (11 + index * 34) * scale;
            if focus == index {
                self.fill_rounded_rect_alpha(
                    menu_x + 7 * scale,
                    row_y,
                    menu_w.saturating_sub(14 * scale),
                    30 * scale,
                    7 * scale,
                    selection_r,
                    selection_g,
                    selection_b,
                    224,
                );
            }
            self.authentication_icon(
                menu_x + 23 * scale,
                row_y + 15 * scale,
                (index + menu_kind * 2) % 13 + 1,
                15 * scale,
                focus == index,
            );
            self.ui_text(
                menu_x + 43 * scale,
                row_y + 5 * scale,
                item,
                if focus == index { 249 } else { 218 },
                if focus == index { 252 } else { 227 },
                if focus == index { 255 } else { 236 },
                1,
            );
        }
    }

    // ------------------------=
    // FUNC: system_status_icon
    // DESC: Draws the coherent small-size volume, radio, power, search, and menu vector family for the top bar.
    // ------------------=
    pub(super) fn system_status_icon(
        &mut self,
        center_x: usize,
        center_y: usize,
        kind: usize,
        size: usize,
    ) {
        let role = match kind {
            0 => 40,
            1 => 37,
            2 => 38,
            3 => 39,
            4 => 27,
            _ => 26,
        };
        if self.themed_icon(center_x, center_y, role, size.max(16)) {
            return;
        }
        let cx = center_x as i32;
        let cy = center_y as i32;
        let half = size.max(10) as i32 / 2;
        let color = (230, 238, 245);
        match kind {
            0 => {
                self.icon_line(
                    cx - half,
                    cy - half / 3,
                    cx - half / 2,
                    cy - half / 3,
                    color,
                    size,
                );
                self.icon_line(
                    cx - half / 2,
                    cy - half / 3,
                    cx,
                    cy - half * 3 / 4,
                    color,
                    size,
                );
                self.icon_line(cx, cy - half * 3 / 4, cx, cy + half * 3 / 4, color, size);
                self.icon_line(
                    cx,
                    cy + half * 3 / 4,
                    cx - half / 2,
                    cy + half / 3,
                    color,
                    size,
                );
                self.icon_line(
                    cx - half / 2,
                    cy + half / 3,
                    cx - half,
                    cy + half / 3,
                    color,
                    size,
                );
                self.icon_line(
                    cx + half / 3,
                    cy - half / 2,
                    cx + half * 2 / 3,
                    cy,
                    color,
                    size,
                );
                self.icon_line(
                    cx + half * 2 / 3,
                    cy,
                    cx + half / 3,
                    cy + half / 2,
                    color,
                    size,
                );
            }
            1 => self.authentication_icon(center_x, center_y, 1, size, false),
            2 => {
                self.icon_line(cx, cy - half, cx, cy + half, color, size);
                self.icon_line(cx, cy - half, cx + half * 2 / 3, cy - half / 3, color, size);
                self.icon_line(
                    cx + half * 2 / 3,
                    cy - half / 3,
                    cx - half / 2,
                    cy + half * 2 / 3,
                    color,
                    size,
                );
                self.icon_line(
                    cx - half / 2,
                    cy - half * 2 / 3,
                    cx + half * 2 / 3,
                    cy + half / 3,
                    color,
                    size,
                );
                self.icon_line(cx + half * 2 / 3, cy + half / 3, cx, cy + half, color, size);
            }
            3 => {
                let left = center_x.saturating_sub(size / 2);
                let top = center_y.saturating_sub(size * 3 / 10);
                self.outline_rounded_rect(
                    left,
                    top,
                    size.saturating_sub(3),
                    size * 3 / 5,
                    2,
                    color.0,
                    color.1,
                    color.2,
                );
                self.fill_rect_alpha(
                    left + size.saturating_sub(2),
                    top + size / 5,
                    3,
                    size / 5,
                    210,
                    224,
                    235,
                    255,
                );
                self.fill_rounded_rect_alpha(
                    left + 3,
                    top + 3,
                    size * 3 / 5,
                    size * 3 / 5 - 6,
                    1,
                    216,
                    231,
                    241,
                    255,
                );
            }
            4 => {
                self.icon_circle(cx - half / 5, cy - half / 5, half * 3 / 5, color, size);
                self.icon_line(
                    cx + half / 4,
                    cy + half / 4,
                    cx + half,
                    cy + half,
                    color,
                    size,
                );
            }
            _ => {
                for offset in [-half / 2, 0, half / 2] {
                    self.icon_line(cx - half, cy + offset, cx + half, cy + offset, color, size);
                }
            }
        }
    }

    // ------------------------=
    // FUNC: onboarding_step_indicator
    // DESC: Renders seven compact progress segments with completed, current, and remaining states.
    // ------------------=
    pub(super) fn onboarding_step_indicator(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        step: usize,
    ) {
        let visual_step = match step {
            0 => 0,
            1 => 1,
            2 | 3 => 2,
            4 => 3,
            5 => 4,
            6 => 5,
            _ => 6,
        };
        let gap = 8usize;
        let segment_width = width.saturating_sub(gap * 6) / 7;
        for index in 0..7usize {
            let x = left + index * (segment_width + gap);
            let (red, green, blue, alpha) = if index < visual_step {
                (63, 197, 239, 235)
            } else if index == visual_step {
                (159, 230, 255, 255)
            } else {
                (61, 80, 96, 180)
            };
            self.fill_rounded_rect_alpha(x, top, segment_width, 4, 2, red, green, blue, alpha);
        }
    }

    // ------------------------=
    // FUNC: polished_button
    // DESC: Draws a shared primary or secondary rounded action with visible focus hierarchy.
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
        let radius = (height / 4).clamp(8, 14);
        if primary {
            // The base is intentionally opaque. Onboarding hover updates repaint
            // only this bounded control, so alpha accumulation must not alter the
            // button each time the pointer crosses its hit target.
            self.fill_rounded_rect_alpha(left, top, width, height, radius, 8, 54, 84, 255);
            self.fill_rounded_rect_alpha(
                left + 2,
                top + 2,
                width.saturating_sub(4),
                height / 2,
                radius.saturating_sub(2),
                25,
                111,
                159,
                if focused { 116 } else { 72 },
            );
            self.outline_rounded_rect(
                left,
                top,
                width,
                height,
                radius,
                if focused { 151 } else { 40 },
                if focused { 229 } else { 181 },
                if focused { 255 } else { 231 },
            );
        } else {
            self.fill_rounded_rect_alpha(left, top, width, height, radius, 5, 15, 27, 255);
            self.outline_rounded_rect(
                left,
                top,
                width,
                height,
                radius,
                if focused { 105 } else { 42 },
                if focused { 197 } else { 69 },
                if focused { 236 } else { 91 },
            );
        }
        self.ui_text_centered_strong(left, width, top + height / 2 - 10, label, 242, 248, 252, 1);
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

    // ------------------------=
    // FUNC: polished_toolbar_button
    // DESC: Draws one equal-size icon-and-label toolbar action with the shared compact gutters.
    // ------------------=
    pub(super) fn polished_toolbar_button(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        label: &[u8],
        role: usize,
    ) {
        let scale = self.ui_scale().max(1);
        let radius = (8 * scale).min(height / 2);
        self.fill_rounded_rect_alpha(left, top, width, height, radius, 7, 27, 45, 218);
        self.outline_rounded_rect(left, top, width, height, radius, 35, 76, 102);
        let icon_center_x = left + 16 * scale;
        let icon_center_y = top + height / 2;
        let _ = self.themed_icon(icon_center_x, icon_center_y, role, 20 * scale);
        self.ui_text(
            left + 32 * scale,
            top + height.saturating_sub(UI_FONT_CELL_HEIGHT) / 2,
            label,
            213,
            232,
            243,
            1,
        );
    }

    // ------------------------=
    // FUNC: onboarding_input_field
    // DESC: Draws one polished first-boot text field with placeholder, focus, and secure masking.
    // ------------------=
    pub(super) fn onboarding_input_field(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        input: &[u8],
        masked: bool,
        focused: bool,
        placeholder: &[u8],
    ) {
        let radius = (height / 4).clamp(8, 14);
        // Focus changes can repaint this field independently of the glass card.
        // Use an opaque base so repeated bounded updates remain color-stable.
        self.fill_rounded_rect_alpha(left, top, width, height, radius, 2, 10, 20, 255);
        self.outline_rounded_rect(
            left,
            top,
            width,
            height,
            radius,
            if focused { 67 } else { 40 },
            if focused { 192 } else { 72 },
            if focused { 241 } else { 95 },
        );
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
            (119, 133, 149)
        } else {
            (239, 245, 250)
        };
        self.ui_text(
            left + crate::ui::system_layout::UI_GUTTER * self.ui_scale().max(1),
            top + height / 2 - 10,
            display,
            color.0,
            color.1,
            color.2,
            1,
        );
    }

    // ------------------------=
    // FUNC: onboarding_frame
    // DESC: Renders the premium progressive first-boot card from shared controls and semantic copy.
    // ------------------=
    pub(super) fn onboarding_frame(
        &mut self,
        step: usize,
        input: &[u8],
        masked: bool,
        focus: usize,
        validation_error: bool,
    ) {
        let scale = self.ui_scale().max(1);
        let top_bar = self.system_identity_bar();
        let card_width = (self.width * 34 / 100).clamp(500, 600 * scale);
        let card_height = (self.height * 68 / 100)
            .clamp(560, 680 * scale)
            .min(self.height.saturating_sub(top_bar + 24));
        let card_left = self.width * 4 / 100;
        let card_top = top_bar + self.height.saturating_sub(top_bar + card_height) / 2;
        self.onboarding_glass_panel(card_left, card_top, card_width, card_height);

        let inner_left = card_left + 32 * scale;
        let inner_width = card_width.saturating_sub(64 * scale);
        self.small_infinity_mark(inner_left + 25 * scale, card_top + 34 * scale, 45 * scale);
        self.ui_text_strong(
            inner_left + 55 * scale,
            card_top + 24 * scale,
            b"INFINITYOS",
            237,
            244,
            249,
            1,
        );
        self.ui_text(
            card_left + card_width.saturating_sub(102 * scale),
            card_top + 24 * scale,
            b"LOCAL  |  PRIVATE",
            135,
            155,
            174,
            1,
        );
        self.onboarding_step_indicator(inner_left, card_top + 66 * scale, inner_width, step);

        let (eyebrow, title, description, placeholder): (&[u8], &[u8], &[u8], &[u8]) = match step {
            0 => (b"WELCOME", b"Welcome to InfinityOS", b"A private system shaped around you.", b""),
            1 => (b"MACHINE", b"Name your Infinity Node", b"Choose a friendly name for this device. You can change it later.", b"InfinityNode"),
            2 => (b"PROFILE", b"Choose your handle", b"Your handle identifies your Personal Space without exposing your full name.", b"your-handle"),
            3 => (b"PROFILE", b"How should we address you?", b"Use the name you want InfinityOS to show across your local experience.", b"Display name"),
            4 => (b"SECURITY", b"Secure your account", b"Use at least eight characters. Your password remains local to this system.", b"Create a password"),
            5 => (b"AI, VOICE & APPEARANCE", b"Private by default", b"Local AI is ready. Remote processing and microphone access begin disabled.", b""),
            6 => (b"NETWORK", b"Connect this Infinity Node", b"Choose wired, Wi-Fi, or continue offline. You can change this later.", b""),
            _ => (b"READY", b"Your Infinity begins here", b"Your identity, Personal Space, privacy policy, and Default Dark appearance are ready.", b""),
        };
        let content_top = card_top + 94 * scale;
        self.ui_text_strong(inner_left, content_top, eyebrow, 72, 196, 238, 1);
        self.ui_text_fit_strong(
            inner_left,
            content_top + 35 * scale,
            inner_width,
            title,
            245,
            248,
            251,
            if self.width >= 1500 { 2 } else { 1 },
        );
        self.ui_text_wrapped(
            inner_left,
            content_top + 82 * scale,
            inner_width,
            description,
            178,
            190,
            204,
            2,
        );

        let body_top = content_top + 132 * scale;
        if step == 0 {
            let items: [(&[u8], &[u8], usize); 3] = [
                (
                    b"Yours from the start",
                    b"Identity and Personal Space are built in.",
                    2,
                ),
                (
                    b"Private by design",
                    b"Explicit capability controls stay local.",
                    8,
                ),
                (
                    b"Ready to grow",
                    b"Objects, apps, and AI share one system.",
                    7,
                ),
            ];
            for (index, (heading, detail, icon)) in items.iter().enumerate() {
                let row_top = body_top + index * 68 * scale;
                self.fill_rounded_rect_alpha(
                    inner_left,
                    row_top,
                    inner_width,
                    54 * scale,
                    12 * scale,
                    8,
                    23,
                    38,
                    208,
                );
                self.authentication_icon(
                    inner_left + 24 * scale,
                    row_top + 27 * scale,
                    *icon,
                    24 * scale,
                    true,
                );
                self.ui_text_strong(
                    inner_left + 50 * scale,
                    row_top + 8 * scale,
                    heading,
                    227,
                    239,
                    247,
                    1,
                );
                self.ui_text(
                    inner_left + 50 * scale,
                    row_top + 29 * scale,
                    detail,
                    145,
                    162,
                    178,
                    1,
                );
            }
        } else if (1..=4).contains(&step) {
            self.ui_text_strong(
                inner_left,
                body_top,
                if step == 4 {
                    b"PASSWORD"
                } else {
                    b"PROFILE DETAIL"
                },
                151,
                168,
                184,
                1,
            );
            self.onboarding_input_field(
                inner_left,
                body_top + 28 * scale,
                inner_width,
                50 * scale,
                input,
                masked,
                focus == 1,
                placeholder,
            );
            let helper = if validation_error {
                if step == 4 {
                    b"Use at least eight characters.".as_slice()
                } else {
                    b"This field is required before continuing.".as_slice()
                }
            } else if step == 4 {
                b"Stored as a salted verifier; plaintext is never persisted.".as_slice()
            } else {
                b"You can revise this value from Settings later.".as_slice()
            };
            self.ui_text(
                inner_left,
                body_top + 88 * scale,
                helper,
                if validation_error { 255 } else { 132 },
                if validation_error { 118 } else { 151 },
                if validation_error { 126 } else { 168 },
                1,
            );
        } else if step == 5 {
            let privacy_rows: [(&[u8], &[u8], usize); 4] = [
                (b"Local AI", b"On", 7usize),
                (b"Remote processing", b"Off", 11usize),
                (b"Voice and microphone", b"Off", 5usize),
                (b"Appearance", b"InfinityOS Default Dark", 8usize),
            ];
            for (index, (label, value, icon)) in privacy_rows.iter().enumerate() {
                let row_top = body_top + index * 54 * scale;
                self.fill_rounded_rect_alpha(
                    inner_left,
                    row_top,
                    inner_width,
                    44 * scale,
                    10 * scale,
                    6,
                    19,
                    32,
                    216,
                );
                self.authentication_icon(
                    inner_left + 22 * scale,
                    row_top + 22 * scale,
                    *icon,
                    20 * scale,
                    false,
                );
                self.ui_text_strong(
                    inner_left + 46 * scale,
                    row_top + 12 * scale,
                    label,
                    220,
                    231,
                    239,
                    1,
                );
                let value_width = self.ui_text_width(value, 1);
                self.ui_text(
                    inner_left + inner_width.saturating_sub(value_width + 16 * scale),
                    row_top + 12 * scale,
                    value,
                    105,
                    204,
                    240,
                    1,
                );
            }
        } else if step == 6 {
            self.onboarding_network_rows(
                inner_left,
                body_top,
                inner_width,
                focus,
                validation_error,
            );
        } else {
            self.fill_rounded_rect_alpha(
                inner_left,
                body_top,
                inner_width,
                150 * scale,
                14 * scale,
                6,
                21,
                35,
                220,
            );
            self.small_infinity_mark(
                inner_left + inner_width / 2,
                body_top + 48 * scale,
                82 * scale,
            );
            self.ui_text_centered_strong(
                inner_left,
                inner_width,
                body_top + 82 * scale,
                b"Everything is ready",
                235,
                243,
                249,
                1,
            );
            self.ui_text_centered(
                inner_left,
                inner_width,
                body_top + 110 * scale,
                b"Enter a secure, local-first InfinityOS session.",
                151,
                166,
                181,
                1,
            );
        }

        self.onboarding_actions(step, focus);
    }

    // ------------------------=
    // FUNC: onboarding_network_rows
    // DESC: Draws real wired, wireless, and offline choices from authoritative Network Runtime state.
    // ------------------=
    pub(super) fn onboarding_network_rows(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        focus: usize,
        validation_error: bool,
    ) {
        use crate::runtime::network::types::{ConnectivityClass, LinkState, NetworkSetupMode};
        let scale = self.ui_scale().max(1);
        let snapshot = crate::runtime::with_runtime(|runtime| runtime.network.setup_snapshot());
        let selected = snapshot
            .map(|value| value.selected)
            .unwrap_or(NetworkSetupMode::Automatic);
        let wired_detail: &[u8] = match snapshot {
            Some(value) if value.wired_available && value.wired_link == LinkState::Up => {
                b"Connected link detected"
            }
            Some(value) if value.wired_available => b"Connect a network cable",
            _ => b"No wired adapter detected",
        };
        let wireless_detail: &[u8] = match snapshot {
            Some(value) if value.wireless_available && value.wireless_link == LinkState::Up => {
                b"Connected wireless link detected"
            }
            Some(value) if value.wireless_available => b"Wireless link is not connected",
            _ => b"No wireless adapter detected",
        };
        let rows: [(&[u8], &[u8], usize, NetworkSetupMode); 3] = [
            (
                b"Wired network",
                wired_detail,
                4usize,
                NetworkSetupMode::Wired,
            ),
            (
                b"Wi-Fi",
                wireless_detail,
                3usize,
                NetworkSetupMode::Wireless,
            ),
            (
                b"Continue offline",
                b"Set up networking later in Settings",
                8usize,
                NetworkSetupMode::Offline,
            ),
        ];
        for (index, (label, detail, icon, mode)) in rows.iter().enumerate() {
            let row_top = top + index * 58 * scale;
            let is_selected = *mode == selected
                || (selected == NetworkSetupMode::Automatic
                    && index
                        == snapshot
                            .map(|value| {
                                if value.wired_available {
                                    0
                                } else if value.wireless_available {
                                    1
                                } else {
                                    2
                                }
                            })
                            .unwrap_or(2));
            let is_focused = focus == index + 2;
            self.fill_rounded_rect_alpha(
                left,
                row_top,
                width,
                48 * scale,
                10 * scale,
                if is_focused { 9 } else { 5 },
                if is_focused { 44 } else { 20 },
                if is_focused { 68 } else { 34 },
                255,
            );
            self.outline_rounded_rect(
                left,
                row_top,
                width,
                48 * scale,
                10 * scale,
                if is_focused || is_selected { 55 } else { 31 },
                if is_focused || is_selected { 194 } else { 74 },
                if is_focused || is_selected { 238 } else { 98 },
            );
            self.authentication_icon(
                left + 22 * scale,
                row_top + 24 * scale,
                *icon,
                20 * scale,
                is_selected,
            );
            self.ui_text_strong(
                left + 46 * scale,
                row_top + 7 * scale,
                label,
                226,
                237,
                245,
                1,
            );
            self.ui_text(
                left + 46 * scale,
                row_top + 27 * scale,
                detail,
                133,
                157,
                177,
                1,
            );
            if is_selected {
                self.ui_text(
                    left + width.saturating_sub(72 * scale),
                    row_top + 16 * scale,
                    if snapshot
                        .map(|value| value.connectivity != ConnectivityClass::Offline)
                        .unwrap_or(false)
                        && index < 2
                    {
                        b"ACTIVE"
                    } else {
                        b"SELECTED"
                    },
                    88,
                    207,
                    244,
                    1,
                );
            }
        }
        if validation_error {
            self.ui_text(
                left,
                top + 178 * scale,
                b"That connection is unavailable. Connect hardware or choose Offline.",
                255,
                118,
                126,
                1,
            );
        }
    }

    // ------------------------=
    // FUNC: onboarding_actions
    // DESC: Repaints only the first-boot navigation controls for flicker-free pointer hover changes.
    // ------------------=
    pub(super) fn onboarding_actions(&mut self, step: usize, focus: usize) {
        let scale = self.ui_scale().max(1);
        let top_bar = (46 * scale).min(self.height / 12).max(40);
        let card_width = (self.width * 34 / 100).clamp(500, 600 * scale);
        let card_height = (self.height * 68 / 100)
            .clamp(560, 680 * scale)
            .min(self.height.saturating_sub(top_bar + 24));
        let card_left = self.width * 4 / 100;
        let card_top = top_bar + self.height.saturating_sub(top_bar + card_height) / 2;
        let inner_left = card_left + 32 * scale;
        let inner_width = card_width.saturating_sub(64 * scale);
        let button_height = crate::ui::system_layout::UI_STANDARD_ACTION_HEIGHT * scale;
        let button_top = card_top + card_height.saturating_sub(72 * scale);
        if step > 0 {
            let back_width = inner_width * 30 / 100;
            self.polished_button(
                inner_left,
                button_top,
                back_width,
                button_height,
                b"Back",
                false,
                focus == 0,
            );
            let primary_left =
                inner_left + back_width + crate::ui::system_layout::UI_CONTROL_GAP * scale;
            self.polished_button(
                primary_left,
                button_top,
                inner_width
                    .saturating_sub(back_width + crate::ui::system_layout::UI_CONTROL_GAP * scale),
                button_height,
                if step >= 7 {
                    b"Enter InfinityOS"
                } else {
                    b"Continue"
                },
                true,
                focus == 1,
            );
        } else {
            self.polished_button(
                inner_left,
                button_top,
                inner_width,
                button_height,
                b"Continue",
                true,
                true,
            );
        }
    }

    // ------------------------=
    // FUNC: onboarding_focus_controls
    // DESC: Updates the bounded first-boot field and actions when pointer hover changes focus.
    // ------------------=
    pub(super) fn onboarding_focus_controls(
        &mut self,
        step: usize,
        input: &[u8],
        masked: bool,
        focus: usize,
    ) {
        if (1..=4).contains(&step) {
            let scale = self.ui_scale().max(1);
            let top_bar = (46 * scale).min(self.height / 12).max(40);
            let card_width = (self.width * 34 / 100).clamp(500, 600 * scale);
            let card_height = (self.height * 68 / 100)
                .clamp(560, 680 * scale)
                .min(self.height.saturating_sub(top_bar + 24));
            let card_left = self.width * 4 / 100;
            let card_top = top_bar + self.height.saturating_sub(top_bar + card_height) / 2;
            let inner_left = card_left + 32 * scale;
            let inner_width = card_width.saturating_sub(64 * scale);
            let body_top = card_top + (94 + 132) * scale;
            let placeholder: &[u8] = match step {
                1 => b"InfinityNode",
                2 => b"your-handle",
                3 => b"Display name",
                4 => b"Create a password",
                _ => b"",
            };
            self.onboarding_input_field(
                inner_left,
                body_top + 28 * scale,
                inner_width,
                50 * scale,
                input,
                masked,
                focus == 1,
                placeholder,
            );
        } else if step == 6 {
            let scale = self.ui_scale().max(1);
            let top_bar = (46 * scale).min(self.height / 12).max(40);
            let card_width = (self.width * 34 / 100).clamp(500, 600 * scale);
            let card_height = (self.height * 68 / 100)
                .clamp(560, 680 * scale)
                .min(self.height.saturating_sub(top_bar + 24));
            let card_left = self.width * 4 / 100;
            let card_top = top_bar + self.height.saturating_sub(top_bar + card_height) / 2;
            let inner_left = card_left + 32 * scale;
            let inner_width = card_width.saturating_sub(64 * scale);
            let body_top = card_top + (94 + 132) * scale;
            self.onboarding_network_rows(inner_left, body_top, inner_width, focus, false);
        }
        self.onboarding_actions(step, focus);
    }

    // ------------------------=
    // FUNC: desktop_native_app_window
    // DESC: Renders the live Text Editor or Command Window over the intact authenticated desktop.
    // ------------------=
    fn desktop_native_app_window(
        &mut self,
        screen: u8,
        input: &[u8],
        output_lines: &[[u8; 96]; 6],
        output_lengths: &[usize; 6],
        output_count: usize,
        window_x: i32,
        window_y: i32,
        window_width: i32,
        window_height: i32,
        maximized: bool,
        editor_saved: bool,
        content_only: bool,
        editor_scroll_row: usize,
        editor_dialog: u8,
        editor_dialog_input: &[u8],
        editor_dialog_focus: usize,
    ) {
        let scale = self.ui_scale().max(1);
        let geometry = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
            .desktop_app_window_geometry(
                window_x,
                window_y,
                window_width,
                window_height,
                maximized,
            );
        let left = geometry.window.x.max(0) as usize;
        let top = geometry.window.y.max(0) as usize;
        let width = geometry.window.width as usize;
        let height = geometry.window.height as usize;
        let toolbar_left = geometry.toolbar.x.max(0) as usize;
        let toolbar_top = geometry.toolbar.y.max(0) as usize;
        if !content_only {
            let (header_r, header_g, header_b) =
                self.active_accent_surface(crate::ui::skin::AccentSurface::Header);
            self.fill_rounded_rect_alpha(
                left.saturating_sub(8 * scale),
                top + 8 * scale,
                width.saturating_add(16 * scale),
                height,
                20 * scale,
                0,
                2,
                10,
                120,
            );
            self.glass_panel(left, top, width, height, true);
            self.fill_rect_alpha(
                left,
                top,
                width,
                48 * scale,
                header_r,
                header_g,
                header_b,
                238,
            );
            let icon_role = if screen == 9 { 49 } else { 25 };
            let _ = self.themed_icon(left + 30 * scale, top + 24 * scale, icon_role, 32 * scale);
            self.ui_text_strong(
                left + 54 * scale,
                top + 16 * scale,
                if screen == 9 {
                    b"Text Editor"
                } else {
                    b"Command Window"
                },
                231,
                243,
                250,
                1,
            );
            for (index, control) in [geometry.minimize, geometry.maximize, geometry.close]
                .iter()
                .enumerate()
            {
                let control_left = control.x.max(0) as usize;
                let control_top = control.y.max(0) as usize;
                let control_size = control.width as usize;
                self.fill_rounded_rect_alpha(
                    control_left,
                    control_top,
                    control_size,
                    control.height as usize,
                    7 * scale,
                    11,
                    31,
                    48,
                    244,
                );
                self.outline_rounded_rect(
                    control_left,
                    control_top,
                    control_size,
                    control.height as usize,
                    7 * scale,
                    65,
                    111,
                    139,
                );
                let center_x = control_left + control_size / 2;
                let center_y = control_top + control.height as usize / 2;
                if index == 0 {
                    self.icon_line(
                        (center_x - 5 * scale) as i32,
                        center_y as i32,
                        (center_x + 5 * scale) as i32,
                        center_y as i32,
                        (194, 222, 238),
                        control_size,
                    );
                } else if index == 1 {
                    self.outline_rect(
                        center_x - 5 * scale,
                        center_y - 5 * scale,
                        10 * scale,
                        10 * scale,
                        194,
                        222,
                        238,
                    );
                } else {
                    self.icon_line(
                        (center_x - 5 * scale) as i32,
                        (center_y - 5 * scale) as i32,
                        (center_x + 5 * scale) as i32,
                        (center_y + 5 * scale) as i32,
                        (194, 222, 238),
                        control_size,
                    );
                    self.icon_line(
                        (center_x + 5 * scale) as i32,
                        (center_y - 5 * scale) as i32,
                        (center_x - 5 * scale) as i32,
                        (center_y + 5 * scale) as i32,
                        (194, 222, 238),
                        control_size,
                    );
                }
            }
            if !maximized {
                let (outline_r, outline_g, outline_b) =
                    self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
                for offset in [5usize, 9, 13] {
                    self.icon_line(
                        (left + width - offset * scale) as i32,
                        (top + height - 3 * scale) as i32,
                        (left + width - 3 * scale) as i32,
                        (top + height - offset * scale) as i32,
                        (outline_r, outline_g, outline_b),
                        16 * scale,
                    );
                }
            }
            self.fill_rect_alpha(
                toolbar_left,
                toolbar_top,
                geometry.toolbar.width as usize,
                geometry.toolbar.height as usize,
                5,
                22,
                38,
                224,
            );
            if screen == 9 {
                let toolbar = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                    .desktop_toolbar_geometry(geometry);
                for (index, (label, role)) in [
                    (b"New".as_slice(), 49usize),
                    (b"Open", 50),
                    (b"Save", 51),
                    (b"Save As", 51),
                    (b"Delete", 52),
                ]
                .iter()
                .enumerate()
                {
                    let action = toolbar.actions[index];
                    self.polished_toolbar_button(
                        action.x.max(0) as usize,
                        action.y.max(0) as usize,
                        action.width as usize,
                        action.height as usize,
                        label,
                        *role,
                    );
                }
                let status: &[u8] = if editor_saved { b"Saved" } else { b"Modified" };
                let status_width = self.ui_text_width(status, 1);
                self.ui_text(
                    toolbar.status.x.max(0) as usize
                        + (toolbar.status.width as usize).saturating_sub(status_width) / 2,
                    toolbar_top
                        + (geometry.toolbar.height as usize).saturating_sub(UI_FONT_CELL_HEIGHT)
                            / 2,
                    status,
                    if editor_saved { 108 } else { 102 },
                    if editor_saved { 221 } else { 195 },
                    if editor_saved { 176 } else { 255 },
                    1,
                );
            } else {
                self.ui_text(
                    toolbar_left + 16 * scale,
                    toolbar_top + 13 * scale,
                    b"INFINITY CONSOLE  /  LOCAL SESSION",
                    103,
                    193,
                    235,
                    1,
                );
            }
        } else if screen == 9 {
            let status: &[u8] = if editor_saved { b"Saved" } else { b"Modified" };
            let status_width = self.ui_text_width(status, 1);
            let toolbar = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                .desktop_toolbar_geometry(geometry);
            let status_left = toolbar.status.x.max(0) as usize;
            self.fill_rect(
                status_left,
                toolbar_top,
                toolbar.status.width as usize,
                geometry.toolbar.height as usize,
                5,
                22,
                38,
            );
            self.ui_text(
                status_left + (toolbar.status.width as usize).saturating_sub(status_width) / 2,
                toolbar_top
                    + (geometry.toolbar.height as usize).saturating_sub(UI_FONT_CELL_HEIGHT) / 2,
                status,
                if editor_saved { 108 } else { 102 },
                if editor_saved { 221 } else { 195 },
                if editor_saved { 176 } else { 255 },
                1,
            );
        }
        let content_left = geometry.content.x.max(0) as usize;
        let content_top = geometry.content.y.max(0) as usize;
        let content_width = geometry.content.width as usize;
        let content_height = geometry.content.height as usize;
        if content_only {
            self.fill_rect(
                content_left + scale,
                content_top + scale,
                content_width.saturating_sub(2 * scale),
                content_height.saturating_sub(2 * scale),
                1,
                10,
                20,
            );
        } else {
            self.fill_rounded_rect_alpha(
                content_left,
                content_top,
                content_width,
                content_height,
                10 * scale,
                1,
                10,
                20,
                246,
            );
            self.outline_rounded_rect(
                content_left,
                content_top,
                content_width,
                content_height,
                10 * scale,
                22,
                83,
                116,
            );
        }
        let line_height = 24 * scale;
        if screen == 9 {
            let mut row = 0usize;
            let columns = content_width.saturating_sub(52 * scale) / (9 * scale).max(1);
            let total_rows = crate::ui::text_editor::visual_line_count(input, columns.max(1));
            let visible_rows = (content_height / line_height).max(1);
            let maximum_scroll = total_rows.saturating_sub(visible_rows);
            let scroll_row = editor_scroll_row.min(maximum_scroll);
            let mut start =
                crate::ui::text_editor::visual_line_start(input, columns.max(1), scroll_row);
            let mut caret_width = 0usize;
            while start < input.len() && row * line_height + 36 * scale < content_height {
                let remaining = &input[start..];
                let explicit_end = remaining
                    .iter()
                    .position(|byte| *byte == b'\n')
                    .unwrap_or(remaining.len());
                let take = explicit_end.min(columns.max(1));
                self.ui_text(
                    content_left + 20 * scale,
                    content_top + 18 * scale + row * line_height,
                    &remaining[..take],
                    218,
                    232,
                    241,
                    1,
                );
                caret_width = self.ui_text_width(&remaining[..take], 1);
                start += take;
                if take == explicit_end && start < input.len() && input[start] == b'\n' {
                    start += 1;
                    caret_width = 0;
                    row += 1;
                }
                if take < explicit_end {
                    row += 1;
                }
            }
            let caret_x = content_left + 20 * scale + caret_width;
            let caret_y = content_top + 18 * scale + row * line_height;
            if scroll_row == maximum_scroll {
                self.fill_rect(caret_x, caret_y, 2 * scale, 18 * scale, 111, 220, 255);
            }
            let scroll = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                .desktop_editor_scroll_geometry(
                    window_x,
                    window_y,
                    window_width,
                    window_height,
                    maximized,
                    total_rows,
                    scroll_row,
                );
            if scroll.maximum_scroll > 0 {
                self.fill_rounded_rect_alpha(
                    scroll.track.x.max(0) as usize,
                    scroll.track.y.max(0) as usize,
                    scroll.track.width as usize,
                    scroll.track.height as usize,
                    4 * scale,
                    9,
                    27,
                    42,
                    210,
                );
                self.fill_rounded_rect_alpha(
                    scroll.thumb.x.max(0) as usize,
                    scroll.thumb.y.max(0) as usize,
                    scroll.thumb.width as usize,
                    scroll.thumb.height as usize,
                    4 * scale,
                    95,
                    206,
                    250,
                    245,
                );
            }
            if editor_dialog != 0 {
                self.desktop_editor_dialog(
                    geometry.content,
                    editor_dialog == 2,
                    editor_dialog_input,
                    output_lines,
                    output_lengths,
                    output_count,
                    editor_dialog_focus,
                    scale,
                );
            }
        } else {
            for row in 0..output_count.min(6) {
                self.ui_text(
                    content_left + 18 * scale,
                    content_top + 18 * scale + row * line_height,
                    &output_lines[row][..output_lengths[row].min(96)],
                    183,
                    218,
                    235,
                    1,
                );
            }
            let prompt_y = content_top + content_height.saturating_sub(42 * scale);
            self.fill_rect_alpha(
                content_left + 10 * scale,
                prompt_y.saturating_sub(8 * scale),
                content_width.saturating_sub(20 * scale),
                34 * scale,
                5,
                28,
                45,
                245,
            );
            self.ui_text_strong(
                content_left + 18 * scale,
                prompt_y,
                b"inf >",
                93,
                218,
                255,
                1,
            );
            self.ui_text(content_left + 74 * scale, prompt_y, input, 232, 242, 248, 1);
        }
    }

    // ------------------------=
    // FUNC: desktop_editor_dialog
    // DESC: Renders native Save As and Open object sheets above editor content with mouse and keyboard focus states.
    // ------------------=
    fn desktop_editor_dialog(
        &mut self,
        content: crate::ui::geometry::Rect,
        open_picker: bool,
        name_input: &[u8],
        output_lines: &[[u8; 96]; 6],
        output_lengths: &[usize; 6],
        output_count: usize,
        focus: usize,
        scale: usize,
    ) {
        let sheet_width = (420 * scale).min((content.width as usize).saturating_sub(40 * scale));
        let sheet_height = if open_picker {
            330 * scale
        } else {
            220 * scale
        };
        let left =
            content.x.max(0) as usize + (content.width as usize).saturating_sub(sheet_width) / 2;
        let top =
            content.y.max(0) as usize + (content.height as usize).saturating_sub(sheet_height) / 2;
        self.fill_rounded_rect_alpha(
            left.saturating_sub(8 * scale),
            top + 8 * scale,
            sheet_width.saturating_add(16 * scale),
            sheet_height,
            18 * scale,
            0,
            2,
            8,
            150,
        );
        self.glass_panel(left, top, sheet_width, sheet_height, true);
        self.ui_text_strong(
            left + 24 * scale,
            top + 24 * scale,
            if open_picker {
                b"Open Document"
            } else {
                b"Save As"
            },
            234,
            244,
            250,
            1,
        );
        if open_picker {
            if output_count == 0 {
                self.ui_text(
                    left + 24 * scale,
                    top + 70 * scale,
                    b"No saved documents yet.",
                    160,
                    184,
                    199,
                    1,
                );
            }
            for index in 0..output_count.min(6) {
                let row_top = top + (62 + index * 34) * scale;
                if index == focus {
                    self.fill_rounded_rect_alpha(
                        left + 24 * scale,
                        row_top,
                        sheet_width.saturating_sub(48 * scale),
                        30 * scale,
                        7 * scale,
                        11,
                        72,
                        108,
                        235,
                    );
                }
                self.ui_text(
                    left + 38 * scale,
                    row_top + 8 * scale,
                    &output_lines[index][..output_lengths[index].min(96)],
                    218,
                    234,
                    244,
                    1,
                );
            }
        } else {
            self.fill_rounded_rect_alpha(
                left + 24 * scale,
                top + 72 * scale,
                sheet_width.saturating_sub(48 * scale),
                46 * scale,
                9 * scale,
                2,
                16,
                29,
                245,
            );
            self.outline_rounded_rect(
                left + 24 * scale,
                top + 72 * scale,
                sheet_width.saturating_sub(48 * scale),
                46 * scale,
                9 * scale,
                78,
                195,
                242,
            );
            self.ui_text(
                left + 40 * scale,
                top + 86 * scale,
                name_input,
                231,
                241,
                247,
                1,
            );
        }
        let button_top = top + sheet_height.saturating_sub(60 * scale);
        let button_width = (sheet_width.saturating_sub(60 * scale)) / 2;
        self.polished_button(
            left + 24 * scale,
            button_top,
            button_width,
            crate::ui::system_layout::UI_COMPACT_ACTION_HEIGHT * scale,
            b"CANCEL",
            false,
            false,
        );
        self.polished_button(
            left + (24 + crate::ui::system_layout::UI_CONTROL_GAP) * scale + button_width,
            button_top,
            button_width,
            crate::ui::system_layout::UI_COMPACT_ACTION_HEIGHT * scale,
            if open_picker { b"OPEN" } else { b"SAVE" },
            true,
            false,
        );
    }

    // ------------------------=
    // FUNC: system_ui_frame
    // DESC: Renders onboarding, authentication, desktop, native apps, menu, lock, and Settings from shared state.
    // ------------------=
    pub(super) fn system_ui_frame(
        &mut self,
        screen: u8,
        step: usize,
        input: &[u8],
        masked: bool,
        focus: usize,
        validation_error: bool,
        window_x: i32,
        window_y: i32,
        window_width: i32,
        window_height: i32,
        window_visible: bool,
        window_maximized: bool,
        home_location: usize,
        selected_item: Option<usize>,
        dragging_item: Option<usize>,
        note_location: usize,
        desktop_items: u8,
        desktop_item_positions: &[[i32; 2]; 7],
        clock: crate::storage::DateTimeConfiguration,
        settings_window: crate::ui::system_layout::SettingsWindowState,
        menu_kind: usize,
        output_lines: &[[u8; 96]; 6],
        output_lengths: &[usize; 6],
        output_count: usize,
        app_window_x: i32,
        app_window_y: i32,
        app_window_width: i32,
        app_window_height: i32,
        app_window_maximized: bool,
        editor_saved: bool,
        editor_input: &[u8],
        command_input: &[u8],
        editor_window: crate::ui::system_layout::DesktopAppWindowState,
        command_window: crate::ui::system_layout::DesktopAppWindowState,
        editor_scroll_row: usize,
        editor_dialog: u8,
        editor_dialog_input: &[u8],
        editor_dialog_focus: usize,
    ) {
        self.mark_dirty_rect(0, 0, self.width, self.height);
        if matches!(screen, 5 | 6) {
            self.paint_authentication_background();
            self.authentication_frame(screen == 6, step, input, focus, true);
            return;
        }
        if screen == 1 {
            self.paint_first_boot_background();
            self.onboarding_frame(step, input, masked, focus, validation_error);
            return;
        }
        if matches!(screen, 2 | 3 | 4 | 7 | 8 | 9) {
            self.paint_desktop_background();
        } else {
            self.paint_first_boot_background();
        }
        let scale = self.ui_scale().max(1);
        let margin = self.width * 4 / 100;
        let top_bar = self.system_top_bar((screen == 3).then_some(menu_kind), clock);

        if matches!(screen, 2 | 3 | 7 | 8 | 9) {
            self.desktop_shell(
                scale,
                window_x,
                window_y,
                window_width,
                window_height,
                window_visible,
                window_maximized,
                home_location,
                selected_item,
                dragging_item,
                note_location,
                desktop_items,
                desktop_item_positions,
                screen == 7,
            );
        }

        if screen == 7 {
            self.blur_framebuffer(4);
            self.app_launcher(scale, input, focus);
        }

        if matches!(screen, 2 | 8 | 9) {
            let active_editor = screen == 9;
            let active_command = screen == 8;
            if command_window.visible && !active_command {
                self.desktop_native_app_window(
                    8,
                    command_input,
                    output_lines,
                    output_lengths,
                    output_count,
                    command_window.x,
                    command_window.y,
                    command_window.width,
                    command_window.height,
                    command_window.maximized,
                    true,
                    false,
                    editor_scroll_row,
                    editor_dialog,
                    editor_dialog_input,
                    editor_dialog_focus,
                );
            }
            if editor_window.visible && !active_editor {
                self.desktop_native_app_window(
                    9,
                    editor_input,
                    output_lines,
                    output_lengths,
                    output_count,
                    editor_window.x,
                    editor_window.y,
                    editor_window.width,
                    editor_window.height,
                    editor_window.maximized,
                    editor_saved,
                    false,
                    editor_scroll_row,
                    editor_dialog,
                    editor_dialog_input,
                    editor_dialog_focus,
                );
            }
        }
        if matches!(screen, 8 | 9) {
            self.desktop_native_app_window(
                screen,
                input,
                output_lines,
                output_lengths,
                output_count,
                app_window_x,
                app_window_y,
                app_window_width,
                app_window_height,
                app_window_maximized,
                editor_saved,
                false,
                editor_scroll_row,
                editor_dialog,
                editor_dialog_input,
                editor_dialog_focus,
            );
        }

        if screen == 4 {
            self.render_settings_window(focus, input, settings_window, scale);
            return;
        }

        if matches!(screen, 1 | 5 | 6) {
            let panel_top = if matches!(screen, 5 | 6) {
                top_bar + self.height * 11 / 100
            } else {
                top_bar + self.height * 8 / 100
            };
            let panel_width = if matches!(screen, 5 | 6) {
                self.width * 42 / 100
            } else {
                self.width * 48 / 100
            };
            let panel_height = self.height * 70 / 100;
            let panel_left = if matches!(screen, 5 | 6) {
                self.width.saturating_sub(panel_width) / 2
            } else {
                margin
            };
            self.glass_panel(panel_left, panel_top, panel_width, panel_height, true);
            self.outline_rect(
                panel_left + 6,
                panel_top + 6,
                panel_width - 12,
                panel_height - 12,
                17,
                73,
                103,
            );
            let x = panel_left + 30 * scale;
            let mut y = panel_top + 34 * scale;
            let (eyebrow, title, description): (&[u8], &[u8], &[u8]) = if screen == 5 {
                (
                    b"WELCOME BACK",
                    b"AUTHENTICATE",
                    b"VERIFY YOUR IDENTITY TO BEGIN A PRIVATE SESSION.",
                )
            } else if screen == 6 {
                (
                    b"SESSION LOCKED",
                    b"YOUR SPACE IS SECURE",
                    b"ENTER YOUR CREDENTIAL TO RESUME THIS SESSION.",
                )
            } else {
                match step {
                    0 => (
                        b"FIRST BOOT  /  01 OF 07",
                        b"WELCOME TO INFINITYOS",
                        b"LET'S MAKE THIS SYSTEM YOURS. SETUP STAYS LOCAL.",
                    ),
                    1 => (
                        b"MACHINE IDENTITY  /  02 OF 07",
                        b"NAME THIS INFINITY NODE",
                        b"THE NAME CAN CHANGE. ITS STABLE ID NEVER DOES.",
                    ),
                    2 => (
                        b"USER IDENTITY  /  03 OF 07",
                        b"CREATE YOUR IDENTITY",
                        b"CHOOSE A SHORT HANDLE FOR YOUR PERSONAL SPACE.",
                    ),
                    3 => (
                        b"PROFILE  /  04 OF 07",
                        b"HOW SHOULD WE ADDRESS YOU?",
                        b"THIS DISPLAY NAME CAN BE CHANGED AT ANY TIME.",
                    ),
                    4 => (
                        b"AUTHENTICATION  /  05 OF 07",
                        b"PROTECT YOUR SPACE",
                        b"USE AT LEAST EIGHT CHARACTERS. YOUR SECRET IS NEVER STORED.",
                    ),
                    5 => (
                        b"AI + VOICE  /  06 OF 07",
                        b"PRIVATE BY DEFAULT",
                        b"LOCAL AI IS ON. REMOTE AI AND VOICE START OFF.",
                    ),
                    _ => (
                        b"READY  /  07 OF 07",
                        b"YOUR INFINITY BEGINS HERE",
                        b"CREATE YOUR SESSION AND ENTER YOUR NEW SYSTEM.",
                    ),
                }
            };
            let smooth_system_text = matches!(screen, 5 | 6);
            if smooth_system_text {
                self.ui_text(x, y, eyebrow, 74, 205, 248, 1);
            } else {
                self.text_scaled(x, y, eyebrow, 74, 205, 248, scale, true);
            }
            y += 42 * scale;
            if smooth_system_text {
                self.ui_text(x, y, title, 244, 249, 252, 2);
            } else {
                self.text_scaled(x, y, title, 244, 249, 252, scale * 2, true);
            }
            y += 42 * scale;
            if smooth_system_text {
                self.ui_text(x, y, description, 174, 202, 218, 1);
            } else {
                self.text_scaled(x, y, description, 174, 202, 218, scale, true);
            }
            y += 58 * scale;
            if screen == 1 && step == 0 {
                for line in [
                    b"STABLE NATIVE IDENTITY".as_slice(),
                    b"ISOLATED PERSONAL SPACE".as_slice(),
                    b"EXPLICIT CAPABILITIES".as_slice(),
                    b"LOCAL-FIRST AI POLICY".as_slice(),
                ] {
                    self.fill_rect(x, y + 3 * scale, 8 * scale, 8 * scale, 65, 198, 245);
                    self.text_scaled(x + 24 * scale, y, line, 216, 235, 245, scale, true);
                    y += 34 * scale;
                }
            } else if matches!(screen, 5 | 6) {
                let user_count =
                    crate::runtime::with_runtime(|runtime| runtime.identity.user_count())
                        .unwrap_or(0)
                        .min(4);
                self.ui_text(x, y, b"CHOOSE AN ACCOUNT", 94, 210, 248, 1);
                y += 30 * scale;
                for index in 0..user_count {
                    let selected = screen == 6 || focus == index;
                    let row_y = y + index * 45 * scale;
                    self.fill_rect_alpha(
                        x,
                        row_y,
                        panel_width.saturating_sub(60 * scale),
                        36 * scale,
                        if selected { 17 } else { 5 },
                        if selected { 74 } else { 24 },
                        if selected { 108 } else { 39 },
                        224,
                    );
                    if selected {
                        self.outline_rect(
                            x,
                            row_y,
                            panel_width.saturating_sub(60 * scale),
                            36 * scale,
                            84,
                            204,
                            246,
                        );
                    }
                    let user =
                        crate::runtime::with_runtime(|runtime| runtime.identity.user_nth(index))
                            .flatten();
                    if let Some(user) = user {
                        self.ui_text(
                            x + 14 * scale,
                            row_y + 7 * scale,
                            user.display_name.as_bytes(),
                            238,
                            247,
                            252,
                            1,
                        );
                    }
                }
                y += user_count.max(1) * 45 * scale + 16 * scale;
                self.ui_text(x, y, b"PASSWORD", 94, 210, 248, 1);
                y += 22 * scale;
                let box_width = panel_width.saturating_sub(60 * scale);
                self.fill_rect_alpha(x, y, box_width, 48 * scale, 7, 22, 36, 230);
                self.outline_rect(x, y, box_width, 48 * scale, 63, 191, 237);
                let mut shown = [0u8; 64];
                let shown_len = input.len().min(shown.len());
                if masked {
                    for byte in &mut shown[..shown_len] {
                        *byte = b'*';
                    }
                } else {
                    shown[..shown_len].copy_from_slice(&input[..shown_len]);
                }
                self.ui_text(
                    x + 14 * scale,
                    y + 13 * scale,
                    &shown[..shown_len],
                    236,
                    247,
                    252,
                    1,
                );
            } else if screen == 1 && (1..=4).contains(&step) {
                let box_width = panel_width.saturating_sub(60 * scale);
                self.fill_rect_alpha(x, y, box_width, 48 * scale, 7, 22, 36, 230);
                self.outline_rect(x, y, box_width, 48 * scale, 63, 191, 237);
                let mut shown = [0u8; 64];
                let shown_len = input.len().min(shown.len());
                if masked {
                    shown[..shown_len].fill(b'*');
                } else {
                    shown[..shown_len].copy_from_slice(&input[..shown_len]);
                }
                self.text_scaled(
                    x + 14 * scale,
                    y + 15 * scale,
                    &shown[..shown_len],
                    236,
                    247,
                    252,
                    scale,
                    true,
                );
            } else if screen == 1 && step == 5 {
                self.text_scaled(
                    x,
                    y,
                    b"AI PROVIDER       LOCAL ONLY",
                    229,
                    242,
                    250,
                    scale,
                    true,
                );
                y += 34 * scale;
                self.text_scaled(x, y, b"REMOTE PROCESSING OFF", 229, 242, 250, scale, true);
                y += 34 * scale;
                self.text_scaled(x, y, b"VOICE + MICROPHONE OFF", 229, 242, 250, scale, true);
            }
            let button_y = panel_top + panel_height - 76 * scale;
            let button_width = panel_width.saturating_sub(60 * scale);
            self.polished_button(
                x,
                button_y,
                button_width,
                crate::ui::system_layout::UI_STANDARD_ACTION_HEIGHT * scale,
                if screen == 5 {
                    b"START SESSION"
                } else if screen == 6 {
                    b"UNLOCK"
                } else if step >= 6 {
                    b"ENTER INFINITYOS"
                } else {
                    b"CONTINUE"
                },
                true,
                focus == 1,
            );
        } else if screen == 3 {
            self.system_menu_panel(menu_kind, focus, scale);
        } else if screen == 4 {
            let restored_width = (self.width * 68 / 100)
                .clamp(900, 1200 * scale)
                .min(self.width.saturating_sub(40));
            let restored_height = (self.height * 62 / 100)
                .clamp(560, 760 * scale)
                .min(self.height.saturating_sub(top_bar + 28));
            let (left, top, width, height) = if settings_window.maximized {
                let inset = 10 * scale;
                (
                    inset,
                    top_bar + inset,
                    self.width.saturating_sub(inset * 2),
                    self.height.saturating_sub(top_bar + inset * 2),
                )
            } else {
                (
                    self.width.saturating_sub(restored_width) / 2,
                    top_bar + self.height.saturating_sub(top_bar + restored_height) / 2,
                    restored_width,
                    restored_height,
                )
            };
            let (header_r, header_g, header_b) =
                self.active_accent_surface(crate::ui::skin::AccentSurface::Header);
            let (selection_r, selection_g, selection_b) =
                self.active_accent_surface(crate::ui::skin::AccentSurface::Selection);
            let (outline_r, outline_g, outline_b) =
                self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
            self.glass_panel(left, top, width, height, true);
            let title_height = 54 * scale;
            self.fill_rect_alpha(
                left,
                top,
                width,
                title_height,
                header_r,
                header_g,
                header_b,
                222,
            );
            let title_center_y = top + title_height / 2;
            self.small_infinity_mark(left + 25 * scale, title_center_y, 31 * scale);
            self.ui_text_strong(
                left + 50 * scale,
                title_center_y.saturating_sub(UI_FONT_CELL_HEIGHT / 2),
                b"System Settings",
                241,
                246,
                250,
                1,
            );
            for index in 0..3usize {
                let control_size = 18 * scale;
                let control_left = left + width.saturating_sub((26 + (2 - index) * 25) * scale);
                self.fill_rounded_rect_alpha(
                    control_left,
                    title_center_y.saturating_sub(control_size / 2),
                    control_size,
                    control_size,
                    5 * scale,
                    14,
                    28,
                    42,
                    225,
                );
                self.outline_rounded_rect(
                    control_left,
                    title_center_y.saturating_sub(control_size / 2),
                    control_size,
                    control_size,
                    5 * scale,
                    56,
                    78,
                    96,
                );
            }
            let nav_w = width * 28 / 100;
            self.fill_rect_alpha(
                left,
                top + title_height,
                nav_w,
                height.saturating_sub(title_height),
                4,
                15,
                27,
                214,
            );
            self.fill_rect_alpha(
                left + nav_w,
                top + title_height,
                1,
                height.saturating_sub(title_height),
                36,
                58,
                76,
                180,
            );
            let sections: [&[u8]; 9] = [
                b"General",
                b"Themes & Skins",
                b"Users & Accounts",
                b"AI & Voice",
                b"Privacy & Security",
                b"Devices",
                b"Network",
                b"Storage",
                b"About",
            ];
            for (index, section) in sections.iter().enumerate() {
                let y = top + title_height + (25 + index * 43) * scale;
                if focus == index {
                    self.fill_rounded_rect_alpha(
                        left + 10 * scale,
                        y - 10 * scale,
                        nav_w.saturating_sub(20 * scale),
                        36 * scale,
                        9 * scale,
                        selection_r,
                        selection_g,
                        selection_b,
                        226,
                    );
                }
                self.authentication_icon(
                    left + 27 * scale,
                    y + 8 * scale,
                    [8usize, 13, 6, 7, 8, 11, 14, 11, 12][index],
                    17 * scale,
                    focus == index,
                );
                self.ui_text_strong(
                    left + 48 * scale,
                    y,
                    section,
                    if focus == index { 237 } else { 180 },
                    if focus == index { 245 } else { 198 },
                    if focus == index { 251 } else { 211 },
                    1,
                );
            }
            let content_x = left + nav_w + 34 * scale;
            let content_width = width.saturating_sub(nav_w + 68 * scale);
            let content_y = top + title_height + 29 * scale;
            self.ui_text_strong(
                content_x,
                content_y,
                sections[focus.min(8)],
                238,
                244,
                249,
                2,
            );
            self.ui_text(
                content_x,
                content_y + 36 * scale,
                b"Changes apply through typed InfinityOS settings operations.",
                143,
                160,
                176,
                1,
            );
            let icon_theme = crate::ui::icon_theme::IconThemeId::from_u8(self.active_icon_theme())
                .unwrap_or(crate::ui::icon_theme::IconThemeId::CrystalBlueGlass);
            let network_status = crate::runtime::with_runtime(|runtime| runtime.network.status());
            let connectivity: &[u8] = match network_status.map(|value| value.connectivity) {
                Some(crate::runtime::network::types::ConnectivityClass::Offline) => b"Offline",
                Some(crate::runtime::network::types::ConnectivityClass::LinkOnly) => b"Link only",
                Some(crate::runtime::network::types::ConnectivityClass::LocalNetwork) => {
                    b"Local network"
                }
                Some(crate::runtime::network::types::ConnectivityClass::Routed) => b"Routed",
                Some(crate::runtime::network::types::ConnectivityClass::LimitedConnectivity) => {
                    b"Limited"
                }
                Some(
                    crate::runtime::network::types::ConnectivityClass::InternetReachableOptional,
                ) => b"Reachable",
                _ => b"Degraded",
            };
            let rows: [(&[u8], &[u8]); 5] = match focus.min(8) {
                0 => [
                    (b"Machine Name", input),
                    (b"Language", b"English (US)"),
                    (b"Region", b"United States"),
                    (b"System Generation", b"Active"),
                    (b"Updates", b"Generation based"),
                ],
                1 => [
                    (b"Skin", b"InfinityOS Default Dark"),
                    (b"Icon Set", icon_theme.name()),
                    (b"UI Scale", b"Automatic"),
                    (b"Accent", b"Custom color"),
                    (b"Wallpaper", b"Cosmic Horizon"),
                ],
                2 => [
                    (b"Current User", b"Active"),
                    (b"Credential", b"Password"),
                    (b"Session", b"Authenticated"),
                    (b"Personal Space", b"Private"),
                    (b"Profile", b"Persistent"),
                ],
                3 => [
                    (b"AI Provider", b"Local only"),
                    (b"Remote Processing", b"Off"),
                    (b"Voice", b"Off"),
                    (b"Activation", b"Disabled"),
                    (b"Model Access", b"Capability gated"),
                ],
                4 => [
                    (b"Ambient Authority", b"Denied"),
                    (b"Microphone", b"Not granted"),
                    (b"Remote AI", b"Denied"),
                    (b"Session Auth", b"Verified"),
                    (b"Trusted UI", b"Active"),
                ],
                5 => [
                    (b"Display", b"Ready"),
                    (b"Keyboard", b"Ready"),
                    (b"Pointer", b"Ready"),
                    (b"Audio Input", b"Unavailable"),
                    (b"Audio Output", b"Unavailable"),
                ],
                6 => [
                    (b"Connectivity", connectivity),
                    (b"Profiles", b"5 operational modes"),
                    (b"Interfaces & Topology", b"Inspect"),
                    (b"Application & Service Access", b"Deny by default"),
                    (b"Diagnostics", b"Observed counters"),
                ],
                7 => [
                    (b"Infinity Pool", b"Online"),
                    (b"System Space", b"Ready"),
                    (b"Personal Space", b"Owned"),
                    (b"Recovery Space", b"Ready"),
                    (b"External Drives", b"Discoverable"),
                ],
                _ => [
                    (b"InfinityOS", b"Development"),
                    (b"Architecture", b"Native"),
                    (b"Boot", b"Verified"),
                    (b"Identity Format", b"Version 1"),
                    (b"Icon Families", b"3 complete sets"),
                ],
            };
            for (index, (label, value)) in rows.iter().enumerate() {
                let y = content_y + (78 + index * 58) * scale;
                self.fill_rounded_rect_alpha(
                    content_x,
                    y,
                    content_width,
                    46 * scale,
                    10 * scale,
                    6,
                    20,
                    33,
                    218,
                );
                self.outline_rounded_rect(
                    content_x,
                    y,
                    content_width,
                    46 * scale,
                    10 * scale,
                    outline_r / 2,
                    outline_g / 2,
                    outline_b / 2,
                );
                self.ui_text_strong(
                    content_x + 16 * scale,
                    y + 13 * scale,
                    label,
                    190,
                    205,
                    217,
                    1,
                );
                let value_width = self.ui_text_width(value, 1);
                self.ui_text(
                    content_x + content_width.saturating_sub(value_width + 16 * scale),
                    y + 13 * scale,
                    value,
                    220,
                    232,
                    240,
                    1,
                );
                self.authentication_icon(
                    content_x + content_width.saturating_sub(12 * scale),
                    y + 23 * scale,
                    4,
                    12 * scale,
                    false,
                );
            }
            if focus == 1 {
                self.settings_color_picker(settings_window, scale, false);
            }
        }
    }

    // ------------------------=
    // FUNC: render_settings_window
    // DESC: Renders the resizable Settings window with inline disclosure wells and bounded overflow scrolling.
    // ------------------=
    fn render_settings_window(
        &mut self,
        focus: usize,
        input: &[u8],
        settings_window: crate::ui::system_layout::SettingsWindowState,
        scale: usize,
    ) {
        let layout = crate::ui::system_layout::SystemLayout::new(self.width, self.height);
        let geometry = layout.settings_window_geometry(settings_window);
        let left = geometry.window.x.max(0) as usize;
        let top = geometry.window.y.max(0) as usize;
        let width = geometry.window.width as usize;
        let height = geometry.window.height as usize;
        let title_height = 54 * scale;
        let (header_r, header_g, header_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Header);
        let (selection_r, selection_g, selection_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Selection);
        let (outline_r, outline_g, outline_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
        let (primary_r, primary_g, primary_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Widget);
        let (background_opacity, _) = self.active_background_effects();
        let panel_alpha = |base: u8| (u16::from(base) * u16::from(background_opacity) / 100) as u8;
        self.glass_panel(left, top, width, height, true);
        self.fill_rect_alpha(
            left,
            top,
            width,
            title_height,
            header_r,
            header_g,
            header_b,
            panel_alpha(222),
        );
        let title_center_y = top + title_height / 2;
        self.small_infinity_mark(left + 25 * scale, title_center_y, 31 * scale);
        self.ui_text_strong(
            left + 50 * scale,
            title_center_y.saturating_sub(UI_FONT_CELL_HEIGHT / 2),
            b"System Settings",
            241,
            246,
            250,
            1,
        );
        for index in 0..3usize {
            let control_size = 18 * scale;
            let control_left = left + width.saturating_sub((26 + (2 - index) * 25) * scale);
            self.fill_rounded_rect_alpha(
                control_left,
                title_center_y.saturating_sub(control_size / 2),
                control_size,
                control_size,
                5 * scale,
                14,
                28,
                42,
                225,
            );
            self.outline_rounded_rect(
                control_left,
                title_center_y.saturating_sub(control_size / 2),
                control_size,
                control_size,
                5 * scale,
                56,
                78,
                96,
            );
        }
        let nav_width = geometry.navigation.width as usize;
        self.fill_rect_alpha(
            left,
            top + title_height,
            nav_width,
            height.saturating_sub(title_height),
            primary_r / 2,
            primary_g / 2,
            primary_b / 2,
            panel_alpha(214),
        );
        self.fill_rect_alpha(
            left + nav_width,
            top + title_height,
            1,
            height.saturating_sub(title_height),
            36,
            58,
            76,
            180,
        );
        let sections: [&[u8]; 9] = [
            b"General",
            b"Themes & Skins",
            b"Users & Accounts",
            b"AI & Voice",
            b"Privacy & Security",
            b"Devices",
            b"Network",
            b"Storage",
            b"About",
        ];
        for (index, section) in sections.iter().enumerate() {
            let y = top + title_height + (25 + index * 43) * scale;
            if focus == index {
                self.fill_rounded_rect_alpha(
                    left + 10 * scale,
                    y.saturating_sub(10 * scale),
                    nav_width.saturating_sub(20 * scale),
                    36 * scale,
                    9 * scale,
                    selection_r,
                    selection_g,
                    selection_b,
                    226,
                );
            }
            self.authentication_icon(
                left + 27 * scale,
                y + 8 * scale,
                [8usize, 13, 6, 7, 8, 11, 14, 11, 12][index],
                17 * scale,
                focus == index,
            );
            self.ui_text_strong(
                left + 48 * scale,
                y,
                section,
                if focus == index { 237 } else { 180 },
                if focus == index { 245 } else { 198 },
                if focus == index { 251 } else { 211 },
                1,
            );
        }
        let content_x = geometry.content.x.max(0) as usize;
        let content_y = geometry.content.y.max(0) as usize;
        self.ui_text_strong(
            content_x,
            content_y,
            sections[focus.min(8)],
            238,
            244,
            249,
            2,
        );
        self.ui_text(
            content_x,
            content_y + 36 * scale,
            b"Open a row to view its controls and configuration details.",
            143,
            160,
            176,
            1,
        );
        let icon_theme = crate::ui::icon_theme::IconThemeId::from_u8(self.active_icon_theme())
            .unwrap_or(crate::ui::icon_theme::IconThemeId::CrystalBlueGlass);
        let (opacity, blur) = self.active_background_effects();
        let mut opacity_value = [b'0'; 4];
        opacity_value[3] = b'%';
        let opacity_text: &[u8] = if opacity == 100 {
            opacity_value[..3].copy_from_slice(b"100");
            &opacity_value
        } else {
            opacity_value[0] = b'0' + opacity / 10;
            opacity_value[1] = b'0' + opacity % 10;
            opacity_value[2] = b'%';
            &opacity_value[..3]
        };
        let blur_value = [b'0' + blur.min(8), b' ', b'p', b'x'];
        let network_status = crate::runtime::with_runtime(|runtime| runtime.network.status());
        let connectivity: &[u8] = match network_status.map(|value| value.connectivity) {
            Some(crate::runtime::network::types::ConnectivityClass::Offline) => b"Offline",
            Some(crate::runtime::network::types::ConnectivityClass::LinkOnly) => b"Link only",
            Some(crate::runtime::network::types::ConnectivityClass::LocalNetwork) => {
                b"Local network"
            }
            Some(crate::runtime::network::types::ConnectivityClass::Routed) => b"Routed",
            Some(crate::runtime::network::types::ConnectivityClass::LimitedConnectivity) => {
                b"Limited"
            }
            Some(crate::runtime::network::types::ConnectivityClass::InternetReachableOptional) => {
                b"Reachable"
            }
            _ => b"Degraded",
        };
        let (chat_enabled, chat_model) = crate::runtime::ai::with_ai_runtime(|runtime| {
            (
                runtime.chat.enabled(),
                runtime.chat.selected_model_descriptor().name,
            )
        });
        if focus == 6 {
            self.render_network_settings_dashboard(settings_window, scale, connectivity, input);
            return;
        }
        let rows: [(&[u8], &[u8]); 8] = match focus.min(8) {
            0 => [
                (b"Machine Name", input),
                (b"Language", b"English (US)"),
                (b"Region", b"United States"),
                (b"System Generation", b"Active"),
                (b"Updates", b"Generation based"),
                (b"", b""),
                (b"", b""),
                (b"", b""),
            ],
            1 => [
                (b"Skin", b"InfinityOS Default Dark"),
                (b"Icon Set", icon_theme.name()),
                (b"Primary", b"Custom color"),
                (b"Secondary", b"Custom color"),
                (b"Opacity", opacity_text),
                (b"Blur", &blur_value),
                (b"UI Scale", b"Automatic"),
                (b"Wallpaper", b"Cosmic Horizon"),
            ],
            2 => [
                (b"Current User", b"Active"),
                (b"Credential", b"Password"),
                (b"Session", b"Authenticated"),
                (b"Personal Space", b"Private"),
                (b"Profile", b"Persistent"),
                (b"", b""),
                (b"", b""),
                (b"", b""),
            ],
            3 => [
                (b"AI Provider", b"Local only"),
                (
                    b"Desktop AI Chat",
                    if chat_enabled {
                        b"Enabled"
                    } else {
                        b"Disabled"
                    },
                ),
                (b"Chat Model", chat_model),
                (b"Remote Processing", b"Off"),
                (b"Voice", b"Off"),
                (b"Activation", b"Disabled"),
                (b"Model Access", b"Capability gated"),
                (b"", b""),
            ],
            4 => [
                (b"Ambient Authority", b"Denied"),
                (b"Microphone", b"Not granted"),
                (b"Remote AI", b"Denied"),
                (b"No Activity Timeout", input),
                (b"Trusted UI", b"Active"),
                (b"", b""),
                (b"", b""),
                (b"", b""),
            ],
            5 => [
                (b"Display", b"Ready"),
                (b"Keyboard", b"Ready"),
                (b"Pointer", b"Ready"),
                (b"Audio Input", b"Unavailable"),
                (b"Audio Output", b"Unavailable"),
                (b"", b""),
                (b"", b""),
                (b"", b""),
            ],
            6 => [
                (b"Connectivity", connectivity),
                (b"Profiles", b"5 operational modes"),
                (b"Interfaces & Topology", b"Inspect"),
                (b"Application & Service Access", b"Deny by default"),
                (b"DNS / Resolution", b"Bounded cache"),
                (b"Routes", b"Deterministic"),
                (b"Connections", b"Owner protected"),
                (b"Diagnostics", b"Observed counters"),
            ],
            7 => [
                (b"Infinity Pool", b"Online"),
                (b"System Space", b"Ready"),
                (b"Personal Space", b"Owned"),
                (b"Recovery Space", b"Ready"),
                (b"External Drives", b"Discoverable"),
                (b"", b""),
                (b"", b""),
                (b"", b""),
            ],
            _ => [
                (b"InfinityOS", b"Development"),
                (b"Architecture", b"Native"),
                (b"Boot", b"Verified"),
                (b"Identity Format", b"Version 1"),
                (b"Icon Families", b"3 complete sets"),
                (b"", b""),
                (b"", b""),
                (b"", b""),
            ],
        };
        for (index, (label, value)) in rows
            .iter()
            .take(settings_window.row_count.clamp(1, 8))
            .enumerate()
        {
            let row = layout.settings_row_geometry(settings_window, index);
            if row.summary.y >= geometry.viewport.y
                && row.summary.bottom() <= geometry.viewport.bottom()
            {
                let summary_left = row.summary.x.max(0) as usize;
                let summary_top = row.summary.y.max(0) as usize;
                let summary_width = row.summary.width as usize;
                let expanded = settings_window.expanded_row == Some(index);
                self.fill_rounded_rect_alpha(
                    summary_left,
                    summary_top,
                    summary_width,
                    row.summary.height as usize,
                    10 * scale,
                    primary_r / 2,
                    primary_g.saturating_mul(3) / 4,
                    primary_b.saturating_mul(3) / 4,
                    panel_alpha(218),
                );
                self.outline_rounded_rect(
                    summary_left,
                    summary_top,
                    summary_width,
                    row.summary.height as usize,
                    10 * scale,
                    if expanded { outline_r } else { outline_r / 2 },
                    if expanded { outline_g } else { outline_g / 2 },
                    if expanded { outline_b } else { outline_b / 2 },
                );
                self.ui_text_strong(
                    summary_left + crate::ui::system_layout::UI_GUTTER * scale,
                    summary_top + 13 * scale,
                    label,
                    190,
                    205,
                    217,
                    1,
                );
                let value_width = self.ui_text_width(value, 1);
                self.ui_text(
                    summary_left + summary_width.saturating_sub(value_width + 40 * scale),
                    summary_top + 13 * scale,
                    value,
                    220,
                    232,
                    240,
                    1,
                );
                let twiddle_x = summary_left + summary_width.saturating_sub(20 * scale);
                let twiddle_y = summary_top + 23 * scale;
                if expanded {
                    self.icon_line(
                        (twiddle_x - 5 * scale) as i32,
                        (twiddle_y - 3 * scale) as i32,
                        twiddle_x as i32,
                        (twiddle_y + 3 * scale) as i32,
                        (109, 220, 255),
                        12 * scale,
                    );
                    self.icon_line(
                        twiddle_x as i32,
                        (twiddle_y + 3 * scale) as i32,
                        (twiddle_x + 5 * scale) as i32,
                        (twiddle_y - 3 * scale) as i32,
                        (109, 220, 255),
                        12 * scale,
                    );
                } else {
                    self.icon_line(
                        (twiddle_x - 3 * scale) as i32,
                        (twiddle_y - 5 * scale) as i32,
                        (twiddle_x + 3 * scale) as i32,
                        twiddle_y as i32,
                        (109, 220, 255),
                        12 * scale,
                    );
                    self.icon_line(
                        (twiddle_x + 3 * scale) as i32,
                        twiddle_y as i32,
                        (twiddle_x - 3 * scale) as i32,
                        (twiddle_y + 5 * scale) as i32,
                        (109, 220, 255),
                        12 * scale,
                    );
                }
            }
            if settings_window.expanded_row == Some(index)
                && row.detail.y >= geometry.viewport.y
                && row.detail.bottom() <= geometry.viewport.bottom()
            {
                let detail_left = row.detail.x.max(0) as usize;
                let detail_top = row.detail.y.max(0) as usize;
                let detail_width = row.detail.width as usize;
                let detail_height = row.detail.height as usize;
                self.fill_rounded_rect_alpha(
                    detail_left,
                    detail_top,
                    detail_width,
                    detail_height,
                    9 * scale,
                    primary_r / 2,
                    primary_g.saturating_mul(2) / 3,
                    primary_b.saturating_mul(2) / 3,
                    panel_alpha(232),
                );
                self.outline_rounded_rect(
                    detail_left,
                    detail_top,
                    detail_width,
                    detail_height,
                    9 * scale,
                    outline_r / 2,
                    outline_g / 2,
                    outline_b / 2,
                );
                if focus == 1 && index == 1 {
                    let card_width = detail_width / 3;
                    for theme in 0..3usize {
                        let card_left = detail_left + theme * card_width + 5 * scale;
                        let selected = self.active_icon_theme() as usize == theme;
                        self.fill_rounded_rect_alpha(
                            card_left,
                            detail_top + 8 * scale,
                            card_width.saturating_sub(10 * scale),
                            detail_height.saturating_sub(16 * scale),
                            8 * scale,
                            if selected { selection_r } else { 8 },
                            if selected { selection_g } else { 28 },
                            if selected { selection_b } else { 44 },
                            226,
                        );
                        let _ = self.icon_theme_preview(
                            theme as u8,
                            card_left + card_width / 2,
                            detail_top + 39 * scale,
                            40 * scale,
                        );
                    }
                } else if focus == 1 && index == 2 {
                    self.settings_color_picker(settings_window, scale, true);
                } else if focus == 1 && index == 3 {
                    self.settings_color_picker(settings_window, scale, false);
                } else if focus == 1 && matches!(index, 4 | 5) {
                    self.settings_effect_slider(settings_window, scale, index);
                } else if focus == 4 && index == 3 {
                    self.settings_slider(
                        settings_window,
                        scale,
                        index,
                        parse_leading_u8(input)
                            .unwrap_or(
                                crate::runtime::identity::DEFAULT_NO_ACTIVITY_TIMEOUT_MINUTES,
                            )
                            .saturating_sub(1),
                        crate::runtime::identity::MAX_NO_ACTIVITY_TIMEOUT_MINUTES - 1,
                    );
                } else {
                    let description: &[u8] = match (focus, index) {
                        (0, 0) => b"Rename this machine through the durable identity service.",
                        (1, 0) => b"Switch between installed, verified InfinityUI skins.",
                        (1, 6) => b"Automatic scale follows the active display density.",
                        (1, 7) => b"Cosmic Horizon is the active packaged desktop wallpaper.",
                        (3, 0) => {
                            b"Choose whether the local provider is strictly required or preferred."
                        }
                        (4, 3) => {
                            b"Lock this user's session after the selected period without input."
                        }
                        _ => b"This value is read from the active System Generation.",
                    };
                    self.ui_text(
                        detail_left + crate::ui::system_layout::UI_GUTTER * scale,
                        detail_top + crate::ui::system_layout::UI_GUTTER * scale,
                        description,
                        167,
                        188,
                        203,
                        1,
                    );
                    let action: Option<&[u8]> = match (focus, index) {
                        (0, 0) => Some(b"EDIT NAME"),
                        (1, 0) => Some(b"SWITCH SKIN"),
                        (3, 0) => Some(b"CHANGE POLICY"),
                        _ => None,
                    };
                    if let Some(action) = action {
                        self.polished_button(
                            detail_left + crate::ui::system_layout::UI_GUTTER * scale,
                            detail_top
                                + detail_height.saturating_sub(
                                    (crate::ui::system_layout::UI_COMPACT_ACTION_HEIGHT + 10)
                                        * scale,
                                ),
                            (170 * scale).min(
                                detail_width.saturating_sub(
                                    crate::ui::system_layout::UI_GUTTER * 2 * scale,
                                ),
                            ),
                            crate::ui::system_layout::UI_COMPACT_ACTION_HEIGHT * scale,
                            action,
                            true,
                            false,
                        );
                    }
                }
            }
        }
        if geometry.maximum_scroll > 0 {
            let track = geometry.scrollbar_track;
            let thumb = geometry.scrollbar_thumb;
            self.fill_rounded_rect_alpha(
                track.x.max(0) as usize,
                track.y.max(0) as usize,
                track.width as usize,
                track.height as usize,
                3 * scale,
                6,
                19,
                31,
                190,
            );
            self.fill_rounded_rect_alpha(
                thumb.x.max(0) as usize,
                thumb.y.max(0) as usize,
                thumb.width as usize,
                thumb.height as usize,
                3 * scale,
                outline_r,
                outline_g,
                outline_b,
                235,
            );
        }
        if !settings_window.maximized {
            for offset in [5usize, 9, 13] {
                self.icon_line(
                    (left + width - offset * scale) as i32,
                    (top + height - 3 * scale) as i32,
                    (left + width - 3 * scale) as i32,
                    (top + height - offset * scale) as i32,
                    (outline_r, outline_g, outline_b),
                    16 * scale,
                );
            }
        }
    }

    // ------------------------=
    // FUNC: render_network_settings_dashboard
    // DESC: Renders a scalable live network overview, topology, telemetry, and operational profile selector.
    // ------------------=
    fn render_network_settings_dashboard(
        &mut self,
        settings_window: crate::ui::system_layout::SettingsWindowState,
        scale: usize,
        connectivity: &[u8],
        input: &[u8],
    ) {
        let layout = crate::ui::system_layout::SystemLayout::new(self.width, self.height);
        let geometry = layout.network_settings_geometry(settings_window);
        let snapshot = crate::runtime::with_runtime(|runtime| {
            let static_address = (0..runtime.network.interfaces.address_count())
                .filter_map(|index| runtime.network.interfaces.address_nth(index))
                .find(|value| {
                    value.interface_id == 2
                        && value.source == crate::runtime::network::types::AddressSource::Static
                })
                .copied();
            let default_route = (0..runtime.network.interfaces.route_count())
                .filter_map(|index| runtime.network.interfaces.route_nth(index))
                .find(|value| value.interface_id == 2 && value.prefix_length == 0)
                .copied();
            (
                runtime.network.status(),
                runtime.network.diagnostics(),
                runtime.network.setup_snapshot(),
                runtime.network.interfaces.interface(2).copied(),
                static_address,
                default_route,
                runtime.network.resolver.enabled(),
                runtime.network.resolver.server(0),
                runtime.network.resolver.server(1),
                runtime.network.policy.default_action(),
            )
        });
        let Some((
            status,
            diagnostics,
            setup,
            interface,
            static_address,
            default_route,
            resolver_enabled,
            resolver_primary,
            resolver_secondary,
            default_policy,
        )) = snapshot
        else {
            return;
        };
        let (outline_r, outline_g, outline_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
        let (selection_r, selection_g, selection_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Selection);
        let cards = [geometry.summary, geometry.main, geometry.sidebar];
        for card in cards {
            self.fill_rounded_rect_alpha(
                card.x.max(0) as usize,
                card.y.max(0) as usize,
                card.width as usize,
                card.height as usize,
                12 * scale,
                4,
                18,
                31,
                220,
            );
            self.outline_rounded_rect(
                card.x.max(0) as usize,
                card.y.max(0) as usize,
                card.width as usize,
                card.height as usize,
                12 * scale,
                outline_r / 2,
                outline_g / 2,
                outline_b / 2,
            );
        }

        let page = settings_window.expanded_row.unwrap_or(0).min(6);
        let page_labels: [&[u8]; 7] = [
            b"OVERVIEW",
            b"INTERFACES",
            b"IPv4",
            b"DNS",
            b"ROUTES",
            b"PROFILES",
            b"POLICY",
        ];
        for (index, tab) in geometry.tabs.iter().enumerate() {
            let active = page == index;
            self.fill_rounded_rect_alpha(
                tab.x.max(0) as usize,
                tab.y.max(0) as usize,
                tab.width as usize,
                tab.height as usize,
                7 * scale,
                if active { selection_r } else { 5 },
                if active { selection_g } else { 20 },
                if active { selection_b } else { 34 },
                226,
            );
            self.outline_rounded_rect(
                tab.x.max(0) as usize,
                tab.y.max(0) as usize,
                tab.width as usize,
                tab.height as usize,
                7 * scale,
                if active { outline_r } else { outline_r / 2 },
                if active { outline_g } else { outline_g / 2 },
                if active { outline_b } else { outline_b / 2 },
            );
            self.ui_text_centered(
                tab.x.max(0) as usize,
                tab.width as usize,
                tab.y.max(0) as usize + 9 * scale,
                page_labels[index],
                if active { 242 } else { 166 },
                if active { 248 } else { 190 },
                if active { 252 } else { 207 },
                1,
            );
        }

        let overview_left = geometry.summary.x.max(0) as usize;
        let overview_top = geometry.summary.y.max(0) as usize;
        self.authentication_icon(
            overview_left + 36 * scale,
            overview_top + geometry.summary.height as usize / 2,
            14,
            44 * scale,
            true,
        );
        self.ui_text_strong(
            overview_left + 74 * scale,
            overview_top + 14 * scale,
            b"CONNECTIVITY",
            outline_r,
            outline_g,
            outline_b,
            1,
        );
        self.ui_text_strong(
            overview_left + 74 * scale,
            overview_top + 39 * scale,
            connectivity,
            239,
            246,
            251,
            2,
        );
        let profile_name: &[u8] = match status.active_profile {
            1 => b"STANDARD",
            2 => b"RESTRICTED",
            3 => b"OFFLINE",
            4 => b"OPERATIONS",
            5 => b"DEVELOPER",
            _ => b"CUSTOM",
        };
        let pill_width = 142 * scale;
        let pill_left = overview_left + geometry.summary.width as usize - pill_width - 18 * scale;
        self.fill_rounded_rect_alpha(
            pill_left,
            overview_top + 17 * scale,
            pill_width,
            48 * scale,
            24 * scale,
            selection_r,
            selection_g,
            selection_b,
            220,
        );
        self.ui_text_centered(
            pill_left,
            pill_width,
            overview_top + 33 * scale,
            profile_name,
            242,
            249,
            253,
            1,
        );

        let mode_labels: [&[u8]; 4] = [b"AUTOMATIC", b"WIRED", b"WI-FI", b"OFFLINE"];
        let profile_labels: [&[u8]; 5] = [
            b"STANDARD",
            b"RESTRICTED",
            b"OFFLINE",
            b"OPERATIONS",
            b"DEVELOPER",
        ];
        let control_labels: [&[u8]; 6] = match page {
            0 => [
                mode_labels[0],
                mode_labels[1],
                mode_labels[2],
                mode_labels[3],
                b"REFRESH STATE",
                b"OPEN DIAGNOSTICS",
            ],
            1 => [
                b"PRIMARY ADAPTER",
                b"ENABLED",
                b"LINK STATE",
                b"HARDWARE ADDRESS",
                b"MAXIMUM FRAME",
                b"REFRESH DEVICES",
            ],
            2 => [
                b"ADDRESSING MODE",
                b"IPv4 ADDRESS",
                b"PREFIX LENGTH",
                b"DEFAULT GATEWAY",
                b"ROUTE METRIC",
                b"APPLY STATIC CONFIG",
            ],
            3 => [
                b"RESOLVER",
                b"PRIMARY DNS",
                b"SECONDARY DNS",
                b"CLEAR DNS SERVERS",
                b"CACHE ENTRIES",
                b"APPLY DNS CONFIG",
            ],
            4 => [
                b"DEFAULT ROUTE",
                b"NEXT HOP",
                b"INTERFACE",
                b"METRIC",
                b"ROUTE SOURCE",
                b"REMOVE DEFAULT ROUTE",
            ],
            5 => [
                profile_labels[0],
                profile_labels[1],
                profile_labels[2],
                profile_labels[3],
                profile_labels[4],
                b"RESTORE STANDARD",
            ],
            _ => [
                b"UNMATCHED OUTBOUND",
                b"LOCAL DISCOVERY",
                b"INBOUND LISTENERS",
                b"AUDIT DECISIONS",
                b"INSTALLED RULES",
                b"RESTORE SAFE POLICY",
            ],
        };
        let (address_text, address_length) =
            Self::network_address_text(static_address.map(|value| value.address));
        let (gateway_text, gateway_length) =
            Self::network_address_text(default_route.and_then(|value| value.next_hop));
        let (primary_text, primary_length) = Self::network_address_text(resolver_primary);
        let (secondary_text, secondary_length) = Self::network_address_text(resolver_secondary);
        let (prefix_text, prefix_length) = Self::network_metric_text(
            static_address
                .map(|value| value.prefix_length as u64)
                .unwrap_or(24),
        );
        let (metric_text, metric_length) = Self::network_metric_text(
            default_route
                .map(|value| value.metric as u64)
                .unwrap_or(100),
        );
        let (mtu_text, mtu_length) = Self::network_metric_text(
            interface
                .map(|value| value.device.maximum_frame_size as u64)
                .unwrap_or(0),
        );
        let active_profile = status.active_profile.saturating_sub(1) as usize;
        for (index, card) in geometry.controls.iter().enumerate() {
            let left = card.x.max(0) as usize;
            let top = card.y.max(0) as usize;
            let active = settings_window.scroll_offset.min(5) == index;
            self.fill_rounded_rect_alpha(
                left,
                top,
                card.width as usize,
                card.height as usize,
                8 * scale,
                if active { selection_r } else { 6 },
                if active { selection_g } else { 24 },
                if active { selection_b } else { 39 },
                230,
            );
            self.outline_rounded_rect(
                left,
                top,
                card.width as usize,
                card.height as usize,
                8 * scale,
                if active { outline_r } else { outline_r / 2 },
                if active { outline_g } else { outline_g / 2 },
                if active { outline_b } else { outline_b / 2 },
            );
            self.ui_text_strong(
                left + 15 * scale,
                top + card.height as usize / 2 - UI_FONT_CELL_HEIGHT / 2,
                control_labels[index],
                if active { 240 } else { 190 },
                if active { 247 } else { 211 },
                if active { 251 } else { 224 },
                1,
            );
            let value: &[u8] = match page {
                0 if index < 4 => {
                    let selected = index
                        == match setup.selected {
                            crate::runtime::network::types::NetworkSetupMode::Automatic => 0,
                            crate::runtime::network::types::NetworkSetupMode::Wired => 1,
                            crate::runtime::network::types::NetworkSetupMode::Wireless => 2,
                            crate::runtime::network::types::NetworkSetupMode::Offline => 3,
                        };
                    if selected {
                        b"ACTIVE"
                    } else {
                        b"SELECT"
                    }
                }
                0 => b"RUN",
                1 => match index {
                    0 => {
                        if interface.is_some() {
                            b"network0"
                        } else {
                            b"Not detected"
                        }
                    }
                    1 => {
                        if interface.map(|value| value.enabled).unwrap_or(false) {
                            b"ON"
                        } else {
                            b"OFF"
                        }
                    }
                    2 => match interface.map(|value| value.device.link_state) {
                        Some(crate::runtime::network::types::LinkState::Up) => b"UP",
                        Some(crate::runtime::network::types::LinkState::Down) => b"DOWN",
                        _ => b"UNKNOWN",
                    },
                    3 => {
                        if interface
                            .and_then(|value| value.device.hardware_address)
                            .is_some()
                        {
                            b"Observed"
                        } else {
                            b"Unavailable"
                        }
                    }
                    4 => &mtu_text[..mtu_length],
                    _ => b"RUN",
                },
                2 => match index {
                    0 => {
                        if static_address.is_some() {
                            b"STATIC"
                        } else {
                            b"DYNAMIC"
                        }
                    }
                    1 => &address_text[..address_length],
                    2 => &prefix_text[..prefix_length],
                    3 => &gateway_text[..gateway_length],
                    4 => &metric_text[..metric_length],
                    _ => b"COMMIT",
                },
                3 => match index {
                    0 => {
                        if resolver_enabled {
                            b"ON"
                        } else {
                            b"OFF"
                        }
                    }
                    1 => &primary_text[..primary_length],
                    2 => &secondary_text[..secondary_length],
                    3 => b"CLEAR",
                    4 => b"BOUNDED",
                    _ => b"COMMIT",
                },
                4 => match index {
                    0 => {
                        if default_route.is_some() {
                            b"INSTALLED"
                        } else {
                            b"NONE"
                        }
                    }
                    1 => &gateway_text[..gateway_length],
                    2 => b"network0",
                    3 => &metric_text[..metric_length],
                    4 => {
                        if default_route
                            .map(|value| {
                                value.source == crate::runtime::network::types::RouteSource::Static
                            })
                            .unwrap_or(false)
                        {
                            b"STATIC"
                        } else {
                            b"DISCOVERED"
                        }
                    }
                    _ => b"REMOVE",
                },
                5 if index < 5 => {
                    if active_profile == index {
                        b"ACTIVE"
                    } else {
                        b"SELECT"
                    }
                }
                5 => b"RESTORE",
                _ => match index {
                    0 => match default_policy {
                        crate::runtime::network::types::PolicyAction::Allow => b"ALLOW",
                        crate::runtime::network::types::PolicyAction::Ask => b"ASK",
                        _ => b"DENY",
                    },
                    1 => b"PROFILE CONTROLLED",
                    2 => b"PROFILE CONTROLLED",
                    3 => b"ON",
                    4 => b"INSPECT",
                    _ => b"SAFE DEFAULTS",
                },
            };
            let displayed = if active
                && !input.is_empty()
                && matches!((page, index), (2, 1..=4) | (3, 1..=2))
            {
                input
            } else {
                value
            };
            let value_width = self.ui_text_width(displayed, 1);
            self.ui_text(
                left + card.width as usize - value_width - 18 * scale,
                top + card.height as usize / 2 - UI_FONT_CELL_HEIGHT / 2,
                displayed,
                outline_r,
                outline_g,
                outline_b,
                1,
            );
        }

        let sidebar_left = geometry.sidebar.x.max(0) as usize;
        let sidebar_top = geometry.sidebar.y.max(0) as usize;
        self.ui_text_strong(
            sidebar_left + 17 * scale,
            sidebar_top + 16 * scale,
            b"LIVE CONFIGURATION",
            outline_r,
            outline_g,
            outline_b,
            1,
        );
        let metrics = [
            (b"Interfaces".as_slice(), status.interfaces as u64),
            (b"Addresses".as_slice(), status.addresses as u64),
            (b"Routes".as_slice(), status.routes as u64),
            (
                b"Connections".as_slice(),
                diagnostics.active_connections as u64,
            ),
            (b"Policy rules".as_slice(), status.policies as u64),
        ];
        for (index, (label, value)) in metrics.iter().enumerate() {
            let row_y = sidebar_top + (48 + index * 30) * scale;
            self.ui_text(sidebar_left + 17 * scale, row_y, label, 169, 190, 206, 1);
            let (digits, length) = Self::network_metric_text(*value);
            self.ui_text_strong(
                sidebar_left + geometry.sidebar.width as usize - (28 + length * 10) * scale,
                row_y,
                &digits[..length],
                235,
                245,
                251,
                1,
            );
        }

        let detail_top = sidebar_top + 220 * scale;
        self.ui_text_strong(
            sidebar_left + 17 * scale,
            detail_top,
            page_labels[page],
            outline_r,
            outline_g,
            outline_b,
            1,
        );
        let state_lines: [&[u8]; 5] = match page {
            0 => [
                b"Choose a connection mode.",
                b"Changes apply transactionally.",
                if setup.wired_available {
                    b"Wired adapter available"
                } else {
                    b"No wired adapter"
                },
                if setup.wireless_available {
                    b"Wi-Fi adapter available"
                } else {
                    b"No Wi-Fi adapter"
                },
                b"Local services remain available offline.",
            ],
            1 => [
                if interface.is_some() {
                    b"Adapter discovered"
                } else {
                    b"No external adapter"
                },
                if interface.map(|value| value.enabled).unwrap_or(false) {
                    b"Interface enabled"
                } else {
                    b"Interface disabled"
                },
                b"Hardware values are observed only.",
                b"Toggle the adapter with ENABLED.",
                b"Refresh never fabricates link state.",
            ],
            2 => [
                if static_address.is_some() {
                    b"Static IPv4 is configured."
                } else {
                    b"Dynamic addressing is active."
                },
                b"Address format: 10.0.2.15",
                b"Prefix range: 0 through 32",
                b"Gateway is optional.",
                b"Apply commits address and route.",
            ],
            3 => [
                if resolver_enabled {
                    b"Resolver enabled"
                } else {
                    b"Resolver disabled"
                },
                if resolver_primary.is_some() {
                    b"Primary server configured"
                } else {
                    b"Primary server automatic"
                },
                if resolver_secondary.is_some() {
                    b"Secondary server configured"
                } else {
                    b"Secondary server not set"
                },
                b"Cache and queries are bounded.",
                b"Wire DNS depends on adapter support.",
            ],
            4 => [
                if default_route.is_some() {
                    b"Default route installed"
                } else {
                    b"No default route"
                },
                b"Longest-prefix selection is active.",
                b"Lower metric wins ties.",
                b"Static routes persist across boot.",
                b"Route changes are capability gated.",
            ],
            5 => [
                b"Standard: ordinary connectivity",
                b"Restricted: local-first policy",
                b"Offline: disables external access",
                b"Operations: controlled inbound",
                b"Developer: broader local services",
            ],
            _ => [
                match default_policy {
                    crate::runtime::network::types::PolicyAction::Allow => b"Default: allow",
                    crate::runtime::network::types::PolicyAction::Ask => b"Default: ask",
                    _ => b"Default: deny",
                },
                b"Rules bind stable identities.",
                b"No executable-path authority.",
                b"Changes persist as typed state.",
                b"Unknown applications remain denied.",
            ],
        };
        for (index, line) in state_lines.iter().enumerate() {
            self.ui_text(
                sidebar_left + 17 * scale,
                detail_top + (32 + index * 28) * scale,
                line,
                174,
                198,
                215,
                1,
            );
        }
        if !settings_window.maximized {
            let window = layout.settings_window_geometry(settings_window).window;
            let right = window.right().max(0) as usize;
            let bottom = window.bottom().max(0) as usize;
            for offset in [5usize, 9, 13] {
                self.icon_line(
                    (right - offset * scale) as i32,
                    (bottom - 3 * scale) as i32,
                    (right - 3 * scale) as i32,
                    (bottom - offset * scale) as i32,
                    (outline_r, outline_g, outline_b),
                    16 * scale,
                );
            }
        }
    }

    // ------------------------=
    // FUNC: network_metric_text
    // DESC: Formats one bounded live network count without allocation or fabricated units.
    // ------------------=
    fn network_metric_text(mut value: u64) -> ([u8; 20], usize) {
        let mut output = [b'0'; 20];
        if value == 0 {
            return (output, 1);
        }
        let mut reverse = [0u8; 20];
        let mut length = 0usize;
        while value > 0 && length < reverse.len() {
            reverse[length] = b'0' + (value % 10) as u8;
            value /= 10;
            length += 1;
        }
        for index in 0..length {
            output[index] = reverse[length - index - 1];
        }
        (output, length)
    }

    // ------------------------=
    // FUNC: network_address_text
    // DESC: Formats an optional typed IPv4 address for the Settings projection without allocating text state.
    // ------------------=
    fn network_address_text(
        address: Option<crate::runtime::network::types::IpAddress>,
    ) -> ([u8; 15], usize) {
        let mut output = [0u8; 15];
        let Some(crate::runtime::network::types::IpAddress::V4(octets)) = address else {
            output[..9].copy_from_slice(b"Automatic");
            return (output, 9);
        };
        let mut length = 0usize;
        for (index, octet) in octets.iter().copied().enumerate() {
            if index != 0 {
                output[length] = b'.';
                length += 1;
            }
            let hundreds = octet / 100;
            let tens = (octet / 10) % 10;
            if hundreds != 0 {
                output[length] = b'0' + hundreds;
                length += 1;
                output[length] = b'0' + tens;
                length += 1;
            } else if tens != 0 {
                output[length] = b'0' + tens;
                length += 1;
            }
            output[length] = b'0' + octet % 10;
            length += 1;
        }
        (output, length)
    }

    // ------------------------=
    // FUNC: settings_effect_slider
    // DESC: Renders one full-strength slider over a background-only opacity or blur preview value.
    // ------------------=
    fn settings_effect_slider(
        &mut self,
        settings_window: crate::ui::system_layout::SettingsWindowState,
        scale: usize,
        index: usize,
    ) {
        let (opacity, blur) = self.active_background_effects();
        let (value, maximum) = if index == 4 {
            (opacity.saturating_sub(40) / 4, 15)
        } else {
            (blur, 8)
        };
        self.settings_slider(settings_window, scale, index, value, maximum);
    }

    // ------------------------=
    // FUNC: settings_slider
    // DESC: Renders the shared polished track and thumb for a bounded Settings value.
    // ------------------=
    fn settings_slider(
        &mut self,
        settings_window: crate::ui::system_layout::SettingsWindowState,
        scale: usize,
        index: usize,
        value: u8,
        maximum: u8,
    ) {
        let layout = crate::ui::system_layout::SystemLayout::new(self.width, self.height);
        let geometry =
            layout.settings_effect_slider_geometry(settings_window, index, value, maximum);
        let (outline_r, outline_g, outline_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
        self.fill_rounded_rect_alpha(
            geometry.track.x.max(0) as usize,
            geometry.track.y.max(0) as usize,
            geometry.track.width as usize,
            geometry.track.height as usize,
            4 * scale,
            214,
            228,
            241,
            94,
        );
        self.outline_rounded_rect(
            geometry.track.x.max(0) as usize,
            geometry.track.y.max(0) as usize,
            geometry.track.width as usize,
            geometry.track.height as usize,
            4 * scale,
            outline_r,
            outline_g,
            outline_b,
        );
        self.fill_rounded_rect_alpha(
            geometry.thumb.x.max(0) as usize,
            geometry.thumb.y.max(0) as usize,
            geometry.thumb.width as usize,
            geometry.thumb.height as usize,
            10 * scale,
            239,
            247,
            255,
            255,
        );
        self.outline_rounded_rect(
            geometry.thumb.x.max(0) as usize,
            geometry.thumb.y.max(0) as usize,
            geometry.thumb.width as usize,
            geometry.thumb.height as usize,
            10 * scale,
            outline_r,
            outline_g,
            outline_b,
        );
    }

    // ------------------------=
    // FUNC: settings_color_picker
    // DESC: Renders a live Primary or Accent HSV picker from shared hit-test geometry.
    // ------------------=
    fn settings_color_picker(
        &mut self,
        settings_window: crate::ui::system_layout::SettingsWindowState,
        scale: usize,
        primary: bool,
    ) {
        let layout = crate::ui::system_layout::SystemLayout::new(self.width, self.height);
        let geometry = if primary {
            layout.settings_primary_geometry(settings_window)
        } else {
            layout.settings_accent_geometry(settings_window)
        };
        let current = if primary {
            self.active_primary_rgb()
        } else {
            self.active_accent_rgb()
        };
        let (hue, saturation, value) = crate::ui::skin::rgb_to_hsv(current);
        let columns = 32usize;
        let rows = 12usize;
        let cell_width = (geometry.spectrum.width as usize / columns).max(1);
        let cell_height = (geometry.spectrum.height as usize / rows).max(1);
        for row in 0..rows {
            for column in 0..columns {
                let sample_saturation = (column * 255 / (columns - 1)) as u8;
                let sample_value = 255u8.saturating_sub((row * 255 / (rows - 1)) as u8);
                let rgb = crate::ui::skin::hsv_to_rgb(hue, sample_saturation, sample_value);
                self.fill_rect(
                    geometry.spectrum.x.max(0) as usize + column * cell_width,
                    geometry.spectrum.y.max(0) as usize + row * cell_height,
                    if column + 1 == columns {
                        geometry.spectrum.width as usize - column * cell_width
                    } else {
                        cell_width
                    },
                    if row + 1 == rows {
                        geometry.spectrum.height as usize - row * cell_height
                    } else {
                        cell_height
                    },
                    ((rgb >> 16) & 0xff) as u8,
                    ((rgb >> 8) & 0xff) as u8,
                    (rgb & 0xff) as u8,
                );
            }
        }
        let hue_steps = 18usize;
        let hue_height = (geometry.hue.height as usize / hue_steps).max(1);
        for step in 0..hue_steps {
            let sample_hue = (step * 359 / (hue_steps - 1)) as u16;
            let rgb = crate::ui::skin::hsv_to_rgb(sample_hue, 255, 255);
            self.fill_rect(
                geometry.hue.x.max(0) as usize,
                geometry.hue.y.max(0) as usize + step * hue_height,
                geometry.hue.width as usize,
                if step + 1 == hue_steps {
                    geometry.hue.height as usize - step * hue_height
                } else {
                    hue_height
                },
                ((rgb >> 16) & 0xff) as u8,
                ((rgb >> 8) & 0xff) as u8,
                (rgb & 0xff) as u8,
            );
        }
        let (outline_r, outline_g, outline_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
        self.outline_rounded_rect(
            geometry.spectrum.x.max(0) as usize,
            geometry.spectrum.y.max(0) as usize,
            geometry.spectrum.width as usize,
            geometry.spectrum.height as usize,
            8 * scale,
            outline_r,
            outline_g,
            outline_b,
        );
        self.outline_rounded_rect(
            geometry.hue.x.max(0) as usize,
            geometry.hue.y.max(0) as usize,
            geometry.hue.width as usize,
            geometry.hue.height as usize,
            7 * scale,
            outline_r,
            outline_g,
            outline_b,
        );
        let marker_x = geometry.spectrum.x.max(0) as usize
            + saturation as usize * geometry.spectrum.width as usize / 255;
        let marker_y = geometry.spectrum.y.max(0) as usize
            + (255usize.saturating_sub(value as usize)) * geometry.spectrum.height as usize / 255;
        self.outline_rounded_rect(
            marker_x.saturating_sub(5 * scale),
            marker_y.saturating_sub(5 * scale),
            10 * scale,
            10 * scale,
            5 * scale,
            255,
            255,
            255,
        );
        let hue_marker_y =
            geometry.hue.y.max(0) as usize + hue as usize * geometry.hue.height as usize / 359;
        self.fill_rect(
            (geometry.hue.x.max(0) as usize).saturating_sub(3 * scale),
            hue_marker_y.saturating_sub(scale),
            geometry.hue.width as usize + 6 * scale,
            2 * scale,
            255,
            255,
            255,
        );
    }

    // ------------------------=
    // FUNC: onboarding_glass_panel
    // DESC: Restores the fixed neutral navy glass used by first-boot configuration before user appearance preferences exist.
    // ------------------=
    pub(super) fn onboarding_glass_panel(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        self.glass_panel_with_palette(left, top, width, height, true, (2, 12, 24), (34, 83, 112));
    }

    // ------------------------=
    // FUNC: glass_panel
    // DESC: Builds a layered translucent panel with restrained shadow, highlight, and cyan edge treatment.
    // ------------------=
    pub(super) fn glass_panel(
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
    fn glass_panel_with_palette(
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
        if blur >= 2 && opacity < 100 {
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

    // ------------------------=
    // FUNC: desktop_icon
    // DESC: Draws one scalable glass application or collection tile and its label.
    // ------------------=
    pub(super) fn desktop_icon(&mut self, left: usize, top: usize, label: &[u8], kind: usize) {
        let size = (self.height / 21).max(44);
        let role = match label {
            b"Documents" => 4,
            b"Downloads" => 5,
            b"Pictures" => 6,
            b"Music" => 7,
            b"Videos" => 8,
            b"Projects" => 9,
            b"notes.txt" => 49,
            _ => 2,
        };
        if self.themed_icon(left + size / 2, top + size / 2, role, size) {
            let label_width = self.ui_text_width(label, 1);
            self.ui_text(
                left + size.saturating_sub(label_width) / 2,
                top + size + 8,
                label,
                221,
                235,
                243,
                1,
            );
            return;
        }
        if kind == 3 {
            let paper_x = left + size / 7;
            let paper_w = size * 5 / 7;
            self.fill_rounded_rect_alpha(left + 4, top + 6, size, size, size / 9, 0, 3, 9, 150);
            self.fill_rounded_rect_alpha(
                paper_x,
                top,
                paper_w,
                size,
                size / 12,
                237,
                243,
                248,
                255,
            );
            self.outline_rounded_rect(paper_x, top, paper_w, size, size / 12, 151, 170, 184);
            self.fill_rect_alpha(
                paper_x + paper_w * 2 / 3,
                top,
                paper_w / 3,
                size / 3,
                188,
                205,
                218,
                255,
            );
            for row in 0..3usize {
                self.fill_rect_alpha(
                    paper_x + size / 8,
                    top + size * (5 + row * 2) / 12,
                    paper_w * 2 / 3,
                    2,
                    104,
                    125,
                    141,
                    210,
                );
            }
        } else {
            let tab_w = size * 9 / 16;
            self.fill_rounded_rect_alpha(
                left + 4,
                top + 7,
                size,
                size * 7 / 8,
                size / 8,
                0,
                4,
                12,
                160,
            );
            self.fill_rounded_rect_alpha(
                left,
                top + size / 7,
                size,
                size * 6 / 7,
                size / 8,
                27,
                137,
                219,
                255,
            );
            self.fill_rounded_rect_alpha(
                left + size / 14,
                top,
                tab_w,
                size / 3,
                size / 10,
                68,
                187,
                246,
                255,
            );
            self.fill_rounded_rect_alpha(
                left + 2,
                top + size / 4,
                size.saturating_sub(4),
                size / 3,
                size / 12,
                89,
                200,
                250,
                190,
            );
            self.outline_rounded_rect(
                left,
                top + size / 7,
                size,
                size * 6 / 7,
                size / 8,
                127,
                224,
                255,
            );
            let glyph_color = (7, 75, 129);
            if kind == 1 {
                self.icon_line(
                    (left + size / 2) as i32,
                    (top + size / 3) as i32,
                    (left + size / 2) as i32,
                    (top + size * 2 / 3) as i32,
                    glyph_color,
                    size,
                );
                self.icon_line(
                    (left + size / 3) as i32,
                    (top + size * 7 / 12) as i32,
                    (left + size / 2) as i32,
                    (top + size * 3 / 4) as i32,
                    glyph_color,
                    size,
                );
                self.icon_line(
                    (left + size * 2 / 3) as i32,
                    (top + size * 7 / 12) as i32,
                    (left + size / 2) as i32,
                    (top + size * 3 / 4) as i32,
                    glyph_color,
                    size,
                );
            } else if kind == 2 {
                self.small_infinity_mark(left + size / 2, top + size / 2, size * 2 / 3);
            } else {
                self.outline_rounded_rect(
                    left + size / 4,
                    top + size * 5 / 12,
                    size / 2,
                    size / 3,
                    size / 14,
                    glyph_color.0,
                    glyph_color.1,
                    glyph_color.2,
                );
            }
        }
        let label_width = self.ui_text_width(label, 1);
        self.ui_text(
            left + size.saturating_sub(label_width) / 2,
            top + size + 8,
            label,
            221,
            235,
            243,
            1,
        );
    }

    // ------------------------=
    // FUNC: desktop_app_icon
    // DESC: Draws one original dimensional InfinityOS application icon for dock-scale presentation.
    // ------------------=
    pub(super) fn desktop_app_icon(
        &mut self,
        left: usize,
        top: usize,
        size: usize,
        kind: usize,
        active: bool,
    ) {
        let role = [25usize, 2, 26, 28, 23, 27, 17, 10]
            .get(kind)
            .copied()
            .unwrap_or(2);
        if self.themed_icon(left + size / 2, top + size / 2, role, size) {
            if active {
                self.fill_rounded_rect_alpha(
                    left + size / 2 - 3,
                    top + size + 5,
                    6,
                    3,
                    2,
                    125,
                    220,
                    255,
                    255,
                );
            }
            return;
        }
        let radius = (size / 5).max(7);
        self.fill_rounded_rect_alpha(left + 3, top + 5, size, size, radius, 0, 3, 9, 180);
        let palette = match kind {
            3 => (77, 60, 221, 124, 92, 255),
            4 => (6, 85, 129, 33, 184, 237),
            5 => (5, 58, 103, 44, 151, 232),
            6 => (7, 67, 104, 42, 172, 224),
            7 => (30, 42, 54, 97, 115, 132),
            _ => (8, 54, 91, 47, 157, 224),
        };
        self.fill_rounded_rect_alpha(
            left, top, size, size, radius, palette.0, palette.1, palette.2, 250,
        );
        self.fill_rounded_rect_alpha(
            left + 2,
            top + 2,
            size.saturating_sub(4),
            size / 2,
            radius.saturating_sub(2),
            palette.3,
            palette.4,
            palette.5,
            155,
        );
        self.outline_rounded_rect(
            left,
            top,
            size,
            size,
            radius,
            if active { 153 } else { 75 },
            if active { 228 } else { 147 },
            if active { 255 } else { 190 },
        );
        let center_x = left + size / 2;
        let center_y = top + size / 2;
        let stroke = (222, 245, 255);
        match kind {
            0 => {
                self.icon_line(
                    (left + size / 4) as i32,
                    (top + size * 2 / 5) as i32,
                    (left + size * 2 / 5) as i32,
                    (top + size / 2) as i32,
                    stroke,
                    size,
                );
                self.icon_line(
                    (left + size * 2 / 5) as i32,
                    (top + size / 2) as i32,
                    (left + size / 4) as i32,
                    (top + size * 3 / 5) as i32,
                    stroke,
                    size,
                );
                self.icon_line(
                    (left + size / 2) as i32,
                    (top + size * 3 / 5) as i32,
                    (left + size * 3 / 4) as i32,
                    (top + size * 3 / 5) as i32,
                    stroke,
                    size,
                );
            }
            1 => {
                self.fill_rounded_rect_alpha(
                    left + size / 5,
                    top + size * 7 / 20,
                    size * 3 / 5,
                    size * 2 / 5,
                    size / 12,
                    88,
                    202,
                    250,
                    255,
                );
                self.fill_rounded_rect_alpha(
                    left + size / 4,
                    top + size / 4,
                    size / 3,
                    size / 4,
                    size / 12,
                    131,
                    225,
                    255,
                    255,
                );
                self.outline_rounded_rect(
                    left + size / 5,
                    top + size * 7 / 20,
                    size * 3 / 5,
                    size * 2 / 5,
                    size / 12,
                    182,
                    239,
                    255,
                );
            }
            2 => {
                self.icon_circle(
                    center_x as i32,
                    center_y as i32,
                    size as i32 / 3,
                    (151, 226, 255),
                    size,
                );
                self.icon_circle(
                    center_x as i32,
                    center_y as i32,
                    size as i32 / 5,
                    (211, 246, 255),
                    size,
                );
                self.icon_line(
                    (left + size / 5) as i32,
                    center_y as i32,
                    (left + size * 4 / 5) as i32,
                    center_y as i32,
                    stroke,
                    size,
                );
                self.icon_line(
                    center_x as i32,
                    (top + size / 5) as i32,
                    center_x as i32,
                    (top + size * 4 / 5) as i32,
                    stroke,
                    size,
                );
            }
            3 => {
                self.ui_text_centered_strong(
                    left,
                    size,
                    top + size / 2 - 10,
                    b"AI",
                    255,
                    255,
                    255,
                    1,
                );
            }
            4 => {
                let microphone_top = top + size / 4;
                self.outline_rounded_rect(
                    center_x - size / 9,
                    microphone_top,
                    size * 2 / 9,
                    size * 7 / 20,
                    size / 9,
                    stroke.0,
                    stroke.1,
                    stroke.2,
                );
                self.icon_line(
                    (center_x - size / 4) as i32,
                    (top + size / 2) as i32,
                    (center_x - size / 4) as i32,
                    (top + size * 3 / 5) as i32,
                    stroke,
                    size,
                );
                self.icon_line(
                    (center_x - size / 4) as i32,
                    (top + size * 3 / 5) as i32,
                    center_x as i32,
                    (top + size * 7 / 10) as i32,
                    stroke,
                    size,
                );
                self.icon_line(
                    center_x as i32,
                    (top + size * 7 / 10) as i32,
                    (center_x + size / 4) as i32,
                    (top + size * 3 / 5) as i32,
                    stroke,
                    size,
                );
                self.icon_line(
                    center_x as i32,
                    (top + size * 7 / 10) as i32,
                    center_x as i32,
                    (top + size * 4 / 5) as i32,
                    stroke,
                    size,
                );
            }
            5 => self.authentication_icon(center_x, center_y, 8, size / 2, true),
            6 => {
                self.authentication_icon(center_x, center_y, 8, size / 2, true);
                self.icon_line(
                    (left + size * 2 / 5) as i32,
                    center_y as i32,
                    center_x as i32,
                    (top + size * 3 / 5) as i32,
                    stroke,
                    size,
                );
                self.icon_line(
                    center_x as i32,
                    (top + size * 3 / 5) as i32,
                    (left + size * 3 / 4) as i32,
                    (top + size * 2 / 5) as i32,
                    stroke,
                    size,
                );
            }
            _ => {
                self.fill_rounded_rect_alpha(
                    left + size / 3,
                    top + size / 3,
                    size / 3,
                    size / 2,
                    size / 10,
                    126,
                    151,
                    169,
                    240,
                );
                self.icon_line(
                    (left + size / 4) as i32,
                    (top + size / 3) as i32,
                    (left + size * 3 / 4) as i32,
                    (top + size / 3) as i32,
                    stroke,
                    size,
                );
            }
        }
        if active {
            self.fill_rounded_rect_alpha(
                left + size / 2 - 3,
                top + size + 5,
                6,
                3,
                2,
                125,
                220,
                255,
                255,
            );
        }
    }

    // ------------------------=
    // FUNC: launcher_dock_icon
    // DESC: Draws the dedicated Infinity launcher control and its live open-state indicator.
    // ------------------=
    pub(super) fn launcher_dock_icon(
        &mut self,
        left: usize,
        top: usize,
        size: usize,
        active: bool,
    ) {
        let radius = (size / 5).max(7);
        self.fill_rounded_rect_alpha(left + 3, top + 5, size, size, radius, 0, 3, 10, 185);
        self.fill_rounded_rect_alpha(
            left,
            top,
            size,
            size,
            radius,
            if active { 7 } else { 4 },
            if active { 64 } else { 28 },
            if active { 103 } else { 52 },
            244,
        );
        self.outline_rounded_rect(
            left,
            top,
            size,
            size,
            radius,
            if active { 115 } else { 55 },
            if active { 217 } else { 137 },
            if active { 255 } else { 184 },
        );
        self.small_infinity_mark(left + size / 2, top + size / 2, size * 3 / 4);
        if active {
            self.fill_rounded_rect_alpha(
                left + size / 2 - 5,
                top + size + 5,
                10,
                3,
                2,
                126,
                222,
                255,
                255,
            );
        }
    }

    // ------------------------=
    // FUNC: app_launcher
    // DESC: Renders the searchable native application and category panel above the installed desktop dock.
    // ------------------=
    pub(super) fn app_launcher(&mut self, scale: usize, query: &[u8], focus: usize) {
        self.paint_app_launcher(scale, query, focus);
    }

    // ------------------------=
    // FUNC: app_launcher_content_update
    // DESC: Repaints only opaque focus outlines so pointer movement cannot alter the glass backdrop.
    // ------------------=
    pub(super) fn app_launcher_content_update(&mut self, scale: usize, query: &[u8], focus: usize) {
        let geometry = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
            .app_launcher_geometry();
        let search_left = geometry.search.x.max(0) as usize;
        let search_top = geometry.search.y.max(0) as usize;
        self.outline_rounded_rect(
            search_left,
            search_top,
            geometry.search.width as usize,
            geometry.search.height as usize,
            geometry.search.height as usize / 2,
            if focus == 0 { 101 } else { 50 },
            if focus == 0 { 205 } else { 121 },
            if focus == 0 { 255 } else { 168 },
        );
        let visible = crate::ui::app_launcher::launcher_visible_count(query);
        for visible_index in 0..visible {
            let column = visible_index % 6;
            let row = visible_index / 6;
            let cell_left = geometry.grid_left + column * geometry.grid_cell_width;
            let cell_top = geometry.grid_top + row * geometry.grid_row_height;
            let well_size = geometry
                .grid_cell_width
                .min(geometry.grid_row_height)
                .saturating_mul(70)
                / 100;
            let well_left = cell_left + geometry.grid_cell_width.saturating_sub(well_size) / 2;
            let selected = focus == visible_index + 1;
            self.outline_rounded_rect(
                well_left,
                cell_top + 3 * scale,
                well_size,
                well_size,
                15 * scale,
                if selected { 100 } else { 48 },
                if selected { 211 } else { 105 },
                if selected { 255 } else { 146 },
            );
        }
        for index in 0..crate::ui::app_launcher::LAUNCHER_CATEGORIES.len() {
            let left = geometry.category_left + index * geometry.category_width + 6 * scale;
            let width = geometry.category_width.saturating_sub(12 * scale);
            let selected = focus == visible + index + 1;
            self.outline_rounded_rect(
                left,
                geometry.category_top,
                width,
                geometry.category_height,
                12 * scale,
                if selected { 98 } else { 45 },
                if selected { 212 } else { 99 },
                if selected { 255 } else { 139 },
            );
        }
    }

    // ------------------------=
    // FUNC: paint_app_launcher
    // DESC: Composes the complete translucent launcher from one stable desktop backdrop.
    // ------------------=
    fn paint_app_launcher(&mut self, scale: usize, query: &[u8], focus: usize) {
        let geometry = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
            .app_launcher_geometry();
        let panel_left = geometry.panel.x.max(0) as usize;
        let panel_top = geometry.panel.y.max(0) as usize;
        let panel_width = geometry.panel.width as usize;
        let panel_height = geometry.panel.height as usize;
        self.fill_rounded_rect_alpha(
            panel_left.saturating_sub(8 * scale),
            panel_top + 10 * scale,
            panel_width.saturating_add(16 * scale),
            panel_height,
            26 * scale,
            0,
            2,
            10,
            132,
        );
        self.glass_panel(panel_left, panel_top, panel_width, panel_height, true);
        self.fill_rounded_rect_alpha(
            panel_left + 2 * scale,
            panel_top + 2 * scale,
            panel_width.saturating_sub(4 * scale),
            panel_height / 3,
            22 * scale,
            14,
            40,
            68,
            74,
        );
        let close_left = geometry.close.x.max(0) as usize;
        let close_top = geometry.close.y.max(0) as usize;
        let close_size = geometry.close.width as usize;
        self.fill_rounded_rect_alpha(
            close_left,
            close_top,
            close_size,
            geometry.close.height as usize,
            7 * scale,
            12,
            30,
            47,
            238,
        );
        self.outline_rounded_rect(
            close_left,
            close_top,
            close_size,
            geometry.close.height as usize,
            7 * scale,
            75,
            111,
            137,
        );
        let center_x = close_left + close_size / 2;
        let center_y = close_top + geometry.close.height as usize / 2;
        self.icon_line(
            (center_x - 5 * scale) as i32,
            (center_y - 5 * scale) as i32,
            (center_x + 5 * scale) as i32,
            (center_y + 5 * scale) as i32,
            (201, 224, 239),
            close_size,
        );
        self.icon_line(
            (center_x + 5 * scale) as i32,
            (center_y - 5 * scale) as i32,
            (center_x - 5 * scale) as i32,
            (center_y + 5 * scale) as i32,
            (201, 224, 239),
            close_size,
        );

        let search_left = geometry.search.x.max(0) as usize;
        let search_top = geometry.search.y.max(0) as usize;
        let search_width = geometry.search.width as usize;
        let search_height = geometry.search.height as usize;
        self.fill_rounded_rect_alpha(
            search_left,
            search_top,
            search_width,
            search_height,
            search_height / 2,
            5,
            18,
            34,
            232,
        );
        self.outline_rounded_rect(
            search_left,
            search_top,
            search_width,
            search_height,
            search_height / 2,
            if focus == 0 { 101 } else { 50 },
            if focus == 0 { 205 } else { 121 },
            if focus == 0 { 255 } else { 168 },
        );
        let search_icon_size = (24 * scale).min(search_height.saturating_sub(10));
        let _ = self.themed_icon(
            search_left + 27 * scale,
            search_top + search_height / 2,
            27,
            search_icon_size,
        );
        let search_text = if query.is_empty() {
            b"Search apps, files, settings and more...".as_slice()
        } else {
            query
        };
        self.ui_text(
            search_left + 50 * scale,
            search_top + search_height / 2 - UI_FONT_CELL_HEIGHT / 2,
            search_text,
            if query.is_empty() { 126 } else { 224 },
            if query.is_empty() { 151 } else { 236 },
            if query.is_empty() { 177 } else { 247 },
            1,
        );
        let shortcut = b"/ SEARCH";
        let shortcut_width = self.ui_text_width(shortcut, 1);
        self.fill_rounded_rect_alpha(
            search_left + search_width.saturating_sub(shortcut_width + 25 * scale),
            search_top + 10 * scale,
            shortcut_width + 14 * scale,
            search_height.saturating_sub(20 * scale),
            6 * scale,
            16,
            37,
            58,
            215,
        );
        self.ui_text(
            search_left + search_width.saturating_sub(shortcut_width + 18 * scale),
            search_top + search_height / 2 - UI_FONT_CELL_HEIGHT / 2,
            shortcut,
            138,
            166,
            192,
            1,
        );

        let visible = crate::ui::app_launcher::launcher_visible_count(query);
        for visible_index in 0..visible {
            let Some(entry) = crate::ui::app_launcher::launcher_visible_entry(query, visible_index)
            else {
                continue;
            };
            let column = visible_index % 6;
            let row = visible_index / 6;
            let cell_left = geometry.grid_left + column * geometry.grid_cell_width;
            let cell_top = geometry.grid_top + row * geometry.grid_row_height;
            let selected = focus == visible_index + 1;
            let well_size = geometry
                .grid_cell_width
                .min(geometry.grid_row_height)
                .saturating_mul(70)
                / 100;
            let well_left = cell_left + geometry.grid_cell_width.saturating_sub(well_size) / 2;
            let well_top = cell_top + 3 * scale;
            self.fill_rounded_rect_alpha(
                well_left,
                well_top,
                well_size,
                well_size,
                13 * scale,
                7,
                28,
                49,
                186,
            );
            self.outline_rounded_rect(
                well_left,
                well_top,
                well_size,
                well_size,
                13 * scale,
                if selected { 100 } else { 48 },
                if selected { 211 } else { 105 },
                if selected { 255 } else { 146 },
            );
            let _ = self.launcher_icon(
                well_left + well_size / 2,
                well_top + well_size / 2,
                entry.icon_role,
                well_size * 84 / 100,
            );
            self.ui_text_centered_strong(
                cell_left,
                geometry.grid_cell_width,
                well_top + well_size + 8 * scale,
                entry.label,
                220,
                235,
                245,
                1,
            );
        }
        if visible == 0 {
            self.ui_text_centered_strong(
                geometry.grid_left,
                geometry.grid_cell_width * 6,
                geometry.grid_top + geometry.grid_row_height - UI_FONT_CELL_HEIGHT / 2,
                b"No matching applications",
                149,
                178,
                201,
                1,
            );
        }

        let divider_y = panel_top + panel_height * 70 / 100;
        self.fill_rect_alpha(
            geometry.category_left,
            divider_y,
            geometry.category_width * 5,
            scale,
            55,
            109,
            143,
            150,
        );
        for (index, entry) in crate::ui::app_launcher::LAUNCHER_CATEGORIES
            .iter()
            .enumerate()
        {
            let left = geometry.category_left + index * geometry.category_width + 6 * scale;
            let width = geometry.category_width.saturating_sub(12 * scale);
            let selected = focus == visible + index + 1;
            self.fill_rounded_rect_alpha(
                left,
                geometry.category_top,
                width,
                geometry.category_height,
                12 * scale,
                7,
                29,
                50,
                220,
            );
            self.outline_rounded_rect(
                left,
                geometry.category_top,
                width,
                geometry.category_height,
                12 * scale,
                if selected { 98 } else { 45 },
                if selected { 212 } else { 99 },
                if selected { 255 } else { 139 },
            );
            let icon_size = geometry.category_height * 48 / 100;
            let _ = self.launcher_icon(
                left + width / 2,
                geometry.category_top + geometry.category_height * 37 / 100,
                entry.icon_role,
                icon_size,
            );
            self.ui_text_centered(
                left,
                width,
                geometry.category_top + geometry.category_height * 68 / 100,
                entry.label,
                197,
                220,
                237,
                1,
            );
        }
    }

    // ------------------------=
    // FUNC: desktop_window_rect
    // DESC: Resolves the movable Home window bounds in framebuffer pixels.
    // ------------------=
    pub(super) fn desktop_window_rect(
        &self,
        window_x: i32,
        window_y: i32,
        window_width: i32,
        window_height: i32,
    ) -> (usize, usize, usize, usize) {
        crate::ui::system_layout::SystemLayout::new(self.width, self.height)
            .home_window_geometry_sized(window_x, window_y, window_width, window_height, false)
    }

    // ------------------------=
    // FUNC: desktop_shell
    // DESC: Renders the screenshot-matched InfinityOS desktop, home browser, status cards, and application dock.
    // ------------------=
    pub(super) fn desktop_shell(
        &mut self,
        scale: usize,
        window_x: i32,
        window_y: i32,
        window_width: i32,
        window_height: i32,
        window_visible: bool,
        window_maximized: bool,
        home_location: usize,
        _selected_item: Option<usize>,
        dragging_item: Option<usize>,
        _note_location: usize,
        desktop_items: u8,
        desktop_item_positions: &[[i32; 2]; 7],
        launcher_open: bool,
    ) {
        self.desktop_widgets(scale);
        for (index, (name, kind)) in [
            (b"Documents".as_slice(), 0),
            (b"Downloads", 1),
            (b"Pictures", 0),
            (b"Music", 0),
            (b"Videos", 0),
            (b"Projects", 2),
            (b"notes.txt", 3),
        ]
        .iter()
        .enumerate()
        {
            if desktop_items & (1u8 << index) == 0 {
                continue;
            }
            let [x, y] = desktop_item_positions[index];
            let pixel_x = self.width * x.clamp(35, 950) as usize / 1000;
            let pixel_y = self.height * y.clamp(90, 880) as usize / 1000;
            self.desktop_icon(pixel_x, pixel_y, name, *kind);
        }
        if window_visible {
            let navigator_state =
                crate::runtime::with_runtime(|runtime| runtime.file_navigator).flatten();
            let navigator_list_view = navigator_state
                .map(|state| state.view_mode == crate::runtime::object_navigation::ViewMode::List)
                .unwrap_or(true);
            let (browser_left, browser_top, browser_width, browser_height) = if window_maximized {
                let left = 10 * scale;
                let top = (46 * scale).min(self.height / 12).max(40) + 10 * scale;
                let bottom = self.height.saturating_sub(90 * scale);
                (
                    left,
                    top,
                    self.width.saturating_sub(left * 2),
                    bottom.saturating_sub(top),
                )
            } else {
                crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                    .home_window_geometry_sized(
                        window_x,
                        window_y,
                        window_width,
                        window_height,
                        false,
                    )
            };
            self.glass_panel(
                browser_left,
                browser_top,
                browser_width,
                browser_height,
                false,
            );
            let (header_r, header_g, header_b) =
                self.active_accent_surface(crate::ui::skin::AccentSurface::Header);
            let (selection_r, selection_g, selection_b) =
                self.active_accent_surface(crate::ui::skin::AccentSurface::Selection);
            let title_h = 34 * scale;
            self.fill_rect_alpha(
                browser_left,
                browser_top,
                browser_width,
                title_h,
                header_r,
                header_g,
                header_b,
                225,
            );
            let title_center_y = browser_top + title_h / 2;
            self.small_infinity_mark(browser_left + 20 * scale, title_center_y, 24 * scale);
            self.ui_text_strong(
                browser_left + 38 * scale,
                title_center_y.saturating_sub(UI_FONT_CELL_HEIGHT / 2),
                b"File Navigator",
                226,
                237,
                245,
                1,
            );
            for index in 0..3usize {
                let control_size = 20 * scale;
                let control_left =
                    browser_left + browser_width.saturating_sub((28 + (2 - index) * 27) * scale);
                self.fill_rounded_rect_alpha(
                    control_left,
                    title_center_y.saturating_sub(control_size / 2),
                    control_size,
                    control_size,
                    6 * scale,
                    13,
                    28,
                    43,
                    235,
                );
                self.outline_rounded_rect(
                    control_left,
                    title_center_y.saturating_sub(control_size / 2),
                    control_size,
                    control_size,
                    6 * scale,
                    63,
                    84,
                    101,
                );
                let center_x = control_left + control_size / 2;
                let center_y = title_center_y;
                if index == 0 {
                    self.icon_line(
                        (center_x - 5 * scale) as i32,
                        center_y as i32,
                        (center_x + 5 * scale) as i32,
                        center_y as i32,
                        (181, 199, 212),
                        control_size,
                    );
                } else if index == 1 {
                    self.outline_rounded_rect(
                        center_x - 5 * scale,
                        center_y - 5 * scale,
                        10 * scale,
                        10 * scale,
                        2 * scale,
                        181,
                        199,
                        212,
                    );
                } else {
                    self.icon_line(
                        (center_x - 5 * scale) as i32,
                        (center_y - 5 * scale) as i32,
                        (center_x + 5 * scale) as i32,
                        (center_y + 5 * scale) as i32,
                        (209, 220, 229),
                        control_size,
                    );
                    self.icon_line(
                        (center_x + 5 * scale) as i32,
                        (center_y - 5 * scale) as i32,
                        (center_x - 5 * scale) as i32,
                        (center_y + 5 * scale) as i32,
                        (209, 220, 229),
                        control_size,
                    );
                }
            }
            if !window_maximized {
                let (outline_r, outline_g, outline_b) =
                    self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
                for offset in [5usize, 9, 13] {
                    self.icon_line(
                        (browser_left + browser_width - offset * scale) as i32,
                        (browser_top + browser_height - 3 * scale) as i32,
                        (browser_left + browser_width - 3 * scale) as i32,
                        (browser_top + browser_height - offset * scale) as i32,
                        (outline_r, outline_g, outline_b),
                        16 * scale,
                    );
                }
            }
            let tool_top = browser_top + title_h;
            self.fill_rect_alpha(
                browser_left,
                tool_top,
                browser_width,
                38 * scale,
                4,
                15,
                27,
                216,
            );
            for (index, role) in [45usize, 46, 47].iter().enumerate() {
                let enabled = navigator_state
                    .map(|state| match index {
                        0 => !state.back_namespace_ref.as_bytes().is_empty(),
                        1 => !state.forward_namespace_ref.as_bytes().is_empty(),
                        _ => state.active_namespace_ref.as_bytes() != b"/",
                    })
                    .unwrap_or(false);
                if !self.themed_icon(
                    browser_left + (20 + index * 28) * scale,
                    tool_top + 19 * scale,
                    *role,
                    18 * scale,
                ) {
                    let center_x = (browser_left + (20 + index * 28) * scale) as i32;
                    let center_y = (tool_top + 19 * scale) as i32;
                    let color = if enabled {
                        (184, 224, 245)
                    } else {
                        (66, 91, 109)
                    };
                    if index == 2 {
                        self.icon_line(
                            center_x - 5 * scale as i32,
                            center_y,
                            center_x,
                            center_y - 5 * scale as i32,
                            color,
                            18 * scale,
                        );
                        self.icon_line(
                            center_x,
                            center_y - 5 * scale as i32,
                            center_x + 5 * scale as i32,
                            center_y,
                            color,
                            18 * scale,
                        );
                    } else {
                        let direction = if index == 0 { -1 } else { 1 };
                        self.icon_line(
                            center_x,
                            center_y - 5 * scale as i32,
                            center_x + direction * 5 * scale as i32,
                            center_y,
                            color,
                            18 * scale,
                        );
                        self.icon_line(
                            center_x + direction * 5 * scale as i32,
                            center_y,
                            center_x,
                            center_y + 5 * scale as i32,
                            color,
                            18 * scale,
                        );
                    }
                }
            }
            let location_left = browser_left + 100 * scale;
            let mode_controls_width = 96 * scale;
            let location_width = browser_width.saturating_sub(154 * scale + mode_controls_width);
            self.fill_rounded_rect_alpha(
                location_left,
                tool_top + 5 * scale,
                location_width,
                28 * scale,
                8 * scale,
                4,
                15,
                27,
                238,
            );
            let location_editing = navigator_state
                .map(|state| state.location_editing)
                .unwrap_or(false);
            self.outline_rounded_rect(
                location_left,
                tool_top + 5 * scale,
                location_width,
                28 * scale,
                8 * scale,
                if location_editing { 74 } else { 38 },
                if location_editing { 190 } else { 62 },
                if location_editing { 235 } else { 81 },
            );
            let location_names: [&[u8]; 9] = [
                b"/home/default",
                b"/home/default",
                b"/home/default/documents",
                b"/home/default/downloads",
                b"/home/default/pictures",
                b"/home/default/media",
                b"/home/default/media",
                b"/home/default/projects",
                b"/trash",
            ];
            self.ui_text(
                location_left + 14 * scale,
                tool_top + 9 * scale,
                navigator_state
                    .as_ref()
                    .map(|state| {
                        if state.location_editing {
                            state.editor_text.as_bytes()
                        } else {
                            state.active_namespace_ref.as_bytes()
                        }
                    })
                    .unwrap_or(location_names[home_location.min(8)]),
                193,
                211,
                224,
                1,
            );
            if location_editing {
                let text_width = navigator_state
                    .map(|state| state.editor_text.as_bytes().len() * UI_FONT_CELL_WIDTH)
                    .unwrap_or(0);
                self.fill_rect(
                    (location_left + 14 * scale + text_width)
                        .min(location_left + location_width.saturating_sub(10 * scale)),
                    tool_top + 10 * scale,
                    scale.max(1),
                    15 * scale,
                    112,
                    221,
                    255,
                );
            }
            for (index, label) in [b"List".as_slice(), b"Grid"].iter().enumerate() {
                let control_left =
                    browser_left + browser_width.saturating_sub((100 - index * 46) * scale);
                let control_width = 42 * scale;
                let selected =
                    (index == 0 && navigator_list_view) || (index == 1 && !navigator_list_view);
                self.fill_rounded_rect_alpha(
                    control_left,
                    tool_top + 6 * scale,
                    control_width,
                    26 * scale,
                    7 * scale,
                    if selected { selection_r } else { 9 },
                    if selected { selection_g } else { 29 },
                    if selected { selection_b } else { 45 },
                    230,
                );
                self.ui_text_centered(
                    control_left,
                    control_width,
                    tool_top + 10 * scale,
                    label,
                    213,
                    231,
                    241,
                    1,
                );
            }
            let sidebar_w = browser_width * 27 / 100;
            self.fill_rect_alpha(
                browser_left,
                tool_top + 38 * scale,
                sidebar_w,
                browser_height.saturating_sub(title_h + 38 * scale),
                5,
                18,
                31,
                216,
            );
            self.ui_text(
                browser_left + 14 * scale,
                tool_top + 51 * scale,
                b"FAVORITES",
                90,
                191,
                230,
                1,
            );
            for (index, item) in [
                b"Home".as_slice(),
                b"Personal Space",
                b"Documents",
                b"Downloads",
                b"Pictures",
                b"Music",
                b"Videos",
                b"Projects",
                b"Trash",
                b"",
                b"DEVICES",
                b"Infinity Storage",
                b"",
            ]
            .iter()
            .enumerate()
            {
                let item_y = tool_top + (72 + index * 20) * scale;
                if index == home_location {
                    self.fill_rect_alpha(
                        browser_left + 7,
                        item_y - 3,
                        sidebar_w - 14,
                        20 * scale,
                        selection_r,
                        selection_g,
                        selection_b,
                        218,
                    );
                }
                if !item.is_empty() && index != 10 {
                    let icon_kind = match index {
                        0 => 2,
                        1 => 8,
                        2 => 5,
                        3 => 4,
                        4 => 13,
                        5 => 7,
                        6 => 11,
                        7 => 8,
                        8 => 9,
                        11 | 12 => 11,
                        _ => 0,
                    };
                    self.authentication_icon(
                        browser_left + 16 * scale,
                        item_y + 8 * scale,
                        icon_kind,
                        13 * scale,
                        index == home_location,
                    );
                }
                self.ui_text(
                    browser_left + (if index == 10 { 16 } else { 30 }) * scale,
                    item_y,
                    item,
                    if index == 10 { 90 } else { 204 },
                    if index == 10 { 191 } else { 224 },
                    236,
                    1,
                );
            }
            let grid_x = browser_left + sidebar_w + 28 * scale;
            let grid_y = tool_top + 58 * scale;
            let gap = (browser_width.saturating_sub(sidebar_w + 55 * scale)) / 4;
            let tile_step = (self.height / 23).max(34) + 40 * scale;
            let active_path = navigator_state
                .map(|state| state.active_namespace_ref)
                .unwrap_or_else(|| {
                    crate::runtime::object_navigation::ByteText::new(b"/home/default").unwrap()
                });
            let object_count =
                crate::storage::namespace_child_count(active_path.as_bytes()).unwrap_or(0);
            let child_count = object_count.saturating_add(
                crate::runtime::object_navigation::FILE_NAVIGATOR_NAVIGATION_ENTRY_COUNT,
            );
            let selected_index = navigator_state
                .map(|state| state.selected_index)
                .unwrap_or(crate::runtime::object_navigation::FILE_NAVIGATOR_NO_SELECTION);
            let scroll_offset = navigator_state
                .map(|state| state.scroll_offset)
                .unwrap_or(0);
            if navigator_list_view {
                self.ui_text_strong(
                    grid_x,
                    grid_y.saturating_sub(22 * scale),
                    b"NAME",
                    132,
                    180,
                    207,
                    1,
                );
                self.ui_text_strong(
                    grid_x + gap * 2,
                    grid_y.saturating_sub(22 * scale),
                    b"KIND",
                    132,
                    180,
                    207,
                    1,
                );
                self.ui_text_strong(
                    grid_x + gap * 3,
                    grid_y.saturating_sub(22 * scale),
                    b"SIZE",
                    132,
                    180,
                    207,
                    1,
                );
            }
            let viewport = browser_height.saturating_sub(150 * scale);
            let extent = if navigator_list_view {
                34 * scale
            } else {
                tile_step
            };
            let (first, end) = crate::runtime::object_navigation::FileNavigatorState::visible_range(
                child_count,
                scroll_offset,
                viewport,
                extent,
            );
            for index in first..end {
                let navigation_name: Option<&[u8]> = match index {
                    0 => Some(b"."),
                    1 => Some(b".."),
                    _ => None,
                };
                let entry = if navigation_name.is_none() {
                    crate::storage::namespace_child_nth_sorted(
                        active_path.as_bytes(),
                        index.saturating_sub(
                            crate::runtime::object_navigation::FILE_NAVIGATOR_NAVIGATION_ENTRY_COUNT,
                        ),
                        navigator_state
                            .map(|state| state.sort_descending)
                            .unwrap_or(false),
                    )
                    .ok()
                    .flatten()
                } else {
                    None
                };
                if navigation_name.is_none() && entry.is_none() {
                    continue;
                }
                let path = entry
                    .as_ref()
                    .map(|value| &value.path[..value.path_len as usize]);
                let metadata = path
                    .and_then(|value| crate::storage::object_inspect_path(value).ok())
                    .map(|value| value.0);
                let kind = if navigation_name.is_some() {
                    Some(crate::storage::object::ObjectType::NamespaceNode)
                } else {
                    metadata.map(|value| value.kind)
                };
                let icon_role = if kind == Some(crate::storage::object::ObjectType::NamespaceNode) {
                    3
                } else {
                    4
                };
                let base_name = navigation_name.unwrap_or_else(|| {
                    crate::runtime::object_navigation::namespace_basename(path.unwrap_or(b"/"))
                });
                let name = if navigator_state
                    .map(|state| state.rename_editing && state.selected_index as usize == index)
                    .unwrap_or(false)
                {
                    navigator_state
                        .as_ref()
                        .map(|state| state.editor_text.as_bytes())
                        .unwrap_or(base_name)
                } else {
                    base_name
                };
                let column = index % 4;
                let row = (index - first) / 4;
                if navigator_list_view {
                    let row_y = grid_y + (index - first) * 34 * scale;
                    self.fill_rect_alpha(
                        grid_x.saturating_sub(18 * scale),
                        row_y.saturating_sub(7 * scale),
                        browser_width.saturating_sub(sidebar_w + 48 * scale),
                        30 * scale,
                        8,
                        28,
                        44,
                        if index % 2 == 0 { 170 } else { 105 },
                    );
                    if selected_index as usize == index {
                        self.fill_rounded_rect_alpha(
                            grid_x.saturating_sub(18 * scale),
                            row_y.saturating_sub(7 * scale),
                            browser_width.saturating_sub(sidebar_w + 48 * scale),
                            30 * scale,
                            6 * scale,
                            selection_r,
                            selection_g,
                            selection_b,
                            190,
                        );
                    }
                    let _ = self.themed_icon(grid_x, row_y + 7 * scale, icon_role, 22 * scale);
                    self.ui_text(grid_x + 22 * scale, row_y, name, 215, 229, 238, 1);
                    let kind_name = match kind {
                        Some(crate::storage::object::ObjectType::NamespaceNode) => {
                            b"Folder".as_slice()
                        }
                        Some(crate::storage::object::ObjectType::Text) => {
                            b"Text document".as_slice()
                        }
                        Some(crate::storage::object::ObjectType::Project) => b"Project".as_slice(),
                        Some(crate::storage::object::ObjectType::Collection) => {
                            b"Collection".as_slice()
                        }
                        _ => b"Object".as_slice(),
                    };
                    self.ui_text(grid_x + gap * 2, row_y, kind_name, 164, 190, 205, 1);
                    let mut size_text = [0u8; 16];
                    let size_len = navigator_decimal(
                        &mut size_text,
                        metadata
                            .map(|value| value.logical_size as usize)
                            .unwrap_or(0),
                    );
                    self.ui_text(
                        grid_x + gap * 3,
                        row_y,
                        &size_text[..size_len],
                        164,
                        190,
                        205,
                        1,
                    );
                } else {
                    let center_x = grid_x + column * gap;
                    let center_y = grid_y + row * tile_step;
                    if selected_index as usize == index {
                        self.fill_rounded_rect_alpha(
                            center_x.saturating_sub(6 * scale),
                            center_y.saturating_sub(8 * scale),
                            gap.max(44 * scale),
                            tile_step.max(54 * scale),
                            8 * scale,
                            selection_r,
                            selection_g,
                            selection_b,
                            190,
                        );
                    }
                    let _ = self.themed_icon(center_x, center_y, icon_role, 52 * scale);
                    self.ui_text_centered(
                        center_x.saturating_sub(gap / 2),
                        gap,
                        center_y + 34 * scale,
                        name,
                        215,
                        229,
                        238,
                        1,
                    );
                }
            }
            let status_top = browser_top + browser_height.saturating_sub(24 * scale);
            self.fill_rect_alpha(
                browser_left + sidebar_w,
                status_top,
                browser_width.saturating_sub(sidebar_w),
                24 * scale,
                4,
                15,
                27,
                220,
            );
            let mut count_text = [0u8; 24];
            let count_len = navigator_decimal(&mut count_text, object_count);
            self.ui_text(
                browser_left + sidebar_w + 14 * scale,
                status_top + 5 * scale,
                &count_text[..count_len],
                142,
                185,
                210,
                1,
            );
            self.ui_text(
                browser_left + sidebar_w + (14 + count_len * 8) * scale,
                status_top + 5 * scale,
                if object_count == 1 {
                    b" item"
                } else {
                    b" items"
                },
                142,
                185,
                210,
                1,
            );
            if dragging_item == Some(6) {
                self.ui_text(
                    browser_left + sidebar_w + 28 * scale,
                    browser_top + browser_height.saturating_sub(28 * scale),
                    b"DROP NOTES.TXT ON A SIDEBAR LOCATION",
                    91,
                    211,
                    250,
                    1,
                );
            }
            if let Some(context) = navigator_state.filter(|state| state.context_menu_open) {
                let menu = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                    .file_navigator_context_geometry(context.context_x, context.context_y);
                let menu_left = menu.x.max(0) as usize;
                let menu_top = menu.y.max(0) as usize;
                let object_menu = context.context_item
                    != crate::runtime::object_navigation::FILE_NAVIGATOR_NO_SELECTION;
                let labels: [&[u8]; 4] = if object_menu {
                    [b"Open", b"Rename", b"Duplicate", b"Move to Trash"]
                } else {
                    [b"New Folder", b"List View", b"Grid View", b"Sort by Name"]
                };
                self.fill_rounded_rect_alpha(
                    menu_left,
                    menu_top,
                    190 * scale,
                    120 * scale,
                    8 * scale,
                    5,
                    18,
                    31,
                    246,
                );
                self.outline_rounded_rect(
                    menu_left,
                    menu_top,
                    190 * scale,
                    148 * scale,
                    8 * scale,
                    73,
                    180,
                    229,
                );
                for (index, label) in labels.iter().enumerate() {
                    let row_top = menu_top + (6 + index * 28) * scale;
                    self.ui_text(
                        menu_left + 14 * scale,
                        row_top + 5 * scale,
                        label,
                        215,
                        231,
                        241,
                        1,
                    );
                }
            }
        }

        self.desktop_dock(scale, launcher_open);
    }

    // ------------------------=
    // FUNC: desktop_widgets
    // DESC: Renders the persistent right-side system overview and AI status foreground layer.
    // ------------------=
    fn desktop_widgets(&mut self, scale: usize) {
        let (accent_r, accent_g, accent_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Focus);
        let geometry = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
            .desktop_foreground_geometry();
        let widget_left = geometry.widgets.x.max(0) as usize;
        let widget_width = geometry.widgets.width as usize;
        let overview_top = geometry.widgets.y.max(0) as usize;
        let overview_height = (330 * scale).min(self.height * 30 / 100);
        self.glass_panel(
            widget_left,
            overview_top,
            widget_width,
            overview_height,
            false,
        );
        self.ui_text(
            widget_left + 18 * scale,
            overview_top + 16 * scale,
            b"SYSTEM OVERVIEW",
            accent_r,
            accent_g,
            accent_b,
            1,
        );
        let machine = crate::runtime::with_runtime(|runtime| runtime.identity.machine()).flatten();
        let machine_name = machine
            .map(|value| value.display_name)
            .unwrap_or(crate::runtime::identity::ShortText::empty());
        for (index, (label, value)) in [
            (b"Machine Name".as_slice(), machine_name.as_bytes()),
            (b"System Generation", b"Active".as_slice()),
            (b"Runtime", b"Online".as_slice()),
            (b"Storage Service", b"Ready".as_slice()),
            (b"Input", b"Ready".as_slice()),
            (b"Appearance", b"Default Dark".as_slice()),
        ]
        .iter()
        .enumerate()
        {
            let row_y = overview_top + (48 + index * 34) * scale;
            self.ui_text(widget_left + 18 * scale, row_y, label, 158, 174, 190, 1);
            let value_width = self.ui_text_width(value, 1);
            self.ui_text_strong(
                widget_left + widget_width.saturating_sub(value_width + 18 * scale),
                row_y,
                value,
                223,
                234,
                242,
                1,
            );
            if matches!(index, 2 | 3 | 4) {
                self.fill_rounded_rect_alpha(
                    widget_left + widget_width.saturating_sub(value_width + 30 * scale),
                    row_y + 7 * scale,
                    6 * scale,
                    6 * scale,
                    3 * scale,
                    73,
                    210,
                    122,
                    255,
                );
            }
        }
        self.desktop_ai_chat(scale);
    }

    // ------------------------=
    // FUNC: desktop_ai_chat
    // DESC: Renders the persistent model-selectable AI status and conversation surface.
    // ------------------=
    fn desktop_ai_chat(&mut self, scale: usize) {
        let chat = crate::runtime::ai::with_ai_runtime(|runtime| runtime.chat);
        if !chat.enabled() {
            return;
        }
        let layout = crate::ui::system_layout::SystemLayout::new(self.width, self.height);
        let geometry = layout.ai_chat_geometry(chat.minimized());
        let left = geometry.panel.x.max(0) as usize;
        let top = geometry.panel.y.max(0) as usize;
        let width = geometry.panel.width as usize;
        let height = geometry.panel.height as usize;
        let (accent_r, accent_g, accent_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Focus);
        self.glass_panel(left, top, width, height, false);
        self.ui_text_strong(
            left + 16 * scale,
            top + 15 * scale,
            b"AI CHAT",
            accent_r,
            accent_g,
            accent_b,
            1,
        );
        let state = if chat.minimized() {
            b"LOCAL  +".as_slice()
        } else {
            b"LOCAL  READY".as_slice()
        };
        let state_width = self.ui_text_width(state, 1);
        self.ui_text(
            left + width.saturating_sub(state_width + 72 * scale),
            top + 15 * scale,
            state,
            139,
            184,
            207,
            1,
        );
        self.ui_text(
            geometry.minimize.x.max(0) as usize + 5 * scale,
            geometry.minimize.y.max(0) as usize + 2 * scale,
            if chat.minimized() { b"+" } else { b"-" },
            225,
            238,
            246,
            1,
        );
        self.ui_text(
            geometry.close.x.max(0) as usize + 4 * scale,
            geometry.close.y.max(0) as usize + 2 * scale,
            b"x",
            225,
            238,
            246,
            1,
        );
        if chat.minimized() {
            return;
        }
        let model = chat.selected_model_descriptor();
        let model_left = geometry.model.x.max(0) as usize;
        let model_top = geometry.model.y.max(0) as usize;
        self.fill_rounded_rect_alpha(
            model_left,
            model_top,
            geometry.model.width as usize,
            geometry.model.height as usize,
            9 * scale,
            7,
            25,
            40,
            226,
        );
        self.outline_rounded_rect(
            model_left,
            model_top,
            geometry.model.width as usize,
            geometry.model.height as usize,
            9 * scale,
            accent_r / 2,
            accent_g / 2,
            accent_b / 2,
        );
        self.ui_text(
            model_left + 12 * scale,
            model_top + 5 * scale,
            b"MODEL",
            123,
            151,
            170,
            1,
        );
        self.ui_text_strong(
            model_left + 12 * scale,
            model_top + 21 * scale,
            model.name,
            224,
            235,
            243,
            1,
        );
        self.ui_text(
            model_left + geometry.model.width as usize - 18 * scale,
            model_top + 15 * scale,
            b">",
            accent_r,
            accent_g,
            accent_b,
            1,
        );
        let timeline_left = geometry.timeline.x.max(0) as usize;
        let timeline_top = geometry.timeline.y.max(0) as usize;
        let timeline_width = geometry.timeline.width as usize;
        let timeline_height = geometry.timeline.height as usize;
        self.fill_rounded_rect_alpha(
            timeline_left,
            timeline_top,
            timeline_width,
            timeline_height,
            9 * scale,
            2,
            12,
            22,
            190,
        );
        if chat.message_count() == 0 {
            self.ui_text_strong(
                timeline_left + 14 * scale,
                timeline_top + 18 * scale,
                b"How can I help with InfinityOS?",
                218,
                232,
                241,
                1,
            );
            self.ui_text(
                timeline_left + 14 * scale,
                timeline_top + 44 * scale,
                b"Ask about this system, storage,",
                137,
                160,
                178,
                1,
            );
            self.ui_text(
                timeline_left + 14 * scale,
                timeline_top + 62 * scale,
                b"devices, memory, or networking.",
                137,
                160,
                178,
                1,
            );
        } else {
            let bubble_width = timeline_width.saturating_sub(35 * scale);
            let text_width = bubble_width.saturating_sub(20 * scale);
            let line_height = (UI_FONT_CELL_HEIGHT + 4) * scale;
            let max_lines = (timeline_height.saturating_sub(27 * scale) / line_height).max(1);
            let mut start = chat.message_count();
            let mut used_height = 7 * scale;
            while start > 0 {
                let Some(message) = chat.message(start - 1) else {
                    break;
                };
                let lines = self
                    .ui_text_wrapped_line_count(text_width, message.text(), max_lines)
                    .max(1);
                let bubble_height = lines * line_height + 12 * scale;
                let row_height = bubble_height + 8 * scale;
                if used_height + row_height > timeline_height && start < chat.message_count() {
                    break;
                }
                used_height += row_height;
                start -= 1;
            }
            let mut row_top = timeline_top + 7 * scale;
            for index in start..chat.message_count() {
                let Some(message) = chat.message(index) else {
                    continue;
                };
                let user = message.role == crate::runtime::ai::chat::ChatRole::User;
                let inset = if user { 28 * scale } else { 7 * scale };
                let lines = self
                    .ui_text_wrapped_line_count(text_width, message.text(), max_lines)
                    .max(1);
                let bubble_height = lines * line_height + 12 * scale;
                self.fill_rounded_rect_alpha(
                    timeline_left + inset,
                    row_top,
                    bubble_width,
                    bubble_height,
                    8 * scale,
                    if user { accent_r / 3 } else { 8 },
                    if user { accent_g / 3 } else { 27 },
                    if user { accent_b / 3 } else { 42 },
                    224,
                );
                self.ui_text_wrapped(
                    timeline_left + inset + 10 * scale,
                    row_top + 8 * scale,
                    text_width,
                    message.text(),
                    218,
                    231,
                    240,
                    max_lines,
                );
                row_top += bubble_height + 8 * scale;
            }
        }
        let composer_left = geometry.composer.x.max(0) as usize;
        let composer_top = geometry.composer.y.max(0) as usize;
        self.fill_rounded_rect_alpha(
            composer_left,
            composer_top,
            geometry.composer.width as usize,
            geometry.composer.height as usize,
            9 * scale,
            3,
            16,
            28,
            236,
        );
        self.outline_rounded_rect(
            composer_left,
            composer_top,
            geometry.composer.width as usize,
            geometry.composer.height as usize,
            9 * scale,
            accent_r / 2,
            accent_g / 2,
            accent_b / 2,
        );
        let composer_text = chat.input();
        self.ui_text(
            composer_left + 12 * scale,
            composer_top + 15 * scale,
            if composer_text.is_empty() {
                b"Ask InfinityOS..."
            } else {
                composer_text
            },
            if composer_text.is_empty() { 130 } else { 224 },
            if composer_text.is_empty() { 151 } else { 235 },
            if composer_text.is_empty() { 168 } else { 243 },
            1,
        );
        self.fill_rounded_rect_alpha(
            geometry.send.x.max(0) as usize,
            geometry.send.y.max(0) as usize,
            geometry.send.width as usize,
            geometry.send.height as usize,
            9 * scale,
            accent_r / 2,
            accent_g / 2,
            accent_b / 2,
            238,
        );
        self.ui_text_strong(
            geometry.send.x.max(0) as usize + 16 * scale,
            geometry.send.y.max(0) as usize + 15 * scale,
            b"Send",
            239,
            247,
            252,
            1,
        );
    }

    // ------------------------=
    // FUNC: desktop_dock
    // DESC: Renders the persistent foreground application dock and launcher state.
    // ------------------=
    fn desktop_dock(&mut self, scale: usize, launcher_open: bool) {
        let geometry = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
            .desktop_foreground_geometry();
        let dock_width = geometry.dock.width as usize;
        let dock_height = geometry.dock.height as usize;
        let dock_left = geometry.dock.x.max(0) as usize;
        let dock_top = geometry.dock.y.max(0) as usize;
        self.glass_panel(dock_left, dock_top, dock_width, dock_height, false);
        let (dock_r, dock_g, dock_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Dock);
        let dock_alpha = self.active_background_alpha(104);
        self.fill_rounded_rect_alpha(
            dock_left + 2 * scale,
            dock_top + 2 * scale,
            dock_width.saturating_sub(4 * scale),
            dock_height.saturating_sub(4 * scale),
            14 * scale,
            dock_r,
            dock_g,
            dock_b,
            dock_alpha,
        );
        let entries = &crate::ui::app_launcher::DESKTOP_DOCK_ENTRIES;
        let icon_gap = dock_width / entries.len();
        for (index, entry) in entries.iter().enumerate() {
            let size = 46 * scale;
            let x = dock_left + icon_gap / 2 + index * icon_gap;
            if index == 0 {
                self.launcher_dock_icon(x, dock_top + 10 * scale, size, launcher_open);
            } else {
                self.desktop_app_icon(
                    x,
                    dock_top + 10 * scale,
                    size,
                    entry.icon_kind,
                    matches!(index, 1 | 2),
                );
            }
            if index == 6 {
                self.fill_rect_alpha(
                    x.saturating_sub(icon_gap / 3),
                    dock_top + 10 * scale,
                    1,
                    size,
                    69,
                    91,
                    108,
                    180,
                );
            }
        }
    }

    // ------------------------=
    // FUNC: system_login_animation
    // DESC: Redraws only a narrow background strip and animated orbit lights on login to avoid full-frame flicker.
    // ------------------=
    pub(super) fn system_login_animation(&mut self, phase: usize) {
        let old_phase = (phase + 382) % 384;
        let radius = (self.height / 180).max(3);
        for index in 0..7usize {
            let old_position = (old_phase + index * 47) % 384;
            let (old_x, old_y) = infinity_point(old_position);
            let old_screen_x =
                self.width as i32 * 75 / 100 + old_x * self.width as i32 * 43 / 22_400;
            let old_screen_y =
                self.height as i32 * 34 / 100 + old_y * self.height as i32 * 30 / 12_400;
            let restore = radius * 5;
            self.paint_authentication_background_rect(
                old_screen_x.saturating_sub(restore as i32).max(0) as usize,
                old_screen_y.saturating_sub(restore as i32).max(0) as usize,
                restore * 2,
                restore * 2,
            );
        }
        for index in 0..7usize {
            let position = (phase + index * 47) % 384;
            let (path_x, path_y) = infinity_point(position);
            let x = self.width as i32 * 75 / 100 + path_x * self.width as i32 * 43 / 22_400;
            let y = self.height as i32 * 34 / 100 + path_y * self.height as i32 * 30 / 12_400;
            let orb_radius = if index == 0 {
                radius
            } else {
                (radius / 2).max(2)
            };
            self.star_orb(
                x,
                y,
                orb_radius as i32,
                (230usize.saturating_sub(index * 25)) as u8,
                index == 0,
            );
        }
    }

    // ------------------------=
    // FUNC: system_ui_input_field
    // DESC: Repaints only the active credential or onboarding field so typing never reblits the full high-resolution scene.
    // ------------------=
    pub(super) fn system_ui_input_field(
        &mut self,
        screen: u8,
        step: usize,
        input: &[u8],
        masked: bool,
    ) {
        if matches!(screen, 5 | 6) {
            let fit = (self.width.saturating_mul(1000) / 1536)
                .min(self.height.saturating_mul(1000) / 1024)
                .max(1);
            let content_width = 1536usize.saturating_mul(fit) / 1000;
            let content_height = 1024usize.saturating_mul(fit) / 1000;
            let offset_x = self.width.saturating_sub(content_width) / 2;
            let offset_y = self.height.saturating_sub(content_height) / 2;
            let card_x = offset_x + 54usize.saturating_mul(fit) / 1000;
            let card_y = offset_y + 123usize.saturating_mul(fit) / 1000;
            let card_w = 521usize.saturating_mul(fit) / 1000;
            let inner_x = card_x + 49usize.saturating_mul(fit) / 1000;
            let inner_w = card_w.saturating_sub(98usize.saturating_mul(fit) / 1000);
            let password_y = card_y + 356usize.saturating_mul(fit) / 1000;
            self.authentication_password_field(
                inner_x,
                password_y,
                inner_w,
                57usize.saturating_mul(fit) / 1000,
                input,
                true,
            );
            return;
        }
        let scale = self.ui_scale().max(1);
        let top_bar = (46 * scale).min(self.height / 12).max(40);
        let card_width = (self.width * 34 / 100).clamp(500, 600 * scale);
        let card_height = (self.height * 68 / 100)
            .clamp(560, 680 * scale)
            .min(self.height.saturating_sub(top_bar + 24));
        let card_left = self.width * 4 / 100;
        let card_top = top_bar + self.height.saturating_sub(top_bar + card_height) / 2;
        let inner_left = card_left + 32 * scale;
        let inner_width = card_width.saturating_sub(64 * scale);
        let content_top = card_top + 94 * scale;
        let body_top = content_top + 132 * scale;
        let placeholder: &[u8] = match step {
            1 => b"InfinityNode",
            2 => b"your-handle",
            3 => b"Display name",
            4 => b"Create a password",
            _ => b"",
        };
        self.onboarding_input_field(
            inner_left,
            body_top + 28 * scale,
            inner_width,
            50 * scale,
            input,
            masked,
            true,
            placeholder,
        );
    }
}

// ------------------------=
// FUNC: navigator_decimal
// DESC: Formats a bounded unsigned File Navigator metadata value for framebuffer text.
// ------------------=
fn navigator_decimal(destination: &mut [u8], mut value: usize) -> usize {
    let mut reversed = [0u8; 20];
    let mut length = 0usize;
    loop {
        reversed[length] = b'0' + (value % 10) as u8;
        length += 1;
        value /= 10;
        if value == 0 || length == reversed.len() {
            break;
        }
    }
    let written = length.min(destination.len());
    for index in 0..written {
        destination[index] = reversed[length - index - 1];
    }
    written
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: system_ui_present
// DESC: Presents first-boot and session GUI surfaces with damage-limited pointer updates.
// ------------------=
pub fn system_ui_present(
    screen: u8,
    step: usize,
    input: &[u8],
    masked: bool,
    focus: usize,
    cursor_x: i32,
    cursor_y: i32,
    validation_error: bool,
    window_x: i32,
    window_y: i32,
    window_width: i32,
    window_height: i32,
    window_visible: bool,
    window_maximized: bool,
    home_location: usize,
    selected_item: Option<usize>,
    dragging_item: Option<usize>,
    note_location: usize,
    desktop_items: u8,
    desktop_item_positions: &[[i32; 2]; 7],
    clock: crate::storage::DateTimeConfiguration,
    settings_window: crate::ui::system_layout::SettingsWindowState,
    menu_kind: usize,
    output_lines: &[[u8; 96]; 6],
    output_lengths: &[usize; 6],
    output_count: usize,
    app_window_x: i32,
    app_window_y: i32,
    app_window_width: i32,
    app_window_height: i32,
    app_window_maximized: bool,
    editor_saved: bool,
    editor_input: &[u8],
    command_input: &[u8],
    editor_window: crate::ui::system_layout::DesktopAppWindowState,
    command_window: crate::ui::system_layout::DesktopAppWindowState,
    editor_scroll_row: usize,
    editor_dialog: u8,
    editor_dialog_input: &[u8],
    editor_dialog_focus: usize,
) {
    unsafe {
        let slot = &raw mut CONSOLE;
        if let Some(console) = (*slot).as_mut() {
            console.system_ui_active = true;
            console.restore_cursor();
            let content = system_content_hash(
                input,
                masked,
                output_lines,
                output_lengths,
                output_count,
                editor_saved,
                editor_input,
                command_input,
                editor_window,
                command_window,
                editor_scroll_row,
                editor_dialog,
                editor_dialog_input,
                editor_dialog_focus,
            ) ^ crate::runtime::ai::with_ai_runtime(|runtime| {
                let hash = runtime.chat.state_hash();
                hash as u32 ^ (hash >> 32) as u32
            });
            let pointer_changed = console.cursor_x != cursor_x || console.cursor_y != cursor_y;
            let focus_changed = console.last_system_focus != focus;
            let clock_changed = console.last_system_clock != clock;
            let icon_theme = console.display.active_icon_theme();
            let icon_theme_changed = console.last_icon_theme != icon_theme;
            let accent_rgb = console.display.active_accent_rgb();
            let accent_changed = crate::ui::redraw::appearance_change_requires_structural_redraw(
                console.last_accent_rgb,
                accent_rgb,
            );
            let primary_rgb = console.display.active_primary_rgb();
            let primary_changed = crate::ui::redraw::appearance_change_requires_structural_redraw(
                console.last_primary_rgb,
                primary_rgb,
            );
            let (background_opacity, background_blur) = console.display.active_background_effects();
            let background_effects_changed = console.last_background_opacity != background_opacity
                || console.last_background_blur != background_blur;
            let file_navigator_state =
                crate::runtime::with_runtime(|runtime| runtime.file_navigator).flatten();
            let file_navigator_changed = console.last_file_navigator_state != file_navigator_state;
            let layout = crate::ui::system_layout::SystemLayout::new(
                console.display.width,
                console.display.height,
            );
            let display_rect = crate::ui::geometry::Rect {
                x: 0,
                y: 0,
                width: console.display.width as u32,
                height: console.display.height as u32,
            };
            let settings_geometry_changed = console.last_settings_window.x != settings_window.x
                || console.last_settings_window.y != settings_window.y
                || console.last_settings_window.width != settings_window.width
                || console.last_settings_window.height != settings_window.height
                || console.last_settings_window.maximized != settings_window.maximized;
            let settings_content_changed = console.last_settings_window.expanded_row
                != settings_window.expanded_row
                || console.last_settings_window.scroll_offset != settings_window.scroll_offset
                || console.last_settings_window.row_count != settings_window.row_count;
            let app_window_geometry_changed = console.last_app_window_x != app_window_x
                || console.last_app_window_y != app_window_y
                || console.last_app_window_width != app_window_width
                || console.last_app_window_height != app_window_height
                || console.last_app_window_maximized != app_window_maximized;
            let bounded_menu_change =
                crate::ui::redraw::desktop_menu_change_requires_bounded_redraw(
                    console.last_system_screen,
                    screen,
                    console.last_system_menu,
                    menu_kind,
                    focus_changed,
                );
            let structural_change_without_window = (!bounded_menu_change
                && (console.last_system_screen != screen
                    || crate::ui::redraw::focus_change_requires_structural_redraw(
                        screen,
                        pointer_changed,
                        focus_changed,
                    )
                    || console.last_system_menu != menu_kind))
                || console.last_system_step != step
                || icon_theme_changed
                || accent_changed
                || primary_changed
                || background_effects_changed
                || console.last_system_validation_error != validation_error
                || console.last_home_window_visible != window_visible
                || console.last_home_window_maximized != window_maximized
                || console.last_home_dragging_item != dragging_item
                || console.last_home_note_location != note_location
                || console.last_desktop_items != desktop_items
                || console.last_desktop_item_positions != *desktop_item_positions
                || crate::ui::redraw::clock_change_requires_structural_redraw(
                    screen,
                    clock_changed,
                )
                || settings_content_changed;
            let window_moved =
                console.last_home_window_x != window_x || console.last_home_window_y != window_y;
            let window_resized = console.last_home_window_width != window_width
                || console.last_home_window_height != window_height;
            let navigator_surface_changed = screen == 2
                && window_visible
                && (file_navigator_changed
                    || console.last_home_location != home_location
                    || console.last_home_selected_item != selected_item);
            let previous_window_rect = console.display.desktop_window_rect(
                console.last_home_window_x,
                console.last_home_window_y,
                console.last_home_window_width,
                console.last_home_window_height,
            );
            let window_move_requires_structural_redraw =
                crate::ui::redraw::desktop_window_move_requires_structural_redraw(
                    screen,
                    window_moved,
                    window_visible,
                    window_maximized,
                );
            let content_changed = console.last_system_content != content;
            let bounded_scene_geometry_change = !structural_change_without_window
                && !content_changed
                && console.last_system_screen == screen
                && navigator_surface_changed
                && !window_moved
                && !window_resized;
            let mut full_surface_redrawn = false;
            if bounded_menu_change
                && !structural_change_without_window
                && !window_moved
                && !window_resized
                && !content_changed
            {
                console.present_desktop_menu(
                    console.last_system_screen,
                    console.last_system_menu,
                    screen,
                    menu_kind,
                    focus,
                    clock,
                );
            } else if bounded_scene_geometry_change {
                let damage = if screen == 2 && (window_moved || window_resized) {
                    let current = console.display.desktop_window_rect(
                        window_x,
                        window_y,
                        window_width,
                        window_height,
                    );
                    crate::ui::system_layout::window_transition_damage(
                        crate::ui::geometry::Rect {
                            x: previous_window_rect.0 as i32,
                            y: previous_window_rect.1 as i32,
                            width: previous_window_rect.2 as u32,
                            height: previous_window_rect.3 as u32,
                        },
                        crate::ui::geometry::Rect {
                            x: current.0 as i32,
                            y: current.1 as i32,
                            width: current.2 as u32,
                            height: current.3 as u32,
                        },
                        display_rect,
                        (16 * layout.scale()) as u32,
                    )
                } else if screen == 2 {
                    let current = console.display.desktop_window_rect(
                        window_x,
                        window_y,
                        window_width,
                        window_height,
                    );
                    crate::ui::geometry::Rect {
                        x: current.0 as i32,
                        y: current.1 as i32,
                        width: current.2 as u32,
                        height: current.3 as u32,
                    }
                } else if screen == 4 {
                    crate::ui::system_layout::window_transition_damage(
                        layout
                            .settings_window_geometry(console.last_settings_window)
                            .window,
                        layout.settings_window_geometry(settings_window).window,
                        display_rect,
                        (16 * layout.scale()) as u32,
                    )
                } else {
                    crate::ui::system_layout::window_transition_damage(
                        layout
                            .desktop_app_window_geometry(
                                console.last_app_window_x,
                                console.last_app_window_y,
                                console.last_app_window_width,
                                console.last_app_window_height,
                                console.last_app_window_maximized,
                            )
                            .window,
                        layout
                            .desktop_app_window_geometry(
                                app_window_x,
                                app_window_y,
                                app_window_width,
                                app_window_height,
                                app_window_maximized,
                            )
                            .window,
                        display_rect,
                        (16 * layout.scale()) as u32,
                    )
                };
                console.display.set_render_clip(
                    damage.x.max(0) as usize,
                    damage.y.max(0) as usize,
                    damage.width as usize,
                    damage.height as usize,
                );
                console.display.system_ui_frame(
                    screen,
                    step,
                    input,
                    masked,
                    focus,
                    validation_error,
                    window_x,
                    window_y,
                    window_width,
                    window_height,
                    window_visible,
                    window_maximized,
                    home_location,
                    selected_item,
                    dragging_item,
                    note_location,
                    desktop_items,
                    desktop_item_positions,
                    clock,
                    settings_window,
                    menu_kind,
                    output_lines,
                    output_lengths,
                    output_count,
                    app_window_x,
                    app_window_y,
                    app_window_width,
                    app_window_height,
                    app_window_maximized,
                    editor_saved,
                    editor_input,
                    command_input,
                    editor_window,
                    command_window,
                    editor_scroll_row,
                    editor_dialog,
                    editor_dialog_input,
                    editor_dialog_focus,
                );
                console.display.clear_render_clip();
            } else if structural_change_without_window
                || window_move_requires_structural_redraw
                || window_resized
                || settings_geometry_changed
                || app_window_geometry_changed
            {
                console.menu_saved = false;
                console.display.system_ui_frame(
                    screen,
                    step,
                    input,
                    masked,
                    focus,
                    validation_error,
                    window_x,
                    window_y,
                    window_width,
                    window_height,
                    window_visible,
                    window_maximized,
                    home_location,
                    selected_item,
                    dragging_item,
                    note_location,
                    desktop_items,
                    desktop_item_positions,
                    clock,
                    settings_window,
                    menu_kind,
                    output_lines,
                    output_lengths,
                    output_count,
                    app_window_x,
                    app_window_y,
                    app_window_width,
                    app_window_height,
                    app_window_maximized,
                    editor_saved,
                    editor_input,
                    command_input,
                    editor_window,
                    command_window,
                    editor_scroll_row,
                    editor_dialog,
                    editor_dialog_input,
                    editor_dialog_focus,
                );
                full_surface_redrawn = true;
            } else if crate::ui::redraw::onboarding_controls_require_repaint(
                screen,
                pointer_changed,
                focus_changed,
            ) {
                console
                    .display
                    .onboarding_focus_controls(step, input, masked, focus);
            } else if crate::ui::redraw::authentication_controls_require_repaint(
                screen,
                pointer_changed,
                focus_changed,
            ) {
                console
                    .display
                    .authentication_focus_controls(screen == 6, step, input, focus);
            } else if screen == 7 && focus_changed && !content_changed {
                console.display.app_launcher_content_update(
                    console.display.ui_scale().max(1),
                    input,
                    focus,
                );
            } else if crate::ui::redraw::desktop_app_content_requires_bounded_redraw(
                screen,
                content_changed,
            ) {
                console.display.desktop_native_app_window(
                    screen,
                    input,
                    output_lines,
                    output_lengths,
                    output_count,
                    app_window_x,
                    app_window_y,
                    app_window_width,
                    app_window_height,
                    app_window_maximized,
                    editor_saved,
                    true,
                    editor_scroll_row,
                    editor_dialog,
                    editor_dialog_input,
                    editor_dialog_focus,
                );
            } else if crate::ui::redraw::desktop_chat_content_requires_bounded_redraw(
                screen,
                content_changed,
            ) {
                let widgets = layout.desktop_foreground_geometry().widgets;
                console.display.set_render_clip(
                    widgets.x.max(0) as usize,
                    widgets.y.max(0) as usize,
                    widgets.width as usize,
                    widgets.height as usize,
                );
                console.display.system_ui_frame(
                    screen,
                    step,
                    input,
                    masked,
                    focus,
                    validation_error,
                    window_x,
                    window_y,
                    window_width,
                    window_height,
                    window_visible,
                    window_maximized,
                    home_location,
                    selected_item,
                    dragging_item,
                    note_location,
                    desktop_items,
                    desktop_item_positions,
                    clock,
                    settings_window,
                    menu_kind,
                    output_lines,
                    output_lengths,
                    output_count,
                    app_window_x,
                    app_window_y,
                    app_window_width,
                    app_window_height,
                    app_window_maximized,
                    editor_saved,
                    editor_input,
                    command_input,
                    editor_window,
                    command_window,
                    editor_scroll_row,
                    editor_dialog,
                    editor_dialog_input,
                    editor_dialog_focus,
                );
                console.display.clear_render_clip();
            } else if content_changed
                && (matches!(screen, 5 | 6) || (screen == 1 && (1..=4).contains(&step)))
            {
                console
                    .display
                    .system_ui_input_field(screen, step, input, masked);
            } else if content_changed {
                console.menu_saved = false;
                console.display.system_ui_frame(
                    screen,
                    step,
                    input,
                    masked,
                    focus,
                    validation_error,
                    window_x,
                    window_y,
                    window_width,
                    window_height,
                    window_visible,
                    window_maximized,
                    home_location,
                    selected_item,
                    dragging_item,
                    note_location,
                    desktop_items,
                    desktop_item_positions,
                    clock,
                    settings_window,
                    menu_kind,
                    output_lines,
                    output_lengths,
                    output_count,
                    app_window_x,
                    app_window_y,
                    app_window_width,
                    app_window_height,
                    app_window_maximized,
                    editor_saved,
                    editor_input,
                    command_input,
                    editor_window,
                    command_window,
                    editor_scroll_row,
                    editor_dialog,
                    editor_dialog_input,
                    editor_dialog_focus,
                );
                full_surface_redrawn = true;
            }
            if !full_surface_redrawn
                && crate::ui::redraw::desktop_clock_requires_bounded_redraw(screen, clock_changed)
            {
                console.display.system_top_bar_clock(clock);
            }
            console.cursor_x = cursor_x;
            console.cursor_y = cursor_y;
            console.save_and_draw_cursor(cursor_x, cursor_y);
            console.last_system_screen = screen;
            console.last_system_step = step;
            console.last_system_focus = focus;
            console.last_system_menu = menu_kind;
            console.last_icon_theme = icon_theme;
            console.last_accent_rgb = accent_rgb;
            console.last_primary_rgb = primary_rgb;
            console.last_background_opacity = background_opacity;
            console.last_background_blur = background_blur;
            console.last_system_content = content;
            console.last_system_validation_error = validation_error;
            console.last_home_window_x = window_x;
            console.last_home_window_y = window_y;
            console.last_home_window_width = window_width;
            console.last_home_window_height = window_height;
            console.last_home_window_visible = window_visible;
            console.last_home_window_maximized = window_maximized;
            console.last_home_location = home_location;
            console.last_home_selected_item = selected_item;
            console.last_home_dragging_item = dragging_item;
            console.last_home_note_location = note_location;
            console.last_file_navigator_state = file_navigator_state;
            console.last_desktop_items = desktop_items;
            console.last_desktop_item_positions = *desktop_item_positions;
            console.last_system_clock = clock;
            console.last_settings_window = settings_window;
            console.last_app_window_x = app_window_x;
            console.last_app_window_y = app_window_y;
            console.last_app_window_width = app_window_width;
            console.last_app_window_height = app_window_height;
            console.last_app_window_maximized = app_window_maximized;
            console.last_editor_saved = editor_saved;
            console.display.present_damage();
        }
    }
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: system_ui_present
// DESC: Provides a text-mode fallback when the graphical session shell is unavailable.
// ------------------=
pub fn system_ui_present(
    _screen: u8,
    _step: usize,
    _input: &[u8],
    _masked: bool,
    _focus: usize,
    _cursor_x: i32,
    _cursor_y: i32,
    _validation_error: bool,
    _window_x: i32,
    _window_y: i32,
    _window_width: i32,
    _window_height: i32,
    _window_visible: bool,
    _window_maximized: bool,
    _home_location: usize,
    _selected_item: Option<usize>,
    _dragging_item: Option<usize>,
    _note_location: usize,
    _desktop_items: u8,
    _desktop_item_positions: &[[i32; 2]; 7],
    _clock: crate::storage::DateTimeConfiguration,
    _settings_window: crate::ui::system_layout::SettingsWindowState,
    _menu_kind: usize,
    _output_lines: &[[u8; 96]; 6],
    _output_lengths: &[usize; 6],
    _output_count: usize,
    _app_window_x: i32,
    _app_window_y: i32,
    _app_window_width: i32,
    _app_window_height: i32,
    _app_window_maximized: bool,
    _editor_saved: bool,
    _editor_input: &[u8],
    _command_input: &[u8],
    _editor_window: crate::ui::system_layout::DesktopAppWindowState,
    _command_window: crate::ui::system_layout::DesktopAppWindowState,
    _editor_scroll_row: usize,
    _editor_dialog: u8,
    _editor_dialog_input: &[u8],
    _editor_dialog_focus: usize,
) {
}

// ------------------------=
// FUNC: parse_leading_u8
// DESC: Reads the bounded numeric prefix used by a Settings value label.
// ------------------=
fn parse_leading_u8(input: &[u8]) -> Option<u8> {
    let mut value = 0u16;
    let mut digits = 0usize;
    for byte in input {
        if !byte.is_ascii_digit() {
            break;
        }
        value = value
            .saturating_mul(10)
            .saturating_add((byte - b'0') as u16);
        digits += 1;
    }
    (digits > 0 && value <= u8::MAX as u16).then_some(value as u8)
}

// ------------------------=
// FUNC: system_content_hash
// DESC: Detects changed GUI content without allocating or storing secret input bytes.
// ------------------=
fn system_content_hash(
    input: &[u8],
    masked: bool,
    output_lines: &[[u8; 96]; 6],
    output_lengths: &[usize; 6],
    output_count: usize,
    editor_saved: bool,
    editor_input: &[u8],
    command_input: &[u8],
    editor_window: crate::ui::system_layout::DesktopAppWindowState,
    command_window: crate::ui::system_layout::DesktopAppWindowState,
    editor_scroll_row: usize,
    editor_dialog: u8,
    editor_dialog_input: &[u8],
    editor_dialog_focus: usize,
) -> u32 {
    let mut hash = if masked {
        0x51ed_271bu32
    } else {
        0x811c_9dc5u32
    };
    for byte in input {
        hash ^= if masked { b'*' } else { *byte } as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    for row in 0..output_count.min(6) {
        for byte in &output_lines[row][..output_lengths[row].min(96)] {
            hash ^= *byte as u32;
            hash = hash.wrapping_mul(0x0100_0193);
        }
    }
    hash ^= editor_saved as u32;
    for byte in editor_input.iter().chain(command_input.iter()) {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    for window in [editor_window, command_window] {
        hash ^= (window.visible as u32) | ((window.maximized as u32) << 1);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash ^= editor_scroll_row as u32;
    hash = hash.wrapping_mul(0x0100_0193);
    hash ^= (editor_dialog as u32) | ((editor_dialog_focus as u32) << 8);
    for byte in editor_dialog_input {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}
