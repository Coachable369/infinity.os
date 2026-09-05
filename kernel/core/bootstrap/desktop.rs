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

impl super::DisplayDevice {
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
            sw(56),
            sw(13),
            7,
            48,
            79,
            if focus == 2 { 248 } else { 226 },
        );
        self.outline_rounded_rect(inner_x, sign_y, inner_w, sw(56), sw(13), 34, 182, 235);
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
            sw(55),
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
            sw(55),
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
        self.fill_rect_alpha(0, 0, self.width, height, 0, 2, 7, 176);
        self.fill_rounded_rect_alpha(
            rail_inset,
            rail_inset,
            rail_width,
            rail_height,
            7 * scale,
            0,
            8,
            18,
            238,
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
            88,
        );
        self.fill_rounded_rect_alpha(
            self.width / 3,
            rail_inset,
            self.width / 2,
            rail_height,
            7 * scale,
            20,
            75,
            112,
            42,
        );
        self.outline_rounded_rect(
            rail_inset,
            rail_inset,
            rail_width,
            rail_height,
            7 * scale,
            64,
            100,
            128,
        );
        self.fill_rect_alpha(
            9 * scale,
            height.saturating_sub(2 * scale),
            self.width.saturating_sub(18 * scale),
            1,
            35,
            113,
            153,
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
                17,
                69,
                101,
                188,
            );
        }
        self.top_bar_infinity_icon(36 * scale, content_y, 68 * scale);
        self.ui_text(76 * scale, text_y, b"I N F I N I T Y O S", 221, 229, 239, 1);

        let menu_positions = [300usize, 360, 420, 482, 610];
        for (index, label) in [b"File".as_slice(), b"Edit", b"View", b"Window", b"Help"]
            .iter()
            .enumerate()
        {
            let menu_x = menu_positions[index] * scale;
            if active_menu == Some(index + 1) {
                let active_width = self.ui_text_width(label, 1) + 18 * scale;
                self.fill_rounded_rect_alpha(
                    menu_x.saturating_sub(9 * scale),
                    4 * scale,
                    active_width,
                    height.saturating_sub(8 * scale),
                    7 * scale,
                    18,
                    55,
                    78,
                    210,
                );
            }
            self.ui_text(menu_x, text_y, label, 213, 222, 231, 1);
        }

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
    // DESC: Draws the live clock inside a recessed glass well aligned to the shared top-bar centerline.
    // ------------------=
    pub(super) fn paint_system_top_bar_clock_well(
        &mut self,
        clock: crate::storage::DateTimeConfiguration,
        height: usize,
        scale: usize,
    ) {
        let well_width = (104 * scale).min(self.width);
        let well_left = self.width.saturating_sub(well_width + 4 * scale);
        let well_top = 4 * scale;
        let well_height = height.saturating_sub(8 * scale);
        let radius = 6 * scale;
        self.fill_rounded_rect_alpha(
            well_left.saturating_sub(scale),
            well_top.saturating_sub(scale),
            well_width.saturating_add(2 * scale),
            well_height.saturating_add(2 * scale),
            radius.saturating_add(scale),
            0,
            1,
            5,
            168,
        );
        self.fill_rounded_rect_alpha(
            well_left,
            well_top,
            well_width,
            well_height,
            radius,
            0,
            7,
            16,
            178,
        );
        self.outline_rounded_rect(
            well_left,
            well_top,
            well_width,
            well_height,
            radius,
            24,
            59,
            82,
        );
        self.fill_rect_alpha(
            well_left + radius,
            well_top + scale,
            well_width.saturating_sub(radius * 2),
            scale,
            0,
            0,
            2,
            190,
        );
        self.fill_rect_alpha(
            well_left + radius,
            well_top + well_height.saturating_sub(2 * scale),
            well_width.saturating_sub(radius * 2),
            scale,
            54,
            121,
            157,
            105,
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
            well_left + well_width.saturating_sub(time_width) / 2,
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
        let width = (112 * scale).min(self.width);
        let left = self.width.saturating_sub(width);
        self.paint_desktop_background_rect(left, 0, width, height);
        self.fill_rect_alpha(left, 0, width, height, 0, 2, 7, 176);
        self.fill_rect_alpha(
            left,
            2 * scale,
            width.saturating_sub(2 * scale),
            height.saturating_sub(4 * scale),
            0,
            8,
            18,
            238,
        );
        self.fill_rect_alpha(
            left,
            2 * scale,
            width.saturating_sub(2 * scale),
            height / 2,
            18,
            39,
            59,
            88,
        );
        self.fill_rect_alpha(
            left,
            height.saturating_sub(2 * scale),
            width.saturating_sub(9 * scale),
            1,
            35,
            113,
            153,
            150,
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
                    20,
                    87,
                    125,
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
    // DESC: Renders six compact progress segments with completed, current, and remaining states.
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
            _ => 5,
        };
        let gap = 8usize;
        let segment_width = width.saturating_sub(gap * 5) / 6;
        for index in 0..6usize {
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
            left + 16,
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
        self.glass_panel(card_left, card_top, card_width, card_height, true);

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
        let button_height = 48 * scale;
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
            let primary_left = inner_left + back_width + 12 * scale;
            self.polished_button(
                primary_left,
                button_top,
                inner_width.saturating_sub(back_width + 12 * scale),
                button_height,
                if step >= 6 {
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
        }
        self.onboarding_actions(step, focus);
    }

    // ------------------------=
    // FUNC: system_ui_frame
    // DESC: Renders onboarding, authentication, desktop, menu, lock, and Settings from shared state.
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
        settings_maximized: bool,
        menu_kind: usize,
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
        if matches!(screen, 2 | 3 | 4 | 7) {
            self.paint_desktop_background();
        } else {
            self.paint_first_boot_background();
        }
        let scale = self.ui_scale().max(1);
        let margin = self.width * 4 / 100;
        let top_bar = self.system_top_bar((screen == 3).then_some(menu_kind), clock);

        if matches!(screen, 2 | 3 | 7) {
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
            self.app_launcher(scale, input, focus);
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
            self.fill_rect_alpha(
                x,
                button_y,
                button_width,
                46 * scale,
                if focus == 1 { 22 } else { 8 },
                if focus == 1 { 75 } else { 28 },
                if focus == 1 { 105 } else { 42 },
                235,
            );
            self.outline_rect(x, button_y, button_width, 46 * scale, 89, 211, 250);
            let label: &[u8] = if screen == 5 {
                b"START SESSION"
            } else if screen == 6 {
                b"UNLOCK"
            } else if step >= 6 {
                b"ENTER INFINITYOS"
            } else {
                b"CONTINUE"
            };
            if smooth_system_text {
                self.ui_text(
                    x + 18 * scale,
                    button_y + 13 * scale,
                    label,
                    246,
                    252,
                    255,
                    1,
                );
            } else {
                self.text_scaled(
                    x + 18 * scale,
                    button_y + 14 * scale,
                    label,
                    246,
                    252,
                    255,
                    scale,
                    true,
                );
            }
        } else if screen == 3 {
            self.system_menu_panel(menu_kind, focus, scale);
        } else if screen == 4 {
            let restored_width = (self.width * 68 / 100)
                .clamp(900, 1200 * scale)
                .min(self.width.saturating_sub(40));
            let restored_height = (self.height * 62 / 100)
                .clamp(560, 760 * scale)
                .min(self.height.saturating_sub(top_bar + 28));
            let (left, top, width, height) = if settings_maximized {
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
            self.glass_panel(left, top, width, height, true);
            let title_height = 54 * scale;
            self.fill_rect_alpha(left, top, width, title_height, 6, 17, 29, 222);
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
            let sections: [&[u8]; 8] = [
                b"General",
                b"Themes & Skins",
                b"Users & Accounts",
                b"AI & Voice",
                b"Privacy & Security",
                b"Devices",
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
                        15,
                        66,
                        100,
                        226,
                    );
                }
                self.authentication_icon(
                    left + 27 * scale,
                    y + 8 * scale,
                    [8usize, 13, 6, 7, 8, 11, 11, 12][index],
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
                sections[focus.min(7)],
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
            let rows: [(&[u8], &[u8]); 5] = match focus.min(7) {
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
                    (b"Accent", b"Infinity Blue"),
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
                    (b"Network", b"Ready"),
                ],
                6 => [
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
                    35,
                    57,
                    74,
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
                let preview_top = content_y + 380 * scale;
                let preview_gap = content_width / 3;
                for theme in 0..3u8 {
                    let preview_left = content_x + theme as usize * preview_gap;
                    let selected = theme == icon_theme as u8;
                    self.fill_rounded_rect_alpha(
                        preview_left + 4 * scale,
                        preview_top,
                        preview_gap.saturating_sub(8 * scale),
                        78 * scale,
                        12 * scale,
                        if selected { 18 } else { 6 },
                        if selected { 75 } else { 24 },
                        if selected { 108 } else { 38 },
                        220,
                    );
                    self.outline_rounded_rect(
                        preview_left + 4 * scale,
                        preview_top,
                        preview_gap.saturating_sub(8 * scale),
                        78 * scale,
                        12 * scale,
                        if selected { 81 } else { 39 },
                        if selected { 210 } else { 64 },
                        if selected { 250 } else { 83 },
                    );
                    self.icon_theme_preview(
                        theme,
                        preview_left + preview_gap / 2,
                        preview_top + 28 * scale,
                        44 * scale,
                    );
                    let name = crate::ui::icon_theme::IconThemeId::from_u8(theme)
                        .unwrap_or(crate::ui::icon_theme::IconThemeId::CrystalBlueGlass)
                        .name();
                    self.ui_text_centered_strong(
                        preview_left,
                        preview_gap,
                        preview_top + 54 * scale,
                        name,
                        if selected { 224 } else { 160 },
                        if selected { 244 } else { 181 },
                        if selected { 252 } else { 194 },
                        1,
                    );
                }
            }
        }
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
        let radius = (width.min(height) / 12).clamp(8, 18);
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
                if strong { 244 } else { 226 },
            );
            self.outline_rounded_rect(left, top, width, height, radius, 122, 145, 166);
            return;
        }
        if self.skin_visual_mode() == 2 {
            self.fill_rounded_rect_alpha(left, top, width, height, radius, 8, 8, 8, 255);
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
            2,
            12,
            24,
            if strong { 232 } else { 204 },
        );
        self.outline_rounded_rect(left, top, width, height, radius, 34, 83, 112);
        if width > 4 && height > 4 {
            let inner_edge = if strong { (20, 61, 82) } else { (9, 35, 54) };
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
        self.paint_app_launcher(scale, query, focus, false);
    }

    // ------------------------=
    // FUNC: app_launcher_content_update
    // DESC: Repaints only the launcher content layers changed by live search or focus movement.
    // ------------------=
    pub(super) fn app_launcher_content_update(&mut self, scale: usize, query: &[u8], focus: usize) {
        self.paint_app_launcher(scale, query, focus, true);
    }

    // ------------------------=
    // FUNC: paint_app_launcher
    // DESC: Composes the full launcher or its bounded mutable content using one shared rendering path.
    // ------------------=
    fn paint_app_launcher(&mut self, scale: usize, query: &[u8], focus: usize, content_only: bool) {
        let geometry = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
            .app_launcher_geometry();
        let panel_left = geometry.panel.x.max(0) as usize;
        let panel_top = geometry.panel.y.max(0) as usize;
        let panel_width = geometry.panel.width as usize;
        let panel_height = geometry.panel.height as usize;
        if content_only {
            let content_left = panel_left + 18 * scale;
            let content_width = panel_width.saturating_sub(36 * scale);
            let mutable_top = (geometry.search.y.max(0) as usize).saturating_sub(3 * scale);
            self.fill_rect_alpha(
                content_left,
                mutable_top,
                content_width,
                (panel_top + panel_height * 70 / 100).saturating_sub(mutable_top),
                2,
                13,
                29,
                255,
            );
            self.fill_rect_alpha(
                content_left,
                geometry.category_top.saturating_sub(5 * scale),
                content_width,
                panel_top
                    .saturating_add(panel_height)
                    .saturating_sub(geometry.category_top + 12 * scale),
                1,
                11,
                25,
                255,
            );
        } else {
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
        }

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
                .saturating_mul(58)
                / 100;
            let well_left = cell_left + geometry.grid_cell_width.saturating_sub(well_size) / 2;
            let well_top = cell_top + 3 * scale;
            self.fill_rounded_rect_alpha(
                well_left,
                well_top,
                well_size,
                well_size,
                13 * scale,
                if selected { 17 } else { 7 },
                if selected { 67 } else { 28 },
                if selected { 105 } else { 49 },
                if selected { 232 } else { 186 },
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
            let _ = self.themed_icon(
                well_left + well_size / 2,
                well_top + well_size / 2,
                entry.icon_role,
                well_size * 68 / 100,
            );
            self.ui_text_centered_strong(
                cell_left,
                geometry.grid_cell_width,
                well_top + well_size + 8 * scale,
                entry.label,
                if selected { 239 } else { 211 },
                if selected { 248 } else { 227 },
                if selected { 255 } else { 239 },
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
                if selected { 17 } else { 7 },
                if selected { 67 } else { 29 },
                if selected { 105 } else { 50 },
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
            let icon_size = geometry.category_height * 36 / 100;
            let _ = self.themed_icon(
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
            if selected {
                self.fill_rounded_rect_alpha(
                    left + width / 2 - 5 * scale,
                    geometry.category_top + geometry.category_height.saturating_sub(4 * scale),
                    10 * scale,
                    3 * scale,
                    2 * scale,
                    116,
                    220,
                    255,
                    255,
                );
            }
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
    // FUNC: copy_framebuffer_rect
    // DESC: Relocates an overlapping framebuffer rectangle using direction-safe row copies.
    // ------------------=
    pub(super) fn copy_framebuffer_rect(
        &mut self,
        source_left: usize,
        source_top: usize,
        destination_left: usize,
        destination_top: usize,
        width: usize,
        height: usize,
    ) {
        if width == 0
            || height == 0
            || (source_left == destination_left && source_top == destination_top)
        {
            return;
        }
        if destination_top > source_top {
            for row in (0..height).rev() {
                unsafe {
                    core::ptr::copy(
                        self.buffer
                            .add((source_top + row) * self.stride + source_left),
                        self.buffer
                            .add((destination_top + row) * self.stride + destination_left),
                        width,
                    );
                }
            }
        } else {
            for row in 0..height {
                unsafe {
                    core::ptr::copy(
                        self.buffer
                            .add((source_top + row) * self.stride + source_left),
                        self.buffer
                            .add((destination_top + row) * self.stride + destination_left),
                        width,
                    );
                }
            }
        }
        self.mark_dirty_rect(destination_left, destination_top, width, height);
    }

    // ------------------------=
    // FUNC: restore_desktop_exposure
    // DESC: Restores only portions of the old window bounds not covered by the relocated window.
    // ------------------=
    pub(super) fn restore_desktop_exposure(
        &mut self,
        old_rect: (usize, usize, usize, usize),
        new_rect: (usize, usize, usize, usize),
    ) {
        let (old_left, old_top, old_width, old_height) = old_rect;
        let (new_left, new_top, new_width, new_height) = new_rect;
        let old_right = old_left + old_width;
        let old_bottom = old_top + old_height;
        let new_right = new_left + new_width;
        let new_bottom = new_top + new_height;
        let overlap_left = old_left.max(new_left);
        let overlap_top = old_top.max(new_top);
        let overlap_right = old_right.min(new_right);
        let overlap_bottom = old_bottom.min(new_bottom);
        if overlap_left >= overlap_right || overlap_top >= overlap_bottom {
            self.paint_desktop_background_rect(old_left, old_top, old_width, old_height);
            return;
        }
        self.paint_desktop_background_rect(
            old_left,
            old_top,
            old_width,
            overlap_top.saturating_sub(old_top),
        );
        self.paint_desktop_background_rect(
            old_left,
            overlap_bottom,
            old_width,
            old_bottom.saturating_sub(overlap_bottom),
        );
        self.paint_desktop_background_rect(
            old_left,
            overlap_top,
            overlap_left.saturating_sub(old_left),
            overlap_bottom - overlap_top,
        );
        self.paint_desktop_background_rect(
            overlap_right,
            overlap_top,
            old_right.saturating_sub(overlap_right),
            overlap_bottom - overlap_top,
        );
    }

    // ------------------------=
    // FUNC: move_desktop_window
    // DESC: Moves the rendered Home window and repairs only newly exposed wallpaper strips.
    // ------------------=
    pub(super) fn move_desktop_window(
        &mut self,
        old_x: i32,
        old_y: i32,
        new_x: i32,
        new_y: i32,
        window_width: i32,
        window_height: i32,
    ) {
        let old_rect = self.desktop_window_rect(old_x, old_y, window_width, window_height);
        let new_rect = self.desktop_window_rect(new_x, new_y, window_width, window_height);
        let width = old_rect.2.min(new_rect.2);
        let height = old_rect.3.min(new_rect.3);
        self.copy_framebuffer_rect(
            old_rect.0, old_rect.1, new_rect.0, new_rect.1, width, height,
        );
        self.restore_desktop_exposure(old_rect, new_rect);
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
        selected_item: Option<usize>,
        dragging_item: Option<usize>,
        note_location: usize,
        desktop_items: u8,
        desktop_item_positions: &[[i32; 2]; 7],
        launcher_open: bool,
    ) {
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
            let title_h = 34 * scale;
            self.fill_rect_alpha(
                browser_left,
                browser_top,
                browser_width,
                title_h,
                7,
                18,
                31,
                225,
            );
            let title_center_y = browser_top + title_h / 2;
            self.small_infinity_mark(browser_left + 20 * scale, title_center_y, 24 * scale);
            self.ui_text_strong(
                browser_left + 38 * scale,
                title_center_y.saturating_sub(UI_FONT_CELL_HEIGHT / 2),
                b"Home",
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
            self.authentication_icon(
                browser_left + 20 * scale,
                tool_top + 19 * scale,
                3,
                15 * scale,
                false,
            );
            self.authentication_icon(
                browser_left + 48 * scale,
                tool_top + 19 * scale,
                4,
                15 * scale,
                false,
            );
            let location_left = browser_left + 72 * scale;
            let location_width = browser_width.saturating_sub(124 * scale);
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
            self.outline_rounded_rect(
                location_left,
                tool_top + 5 * scale,
                location_width,
                28 * scale,
                8 * scale,
                38,
                62,
                81,
            );
            let location_names: [&[u8]; 9] = [
                b"Home",
                b"Personal Space",
                b"Documents",
                b"Downloads",
                b"Pictures",
                b"Music",
                b"Videos",
                b"Projects",
                b"Recycle Bin",
            ];
            self.ui_text(
                location_left + 14 * scale,
                tool_top + 9 * scale,
                location_names[home_location.min(8)],
                193,
                211,
                224,
                1,
            );
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
                b"Recycle Bin",
                b"",
                b"DEVICES",
                b"Infinity Storage",
                b"Backup Drive",
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
                        13,
                        66,
                        105,
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
                if (index < 6 && home_location != 0)
                    || (index == 6 && note_location != home_location)
                {
                    continue;
                }
                let column = index % 4;
                let row = index / 4;
                if selected_item == Some(index) {
                    self.fill_rounded_rect_alpha(
                        grid_x + column * gap.saturating_sub(6 * scale),
                        grid_y + row * tile_step.saturating_sub(8 * scale),
                        gap.max(44 * scale),
                        tile_step.max(54 * scale),
                        8 * scale,
                        17,
                        79,
                        112,
                        190,
                    );
                }
                self.desktop_icon(grid_x + column * gap, grid_y + row * tile_step, name, *kind);
            }
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
        }

        let widget_left = self.width * 76 / 100;
        let widget_width = self.width * 22 / 100;
        let overview_top = self.height * 7 / 100;
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
            77,
            208,
            250,
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
        let ai_top = overview_top + overview_height + 20 * scale;
        let ai_height = (250 * scale).min(self.height * 24 / 100);
        self.glass_panel(widget_left, ai_top, widget_width, ai_height, false);
        self.ui_text(
            widget_left + 18 * scale,
            ai_top + 16 * scale,
            b"AI STATUS",
            77,
            208,
            250,
            1,
        );
        for (index, (label, value)) in [
            (b"Processing".as_slice(), b"Local only".as_slice()),
            (b"Remote access", b"Disabled".as_slice()),
            (b"Voice", b"Permission required".as_slice()),
            (b"Privacy", b"Capability gated".as_slice()),
        ]
        .iter()
        .enumerate()
        {
            let row_y = ai_top + (50 + index * 34) * scale;
            self.ui_text(widget_left + 18 * scale, row_y, label, 158, 174, 190, 1);
            let value_width = self.ui_text_width(value, 1);
            self.ui_text_strong(
                widget_left + widget_width.saturating_sub(value_width + 18 * scale),
                row_y,
                value,
                209,
                225,
                235,
                1,
            );
        }

        let dock_width = self.width * 54 / 100;
        let dock_height = 72 * scale;
        let dock_left = self.width.saturating_sub(dock_width) / 2;
        let dock_top = self.height.saturating_sub(dock_height + 10 * scale);
        self.glass_panel(dock_left, dock_top, dock_width, dock_height, false);
        let icon_gap = dock_width / 9;
        for index in 0..9usize {
            let size = 46 * scale;
            let x = dock_left + icon_gap / 2 + index * icon_gap;
            if index == 0 {
                self.launcher_dock_icon(x, dock_top + 10 * scale, size, launcher_open);
            } else {
                self.desktop_app_icon(
                    x,
                    dock_top + 10 * scale,
                    size,
                    index - 1,
                    matches!(index, 1 | 2),
                );
            }
            if index == 7 {
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
    settings_maximized: bool,
    menu_kind: usize,
) {
    unsafe {
        let slot = &raw mut CONSOLE;
        if let Some(console) = (*slot).as_mut() {
            console.system_ui_active = true;
            console.restore_cursor();
            let content = system_content_hash(input, masked);
            let pointer_changed = console.cursor_x != cursor_x || console.cursor_y != cursor_y;
            let focus_changed = console.last_system_focus != focus;
            let clock_changed = console.last_system_clock != clock;
            let icon_theme = console.display.active_icon_theme();
            let icon_theme_changed = console.last_icon_theme != icon_theme;
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
                || console.last_system_validation_error != validation_error
                || console.last_home_window_visible != window_visible
                || console.last_home_window_maximized != window_maximized
                || console.last_home_location != home_location
                || console.last_home_selected_item != selected_item
                || console.last_home_dragging_item != dragging_item
                || console.last_home_note_location != note_location
                || console.last_desktop_items != desktop_items
                || console.last_desktop_item_positions != *desktop_item_positions
                || crate::ui::redraw::clock_change_requires_structural_redraw(
                    screen,
                    clock_changed,
                )
                || console.last_settings_maximized != settings_maximized;
            let window_moved =
                console.last_home_window_x != window_x || console.last_home_window_y != window_y;
            let window_resized = console.last_home_window_width != window_width
                || console.last_home_window_height != window_height;
            let window_move_requires_structural_redraw =
                crate::ui::redraw::desktop_window_move_requires_structural_redraw(
                    screen,
                    window_moved,
                    window_visible,
                    window_maximized,
                );
            let content_changed = console.last_system_content != content;
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
            } else if window_moved
                && !structural_change_without_window
                && !content_changed
                && !window_move_requires_structural_redraw
                && screen == 2
                && console.last_system_screen == 2
                && window_visible
                && !window_maximized
            {
                console.display.move_desktop_window(
                    console.last_home_window_x,
                    console.last_home_window_y,
                    window_x,
                    window_y,
                    window_width,
                    window_height,
                );
            } else if structural_change_without_window
                || window_move_requires_structural_redraw
                || window_resized
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
                    settings_maximized,
                    menu_kind,
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
            } else if screen == 7 && (content_changed || focus_changed) {
                console.display.app_launcher_content_update(
                    console.display.ui_scale().max(1),
                    input,
                    focus,
                );
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
                    settings_maximized,
                    menu_kind,
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
            console.last_desktop_items = desktop_items;
            console.last_desktop_item_positions = *desktop_item_positions;
            console.last_system_clock = clock;
            console.last_settings_maximized = settings_maximized;
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
    _settings_maximized: bool,
    _menu_kind: usize,
) {
}

// ------------------------=
// FUNC: system_content_hash
// DESC: Detects changed GUI content without allocating or storing secret input bytes.
// ------------------=
fn system_content_hash(input: &[u8], masked: bool) -> u32 {
    let mut hash = if masked {
        0x51ed_271bu32
    } else {
        0x811c_9dc5u32
    };
    for byte in input {
        hash ^= if masked { b'*' } else { *byte } as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}
