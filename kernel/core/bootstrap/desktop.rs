//! Authentication, onboarding, desktop shell, menus, settings, windows, and file management.

use super::*;
use super::access_style as access;
use crate::ui::panel_recipe::Surface as AccessSurface;
static SETTINGS_PRESENTATION_DIRTY: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

// ------------------------=
// FUNC: system_ui_settings_changed
// DESC: Invalidates desktop and retained settings surfaces once after a native settings mutation.
// ------------------=
pub fn system_ui_settings_changed() {
    SETTINGS_PRESENTATION_DIRTY.store(true, core::sync::atomic::Ordering::Release);
}
#[path = "minimized_shelf.rs"]
mod minimized_shelf;
#[path = "desktop_widget_menu.rs"]
mod desktop_widget_menu;
#[path = "launcher_view.rs"]
mod launcher_view;
#[path = "launcher_icons.rs"]
mod launcher_icons;
#[path = "glass.rs"]
mod glass;
#[path = "../../ui/dock_material.rs"]
mod dock_material;
static mut DOCK_BACKDROP: dock_material::Backdrop = dock_material::Backdrop::new();

static mut THINKING_ANIMATION: crate::ui::thinking::ThinkingAnimation =
    crate::ui::thinking::ThinkingAnimation::new();
static mut THINKING_HEADER_DIRTY: bool = false;
static mut LAST_BROWSER_REVISION:u64=0;
static mut LAST_ASSISTANT_REVISION: u32 = 0;
static mut LAST_BROWSER_PAGE_KEY:Option<infinity_browser_core::damage::PageKey>=None;

impl DisplayDevice {
    // ------------------------=
    // FUNC: native_gui_window
    // DESC: Presents real toolkit client pixels with the existing glass title, Inter typography, authored controls and exterior resize affordances.
    // ------------------=
    pub(super) fn native_gui_window(&mut self, window: crate::ui::window::Window, frame: crate::ui::compositor::SurfaceFrame<'_>, active:bool) {
        use crate::ui::window::WindowState;
        self.native_gui_surface(frame);
        let scale=self.ui_scale();
        let title=crate::runtime::native_gui::title_bounds(window,scale);
        if title.height==0{return;}
        let old_clip=self.render_clip;
        let left=title.x.max(0) as usize;
        let top=title.y.max(0) as usize;
        let right=title.right().max(0).min(self.width as i32) as usize;
        let bottom=title.bottom().max(0).min(self.height as i32) as usize;
        if right>left&&bottom>top {
            self.intersect_render_clip(left,top,right-left,bottom-top);
            self.glass_window_substrate(title,true);
            let controls=crate::ui::app_launcher::window_controls::bounds(title,title.height,scale);
            let tx=title.x.saturating_add((16*scale) as i32);
            let available=controls[0].x.saturating_sub(tx).saturating_sub((12*scale) as i32).max(0) as usize;
            if tx>=0&&title.y>=0 {
                let mut text=window.title().as_bytes();
                while !text.is_empty()&&self.app_text_width(text,true,scale)>available {text=&text[..text.len()-1];}
                self.app_text(tx as usize,title.y as usize+10*scale,text,(226,242,252),true,scale);
            }
            for (index,rect) in controls.into_iter().enumerate() {
                if rect.x>=0&&rect.y>=0 {
                    self.window_control(rect.x as usize,rect.y as usize,rect.width as usize,index,window.state==WindowState::Maximized);
                }
            }
            self.render_clip=old_clip;
        }
        let bounds=if window.visible {window.bounds.union(title)}else{title};
        self.precision_window_frame(bounds,active,scale);
    }
}

