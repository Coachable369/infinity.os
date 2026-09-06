//! InfinityUI architecture-neutral retained UI core.
//!
//! This module owns semantic elements, deterministic logical layout, skin
//! selection, input focus, window/surface policy, damage tracking, and trusted
//! UI state. Platform framebuffer code consumes its bounded render model.

pub mod app_launcher;
pub mod async_model;
pub mod clipboard;
pub mod compositor;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub mod crash_layout;
pub mod geometry;
pub mod icon_theme;
pub mod input;
pub mod input_router;
pub mod installer_layout;
pub mod installer_template;
pub mod localization;
pub mod platform;
pub mod redraw;
pub mod scene;
pub mod session_state;
pub mod skin;
pub mod surface;
pub mod system_layout;
pub mod text_editor;
pub mod text_input;
pub mod trusted;
pub mod vector;
pub mod window;

use icon_theme::IconThemeRegistry;
use input::FocusManager;
use scene::{DamageTracker, UiScene};
use skin::SkinRegistry;
use surface::{SurfaceRegistry, DEFAULT_SURFACE_BUDGET_BYTES};
use window::WindowServer;

pub const INFINITY_UI_ABI_VERSION: u16 = 1;

pub struct InfinityUiRuntime {
    pub async_tasks: async_model::AsyncUiModel,
    pub clipboard: clipboard::ClipboardService,
    pub icons: IconThemeRegistry,
    pub skins: SkinRegistry,
    pub scene: UiScene,
    pub focus: FocusManager,
    pub windows: WindowServer,
    pub surfaces: SurfaceRegistry,
    pub damage: DamageTracker,
    pub frame_sequence: u64,
    pub frame_clock: platform::FrameClock,
    pub quality: platform::AdaptiveQualityController,
    pub trusted: trusted::TrustedUiManager,
}

impl InfinityUiRuntime {
    // ------------------------=
    // FUNC: new
    // DESC: Creates the bounded architecture-neutral InfinityUI runtime state.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            async_tasks: async_model::AsyncUiModel::new(),
            clipboard: clipboard::ClipboardService::new(),
            icons: IconThemeRegistry::new(),
            skins: SkinRegistry::new(),
            scene: UiScene::new(),
            focus: FocusManager::new(),
            windows: WindowServer::new(),
            surfaces: SurfaceRegistry::new(DEFAULT_SURFACE_BUDGET_BYTES),
            damage: DamageTracker::new(),
            frame_sequence: 0,
            frame_clock: platform::FrameClock::new(60),
            quality: platform::AdaptiveQualityController::new(),
            trusted: trusted::TrustedUiManager::new(),
        }
    }

    // ------------------------=
    // FUNC: begin_frame
    // DESC: Starts one transactional UI frame without mutating presented pixels.
    // ------------------=
    pub fn begin_frame(&mut self) {
        self.frame_sequence = self.frame_sequence.wrapping_add(1);
        self.scene.begin_frame(self.frame_sequence);
        self.damage.begin_frame();
    }

    // ------------------------=
    // FUNC: commit_frame
    // DESC: Commits retained geometry and returns the bounded damage set for presentation.
    // ------------------=
    pub fn commit_frame(&mut self) -> &[geometry::Rect] {
        self.scene.commit_frame(&mut self.damage);
        self.damage.regions()
    }

    // ------------------------=
    // FUNC: context_failed
    // DESC: Reclaims a failed context's windows and surface reservations while preserving unrelated UI state.
    // ------------------=
    pub fn context_failed(&mut self, owner: window::ContextId) -> (usize, usize) {
        let windows = self.windows.context_failed(owner);
        let surfaces = self.surfaces.context_failed(owner);
        (windows, surfaces)
    }
}
