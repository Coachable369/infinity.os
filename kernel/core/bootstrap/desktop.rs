//! Authentication, onboarding, desktop shell, menus, settings, windows, and file management.

use super::*;
#[path = "minimized_shelf.rs"]
mod minimized_shelf;
#[path = "desktop_widget_menu.rs"]
mod desktop_widget_menu;
#[path = "glass.rs"]
mod glass;

static mut THINKING_ANIMATION: crate::ui::thinking::ThinkingAnimation =
    crate::ui::thinking::ThinkingAnimation::new();
static mut THINKING_HEADER_DIRTY: bool = false;
static mut LAST_BROWSER_REVISION:u64=0;
static mut LAST_BROWSER_PAGE_KEY:Option<infinity_browser_core::damage::PageKey>=None;

// ------------------------=
// FUNC: thinking_animation_tick
// DESC: Schedules a bounded header repaint on the UI thread while generation is visible.
// ------------------=
pub fn thinking_animation_tick(visible: bool) -> bool {
    let mut active = visible && crate::runtime::ai::with_ai_runtime(|ai|
        ai.chat.enabled() && !ai.chat.minimized()
        && ai.chat.generation_state == crate::runtime::ai::chat::GenerationState::Running);
    #[cfg(target_os="none")]
    { active |= visible && crate::runtime::ai::voice_conversation::state().0 == crate::runtime::ai::voice_conversation::State::Listening; }
    let now = crate::ui::performance::monotonic_ns().unwrap_or(0);
    unsafe {
        let changed = (&mut *(&raw mut THINKING_ANIMATION)).advance(active, now);
        THINKING_HEADER_DIRTY |= changed;
        changed
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const AUTHENTICATION_BMP: &[u8] =
    include_bytes!("../../../assets/desktop/infinity-auth-success-stage-v1.bmp");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const AUTHENTICATION_ORB_BMP: &[u8] =
    include_bytes!("../../../assets/desktop/infinity-auth-success-orb-v1.bmp");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const AUTHENTICATION_RIPPLE_BMP: &[u8] =
    include_bytes!("../../../assets/desktop/infinity-auth-success-ripple-v1.bmp");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const AUTHENTICATION_SPLASH_BMP: &[u8] =
    include_bytes!("../../../assets/desktop/infinity-auth-success-splash-v1.bmp");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const DESKTOP_BMP: &[u8] =
    include_bytes!("../../../assets/desktop/infinity-shell-wallpaper-v3.bmp");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const ONBOARDING_BMP: &[u8] =
    include_bytes!("../../../assets/desktop/infinity-onboarding-wallpaper-v1.bmp");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const TOP_BAR_INFINITY_BMP: &[u8] =
    include_bytes!("../../../assets/desktop/infinity-topbar-icon-v2.bmp");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const WORLD_SHIFT_HERO_BMP: &[u8] =
    include_bytes!("../../../assets/desktop/worldshift-hero-v1.bmp");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const NODE_TRUST_TOPOLOGY_BMP: &[u8] =
    include_bytes!("../../../assets/mesh/infinity-node-trust-topology-v1.bmp");
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
#[cfg(all(
    not(feature = "installer"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const AURORA_HARMONY_BASE_BMP: &[u8] =
    include_bytes!("../../../assets/icons/runtime/aurora-harmony-base.bmp");
#[cfg(all(
    not(feature = "installer"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const AURORA_HARMONY_ACTIONS_BMP: &[u8] =
    include_bytes!("../../../assets/icons/runtime/aurora-harmony-actions.bmp");
#[cfg(all(
    not(feature = "installer"),
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub(super) const AURORA_HARMONY_LAUNCHER_BMP: &[u8] =
    include_bytes!("../../../assets/icons/runtime/aurora-harmony-launcher-256.bmp");

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
            (3, false) => AURORA_HARMONY_BASE_BMP,
            (3, true) => AURORA_HARMONY_ACTIONS_BMP,
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
        #[cfg(feature="native-browser")]
        if role==60 {
            self.paint_bitmap_alpha_fit_rect(include_bytes!("../../../assets/apps/infinity-browser-icon-v1.bmp"),
                center_x.saturating_sub(size/2),center_y.saturating_sub(size/2),size,size);
            return true;
        }
        if (57..=59).contains(&role) {
            self.spatial_identity_icon(center_x, center_y, size, role - 57);
            return true;
        }
        let roles = [0usize, 1, 4, 8, 9, 10, 12, 19, 23, 25, 26, 28, 32, 49];
        let Some(cell) = roles.iter().position(|candidate| *candidate == role) else {
            return self.themed_icon(center_x, center_y, role, size);
        };
        let bitmap = match self.active_icon_theme() {
            1 => LUMINOUS_OBSIDIAN_LAUNCHER_BMP,
            2 => FROSTED_QUARTZ_LAUNCHER_BMP,
            3 => AURORA_HARMONY_LAUNCHER_BMP,
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
            3 => AURORA_HARMONY_BASE_BMP,
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
        if (57..=59).contains(&role) {
            self.spatial_identity_icon(center_x, center_y, size, role - 57);
            true
        } else {
            self.themed_icon(center_x, center_y, role, size)
        }
    }

    // ------------------------=
    // FUNC: spatial_identity_icon
    // DESC: Draws the spatial identity and AI-authored Holographic Desktop and World Shift artwork.
    // ------------------=
    fn spatial_identity_icon(&mut self, center_x: usize, center_y: usize, size: usize, kind: usize) {
        let left = center_x.saturating_sub(size / 2);
        let top = center_y.saturating_sub(size / 2);
        if kind == 1 {
            self.paint_bitmap_alpha_fit_rect(include_bytes!("../../../assets/apps/infinity-holographic-desktop-icon-v1.bmp"),
                left,top,size,size);
            return;
        }
        if kind == 2 {
            self.paint_bitmap_alpha_fit_rect(include_bytes!("../../../assets/apps/infinity-world-shift-icon-v1.bmp"),
                left,top,size,size);
            return;
        }
        let radius = (size / 5).max(5);
        let palette = [(12, 65, 105), (25, 52, 108), (34, 38, 104)][kind.min(2)];
        self.fill_rounded_rect_alpha(left, top, size, size, radius, palette.0, palette.1, palette.2, 248);
        self.outline_rounded_rect(left, top, size, size, radius, 112, 224, 255);
        let stroke: (u8, u8, u8) = (173, 240, 255);
        if kind == 0 {
            for ring in 0..3 {
                let inset = size * (18 + ring * 10) / 100;
                self.outline_rounded_rect(left + inset, top + inset, size - inset * 2, size - inset * 2,
                    size / 2, stroke.0, stroke.1.saturating_sub((ring * 28) as u8), stroke.2);
            }
            self.fill_rounded_rect_alpha(center_x.saturating_sub(size / 12), center_y.saturating_sub(size / 12), size / 6, size / 6, size / 12, 235, 252, 255, 255);
        }
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
        if let Some(bitmap) = super::spatial_view::WORLD_ART.get(crate::ui::spatial::desktop_world() as usize) {
            self.paint_bitmap_cover_rect(bitmap, 0, 0, self.width, self.height);
            return;
        }
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
            _ => {
                self.paint_bitmap_cover_rect(AUTHENTICATION_BMP, 0, 0, self.width, self.height);
                self.authentication_ambient_art();
            }
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
    // FUNC: authentication_art_geometry
    // DESC: Maps authored 1920x1080 animation coordinates through the authentication background's aspect-fill transform.
    // ------------------=
    fn authentication_art_geometry(&self) -> (i32, i32, usize) {
        let scale = (self.width.saturating_mul(1_000) / 1_920)
            .max(self.height.saturating_mul(1_000) / 1_080)
            .max(1);
        let rendered_width = 1_920usize.saturating_mul(scale) / 1_000;
        let rendered_height = 1_080usize.saturating_mul(scale) / 1_000;
        (
            (self.width as i32 - rendered_width as i32) / 2,
            (self.height as i32 - rendered_height as i32) / 2,
            scale,
        )
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: authentication_ambient_art
    // DESC: Reconstructs the resting orb over the generated stage plate whose water rings are already composited.
    // ------------------=
    fn authentication_ambient_art(&mut self) {
        let (offset_x, offset_y, scale) = self.authentication_art_geometry();
        let center_x = offset_x + 1_335 * scale as i32 / 1_000;
        let orb_y = offset_y + 783 * scale as i32 / 1_000;
        let orb_size = (155 * scale / 1_000).max(1);
        self.paint_bitmap_alpha_fit_rect(
            AUTHENTICATION_ORB_BMP,
            center_x.saturating_sub((orb_size / 2) as i32).max(0) as usize,
            orb_y.saturating_sub((orb_size / 2) as i32).max(0) as usize,
            orb_size,
            orb_size,
        );
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: authentication_success_frame_region
    // DESC: Restores and composites one non-interface portion of an authentication-success keyframe.
    // ------------------=
    fn authentication_success_frame_region(
        &mut self,
        presentation: crate::ui::authentication_motion::Presentation,
        left: usize,
        top: usize,
        right: usize,
        bottom: usize,
        offset_y: i32,
        scale: usize,
        center_x: i32,
        water_y: i32,
    ) {
        if left >= right || top >= bottom {
            return;
        }
        self.set_render_clip(
            left,
            top,
            right.saturating_sub(left),
            bottom.saturating_sub(top),
        );
        self.paint_authentication_background_rect(
            left,
            top,
            right.saturating_sub(left),
            bottom.saturating_sub(top),
        );
        let base_ripple_width = (768 * scale / 1_000).max(1);
        let base_ripple_height = (344 * scale / 1_000).max(1);
        for (wave_scale, opacity) in [
            (
                presentation.primary_ripple_scale,
                presentation.primary_ripple_opacity,
            ),
            (
                presentation.secondary_ripple_scale,
                presentation.secondary_ripple_opacity,
            ),
        ] {
            if opacity == 0 {
                continue;
            }
            let width = (base_ripple_width * wave_scale as usize / 1_000).max(1);
            let height = (base_ripple_height * wave_scale as usize / 1_000).max(1);
            self.paint_bitmap_alpha_fit_rect_opacity(
                AUTHENTICATION_RIPPLE_BMP,
                center_x.saturating_sub((width / 2) as i32).max(0) as usize,
                water_y.saturating_sub((height / 2) as i32).max(0) as usize,
                width,
                height,
                opacity,
            );
        }

        if presentation.orb_opacity != 0 {
            let base_orb = (155 * scale / 1_000).max(1);
            let width = (base_orb * presentation.orb_width_per_mille as usize / 1_000).max(1);
            let height = (base_orb * presentation.orb_height_per_mille as usize / 1_000).max(1);
            let orb_y = offset_y
                + presentation.orb_y_per_mille as i32 * 1_080 * scale as i32 / 1_000_000;
            self.paint_bitmap_alpha_fit_rect_opacity(
                AUTHENTICATION_ORB_BMP,
                center_x.saturating_sub((width / 2) as i32).max(0) as usize,
                orb_y.saturating_sub((height / 2) as i32).max(0) as usize,
                width,
                height,
                presentation.orb_opacity,
            );
        }

        if presentation.splash_opacity != 0 {
            let base_width = (270 * scale / 1_000).max(1);
            let base_height = (165 * scale / 1_000).max(1);
            let width = (base_width * presentation.splash_scale as usize / 1_000).max(1);
            let height = (base_height * presentation.splash_scale as usize / 1_000).max(1);
            self.paint_bitmap_alpha_fit_rect_opacity(
                AUTHENTICATION_SPLASH_BMP,
                center_x.saturating_sub((width / 2) as i32).max(0) as usize,
                water_y.saturating_sub((height * 3 / 5) as i32).max(0) as usize,
                width,
                height,
                presentation.splash_opacity,
            );
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: authentication_success_frame
    // DESC: Composites one orb-drop and dual-ripple keyframe while retaining the card and utility tray above the water.
    // ------------------=
    fn authentication_success_frame(
        &mut self,
        presentation: crate::ui::authentication_motion::Presentation,
    ) {
        if self.skin_visual_mode() != 0 {
            return;
        }
        let (offset_x, offset_y, scale) = self.authentication_art_geometry();
        let source_left = offset_x
            + crate::ui::authentication_motion::DAMAGE_LEFT as i32 * scale as i32 / 1_000;
        let source_top = offset_y
            + crate::ui::authentication_motion::DAMAGE_TOP as i32 * scale as i32 / 1_000;
        let source_right = offset_x
            + crate::ui::authentication_motion::DAMAGE_RIGHT as i32 * scale as i32 / 1_000;
        let source_bottom = offset_y
            + crate::ui::authentication_motion::DAMAGE_BOTTOM as i32 * scale as i32 / 1_000;
        let left = source_left.max(0) as usize;
        let top = source_top.max(0) as usize;
        let right = source_right.max(0).min(self.width as i32) as usize;
        let bottom = source_bottom.max(0).min(self.height as i32) as usize;
        let center_x = offset_x
            + crate::ui::authentication_motion::ARTWORK_CENTER_X as i32 * scale as i32 / 1_000;
        let water_y = offset_y
            + crate::ui::authentication_motion::ARTWORK_WATER_Y as i32 * scale as i32 / 1_000;

        let fit = (self.width.saturating_mul(1_000) / 1_536)
            .min(self.height.saturating_mul(1_000) / 1_024)
            .max(1);
        let content_width = 1_536usize.saturating_mul(fit) / 1_000;
        let content_height = 1_024usize.saturating_mul(fit) / 1_000;
        let ui_offset_x = self.width.saturating_sub(content_width) / 2;
        let ui_offset_y = self.height.saturating_sub(content_height) / 2;
        let sx = |value: usize| ui_offset_x + value.saturating_mul(fit) / 1_000;
        let sy = |value: usize| ui_offset_y + value.saturating_mul(fit) / 1_000;
        let sw = |value: usize| value.saturating_mul(fit) / 1_000;
        let card_left = sx(54);
        let card_top = sy(123);
        let card_right = card_left.saturating_add(sw(521));
        let card_bottom = card_top.saturating_add(sw(754));
        let tray_left = sx(529);
        let tray_top = sy(914);
        let tray_right = tray_left.saturating_add(sw(478));
        let tray_bottom = tray_top.saturating_add(sw(90));

        let regions = [
            (left, top, right, bottom.min(card_top)),
            (left.max(card_right), top.max(card_top), right, bottom.min(card_bottom)),
            (left, top.max(card_bottom), right, bottom.min(tray_top)),
            (left, top.max(tray_top), right.min(tray_left), bottom.min(tray_bottom)),
            (left.max(tray_right), top.max(tray_top), right, bottom.min(tray_bottom)),
            (left, top.max(tray_bottom), right, bottom),
        ];
        for (region_left, region_top, region_right, region_bottom) in regions {
            self.authentication_success_frame_region(
                presentation,
                region_left,
                region_top,
                region_right,
                region_bottom,
                offset_y,
                scale,
                center_x,
                water_y,
            );
        }
        self.clear_render_clip();
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
        if let Some(bitmap) = super::spatial_view::WORLD_ART.get(crate::ui::spatial::desktop_world() as usize) {
            self.paint_bitmap_cover_rect(bitmap, left, top, width, height);
            return;
        }
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
            self.top_bar_infinity_icon(sx(57), top_height / 2, sw(64));
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
        self.top_bar_infinity_icon(card_x + card_w / 2, card_y + sw(91), sw(156));
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
                y + height.saturating_sub(UI_FONT_CELL_HEIGHT) / 2,
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
                y + height.saturating_sub(UI_FONT_CELL_HEIGHT) / 2,
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
        self.text_field_caret(x + height, y, height, input, focused, 1);
    }

    // ------------------------=
    // FUNC: text_field_caret
    // DESC: Draws the shared blinking insertion caret at the active bounded text index.
    // ------------------=
    fn text_field_caret(
        &mut self,
        text_left: usize,
        top: usize,
        height: usize,
        input: &[u8],
        focused: bool,
        kind: usize,
    ) {
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
            112,
            221,
            255,
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
        crate::ui::status_menu::set_today(clock.year, clock.month, clock.day);
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
            height.saturating_sub(UI_FONT_CELL_HEIGHT) / 2,
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
            height.saturating_sub(UI_FONT_CELL_HEIGHT) / 2,
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
        if menu_kind >= 8 {
            self.status_menu_panel(menu_kind, focus, scale);
            return;
        }
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
            top + height.saturating_sub(UI_FONT_CELL_HEIGHT) / 2,
            display,
            color.0,
            color.1,
            color.2,
            1,
        );
        self.text_field_caret(
            left + crate::ui::system_layout::UI_GUTTER * self.ui_scale().max(1),
            top,
            height,
            &shown[..shown_len],
            focused,
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
        if self.configuration_template_screen(step) {
            self.configuration_template_live_content(step, input, masked, focus, validation_error);
            return;
        }
        let scale = self.ui_scale().max(1);
        let top_bar = self.system_identity_bar();
        let fallback_card_width = (self.width * 34 / 100).clamp(500, 600 * scale);
        let fallback_card_height = (self.height * 68 / 100)
            .clamp(560, 680 * scale)
            .min(self.height.saturating_sub(top_bar + 24));
        let fallback_card_left = self.width * 4 / 100;
        let fallback_card_top =
            top_bar + self.height.saturating_sub(top_bar + fallback_card_height) / 2;
        let (card_left, card_top, card_width, card_height) = if let Some(card) =
            crate::ui::installer_layout::configuration_template_rect(
                step,
                crate::ui::installer_template::InstallerTemplateRole::Console,
                self.width,
                self.height,
            ) {
            (card.left, card.top, card.width, card.height)
        } else {
            (
                fallback_card_left,
                fallback_card_top,
                fallback_card_width,
                fallback_card_height,
            )
        };
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
            5 => (b"AI, VOICE & APPEARANCE", b"Local by default", b"Local voice listens after login. You can turn listening off in the AI widget or Settings. Remote processing stays disabled.", b""),
            6 => (b"NETWORK", b"Connect this Infinity Node", b"Choose wired, Wi-Fi, or continue offline. You can change this later.", b""),
            _ => (b"READY", b"Your Infinity begins here", b"Your identity, Personal Space, privacy policy, and Default Dark appearance are ready.", b""),
        };
        let content_top = card_top + 94 * scale;
        self.ui_text_strong(inner_left, content_top, eyebrow, 72, 196, 238, 1);
        let authored_title = crate::ui::installer_layout::configuration_template_text(
            step,
            crate::ui::installer_template::InstallerTemplateRole::Title,
        )
        .unwrap_or(title);
        let authored_description = crate::ui::installer_layout::configuration_template_text(
            step,
            crate::ui::installer_template::InstallerTemplateRole::Body,
        )
        .unwrap_or(description);
        let title_frame = crate::ui::installer_layout::configuration_template_rect(
            step,
            crate::ui::installer_template::InstallerTemplateRole::Title,
            self.width,
            self.height,
        );
        let body_frame = crate::ui::installer_layout::configuration_template_rect(
            step,
            crate::ui::installer_template::InstallerTemplateRole::Body,
            self.width,
            self.height,
        );
        self.ui_text_fit_strong(
            title_frame.map(|frame| frame.left).unwrap_or(inner_left),
            title_frame
                .map(|frame| frame.top)
                .unwrap_or(content_top + 35 * scale),
            title_frame.map(|frame| frame.width).unwrap_or(inner_width),
            authored_title,
            245,
            248,
            251,
            if self.width >= 1500 { 2 } else { 1 },
        );
        self.ui_text_wrapped(
            body_frame.map(|frame| frame.left).unwrap_or(inner_left),
            body_frame
                .map(|frame| frame.top)
                .unwrap_or(content_top + 82 * scale),
            body_frame.map(|frame| frame.width).unwrap_or(inner_width),
            authored_description,
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
            let authored_input = crate::ui::installer_layout::configuration_template_input_rect(
                step,
                self.width,
                self.height,
            );
            let input_left = authored_input.map(|frame| frame.left).unwrap_or(inner_left);
            let input_top = authored_input
                .map(|frame| frame.top)
                .unwrap_or(body_top + 28 * scale);
            let input_width = authored_input
                .map(|frame| frame.width)
                .unwrap_or(inner_width);
            let input_height = authored_input
                .map(|frame| frame.height)
                .unwrap_or(50 * scale);
            let authored_placeholder = crate::ui::installer_layout::configuration_template_text(
                step,
                crate::ui::installer_template::InstallerTemplateRole::Input,
            )
            .filter(|value| !value.is_empty())
            .unwrap_or(placeholder);
            self.ui_text_strong(
                input_left,
                input_top.saturating_sub(28 * scale),
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
                input_left,
                input_top,
                input_width,
                input_height,
                input,
                masked,
                focus == 1,
                authored_placeholder,
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
                input_left,
                input_top + input_height + 10 * scale,
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
    // FUNC: configuration_template_live_content
    // DESC: Binds typed fields, network state, validation, and focus to the authored first-boot scene.
    // ------------------=
    pub(super) fn configuration_template_live_content(
        &mut self,
        step: usize,
        input: &[u8],
        masked: bool,
        focus: usize,
        validation_error: bool,
    ) {
        let input_variable =
            crate::ui::installer_layout::configuration_template_input_variable(step);
        if input_variable != crate::ui::installer_template::InstallerTemplateVariable::None {
            if let Some(frame) = crate::ui::installer_layout::configuration_template_input_rect(
                step,
                self.width,
                self.height,
            ) {
                let placeholder = crate::ui::installer_layout::configuration_template_text(
                    step,
                    crate::ui::installer_template::InstallerTemplateRole::Input,
                )
                .unwrap_or(b"");
                self.onboarding_input_field(
                    frame.left,
                    frame.top,
                    frame.width,
                    frame.height,
                    input,
                    masked,
                    focus == 1,
                    placeholder,
                );
                if validation_error {
                    self.ui_text(
                        frame.left,
                        frame.bottom().saturating_add(8 * self.ui_scale().max(1)),
                        if input_variable
                            == crate::ui::installer_template::InstallerTemplateVariable::Password
                        {
                            b"Use at least eight characters."
                        } else {
                            b"This field is required before continuing."
                        },
                        255,
                        118,
                        126,
                        1,
                    );
                }
            }
        } else if step == 6 {
            self.onboarding_network_rows(0, 0, 0, focus, validation_error);
        }
        self.configuration_template_navigation(step, focus);
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
            Some(value) if value.wired_available && value.wired_link == LinkState::Down => {
                b"Connect a network cable"
            }
            Some(value) if value.wired_available => b"Wired adapter detected",
            _ => b"No wired adapter detected",
        };
        let wireless_detail: &[u8] = match snapshot {
            Some(value) if value.wireless_available && value.wireless_link == LinkState::Up => {
                b"Connected wireless link detected"
            }
            Some(value) if value.wireless_available && value.wireless_link == LinkState::Down => {
                b"Wireless link is not connected"
            }
            Some(value) if value.wireless_available => b"Wireless adapter detected",
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
            let authored = crate::ui::installer_layout::configuration_network_row_rect(
                index,
                self.width,
                self.height,
            );
            if width == 0 && authored.is_none() {
                continue;
            }
            let left = authored.map(|frame| frame.left).unwrap_or(left);
            let width = authored.map(|frame| frame.width).unwrap_or(width);
            let row_top = authored
                .map(|frame| frame.top)
                .unwrap_or(top + index * 58 * scale);
            let row_height = authored.map(|frame| frame.height).unwrap_or(48 * scale);
            let unit = |value: usize| {
                if authored.is_some() {
                    (value * self.height / 1000).max(1)
                } else {
                    value * scale
                }
            };
            let radius = unit(10);
            let inset = unit(12);
            let icon_size = unit(20);
            let label_left = left + unit(42);
            let font_size = unit(14);
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
                row_height,
                radius,
                if is_focused { 9 } else { 5 },
                if is_focused { 44 } else { 20 },
                if is_focused { 68 } else { 34 },
                255,
            );
            self.outline_rounded_rect(
                left,
                row_top,
                width,
                row_height,
                radius,
                if is_focused || is_selected { 55 } else { 31 },
                if is_focused || is_selected { 194 } else { 74 },
                if is_focused || is_selected { 238 } else { 98 },
            );
            self.authentication_icon(
                left + inset + icon_size / 2,
                row_top + row_height / 2,
                *icon,
                icon_size,
                is_selected,
            );
            self.template_text(
                label_left,
                row_top + row_height * 14 / 100,
                label,
                226,
                237,
                245,
                255,
                font_size,
                true,
            );
            self.template_text(
                label_left,
                row_top + row_height * 54 / 100,
                detail,
                133,
                157,
                177,
                255,
                font_size,
                false,
            );
            if is_selected {
                let status: &[u8] = if snapshot
                    .map(|value| value.connectivity != ConnectivityClass::Offline)
                    .unwrap_or(false)
                    && index < 2
                {
                    b"ACTIVE"
                } else {
                    b"SELECTED"
                };
                let status_width = self.template_text_width(status, font_size, false);
                self.template_text(
                    left + width.saturating_sub(inset + status_width),
                    row_top + row_height.saturating_sub(font_size) / 2,
                    status,
                    88,
                    207,
                    244,
                    255,
                    font_size,
                    false,
                );
            }
        }
        if validation_error {
            let last = crate::ui::installer_layout::configuration_network_row_rect(
                2,
                self.width,
                self.height,
            );
            self.ui_text(
                last.map(|frame| frame.left).unwrap_or(left),
                last.map(|frame| frame.bottom() + 8 * scale)
                    .unwrap_or(top + 178 * scale),
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
        let authored_back = crate::ui::installer_layout::configuration_template_rect(
            step,
            crate::ui::installer_template::InstallerTemplateRole::BackButton,
            self.width,
            self.height,
        );
        let authored_primary = crate::ui::installer_layout::configuration_template_rect(
            step,
            crate::ui::installer_template::InstallerTemplateRole::PrimaryButton,
            self.width,
            self.height,
        );
        let primary_label = crate::ui::installer_layout::configuration_template_text(
            step,
            crate::ui::installer_template::InstallerTemplateRole::PrimaryButton,
        )
        .filter(|value| !value.is_empty())
        .unwrap_or(if step >= 7 {
            b"Enter InfinityOS"
        } else {
            b"Continue"
        });
        if step > 0 {
            let back_width = inner_width * 30 / 100;
            let back_left = authored_back.map(|frame| frame.left).unwrap_or(inner_left);
            let back_top = authored_back.map(|frame| frame.top).unwrap_or(button_top);
            let back_width = authored_back.map(|frame| frame.width).unwrap_or(back_width);
            let back_height = authored_back
                .map(|frame| frame.height)
                .unwrap_or(button_height);
            self.polished_button(
                back_left,
                back_top,
                back_width,
                back_height,
                b"Back",
                false,
                focus == 0,
            );
            let fallback_primary_left =
                inner_left + back_width + crate::ui::system_layout::UI_CONTROL_GAP * scale;
            let primary_left = authored_primary
                .map(|frame| frame.left)
                .unwrap_or(fallback_primary_left);
            let primary_top = authored_primary
                .map(|frame| frame.top)
                .unwrap_or(button_top);
            let primary_width = authored_primary
                .map(|frame| frame.width)
                .unwrap_or_else(|| {
                    inner_width.saturating_sub(
                        back_width + crate::ui::system_layout::UI_CONTROL_GAP * scale,
                    )
                });
            let primary_height = authored_primary
                .map(|frame| frame.height)
                .unwrap_or(button_height);
            self.polished_button(
                primary_left,
                primary_top,
                primary_width,
                primary_height,
                primary_label,
                true,
                focus == 1,
            );
        } else {
            let primary_left = authored_primary
                .map(|frame| frame.left)
                .unwrap_or(inner_left);
            let primary_top = authored_primary
                .map(|frame| frame.top)
                .unwrap_or(button_top);
            let primary_width = authored_primary
                .map(|frame| frame.width)
                .unwrap_or(inner_width);
            let primary_height = authored_primary
                .map(|frame| frame.height)
                .unwrap_or(button_height);
            self.polished_button(
                primary_left,
                primary_top,
                primary_width,
                primary_height,
                primary_label,
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
        validation_error: bool,
    ) {
        // Recompose the same authored scene on focus changes so translucent surfaces
        // and live controls cannot leave stale pixels at legacy coordinates.
        if self.configuration_template_screen(step) {
            self.configuration_template_live_content(step, input, masked, focus, validation_error);
            return;
        }
        if crate::ui::installer_layout::configuration_template_input_variable(step)
            != crate::ui::installer_template::InstallerTemplateVariable::None
        {
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
            let authored_placeholder = crate::ui::installer_layout::configuration_template_text(
                step,
                crate::ui::installer_template::InstallerTemplateRole::Input,
            )
            .filter(|value| !value.is_empty())
            .unwrap_or(placeholder);
            let authored_input =
                crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                    .onboarding_input_geometry(step);
            self.onboarding_input_field(
                authored_input
                    .map(|frame| frame.x.max(0) as usize)
                    .unwrap_or(inner_left),
                authored_input
                    .map(|frame| frame.y.max(0) as usize)
                    .unwrap_or(body_top + 28 * scale),
                authored_input
                    .map(|frame| frame.width.max(0) as usize)
                    .unwrap_or(inner_width),
                authored_input
                    .map(|frame| frame.height.max(0) as usize)
                    .unwrap_or(50 * scale),
                input,
                masked,
                focus == 1,
                authored_placeholder,
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
    // DESC: Renders a live native application window over the intact authenticated desktop.
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
        task_manager_selected: usize,
        app_menu: usize,
    ) {
        let task_menu_open = app_menu == 10;
        if !self.recording_surface {
            let rect = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                .desktop_app_window_geometry(
                    window_x,
                    window_y,
                    window_width,
                    window_height,
                    maximized,
                )
                .window;
            self.retained_window(
                (screen.saturating_sub(8) + 1) as usize,
                (
                    rect.x.max(0) as usize,
                    rect.y.max(0) as usize,
                    rect.width as usize,
                    rect.height as usize,
                ),
                |target| {
                    target.desktop_native_app_window(
                        screen,
                        input,
                        output_lines,
                        output_lengths,
                        output_count,
                        window_x,
                        window_y,
                        window_width,
                        window_height,
                        maximized,
                        editor_saved,
                        false,
                        editor_scroll_row,
                        editor_dialog,
                        editor_dialog_input,
                        editor_dialog_focus,
                        task_manager_selected,
                        app_menu,
                    )
                },
            );
            return;
        }
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
        if screen == 9 {
            self.editor_window(geometry, input, editor_scroll_row, editor_saved, maximized, scale);
            if editor_dialog == 3 {
                self.desktop_editor_unsaved_dialog(
                    geometry.content,
                    editor_dialog_input,
                    editor_dialog_focus,
                    scale,
                );
            } else if editor_dialog != 0 {
                self.desktop_editor_dialog(geometry.content, editor_dialog == 2, editor_dialog_input, output_lines, output_lengths, output_count, editor_dialog_focus, scale);
            }
            return;
        }
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
            let icon_role = if screen == 9 {
                49
            } else if screen == 10 {
                19
            } else {
                25
            };
            let _ = self.themed_icon(left + 30 * scale, top + 24 * scale, icon_role, 32 * scale);
            self.ui_text_elided_strong(
                left + 54 * scale,
                top + 16 * scale,
                108 * scale,
                if screen == 9 {
                    b"Text Editor"
                } else if screen == 10 {
                    b"Task Manager"
                } else {
                    b"Command Window"
                },
                231,
                243,
                250,
            );
            self.ui_text(
                left + 174 * scale,
                top + 16 * scale,
                b"File",
                221,
                233,
                241,
                1,
            );
            self.ui_text(
                left + 225 * scale,
                top + 16 * scale,
                b"Performance",
                221,
                233,
                241,
                1,
            );
            for (index, control) in [geometry.minimize, geometry.maximize, geometry.close]
                .iter()
                .enumerate()
            {
                self.window_control(
                    control.x.max(0) as usize,
                    control.y.max(0) as usize,
                    control.width as usize,
                    index,
                    maximized,
                );
            }
            if !maximized {
                let outline =
                    self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
                self.window_resize_affordances(left, top, width, height, scale, outline);
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
            } else if screen == 10 {
                self.fill_rounded_rect_alpha(
                    toolbar_left + 10 * scale,
                    toolbar_top + 6 * scale,
                    94 * scale,
                    geometry.toolbar.height as usize - 12 * scale,
                    7 * scale,
                    if task_menu_open { 29 } else { 8 },
                    if task_menu_open { 91 } else { 31 },
                    if task_menu_open { 132 } else { 48 },
                    240,
                );
                self.ui_text_strong(
                    toolbar_left + 24 * scale,
                    toolbar_top + 15 * scale,
                    b"Task",
                    225,
                    240,
                    249,
                    1,
                );
                self.ui_text(
                    toolbar_left + 79 * scale,
                    toolbar_top + 15 * scale,
                    if task_menu_open { b"^" } else { b"v" },
                    103,
                    211,
                    252,
                    1,
                );
                let live = b"LIVE TELEMETRY  /  1 SEC";
                let live_width = self.ui_text_width(live, 1);
                self.ui_text(
                    toolbar_left + geometry.toolbar.width as usize - live_width - 18 * scale,
                    toolbar_top + 15 * scale,
                    live,
                    107,
                    199,
                    236,
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
        if screen == 10 {
            self.render_task_manager_dashboard(
                geometry,
                task_manager_selected,
                task_menu_open,
                scale,
            );
        } else if content_only {
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
        if screen != 10 {
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
            self.text_field_caret(
                content_left + 74 * scale,
                prompt_y.saturating_sub(8 * scale),
                34 * scale,
                input,
                true,
                1,
            );
        }
        if app_menu == 20 {
            self.desktop_native_performance_menu(left, top, screen, scale);
        }
    }

    // ------------------------=
    // FUNC: window_resize_affordances
    // DESC: Draws bright edge bars and corner brackets for every supported window resize direction.
    // ------------------=
    fn window_resize_affordances(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        scale: usize,
        outline: (u8, u8, u8),
    ) {
        let scale = scale.max(1);
        let (red, green, blue) = outline;
        let thickness = 3 * scale;
        let edge_length = (44 * scale).min(height.saturating_sub(24 * scale));
        let corner_length = (15 * scale).min(width / 4).min(height / 4);
        let right = left.saturating_add(width);
        let bottom = top.saturating_add(height);
        let side_top = top.saturating_add(height.saturating_sub(edge_length) / 2);
        let bottom_left = left.saturating_add(width.saturating_sub(edge_length) / 2);
        self.fill_rounded_rect_alpha(
            left.saturating_add(scale),
            side_top,
            thickness,
            edge_length,
            thickness / 2,
            red,
            green,
            blue,
            238,
        );
        self.fill_rounded_rect_alpha(
            right.saturating_sub(thickness + scale),
            side_top,
            thickness,
            edge_length,
            thickness / 2,
            red,
            green,
            blue,
            238,
        );
        self.fill_rounded_rect_alpha(
            bottom_left,
            bottom.saturating_sub(thickness + scale),
            edge_length,
            thickness,
            thickness / 2,
            red,
            green,
            blue,
            238,
        );
        for x in [
            left.saturating_add(scale),
            right.saturating_sub(corner_length + scale),
        ] {
            self.fill_rect_alpha(
                x,
                top + scale,
                corner_length,
                thickness,
                red,
                green,
                blue,
                225,
            );
            self.fill_rect_alpha(
                x,
                bottom.saturating_sub(thickness + scale),
                corner_length,
                thickness,
                red,
                green,
                blue,
                225,
            );
        }
        for y in [
            top.saturating_add(scale),
            bottom.saturating_sub(corner_length + scale),
        ] {
            self.fill_rect_alpha(
                left + scale,
                y,
                thickness,
                corner_length,
                red,
                green,
                blue,
                225,
            );
            self.fill_rect_alpha(
                right.saturating_sub(thickness + scale),
                y,
                thickness,
                corner_length,
                red,
                green,
                blue,
                225,
            );
        }
    }

    // ------------------------=
    // FUNC: status_menu_panel
    // DESC: Paints actionable status dropdowns and a live-date Gregorian calendar using shared hit geometry.
    // ------------------=
    fn status_menu_panel(&mut self, menu: usize, focus: usize, scale: usize) {
        let layout = crate::ui::system_layout::SystemLayout::new(self.width, self.height);
        let (x, y, width, height, count) = layout.system_menu_geometry(menu);
        self.glass_panel(x, y, width, height, true);
        for (index, (label, _)) in crate::ui::status_menu::items(menu).iter().enumerate() {
            let top = y + (11 + index * 34) * scale;
            if index == focus {
                self.fill_rounded_rect_alpha(
                    x + 7 * scale,
                    top,
                    width.saturating_sub(14 * scale),
                    30 * scale,
                    7 * scale,
                    8,
                    84,
                    126,
                    220,
                );
            }
            self.ui_text(x + 18 * scale, top + 5 * scale, label, 220, 237, 248, 1);
        }
        if !matches!(menu,15|16) {
            return;
        }
        let (year, month, today) = crate::ui::status_menu::month();
        let top = y + (22 + count * 34) * scale;
        let months: [&[u8]; 12] = [
            b"January",
            b"February",
            b"March",
            b"April",
            b"May",
            b"June",
            b"July",
            b"August",
            b"September",
            b"October",
            b"November",
            b"December",
        ];
        let year_text = [
            b'0' + (year / 1000) as u8,
            b'0' + (year / 100 % 10) as u8,
            b'0' + (year / 10 % 10) as u8,
            b'0' + (year % 10) as u8,
        ];
        self.ui_text_strong(
            x + 18 * scale,
            top,
            months[month as usize - 1],
            114,
            210,
            250,
            1,
        );
        self.ui_text(
            x + width.saturating_sub(60 * scale),
            top,
            &year_text,
            218,
            236,
            248,
            1,
        );
        let cell = width.saturating_sub(20 * scale) / 7;
        for (index, name) in [b"Su", b"Mo", b"Tu", b"We", b"Th", b"Fr", b"Sa"]
            .iter()
            .enumerate()
        {
            self.ui_text(
                x + 10 * scale + index * cell + cell / 4,
                top + 28 * scale,
                *name,
                127,
                157,
                182,
                1,
            );
        }
        let first = crate::ui::status_menu::weekday(year, month, 1);
        for day in 1..=crate::ui::status_menu::days(year, month) {
            let slot = first + day as usize - 1;
            let left = x + 10 * scale + slot % 7 * cell;
            let row = top + (54 + slot / 7 * 28) * scale;
            if day == today {
                self.fill_rounded_rect_alpha(
                    left,
                    row.saturating_sub(2 * scale),
                    cell.saturating_sub(2 * scale),
                    26 * scale,
                    7 * scale,
                    6,
                    103,
                    160,
                    245,
                );
            }
            let number = [
                if day < 10 { b' ' } else { b'0' + day / 10 },
                b'0' + day % 10,
            ];
            self.ui_text_strong(left + cell / 4, row, &number, 226, 240, 251, 1);
        }
    }

    // ------------------------=
    // FUNC: desktop_native_performance_menu
    // DESC: Renders the shared native-application performance menu above application content.
    // ------------------=
    fn desktop_native_performance_menu(
        &mut self,
        left: usize,
        top: usize,
        screen: u8,
        scale: usize,
    ) {
        let menu_left = left + 220 * scale;
        let menu_top = top + 42 * scale;
        let menu_width = 232 * scale;
        self.fill_rounded_rect_alpha(
            menu_left,
            menu_top,
            menu_width,
            140 * scale,
            9 * scale,
            5,
            18,
            31,
            250,
        );
        self.outline_rounded_rect(
            menu_left,
            menu_top,
            menu_width,
            140 * scale,
            9 * scale,
            74,
            171,
            218,
        );
        let image = if screen == 9 {
            crate::runtime::task_manager::IMAGE_TEXT_EDITOR
        } else if screen == 10 {
            crate::runtime::task_manager::IMAGE_TASK_MANAGER
        } else {
            crate::runtime::task_manager::IMAGE_COMMAND_WINDOW
        };
        let mode = crate::runtime::with_runtime(|runtime| {
            runtime
                .resources
                .override_for(crate::runtime::resource_policy::AppId(image))
                .map(|policy| policy.mode)
                .unwrap_or(runtime.resources.defaults().mode)
        })
        .unwrap_or(crate::runtime::resource_policy::ResourceMode::Balanced);
        for (row, label) in [
            b"Restricted".as_slice(),
            b"Balanced",
            b"Expanded",
            b"Performance Settings...",
        ]
        .iter()
        .enumerate()
        {
            let marked = matches!(
                (row, mode),
                (0, crate::runtime::resource_policy::ResourceMode::Restricted)
                    | (1, crate::runtime::resource_policy::ResourceMode::Balanced)
                    | (2, crate::runtime::resource_policy::ResourceMode::Expanded)
            );
            self.ui_text(
                menu_left + 14 * scale,
                menu_top + (12 + row * 30) * scale,
                if marked { b"*" } else { b"" },
                94,
                211,
                250,
                1,
            );
            self.ui_text(
                menu_left + 32 * scale,
                menu_top + (12 + row * 30) * scale,
                label,
                220,
                232,
                240,
                1,
            );
        }
    }

    // ------------------------=
    // FUNC: render_task_manager_dashboard
    // DESC: Renders animated authoritative CPU, memory, disk, process, and selected-task telemetry with a native Task menu.
    // ------------------=
    fn render_task_manager_dashboard(
        &mut self,
        geometry: crate::ui::system_layout::DesktopAppWindowGeometry,
        selected: usize,
        menu_open: bool,
        scale: usize,
    ) {
        let left = geometry.content.x.max(0) as usize;
        let top = geometry.content.y.max(0) as usize;
        let width = geometry.content.width as usize;
        let height = geometry.content.height as usize;
        let (accent_r, accent_g, accent_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Selection);
        self.fill_rounded_rect_alpha(left, top, width, height, 11 * scale, 1, 9, 19, 246);
        self.outline_rounded_rect(left, top, width, height, 11 * scale, 55, 133, 178);

        let (count, running, total_ticks, total_memory, total_memory_limit, total_disk) =
            crate::runtime::with_runtime(|runtime| {
                let count = runtime.task_manager.task_count(&runtime.execution);
                let mut running = 0usize;
                let mut ticks = 0u64;
                let mut memory = 0u64;
                let mut memory_limit = 0u64;
                let mut disk = 0u64;
                for index in 0..count {
                    if let Some(task) = runtime.task_manager.task_nth(&runtime.execution, index) {
                        if matches!(
                            task.state,
                            crate::runtime::execution::ContextState::Runnable
                                | crate::runtime::execution::ContextState::Running
                        ) {
                            running += 1;
                        }
                        ticks = ticks.saturating_add(task.usage.cpu_ticks);
                        memory = memory.saturating_add(task.usage.memory_bytes);
                        memory_limit = memory_limit.saturating_add(task.budget.memory_limit);
                        disk = disk.saturating_add(task.installed_bytes);
                    }
                }
                (count, running, ticks, memory, memory_limit, disk)
            })
            .unwrap_or((0, 0, 0, 0, 0, 0));

        let gap = 10 * scale;
        let card_top = top + 12 * scale;
        let card_height = 92 * scale;
        let card_width = width.saturating_sub(32 * scale + gap * 2) / 3;
        let memory_percent = if total_memory_limit == 0 {
            0
        } else {
            total_memory.saturating_mul(100) / total_memory_limit
        } as usize;
        let cpu_percent = if count == 0 {
            0
        } else {
            running.saturating_mul(100) / count
        };
        let disk_capacity = (count.max(1) as u64).saturating_mul(256 * 1024);
        let disk_percent =
            (total_disk.saturating_mul(100) / disk_capacity.max(1)).min(100) as usize;
        let cards = [
            (
                b"CPU ACTIVITY".as_slice(),
                total_ticks,
                cpu_percent,
                19usize,
            ),
            (
                b"MEMORY IN USE".as_slice(),
                total_memory,
                memory_percent,
                18usize,
            ),
            (
                b"APP DISK SIZE".as_slice(),
                total_disk,
                disk_percent,
                11usize,
            ),
        ];
        for (index, (label, value, percent, role)) in cards.iter().enumerate() {
            let card_left = left + 12 * scale + index * (card_width + gap);
            self.fill_rounded_rect_alpha(
                card_left,
                card_top,
                card_width,
                card_height,
                10 * scale,
                5,
                23,
                39,
                236,
            );
            self.outline_rounded_rect(
                card_left,
                card_top,
                card_width,
                card_height,
                10 * scale,
                accent_r / 2,
                accent_g / 2,
                accent_b / 2,
            );
            let _ = self.themed_icon(
                card_left + 24 * scale,
                card_top + 25 * scale,
                *role,
                25 * scale,
            );
            self.ui_text_strong(
                card_left + 44 * scale,
                card_top + 17 * scale,
                label,
                172,
                204,
                222,
                1,
            );
            let (value_text, value_length) = if index == 0 {
                Self::task_manager_count_text(*value)
            } else {
                Self::task_manager_bytes_text(*value)
            };
            self.ui_text_strong(
                card_left + 16 * scale,
                card_top + 45 * scale,
                &value_text[..value_length],
                237,
                247,
                252,
                1,
            );
            self.task_manager_meter(
                card_left + 16 * scale,
                card_top + 72 * scale,
                card_width.saturating_sub(32 * scale),
                *percent,
                scale,
                (accent_r, accent_g, accent_b),
            );
        }

        let table_top = top + 116 * scale;
        let table_height = height.saturating_sub(172 * scale);
        self.fill_rounded_rect_alpha(
            left + 12 * scale,
            table_top,
            width.saturating_sub(24 * scale),
            table_height,
            10 * scale,
            3,
            17,
            30,
            238,
        );
        self.ui_text_strong(
            left + 28 * scale,
            table_top + 13 * scale,
            b"TASK",
            129,
            197,
            229,
            1,
        );
        self.ui_text_strong(
            left + width * 43 / 100,
            table_top + 13 * scale,
            b"STATUS",
            129,
            197,
            229,
            1,
        );
        self.ui_text_strong(
            left + width * 57 / 100,
            table_top + 13 * scale,
            b"CPU",
            129,
            197,
            229,
            1,
        );
        self.ui_text_strong(
            left + width * 69 / 100,
            table_top + 13 * scale,
            b"MEMORY",
            129,
            197,
            229,
            1,
        );
        self.ui_text_strong(
            left + width * 84 / 100,
            table_top + 13 * scale,
            b"DISK",
            129,
            197,
            229,
            1,
        );

        let first = selected.saturating_sub(3);
        let row_height = 38 * scale;
        for visible in 0..5usize {
            let index = first + visible;
            let task = crate::runtime::with_runtime(|runtime| {
                runtime.task_manager.task_nth(&runtime.execution, index)
            })
            .flatten();
            let Some(task) = task else { break };
            let row_top = table_top + (38 + visible * 38) * scale;
            if row_top + row_height > top + height.saturating_sub(28 * scale) {
                break;
            }
            let active = index == selected;
            self.fill_rounded_rect_alpha(
                left + 18 * scale,
                row_top,
                width.saturating_sub(36 * scale),
                row_height.saturating_sub(3 * scale),
                7 * scale,
                if active { accent_r } else { 5 },
                if active { accent_g } else { 25 },
                if active { accent_b } else { 41 },
                if active { 155 } else { 210 },
            );
            let app = Self::task_manager_task_name(task.service_identity, task.image_identity);
            self.ui_text_strong(
                left + 30 * scale,
                row_top + 11 * scale,
                app,
                224,
                238,
                247,
                1,
            );
            self.ui_text(
                left + width * 43 / 100,
                row_top + 11 * scale,
                Self::task_manager_state_text(task.state),
                163,
                211,
                230,
                1,
            );
            let (cpu, cpu_len) = Self::task_manager_percent_text(task.cpu_share_percent as usize);
            let (memory, memory_len) = Self::task_manager_bytes_text(task.usage.memory_bytes);
            let (disk, disk_len) = Self::task_manager_bytes_text(task.installed_bytes);
            self.ui_text(
                left + width * 57 / 100,
                row_top + 11 * scale,
                &cpu[..cpu_len],
                104,
                220,
                251,
                1,
            );
            self.ui_text(
                left + width * 69 / 100,
                row_top + 11 * scale,
                &memory[..memory_len],
                204,
                224,
                236,
                1,
            );
            self.ui_text(
                left + width * 84 / 100,
                row_top + 11 * scale,
                &disk[..disk_len],
                204,
                224,
                236,
                1,
            );
        }
        let footer = if count == 1 {
            b"1 task".as_slice()
        } else {
            b"Live tasks  /  Use Task menu for actions".as_slice()
        };
        self.ui_text(
            left + 20 * scale,
            top + height.saturating_sub(24 * scale),
            footer,
            117,
            175,
            205,
            1,
        );
        if menu_open {
            self.render_task_manager_menu(geometry, scale, (accent_r, accent_g, accent_b));
        }
    }

    // ------------------------=
    // FUNC: task_manager_meter
    // DESC: Draws a bounded telemetry meter with a moving highlight tied to the compositor frame clock.
    // ------------------=
    fn task_manager_meter(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        percent: usize,
        scale: usize,
        accent: (u8, u8, u8),
    ) {
        self.fill_rounded_rect_alpha(left, top, width, 7 * scale, 3 * scale, 6, 18, 30, 245);
        let filled = width.saturating_mul(percent.min(100)) / 100;
        self.fill_rounded_rect_alpha(
            left,
            top,
            filled,
            7 * scale,
            3 * scale,
            accent.0,
            accent.1,
            accent.2,
            230,
        );
        if filled > 8 * scale {
            let shimmer = left + (self.presented_frames as usize * 5 * scale) % filled;
            self.fill_rounded_rect_alpha(
                shimmer,
                top,
                4 * scale,
                7 * scale,
                2 * scale,
                225,
                249,
                255,
                210,
            );
        }
    }

    // ------------------------=
    // FUNC: render_task_manager_menu
    // DESC: Renders the conventional application Task menu above dashboard content.
    // ------------------=
    fn render_task_manager_menu(
        &mut self,
        geometry: crate::ui::system_layout::DesktopAppWindowGeometry,
        scale: usize,
        accent: (u8, u8, u8),
    ) {
        let left = geometry.toolbar.x.max(0) as usize + 10 * scale;
        let top = geometry.toolbar.bottom().max(0) as usize - 2 * scale;
        let width = 232 * scale;
        self.fill_rounded_rect_alpha(left, top, width, 214 * scale, 10 * scale, 3, 15, 27, 250);
        self.outline_rounded_rect(
            left,
            top,
            width,
            214 * scale,
            10 * scale,
            accent.0,
            accent.1,
            accent.2,
        );
        let labels: [&[u8]; 6] = [
            b"Launch File Navigator",
            b"Relaunch Selected",
            b"Pause / Resume",
            b"Throttle Selected",
            b"End Selected Task",
            b"Refresh Now",
        ];
        for (index, label) in labels.iter().enumerate() {
            let row_top = top + (8 + index * 32) * scale;
            if index == 4 {
                self.fill_rect_alpha(
                    left + 12 * scale,
                    row_top.saturating_sub(4 * scale),
                    width.saturating_sub(24 * scale),
                    1,
                    51,
                    82,
                    101,
                    180,
                );
            }
            self.ui_text(
                left + 18 * scale,
                row_top + 8 * scale,
                label,
                if index == 4 { 255 } else { 218 },
                if index == 4 { 149 } else { 232 },
                if index == 4 { 149 } else { 241 },
                1,
            );
        }
    }

    // ------------------------=
    // FUNC: task_manager_bytes_text
    // DESC: Formats telemetry bytes into a compact KiB or MiB value without allocation.
    // ------------------=
    fn task_manager_bytes_text(bytes: u64) -> ([u8; 20], usize) {
        let (value, suffix): (u64, &[u8]) = if bytes >= 1024 * 1024 {
            (bytes / (1024 * 1024), b" MiB")
        } else {
            (bytes / 1024, b" KiB")
        };
        let (digits, length) = Self::task_manager_count_text(value);
        let mut output = [0u8; 20];
        output[..length].copy_from_slice(&digits[..length]);
        output[length..length + suffix.len()].copy_from_slice(suffix);
        (output, length + suffix.len())
    }

    // ------------------------=
    // FUNC: task_manager_count_text
    // DESC: Formats an unsigned task metric into a fixed stack buffer.
    // ------------------=
    fn task_manager_count_text(mut value: u64) -> ([u8; 20], usize) {
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
        let mut output = [0u8; 20];
        for index in 0..length {
            output[index] = reversed[length - index - 1];
        }
        (output, length)
    }

    // ------------------------=
    // FUNC: task_manager_percent_text
    // DESC: Formats one bounded CPU share as a percentage.
    // ------------------=
    fn task_manager_percent_text(value: usize) -> ([u8; 20], usize) {
        let (digits, length) = Self::task_manager_count_text(value.min(100) as u64);
        let mut output = [0u8; 20];
        output[..length].copy_from_slice(&digits[..length]);
        output[length] = b'%';
        (output, length + 1)
    }

    // ------------------------=
    // FUNC: task_manager_task_name
    // DESC: Resolves stable task identities to concise first-class display names.
    // ------------------=
    fn task_manager_task_name(service: u32, image: u32) -> &'static [u8] {
        match image {
            crate::runtime::task_manager::IMAGE_FILE_NAVIGATOR => b"File Navigator",
            crate::runtime::task_manager::IMAGE_TEXT_EDITOR => b"Text Editor",
            crate::runtime::task_manager::IMAGE_COMMAND_WINDOW => b"Command Window",
            crate::runtime::task_manager::IMAGE_TASK_MANAGER => b"Task Manager",
            _ if service != 0 => b"System Service",
            _ => b"Application",
        }
    }

    // ------------------------=
    // FUNC: task_manager_state_text
    // DESC: Maps typed execution state to a compact status label.
    // ------------------=
    fn task_manager_state_text(state: crate::runtime::execution::ContextState) -> &'static [u8] {
        match state {
            crate::runtime::execution::ContextState::Defined => b"Defined",
            crate::runtime::execution::ContextState::Runnable => b"Ready",
            crate::runtime::execution::ContextState::Running => b"Running",
            crate::runtime::execution::ContextState::Waiting => b"Paused",
            crate::runtime::execution::ContextState::Stopped => b"Stopped",
            crate::runtime::execution::ContextState::Failed => b"Failed",
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
        let picker=crate::ui::object_picker::presentation();
        let g=crate::ui::object_picker::geometry(content,scale,!open_picker);
        let _=(output_lines,output_lengths,output_count,focus);
        let x=g.sheet.x.max(0) as usize; let y=g.sheet.y.max(0) as usize;
        self.glass_panel(x,y,g.sheet.width as usize,g.sheet.height as usize,true);
        self.ui_text_strong(x+24*scale,y+18*scale,if open_picker {b"Open from Infinity Pool"} else {b"Save As - Infinity Pool"},234,244,250,1);
        let clip=self.render_clip;
        for (field,rect,value,label) in [
            (0u8,g.name,if picker.field==0 {name_input}else{picker.name.bytes()},b"File name" as &[u8]),
            (1u8,g.location,if picker.field==1 {name_input}else{picker.location.bytes()},b"Location" as &[u8])] {
            if open_picker && field==0 {continue;}
            let fx=rect.x.max(0) as usize; let fy=rect.y.max(0) as usize;
            self.fill_rounded_rect_alpha(fx,fy,rect.width as usize,rect.height as usize,8*scale,2,16,29,245);
            self.outline_rounded_rect(fx,fy,rect.width as usize,rect.height as usize,8*scale,78,195,242);
            let text_y=fy+(rect.height as usize).saturating_sub(28)/2;
            self.ui_text(fx+10*scale,text_y,label,158,187,204,1);
            let value_x=fx+120*scale;
            self.intersect_render_clip(value_x,fy,rect.width.saturating_sub((130*scale) as u32) as usize,rect.height as usize);
            self.ui_text(value_x,text_y,value,231,241,247,1);
            self.text_field_caret(value_x,fy,rect.height as usize,value,picker.field==field,1);
            self.render_clip=clip;
        }
        let page=picker.selected/g.rows;
        for row in 0..g.rows {
            let index=page*g.rows+row; if index>=picker.count {break;}
            let entry=picker.entries[index]; let top=g.list.y.max(0) as usize+row*g.row_height;
            let left=g.list.x.max(0) as usize;
            if index==picker.selected {
                self.fill_rounded_rect_alpha(left,top,g.list.width as usize,g.row_height,7*scale,11,72,108,235);
            }
            self.themed_icon(left+14*scale,top+g.row_height/2,if entry.folder {2}else{4},20*scale);
            self.intersect_render_clip(left+30*scale,top,g.list.width.saturating_sub((38*scale) as u32) as usize,g.row_height);
            self.ui_text(left+32*scale,top+g.row_height.saturating_sub(28)/2,crate::runtime::object_navigation::namespace_basename(entry.bytes()),218,234,244,1);
            self.render_clip=clip;
        }
        if picker.count==0 {self.ui_text(g.list.x.max(0) as usize,g.list.y.max(0) as usize,b"This location is empty.",160,184,199,1);}
        for (r,label,primary) in [(g.parent,b"UP" as &[u8],false),(g.previous,b"PREV",false),
            (g.next,b"NEXT",false),(g.cancel,b"CANCEL",false),(g.accept,if open_picker {b"OPEN"}else{b"SAVE"},true)] {
            self.polished_button(r.x.max(0) as usize,r.y.max(0) as usize,r.width as usize,r.height as usize,label,primary,false);
        }
        let error:&[u8]=match picker.error {1=>b"Enter a valid location and filename.",2=>b"That name exists. Choose another.",
            3=>b"Cannot open this object as editable text.",4=>b"Save failed. Check storage and location.",
            5=>b"Too many entries. Enter a narrower location.",_=>b"Tab: fields   Arrows: select   Enter: accept"};
        self.intersect_render_clip(x+24*scale,g.cancel.y.saturating_sub((32*scale) as i32).max(0) as usize,
            g.sheet.width.saturating_sub((48*scale) as u32) as usize,28*scale);
        self.ui_text(x+24*scale,g.cancel.y.saturating_sub((32*scale) as i32).max(0) as usize,error,
            if picker.error>0 {255}else{150},if picker.error>0 {120}else{185},if picker.error>0 {120}else{207},1);
        self.render_clip=clip;
    }

    // ------------------------=
    // FUNC: desktop_editor_unsaved_dialog
    // DESC: Renders the blocking kit-aligned Save, Discard, Cancel decision without obscuring document identity.
    // ------------------=
    fn desktop_editor_unsaved_dialog(
        &mut self,
        content: crate::ui::geometry::Rect,
        filename: &[u8],
        focus: usize,
        scale: usize,
    ) {
        let g = crate::ui::editor_chrome::unsaved_dialog_geometry(content, scale);
        let clip = self.render_clip;
        self.intersect_render_clip(
            content.x.max(0) as usize,
            content.y.max(0) as usize,
            content.width as usize,
            content.height as usize,
        );
        self.fill_rect_alpha(
            content.x.max(0) as usize,
            content.y.max(0) as usize,
            content.width as usize,
            content.height as usize,
            3,
            9,
            18,
            174,
        );
        let x = g.sheet.x.max(0) as usize;
        let y = g.sheet.y.max(0) as usize;
        self.fill_rounded_rect_alpha(
            x,
            y,
            g.sheet.width as usize,
            g.sheet.height as usize,
            12 * scale,
            15,
            27,
            46,
            252,
        );
        self.outline_rounded_rect(
            x,
            y,
            g.sheet.width as usize,
            g.sheet.height as usize,
            12 * scale,
            34,
            211,
            238,
        );
        self.ui_text_strong(
            x + 26 * scale,
            y + 24 * scale,
            b"Save changes before closing?",
            255,
            255,
            255,
            1,
        );
        self.ui_text(
            x + 26 * scale,
            y + 61 * scale,
            if filename.is_empty() { b"Untitled" } else { filename },
            34,
            211,
            238,
            1,
        );
        self.ui_text(
            x + 26 * scale,
            y + 91 * scale,
            b"Your edits are still in memory. Choose Save, Discard, or Cancel.",
            159,
            176,
            200,
            1,
        );
        for (index, rect, label, primary) in [
            (0usize, g.cancel, b"CANCEL" as &[u8], false),
            (1usize, g.discard, b"DISCARD", false),
            (2usize, g.save, b"SAVE", true),
        ] {
            self.polished_button(
                rect.x.max(0) as usize,
                rect.y.max(0) as usize,
                rect.width as usize,
                rect.height as usize,
                label,
                primary,
                focus == index,
            );
        }
        self.render_clip = clip;
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
        task_manager_window: crate::ui::system_layout::DesktopAppWindowState,
        editor_scroll_row: usize,
        editor_dialog: u8,
        editor_dialog_input: &[u8],
        editor_dialog_focus: usize,
    ) {
        self.mark_dirty_rect(0, 0, self.width, self.height);
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        if screen == 7 && super::launcher_backdrop::restore(self) {
            self.launcher_reveal(input, focus);
            return;
        }
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        if screen != 7 {
            if !super::spatial_view::refreshing() { super::launcher_backdrop::invalidate(); }
        }
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
        if matches!(screen, 2 | 3 | 4 | 7 | 8 | 9 | 10 | 11) {
            self.paint_desktop_background();
        } else {
            self.paint_first_boot_background();
        }
        let scale = self.ui_scale().max(1);
        let margin = self.width * 4 / 100;
        let top_bar = self.system_top_bar((screen == 3).then_some(menu_kind), clock);

        if matches!(screen, 3 | 7) {
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

        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        if matches!(screen, 3 | 7) { self.minimized_app_shelf(); }

        if screen == 7 {
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            super::launcher_backdrop::capture(self);
            let launcher_clip = self.render_clip;
            let reveal = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                .app_launcher_visible_region();
            self.intersect_render_clip(
                reveal.x.max(0) as usize,
                reveal.y.max(0) as usize,
                reveal.width as usize,
                reveal.height as usize,
            );
            self.app_launcher(scale, input, focus);
            self.render_clip = launcher_clip;
        }

        if matches!(screen,2|4|8|9|10|11) {
            let stack=crate::ui::desktop_stack::current();
            self.desktop_base(scale,desktop_items,desktop_item_positions,false);
            for id in stack.order {
                if !stack.visible[id] {continue;}
                if id==0 {
                    self.desktop_navigator_windows(scale,window_x,window_y,window_width,window_height,
                        window_visible,window_maximized,home_location,dragging_item);
                } else if id==4 {
                    self.render_settings_window(stack.settings_section,if screen==4 {input}else{b""},settings_window,scale);
                } else if id==5 {
                    #[cfg(feature="native-browser")]
                    self.browser_window(crate::console::browser_window());
                } else {
                    let (kind,state,text)=match id {
                        1=>(8,command_window,command_input),
                        2=>(9,editor_window,editor_input),
                        _=>(10,task_manager_window,input),
                    };
                    self.desktop_native_app_window(kind,text,output_lines,output_lengths,output_count,
                        state.x,state.y,state.width,state.height,state.maximized,editor_saved,false,
                        editor_scroll_row,if kind==screen {editor_dialog}else{0},editor_dialog_input,
                        editor_dialog_focus,if kind==screen {focus}else{0},if kind==screen {menu_kind}else{0});
                }
                {
                    let layout=crate::ui::system_layout::SystemLayout::new(self.width,self.height);
                    let rect=if id==4 {layout.settings_window_geometry(settings_window).window}
                    else if id==0 {let (x,y,w,h)=layout.home_window_geometry_sized(window_x,window_y,window_width,window_height,window_maximized);
                        crate::ui::geometry::Rect{x:x as i32,y:y as i32,width:w as u32,height:h as u32}}
                    else {let state=match id {1=>command_window,2=>editor_window,5=>crate::console::browser_window(),_=>task_manager_window};
                        layout.desktop_app_window_geometry(state.x,state.y,state.width,state.height,state.maximized).window};
                    if id!=0 && id!=5 && !(id==2 && editor_dialog!=0) {self.window_assistant(id,rect,scale);}
                    if id==stack.active {self.outline_rounded_rect(rect.x.max(0) as usize,rect.y.max(0) as usize,
                        rect.width as usize,rect.height as usize,10*scale,105,199,245);}
                }
            }
        }
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        if matches!(screen,2|4|8|9|10|11) { self.minimized_app_shelf(); self.desktop_widget_menu(scale); }
        if screen==4 {return;}
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
                self.window_control(
                    control_left,
                    title_center_y.saturating_sub(control_size / 2),
                    control_size,
                    index,
                    settings_window.maximized,
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
                    (b"Icon Families", b"Installed theme registry"),
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
    // FUNC: settings_card_sheen
    // DESC: Adds a bounded soft reflection to retained Settings cards without blur or framebuffer-sized work.
    // ------------------=
    fn settings_card_sheen(&mut self, x: usize, y: usize, width: usize, height: usize, scale: usize) {
        let inset = 12 * scale;
        if width <= inset * 2 || height < 16 { return; }
        let reflection_height = (height / 2).min(48 * scale);
        for band in 0..8 {
            let top = y + 2 + band * reflection_height / 8;
            let bottom = y + 2 + (band + 1) * reflection_height / 8;
            self.fill_rect_alpha(x + inset, top, width - inset * 2,
                bottom.saturating_sub(top), 157, 205, 236, (12 - band) as u8);
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
        let caller_clip = self.render_clip;
        if !self.recording_surface {
            let rect = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                .settings_window_geometry(settings_window)
                .window;
            self.retained_window(
                4,
                (
                    rect.x.max(0) as usize,
                    rect.y.max(0) as usize,
                    rect.width as usize,
                    rect.height as usize,
                ),
                |target| target.render_settings_window(focus, input, settings_window, scale),
            );
            return;
        }
        let layout = crate::ui::system_layout::SystemLayout::new(self.width, self.height);
        let geometry = layout.settings_window_geometry_for_section(settings_window, focus);
        let left = geometry.window.x.max(0) as usize;
        let top = geometry.window.y.max(0) as usize;
        let width = geometry.window.width as usize;
        let height = geometry.window.height as usize;
        let title_height = geometry.title.height as usize;
        let template_console = crate::ui::settings_template::element(
            focus,
            crate::ui::installer_template::InstallerTemplateRole::Console,
        );
        let settings_template = crate::ui::settings_template::template();
        let authored_rect = |element: crate::ui::installer_template::InstallerTemplateElement<'static>| {
            let console = template_console?;
            Some((
                left + element.frame.x.saturating_sub(console.frame.x) as usize * width
                    / console.frame.width.max(1) as usize,
                top + element.frame.y.saturating_sub(console.frame.y) as usize * height
                    / console.frame.height.max(1) as usize,
                element.frame.width as usize * width / console.frame.width.max(1) as usize,
                element.frame.height as usize * height / console.frame.height.max(1) as usize,
            ))
        };
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
        let authored_header = crate::ui::settings_template::element(
            focus,
            crate::ui::installer_template::InstallerTemplateRole::Masthead,
        );
        let authored_navigation = crate::ui::settings_template::element(
            focus,
            crate::ui::installer_template::InstallerTemplateRole::SectionLabel,
        );
        let authored_content = crate::ui::settings_template::element(
            focus,
            crate::ui::installer_template::InstallerTemplateRole::Content,
        );
        self.glass_panel(left, top, width, height, true);
        self.fill_rect_alpha(
            left,
            top,
            width,
            geometry.title.height as usize,
            authored_header.map(|element| element.fill[0]).unwrap_or(header_r),
            authored_header.map(|element| element.fill[1]).unwrap_or(header_g),
            authored_header.map(|element| element.fill[2]).unwrap_or(header_b),
            authored_header.map(|element| panel_alpha(element.fill[3])).unwrap_or(panel_alpha(222)),
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
            self.window_control(
                control_left,
                title_center_y.saturating_sub(control_size / 2),
                control_size,
                index,
                settings_window.maximized,
            );
        }
        let nav_width = geometry.navigation.width as usize;
        self.fill_rect_alpha(
            left,
            top + title_height,
            nav_width,
            height.saturating_sub(title_height),
            authored_navigation.map(|element| element.fill[0]).unwrap_or(primary_r / 2),
            authored_navigation.map(|element| element.fill[1]).unwrap_or(primary_g / 2),
            authored_navigation.map(|element| element.fill[2]).unwrap_or(primary_b / 2),
            authored_navigation.map(|element| panel_alpha(element.fill[3])).unwrap_or(panel_alpha(214)),
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
        let sections: [&[u8]; 11] = [
            b"General",
            b"Themes & Skins",
            b"Users & Accounts",
            b"AI & Voice",
            b"Privacy & Security",
            b"Devices",
            b"Network",
            b"Nodes & Mesh",
            b"Storage",
            b"About",
            b"Input",
        ];
        self.intersect_render_clip(
            geometry.navigation.x.max(0) as usize,
            geometry.navigation.y.max(0) as usize,
            geometry.navigation.width as usize,
            geometry.navigation.height as usize,
        );
        for (index, section) in sections.iter().enumerate() {
            let row = layout.settings_section_geometry_for_section(settings_window, index, focus);
            let (x, y, w, h) = (row.x.max(0) as usize, row.y.max(0) as usize,
                row.width as usize, row.height as usize);
            if index == focus {
                self.fill_rounded_rect_alpha(x, y, w, h, 10 * scale,
                    selection_r, selection_g, selection_b, 235);
                self.outline_rounded_rect(x, y, w, h, 10 * scale, outline_r, outline_g, outline_b);
                self.settings_card_sheen(x, y, w, h, scale);
            }
            let icon_size = (28 * scale).min(h.saturating_sub(8));
            let _ = self.themed_icon(x + 28 * scale, y + h / 2,
                crate::ui::icon_theme::settings_section_icon(index), icon_size);
            self.ui_text_elided_strong(x + 52 * scale,
                y + h.saturating_sub(UI_FONT_CELL_HEIGHT) / 2,
                w.saturating_sub(68 * scale), section,
                if index == focus { 240 } else { 182 },
                if index == focus { 248 } else { 204 },
                if index == focus { 252 } else { 220 });
        }
        self.render_clip = caller_clip;
        let content_x = geometry.content.x.max(0) as usize;
        let content_y = geometry.content.y.max(0) as usize;
        if let Some(element) = authored_content {
            self.fill_rounded_rect_alpha(
                content_x,
                content_y,
                geometry.content.width as usize,
                geometry.content.height as usize,
                element.corner_radius as usize * scale,
                element.fill[0], element.fill[1], element.fill[2], panel_alpha(element.fill[3]),
            );
        }
        if let Some(template) = settings_template {
            if let Some(count) = template.element_count(focus.saturating_add(1) as u8) {
                for index in 0..count {
                    let Some(element) = crate::ui::settings_template::layer_at(focus, index)
                        .filter(|element| !element.hidden && element.opacity > 0)
                    else { continue };
                    if !matches!(
                        element.role,
                        value if value == 0
                            || value == crate::ui::installer_template::InstallerTemplateRole::Image as u8
                            || (value == crate::ui::installer_template::InstallerTemplateRole::SettingsArtwork as u8
                                && !matches!(focus, 6 | 7 | 8))
                    ) {
                        continue;
                    }
                    let Some((x, y, mapped_width, mapped_height)) = authored_rect(element) else {
                        continue;
                    };
                    self.template_element_in_rect(
                        element,
                        if element.kind == 2 { template.asset(element.image_asset) } else { None },
                        None,
                        crate::ui::installer_layout::InstallerRect {
                            left: x, top: y, width: mapped_width, height: mapped_height,
                        },
                    );
                }
            }
        }
        let authored_title = crate::ui::settings_template::element(
            focus,
            crate::ui::installer_template::InstallerTemplateRole::Title,
        );
        let authored_body = crate::ui::settings_template::element(
            focus,
            crate::ui::installer_template::InstallerTemplateRole::Body,
        );
        let title_frame = authored_title.and_then(authored_rect);
        let body_frame = authored_body.and_then(authored_rect);
        let section_title = authored_title.map(|element| element.text)
            .filter(|text| !text.is_empty()).unwrap_or(sections[focus.min(10)]);
        self.ui_text_strong(
            title_frame.map(|frame| frame.0).unwrap_or(content_x),
            title_frame.map(|frame| frame.1).unwrap_or(content_y),
            section_title,
            authored_title.map(|element| element.fill[0]).unwrap_or(238),
            authored_title.map(|element| element.fill[1]).unwrap_or(244),
            authored_title.map(|element| element.fill[2]).unwrap_or(249),
            authored_title.map(|element| (element.font_size as usize / 16).clamp(1, 2)).unwrap_or(2),
        );
        if focus != 8 { self.ui_text(
            body_frame.map(|frame| frame.0).unwrap_or(content_x),
            body_frame.map(|frame| frame.1).unwrap_or(
                content_y + if focus == 7 { 64 } else { 36 } * scale
            ),
            authored_body.map(|element| element.text).filter(|text| !text.is_empty()).unwrap_or(
                if focus == 10 {
                    b"Click a row to change its value. Changes apply immediately."
                } else {
                    b"Open a row to view its controls and configuration details."
                }
            ),
            authored_body.map(|element| element.fill[0]).unwrap_or(143),
            authored_body.map(|element| element.fill[1]).unwrap_or(160),
            authored_body.map(|element| element.fill[2]).unwrap_or(176),
            1,
        ); }
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
                if runtime.chat.selected_model_ready() {
                    runtime.chat.selected_model_descriptor().name
                } else { b"Local model unavailable".as_slice() },
            )
        });
        let voice_enabled = crate::runtime::with_runtime(|runtime| {
            (0..crate::runtime::identity::MAX_SESSIONS)
                .filter_map(|index| runtime.identity.session_nth(index))
                .find(|session| {
                    session.state == crate::runtime::identity::SessionState::Active
                })
                .and_then(|session| runtime.identity.voice_profile(session.user))
                .map(|profile| {
                    profile.enabled
                        && profile.activation
                            != crate::runtime::identity::VoiceActivation::Disabled
                })
        })
        .flatten()
        .unwrap_or(false);
        let speech_enabled = crate::runtime::with_runtime(|runtime| {
            (0..crate::runtime::identity::MAX_SESSIONS)
                .filter_map(|index| runtime.identity.session_nth(index))
                .find(|session| session.state == crate::runtime::identity::SessionState::Active)
                .and_then(|session| runtime.identity.ai_profile(session.user))
                .map(|profile| profile.speech_output_enabled)
        }).flatten().unwrap_or(false);
        if focus == 6 {
            self.intersect_render_clip(
                geometry.viewport.x.max(0) as usize,
                geometry.viewport.y.max(0) as usize,
                geometry.viewport.width as usize,
                geometry.viewport.height as usize,
            );
            self.render_network_settings_dashboard(settings_window, scale, connectivity, input);
            self.render_clip = caller_clip;
            self.render_settings_overflow_chrome(
                geometry,
                settings_window,
                scale,
                (outline_r, outline_g, outline_b),
            );
            return;
        }
        if focus == 7 {
            self.intersect_render_clip(
                geometry.viewport.x.max(0) as usize,
                geometry.viewport.y.max(0) as usize,
                geometry.viewport.width as usize,
                geometry.viewport.height as usize,
            );
            self.render_node_settings_dashboard(settings_window, scale, input);
            self.render_clip = caller_clip;
            self.render_settings_overflow_chrome(
                geometry,
                settings_window,
                scale,
                (outline_r, outline_g, outline_b),
            );
            return;
        }
        let preferences = crate::ui::input_preferences::current();
        let pool = crate::runtime::storage_view::snapshot();
        let pool_object = pool.objects[pool.selected.min(7)];
        let pool_health: &[u8] = if pool.failed { b"Unavailable - retry" } else if !pool.ready { b"Loading" }
            else if pool.count == 0 { b"No protected objects" }
            else if pool.objects.iter().flatten().all(|o| o.verified >= o.desired) { b"Healthy" } else { b"Degraded" };
        let mut pool_identity = [b' ';32];
        if let Some(object) = pool_object { for (i,b) in object.id.iter().enumerate() {
            pool_identity[i*2]=b"0123456789abcdef"[(b>>4) as usize];pool_identity[i*2+1]=b"0123456789abcdef"[(b&15) as usize];
        }}
        let pool_node=pool.node_rows[pool.selected_node.min(31)];
        let pool_placement=pool.placements[pool.selected_placement.min(7)];
        let mut pool_node_identity=[b' ';64];let mut pool_replica_identity=[b' ';32];
        if let Some(node)=pool_node{for(i,b)in node.id.iter().enumerate(){pool_node_identity[i*2]=b"0123456789abcdef"[(b>>4)as usize];pool_node_identity[i*2+1]=b"0123456789abcdef"[(b&15)as usize];}}
        if let Some(placement)=pool_placement{for(i,b)in placement.resource.iter().enumerate(){pool_replica_identity[i*2]=b"0123456789abcdef"[(b>>4)as usize];pool_replica_identity[i*2+1]=b"0123456789abcdef"[(b&15)as usize];}}
        let pool_replica_state:&[u8]=match pool_placement.map(|p|p.state){
            Some(2)=>b"Verified",Some(3)=>b"Offline",Some(4)=>b"Stale",Some(5)=>b"Corrupt",Some(_)=>b"Not verified",None=>b"No placement"};
        let rows: [(&[u8], &[u8]); 8] = match focus.min(10) {
            10 => core::array::from_fn(|index| {
                (
                    crate::ui::input_preferences::LABELS[index],
                    preferences.value(index),
                )
            }),
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
                (b"Voice", if voice_enabled { b"Granted" } else { b"Restricted" }),
                (b"Activation", if voice_enabled { b"Continuous listening" } else { b"Disabled" }),
                (b"Spoken Replies", if speech_enabled { b"Enabled" } else { b"Disabled" }),
                (b"", b""),
            ],
            4 => [
                (b"Ambient Authority", b"Denied"),
                (b"Microphone", if voice_enabled { b"Granted" } else { b"Restricted" }),
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
                (b"Pointer", crate::ui::personalization::CURSOR_NAMES[preferences.cursor_style as usize]),
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
            8 => [
                (b"Infinity Pool", pool_health),
                (b"Capacity", if pool.ready {b"Measured bytes"}else{b"Awaiting observation"}),
                (b"Nodes", if pool_node.is_some_and(|n|n.online){b"Online / observed"}else if pool_node.is_some(){b"Offline / retained"}else{b"No observed nodes"}),
                (b"Selected Object", if pool_object.is_some() { &pool_identity[..12] } else { b"None" }),
                (b"Replica Location",pool_replica_state),
                (b"Temporary", if pool_object.is_some_and(|o|o.desired==1) { b"Selected" } else { b"Target: 1 verified node" }),
                (b"Protected", if pool_object.is_some_and(|o|o.desired==2) { b"Selected" } else { b"Target: 2 verified nodes" }),
                (b"Critical", if pool_object.is_some_and(|o|o.desired==3) { b"Selected" } else { b"Target: 3 verified nodes" }),
            ],
            _ => [
                (b"InfinityOS", b"Development"),
                (b"Architecture", b"Native"),
                (b"Boot", b"Verified"),
                (b"Identity Format", b"Version 1"),
                (b"Icon Families", b"Installed theme registry"),
                (b"", b""),
                (b"", b""),
                (b"", b""),
            ],
        };
        self.intersect_render_clip(
            geometry.viewport.x.max(0) as usize,
            geometry.viewport.y.max(0) as usize,
            geometry.viewport.width as usize,
            geometry.viewport.height as usize,
        );
        if focus == 1 {
            let hero = layout.settings_world_shift_hero_geometry(settings_window);
            let hero_left = hero.x.max(0) as usize;
            let hero_top = hero.y.max(0) as usize;
            let hero_width = hero.width as usize;
            let hero_height = hero.height as usize;
            self.paint_bitmap_cover_box(
                WORLD_SHIFT_HERO_BMP,
                hero_left,
                hero_top,
                hero_width,
                hero_height,
            );
            // Continuous scrim: retain the artwork without a hard vertical
            // seam through the hero. Cache ownership remains with the window.
            for column in 0..hero_width {
                let alpha = 230usize.saturating_sub(column * 210 / hero_width.max(1));
                self.fill_rect_alpha(hero_left + column, hero_top, 1, hero_height,
                    2, 12, 28, alpha as u8);
            }
            self.outline_rounded_rect(
                hero_left,
                hero_top,
                hero_width,
                hero_height,
                14 * scale,
                117,
                218,
                255,
            );
            self.ui_text_strong(
                hero_left + 24 * scale,
                hero_top + 30 * scale,
                b"WORLD SHIFT",
                235,
                250,
                255,
                1,
            );
            self.ui_text_elided_strong(
                hero_left + 24 * scale,
                hero_top + 62 * scale,
                hero_width * 48 / 100,
                b"Move your complete workspace between living worlds.",
                190,
                224,
                243,
            );
            self.spatial_glass_action(
                hero_left + 24 * scale,
                hero_top + hero_height.saturating_sub(56 * scale),
                176 * scale,
                38 * scale,
                b"Open World Shift",
                true,
            );
        }
        for (index, (label, value)) in rows
            .iter()
            .take(settings_window.row_count.clamp(1, 8))
            .enumerate()
        {
            let row = layout.settings_row_geometry_for_section(settings_window, index, focus);
            let authored_row = crate::ui::settings_template::role_at(
                focus,
                crate::ui::installer_template::InstallerTemplateRole::Metadata,
                index,
            );
            let authored_label = crate::ui::settings_template::role_at(
                focus,
                crate::ui::installer_template::InstallerTemplateRole::SettingsRowLabel,
                index,
            );
            let authored_value = crate::ui::settings_template::role_at(
                focus,
                crate::ui::installer_template::InstallerTemplateRole::SettingsRowValue,
                index,
            );
            let row_frame = |role| layout.settings_row_element_geometry(
                settings_window, focus, index, role,
            ).map(|frame| (frame.x.max(0) as usize, frame.y.max(0) as usize,
                frame.width as usize, frame.height as usize));
            let label_frame = row_frame(crate::ui::installer_template::InstallerTemplateRole::SettingsRowLabel);
            let value_frame = row_frame(crate::ui::installer_template::InstallerTemplateRole::SettingsRowValue);
            let disclosure_frame = row_frame(crate::ui::installer_template::InstallerTemplateRole::SettingsDisclosure);
            // Input labels are coupled to the preference operation, not decorative template copy.
            let label = if focus == 10 { *label } else {
                authored_label.map(|element| element.text)
                    .filter(|text| !text.is_empty()).unwrap_or(*label)
            };
            if row.summary.intersects(geometry.viewport) {
                let summary_left = row.summary.x.max(0) as usize;
                let summary_top = row.summary.y.max(0) as usize;
                let summary_width = row.summary.width as usize;
                let expanded = settings_window.expanded_row == Some(index);
                self.fill_rounded_rect_alpha(
                    summary_left,
                    summary_top,
                    summary_width,
                    row.summary.height as usize,
                    authored_row.map(|element| element.corner_radius as usize * scale).unwrap_or(10 * scale),
                    authored_row.map(|element| element.fill[0]).unwrap_or(primary_r / 2),
                    authored_row.map(|element| element.fill[1]).unwrap_or(primary_g.saturating_mul(3) / 4),
                    authored_row.map(|element| element.fill[2]).unwrap_or(primary_b.saturating_mul(3) / 4),
                    authored_row.map(|element| panel_alpha(element.fill[3])).unwrap_or(panel_alpha(218)),
                );
                self.outline_rounded_rect(
                    summary_left,
                    summary_top,
                    summary_width,
                    row.summary.height as usize,
                    10 * scale,
                    authored_row.map(|element| element.border[0]).unwrap_or(if expanded { outline_r } else { outline_r / 2 }),
                    authored_row.map(|element| element.border[1]).unwrap_or(if expanded { outline_g } else { outline_g / 2 }),
                    authored_row.map(|element| element.border[2]).unwrap_or(if expanded { outline_b } else { outline_b / 2 }),
                );
                self.settings_card_sheen(summary_left, summary_top, summary_width,
                    row.summary.height as usize, scale);
                let card = crate::ui::settings_cards::content(row.summary, scale);
                self.fill_rounded_rect_alpha(card.icon.x.max(0) as usize,
                    card.icon.y.max(0) as usize, card.icon.width as usize,
                    card.icon.height as usize, 9 * scale, 4, 20, 34, panel_alpha(150));
                self.outline_rounded_rect(card.icon.x.max(0) as usize,
                    card.icon.y.max(0) as usize, card.icon.width as usize,
                    card.icon.height as usize, 9 * scale, 35, 64, 84);
                let _ = self.themed_icon(
                    (card.icon.x + card.icon.width as i32 / 2).max(0) as usize,
                    (card.icon.y + card.icon.height as i32 / 2).max(0) as usize,
                    crate::ui::icon_theme::settings_section_icon(focus),
                    (28 * scale).min(card.icon.width as usize));
                self.ui_text_elided_strong(card.description.x.max(0) as usize,
                    card.description.y.max(0) as usize, card.description.width as usize,
                    crate::ui::settings_cards::description(focus, index), 141, 174, 198);
                let label_limit = label_frame.map(|frame| frame.2).unwrap_or(summary_width * 46 / 100);
                let mut label_length = label.len();
                while label_length > 0
                    && self.ui_text_width(&label[..label_length], 1) > label_limit
                {
                    label_length -= 1;
                }
                self.ui_text_strong(
                    label_frame.map(|frame| frame.0).unwrap_or(
                        summary_left + crate::ui::system_layout::UI_GUTTER * scale
                    ),
                    label_frame.map(|frame| frame.1).unwrap_or(summary_top + 13 * scale),
                    &label[..label_length],
                    authored_label.map(|element| element.fill[0]).unwrap_or(190),
                    authored_label.map(|element| element.fill[1]).unwrap_or(205),
                    authored_label.map(|element| element.fill[2]).unwrap_or(217),
                    1,
                );
                let value_limit = value_frame.map(|frame| frame.2).unwrap_or(summary_width * 42 / 100);
                let mut value_length = value.len();
                while value_length > 0
                    && self.ui_text_width(&value[..value_length], 1) > value_limit
                {
                    value_length -= 1;
                }
                let displayed_value = &value[..value_length];
                let value_width = self.ui_text_width(displayed_value, 1);
                self.ui_text(
                    value_frame.map(|frame| frame.0 + frame.2.saturating_sub(value_width))
                        .unwrap_or(summary_left + summary_width.saturating_sub(value_width + 40 * scale)),
                    value_frame.map(|frame| frame.1).unwrap_or(summary_top + 13 * scale),
                    displayed_value,
                    authored_value.map(|element| element.fill[0]).unwrap_or(220),
                    authored_value.map(|element| element.fill[1]).unwrap_or(232),
                    authored_value.map(|element| element.fill[2]).unwrap_or(240),
                    1,
                );
                if focus == 0 && index == 0 {
                    self.text_field_caret(
                        value_frame.map(|frame| frame.0 + frame.2.saturating_sub(value_width))
                            .unwrap_or(summary_left + summary_width.saturating_sub(value_width + 40 * scale)),
                        value_frame.map(|frame| frame.1).unwrap_or(summary_top),
                        value_frame.map(|frame| frame.3).unwrap_or(row.summary.height as usize),
                        displayed_value,
                        true,
                        1,
                    );
                }
                let twiddle_x = disclosure_frame.map(|frame| frame.0 + frame.2 / 2)
                    .unwrap_or(summary_left + summary_width.saturating_sub(20 * scale));
                let twiddle_y = disclosure_frame.map(|frame| frame.1 + frame.3 / 2)
                    .unwrap_or(summary_top + 23 * scale);
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
                && row.detail.intersects(geometry.viewport)
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
                if focus == 8 {
                    let gutter = crate::ui::system_layout::UI_GUTTER * scale;
                    let available=detail_width.saturating_sub(gutter*2);
                    if index==1 {
                        let labels:[&[u8];3]=[b"Raw",b"Eligible",b"Reserved"];
                        for(column,value)in [pool.capacity,pool.eligible,pool.reserved].iter().enumerate(){
                            let x=detail_left+gutter+column*available/3;
                            self.ui_text_elided_strong(x,detail_top+8*scale,available/3,labels[column],160,191,211);
                            let mut digits=[0;24];let n=navigator_decimal(&mut digits,*value as usize);
                            self.ui_text_elided_strong(x,detail_top+40*scale,available/3,&digits[..n],230,242,250);
                        }
                    } else if index==2 {
                        self.ui_text_elided_strong(detail_left+gutter,detail_top+8*scale,available,&pool_node_identity[..32],210,231,245);
                        self.ui_text_elided_strong(detail_left+gutter,detail_top+40*scale,available,&pool_node_identity[32..],210,231,245);
                    } else if index==3 {
                        self.ui_text_elided_strong(detail_left+gutter,detail_top+8*scale,available,&pool_identity,210,231,245);
                        let labels:[&[u8];3]=[b"Version",b"Desired",b"Verified"];
                        let values=pool_object.map(|o|[o.version,o.desired as u64,o.verified as u64]).unwrap_or([0;3]);
                        for column in 0..3 {let x=detail_left+gutter+column*available/3;
                            self.ui_text_elided_strong(x,detail_top+44*scale,available/3*2/3,labels[column],160,191,211);
                            let mut digits=[0;24];let n=navigator_decimal(&mut digits,values[column]as usize);
                            self.ui_text_elided_strong(x+available/3*2/3,detail_top+44*scale,available/9,&digits[..n],230,242,250);
                        }
                    } else {
                        let explanation:&[u8]=if index==0 {if pool.failed{b"Observation failed. Refresh to retry."}else if pool_object.is_some_and(|o|o.healing){b"Healing is active for the selected object."}
                            else if pool_object.is_some_and(|o|o.verified<o.desired){b"Insufficient verified independent replicas."}else{b"Owner-scoped authoritative Pool state."}}
                            else if index==4 {if pool_placement.is_some(){&pool_replica_identity}else{b"Select an observed object first."}}
                            else if pool_object.is_none(){b"Select an object before changing policy."}
                            else{b"Changes apply to the selected object only."};
                        self.ui_text_elided_strong(detail_left+gutter,detail_top+4*scale,available,explanation,160,191,211);
                    }
                    let action: &[u8] = match index {0|1=>b"REFRESH",2=>b"NEXT NODE",3=>b"NEXT OBJECT",4=>b"NEXT REPLICA",5=>b"USE TEMPORARY",6=>b"USE PROTECTED",_=>b"USE CRITICAL"};
                    self.polished_button(detail_left+gutter,detail_top+detail_height.saturating_sub((crate::ui::system_layout::UI_COMPACT_ACTION_HEIGHT+10)*scale),
                        (240*scale).min(available),crate::ui::system_layout::UI_COMPACT_ACTION_HEIGHT*scale,action,
                        index<=1 || (index==2 && pool_node.is_some()) || (index>=3 && pool_object.is_some()),false);
                } else if focus == 1 && index == 1 {
                    let theme_count = crate::ui::icon_theme::ICON_THEME_COUNT as usize;
                    let card_width = detail_width / theme_count;
                    for theme in 0..theme_count {
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
                } else if (focus == 5 && index == 2) || (focus == 1 && index == 7) {
                    let row=crate::ui::system_layout::SystemLayout::new(self.width,self.height)
                        .settings_row_geometry_for_section(settings_window,index,focus);
                    self.settings_personalization_panel(row.detail,scale,focus==5);
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
                        4,
                    );
                } else {
                    let description: &[u8] = match (focus, index) {
                        (0, 0) => b"Rename this machine through the durable identity service.",
                        (0, 1) => b"The installed interface language is English (US).",
                        (0, 2) => b"Regional formatting follows the installed US locale.",
                        (0, 3) => b"This is the active installed System Generation.",
                        (0, 4) => b"Updates are delivered as a complete System Generation.",
                        (1, 0) => b"Switch between installed, verified InfinityUI skins.",
                        (1, 6) => b"Automatic scale follows the active display density.",
                        (1, 7) => b"Cosmic Horizon is the active packaged desktop wallpaper.",
                        (3, 0) => {
                            b"Choose whether the local provider is strictly required or preferred."
                        }
                        (3, 1) => b"Show or hide your local desktop AI chat widget.",
                        (3, 2) => b"Choose the next installed local model without restarting.",
                        (2, 0) => b"The authenticated user owns this desktop session.",
                        (2, 1) => b"Password credentials are managed by the trusted identity flow.",
                        (2, 2) => b"This session is authenticated; locking preserves your workspace.",
                        (2, 3) => b"Personal Space isolates this user's objects and preferences.",
                        (2, 4) => b"The user profile persists with the installed system.",
                        (3, 3) => b"Remote processing is disabled; local inference stays on this device.",
                        (3, 4) => b"Microphone access follows the current user permission.",
                        (3, 5) => b"Voice activation follows the current user permission.",
                        (3, 6) => b"Model operations remain subject to capability enforcement.",
                        (4, 0) => b"Applications receive no implicit authority over your data.",
                        (4, 1) => b"Microphone access is explicitly granted or restricted here.",
                        (4, 2) => b"Remote AI access requires explicit capability authorization.",
                        (4, 3) => {
                            b"Lock this user's session after the selected period without input."
                        }
                        (4, 4) => b"Sensitive decisions use the trusted system interface.",
                        (5, 0) => b"The active display is used by the native desktop compositor.",
                        (5, 1) => b"Open Input to adjust keyboard repeat delay and rate.",
                        (5, 3 | 4) => b"This audio path is unavailable; no device control is exposed.",
                        (9, 0) => b"InfinityOS development System Generation.",
                        (9, 1) => b"The desktop and services execute in the native OS runtime.",
                        (9, 2) => b"Startup uses the installed boot contract and generation checks.",
                        (9, 3) => b"Identity records use the version 1 durable format.",
                        (9, 4) => b"Select an installed icon family in Themes & Skins.",
                        _ => b"This value is read from the active System Generation.",
                    };
                    self.ui_text_elided_strong(
                        detail_left + crate::ui::system_layout::UI_GUTTER * scale,
                        detail_top + crate::ui::system_layout::UI_GUTTER * scale,
                        detail_width.saturating_sub(crate::ui::system_layout::UI_GUTTER * 2 * scale),
                        description,
                        167,
                        188,
                        203,
                    );
                    let action: Option<&[u8]> = match (focus, index) {
                        (0, 0) => Some(b"EDIT NAME"),
                        (1, 0) => Some(b"SWITCH SKIN"),
                        (3, 0) => Some(b"CHANGE POLICY"),
                        (3, 1) => Some(if chat_enabled { b"DISABLE CHAT" } else { b"ENABLE CHAT" }),
                        (3, 2) => Some(b"NEXT MODEL"),
                        (3, 6) => Some(if speech_enabled { b"MUTE REPLIES" } else { b"SPEAK REPLIES" }),
                        (3, 4) | (3, 5) | (4, 1) => Some(if voice_enabled { b"RESTRICT" } else { b"GRANT" }),
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
                            (240 * scale).min(
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
        self.render_clip = caller_clip;
        self.render_settings_overflow_chrome(
            geometry,
            settings_window,
            scale,
            (outline_r, outline_g, outline_b),
        );
    }

    // ------------------------=
    // FUNC: render_settings_overflow_chrome
    // DESC: Draws the proportional Settings scrollbar and resize grip outside the clipped content viewport.
    // ------------------=
    fn render_settings_overflow_chrome(
        &mut self,
        geometry: crate::ui::system_layout::SettingsWindowGeometry,
        settings_window: crate::ui::system_layout::SettingsWindowState,
        scale: usize,
        outline: (u8, u8, u8),
    ) {
        let (outline_r, outline_g, outline_b) = outline;
        let left = geometry.window.x.max(0) as usize;
        let top = geometry.window.y.max(0) as usize;
        let width = geometry.window.width as usize;
        let height = geometry.window.height as usize;
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
            self.window_resize_affordances(
                left,
                top,
                width,
                height,
                scale,
                (outline_r, outline_g, outline_b),
            );
        }
    }

    // ------------------------=
    // FUNC: render_node_settings_dashboard
    // DESC: Renders the five Milestone 9 node trust, pairing, mesh, policy, and audit operator surfaces from typed state.
    // ------------------=
    fn render_node_settings_dashboard(
        &mut self,
        settings_window: crate::ui::system_layout::SettingsWindowState,
        scale: usize,
        input: &[u8],
    ) {
        let layout = crate::ui::system_layout::SystemLayout::new(self.width, self.height);
        let geometry = layout.node_settings_geometry(settings_window);
        let snapshot = crate::runtime::with_runtime(|runtime| {
            let view = &runtime.node_projection;
            let discovered = view.node_count;
            let trusted = view.nodes[..view.node_count].iter().filter(|record| matches!(record[85], 3 | 4)).count();
            let online = view.nodes[..view.node_count].iter().filter(|record| record[84] == 1).count();
            let pairings = runtime
                .nodes
                .pairings()
                .iter()
                .flatten()
                .filter(|pairing| {
                    pairing.state == crate::runtime::node::types::PairingState::AwaitingConfirmation
                })
                .count();
            let members = runtime
                .nodes
                .mesh_members()
                .iter()
                .flatten()
                .filter(|member| member.enabled)
                .count();
            let sessions = runtime
                .nodes
                .sessions()
                .iter()
                .flatten()
                .filter(|session| {
                    session.state == crate::runtime::node::types::SessionState::Established
                })
                .count();
            let grants = runtime
                .nodes
                .remote_grants()
                .iter()
                .flatten()
                .filter(|grant| !grant.revoked)
                .count();
            let audit = runtime.nodes.audit_records().iter().flatten().count();
            (
                runtime.nodes.local_id().is_some() && !runtime.node_projection.stale,
                discovered,
                trusted,
                online,
                pairings,
                members,
                sessions,
                grants,
                audit,
            )
        })
        .unwrap_or((false, 0, 0, 0, 0, 0, 0, 0, 0));
        let (
            identity_ready,
            discovered,
            trusted,
            online,
            pairings,
            members,
            sessions,
            grants,
            audit,
        ) = snapshot;
        let (outline_r, outline_g, outline_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
        let (selection_r, selection_g, selection_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Selection);
        for (card, role) in [
            (geometry.summary, crate::ui::installer_template::InstallerTemplateRole::SettingsSummaryCard),
            (geometry.main, crate::ui::installer_template::InstallerTemplateRole::SettingsMainCard),
            (geometry.sidebar, crate::ui::installer_template::InstallerTemplateRole::SettingsSidebarCard),
        ] {
            let authored = crate::ui::settings_template::element(7, role);
            self.fill_rounded_rect_alpha(
                card.x.max(0) as usize,
                card.y.max(0) as usize,
                card.width as usize,
                card.height as usize,
                authored.map(|value| value.corner_radius as usize * scale).unwrap_or(12 * scale),
                authored.map(|value| value.fill[0]).unwrap_or(4),
                authored.map(|value| value.fill[1]).unwrap_or(18),
                authored.map(|value| value.fill[2]).unwrap_or(31),
                authored.map(|value| value.fill[3]).unwrap_or(224),
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
        let page = settings_window.expanded_row.unwrap_or(0).min(4);
        let selected = crate::runtime::with_runtime(|runtime| {
            let peer = runtime.node_selection?;
            let verification = runtime.nodes.local_id().and_then(|local| runtime.node_transport.trust.verification(local, peer));
            Some((peer, verification))
        }).flatten();
        let tabs: [&[u8]; 5] = [
            b"TRUSTED NODES",
            b"PAIR NODE",
            b"MESH HEALTH",
            b"ACCESS POLICY",
            b"SECURITY AUDIT",
        ];
        for index in 0..5 {
            let tab = geometry.tabs[index];
            let active = page == index;
            let authored = crate::ui::settings_template::role_at(
                7, crate::ui::installer_template::InstallerTemplateRole::SettingsTab, index,
            );
            self.fill_rounded_rect_alpha(
                tab.x.max(0) as usize,
                tab.y.max(0) as usize,
                tab.width as usize,
                tab.height as usize,
                authored.map(|value| value.corner_radius as usize * scale).unwrap_or(7 * scale),
                if active { selection_r } else { authored.map(|value| value.fill[0]).unwrap_or(5) },
                if active { selection_g } else { authored.map(|value| value.fill[1]).unwrap_or(20) },
                if active { selection_b } else { authored.map(|value| value.fill[2]).unwrap_or(34) },
                authored.map(|value| value.fill[3]).unwrap_or(228),
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
            let label = authored.map(|value| value.text).filter(|text| !text.is_empty()).unwrap_or(tabs[index]);
            let available = (tab.width as usize).saturating_sub(24 * scale);
            let text_width = self.ui_text_width_weighted(label, 1, true).min(available);
            self.ui_text_elided_strong(
                tab.x.max(0) as usize + (tab.width as usize - text_width) / 2,
                tab.y.max(0) as usize + (tab.height as usize).saturating_sub(UI_FONT_CELL_HEIGHT) / 2,
                available,
                label,
                if active { 242 } else { 166 },
                if active { 248 } else { 190 },
                if active { 252 } else { 207 },
            );
        }
        let summary_left = geometry.summary.x.max(0) as usize;
        let summary_top = geometry.summary.y.max(0) as usize;
        self.authentication_icon(
            summary_left + 36 * scale,
            summary_top + geometry.summary.height as usize / 2,
            6,
            44 * scale,
            true,
        );
        self.ui_text_strong(
            summary_left + 74 * scale,
            summary_top + 14 * scale,
            crate::ui::settings_template::element(
                7, crate::ui::installer_template::InstallerTemplateRole::SettingsSummaryCard,
            ).map(|value| value.text).filter(|text| !text.is_empty()).unwrap_or(b"NODE IDENTITY"),
            outline_r,
            outline_g,
            outline_b,
            1,
        );
        self.ui_text_elided_strong(
            summary_left + 74 * scale,
            summary_top + 46 * scale,
            (geometry.summary.width as usize).saturating_sub(96 * scale),
            if identity_ready {
                b"Cryptographic identity ready"
            } else {
                b"Identity state refreshing / unavailable"
            },
            239,
            246,
            251,
        );
        let mut labels: [[&[u8]; 2]; 6] = match page {
            0 => [
                [b"DISCOVERED", b"Observed, not trusted"],
                [b"TRUSTED", b"Explicit relationships"],
                [b"ONLINE", b"Authenticated reachability"],
                [b"SESSIONS", b"Mutually authenticated"],
                [b"AUTHORITY", b"Capability scoped"],
                [b"REFRESH", b"Discover local nodes"],
            ],
            1 => [
                [b"SELECT NODE", b"Choose an untrusted peer"],
                [b"VERIFY IDENTITY", b"Compare fingerprint"],
                [b"PAIRING CODE", b"Confirm on both nodes"],
                [b"CONFIRM", b"Trusted UI required"],
                [b"CANCEL", b"Grant no authority"],
                [b"PAIRING STATE", b"Short lived transaction"],
            ],
            2 => [
                [b"MEMBERS", b"Explicit domain membership"],
                [b"ONLINE", b"Recent authenticated heartbeat"],
                [b"DEGRADED", b"Partial availability"],
                [b"OFFLINE", b"State retained safely"],
                [b"LEAVE DOMAIN", b"Preserve node trust"],
                [b"DIAGNOSTICS", b"Health and compatibility"],
            ],
            3 => [
                [b"OBJECT ACCESS", b"Deny by default"],
                [b"REMOTE OPERATIONS", b"Scoped capabilities"],
                [b"AI CONTEXT", b"No ambient sharing"],
                [b"RESOURCE USE", b"Separate milestone"],
                [b"LEASE", b"Expiry enforced"],
                [b"COMMIT POLICY", b"Persist then announce"],
            ],
            _ => [
                [b"TRUST EVENTS", b"Structured records"],
                [b"PAIRING EVENTS", b"Correlation preserved"],
                [b"SESSION EVENTS", b"No secret material"],
                [b"POLICY EVENTS", b"Auditable changes"],
                [b"EXPORT", b"Typed projection"],
                [b"REFRESH", b"Read durable records"],
            ],
        };
        let values = [
            discovered,
            trusted,
            online,
            sessions,
            grants,
            if page == 1 {
                pairings
            } else if page == 2 {
                members
            } else {
                audit
            },
        ];
        let (policy_offset, policy) = crate::runtime::with_runtime(|runtime| {
            (runtime.node_policy_offset, selected.and_then(|(peer, _)| runtime.node_projection.nodes[..runtime.node_projection.node_count].iter().find(|record| record[..32] == peer.0).copied()))
        }).unwrap_or((0, None));
        if page == 3 {
            let categories: [&[u8]; 12] = [b"OBJECT", b"NAMESPACE", b"COMPUTE", b"AI", b"SERVICE", b"EVENT", b"STORAGE", b"CLIPBOARD", b"DEVICE", b"DIAGNOSTICS", b"MESH", b"ADMINISTRATIVE"];
            for index in 0..5 {
                labels[index] = if policy_offset + index < 12 { [categories[policy_offset + index], b"Deny / session / allow"] } else { [b"", b""] };
            }
            labels[5] = [b"NEXT CATEGORIES", b"Twelve independent policies"];
        }
        for index in 0..6 {
            let card = geometry.controls[index];
            let active = settings_window.control_focus.min(5) == index;
            let authored = crate::ui::settings_template::role_at(
                7, crate::ui::installer_template::InstallerTemplateRole::Metadata, index,
            );
            self.fill_rounded_rect_alpha(
                card.x.max(0) as usize,
                card.y.max(0) as usize,
                card.width as usize,
                card.height as usize,
                authored.map(|value| value.corner_radius as usize * scale).unwrap_or(8 * scale),
                if active { selection_r } else { authored.map(|value| value.fill[0]).unwrap_or(6) },
                if active { selection_g } else { authored.map(|value| value.fill[1]).unwrap_or(24) },
                if active { selection_b } else { authored.map(|value| value.fill[2]).unwrap_or(39) },
                authored.map(|value| value.fill[3]).unwrap_or(230),
            );
            self.outline_rounded_rect(
                card.x.max(0) as usize,
                card.y.max(0) as usize,
                card.width as usize,
                card.height as usize,
                8 * scale,
                if active { outline_r } else { outline_r / 2 },
                if active { outline_g } else { outline_g / 2 },
                if active { outline_b } else { outline_b / 2 },
            );
            self.settings_card_sheen(card.x.max(0) as usize, card.y.max(0) as usize,
                card.width as usize, card.height as usize, scale);
            let (mut number, mut length) = Self::network_metric_text(values[index] as u64);
            if page == 1 {
                let verification = selected.and_then(|(_, verification)| verification);
                let value: &[u8] = match index {
                    0 => if selected.is_some() { b"Selected" } else { b"Choose" },
                    1 => if verification.is_some() { b"Ready" } else { b"Begin" },
                    2 => if verification.is_some() { b"Compare" } else { b"Waiting" },
                    3 => b"Enter code",
                    4 => b"Cancel",
                    _ => match verification.map(|value| value.state) {
                        Some(crate::runtime::node::wire_trust::WireState::LocallyConfirmed) => b"Peer pending",
                        Some(crate::runtime::node::wire_trust::WireState::RemotelyConfirmed) => b"Your turn",
                        Some(crate::runtime::node::wire_trust::WireState::Confirmed) => b"Confirmed",
                        Some(_) => b"Verify",
                        None => b"No transcript",
                    },
                };
                length = value.len().min(number.len()); number[..length].copy_from_slice(&value[..length]);
            }
            if page == 3 {
                let value: &[u8] = if index == 5 { match policy_offset { 0 => b"1 / 3", 5 => b"2 / 3", _ => b"3 / 3" } }
                else if policy_offset + index >= 12 { b"" }
                else { match policy.map(|record| record[86 + policy_offset + index]) { Some(0) => b"Deny", Some(1) => b"Allow", Some(2) => b"Session", Some(3) => b"Leased", _ => b"Select peer" } };
                length = value.len().min(number.len()); number[..length].copy_from_slice(&value[..length]);
            }
            let number_width = self.ui_text_width(&number[..length], 1);
            let label_width = (card.width as usize).saturating_sub(number_width + 48 * scale);
            self.ui_text_elided_strong(card.x.max(0) as usize + 15 * scale,
                card.y.max(0) as usize + 12 * scale, label_width, labels[index][0], 220, 239, 249);
            self.ui_text_elided_strong(card.x.max(0) as usize + 15 * scale,
                card.y.max(0) as usize + 44 * scale,
                (card.width as usize).saturating_sub(30 * scale), labels[index][1], 145, 174, 193);
            self.ui_text(
                card.right().max(0) as usize - number_width - 14 * scale,
                card.y.max(0) as usize + 18 * scale,
                &number[..length],
                outline_r,
                outline_g,
                outline_b,
                1,
            );
        }
        let art = geometry.sidebar;
        if page == 1 {
            // Verification is live authenticated state, never decorative artwork.
            let left = art.x.max(0) as usize + 15 * scale;
            let top = art.y.max(0) as usize + 14 * scale;
            let width = (art.width as usize).saturating_sub(30 * scale);
            self.ui_text_wrapped(left, top, width, b"COMPARE BOTH SCREENS", 220, 239, 249, 2);
            if let Some((_, Some(verification))) = selected {
                let mut code = [b'0'; 6]; let mut value = verification.code;
                for digit in code.iter_mut().rev() { *digit += (value % 10) as u8; value /= 10; }
                self.ui_text_fit_strong(left, top + 72 * scale, width, &code, outline_r, outline_g, outline_b, 2);
                self.ui_text(left, top + 144 * scale, b"Transcript fingerprint", 145, 174, 193, 1);
                let digits = b"0123456789abcdef";
                for row in 0..4 {
                    let mut line = [0; 16];
                    for column in 0..8 { let byte = verification.fingerprint[row * 8 + column]; line[column * 2] = digits[(byte >> 4) as usize]; line[column * 2 + 1] = digits[(byte & 15) as usize]; }
                    self.ui_text_fit_strong(left, top + (176 + row * 32) * scale, width, &line, 220, 239, 249, 1);
                }
                let remaining = crate::runtime::node_client::clock().map(|now| verification.expires.saturating_sub(now)).unwrap_or(0);
                let (value, length) = Self::network_metric_text(remaining);
                self.ui_text(left, top + 328 * scale, b"Seconds remaining", 145, 174, 193, 1);
                self.ui_text(left, top + 360 * scale, &value[..length], outline_r, outline_g, outline_b, 1);
                self.ui_text(left, top + 408 * scale, b"Enter peer code", 145, 174, 193, 1);
                self.ui_text_fit_strong(left, top + 440 * scale, width, &input[..input.len().min(6)], 242, 248, 252, 2);
                self.ui_text(left, top + 520 * scale, b"Enter: confirm", 145, 174, 193, 1);
                self.ui_text(left, top + 552 * scale, b"Esc: cancel", 145, 174, 193, 1);
            } else {
                self.ui_text_wrapped(left, top + 42 * scale, width, b"Select a peer and begin pairing. No authority is granted until both operators confirm the matching fingerprint and code.", 145, 174, 193, 10);
            }
            return;
        }
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        {
            let template = crate::ui::installer_template::InstallerTemplate::parse_settings(
                crate::ui::installer_layout::SETTINGS_TEMPLATE_BYTES,
            ).ok();
            let authored = crate::ui::settings_template::element(
                7, crate::ui::installer_template::InstallerTemplateRole::SettingsArtwork,
            );
            let authored_rect = art;
            let bytes = authored
                .and_then(|element| template.and_then(|value| value.asset(element.image_asset)))
                .unwrap_or(NODE_TRUST_TOPOLOGY_BMP);
            let available_width = (authored_rect.width as usize).saturating_sub(24 * scale);
            let available_height = (authored_rect.height as usize).saturating_sub(144 * scale);
            let source_width = if bytes.len() >= 54 { le32(bytes, 18) as usize } else { 1 }.max(1);
            let source_height = if bytes.len() >= 54 { (le32(bytes, 22) as i32).unsigned_abs() as usize } else { 1 }.max(1);
            let image_width = available_width.min(available_height * source_width / source_height);
            let image_height = image_width * source_height / source_width;
            self.paint_bitmap_fit_rect(
                bytes,
                authored_rect.x.max(0) as usize + (authored_rect.width as usize - image_width) / 2,
                authored_rect.y.max(0) as usize + 12 * scale,
                image_width,
                image_height,
            );
        }
        #[cfg(target_arch = "x86")]
        self.fill_rect(
            art.x.max(0) as usize + 2 * scale,
            art.y.max(0) as usize + 2 * scale,
            (art.width as usize).saturating_sub(4 * scale),
            (art.height as usize).saturating_sub(4 * scale),
            2,
            13,
            25,
        );
        self.fill_rect_alpha(
            art.x.max(0) as usize + 2 * scale,
            art.bottom().max(0) as usize - 70 * scale,
            (art.width as usize).saturating_sub(4 * scale),
            68 * scale,
            1,
            11,
            22,
            220,
        );
        self.ui_text_wrapped(art.x.max(0) as usize + 16 * scale,
            art.bottom().max(0) as usize - 132 * scale,
            (art.width as usize).saturating_sub(32 * scale),
            b"Discovery is not trust. Trust is not authority.", 185, 215, 233, 4);
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
        let connectivity: &[u8] = match interface {
            Some(value) if !value.enabled => b"Adapter disabled",
            Some(_) if status.active_profile == 3 => b"Offline profile",
            Some(value) if value.device.link_state == crate::runtime::network::types::LinkState::Unknown => b"Link not verified",
            Some(value) if value.device.link_state == crate::runtime::network::types::LinkState::Down => b"Link disconnected",
            Some(_) if status.connectivity == crate::runtime::network::types::ConnectivityClass::LinkOnly => b"Link up, no address",
            _ => connectivity,
        };
        let (outline_r, outline_g, outline_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
        let (selection_r, selection_g, selection_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Selection);
        let cards = [
            (geometry.summary, crate::ui::installer_template::InstallerTemplateRole::SettingsSummaryCard),
            (geometry.main, crate::ui::installer_template::InstallerTemplateRole::SettingsMainCard),
            (geometry.sidebar, crate::ui::installer_template::InstallerTemplateRole::SettingsSidebarCard),
        ];
        for (card, role) in cards {
            let authored = crate::ui::settings_template::element(6, role);
            self.fill_rounded_rect_alpha(
                card.x.max(0) as usize,
                card.y.max(0) as usize,
                card.width as usize,
                card.height as usize,
                authored.map(|value| value.corner_radius as usize * scale).unwrap_or(12 * scale),
                authored.map(|value| value.fill[0]).unwrap_or(4),
                authored.map(|value| value.fill[1]).unwrap_or(18),
                authored.map(|value| value.fill[2]).unwrap_or(31),
                authored.map(|value| value.fill[3]).unwrap_or(220),
            );
            self.outline_rounded_rect(
                card.x.max(0) as usize,
                card.y.max(0) as usize,
                card.width as usize,
                card.height as usize,
                authored.map(|value| value.corner_radius as usize * scale).unwrap_or(12 * scale),
                authored.map(|value| value.border[0]).unwrap_or(outline_r / 2),
                authored.map(|value| value.border[1]).unwrap_or(outline_g / 2),
                authored.map(|value| value.border[2]).unwrap_or(outline_b / 2),
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
            let authored = crate::ui::settings_template::role_at(
                6, crate::ui::installer_template::InstallerTemplateRole::SettingsTab, index,
            );
            self.fill_rounded_rect_alpha(
                tab.x.max(0) as usize,
                tab.y.max(0) as usize,
                tab.width as usize,
                tab.height as usize,
                authored.map(|value| value.corner_radius as usize * scale).unwrap_or(7 * scale),
                if active { selection_r } else { authored.map(|value| value.fill[0]).unwrap_or(5) },
                if active { selection_g } else { authored.map(|value| value.fill[1]).unwrap_or(20) },
                if active { selection_b } else { authored.map(|value| value.fill[2]).unwrap_or(34) },
                authored.map(|value| value.fill[3]).unwrap_or(226),
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
            let label = authored.map(|value| value.text).filter(|text| !text.is_empty()).unwrap_or(page_labels[index]);
            let available = (tab.width as usize).saturating_sub(24 * scale);
            let text_width = self.ui_text_width_weighted(label, 1, true).min(available);
            self.ui_text_elided_strong(
                tab.x.max(0) as usize + (tab.width as usize - text_width) / 2,
                tab.y.max(0) as usize + (tab.height as usize).saturating_sub(UI_FONT_CELL_HEIGHT) / 2,
                available,
                label,
                if active { 242 } else { 166 },
                if active { 248 } else { 190 },
                if active { 252 } else { 207 },
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
            crate::ui::settings_template::element(
                6, crate::ui::installer_template::InstallerTemplateRole::SettingsSummaryCard,
            ).map(|value| value.text).filter(|text| !text.is_empty()).unwrap_or(b"CONNECTIVITY"),
            outline_r,
            outline_g,
            outline_b,
            1,
        );
        self.ui_text_elided_strong(
            overview_left + 74 * scale,
            overview_top + 39 * scale,
            (geometry.summary.width as usize).saturating_sub(252 * scale),
            connectivity,
            239,
            246,
            251,
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
            let active = settings_window.control_focus.min(5) == index;
            let authored = crate::ui::settings_template::role_at(
                6, crate::ui::installer_template::InstallerTemplateRole::Metadata, index,
            );
            self.fill_rounded_rect_alpha(
                left,
                top,
                card.width as usize,
                card.height as usize,
                authored.map(|value| value.corner_radius as usize * scale).unwrap_or(8 * scale),
                if active { selection_r } else { authored.map(|value| value.fill[0]).unwrap_or(6) },
                if active { selection_g } else { authored.map(|value| value.fill[1]).unwrap_or(24) },
                if active { selection_b } else { authored.map(|value| value.fill[2]).unwrap_or(39) },
                authored.map(|value| value.fill[3]).unwrap_or(230),
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
            if active && matches!((page, index), (2, 1..=4) | (3, 1..=2)) {
                self.text_field_caret(
                    left + card.width as usize - value_width - 18 * scale,
                    top,
                    card.height as usize,
                    input,
                    true,
                    1,
                );
            }
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
                    b"No primary server configured"
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
        let mut line_top = detail_top + 36 * scale;
        let line_width = (geometry.sidebar.width as usize).saturating_sub(34 * scale);
        for line in state_lines.iter() {
            let lines = self.ui_text_wrapped_line_count(line_width, line, 3).max(1);
            self.ui_text_wrapped(
                sidebar_left + 17 * scale,
                line_top,
                line_width,
                line,
                174,
                198,
                215,
                3,
            );
            line_top += lines * (UI_FONT_CELL_HEIGHT + 4) * self.ui_scale() + 8 * scale;
        }
        if !settings_window.maximized {
            let window = layout.settings_window_geometry(settings_window).window;
            self.window_resize_affordances(
                window.x.max(0) as usize,
                window.y.max(0) as usize,
                window.width as usize,
                window.height as usize,
                scale,
                (outline_r, outline_g, outline_b),
            );
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
        self.settings_slider(settings_window, scale, index, value, maximum, 1);
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
        section: usize,
    ) {
        let layout = crate::ui::system_layout::SystemLayout::new(self.width, self.height);
        let geometry = layout.settings_effect_slider_geometry_for_section(
            settings_window,
            index,
            value,
            maximum,
            section,
        );
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
        let role = [25usize, 2, 26, 28, 23, 27, 17, 10, 19, 57]
            .get(kind)
            .copied()
            .unwrap_or(2);
        let painted = if role >= 57 {
            self.launcher_icon(left + size / 2, top + size / 2, role, size)
        } else {
            self.themed_icon(left + size / 2, top + size / 2, role, size)
        };
        if painted {
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
            8 => {
                for inset in [size / 5, size / 3] {
                    self.fill_rounded_rect_alpha(
                        left + inset,
                        top + inset,
                        size / 2,
                        size / 3,
                        (size / 16).max(2),
                        20,
                        68,
                        100,
                        230,
                    );
                    self.outline_rounded_rect(
                        left + inset,
                        top + inset,
                        size / 2,
                        size / 3,
                        (size / 16).max(2),
                        160,
                        226,
                        255,
                    );
                }
            }
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
        self.top_bar_infinity_icon(left + size / 2, top + size / 2, size * 3 / 4);
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
        let panel = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
            .app_launcher_geometry()
            .panel;
        self.retained_window(
            5,
            (
                panel.x.max(0) as usize,
                panel.y.max(0) as usize,
                panel.width as usize,
                panel.height as usize,
            ),
            |target| target.paint_app_launcher(scale, query, focus),
        );
    }

    // ------------------------=
    // FUNC: launcher_reveal
    // DESC: Reveals the cached launcher surface over a frozen backdrop with bounded damage.
    // ------------------=
    fn launcher_reveal(&mut self, query: &[u8], focus: usize) {
        let clip = self.render_clip;
        let reveal = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
            .app_launcher_visible_region();
        self.intersect_render_clip(
            reveal.x.max(0) as usize,
            reveal.y.max(0) as usize,
            reveal.width as usize,
            reveal.height as usize,
        );
        self.app_launcher(self.ui_scale().max(1), query, focus);
        self.render_clip = clip;
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
        let presentation = crate::ui::app_launcher::launcher_presentation();
        let launcher_clip = self.render_clip;
        self.intersect_render_clip(
            geometry.grid_viewport.x.max(0) as usize,
            geometry.grid_viewport.y.max(0) as usize,
            geometry.grid_viewport.width as usize,
            geometry.grid_viewport.height as usize,
        );
        for visible_index in 0..visible {
            let slot = crate::ui::app_launcher::launcher_display_slot(visible_index, presentation);
            let column = slot % crate::ui::app_launcher::LAUNCHER_COLUMNS;
            let row = slot / crate::ui::app_launcher::LAUNCHER_COLUMNS;
            let cell_left = geometry.grid_left + column * geometry.grid_cell_width;
            let cell_top = (geometry.grid_top + row * geometry.grid_row_height)
                .saturating_sub(presentation.scroll);
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
        self.render_clip = launcher_clip;
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
        let presentation = crate::ui::app_launcher::launcher_presentation();
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
        self.text_field_caret(
            search_left + 50 * scale,
            search_top,
            search_height,
            query,
            focus == 0,
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
        let launcher_clip = self.render_clip;
        self.intersect_render_clip(
            geometry.grid_viewport.x.max(0) as usize,
            geometry.grid_viewport.y.max(0) as usize,
            geometry.grid_viewport.width as usize,
            geometry.grid_viewport.height as usize,
        );
        for visible_index in 0..visible {
            let Some(entry) = crate::ui::app_launcher::launcher_visible_entry(query, visible_index)
            else {
                continue;
            };
            if presentation.drag_moved && presentation.drag_source == Some(visible_index) {
                continue;
            }
            let slot = crate::ui::app_launcher::launcher_display_slot(visible_index, presentation);
            let column = slot % crate::ui::app_launcher::LAUNCHER_COLUMNS;
            let row = slot / crate::ui::app_launcher::LAUNCHER_COLUMNS;
            let cell_left = geometry.grid_left + column * geometry.grid_cell_width;
            let cell_top = (geometry.grid_top + row * geometry.grid_row_height)
                .saturating_sub(presentation.scroll);
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
        self.render_clip = launcher_clip;
        let scroll = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
            .app_launcher_scroll_geometry(visible);
        if scroll.maximum_scroll != 0 {
            self.fill_rounded_rect_alpha(
                geometry.scrollbar_track.x.max(0) as usize,
                geometry.scrollbar_track.y.max(0) as usize,
                geometry.scrollbar_track.width as usize,
                geometry.scrollbar_track.height as usize,
                geometry.scrollbar_track.width as usize / 2,
                16,
                43,
                65,
                176,
            );
            self.fill_rounded_rect_alpha(
                scroll.thumb.x.max(0) as usize,
                scroll.thumb.y.max(0) as usize,
                scroll.thumb.width as usize,
                scroll.thumb.height as usize,
                scroll.thumb.width as usize / 2,
                84,
                199,
                249,
                242,
            );
        }
        if presentation.drag_moved && crate::ui::app_launcher::shortcuts::current().drag.is_none() {
            if let Some(source) = presentation.drag_source {
                if let Some(entry) = crate::ui::app_launcher::launcher_visible_entry(query, source)
                {
                    let ghost_size = geometry
                        .grid_cell_width
                        .min(geometry.grid_row_height)
                        .saturating_mul(70)
                        / 100;
                    let center_x = (self.width as i32 * presentation.drag_x / 1000)
                        .clamp(0, self.width.saturating_sub(1) as i32)
                        as usize;
                    let center_y = (self.height as i32 * presentation.drag_y / 1000)
                        .clamp(0, self.height.saturating_sub(1) as i32)
                        as usize;
                    let left = center_x.saturating_sub(ghost_size / 2);
                    let top = center_y.saturating_sub(ghost_size / 2);
                    self.fill_rounded_rect_alpha(
                        left.saturating_sub(5 * scale),
                        top.saturating_sub(5 * scale),
                        ghost_size + 10 * scale,
                        ghost_size + 10 * scale,
                        17 * scale,
                        29,
                        105,
                        151,
                        210,
                    );
                    self.outline_rounded_rect(
                        left,
                        top,
                        ghost_size,
                        ghost_size,
                        13 * scale,
                        112,
                        220,
                        255,
                    );
                    let _ = self.launcher_icon(
                        center_x,
                        center_y,
                        entry.icon_role,
                        ghost_size * 84 / 100,
                    );
                }
            }
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
        self.desktop_base(scale,desktop_items,desktop_item_positions,launcher_open);
        self.desktop_navigator_windows(scale,window_x,window_y,window_width,window_height,
            window_visible,window_maximized,home_location,dragging_item);
    }

    // ------------------------=
    // FUNC: desktop_base
    // DESC: Paints desktop widgets, objects and dock once beneath the ordered application windows.
    // ------------------=
    fn desktop_base(&mut self,scale:usize,desktop_items:u8,desktop_item_positions:&[[i32;2];7],launcher_open:bool) {
        self.desktop_widgets(scale);
        let shortcuts=crate::ui::app_launcher::shortcuts::current();
        for (id,position) in shortcuts.stationary_positions().iter().enumerate() {
            if let Some((x,y,_,_))=crate::ui::app_launcher::shortcuts::icon_rect(*position,self.width,self.height) {
                self.desktop_app_shortcut(id,x,y,false);
            }
        }
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
        self.desktop_dock(scale, launcher_open);
    }
    // ------------------------=
    // FUNC: desktop_navigator_windows
    // DESC: Paints only navigator layers without repainting desktop widgets over other applications.
    // ------------------=
    fn desktop_navigator_windows(&mut self,scale:usize,window_x:i32,window_y:i32,
        window_width:i32,window_height:i32,window_visible:bool,window_maximized:bool,
        home_location:usize,dragging_item:Option<usize>) {
        let active_navigator =
            crate::runtime::with_runtime(|runtime| runtime.file_navigators.active_index())
                .flatten();
        for layer in 0..crate::runtime::object_navigation::MAX_FILE_NAVIGATOR_INSTANCES {
            let layered = crate::runtime::with_runtime(|runtime| {
                runtime.file_navigators.back_to_front(layer)
            })
            .flatten();
            let Some((index, navigator)) = layered else {
                continue;
            };
            if Some(index) == active_navigator || !navigator.visible {
                continue;
            }
            let (left, top, width, height) =
                crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                    .home_window_geometry_sized(
                        navigator.x,
                        navigator.y,
                        navigator.width,
                        navigator.height,
                        navigator.maximized,
                    );
            self.retained_window(6 + index, (left, top, width, height), |target| {
                target.paint_navigator_window(scale, navigator.x, navigator.y,
                    navigator.width, navigator.height, navigator.maximized,
                    home_location, None, Some(navigator.state));
            });
            self.window_assistant(5+index,crate::ui::geometry::Rect{x:left as i32,y:top as i32,width:width as u32,height:height as u32},scale);
            if !navigator.maximized {
                let outline =
                    self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
                self.window_resize_affordances(left, top, width, height, scale, outline);
            }
        }
        if window_visible {
            let bounds = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                .home_window_geometry_sized(
                    window_x,
                    window_y,
                    window_width,
                    window_height,
                    window_maximized,
                );
            let navigator_state = crate::runtime::with_runtime(|r| r.file_navigator).flatten();
            self.retained_window(active_navigator.map(|i| 6 + i).unwrap_or(0), bounds, |target| {
                target.paint_navigator_window(
                    scale,
                    window_x,
                    window_y,
                    window_width,
                    window_height,
                    window_maximized,
                    home_location,
                    dragging_item,
                    navigator_state,
                )
            });
            self.window_assistant(5+active_navigator.unwrap_or(0),crate::ui::geometry::Rect{x:bounds.0 as i32,y:bounds.1 as i32,width:bounds.2 as u32,height:bounds.3 as u32},scale);
        }
    }

    // ------------------------=
    // FUNC: paint_navigator_window
    // DESC: Rasterizes the active navigator into its desktop-owned retained surface after invalidation.
    // ------------------=
    fn paint_navigator_window(
        &mut self,
        scale: usize,
        window_x: i32,
        window_y: i32,
        window_width: i32,
        window_height: i32,
        window_maximized: bool,
        home_location: usize,
        dragging_item: Option<usize>,
        navigator_state: Option<crate::runtime::object_navigation::FileNavigatorState>,
    ) {
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
                .home_window_geometry_sized(window_x, window_y, window_width, window_height, false)
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
        self.app_text(
            browser_left + 14 * scale,
            title_center_y.saturating_sub(12 * scale),
            b"File Navigator",
            (226, 237, 245), true, scale,
        );
        for (index, (label, offset)) in [
            (b"File".as_slice(), 160usize),
            (b"Performance".as_slice(), 211),
            (b"View".as_slice(), 322),
            (b"Navigate".as_slice(), 373),
            (b"Help".as_slice(), 458),
        ]
        .iter()
        .enumerate()
        {
            let selected = navigator_state
                .and_then(|state| state.menu_open)
                .map(|menu| menu.index() == index + 1)
                .unwrap_or(false);
            if selected {
                self.fill_rounded_rect_alpha(
                    browser_left + offset * scale - 5 * scale,
                    browser_top + 4 * scale,
                    (label.len() * 8 + 10) * scale,
                    26 * scale,
                    6 * scale,
                    selection_r,
                    selection_g,
                    selection_b,
                    190,
                );
            }
            self.app_text(
                browser_left + offset * scale,
                title_center_y.saturating_sub(12 * scale),
                label,
                (221, 233, 241), false, scale,
            );
        }
        for index in 0..3usize {
            let control_size = 20 * scale;
            let control_left =
                browser_left + browser_width.saturating_sub((28 + (2 - index) * 27) * scale);
            self.window_control(
                control_left,
                title_center_y.saturating_sub(control_size / 2),
                control_size,
                index,
                window_maximized,
            );
        }
        if !window_maximized {
            let outline = self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
            self.window_resize_affordances(
                browser_left,
                browser_top,
                browser_width,
                browser_height,
                scale,
                outline,
            );
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
            let editing_text = navigator_state
                .as_ref()
                .map(|state| state.editor_text.as_bytes())
                .unwrap_or(b"");
            self.text_field_caret(
                location_left + 14 * scale,
                tool_top + 5 * scale,
                28 * scale,
                editing_text,
                true,
                2,
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
        if scale == 1 {
            self.app_text(browser_left + 14, tool_top + 51, b"FAVORITES", (90, 191, 230), true, 1);
        } else {
            self.ui_text_strong(
                browser_left + 14 * scale,
                tool_top + 51 * scale,
                b"FAVORITES",
                90, 191, 230, 1,
            );
        }
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
            let row = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                .navigator_sidebar_row(crate::ui::geometry::Rect {x: browser_left as i32, y: browser_top as i32,
                    width: browser_width as u32, height: browser_height as u32}, index);
            let item_y = row.y.max(0) as usize + (row.height as usize).saturating_sub(20*scale)/2;
            if index == home_location {
                self.fill_rect_alpha(
                    row.x.max(0) as usize,
                    row.y.max(0) as usize,
                    row.width as usize,
                    row.height as usize,
                    selection_r,
                    selection_g,
                    selection_b,
                    218,
                );
            }
            if !item.is_empty() && index != 10 {
                let role = [0,1,4,5,6,7,8,9,10,2,2,12,13][index];
                self.themed_icon(row.x.max(0) as usize + 10*scale,
                    row.y.max(0) as usize + row.height as usize/2, role,
                    (20*scale).min(row.height as usize));
            }
            if scale == 1 {
                self.app_text(row.x.max(0) as usize + (if index == 10 { 0 } else { 32 }), item_y, item,
                    (if index == 10 { 90 } else { 204 }, if index == 10 { 191 } else { 224 }, 236),
                    index == 10, 1);
            } else {
                self.ui_text_weighted(
                    row.x.max(0) as usize + (if index == 10 { 0 } else { 32 }) * scale,
                    item_y,
                    item,
                    if index == 10 { 90 } else { 204 }, if index == 10 { 191 } else { 224 }, 236,
                    1, index == 10,
                );
            }
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
            let is_renaming = navigator_state
                .map(|state| state.rename_editing && state.selected_index as usize == index)
                .unwrap_or(false);
            let name = if is_renaming {
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
                if is_renaming {
                    self.text_field_caret(
                        grid_x + 22 * scale,
                        row_y.saturating_sub(7 * scale),
                        30 * scale,
                        name,
                        true,
                        2,
                    );
                }
                let kind_name = match kind {
                    Some(crate::storage::object::ObjectType::NamespaceNode) => b"Folder".as_slice(),
                    Some(crate::storage::object::ObjectType::Text) => b"Text document".as_slice(),
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
                if is_renaming {
                    let name_width = self.ui_text_width(name, 1);
                    self.text_field_caret(
                        center_x.saturating_sub(name_width / 2),
                        center_y + 28 * scale,
                        28 * scale,
                        name,
                        true,
                        2,
                    );
                }
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
        if navigator_state
            .map(|state| state.inspector_open)
            .unwrap_or(false)
        {
            let preview_width = (220 * scale).min(browser_width.saturating_sub(sidebar_w) / 2);
            let preview_left = browser_left + browser_width.saturating_sub(preview_width);
            let preview_top = tool_top + 38 * scale;
            let preview_height = browser_height.saturating_sub(title_h + 62 * scale);
            self.fill_rect_alpha(
                preview_left,
                preview_top,
                preview_width,
                preview_height,
                5,
                20,
                34,
                238,
            );
            self.outline_rounded_rect(
                preview_left,
                preview_top,
                preview_width,
                preview_height,
                8 * scale,
                55,
                132,
                177,
            );
            self.ui_text_strong(
                preview_left + 18 * scale,
                preview_top + 18 * scale,
                b"PREVIEW",
                91,
                205,
                247,
                1,
            );
            let selected = navigator_state
                .map(|state| state.selected_index)
                .unwrap_or(crate::runtime::object_navigation::FILE_NAVIGATOR_NO_SELECTION);
            if selected == crate::runtime::object_navigation::FILE_NAVIGATOR_NO_SELECTION {
                self.ui_text(
                    preview_left + 18 * scale,
                    preview_top + 54 * scale,
                    b"Select an object to inspect it.",
                    168,
                    191,
                    207,
                    1,
                );
            } else {
                self.ui_text(
                    preview_left + 18 * scale,
                    preview_top + 54 * scale,
                    b"Selected object",
                    213,
                    228,
                    238,
                    1,
                );
                let mut selection_text = [0u8; 18];
                let selection_len = navigator_decimal(&mut selection_text, selected as usize + 1);
                self.ui_text(
                    preview_left + 18 * scale,
                    preview_top + 80 * scale,
                    &selection_text[..selection_len],
                    139,
                    184,
                    211,
                    1,
                );
            }
        }
        if let Some(context) = navigator_state.filter(|state| state.context_menu_open) {
            let scale = crate::ui::system_layout::SystemLayout::new(self.width, self.height).scale();
            let menu = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                .file_navigator_context_geometry(context.context_x, context.context_y, context.context_actions().len());
            let menu_left = menu.x.max(0) as usize;
            let menu_top = menu.y.max(0) as usize;
            let actions = context.context_actions();
            self.fill_rounded_rect_alpha(
                menu_left,
                menu_top,
                menu.width as usize,
                menu.height as usize,
                8 * scale,
                5,
                18,
                31,
                246,
            );
            self.outline_rounded_rect(
                menu_left,
                menu_top,
                menu.width as usize,
                menu.height as usize,
                8 * scale,
                73,
                180,
                229,
            );
            for (index, action) in actions.iter().enumerate() {
                let row_top = menu_top + (6 + index * 28) * scale;
                if index == context.menu_selection as usize {
                    self.fill_rounded_rect_alpha(menu_left + 5 * scale, row_top,
                        menu.width as usize - 10 * scale, 28 * scale, 4 * scale, 20, 66, 91, 255);
                }
                self.app_text(
                    menu_left + 14 * scale,
                    row_top + 2 * scale,
                    action.label(),
                    (215, 231, 241), false, scale,
                );
            }
        }
        if let Some(menu) = navigator_state.and_then(|state| state.menu_open) {
            let labels: &[&[u8]] = match menu {
                crate::runtime::object_navigation::FileNavigatorMenu::File => {
                    &[b"New Window", b"Settings", b"Empty Trash", b"About"]
                }
                crate::runtime::object_navigation::FileNavigatorMenu::Performance => &[
                    b"Restricted",
                    b"Balanced",
                    b"Expanded",
                    b"Performance Settings...",
                ],
                crate::runtime::object_navigation::FileNavigatorMenu::View => {
                    &[b"As List", b"As Grid", b"Preview Panel Enabled"]
                }
                crate::runtime::object_navigation::FileNavigatorMenu::Navigate => &[
                    b"Home",
                    b"Personal Space",
                    b"Documents",
                    b"Downloads",
                    b"Pictures",
                    b"Music",
                    b"Videos",
                    b"Projects",
                    b"Custom Location...",
                ],
                crate::runtime::object_navigation::FileNavigatorMenu::Help => {
                    &[b"File Navigator Help"]
                }
            };
            let menu_rect = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                .file_navigator_menu_geometry(
                    browser_left,
                    browser_top,
                    menu.index(),
                    labels.len(),
                );
            let menu_left = menu_rect.x.max(0) as usize;
            let menu_top = menu_rect.y.max(0) as usize;
            self.fill_rounded_rect_alpha(
                menu_left,
                menu_top,
                menu_rect.width as usize,
                menu_rect.height as usize,
                9 * scale,
                5,
                18,
                31,
                248,
            );
            self.outline_rounded_rect(
                menu_left,
                menu_top,
                menu_rect.width as usize,
                menu_rect.height as usize,
                9 * scale,
                74,
                171,
                218,
            );
            let selected = navigator_state
                .map(|state| state.menu_selection as usize)
                .unwrap_or(0);
            for (index, label) in labels.iter().enumerate() {
                let row_top = menu_top + (6 + index * 30) * scale;
                if index == selected {
                    self.fill_rounded_rect_alpha(
                        menu_left + 5 * scale,
                        row_top,
                        menu_rect.width as usize - 10 * scale,
                        28 * scale,
                        5 * scale,
                        selection_r,
                        selection_g,
                        selection_b,
                        190,
                    );
                }
                let marked = match (menu, index) {
                    (crate::runtime::object_navigation::FileNavigatorMenu::View, 0) => {
                        navigator_list_view
                    }
                    (crate::runtime::object_navigation::FileNavigatorMenu::View, 1) => {
                        !navigator_list_view
                    }
                    (crate::runtime::object_navigation::FileNavigatorMenu::View, 2) => {
                        navigator_state
                            .map(|state| state.inspector_open)
                            .unwrap_or(false)
                    }
                    (crate::runtime::object_navigation::FileNavigatorMenu::Performance, row) => {
                        let mode = crate::runtime::with_runtime(|runtime| {
                            runtime
                                .resources
                                .override_for(crate::runtime::resource_policy::AppId(
                                    crate::runtime::task_manager::IMAGE_FILE_NAVIGATOR,
                                ))
                                .map(|policy| policy.mode)
                                .unwrap_or(runtime.resources.defaults().mode)
                        })
                        .unwrap_or(crate::runtime::resource_policy::ResourceMode::Balanced);
                        matches!(
                            (row, mode),
                            (0, crate::runtime::resource_policy::ResourceMode::Restricted)
                                | (1, crate::runtime::resource_policy::ResourceMode::Balanced)
                                | (2, crate::runtime::resource_policy::ResourceMode::Expanded)
                        )
                    }
                    _ => false,
                };
                self.ui_text(
                    menu_left + 14 * scale,
                    row_top + 6 * scale,
                    if marked { b"*" } else { b"" },
                    94,
                    211,
                    250,
                    1,
                );
                self.ui_text(
                    menu_left + 32 * scale,
                    row_top + 6 * scale,
                    label,
                    220,
                    232,
                    240,
                    1,
                );
            }
        }
        if let Some(dialog) = navigator_state.and_then(|state| state.dialog_open) {
            let dialog_rect = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                .file_navigator_dialog_geometry(
                    browser_left,
                    browser_top,
                    browser_width,
                    browser_height,
                );
            let left = dialog_rect.x.max(0) as usize;
            let top = dialog_rect.y.max(0) as usize;
            let width = dialog_rect.width as usize;
            let height = dialog_rect.height as usize;
            self.fill_rounded_rect_alpha(left, top, width, height, 14 * scale, 4, 17, 30, 252);
            self.outline_rounded_rect(left, top, width, height, 14 * scale, 76, 188, 235);
            let (title, detail): (&[u8], &[u8]) = match dialog {
                crate::runtime::object_navigation::FileNavigatorDialog::EmptyTrash => (
                    b"Empty Trash?",
                    b"All objects in Trash will be permanently removed.",
                ),
                crate::runtime::object_navigation::FileNavigatorDialog::About => (
                    b"About File Navigator",
                    b"Browse InfinityOS objects through human Namespace references.",
                ),
                crate::runtime::object_navigation::FileNavigatorDialog::Help => (
                    b"File Navigator Help",
                    b"Use menus, the sidebar, or the location field to navigate.",
                ),
                crate::runtime::object_navigation::FileNavigatorDialog::Location => (
                    b"Go to a custom location",
                    b"Enter an absolute InfinityOS Namespace reference.",
                ),
            };
            self.ui_text_strong(left + 24 * scale, top + 22 * scale, title, 231, 239, 245, 1);
            self.ui_text(
                left + 24 * scale,
                top + 54 * scale,
                detail,
                166,
                190,
                207,
                1,
            );
            if dialog == crate::runtime::object_navigation::FileNavigatorDialog::Location {
                self.fill_rounded_rect_alpha(
                    left + 24 * scale,
                    top + 82 * scale,
                    width - 48 * scale,
                    38 * scale,
                    8 * scale,
                    2,
                    13,
                    24,
                    250,
                );
                self.outline_rounded_rect(
                    left + 24 * scale,
                    top + 82 * scale,
                    width - 48 * scale,
                    38 * scale,
                    8 * scale,
                    72,
                    188,
                    235,
                );
                let path_text = navigator_state
                    .map(|state| state.editor_text)
                    .unwrap_or(crate::runtime::object_navigation::ByteText::empty());
                let path = path_text.as_bytes();
                self.ui_text(left + 36 * scale, top + 93 * scale, path, 220, 232, 240, 1);
                self.text_field_caret(
                    left + 36 * scale,
                    top + 82 * scale,
                    38 * scale,
                    path,
                    true,
                    2,
                );
            }
            let button_top = top + height.saturating_sub(54 * scale);
            let two_buttons = matches!(
                dialog,
                crate::runtime::object_navigation::FileNavigatorDialog::EmptyTrash
                    | crate::runtime::object_navigation::FileNavigatorDialog::Location
            );
            if two_buttons {
                let half = width / 2;
                for (index, label) in [
                    b"Cancel".as_slice(),
                    if dialog == crate::runtime::object_navigation::FileNavigatorDialog::EmptyTrash
                    {
                        b"Empty Trash"
                    } else {
                        b"Go"
                    },
                ]
                .iter()
                .enumerate()
                {
                    let button_left = left
                        + if index == 0 {
                            20 * scale
                        } else {
                            half + 6 * scale
                        };
                    let button_width = half.saturating_sub(26 * scale);
                    self.fill_rounded_rect_alpha(
                        button_left,
                        button_top,
                        button_width,
                        38 * scale,
                        8 * scale,
                        if index == 1 { selection_r } else { 10 },
                        if index == 1 { selection_g } else { 31 },
                        if index == 1 { selection_b } else { 48 },
                        235,
                    );
                    self.outline_rounded_rect(
                        button_left,
                        button_top,
                        button_width,
                        38 * scale,
                        8 * scale,
                        76,
                        165,
                        207,
                    );
                    self.ui_text_centered(
                        button_left,
                        button_width,
                        button_top + 10 * scale,
                        label,
                        226,
                        236,
                        243,
                        1,
                    );
                }
            } else {
                let button_left = left + width / 2 - 74 * scale;
                self.fill_rounded_rect_alpha(
                    button_left,
                    button_top,
                    148 * scale,
                    38 * scale,
                    8 * scale,
                    selection_r,
                    selection_g,
                    selection_b,
                    225,
                );
                self.ui_text_centered(
                    button_left,
                    148 * scale,
                    button_top + 10 * scale,
                    b"Close",
                    230,
                    239,
                    245,
                    1,
                );
            }
        }
    }

    // ------------------------=
    // FUNC: desktop_widgets
    // DESC: Renders the persistent right-side system overview and AI status foreground layer.
    // ------------------=
    fn desktop_widgets(&mut self, scale: usize) {
        if crate::ui::desktop_widgets::current().visible & 1 != 0 { self.desktop_overview_widget(scale); }
        self.desktop_ai_chat(scale);
    }

    // ------------------------=
    // FUNC: desktop_overview_widget
    // DESC: Paints the system widget at its shared movable geometry.
    // ------------------=
    fn desktop_overview_widget(&mut self, scale: usize) {
        let (accent_r, accent_g, accent_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Focus);
        let geometry = crate::ui::desktop_widgets::current().rect(0,self.width,self.height,scale,false);
        let widget_left = geometry.x.max(0) as usize;
        let widget_width = geometry.width as usize;
        let overview_top = geometry.y.max(0) as usize;
        let overview_height = geometry.height as usize;
        self.glass_panel(
            widget_left,
            overview_top,
            widget_width,
            overview_height,
            false,
        );
        for row in 0..3 { for col in 0..2 { self.fill_rect(widget_left+(5+col*4)*scale,overview_top+(17+row*4)*scale,scale,scale,120,191,226); } }
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
        ]
        .iter()
        .enumerate()
        {
            let row_y = overview_top + 48 * scale + index * (overview_height.saturating_sub(48 * scale) / 5).min(34 * scale);
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
    }

    // ------------------------=
    // FUNC: desktop_ai_chat
    // DESC: Renders the persistent model-selectable AI status and conversation surface.
    // ------------------=
    fn desktop_ai_chat(&mut self, scale: usize) {
        if crate::ui::desktop_widgets::current().visible & 2 == 0 { return; }
        let chat = crate::runtime::ai::with_ai_runtime(|runtime| runtime.chat);
        if !chat.enabled() {
            return;
        }
        let voice_enabled = crate::runtime::with_runtime(|runtime| {
            (0..crate::runtime::identity::MAX_SESSIONS)
                .filter_map(|index| runtime.identity.session_nth(index))
                .find(|session| {
                    session.state == crate::runtime::identity::SessionState::Active
                })
                .and_then(|session| runtime.identity.voice_profile(session.user))
                .map(|profile| {
                    profile.enabled
                        && profile.activation
                            != crate::runtime::identity::VoiceActivation::Disabled
                })
        })
        .flatten()
        .unwrap_or(false);
        let layout = crate::ui::system_layout::SystemLayout::new(self.width, self.height);
        let geometry = layout.ai_chat_geometry(chat.minimized());
        let left = geometry.panel.x.max(0) as usize;
        let top = geometry.panel.y.max(0) as usize;
        let width = geometry.panel.width as usize;
        let height = geometry.panel.height as usize;
        let (accent_r, accent_g, accent_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Focus);
        self.glass_panel(left, top, width, height, false);
        for row in 0..3 { for col in 0..2 { self.fill_rect(left+(3+col*4)*scale,top+(17+row*4)*scale,scale,scale,120,191,226); } }
        self.ui_text_strong(
            left + 16 * scale,
            top + 15 * scale,
            b"AI CHAT",
            accent_r,
            accent_g,
            accent_b,
            1,
        );
        let mut throughput = [0u8; 32];
        let throughput_len = crate::runtime::ai::with_ai_runtime(|runtime| {
            if !matches!(chat.selected_model(), crate::runtime::ai::chat::MINISTRAL_MODEL_ID | crate::runtime::ai::chat::HERMES_MODEL_ID)
                || runtime.qwen_tokens < 2 || runtime.qwen_decode_ns == 0 { return 0; }
            let tenths = runtime.qwen_tokens.saturating_sub(1).saturating_mul(10_000_000_000)
                / runtime.qwen_decode_ns;
            let n = navigator_decimal(&mut throughput, (tenths / 10) as usize);
            throughput[n] = b'.';
            throughput[n + 1] = b'0' + (tenths % 10) as u8;
            throughput[n + 2..n + 8].copy_from_slice(b" TOK/S");
            n + 8
        });
        let state = if chat.minimized() {
            b"LOCAL  +".as_slice()
        } else if !chat.selected_model_ready() {
            b"UNAVAILABLE".as_slice()
        } else {
            use crate::runtime::ai::chat::GenerationState;
            match chat.generation_state {
                GenerationState::Running => b"Thinking...".as_slice(),
                GenerationState::Cancelled => b"CANCELLED".as_slice(),
                GenerationState::Failed => b"FAILED".as_slice(),
                GenerationState::ContextFull => b"CONTEXT FULL".as_slice(),
                GenerationState::Complete if throughput_len != 0 => &throughput[..throughput_len],
                _ => b"LOCAL  READY".as_slice(),
            }
        };
        #[cfg(target_os="none")]
        let state = {
            use crate::runtime::ai::voice_conversation::{state,State};
            if !voice_enabled {
                b"VOICE OFF".as_slice()
            } else { match state().0 {
                State::Off if chat.generation_state==crate::runtime::ai::chat::GenerationState::Running=>b"Thinking...".as_slice(),
                State::Off=>b"VOICE OFF".as_slice(),State::Listening=>b"LISTEN".as_slice(),
                State::Recognizing=>b"HEARING".as_slice(),State::Thinking=>b"THINKING".as_slice(),
                State::Speaking=>b"SPEAKING".as_slice(),State::Stopping=>b"STOPPING".as_slice(),
                State::Failed=>b"UNAVAILABLE".as_slice(),
            }}
        };
        let state_width = self.ui_text_width(state, 1);
        let thinking = chat.generation_state == crate::runtime::ai::chat::GenerationState::Running
            && !chat.minimized();
        let state_x = left + width.saturating_sub(state_width + 72 * scale);
        let frame = unsafe { THINKING_ANIMATION.frame as usize };
        #[cfg(target_os="none")]
        {
            use crate::runtime::ai::voice_conversation::{state,State,indicator};
            let region=indicator::control(geometry.panel,scale);
            self.outline_rounded_rect(region.x.max(0)as usize,region.y.max(0)as usize,region.width as usize,region.height as usize,8*scale,46,107,137);
            let (voice,level)=state();
            if voice_enabled && voice==State::Listening{
                let x=region.x+6*scale as i32;
                let y=top as i32+23*scale as i32;
                let available=(state_x as i32-x-6*scale as i32).max(0)as usize;
                for offset in 1..available.min(42*scale){
                    let a=indicator::sample((offset-1)/scale,frame,level)*scale as i32;
                    let b=indicator::sample(offset/scale,frame,level)*scale as i32;
                    self.line(x+offset as i32-1,y+a,x+offset as i32,y+b,80,215,249);
                }
            }
        }
        self.ui_text_shaded(
            state_x,
            top + 15 * scale,
            state,
            139,
            184,
            207,
            1,
            false,
            thinking.then_some((frame * 3 * scale, 96 * scale)),
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
        if chat.minimized()
            || self.render_clip.is_some_and(|clip| clip.bottom <= top + 48 * scale)
        {
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
            crate::runtime::ai::with_ai_runtime(|runtime| {
                runtime.chat.set_timeline_scroll_metrics(0)
            });
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
            let bubble_width = timeline_width.saturating_sub(43 * scale);
            let text_width = bubble_width.saturating_sub(20 * scale);
            // Chat glyphs below use font scale 1, independently of the desktop
            // geometry scale. Scaling their baseline twice creates blank rows.
            let line_height = crate::runtime::ai::chat::response_line_height(
                UI_FONT_CELL_HEIGHT, UI_FONT_SIZE_PX, UI_FONT_NATIVE_SIZE_PX);
            let vertical_padding = 12 * scale;
            let message_gap = 7 * scale;
            let viewport_padding = 7 * scale;
            let mut content_height = viewport_padding * 2;
            for index in 0..chat.message_count() {
                let Some(message) = chat.message(index) else { continue; };
                let lines = self
                    .ui_text_wrapped_line_count(text_width, message.text(), usize::MAX)
                    .max(1);
                content_height = content_height
                    .saturating_add(lines * line_height + vertical_padding + message_gap);
            }
            content_height = content_height.saturating_sub(message_gap);
            let maximum_scroll = content_height.saturating_sub(timeline_height);
            let scroll_offset = crate::runtime::ai::with_ai_runtime(|runtime| {
                runtime.chat.set_timeline_scroll_metrics(maximum_scroll)
            });
            let viewport_bottom = timeline_top.saturating_add(timeline_height);
            let mut row_top = timeline_top as i32 + viewport_padding as i32
                - scroll_offset as i32;
            let previous_clip = self.render_clip;
            self.intersect_render_clip(
                timeline_left,
                timeline_top,
                timeline_width,
                timeline_height,
            );
            for index in 0..chat.message_count() {
                let Some(message) = chat.message(index) else {
                    continue;
                };
                let user = message.role == crate::runtime::ai::chat::ChatRole::User;
                let inset = if user { 28 * scale } else { 7 * scale };
                let lines = self
                    .ui_text_wrapped_line_count(text_width, message.text(), usize::MAX)
                    .max(1);
                let bubble_height = lines * line_height + vertical_padding;
                let row_bottom = row_top.saturating_add(bubble_height as i32);
                if row_bottom > timeline_top as i32 && row_top < viewport_bottom as i32 {
                    let visible_top = row_top.max(timeline_top as i32) as usize;
                    let visible_bottom = row_bottom.min(viewport_bottom as i32) as usize;
                    self.fill_rounded_rect_alpha(
                        timeline_left + inset,
                        visible_top,
                        bubble_width,
                        visible_bottom.saturating_sub(visible_top),
                        8 * scale,
                        if user { accent_r / 3 } else { 8 },
                        if user { accent_g / 3 } else { 27 },
                        if user { accent_b / 3 } else { 42 },
                        224,
                    );
                    self.ui_text_wrapped_compact_clipped(
                        timeline_left + inset + 10 * scale,
                        row_top + 6 * scale as i32,
                        text_width,
                        message.text(),
                        218,
                        231,
                        240,
                        line_height,
                        timeline_top,
                        viewport_bottom,
                    );
                }
                row_top = row_bottom.saturating_add(message_gap as i32);
            }
            self.render_clip = previous_clip;
            let scrollbar = layout.ai_chat_scroll_geometry(
                false,
                content_height,
                scroll_offset,
            );
            if scrollbar.maximum_scroll != 0 {
                self.fill_rounded_rect_alpha(
                    scrollbar.track.x.max(0) as usize,
                    scrollbar.track.y.max(0) as usize,
                    scrollbar.track.width as usize,
                    scrollbar.track.height as usize,
                    2 * scale,
                    31,
                    57,
                    74,
                    176,
                );
                self.fill_rounded_rect_alpha(
                    scrollbar.thumb.x.max(0) as usize,
                    scrollbar.thumb.y.max(0) as usize,
                    scrollbar.thumb.width as usize,
                    scrollbar.thumb.height as usize,
                    2 * scale,
                    accent_r,
                    accent_g,
                    accent_b,
                    238,
                );
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
                if chat.selected_model_ready() { b"Ask InfinityOS..." } else { b"Model unavailable" }
            } else {
                composer_text
            },
            if composer_text.is_empty() { 130 } else { 224 },
            if composer_text.is_empty() { 151 } else { 235 },
            if composer_text.is_empty() { 168 } else { 243 },
            1,
        );
        self.text_field_caret(
            composer_left + 12 * scale,
            composer_top,
            geometry.composer.height as usize,
            composer_text,
            true,
            3,
        );
        let send_left = geometry.send.x.max(0) as usize;
        let send_top = geometry.send.y.max(0) as usize;
        let send_width = geometry.send.width as usize;
        let send_height = geometry.send.height as usize;
        let send_ready = !composer_text.is_empty()
            && chat.selected_model_ready()
            && chat.generation_state != crate::runtime::ai::chat::GenerationState::Running;
        self.fill_rounded_rect_alpha(
            send_left,
            send_top + 2 * scale,
            send_width,
            send_height,
            11 * scale,
            0,
            6,
            13,
            180,
        );
        self.fill_rounded_rect_alpha(
            send_left,
            send_top,
            geometry.send.width as usize,
            geometry.send.height as usize,
            11 * scale,
            if send_ready { accent_r } else { accent_r / 3 },
            if send_ready { accent_g } else { accent_g / 3 },
            if send_ready { accent_b } else { accent_b / 3 },
            if send_ready { 248 } else { 190 },
        );
        self.outline_rounded_rect(
            send_left,
            send_top,
            send_width,
            send_height,
            11 * scale,
            if send_ready { 171 } else { 73 },
            if send_ready { 228 } else { 107 },
            if send_ready { 255 } else { 126 },
        );
        self.ui_text_centered_strong(
            send_left,
            send_width.saturating_sub(15 * scale),
            send_top + send_height.saturating_sub(UI_FONT_CELL_HEIGHT) / 2,
            b"Send",
            if send_ready { 250 } else { 158 },
            if send_ready { 253 } else { 177 },
            if send_ready { 255 } else { 190 },
            1,
        );
        let arrow_x = send_left.saturating_add(send_width).saturating_sub(17 * scale) as i32;
        let arrow_y = send_top.saturating_add(send_height / 2) as i32;
        let arrow_color = if send_ready { (250, 253, 255) } else { (120, 145, 160) };
        self.line(
            arrow_x - 4 * scale as i32,
            arrow_y,
            arrow_x + 3 * scale as i32,
            arrow_y,
            arrow_color.0,
            arrow_color.1,
            arrow_color.2,
        );
        self.line(
            arrow_x,
            arrow_y - 3 * scale as i32,
            arrow_x + 3 * scale as i32,
            arrow_y,
            arrow_color.0,
            arrow_color.1,
            arrow_color.2,
        );
        self.line(
            arrow_x,
            arrow_y + 3 * scale as i32,
            arrow_x + 3 * scale as i32,
            arrow_y,
            arrow_color.0,
            arrow_color.1,
            arrow_color.2,
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
        let Some(field) = crate::ui::installer_layout::configuration_template_input_rect(
            step,
            self.width,
            self.height,
        ) else {
            return;
        };
        let placeholder: &[u8] = match step {
            1 => b"InfinityNode",
            2 => b"your-handle",
            3 => b"Display name",
            4 => b"Create a password",
            _ => b"",
        };
        self.onboarding_input_field(
            field.left,
            field.top,
            field.width,
            field.height,
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
    task_manager_window: crate::ui::system_layout::DesktopAppWindowState,
    editor_scroll_row: usize,
    editor_dialog: u8,
    editor_dialog_input: &[u8],
    editor_dialog_focus: usize,
    fast_motion_frame: bool,
) {
    unsafe {
        let slot = &raw mut CONSOLE;
        if let Some(console) = (*slot).as_mut() {
            console.display.frame_started_ns = crate::ui::performance::monotonic_ns();
            console.display.fast_motion_frame = fast_motion_frame;
            console.system_ui_active = true;
            console.restore_cursor();
            let mut chat_content = crate::runtime::ai::with_ai_runtime(|runtime| runtime.chat.state_hash());
            #[cfg(target_os="none")]
            { chat_content ^= (crate::runtime::ai::voice_conversation::state().0 as u64).wrapping_mul(0x9e3779b97f4a7c15); }
            let chat_changed = console.last_chat_content != chat_content;
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
            ) ^ (chat_content as u32 ^ (chat_content >> 32) as u32) ^ crate::ui::text_input::presentation_hash()
                ^ crate::ui::object_picker::presentation().state_hash()
                ^ if screen == 10 {
                    u32::from(clock.second)
                } else {
                    0
                };
            let pointer_changed = console.cursor_x != cursor_x || console.cursor_y != cursor_y;
            let static_content = if screen == 8 {
                system_content_hash(&[], masked, output_lines, output_lengths, output_count,
                    editor_saved, editor_input, &[], editor_window, command_window,
                    editor_scroll_row, editor_dialog, editor_dialog_input, editor_dialog_focus)
                    ^ (chat_content as u32 ^ (chat_content >> 32) as u32)
            } else { 0 };
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
            if icon_theme_changed || accent_changed || primary_changed || background_effects_changed
            {
                console.display.fallback_reason =
                    crate::ui::performance::FallbackReason::AppearanceChange;
            }
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
            let network_settings = if screen == 4 && focus == 6 {
                crate::runtime::with_runtime(|runtime| (runtime.network.status(), runtime.network.interfaces.interface(2).copied(), runtime.network.resolver.server(0), runtime.network.resolver.server(1)))
            } else { None };
            let network_settings_changed = network_settings != console.last_network_settings;
            let node_settings = if screen == 4 && focus == 7 {
                crate::runtime::with_runtime(|runtime| crate::runtime::node_client::presentation(runtime))
            } else { None };
            let node_settings_changed = node_settings != console.last_node_settings;
            let pool_revision = if screen == 4 && focus == 8 {
                crate::runtime::with_runtime(|runtime|runtime.storage_view.revision)
            } else {None};
            let pool_settings_changed = super::retained_windows::invalidate_revision(
                4,&mut console.last_pool_settings_revision,pool_revision);
            let settings_content_changed = console.last_settings_window.expanded_row
                != settings_window.expanded_row
                || console.last_settings_window.scroll_offset != settings_window.scroll_offset
                || console.last_settings_window.control_focus != settings_window.control_focus
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
            let desktop_layer_focus_changed =
                crate::ui::redraw::desktop_layer_focus_change_uses_bounded_reconstruction(
                    console.last_system_screen,
                    screen,
                );
            let launcher_state = crate::ui::app_launcher::launcher_state_hash();
            let launcher_presentation = crate::ui::app_launcher::launcher_presentation();
            let launcher_interaction_state =
                crate::ui::app_launcher::launcher_interaction_state_hash();
            let launcher_state_changed = console.last_launcher_state != launcher_state;
            let launcher_interaction_changed =
                console.last_launcher_interaction_state != launcher_interaction_state;
            let structural_change_without_window = crate::ui::spatial::take_world_damage() || (!bounded_menu_change
                && ((console.last_system_screen != screen && !desktop_layer_focus_changed)
                    || (!desktop_layer_focus_changed
                        && crate::ui::redraw::focus_change_requires_structural_redraw(
                            screen,
                            pointer_changed,
                            focus_changed,
                        ))
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
            let (browser_revision,browser_page_key)={
                #[cfg(feature="native-browser")]
                {let view=crate::runtime::browser::presentation();(view.revision,Some(view.page_key()))}
                #[cfg(not(feature="native-browser"))]
                {(0,None)}
            };
            let browser_chrome_only=browser_page_key.is_some_and(|key|
                infinity_browser_core::damage::chrome_only(LAST_BROWSER_PAGE_KEY,key));
            LAST_BROWSER_PAGE_KEY=browser_page_key;
            let browser_damage_rect={
                let state=crate::console::browser_window();
                let mut rect=layout.desktop_app_window_geometry(state.x,state.y,state.width,state.height,state.maximized).window;
                let scale=layout.scale().max(1).min((rect.width as usize/760).max(1));
                if browser_chrome_only {
                    if let Some(chrome)=infinity_browser_core::layout::Layout::new(rect.width,rect.height,scale as u32) {
                        rect.height=chrome.content.y.max(0) as u32;
                    }
                }
                rect
            };
            let browser_changed=core::mem::replace(&mut *(&raw mut LAST_BROWSER_REVISION),browser_revision)!=browser_revision
                && crate::console::browser_window().visible && matches!(screen,2|4|8|9|10|11);
            let thinking_header_changed = core::mem::replace(&mut *(&raw mut THINKING_HEADER_DIRTY), false);
            let command_input_only = screen == 8 && console.last_system_screen == 8
                && content_changed && static_content == console.last_system_static_content
                && !structural_change_without_window && !file_navigator_changed && !focus_changed
                && !window_moved && !window_resized && !app_window_geometry_changed
                && !settings_geometry_changed && menu_kind == 0 && console.last_system_menu == 0;
            if !command_input_only && (structural_change_without_window
                || content_changed
                || file_navigator_changed
                || focus_changed
                || launcher_interaction_changed
                || window_resized
                || settings_content_changed)
            {
                super::retained_windows::invalidate();
            }
            if network_settings_changed || node_settings_changed {
                super::retained_windows::invalidate();
            }
            let bounded_launcher_change = screen == 7
                && console.last_system_screen == 7
                && launcher_state_changed
                && !structural_change_without_window
                && !content_changed
                && !window_moved
                && !window_resized
                && !settings_geometry_changed
                && !app_window_geometry_changed;
            let bounded_scene_geometry_change = !structural_change_without_window
                && (!content_changed || matches!(screen, 8 | 9 | 10 | 11) || (screen == 4 && chat_changed))
                && console.last_system_screen == screen
                && (navigator_surface_changed
                    || network_settings_changed
                    || node_settings_changed
                    || pool_settings_changed
                    || window_moved
                    || window_resized
                    || settings_geometry_changed
                    || app_window_geometry_changed
                    || browser_changed
                    || (chat_changed && screen == 4)
                    || (content_changed && matches!(screen, 8 | 9 | 10 | 11)));
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
            } else if bounded_launcher_change {
                let padding = (12 * layout.scale()) as u32;
                let damage = if launcher_interaction_changed {
                    let panel = layout.app_launcher_geometry().panel;
                    crate::ui::system_layout::window_transition_damage(
                        panel,
                        panel,
                        display_rect,
                        padding,
                    )
                } else {
                    layout.app_launcher_transition_damage(
                        console.last_launcher_transition,
                        launcher_presentation.transition,
                        padding,
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
                    task_manager_window,
                    editor_scroll_row,
                    editor_dialog,
                    editor_dialog_input,
                    editor_dialog_focus,
                );
                console.display.clear_render_clip();
            } else if desktop_layer_focus_changed && !structural_change_without_window {
                let previous = if console.last_system_screen == 2 {
                    let bounds = layout.home_window_geometry_sized(
                        console.last_home_window_x,
                        console.last_home_window_y,
                        console.last_home_window_width,
                        console.last_home_window_height,
                        console.last_home_window_maximized,
                    );
                    crate::ui::geometry::Rect {
                        x: bounds.0 as i32,
                        y: bounds.1 as i32,
                        width: bounds.2 as u32,
                        height: bounds.3 as u32,
                    }
                } else {
                    layout
                        .desktop_app_window_geometry(
                            console.last_app_window_x,
                            console.last_app_window_y,
                            console.last_app_window_width,
                            console.last_app_window_height,
                            console.last_app_window_maximized,
                        )
                        .window
                };
                let current = if screen == 2 {
                    let bounds = layout.home_window_geometry_sized(
                        window_x,
                        window_y,
                        window_width,
                        window_height,
                        window_maximized,
                    );
                    crate::ui::geometry::Rect {
                        x: bounds.0 as i32,
                        y: bounds.1 as i32,
                        width: bounds.2 as u32,
                        height: bounds.3 as u32,
                    }
                } else {
                    layout
                        .desktop_app_window_geometry(
                            app_window_x,
                            app_window_y,
                            app_window_width,
                            app_window_height,
                            app_window_maximized,
                        )
                        .window
                };
                let damage = crate::ui::system_layout::window_transition_damage(
                    previous,
                    current,
                    display_rect,
                    ((crate::ui::app_assistant::TAB_WIDTH + 12) * layout.scale()) as u32,
                );
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
                    task_manager_window,
                    editor_scroll_row,
                    editor_dialog,
                    editor_dialog_input,
                    editor_dialog_focus,
                );
                console.display.clear_render_clip();
            } else if bounded_scene_geometry_change {
                let (previous_damage_window, current_damage_window, split_motion_damage) =
                    if command_input_only {
                        let content = layout.desktop_app_window_geometry(app_window_x, app_window_y,
                            app_window_width, app_window_height, app_window_maximized).content;
                        let scale = layout.scale() as u32;
                        // Include the complete prompt, caret and text antialiasing
                        // gutter; output/history and window chrome are unchanged.
                        let rect = crate::ui::geometry::Rect { x: content.x,
                            y: content.y + content.height.saturating_sub(54 * scale) as i32,
                            width: content.width, height: (54 * scale).min(content.height) };
                        super::retained_windows::invalidate_region(1, super::PresentRegion {
                            left: rect.x.max(0) as usize, top: rect.y.max(0) as usize,
                            right: rect.right().max(0) as usize, bottom: rect.bottom().max(0) as usize,
                        });
                        (rect, rect, false)
                    } else if browser_changed && !content_changed && !window_moved && !window_resized
                        && !app_window_geometry_changed && !settings_geometry_changed {
                        let rect=browser_damage_rect;
                        (rect,rect,false)
                    } else if screen == 2 && (window_moved || window_resized) {
                        let current = console.display.desktop_window_rect(
                            window_x,
                            window_y,
                            window_width,
                            window_height,
                        );
                        (
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
                            window_moved && !window_resized,
                        )
                    } else if screen == 2 {
                        let current = console.display.desktop_window_rect(
                            window_x,
                            window_y,
                            window_width,
                            window_height,
                        );
                        let rect = crate::ui::geometry::Rect {
                            x: current.0 as i32,
                            y: current.1 as i32,
                            width: current.2 as u32,
                            height: current.3 as u32,
                        };
                        (rect, rect, false)
                    } else if screen == 4 {
                        let previous = layout
                            .settings_window_geometry(console.last_settings_window)
                            .window;
                        let current = layout.settings_window_geometry(settings_window).window;
                        (
                            previous,
                            current,
                            previous.width == current.width && previous.height == current.height,
                        )
                    } else {
                        let previous = layout
                            .desktop_app_window_geometry(
                                console.last_app_window_x,
                                console.last_app_window_y,
                                console.last_app_window_width,
                                console.last_app_window_height,
                                console.last_app_window_maximized,
                            )
                            .window;
                        let current = layout
                            .desktop_app_window_geometry(
                                app_window_x,
                                app_window_y,
                                app_window_width,
                                app_window_height,
                                app_window_maximized,
                            )
                            .window;
                        (
                            previous,
                            current,
                            previous.width == current.width && previous.height == current.height,
                        )
                    };
                let padding = if command_input_only {
                    0
                } else {
                    ((crate::ui::app_assistant::TAB_WIDTH + 12) * layout.scale()) as u32
                };
                let mut damages = [
                    crate::ui::system_layout::window_transition_damage(
                        previous_damage_window,
                        current_damage_window,
                        display_rect,
                        padding,
                    ),
                    crate::ui::geometry::Rect {
                        x: 0,
                        y: 0,
                        width: 0,
                        height: 0,
                    },
                ];
                let split_damages = crate::ui::system_layout::window_motion_damage_regions(
                    previous_damage_window,
                    current_damage_window,
                    display_rect,
                    padding,
                );
                if browser_changed {
                    damages[0]=damages[0].union(browser_damage_rect);
                }
                let union_pixels = damages[0].width as u64 * damages[0].height as u64;
                let split_pixels = split_damages
                    .iter()
                    .map(|region| region.width as u64 * region.height as u64)
                    .sum::<u64>();
                let damage_count = if split_motion_damage
                    && !browser_changed
                    && previous_damage_window != current_damage_window
                    && split_pixels < union_pixels
                {
                    damages = split_damages;
                    2
                } else {
                    1
                };
                // A window repaint does not cover the independently owned chat
                // column. Keep that damage separate instead of inflating the
                // window's bounds or repainting the entire desktop.
                let chat_damage = crate::ui::redraw::chat_requires_independent_widget_damage(screen, chat_changed)
                    .then(|| layout.ai_chat_geometry(false).panel);
                for damage in damages.iter().take(damage_count).chain(chat_damage.iter()) {
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
                        task_manager_window,
                        editor_scroll_row,
                        editor_dialog,
                        editor_dialog_input,
                        editor_dialog_focus,
                    );
                }
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
                    task_manager_window,
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
                console.display.onboarding_focus_controls(
                    step,
                    input,
                    masked,
                    focus,
                    validation_error,
                );
            } else if crate::ui::redraw::authentication_controls_require_repaint(
                screen,
                pointer_changed,
                focus_changed,
            ) {
                console
                    .display
                    .authentication_focus_controls(screen == 6, step, input, focus);
            } else if screen == 7 && (focus_changed || content_changed) {
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
                    focus,
                    menu_kind,
                );
            } else if (matches!(screen,4|8|9|10|11) && thinking_header_changed && !content_changed) || crate::ui::redraw::desktop_chat_content_requires_bounded_redraw(
                screen,
                content_changed || thinking_header_changed,
            ) {
                let mut widgets = layout.ai_chat_geometry(false).panel;
                if thinking_header_changed && !content_changed {
                    widgets = layout.ai_chat_geometry(false).panel;
                    widgets.height = (48 * layout.scale()) as u32;
                }
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
                    task_manager_window,
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
                    task_manager_window,
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
            if matches!(screen, 2 | 4 | 7 | 8 | 9 | 10 | 11) {
                for damage in [crate::ui::app_launcher::minimized_shelf::take_damage(console.display.width, console.display.height),
                    crate::ui::desktop_widgets::take_damage(console.display.width, console.display.height,layout.scale()),
                    crate::ui::app_launcher::shortcuts::take_damage(console.display.width,console.display.height).map(|(x,y,w,h)|crate::ui::geometry::Rect{x:x as i32,y:y as i32,width:w as u32,height:h as u32})].into_iter().flatten() {
                    if !full_surface_redrawn {
                        console.display.set_render_clip(damage.x.max(0) as usize, damage.y.max(0) as usize, damage.width as usize, damage.height as usize);
                        console.display.system_ui_frame(screen, step, input, masked, focus, validation_error,
                            window_x, window_y, window_width, window_height, window_visible, window_maximized,
                            home_location, selected_item, dragging_item, note_location, desktop_items,
                            desktop_item_positions, clock, settings_window, menu_kind, output_lines, output_lengths,
                            output_count, app_window_x, app_window_y, app_window_width, app_window_height,
                            app_window_maximized, editor_saved, editor_input, command_input, editor_window,
                            command_window, task_manager_window, editor_scroll_row, editor_dialog,
                            editor_dialog_input, editor_dialog_focus);
                        console.display.clear_render_clip();
                    }
                }
            }
            if matches!(screen,2|4|7|8|9|10|11) {
                let shortcuts=crate::ui::app_launcher::shortcuts::current();
                if let Some((id,_,_,true))=shortcuts.drag {
                    console.display.desktop_app_shortcut(id,
                        console.display.width*shortcuts.pointer[0].clamp(0,1000) as usize/1000,
                        console.display.height*shortcuts.pointer[1].clamp(0,1000) as usize/1000,shortcuts.positions[id]==[0,0]);
                }
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
            console.last_chat_content = chat_content;
            console.last_system_static_content = static_content;
            console.last_launcher_state = launcher_state;
            console.last_launcher_interaction_state = launcher_interaction_state;
            console.last_launcher_transition = launcher_presentation.transition;
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
            console.last_network_settings = network_settings;
            console.last_node_settings = node_settings;
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

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: system_ui_authentication_opacity
// DESC: Fades the persistent composed scene without darkening its backing pixels or repainting application contents.
// ------------------=
pub fn system_ui_authentication_opacity(opacity: u8, present: bool) {
    unsafe {
        if let Some(console) = (*(&raw mut CONSOLE)).as_mut() {
            let display = &mut console.display;
            display.clear_render_clip();
            if display.presentation_opacity != opacity {
                display.presentation_opacity = opacity;
                display.mark_dirty_rect(0, 0, display.width, display.height);
            }
            if present { display.present_damage(); }
        }
    }
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: system_ui_authentication_opacity
// DESC: Preserves the legacy text-mode interface where framebuffer fading is unavailable.
// ------------------=
pub fn system_ui_authentication_opacity(_opacity: u8, _present: bool) {}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: system_ui_authentication_success
// DESC: Presents one bounded authentication-success frame while preserving coherent cursor backing pixels.
// ------------------=
pub fn system_ui_authentication_success(
    presentation: crate::ui::authentication_motion::Presentation,
) {
    unsafe {
        if let Some(console) = (*(&raw mut CONSOLE)).as_mut() {
            console.display.frame_started_ns = crate::ui::performance::monotonic_ns();
            console.display.clear_render_clip();
            console.restore_cursor();
            console.display.authentication_success_frame(presentation);
            console.save_and_draw_cursor(console.cursor_x, console.cursor_y);
            console.display.present_damage();
        }
    }
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: system_ui_authentication_success
// DESC: Keeps the authentication-success presentation API available on the legacy text architecture.
// ------------------=
pub fn system_ui_authentication_success(
    _presentation: crate::ui::authentication_motion::Presentation,
) {
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
    _task_manager_window: crate::ui::system_layout::DesktopAppWindowState,
    _editor_scroll_row: usize,
    _editor_dialog: u8,
    _editor_dialog_input: &[u8],
    _editor_dialog_focus: usize,
    _fast_motion_frame: bool,
) {
}

// ------------------------=
// FUNC: system_ui_cursor
// DESC: Presents only saved and new cursor pixels; no application hashes, painting, or service calls.
// ------------------=
pub fn system_ui_cursor(x: i32, y: i32) {
    unsafe {
        if let Some(console) = (*(&raw mut CONSOLE)).as_mut() {
            console.display.frame_started_ns = crate::ui::performance::monotonic_ns();
            console.display.clear_render_clip();
            console.restore_cursor();
            console.cursor_x = x;
            console.cursor_y = y;
            console.save_and_draw_cursor(x, y);
            console.display.present_damage();
        }
    }
}

// ------------------------=
// FUNC: system_ui_editor_blink
// DESC: Updates the active editor caret and clock without invalidating retained application surfaces.
// ------------------=
pub fn system_ui_editor_blink(g: crate::ui::editor_chrome::Layout, input: &[u8], scroll: usize, clock: crate::storage::DateTimeConfiguration) {
    unsafe {
        if let Some(console) = (*(&raw mut CONSOLE)).as_mut() {
            console.display.frame_started_ns = crate::ui::performance::monotonic_ns();
            console.restore_cursor();
            console.display.editor_caret_blink(g, input, scroll);
            if console.last_system_clock != clock {
                console.display.system_top_bar_clock(clock);
                console.last_system_clock = clock;
            }
            console.save_and_draw_cursor(console.cursor_x, console.cursor_y);
            console.display.present_damage();
        }
    }
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
    for byte in crate::ui::input_preferences::current().encode().iter().chain(crate::ui::spatial::backdrop_tint().iter()) {
        hash=(hash ^ *byte as u32).wrapping_mul(0x0100_0193);
    }
    hash ^= crate::ui::personalization::save_status() as u32;
    for row in 0..output_count.min(6) {
        for byte in &output_lines[row][..output_lengths[row].min(96)] {
            hash ^= *byte as u32;
            hash = hash.wrapping_mul(0x0100_0193);
        }
    }
    hash ^= editor_saved as u32;
    hash ^= crate::ui::editor_tools::revision().wrapping_mul(0x01000193);
    hash ^= crate::ui::app_assistant::revision().rotate_left(13);
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