// ------------------------=
// FUNC: thinking_animation_tick
// DESC: Schedules a bounded header repaint on the UI thread while generation is visible.
// ------------------=
pub fn thinking_animation_tick(visible: bool) -> bool {
    let mut active = visible && crate::runtime::ai::with_ai_runtime(|ai|
        ai.chat.enabled() && !ai.chat.minimized()
        && ai.chat.generation_state == crate::runtime::ai::chat::GenerationState::Running);
    #[cfg(target_os="none")]
    { active |= visible && matches!(crate::runtime::ai::voice_conversation::state().0,
        crate::runtime::ai::voice_conversation::State::Listening
        | crate::runtime::ai::voice_conversation::State::Starting); }
    let now = crate::ui::performance::monotonic_ns().unwrap_or(0);
    unsafe {
        let veil = visible && crate::runtime::ai::with_ai_runtime(|ai|
            ai.chat.enabled() && !ai.chat.minimized() && !ai.chat.selected_model_ready());
        let animation = &mut *(&raw mut THINKING_ANIMATION);
        let changed = animation.set_veil(veil) | animation.advance(active || veil, now);
        if changed {
            #[cfg(target_os="none")]
            crate::ui::ai_widget::update_meter(crate::drivers::audio::take_meter_peak());
            if !active { crate::ui::ai_widget::clear_meter(); }
        }
        THINKING_HEADER_DIRTY |= changed;
        changed
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) const AUTHENTICATION_BMP: &[u8] =
    include_bytes!("../../../assets/ui-design-kit/default/eclipse-access-wallpaper-v1.bmp");
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
    include_bytes!("../../../assets/ui-design-kit/default/eclipse-access-wallpaper-v1.bmp");
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
    // FUNC: clipboard_overlay
    // DESC: Paints native clipboard actions and bounded paged text using the shared menu surface.
    // ------------------=
    pub(super) fn clipboard_overlay(&mut self) {
        let menu=crate::console::clipboard_menu::current();if !menu.open {return;}
        let s=menu.scale;let x=menu.rect.x.max(0) as usize;let y=menu.rect.y.max(0) as usize;
        let w=menu.rect.width as usize;
        self.app_card_alpha(menu.rect,(5,18,31),(73,180,229),250,s);
        if !menu.viewer {
            for (row,label) in crate::console::clipboard_menu::LABELS.iter().enumerate() {
                let label=if row==6 && menu.target==crate::console::clipboard_menu::Target::Browser {b"Save image to Downloads" as &[u8]}else{*label};
                let top=y+(6+row*30)*s;
                if row==menu.row {self.fill_rounded_rect_alpha(x+5*s,top,w-10*s,30*s,4*s,20,66,91,255);}
                self.app_text(x+14*s,top+3*s,label,
                    if row==6 && !matches!(menu.target,crate::console::clipboard_menu::Target::Editor|crate::console::clipboard_menu::Target::Browser) {(83,111,132)}else{(215,231,241)},false,s);
            }
            return;
        }
        self.app_text(x+16*s,y+12*s,b"Visual Clipboard",(231,239,245),true,s);
        self.app_text(x+w.saturating_sub(58*s),y+12*s,b"Close",(100,215,245),false,s);
        self.fill_rounded_rect_alpha(x+16*s,y+48*s,w.saturating_sub(32*s),36*s,8*s,3,16,28,255);
        self.outline_rounded_rect(x+16*s,y+48*s,w.saturating_sub(32*s),36*s,8*s,54,204,237);
        self.app_label(crate::ui::geometry::Rect{x:(x+26*s) as i32,y:(y+54*s) as i32,width:w.saturating_sub(52*s) as u32,height:(24*s) as u32},
            if menu.query_length==0 {b"Type to search retained copies..."}else{&menu.query[..menu.query_length]},(190,214,230),false,s);
        for slot in 0..3 {
            let index=menu.row/3*3+slot;
            let Some(entry)=menu.entries.get(index).filter(|e|e.id!=0) else {continue;};
            let top=y+(100+slot*64)*s;
            self.app_card_alpha(crate::ui::geometry::Rect{x:(x+16*s) as i32,y:top as i32,
                width:(184*s) as u32,height:(56*s) as u32},(12,37,54),
                if index==menu.row {(54,204,237)}else{(39,71,89)},245,s);
            let mut label=entry.bytes;for c in &mut label {if c.is_ascii_control(){*c=b' ';}}
            self.app_label(crate::ui::geometry::Rect{x:(x+26*s) as i32,y:(top+6*s) as i32,width:(164*s) as u32,height:(24*s) as u32},&label[..entry.length],(223,237,245),false,s);
            self.app_text(x+26*s,top+30*s,b"Text  /  this session",(147,182,200),false,s);
        }
        self.app_text(x+220*s,y+100*s,if menu.selected_id==0 {b"No retained copies"}else{b"Text preview"},(223,237,245),true,s);
        let start=(menu.page*192).min(menu.length);let end=(start+192).min(menu.length);
        for (row,line) in menu.bytes[start..end].chunks(32).enumerate() {
            let mut label=[b' ';32];for (n,&c) in line.iter().enumerate() {label[n]=if c.is_ascii_control(){b' '}else{c};}
            self.app_label(crate::ui::geometry::Rect{x:(x+220*s) as i32,y:(y+(128+row*24)*s) as i32,width:w.saturating_sub(236*s) as u32,height:(24*s) as u32},&label[..line.len()],(190,214,230),false,s);
        }
        self.app_text(x+220*s,y+280*s,b"Left / Right: preview pages",(147,182,200),false,s);
        for (i,label) in [b"Previous".as_slice(),b"Next",b"Copy",b"Paste",b"Remove",b"Clear all"].iter().enumerate() {
            let left=x+(16+i*80)*s;
            self.fill_rounded_rect_alpha(left,y+308*s,74*s,32*s,6*s,12,37,54,255);
            self.app_label(crate::ui::geometry::Rect{x:(left+4*s) as i32,y:(y+312*s) as i32,width:(66*s) as u32,height:(24*s) as u32},label,
                if i==3 && !menu.can_paste {(83,111,132)}else{(190,230,244)},false,s);
        }
    }
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
    // DESC: Reuses the generated search artwork in the live installer; other roles retain their fallbacks.
    // ------------------=
    pub(super) fn themed_icon(
        &mut self,
        _center_x: usize,
        _center_y: usize,
        _role: usize,
        _size: usize,
    ) -> bool {
        #[cfg(all(feature = "installer", any(target_arch = "aarch64", target_arch = "x86_64")))]
        if _role == 27 {
            return self.paint_bitmap_alpha_atlas_cell(
                include_bytes!("../../../assets/icons/runtime/crystal-blue-glass-base.bmp"),
                5, 9, 27,
                _center_x.saturating_sub(_size / 2),
                _center_y.saturating_sub(_size / 2), _size,
            );
        }
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
        if crate::ui::user_wallpaper::bitmap(|b|{if let Some(b)=b{self.paint_bitmap_cover_rect(b,0,0,self.width,self.height);true}else{false}}){return;}
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
        if self.skin_visual_mode() == 0 && self.repaint_login_motion(presentation) { return; }
        if self.skin_visual_mode() != 0
            || !crate::ui::installer_layout::login_uses_legacy_animation(
                crate::ui::installer_layout::LOGIN_TEMPLATE_BYTES,
            )
        {
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
        if crate::ui::user_wallpaper::bitmap(|b|{if let Some(b)=b{self.paint_bitmap_cover_rect(b,left,top,width,height);true}else{false}}){return;}
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
        let account = crate::runtime::with_runtime(|runtime| runtime.identity.user_nth(selected_user)).flatten();
        if self.login_template_scene(crate::ui::installer_layout::LOGIN_TEMPLATE_BYTES,
            account.as_ref().map(|user| user.display_name.as_bytes()).unwrap_or(b"Select an account"), input, focus) {
            return;
        }
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
            self.fill_rect_alpha(0, 0, self.width, top_height, 9, 15, 38, 238);
            self.outline_rect(0, top_height.saturating_sub(1), self.width, 1, 67, 88, 140);
            self.top_bar_infinity_icon(sx(57), top_height / 2, sw(64));
            self.access_label((sx(92), 0, sw(650), top_height),
                b"WELCOME TO I N F I N I T Y O S", sw(21), access::TEXT, false, false);
            let machine = crate::runtime::with_runtime(|runtime| runtime.identity.machine()).flatten();
            let name = machine.map(|value| value.display_name)
                .unwrap_or(crate::runtime::identity::ShortText::empty());
            self.access_label((sx(1210), 0, sw(240), top_height),
                name.as_bytes(), sw(20), access::SECONDARY, true, false);
            self.access_icon(sx(1497), top_height / 2, 1, sw(21), false);
        }

        let card_x = sx(54);
        let card_y = sy(123);
        let card_w = sw(521);
        let card_h = sw(754);
        if paint_header {
            self.access_wordmark(sx(900), sy(92), sw(570), sw(92));
        }
        self.access_panel(card_x, card_y, card_w, card_h);
        self.top_bar_infinity_icon(card_x + card_w / 2, card_y + sw(91), sw(156));
        self.access_label((card_x + sw(28), card_y + sw(148), card_w - sw(56), sw(49)),
            if locked { b"Welcome Back" } else { b"Welcome to InfinityOS" },
            sw(36), access::TEXT, true, true);
        self.access_label((card_x + sw(28), card_y + sw(202), card_w - sw(56), sw(31)),
            b"Secure. Limitless.", sw(22), access::SECONDARY, true, false);

        let inner_x = card_x + sw(49);
        let inner_w = card_w.saturating_sub(sw(98));
        let user_y = card_y + sw(253);
        self.access_surface(
            inner_x,
            user_y,
            inner_w,
            sw(86),
            AccessSurface::Panel,
            access::BACKGROUND, access::BORDER, 255, focus == 0,
        );
        self.access_icon(inner_x + sw(45), user_y + sw(43), 2, sw(50), focus == 0);
        let user = crate::runtime::with_runtime(|runtime| runtime.identity.user_nth(selected_user))
            .flatten();
        self.access_label((inner_x + sw(91), user_y + sw(17), inner_w - sw(140), sw(29)),
            user.as_ref().map(|value| value.display_name.as_bytes()).unwrap_or(b"Select an account"),
            sw(22), access::TEXT, false, true);
        self.access_label((inner_x + sw(91), user_y + sw(49), inner_w - sw(140), sw(24)),
            b"Local account on this device", sw(17), access::SECONDARY, false, false);
        self.access_icon(
            inner_x + inner_w - sw(28),
            user_y + sw(43),
            3,
            sw(15),
            focus == 0,
        );

        let password_y = card_y + sw(356);
        self.authentication_password_field(inner_x, password_y, inner_w, sw(57), input, focus == 1);

        let sign_y = card_y + sw(439);
        self.access_surface(
            inner_x,
            sign_y,
            inner_w,
            sw(crate::ui::system_layout::UI_HERO_ACTION_HEIGHT),
            AccessSurface::Primary,
            access::PRIMARY, access::BORDER, 255, focus == 2,
        );
        self.access_label((inner_x + sw(40), sign_y, inner_w - sw(80),
            sw(crate::ui::system_layout::UI_HERO_ACTION_HEIGHT)),
            if locked { b"Unlock" } else { b"Sign In" }, sw(22), access::TEXT, true, true);
        self.access_icon(
            inner_x + inner_w - sw(30),
            sign_y + sw(28),
            4,
            sw(18),
            focus == 2,
        );

        let separator_y = card_y + sw(527);
        self.fill_rect(inner_x, separator_y, inner_w, 1, 67, 88, 140);
        self.fill_rect_alpha(
            inner_x + inner_w / 2 - sw(23),
            separator_y - sw(10),
            sw(46),
            sw(20),
            16,
            27,
            58,
            255,
        );
        self.access_label((inner_x + inner_w / 2 - sw(23), separator_y - sw(12), sw(46), sw(24)),
            b"or", sw(18), access::SECONDARY, true, false);

        let key_y = card_y + sw(557);
        self.access_surface(
            inner_x,
            key_y,
            inner_w,
            sw(crate::ui::system_layout::UI_HERO_ACTION_HEIGHT),
            AccessSurface::Secondary,
            access::ELEVATED, access::BORDER, 255, focus == 3,
        );
        self.access_label((inner_x + sw(28), key_y, inner_w - sw(72),
            sw(crate::ui::system_layout::UI_HERO_ACTION_HEIGHT)),
            b"Sign In with Facial Recognition", sw(21), access::TEXT, true, false);
        self.access_icon(
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
            self.access_icon(center, actions_y, 6 + index, sw(27), focus == index + 4);
            self.access_label((inner_x + third * index + sw(4), actions_y + sw(25),
                third - sw(8), sw(33)), label, sw(18),
                if focus == index + 4 { access::TEXT } else { access::SECONDARY }, true, false);
        }

        if focus < 7 { self.access_panel_glow(card_x,card_y,card_w,card_h); }
        self.access_panel_entrance((card_x,card_y,card_w,card_h),false);
        let tray_x = sx(529);
        let tray_y = sy(914);
        let tray_w = sw(478);
        let tray_h = sw(90);
        self.access_panel(tray_x, tray_y, tray_w, tray_h);
        let utility_labels: [&[u8]; 4] = [b"Shut Down", b"Restart", b"Network", b"Accessibility"];
        for (index, label) in utility_labels.iter().enumerate() {
            let cell = tray_w / 4;
            let center = tray_x + cell * index + cell / 2;
            self.access_icon(
                center,
                tray_y + sw(31),
                9 + index,
                sw(25),
                focus == index + 7,
            );
            self.access_label((tray_x + cell * index + sw(5), tray_y + sw(52), cell - sw(10), sw(28)),
                label, sw(18), if focus == index + 7 { access::TEXT } else { access::SECONDARY }, true, false);
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
        if crate::ui::installer_template::InstallerTemplate::parse_login(crate::ui::installer_layout::LOGIN_TEMPLATE_BYTES).is_ok() {
            self.paint_authentication_background_rect(0, 0, self.width, self.height);
            self.authentication_frame(locked, selected_user, input, focus, true);
            return;
        }
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
        self.access_surface(x, y, width, height, AccessSurface::Field, access::BACKGROUND,
            access::BORDER, 255, focused);
        self.access_icon(x + height / 2, y + height / 2, 5, height / 3, focused);
        let previous_clip = self.render_clip;
        self.intersect_render_clip(x + height, y + 2,
            width.saturating_sub(height * 2), height.saturating_sub(4));
        let mut masked = [0u8; 64];
        let length = input.len().min(masked.len());
        masked[..length].fill(b'*');
        let pixels = (height * 21 / 57).max(8);
        let color = if input.is_empty() { access::SECONDARY } else { access::TEXT };
        let text = if input.is_empty() { b"Enter your password".as_slice() } else { &masked[..length] };
        let text_top = y + height.saturating_sub(pixels * 7 / 6) / 2;
        self.access_text(x + height, text_top, text, color.0, color.1, color.2, 255, pixels, false);
        if let Some((true, index)) = crate::ui::text_input::caret(1).filter(|_| focused) {
            let caret_x = x + height + self.access_text_width(&masked[..index.min(length)], pixels, false);
            self.fill_rect(caret_x, text_top, 1, pixels * 7 / 6,
                access::FOCUS.0, access::FOCUS.1, access::FOCUS.2);
        }
        self.render_clip = previous_clip;
        self.access_icon(
            x + width - height / 2,
            y + height / 2,
            13,
            height / 3,
            focused,
        );
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
        let mut time = *b"00:00:00";
        time[0] = b'0' + clock.hour / 10;
        time[1] = b'0' + clock.hour % 10;
        time[3] = b'0' + clock.minute / 10;
        time[4] = b'0' + clock.minute % 10;
        time[6] = b'0' + clock.second / 10;
        time[7] = b'0' + clock.second % 10;
        self.desktop_template_scene(crate::ui::desktop_template::BYTES,1,
            &[(crate::ui::installer_template::InstallerTemplateRole::DesktopClock as u8,&time)],
            None,false,&[]);
        let _ = active_menu;
        crate::ui::desktop_template::bounds(1,2,self.width,self.height)
            .map(|r|r.bottom().max(0) as usize).unwrap_or(0)
    }

    // ------------------------=
    // FUNC: system_top_bar_clock
    // DESC: Reconstructs the authored clock region from wallpaper and all intersecting top-bar layers.
    // ------------------=
    pub(super) fn system_top_bar_clock(&mut self, clock: crate::storage::DateTimeConfiguration) {
        crate::ui::status_menu::set_today(clock.year, clock.month, clock.day);
        let Some(r)=crate::ui::desktop_template::bounds(1,52,self.width,self.height) else{return;};
        let saved=self.render_clip;
        self.intersect_render_clip(r.x.max(0) as usize,r.y.max(0) as usize,r.width as usize,r.height as usize);
        self.paint_desktop_background_rect(r.x.max(0) as usize,r.y.max(0) as usize,r.width as usize,r.height as usize);
        self.system_top_bar(None,clock);
        self.render_clip=saved;
    }

    // ------------------------=
    // FUNC: system_identity_bar
    // DESC: Draws the minimal trusted first-boot bar without presenting desktop commands before a session exists.
    // ------------------=
    pub(super) fn system_identity_bar(&mut self) -> usize {
        let scale = self.ui_scale().max(1);
        let height = (46 * scale).min(self.height / 12).max(40);
        self.fill_rect_alpha(0, 0, self.width, height,
            access::BACKGROUND.0, access::BACKGROUND.1, access::BACKGROUND.2, 232);
        self.fill_rect_alpha(0, height.saturating_sub(1), self.width, 1,
            access::MUTED_BORDER.0, access::MUTED_BORDER.1, access::MUTED_BORDER.2, 170);
        self.top_bar_infinity_icon(35 * scale, height / 2, 48 * scale);
        self.ui_text_strong(
            64 * scale,
            height.saturating_sub(UI_FONT_CELL_HEIGHT) / 2,
            b"INFINITYOS",
            access::TEXT.0,
            access::TEXT.1,
            access::TEXT.2,
            1,
        );
        let status = b"LOCAL  |  PRIVATE";
        let status_width = self.ui_text_width(status, 1);
        let wifi_center = self.width.saturating_sub(29 * scale);
        self.ui_text(
            wifi_center.saturating_sub(status_width + 36 * scale),
            height.saturating_sub(UI_FONT_CELL_HEIGHT) / 2,
            status,
            access::SECONDARY.0,
            access::SECONDARY.1,
            access::SECONDARY.2,
            1,
        );
        self.access_icon(wifi_center, height / 2, 1, 18 * scale, false);
        height
    }

    // ------------------------=
    // FUNC: system_menu_panel
    // DESC: Draws a native vector-backed desktop menu for the active top-bar domain.
    // ------------------=
    pub(super) fn system_menu_panel(&mut self, menu_kind: usize, focus: usize, scale: usize) {
        if menu_kind==0&&self.authored_infinity_menu(scale){return;}
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
    // DESC: Renders the kit's circular progress markers with completed, current, and remaining states.
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
            let diameter = 24.min(segment_width);
            let center_x = x + segment_width / 2;
            self.access_surface(center_x - diameter / 2, top, diameter, diameter,
                if index <= visual_step {AccessSurface::Current}else{AccessSurface::Inactive},
                if index <= visual_step { access::PRIMARY } else { access::BACKGROUND },
                access::BORDER, 255, false);
            if index<visual_step {self.access_glyph(center_x,top+diameter/2,15,diameter*3/4,false);}
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
        self.app_button_surface(crate::ui::geometry::Rect{x:left as i32,y:top as i32,
            width:width as u32,height:height as u32},false,0,false,scale);
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
        #[cfg(all(feature="native-face",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        if step==7 {self.face_setup_frame(input,focus,validation_error);return;}
        #[cfg(all(feature="native-face",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        let step=if step==8 {7}else{step};
        let card=crate::ui::installer_layout::configuration_template_rect(step,
            crate::ui::installer_template::InstallerTemplateRole::Console,self.width,self.height);
        self.onboarding_frame_content(step,input,masked,focus,validation_error);
        if let Some(card)=card {
            if let Ok(template)=crate::ui::installer_template::InstallerTemplate::parse(
                crate::ui::installer_layout::CONFIGURATION_TEMPLATE_BYTES) {
                if let Some(element)=template.element(step.saturating_add(1)as u8,
                    crate::ui::installer_template::InstallerTemplateRole::Console) {
                    self.access_template_panel_glow(element);
                }
            }
            self.access_panel_entrance((card.left,card.top,card.width,card.height),true);
        }
    }

    // ------------------------=
    // FUNC: onboarding_frame_content
    // DESC: Paints authored configuration content within the presentation-only entrance aperture.
    // ------------------=
    fn onboarding_frame_content(
        &mut self,
        step: usize,
        input: &[u8],
        masked: bool,
        focus: usize,
        validation_error: bool,
    ) {
        let network=if step==6 {Some(self.onboarding_network_state())}else{None};
        if self.configuration_template_scene_data(crate::ui::installer_layout::CONFIGURATION_TEMPLATE_BYTES,
            step,focus,super::template_compositor::ConfigurationViewState{input,masked,validation_error,network}) {
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
        self.top_bar_infinity_icon(inner_left + 25 * scale, card_top + 34 * scale, 45 * scale);
        self.access_label((inner_left + 55 * scale, card_top + 20 * scale,
            inner_width / 2, 28 * scale), b"INFINITYOS", 18 * scale, access::TEXT, false, true);
        self.access_label((inner_left + inner_width * 2 / 3, card_top + 20 * scale,
            inner_width / 3, 28 * scale), b"LOCAL  |  PRIVATE", 12 * scale, access::SECONDARY, false, false);
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
        self.access_label((inner_left, content_top, inner_width, 24 * scale), eyebrow,
            14 * scale, access::BORDER, false, true);
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
        self.access_label((
            title_frame.map(|frame| frame.left).unwrap_or(inner_left),
            title_frame
                .map(|frame| frame.top)
                .unwrap_or(content_top + 35 * scale),
            title_frame.map(|frame| frame.width).unwrap_or(inner_width),
            title_frame.map(|frame| frame.height).unwrap_or(42 * scale)),
            authored_title, 32 * scale, access::TEXT, false, true);
        self.access_text_wrapped(
            body_frame.map(|frame| frame.left).unwrap_or(inner_left),
            body_frame
                .map(|frame| frame.top)
                .unwrap_or(content_top + 82 * scale),
            body_frame.map(|frame| frame.width).unwrap_or(inner_width),
            authored_description,
            access::SECONDARY.0, access::SECONDARY.1, access::SECONDARY.2,
            255, 16 * scale, false,
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
                self.access_surface(inner_left,row_top,inner_width,54*scale,AccessSurface::Panel,
                    access::BACKGROUND,access::BORDER,240,false);
                self.access_icon(
                    inner_left + 24 * scale,
                    row_top + 27 * scale,
                    *icon,
                    24 * scale,
                    true,
                );
                self.access_label((inner_left + 50 * scale, row_top + 5 * scale,
                    inner_width.saturating_sub(62 * scale), 23 * scale), heading,
                    16 * scale, access::TEXT, false, true);
                self.access_label((inner_left + 50 * scale, row_top + 28 * scale,
                    inner_width.saturating_sub(62 * scale), 21 * scale), detail,
                    14 * scale, access::SECONDARY, false, false);
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
            self.access_label((input_left, input_top.saturating_sub(28 * scale), input_width, 24 * scale),
                if step == 4 {
                    b"PASSWORD"
                } else {
                    b"PROFILE DETAIL"
                },
                14 * scale, access::SECONDARY, false, true);
            self.onboarding_input_field(
                step,
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
            self.access_label((input_left, input_top + input_height + 10 * scale, input_width, 24 * scale),
                helper, 14 * scale, if validation_error { access::ERROR } else { access::SECONDARY }, false, false);
        } else if step == 5 {
            let privacy_rows: [(&[u8], &[u8], usize); 4] = [
                (b"Local AI", b"On", 7usize),
                (b"Remote processing", b"Off", 11usize),
                (b"Voice and microphone", b"Off", 5usize),
                (b"Appearance", b"InfinityOS Default Dark", 8usize),
            ];
            for (index, (label, value, icon)) in privacy_rows.iter().enumerate() {
                let row_top = body_top + index * 54 * scale;
                self.access_surface(inner_left,row_top,inner_width,44*scale,AccessSurface::Panel,
                    access::BACKGROUND,access::BORDER,240,false);
                self.access_icon(
                    inner_left + 22 * scale,
                    row_top + 22 * scale,
                    *icon,
                    20 * scale,
                    false,
                );
                self.access_label((inner_left + 46 * scale, row_top + 8 * scale,
                    inner_width / 2 - 46 * scale, 28 * scale), label,
                    14 * scale, access::TEXT, false, true);
                self.access_label((inner_left + inner_width / 2 + 8 * scale, row_top + 8 * scale,
                    inner_width / 2 - 24 * scale, 28 * scale), value,
                    14 * scale, access::BORDER, false, false);
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
            self.access_surface(inner_left,body_top,inner_width,150*scale,AccessSurface::Panel,
                access::BACKGROUND,access::BORDER,240,false);
            self.top_bar_infinity_icon(
                inner_left + inner_width / 2,
                body_top + 48 * scale,
                82 * scale,
            );
            self.access_label((inner_left, body_top + 82 * scale, inner_width, 28 * scale),
                b"Everything is ready", 18 * scale, access::TEXT, true, true);
            self.access_label((inner_left, body_top + 110 * scale, inner_width, 24 * scale),
                b"Enter a secure, local-first InfinityOS session.", 14 * scale, access::SECONDARY, true, false);
        }

        self.onboarding_actions(step, focus);
    }

    // ------------------------=
    // FUNC: onboarding_network_state
    // DESC: Reads a bounded native network snapshot for ordered configuration rendering without performing service actions.
    // ------------------=
    fn onboarding_network_state(&self)->super::onboarding_network_view::NetworkViewState<'static> {
        use crate::runtime::network::types::{ConnectivityClass, LinkState, NetworkSetupMode};
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
        let selected_index = match selected {
            NetworkSetupMode::Wired => 0,
            NetworkSetupMode::Wireless => 1,
            NetworkSetupMode::Offline => 2,
            NetworkSetupMode::Automatic => snapshot.map(|value| {
                if value.wired_available { 0 } else if value.wireless_available { 1 } else { 2 }
            }).unwrap_or(2),
        };
        super::onboarding_network_view::NetworkViewState {
                selected:selected_index,
                connected:snapshot.map(|value|value.connectivity != ConnectivityClass::Offline).unwrap_or(false),
                wired_detail,wireless_detail,
            }
    }

    // ------------------------=
    // FUNC: onboarding_network_rows
    // DESC: Supplies native service state to the shared fallback network view when no valid authored scene exists.
    // ------------------=
    pub(super) fn onboarding_network_rows(&mut self,left:usize,top:usize,width:usize,focus:usize,validation_error:bool) {
        let frames=core::array::from_fn(|index|
            crate::ui::installer_layout::configuration_network_row_rect(index,self.width,self.height));
        self.onboarding_network_rows_from_state((left,top,width),frames,focus,validation_error,self.onboarding_network_state());
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
            self.access_button(
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
            self.access_button(
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
            self.access_button(
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
        #[cfg(all(feature="native-face",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        if step>=7 {
            self.paint_first_boot_background();self.onboarding_frame(step,input,masked,focus,validation_error);return;
        }
        // Restore the authored backdrop under the card, bounded to its exact frame.
        // This also contains focus-easing damage without repainting the wallpaper.
        let previous_clip=self.render_clip;
        if let Some(card)=crate::ui::installer_layout::configuration_template_rect(step,
            crate::ui::installer_template::InstallerTemplateRole::Console,self.width,self.height) {
            self.intersect_render_clip(card.left,card.top,card.width,card.height);
            self.paint_first_boot_background();
            self.onboarding_frame(step,input,masked,focus,validation_error);
            self.render_clip=previous_clip;
            return;
        }
        self.render_clip=previous_clip;
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
                step,
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
        if !super::retained_windows::application_paint_active(self.recording_surface) {
            let rect = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                .desktop_app_window_geometry(
                    window_x,
                    window_y,
                    window_width,
                    window_height,
                    maximized,
                )
                .window;
            self.retained_application_window(
                (screen.saturating_sub(8) + 1) as usize,
                (
                    rect.x,
                    rect.y,
                    rect.width as usize,
                    rect.height as usize,
                ),
                crate::ui::desktop_stack::current().active == (screen.saturating_sub(8) + 1) as usize,
                |target| {
                    target.desktop_native_app_window(
                        screen,
                        input,
                        output_lines,
                        output_lengths,
                        output_count,
                        window_x.max(0),
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
            self.glass_window_substrate(geometry.window, true);
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
            let native_ui=crate::ui::native_uikit::presentation();
            let authored_window = screen == 8 && crate::ui::installer_template::navigation::scene() != 0;
            let window_title:&[u8]=if authored_window {
                crate::ui::desktop_template::template().and_then(|t|t.screen_title(crate::ui::installer_template::navigation::scene())).unwrap_or(b"InfinityOS")
            } else if screen==8&&native_ui.active {
                &native_ui.title[..native_ui.title_length]
            } else if screen == 9 {
                b"Text Editor"
            } else if screen == 10 {
                b"Task Manager"
            } else {
                b"Command Window"
            };
            self.ui_text_elided_strong(
                left + 54 * scale,
                top + 16 * scale,
                if authored_window { width.saturating_sub(210 * scale) } else { 108 * scale },
                window_title,
                231,
                243,
                250,
            );
            if !authored_window { self.ui_text(
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
            ); }
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
                    if crate::ui::native_uikit::presentation().active {
                        b"NATIVE C  /  INFINITY UI KIT"
                    } else { b"INFINITY CONSOLE  /  LOCAL SESSION" },
                    190,
                    190,
                    190,
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
        if screen==8&&crate::ui::installer_template::navigation::scene()!=0 {
            let content=crate::ui::geometry::Rect{x:geometry.content.x,y:geometry.toolbar.y,width:geometry.content.width,
                height:(geometry.content.bottom()-geometry.toolbar.y).max(0)as u32};
            if self.authored_desktop_content(crate::ui::installer_template::navigation::scene(),content){return;}
        }
        if screen==8 && crate::ui::native_uikit::presentation().active {
            self.render_native_uikit(geometry.content,scale);
            return;
        }
        if screen == 10 {
            self.render_task_manager_dashboard(
                geometry,
                task_manager_selected,
                task_menu_open,
                scale,
            );
        } else {
            self.app_card(crate::ui::geometry::Rect {
                x:content_left as i32,y:content_top as i32,
                width:content_width as u32,height:content_height as u32,
            },(9,15,38),(74,99,143),scale);
        }
        let line_height = 24 * scale;
        if screen != 10 {
            for row in 0..output_count.min(6) {
                self.app_label(
                    crate::ui::geometry::Rect {
                        x: (content_left + 18 * scale) as i32,
                        y: (content_top + 18 * scale + row * line_height) as i32,
                        width: content_width.saturating_sub(36 * scale) as u32,
                        height: line_height as u32,
                    },
                    &output_lines[row][..output_lengths[row].min(96)],
                    (235, 240, 248),
                    false,
                    scale,
                );
            }
            let prompt_y = content_top + content_height.saturating_sub(42 * scale);
            self.app_card(crate::ui::geometry::Rect {
                x: (content_left + 10 * scale) as i32,
                y: prompt_y.saturating_sub(8 * scale) as i32,
                width: content_width.saturating_sub(20 * scale) as u32,
                height: (34 * scale) as u32,
            }, (16, 27, 58), (134, 199, 255), scale);
            self.ui_text_strong(
                content_left + 18 * scale,
                prompt_y,
                b"inf >",
                134,
                199,
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
    // FUNC: render_native_uikit
    // DESC: Renders declarative native C elements with the reusable theme-aware Infinity UI Kit recipe.
    // ------------------=
    fn render_native_uikit(&mut self,content:crate::ui::geometry::Rect,scale:usize) {
        let presentation=crate::ui::native_uikit::presentation();
        if !presentation.active{return;}
        let left=content.x.max(0) as usize;let top=content.y.max(0) as usize;
        let width=content.width as usize;let height=content.height as usize;
        self.app_card(content,self.active_accent_surface(crate::ui::skin::AccentSurface::Widget),
            self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline),scale);
        let usable_width=width;let usable_height=height;
        for element in presentation.elements.iter().flatten() {
            let x=left+(element.x as usize).saturating_mul(usable_width)/presentation.width as usize;
            let y=top+(element.y as usize).saturating_mul(usable_height)/presentation.height as usize;
            let w=(element.width as usize).saturating_mul(usable_width)/presentation.width as usize;
            let h=(element.height as usize).saturating_mul(usable_height)/presentation.height as usize;
            let label=&element.label[..element.label_length];
            match element.kind {
                crate::ui::native_uikit::ElementKind::Label=>{
                    self.ui_text_elided_strong(x,y+h.saturating_sub(UI_FONT_CELL_HEIGHT)/2,w,label,220,237,248);
                }
                crate::ui::native_uikit::ElementKind::Display=>{
                    self.precision_surface(crate::ui::geometry::Rect{x:x as i32,y:y as i32,width:w as u32,height:h as u32},
                        crate::ui::panel_recipe::Surface::Field,crate::ui::panel_recipe::Material{
                            fill:self.active_accent_surface(crate::ui::skin::AccentSurface::Widget),
                            edge:self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline),
                            opacity:255,edge_opacity:255,focus:0,pressed:false,scale});
                    let text_width=self.ui_text_width(label,1);
                    self.ui_text_elided_strong(x+w.saturating_sub(text_width+16*scale),
                        y+h.saturating_sub(UI_FONT_CELL_HEIGHT)/2,w.saturating_sub(24*scale),
                        label,235,247,253);
                }
                crate::ui::native_uikit::ElementKind::Button=>{
                    self.app_button_surface(crate::ui::geometry::Rect{x:x as i32,y:y as i32,width:w as u32,height:h as u32},
                        false,0,false,scale);
                    let text_width=self.ui_text_width(label,1);
                    self.ui_text_strong(x+w.saturating_sub(text_width)/2,
                        y+h.saturating_sub(UI_FONT_CELL_HEIGHT)/2,label,225,242,251,1);
                }
            }
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
        self.app_dropdown_panel(crate::ui::geometry::Rect{x:menu_left as i32,y:menu_top as i32,
            width:menu_width as u32,
            height:(crate::ui::app_menu::panel_height(4,false)*scale)as u32},scale);
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
            let row_top=menu_top+(crate::ui::app_menu::PANEL_PADDING
                +row*crate::ui::app_menu::ROW_HEIGHT)*scale;
            self.app_menu_label(
                crate::ui::geometry::Rect{x:(menu_left+14*scale)as i32,y:row_top as i32,
                    width:(18*scale)as u32,height:(crate::ui::app_menu::ROW_HEIGHT*scale)as u32},
                if marked { b"*" } else { b"" },
                (94,211,250),scale,
            );
            self.app_menu_label(
                crate::ui::geometry::Rect{x:(menu_left+32*scale)as i32,y:row_top as i32,
                    width:(menu_width-46*scale)as u32,
                    height:(crate::ui::app_menu::ROW_HEIGHT*scale)as u32},
                label,
                (220,232,240),scale,
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
        self.app_card_alpha(geometry.content,(1,9,19),(55,133,178),246,scale);

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
            self.app_card_alpha(crate::ui::geometry::Rect {
                x:card_left as i32,y:card_top as i32,width:card_width as u32,height:card_height as u32,
            },(5,23,39),(accent_r/2,accent_g/2,accent_b/2),236,scale);
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
            self.render_task_manager_menu(geometry, scale);
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
    ) {
        let left = geometry.toolbar.x.max(0) as usize + 10 * scale;
        let top = geometry.toolbar.bottom().max(0) as usize - 2 * scale;
        let width = 232 * scale;
        let labels: [&[u8]; 6] = [
            b"Launch File Navigator",
            b"Relaunch Selected",
            b"Pause / Resume",
            b"Throttle Selected",
            b"End Selected Task",
            b"Refresh Now",
        ];
        let panel = crate::ui::geometry::Rect{x:left as i32,y:top as i32,width:width as u32,
            height:(crate::ui::app_menu::panel_height(labels.len(),false)*scale)as u32};
        self.app_dropdown_panel(panel, scale);
        for (index, label) in labels.iter().enumerate() {
            let row_top = top + (crate::ui::app_menu::PANEL_PADDING
                + index * crate::ui::app_menu::ROW_HEIGHT) * scale;
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
            self.app_menu_label(
                crate::ui::geometry::Rect{x:(left+14*scale)as i32,y:row_top as i32,
                    width:(width-28*scale)as u32,
                    height:(crate::ui::app_menu::ROW_HEIGHT*scale)as u32},
                label,
                if index==4 {(255,149,149)}else{(218,232,241)},scale,
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
        self.glass_window_substrate(g.sheet,true);
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
        self.precision_window_frame(g.sheet,true,scale);
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
        self.glass_window_substrate(g.sheet,true);
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
        self.precision_window_frame(g.sheet,true,scale);
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
        let incoming_clip=self.render_clip;
        if matches!(screen,2|3|4|7|8|9|10|11) {
            if let Some(clip)=incoming_clip {
                let dock=crate::ui::system_layout::SystemLayout::new(self.width,self.height).desktop_foreground_geometry().dock;
                let (left,top,right,bottom)=dock_material::expand_damage(
                    (dock.x.max(0)as usize,dock.y.max(0)as usize,dock.width as usize,dock.height as usize),
                    (clip.left,clip.top,clip.right,clip.bottom));
                self.render_clip=Some(super::PresentRegion{left,top,right,bottom});
            }
        }
        self.mark_dirty_rect(0, 0, self.width, self.height);
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        if screen == 7 && super::launcher_backdrop::restore(self) {
            self.launcher_reveal(input, focus);
            self.render_clip=incoming_clip;
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
                crate::ui::desktop_effects::layer(Some(id));
                if id==stack.active && id!=0 {
                    let dim=crate::ui::desktop_effects::current().dim;
                    if dim!=0 {self.fill_rect_alpha(0,top_bar,self.width,self.height.saturating_sub(top_bar),3,10,19,dim);}
                }
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
                    if crate::ui::desktop_effects::opacity()==255 && id!=0 && !(id==2 && editor_dialog!=0) {self.window_assistant(if id==5 {0}else{id},rect,scale);}
                }
            }
        }
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        if matches!(screen,2|4|8|9|10|11) { crate::ui::desktop_effects::layer(None); self.minimized_app_shelf(); self.desktop_widget_menu(scale); }
        if screen==4 {self.render_clip=incoming_clip;return;}
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
            for (index,control) in crate::ui::app_launcher::window_controls::bounds(
                crate::ui::geometry::Rect{x:left as i32,y:top as i32,width:width as u32,height:height as u32},
                title_height as u32,scale).into_iter().enumerate() {
                self.window_control(
                    control.x.max(0) as usize,
                    control.y.max(0) as usize,
                    control.width as usize,
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
                    (b"InfinityOS", option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.1").as_bytes()),
                    (b"Architecture", if cfg!(target_arch="aarch64") { b"ARM64" } else { b"x86_64" }),
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
        self.render_clip=incoming_clip;
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
        if !super::retained_windows::application_paint_active(self.recording_surface) {
            let rect = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                .settings_window_geometry(settings_window)
                .window;
            self.retained_application_window(
                4,
                (
                    rect.x,
                    rect.y,
                    rect.width as usize,
                    rect.height as usize,
                ),
                crate::ui::desktop_stack::current().active == 4,
                |target| target.render_settings_window(focus, input, settings_window, scale),
            );
            return;
        }
        let chrome=self.paint_settings_chrome(focus,settings_window,scale);
        let geometry=chrome.geometry;
        let layout=crate::ui::system_layout::SystemLayout::new(self.width,self.height);
        let (primary_r,primary_g,primary_b)=chrome.primary;
        let (outline_r,outline_g,outline_b)=chrome.outline;
        let (selection_r,selection_g,selection_b)=chrome.selection;
        let panel_alpha=|base:u8|(u16::from(base)*u16::from(chrome.opacity)/100)as u8;
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
        let voice_profile = crate::runtime::with_runtime(|runtime| {
            (0..crate::runtime::identity::MAX_SESSIONS)
                .filter_map(|index| runtime.identity.session_nth(index))
                .find(|session| {
                    session.state == crate::runtime::identity::SessionState::Active
                })
                .and_then(|session| runtime.identity.voice_profile(session.user))
        })
        .flatten();
        let voice_enabled = voice_profile
            .map(|profile| {
                profile.enabled
                    && profile.activation != crate::runtime::identity::VoiceActivation::Disabled
            })
            .unwrap_or(false);
        let wake_word = voice_profile
            .map(|profile| profile.wake_word.phrase())
            .unwrap_or(b"Infinity");
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
        let output_voice = crate::runtime::identity::OUTPUT_VOICE_NAMES[voice_profile.map(|p| p.output_voice as usize).unwrap_or(0).min(4)];
        let local_only=voice_profile.and_then(|v|crate::runtime::with_runtime(|r|r.identity.ai_profile(v.user)).flatten())
            .map(|p|p.provider_policy==crate::runtime::identity::AiProviderPolicy::LocalOnly).unwrap_or(true);
        let basic_state=super::settings_view::BasicSettingsSnapshot{local_only,output_voice,chat_enabled,chat_model,voice_enabled,wake_word,speech_enabled,preferences,
            face_login_enabled:voice_profile.is_some_and(|v|crate::runtime::with_runtime(|r|r.identity.user_profile(v.user).is_some_and(|p|p.face_login_enabled)).unwrap_or(false)),
            timeout_minutes:parse_leading_u8(input).unwrap_or(crate::runtime::identity::DEFAULT_NO_ACTIVITY_TIMEOUT_MINUTES),
            timeout_maximum:crate::runtime::identity::MAX_NO_ACTIVITY_TIMEOUT_MINUTES};
        if super::settings_view::supports_basic_section(focus) {
            self.paint_basic_settings_body(focus,input,settings_window,scale,basic_state,chrome);
            self.render_clip=caller_clip;
            self.render_settings_overflow_chrome(geometry,settings_window,scale,chrome.outline);
            return;
        }
        let rows:[(&[u8],&[u8]);8]=match focus.min(10) {
            1 => [
                (b"Skin", match self.skin_visual_mode() { 1 => b"Diagnostic Light", 2 => b"Safe", _ => b"Cosmic Dark" }),
                (b"Icon Set", icon_theme.name()),
                (b"Primary", b"Custom color"),
                (b"Secondary", b"Custom color"),
                (b"Opacity", opacity_text),
                (b"Blur", &blur_value),
                (b"Fonts", crate::runtime::font::FontCatalog::new().by_id(crate::runtime::font_faces::selected() as u32).map(|f|f.family).unwrap_or(b"Roboto")),
                (b"Wallpaper", b"Cosmic Horizon"),
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
            _=>super::settings_view::basic_settings_rows(focus,input,basic_state),
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
            self.precision_surface(hero,crate::ui::panel_recipe::Surface::Panel,
                crate::ui::panel_recipe::Material { fill:(2,12,28),edge:(117,218,255),
                    opacity:0,edge_opacity:255,focus:0,pressed:false,scale });
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
            let row=self.paint_settings_row_summary(focus,index,label,value,settings_window,scale,chrome);
            if focus==8 && (5..=7).contains(&index) {
                let bounds=layout.settings_row_element_geometry(settings_window,focus,index,
                    crate::ui::installer_template::InstallerTemplateRole::SettingsDisclosure)
                    .unwrap_or(crate::ui::geometry::Rect{x:row.summary.right()-(30*scale)as i32,
                        y:row.summary.y+(20*scale)as i32,width:(20*scale)as u32,height:(20*scale)as u32});
                self.precision_radio(crate::ui::geometry::Rect{height:bounds.width,..bounds},
                    pool_object.is_some_and(|o|o.desired==(index-4)as u8),pool_object.is_some(),
                    crate::ui::panel_recipe::Material{fill:chrome.primary,edge:chrome.outline,
                        opacity:255,edge_opacity:255,focus:0,pressed:false,scale});
            }
            if settings_window.expanded_row == Some(index)
                && row.detail.intersects(geometry.viewport)
            {
                let detail_left = row.detail.x.max(0) as usize;
                let detail_top = row.detail.y.max(0) as usize;
                let detail_width = row.detail.width as usize;
                let detail_height = row.detail.height as usize;
                self.app_card_alpha(row.detail,
                    (primary_r/2,primary_g.saturating_mul(2)/3,primary_b.saturating_mul(2)/3),
                    (outline_r/2,outline_g/2,outline_b/2),panel_alpha(232),scale);
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
                } else if focus == 1 && index == 6 {
                    let row=layout.settings_row_geometry_for_section(settings_window,index,focus);
                    self.settings_font_library(row.detail,scale);
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
                    self.paint_settings_basic_detail(focus,index,row.detail,scale,basic_state,settings_window);
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
    // FUNC: render_node_settings_dashboard
    // DESC: Captures authorized native node observations for the shared immutable dashboard painter.
    // ------------------=
    fn render_node_settings_dashboard(&mut self,settings_window:crate::ui::system_layout::SettingsWindowState,scale:usize,input:&[u8]) {
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
        let selected = crate::runtime::with_runtime(|runtime| {
            let peer = runtime.node_selection?;
            let verification = runtime.nodes.local_id().and_then(|local| runtime.node_transport.trust.verification(local, peer));
            Some((peer, verification))
        }).flatten();
        let (policy_offset, policy) = crate::runtime::with_runtime(|runtime| {
            (runtime.node_policy_offset, selected.and_then(|(peer, _)| runtime.node_projection.nodes[..runtime.node_projection.node_count].iter().find(|record| record[..32] == peer.0).copied()))
        }).unwrap_or((0, None));
        use super::settings_dashboard_view::{NodeDashboardSnapshot,NodeVerificationSnapshot,VerificationState};
        let verification=selected.and_then(|(_,v)|v).map(|v|NodeVerificationSnapshot{code:v.code,fingerprint:v.fingerprint,remaining:crate::runtime::node_client::clock().map(|now|v.expires.saturating_sub(now)).unwrap_or(0),state:match v.state {crate::runtime::node::wire_trust::WireState::LocallyConfirmed=>VerificationState::Local,crate::runtime::node::wire_trust::WireState::RemotelyConfirmed=>VerificationState::Remote,crate::runtime::node::wire_trust::WireState::Confirmed=>VerificationState::Confirmed,_=>VerificationState::Pending}});
        self.paint_node_settings_dashboard(settings_window,scale,input,NodeDashboardSnapshot{identity_ready:snapshot.0,counts:[snapshot.1,snapshot.2,snapshot.3,snapshot.4,snapshot.5,snapshot.6,snapshot.7,snapshot.8],selected:selected.is_some(),verification,policy_offset,policy:policy.map(|record|core::array::from_fn(|i|record[86+i]))});
    }

    // ------------------------=
    // FUNC: render_network_settings_dashboard
    // DESC: Projects real network service observations into the shared dashboard without changing authority or actions.
    // ------------------=
    fn render_network_settings_dashboard(&mut self,settings_window:crate::ui::system_layout::SettingsWindowState,scale:usize,connectivity:&[u8],input:&[u8]) {
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
        use super::settings_dashboard_view::*;
        let ipv4=|value:crate::runtime::network::types::IpAddress|match value{crate::runtime::network::types::IpAddress::V4(v)=>Some(v),_=>None};
        self.paint_network_settings_dashboard(settings_window,scale,connectivity,input,NetworkDashboardSnapshot {
            status:NetworkStatusSnapshot{active_profile:status.active_profile,link_only:status.connectivity==crate::runtime::network::types::ConnectivityClass::LinkOnly,interfaces:status.interfaces,addresses:status.addresses,routes:status.routes,policies:status.policies},
            active_connections:diagnostics.active_connections as u64,
            setup:NetworkSetupSnapshot{selected:match setup.selected{crate::runtime::network::types::NetworkSetupMode::Automatic=>SetupMode::Automatic,crate::runtime::network::types::NetworkSetupMode::Wired=>SetupMode::Wired,crate::runtime::network::types::NetworkSetupMode::Wireless=>SetupMode::Wireless,crate::runtime::network::types::NetworkSetupMode::Offline=>SetupMode::Offline},wired_available:setup.wired_available,wireless_available:setup.wireless_available},
            interface:interface.map(|v|NetworkInterfaceSnapshot{enabled:v.enabled,link:match v.device.link_state{crate::runtime::network::types::LinkState::Up=>LinkState::Up,crate::runtime::network::types::LinkState::Down=>LinkState::Down,_=>LinkState::Unknown},hardware_address:v.device.hardware_address.is_some(),maximum_frame_size:v.device.maximum_frame_size as u64}),
            static_address:static_address.map(|v|AddressSnapshot{address:ipv4(v.address),prefix_length:v.prefix_length}),
            default_route:default_route.map(|v|RouteSnapshot{next_hop:v.next_hop.and_then(ipv4),metric:v.metric,is_static:v.source==crate::runtime::network::types::RouteSource::Static}),
            resolver_enabled,resolver_primary:resolver_primary.and_then(ipv4),resolver_secondary:resolver_secondary.and_then(ipv4),
            default_policy:match default_policy{crate::runtime::network::types::PolicyAction::Allow=>PolicyAction::Allow,crate::runtime::network::types::PolicyAction::Ask=>PolicyAction::Ask,_=>PolicyAction::Deny}});
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
        self.access_panel(left, top, width, height);
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
    // DESC: Reuses the quiet launcher plate with an outer glow, never a pressed fill or shifted icon.
    // ------------------=
    pub(super) fn launcher_dock_icon(
        &mut self,
        left: usize,
        top: usize,
        size: usize,
        active: bool,
    ) {
        let accent=self.active_accent_rgb();
        let glow=(size/46).max(1)*5;
        for y in -(glow as i32)..(size+glow) as i32 {
            for x in -(glow as i32)..(size+glow) as i32 {
                let (r,g,b,a)=dock_material::launcher_pixel(size,x,y,accent,active);
                if a!=0 {self.blend_color(left as i32+x,top as i32+y,r,g,b,a);}
            }
        }
        self.top_bar_infinity_icon(left + size / 2, top + size / 2, size * 3 / 4);
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
        self.window_assistant(11,panel,scale);
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
        let vignette = (self.width / 22).max(16 * scale);
        for step in 0..4 {
            let alpha = (22usize.saturating_sub(step * 4)) as u8;
            let inset = step * vignette / 4;
            let edge = (vignette / 4).max(1);
            self.fill_rect_alpha(inset, 0, edge, self.height, 0, 4, 12, alpha);
            self.fill_rect_alpha(
                self.width.saturating_sub(inset + edge),
                0,
                edge,
                self.height,
                0,
                4,
                12,
                alpha,
            );
        }
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
        for (index,file) in crate::ui::file_preview::desktop_files().iter().enumerate() {
            if file.length==0 {continue;}
            let r=crate::ui::file_preview::desktop_rect(index,self.width,self.height);
            if r.bottom() as usize>=self.height.saturating_sub(100*scale){continue;}
            self.themed_icon(r.x as usize+r.width as usize/2,r.y as usize+24,49,44);
            self.app_label(crate::ui::geometry::Rect{x:r.x,y:r.y+54,width:r.width,height:28},
                crate::runtime::object_navigation::namespace_basename(&file.path[..file.length]),(221,235,243),false,1);
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
            crate::ui::desktop_effects::layer(None);
            let (left, top, width, height) =
                crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                    .home_window_geometry_sized(
                        navigator.x,
                        navigator.y,
                        navigator.width,
                        navigator.height,
                        navigator.maximized,
                    );
            self.retained_application_window(6 + index, (left as i32, top as i32, width, height), false, |target| {
                target.paint_navigator_window(scale, navigator.x, navigator.y,
                    navigator.width, navigator.height, navigator.maximized,
                    home_location, None, Some(navigator.state));
            });
            if crate::ui::desktop_effects::opacity()==255 {self.window_assistant(5+index,crate::ui::geometry::Rect{x:left as i32,y:top as i32,width:width as u32,height:height as u32},scale);}
        }
        if window_visible {
            crate::ui::desktop_effects::layer(Some(0));
            if crate::ui::desktop_stack::current().active==0 {
                let dim=crate::ui::desktop_effects::current().dim;
                if dim!=0 {let top=crate::ui::system_layout::SystemLayout::new(self.width,self.height).top_bar_height(); self.fill_rect_alpha(0,top,self.width,self.height.saturating_sub(top),3,10,19,dim);}
            }
            let bounds = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
                .home_window_geometry_sized(
                    window_x,
                    window_y,
                    window_width,
                    window_height,
                    window_maximized,
                );
            let navigator_state = crate::runtime::with_runtime(|r| r.file_navigator).flatten();
            self.retained_application_window(active_navigator.map(|i| 6 + i).unwrap_or(0),
                (bounds.0 as i32,bounds.1 as i32,bounds.2,bounds.3),
                crate::ui::desktop_stack::current().active==0, |target| {
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
            if crate::ui::desktop_effects::opacity()==255 {self.window_assistant(5+active_navigator.unwrap_or(0),crate::ui::geometry::Rect{x:bounds.0 as i32,y:bounds.1 as i32,width:bounds.2 as u32,height:bounds.3 as u32},scale);}
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
        self.glass_window_substrate(crate::ui::geometry::Rect {
            x: browser_left as i32, y: browser_top as i32,
            width: browser_width as u32, height: browser_height as u32,
        }, false);
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
        for (index,control) in crate::ui::app_launcher::window_controls::bounds(
            crate::ui::geometry::Rect{x:browser_left as i32,y:browser_top as i32,width:browser_width as u32,height:browser_height as u32},
            title_h as u32,scale).into_iter().enumerate() {
            self.window_control(
                control.x.max(0) as usize,
                control.y.max(0) as usize,
                control.width as usize,
                index,
                window_maximized,
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
        if location_editing && navigator_state.is_some_and(|state|state.editor_selected) {
            self.fill_rect_alpha(location_left+12*scale,tool_top+7*scale,location_width.saturating_sub(24*scale),24*scale,20,90,140,220);
        }
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
        let workplace_notice=navigator_state.map(|state|state.workplace_notice).unwrap_or(0);
        if workplace_notice!=0 {
            let text:&[u8]=match workplace_notice {
                1=>b"Paste complete",2=>b"Ready to move",3=>b"Copied",4=>b"Transfer failed; source or destination unavailable",
                5=>b"No installed application can open this object",
                6=>b"The selected application is unavailable or incompatible",
                7=>b"Opened with the selected application",
                _=>b"File operation could not be completed",
            };
            let limit=browser_width.saturating_sub(sidebar_w+28*scale)/(8*scale);
            self.ui_text(browser_left+sidebar_w+14*scale,status_top+5*scale,&text[..text.len().min(limit)],142,185,210,1);
        } else {
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
            self.app_dropdown_panel(menu, scale);
            for (index, action) in actions.iter().enumerate() {
                let row_top = menu_top + (crate::ui::app_menu::PANEL_PADDING
                    + index * crate::ui::app_menu::ROW_HEIGHT) * scale;
                let row = crate::ui::geometry::Rect{x:(menu_left+5*scale)as i32,y:row_top as i32,
                    width:(menu.width as usize-10*scale)as u32,
                    height:(crate::ui::app_menu::ROW_HEIGHT*scale)as u32};
                if index == context.menu_selection as usize {
                    self.app_dropdown_row(row, true, scale);
                }
                let marked = action.handler_index().is_some_and(|handler| handler as u8 == context.context_default_handler);
                self.app_menu_label(
                    crate::ui::geometry::Rect{x:(menu_left+12*scale)as i32,y:row_top as i32,
                        width:(16*scale)as u32,height:(crate::ui::app_menu::ROW_HEIGHT*scale)as u32},
                    if marked { b"*" } else { b"" }, (94,211,250), scale,
                );
                self.app_menu_label(
                    crate::ui::geometry::Rect{x:(menu_left+30*scale)as i32,y:row_top as i32,
                        width:(menu.width as usize-44*scale)as u32,
                        height:(crate::ui::app_menu::ROW_HEIGHT*scale)as u32},
                    context.context_action_label(*action),
                    (215, 231, 241), scale,
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
            self.app_dropdown_panel(menu_rect, scale);
            let selected = navigator_state
                .map(|state| state.menu_selection as usize)
                .unwrap_or(0);
            for (index, label) in labels.iter().enumerate() {
                let row_top = menu_top + (crate::ui::app_menu::PANEL_PADDING
                    + index * crate::ui::app_menu::ROW_HEIGHT) * scale;
                let row = crate::ui::geometry::Rect{x:(menu_left+5*scale)as i32,y:row_top as i32,
                    width:(menu_rect.width as usize-10*scale)as u32,
                    height:(crate::ui::app_menu::ROW_HEIGHT*scale)as u32};
                if index == selected {
                    self.app_dropdown_row(row, true, scale);
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
                self.app_menu_label(
                    crate::ui::geometry::Rect{x:(menu_left+14*scale)as i32,y:row_top as i32,
                        width:(18*scale)as u32,height:(crate::ui::app_menu::ROW_HEIGHT*scale)as u32},
                    if marked { b"*" } else { b"" },
                    (94,211,250),scale,
                );
                self.app_menu_label(
                    crate::ui::geometry::Rect{x:(menu_left+32*scale)as i32,y:row_top as i32,
                        width:(menu_rect.width as usize-46*scale)as u32,
                        height:(crate::ui::app_menu::ROW_HEIGHT*scale)as u32},
                    label,
                    (220,232,240),scale,
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
            self.glass_window_substrate(dialog_rect,true);
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
            let paste_to=navigator_state.is_some_and(|state|state.paste_to_editing);
            let title=if paste_to {b"Paste to location".as_slice()}else{title};
            let detail=if paste_to && navigator_state.is_some_and(|state|state.workplace_notice==4) {
                b"Paste failed. Check folder or existing names.".as_slice()
            }else if paste_to {b"Destination folder".as_slice()}else{detail};
            if paste_to {
                for (offset,text,strong,color) in [(22,title,true,(231,239,245)),(54,detail,false,(166,190,207))] {
                    self.app_label(crate::ui::geometry::Rect{x:(left+24*scale) as i32,y:(top+offset*scale) as i32,
                        width:(width-48*scale) as u32,height:(24*scale) as u32},text,color,strong,scale);
                }
            }else{
                self.ui_text_strong(left + 24 * scale, top + 22 * scale, title, 231, 239, 245, 1);
                self.ui_text(left+24*scale,top+54*scale,detail,166,190,207,1);
            }
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
                if navigator_state.is_some_and(|state|state.editor_selected) {
                    self.fill_rect_alpha(left+34*scale,top+89*scale,width-68*scale,24*scale,20,90,140,220);
                }
                self.ui_text_elided_strong(left + 36 * scale, top + 93 * scale, width-72*scale,path, 220, 232, 240);
                let field_clip=self.render_clip;
                self.intersect_render_clip(left+34*scale,top+82*scale,width-68*scale,38*scale);
                self.text_field_caret(
                    left + 36 * scale,
                    top + 82 * scale,
                    38 * scale,
                    path,
                    true,
                    2,
                );
                self.render_clip=field_clip;
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
                    } else if paste_to {
                        b"Paste"
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
                    self.app_button(crate::ui::geometry::Rect{x:button_left as i32,y:button_top as i32,
                        width:button_width as u32,height:(38*scale) as u32},label,index==1,scale);
                }
            } else {
                let button_left = left + width / 2 - 74 * scale;
                self.app_button(crate::ui::geometry::Rect{x:button_left as i32,y:button_top as i32,
                    width:(148*scale) as u32,height:(38*scale) as u32},b"Close",false,scale);
            }
            self.precision_window_frame(dialog_rect,true,scale);
        }
    }

    // ------------------------=
    // FUNC: desktop_widgets
    // DESC: Renders the persistent right-side system overview and AI status foreground layer.
    // ------------------=
    fn desktop_widgets(&mut self, scale: usize) {
        if crate::ui::desktop_widgets::current().shown(0,crate::ui::desktop_stack::current().visible[crate::ui::desktop_stack::BROWSER]) { self.desktop_overview_widget(scale); }
        self.desktop_ai_chat(scale);
    }

    // ------------------------=
    // FUNC: desktop_overview_widget
    // DESC: Paints the system widget at its shared movable geometry.
    // ------------------=
    fn desktop_overview_widget(&mut self, scale: usize) {
        let geometry=crate::ui::desktop_widgets::current().rect(0,self.width,self.height,scale,false);
        let Some(base)=crate::ui::desktop_template::bounds(2,2,self.width,self.height) else{return;};
        let machine=crate::runtime::with_runtime(|runtime|runtime.identity.machine()).flatten();
        let name=machine.map(|v|v.display_name).unwrap_or(crate::runtime::identity::ShortText::empty());
        self.desktop_template_scene(crate::ui::desktop_template::BYTES,2,
            &[(60,name.as_bytes()),(61,b"Active"),(62,b"Online"),(63,b"Ready"),(64,b"Ready")],
            Some((geometry.x-base.x,geometry.y-base.y)),false,&[]);
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
        let voice_profile = crate::runtime::with_runtime(|runtime| {
            (0..crate::runtime::identity::MAX_SESSIONS)
                .filter_map(|index| runtime.identity.session_nth(index))
                .find(|session| {
                    session.state == crate::runtime::identity::SessionState::Active
                })
                .and_then(|session| runtime.identity.voice_profile(session.user))
        })
        .flatten();
        let voice_enabled = voice_profile
            .map(|profile| {
                profile.enabled
                    && profile.activation != crate::runtime::identity::VoiceActivation::Disabled
            })
            .unwrap_or(false);
        let layout = crate::ui::system_layout::SystemLayout::new(self.width, self.height);
        let wake_prompt: &[u8] = match voice_profile.map(|profile|profile.wake_word) {
            Some(crate::runtime::identity::WakeWord::Computer)=>b"Say Computer",
            _=>b"Say Infinity",
        };
        let geometry = layout.ai_chat_geometry(chat.minimized());
        let left = geometry.panel.x.max(0) as usize;
        let top = geometry.panel.y.max(0) as usize;
        let width = geometry.panel.width as usize;
        let height = geometry.panel.height as usize;
        let (accent_r, accent_g, accent_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
        if let Some(base)=crate::ui::desktop_template::bounds(3,2,self.width,self.height) {
            let saved=self.render_clip;
            self.intersect_render_clip(left,top,width,height);
            self.desktop_template_scene(crate::ui::desktop_template::BYTES,3,&[],
                Some((geometry.panel.x-base.x,geometry.panel.y-base.y)),false,&[65,66,67,68,69,70,71]);
            self.render_clip=saved;
        }
        if !chat.minimized() && !crate::console::reduced_motion() {
            let level=crate::ui::ai_widget::meter();
            let generating=chat.generation_state==crate::runtime::ai::chat::GenerationState::Running;
            if generating || level>0 {
                let frame=unsafe { (*(&raw const THINKING_ANIMATION)).frame as usize };
                let span=width.saturating_sub(24*scale).max(1);
                let phase=(frame*2*scale)%(span*2);
                let center=phase.min(span*2-phase);
                for band in (0..8usize).rev() {
                    let radius=(18+band*7)*scale;
                    let x=left+12*scale+center.saturating_sub(radius);
                    let w=(radius*2).min(left+width-12*scale-x);
                    self.fill_rounded_rect_alpha(x,top+height.saturating_sub((72+band*2)*scale),
                        w,(24+band*4)*scale,12*scale,accent_r,accent_g,accent_b,
                        (2+level/4096+usize::from(generating)*2).min(12)as u8);
                }
            }
        }
        for row in 0..3 { for col in 0..2 { self.fill_rect(left+(3+col*4)*scale,top+(17+row*4)*scale,scale,scale,120,191,226); } }
        let app_progress = crate::runtime::ai::with_ai_runtime(|runtime| runtime.app_generation_progress());
        let can_stop = chat.generation_state == crate::runtime::ai::chat::GenerationState::Running
            || crate::runtime::ai::with_ai_runtime(|runtime| runtime.app_turn_owner().is_some())
            || matches!(crate::ui::app_assistant::read(2).generation,
                crate::ui::app_assistant::GenerationStatus::Queued | crate::ui::app_assistant::GenerationStatus::Running);
        #[cfg(target_os="none")]
        let can_stop = can_stop || matches!(crate::runtime::ai::voice_conversation::state().0,
            crate::runtime::ai::voice_conversation::State::Recognizing |
            crate::runtime::ai::voice_conversation::State::Submitting |
            crate::runtime::ai::voice_conversation::State::Thinking |
            crate::runtime::ai::voice_conversation::State::Speaking);
        #[cfg(target_os="none")]
        let voice_starting = {
            voice_enabled && crate::runtime::ai::voice_conversation::preparing()
        };
        #[cfg(not(target_os="none"))]
        let voice_starting = false;
        let state = if let Some(progress) = app_progress {
            if progress.generated_tokens == 0 { b"Preparing".as_slice() } else { b"Generating".as_slice() }
        } else if matches!(crate::ui::app_assistant::read(2).generation,
            crate::ui::app_assistant::GenerationStatus::Queued | crate::ui::app_assistant::GenerationStatus::Running) {
            b"Preparing".as_slice()
        } else if !chat.selected_model_ready() {
            b"Preparing".as_slice()
        } else {
            use crate::runtime::ai::chat::GenerationState;
            match chat.generation_state {
                GenerationState::Running => b"Generating".as_slice(),
                GenerationState::Cancelled => b"Cancelled".as_slice(),
                GenerationState::Failed => b"Failed".as_slice(),
                GenerationState::ContextFull => b"Context full".as_slice(),
                _ => b"Ready".as_slice(),
            }
        };
        #[cfg(target_os="none")]
        let state = {
            use crate::runtime::ai::voice_conversation::{state as voice_state,State};
            if app_progress.is_some() || chat.generation_state == crate::runtime::ai::chat::GenerationState::Running || !chat.selected_model_ready() {
                state
            } else if !voice_enabled {
                b"Voice off".as_slice()
            } else if voice_starting {
                b"Preparing".as_slice()
            } else { match voice_state().0 {
                State::Off=>b"Voice off".as_slice(),State::Starting=>b"Preparing".as_slice(),
                State::Listening=>if crate::runtime::ai::voice_conversation::waiting_for_pause() {
                    b"Pause to retry".as_slice()
                } else if crate::runtime::ai::voice_conversation::wake_armed() {b"Go ahead".as_slice()} else {wake_prompt},
                State::Recognizing=>b"Recognizing".as_slice(),State::Submitting=>b"Queued".as_slice(),
                State::Thinking=>b"Generating".as_slice(),
                State::Speaking=>match crate::runtime::ai::voice_output::status().state {
                    crate::runtime::ai::voice_output::OutputState::Speaking=>b"Speaking".as_slice(),
                    _=>b"Preparing speech".as_slice(),
                },State::Stopping=>b"Stopping".as_slice(),
                State::Failed=>b"Unavailable".as_slice(),
            }}
        };
        let frame = unsafe { THINKING_ANIMATION.frame as usize };
        self.ai_widget_header(geometry,state,chat.minimized(),can_stop || voice_starting,scale,(accent_r,accent_g,accent_b));
        if chat.minimized()
            || self.render_clip.is_some_and(|clip| clip.bottom <= geometry.header.bottom().max(0) as usize)
        {
            self.ai_widget_status_menu(geometry.status,crate::ui::ai_widget::current_menu(),voice_enabled,can_stop,scale);
            return;
        }
        let model = chat.selected_model_descriptor();
        self.ai_widget_model(geometry.model,model.name,scale);
        if !chat.selected_model_ready() {
            let loading_left = geometry.timeline.x.max(0) as usize;
            let loading_top = geometry.timeline.y.max(0) as usize;
            let loading_width = geometry.timeline.width as usize;
            let loading_bottom = geometry.composer.y
                .saturating_add(geometry.composer.height as i32)
                .max(0) as usize;
            let loading_height = loading_bottom.saturating_sub(loading_top);
            self.app_card_alpha(crate::ui::geometry::Rect {
                x:loading_left as i32,y:loading_top as i32,
                width:loading_width as u32,height:loading_height as u32,
            },(2,12,22),(accent_r/2,accent_g/2,accent_b/2),244,scale);
            let center_x = loading_left + loading_width / 2;
            let center_y = (loading_top + loading_height / 2).saturating_sub(30 * scale);
            let points = [
                (-22, 0), (-16, -16), (0, -22), (16, -16),
                (22, 0), (16, 16), (0, 22), (-16, 16),
            ];
            for index in 0..8 {
                let next = (index + 1) % 8;
                let active = index == (frame / 2) % 8 || next == (frame / 2) % 8;
                self.line(
                    center_x as i32 + points[index].0 * scale as i32,
                    center_y as i32 + points[index].1 * scale as i32,
                    center_x as i32 + points[next].0 * scale as i32,
                    center_y as i32 + points[next].1 * scale as i32,
                    if active { 116 } else { 28 },
                    if active { 224 } else { 94 },
                    if active { 255 } else { 126 },
                );
            }
            self.ui_text_centered_strong(
                loading_left, loading_width, center_y + 42 * scale,
                b"PREPARING CONVERSATION", 218, 238, 248, 1,
            );
            self.ui_text_centered(
                loading_left, loading_width, center_y + 66 * scale,
                b"Loading local language model", 137, 178, 198, 1,
            );
            self.ui_text_centered(
                loading_left, loading_width, center_y + 86 * scale,
                b"Please wait before speaking", accent_r, accent_g, accent_b, 1,
            );
            self.ai_widget_status_menu(geometry.status,crate::ui::ai_widget::current_menu(),voice_enabled,can_stop,scale);
            return;
        }
        let viewport=geometry.transcript_viewport(scale);
        let timeline_left = viewport.x.max(0) as usize;
        let timeline_top = viewport.y.max(0) as usize;
        let timeline_width = viewport.width as usize;
        let timeline_height = viewport.height as usize;
        self.desktop_bound_surface(67,geometry.timeline);
        if timeline_width == 0 || timeline_height == 0 {
            crate::runtime::ai::with_ai_runtime(|runtime| runtime.chat.set_timeline_scroll_metrics(0));
        } else if chat.message_count() == 0 {
            crate::runtime::ai::with_ai_runtime(|runtime| {
                runtime.chat.set_timeline_scroll_metrics(0)
            });
            self.ai_widget_empty_timeline(geometry.timeline,scale);
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
            let message_count = chat.message_count();
            let speech_turn = chat.turn_id();
            let mut content_height = viewport_padding * 2;
            for index in 0..message_count {
                let Some(message) = chat.message(index) else { continue; };
                let full_text = message.text();
                let visible_length = if index + 1 == message_count
                    && message.role == crate::runtime::ai::chat::ChatRole::Assistant {
                    crate::runtime::ai::voice_conversation::visible_reply_length(
                        speech_turn, full_text.len())
                } else { full_text.len() };
                let visible_text = &full_text[..visible_length];
                let lines = self
                    .ui_text_wrapped_line_count(text_width, visible_text, usize::MAX)
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
            for index in 0..message_count {
                let Some(message) = chat.message(index) else {
                    continue;
                };
                let full_text = message.text();
                let visible_length = if index + 1 == message_count
                    && message.role == crate::runtime::ai::chat::ChatRole::Assistant {
                    crate::runtime::ai::voice_conversation::visible_reply_length(
                        speech_turn, full_text.len())
                } else { full_text.len() };
                let visible_text = &full_text[..visible_length];
                let user = message.role == crate::runtime::ai::chat::ChatRole::User;
                let inset = if user { 28 * scale } else { 7 * scale };
                let lines = self
                    .ui_text_wrapped_line_count(text_width, visible_text, usize::MAX)
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
                        visible_text,
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
        let composer_text = chat.input();
        self.ai_widget_composer(geometry.composer,composer_text,chat.input_cursor(),
            crate::ui::text_input::caret(3),scale,(accent_r,accent_g,accent_b));
        let send_ready = !composer_text.is_empty() && chat.selected_model_ready()
            && chat.generation_state != crate::runtime::ai::chat::GenerationState::Running;
        self.ai_widget_send(geometry.send,send_ready,scale);
        self.ai_widget_status_menu(geometry.status,crate::ui::ai_widget::current_menu(),voice_enabled,can_stop,scale);
    }

    // ------------------------=
    // FUNC: desktop_dock
    // DESC: Renders the persistent foreground application dock and launcher state.
    // ------------------=
    fn desktop_dock(&mut self, scale: usize, launcher_open: bool) {
        if let Some(scene) = crate::ui::desktop_template::dock_scene() {
            self.desktop_template_scene(crate::ui::desktop_template::BYTES,scene,&[],None,false,&[]);
            let hovered = match crate::ui::desktop_polish::current() {
                crate::ui::desktop_polish::HoverTarget::Dock(index)=>Some(index),_=>None,
            };
            let stack=crate::ui::desktop_stack::current();
            let widgets=crate::ui::desktop_widgets::current();
            for index in 0..8 {
                let Some(r)=crate::ui::desktop_template::bounds(scene,73+index as u8,self.width,self.height) else{continue;};
                if hovered==Some(index) || (index==0 && launcher_open) {
                    self.app_card_alpha(r,(16,27,58),(134,199,255),0,scale);
                }
                if index!=0 && crate::ui::desktop_polish::dock_running(index,launcher_open,
                    stack.visible,stack.settings_section,widgets.visible&2!=0) {
                    let size=(3*scale).min(r.width as usize).min(r.height as usize);
                    self.fill_rounded_rect_alpha((r.x.max(0)as usize+r.width as usize/2).saturating_sub(size/2),
                        (r.bottom().max(0)as usize).saturating_sub(size),size,size,size/2,91,210,255,255);
                }
            }
            return;
        }
        let geometry = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
            .desktop_foreground_geometry();
        let dock_width = geometry.dock.width as usize;
        let dock_height = geometry.dock.height as usize;
        let dock_left = geometry.dock.x.max(0) as usize;
        let dock_top = geometry.dock.y.max(0) as usize;
        if let Some(region)=self.clipped_render_region(dock_left,dock_top,dock_width,dock_height) {
            let (opacity,blur)=self.active_background_effects();
            let mut material=dock_material::Material{primary:self.active_primary_rgb(),accent:0x86c7ff,
                opacity,blur,scale,bgr:self.format!=0};
            match self.skin_visual_mode() {
                1=>{material.primary=0xf8fbff;material.accent=0x7a91a6;},
                2=>{material.primary=0x080808;material.accent=0xffffff;material.opacity=100;material.blur=0;},
                _=>{},
            }
            // The dock is painted over the restored desktop, before application windows.
            // system_ui_frame expands intersecting damage before desktop restoration,
            // so the complete blur source is clean and stable across partial repaints.
            unsafe {(&mut *(&raw mut DOCK_BACKDROP)).paint(
                core::slice::from_raw_parts_mut(self.buffer,self.stride*self.height),self.stride,
                (dock_left,dock_top,dock_width,dock_height),
                (region.left,region.top,region.right,region.bottom),material);}
            self.mark_dirty_rect(region.left,region.top,region.right-region.left,region.bottom-region.top);
        }
        let entries = &crate::ui::app_launcher::DESKTOP_DOCK_ENTRIES;
        let icon_gap = dock_width / entries.len();
        let stack = crate::ui::desktop_stack::current();
        let widget_state = crate::ui::desktop_widgets::current();
        let hovered = match crate::ui::desktop_polish::current() {
            crate::ui::desktop_polish::HoverTarget::Dock(index) => Some(index),
            _ => None,
        };
        for (index, entry) in entries.iter().enumerate() {
            let size = 46 * scale;
            let x = dock_left + icon_gap / 2 + index * icon_gap;
            let is_hovered = hovered == Some(index);
            let icon_top = dock_top + (if is_hovered && index!=0 { 7 } else { 10 }) * scale;
            if is_hovered && index!=0 {
                self.fill_rounded_rect_alpha(
                    x.saturating_sub(5 * scale),
                    icon_top.saturating_sub(4 * scale),
                    size + 10 * scale,
                    size + 10 * scale,
                    16 * scale,
                    31,
                    116,
                    171,
                    104,
                );
                self.outline_rounded_rect(
                    x.saturating_sub(3 * scale),
                    icon_top.saturating_sub(2 * scale),
                    size + 6 * scale,
                    size + 6 * scale,
                    14 * scale,
                    91,
                    210,
                    255,
                );
            }
            let running = crate::ui::desktop_polish::dock_running(
                index,
                launcher_open,
                stack.visible,
                stack.settings_section,
                widget_state.visible & 2 != 0,
            );
            if index == 0 {
                self.launcher_dock_icon(x, icon_top, size, launcher_open || is_hovered);
            } else {
                self.desktop_app_icon(
                    x,
                    icon_top,
                    size,
                    entry.icon_kind,
                    running,
                );
            }
            if index == 6 {
                self.fill_rect_alpha(
                    x.saturating_sub(icon_gap / 3),
                    dock_top + 12 * scale,
                    1,
                    size.saturating_sub(4 * scale),
                    87,
                    137,
                    171,
                    190,
                );
            }
        }
        if let Some(index) = hovered {
            let label = crate::ui::desktop_polish::dock_label(index);
            let label_width = self.ui_text_width(label, 1) + 20 * scale;
            let center = dock_left + icon_gap / 2 + index * icon_gap + 23 * scale;
            let left = center
                .saturating_sub(label_width / 2)
                .clamp(8 * scale, self.width.saturating_sub(label_width + 8 * scale));
            let top = dock_top.saturating_sub(35 * scale);
            self.app_card_alpha(crate::ui::geometry::Rect {
                x:left as i32,y:top as i32,width:label_width as u32,height:(28*scale) as u32,
            },(3,20,35),(75,170,216),238,scale);
            self.ui_text_centered_strong(
                left,
                label_width,
                top + 6 * scale,
                label,
                226,
                239,
                248,
                1,
            );
        }
    }

    // ------------------------=
    // FUNC: system_login_animation
    // DESC: Reuses the authored ripple under the resting orb, restoring only bounded water pixels and never erasing the access wordmark or controls.
    // ------------------=
    pub(super) fn system_login_animation(&mut self, phase: usize) {
        if self.skin_visual_mode() != 0 || crate::console::reduced_motion() { return; }
        let elapsed = crate::ui::performance::monotonic_ns().unwrap_or(0) / 1_000_000;
        if self.repaint_login_motion(crate::ui::authentication_motion::idle_presentation(elapsed, false)) { return; }
        if !crate::ui::installer_layout::login_uses_legacy_animation(
            crate::ui::installer_layout::LOGIN_TEMPLATE_BYTES,
        ) { return; }
        use crate::ui::authentication_motion as motion;
        let elapsed_ms = crate::ui::performance::monotonic_ns().map(|now| now / 1_000_000)
            .unwrap_or((phase % 384) as u64 * motion::IDLE_PERIOD_MS / 384);
        let presentation = motion::idle_presentation(elapsed_ms, false);
        let (offset_x, offset_y, scale) = self.authentication_art_geometry();
        let center_x = offset_x + motion::ARTWORK_CENTER_X as i32 * scale as i32 / 1_000;
        let water_y = offset_y + motion::ARTWORK_WATER_Y as i32 * scale as i32 / 1_000;
        let previous_clip = self.render_clip;
        for (mut left, mut top, mut right, mut bottom) in motion::idle_water_regions(self.width, self.height) {
            if let Some(clip) = previous_clip {
                left = left.max(clip.left);
                top = top.max(clip.top);
                right = right.min(clip.right);
                bottom = bottom.min(clip.bottom);
            }
            self.authentication_success_frame_region(
                presentation, left, top, right, bottom, offset_y, scale, center_x, water_y,
            );
        }
        self.render_clip = previous_clip;
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
        if access::panel_motion().extent < 1024 {
            if matches!(screen,5|6) { self.authentication_focus_controls(screen==6,step,input,1); }
            else if screen==1 { self.onboarding_focus_controls(step,input,masked,1,false); }
            return;
        }
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
            let password_h = 57usize.saturating_mul(fit) / 1000;
            let previous_clip = self.render_clip;
            self.intersect_render_clip(inner_x, password_y, inner_w, password_h);
            self.paint_authentication_background_rect(inner_x, password_y, inner_w, password_h);
            self.authentication_frame(screen == 6, step, input, 1, false);
            self.render_clip = previous_clip;
            return;
        }
        let Some(field) = crate::ui::installer_layout::configuration_template_input_rect(
            step,
            self.width,
            self.height,
        ) else {
            return;
        };
        let previous_clip = self.render_clip;
        self.intersect_render_clip(field.left, field.top, field.width, field.height);
        // Recompose in the same authored order under damage, including layers
        // above the field; never repaint live text on top of later artwork.
        self.paint_first_boot_background();
        self.onboarding_frame(step,input,masked,1,false);
        self.render_clip = previous_clip;
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
            chat_content ^= crate::ui::ai_widget::menu_hash().wrapping_mul(0x94d049bb133111eb);
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
            ) ^ crate::ui::installer_template::navigation::revision().rotate_left(17)
                ^ (chat_content as u32 ^ (chat_content >> 32) as u32) ^ crate::ui::text_input::presentation_hash()
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
            let access_motion = matches!(screen, 1 | 5 | 6) && super::access_style::take_dirty();
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
            let effect_damage=crate::ui::desktop_effects::take_damage();
            let effect_changed=effect_damage!=0;
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
            let font_changed=crate::ui::font_library::take_damage();
            if font_changed {super::retained_windows::invalidate();}
            let wallpaper_changed=crate::ui::user_wallpaper::take_damage(console.display.width,console.display.height).is_some();
            let visibility_changed = crate::ui::desktop_stack::take_visibility_damage();
            let settings_mutated = SETTINGS_PRESENTATION_DIRTY.swap(false, core::sync::atomic::Ordering::AcqRel);
            let structural_change_without_window = settings_mutated || visibility_changed || font_changed || wallpaper_changed || crate::ui::spatial::take_world_damage() || (!bounded_menu_change
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
            let assistant_revision = crate::ui::app_assistant::revision();
            let assistant_changed = crate::ui::redraw::assistant_requires_owner_damage(
                screen,
                core::mem::replace(&mut *(&raw mut LAST_ASSISTANT_REVISION), assistant_revision),
                assistant_revision,
            );
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
            let thinking_body_changed = (&mut *(&raw mut THINKING_ANIMATION)).take_body_damage();
            let command_input_only = screen == 8 && console.last_system_screen == 8
                && crate::ui::installer_template::navigation::scene()==0
                && !assistant_changed
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
            let bounded_scene_geometry_change = effect_damage&2==0 && !structural_change_without_window
                && crate::ui::redraw::scene_content_allows_bounded_redraw(
                    screen, content_changed, navigator_surface_changed, assistant_changed, chat_changed)
                && console.last_system_screen == screen
                && (effect_changed || navigator_surface_changed
                    || assistant_changed
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
            if bounded_menu_change && !effect_changed
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
            } else if bounded_launcher_change && !effect_changed {
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
            } else if desktop_layer_focus_changed && !structural_change_without_window && !effect_changed {
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
                        let current = layout.home_window_geometry_sized(
                            window_x,
                            window_y,
                            window_width,
                            window_height,
                            window_maximized,
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
            } else if effect_changed || structural_change_without_window
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
                pointer_changed || access_motion,
                focus_changed || access_motion,
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
                pointer_changed || access_motion,
                focus_changed || access_motion,
            ) {
                console
                    .display
                    .authentication_focus_controls(screen == 6, step, input, focus);
                full_surface_redrawn = true;
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
                assistant_changed,
            ) {
                let mut widgets = layout.ai_chat_geometry(false).panel;
                if thinking_header_changed && !content_changed {
                    widgets = layout.ai_chat_geometry(false).panel;
                    widgets.height = crate::ui::thinking::repaint_height(
                        widgets.height, layout.scale(), thinking_body_changed)
                        .max(layout.ai_chat_geometry(false).header.height);
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
                let mut native_windows = [None; 8];
                let mut native_count = 0;
                crate::console::native_gui_frames(|window, _, _| {
                    if native_count < native_windows.len() {
                        native_windows[native_count] = Some(window);
                        native_count += 1;
                    }
                });
                let native_damage = console.native_scene_damage.update(&native_windows, display_rect, layout.scale());
                let control_damage=crate::ui::app_launcher::window_controls::take_damage();
                for rect in control_damage.into_iter().flatten() {
                    for slot in 0..13 {super::retained_windows::invalidate_region(slot,super::PresentRegion {
                        left:rect.x.max(0) as usize,top:rect.y.max(0) as usize,
                        right:rect.right().max(0) as usize,bottom:rect.bottom().max(0) as usize});}
                }
                for damage in [native_damage,control_damage[0],control_damage[1],crate::ui::file_preview::desktop_damage(console.display.width,console.display.height),crate::ui::file_preview::take_damage(),crate::console::clipboard_menu::take_damage(),crate::ui::app_launcher::minimized_shelf::take_damage(console.display.width, console.display.height),
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
            if matches!(screen,2|4|7|8|9|10|11) {
                crate::console::native_gui_frames(|window,frame,active|console.display.native_gui_window(window,frame,active));
            }
            console.display.clipboard_overlay();
            if matches!(screen,2|4|7|8|9|10|11) {console.display.file_preview_overlay();}
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
    if !crate::console::input_cursor_presentation(x, y) { return; }
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
