//! Thin framebuffer/UI facade preserving the public display and presentation API.

use crate::boot_info::BootInfo;

mod bootstrap;
mod crash;
mod desktop;
mod editor_view;
mod assistant_view;
mod app_style;
mod installer;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
mod launcher_backdrop;
mod primitives;
mod retained_windows;
mod template_compositor;
mod window_chrome;

use self::primitives::*;

pub use self::bootstrap::{animation_tick, console_present, note_pointer_activity, show_splash};
pub use self::crash::show_fatal_crash;
pub use self::desktop::{system_ui_cursor, system_ui_present};
pub use self::installer::{installer_progress_update, installer_reboot_countdown};

#[derive(Clone, Copy)]
pub struct DisplayDevice {
    front_buffer: *mut u32,
    buffer: *mut u32,
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    format: u32,
    back_buffered: bool,
    dirty_regions: [PresentRegion; MAX_PRESENT_REGIONS],
    dirty_count: u8,
    presented_frames: u64,
    presented_pixels: u64,
    full_frame_fallbacks: u32,
    damage_collapses: u32,
    render_clip: Option<PresentRegion>,
    fast_motion_frame: bool,
    frame_started_ns: Option<u64>,
    submitted_regions: u32,
    merged_regions: u32,
    fallback_reason: crate::ui::performance::FallbackReason,
    recording_surface: bool,
}

const MAX_PRESENT_REGIONS: usize = 8;

use crate::ui::present_damage::Region as PresentRegion;

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct DisplayPresentDiagnostics {
    pub software_back_buffered: bool,
    pub back_buffer_bytes: u64,
    pub presented_frames: u64,
    pub presented_pixels: u64,
    pub full_frame_fallbacks: u32,
    pub damage_collapses: u32,
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const MAX_SOFTWARE_BACK_BUFFER_PIXELS: usize = 3840 * 2160;

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[repr(align(64))]
struct AlignedBackBuffer([u32; MAX_SOFTWARE_BACK_BUFFER_PIXELS]);

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
static mut PRIMARY_BACK_BUFFER: AlignedBackBuffer =
    AlignedBackBuffer([0; MAX_SOFTWARE_BACK_BUFFER_PIXELS]);

impl DisplayDevice {
    // ------------------------=
    // FUNC: from_boot_info
    // DESC: Validates boot framebuffer metadata and creates a drawable display device.
    // ------------------=
    pub fn from_boot_info(info: &BootInfo) -> Option<Self> {
        let required = (info.framebuffer_stride as u64)
            .checked_mul(info.framebuffer_height as u64)?
            .checked_mul(4)?;
        if info.framebuffer_address == 0
            || info.framebuffer_address & 3 != 0
            || info.framebuffer_format > 1
            || required > usize::MAX as u64
            || info.framebuffer_stride < info.framebuffer_width
            || info
                .framebuffer_address
                .checked_add(info.framebuffer_size)
                .is_none()
            || info.framebuffer_stride == 0
            || info.framebuffer_width == 0
            || info.framebuffer_height == 0
            || required > info.framebuffer_size
        {
            return None;
        }
        let front_buffer = info.framebuffer_address as usize as *mut u32;
        let required_pixels = info.framebuffer_stride as usize * info.framebuffer_height as usize;
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        let (buffer, back_buffered) = if required_pixels <= MAX_SOFTWARE_BACK_BUFFER_PIXELS {
            let back_buffer = unsafe { (&raw mut PRIMARY_BACK_BUFFER.0).cast::<u32>() };
            unsafe { core::ptr::copy_nonoverlapping(front_buffer, back_buffer, required_pixels) };
            (back_buffer, true)
        } else {
            (front_buffer, false)
        };
        #[cfg(target_arch = "x86")]
        let (buffer, back_buffered) = (front_buffer, false);
        Some(Self {
            front_buffer,
            buffer,
            width: info.framebuffer_width as usize,
            height: info.framebuffer_height as usize,
            stride: info.framebuffer_stride as usize,
            format: info.framebuffer_format,
            back_buffered,
            dirty_regions: [PresentRegion::default(); MAX_PRESENT_REGIONS],
            dirty_count: 0,
            presented_frames: 0,
            presented_pixels: 0,
            full_frame_fallbacks: 0,
            damage_collapses: 0,
            render_clip: None,
            fast_motion_frame: false,
            frame_started_ns: None,
            submitted_regions: 0,
            merged_regions: 0,
            fallback_reason: crate::ui::performance::FallbackReason::InitialRender,
            recording_surface: false,
        })
    }

    // ------------------------=
    // FUNC: set_render_clip
    // DESC: Restricts scene reconstruction and presentation damage to one clipped display region.
    // ------------------=
    fn set_render_clip(&mut self, left: usize, top: usize, width: usize, height: usize) {
        self.render_clip = Some(PresentRegion {
            left: left.min(self.width),
            top: top.min(self.height),
            right: left.saturating_add(width).min(self.width),
            bottom: top.saturating_add(height).min(self.height),
        });
    }

    // ------------------------=
    // FUNC: intersect_render_clip
    // DESC: Narrows the active reconstruction clip without allowing a nested component to escape its caller's damage region.
    // ------------------=
    fn intersect_render_clip(&mut self, left: usize, top: usize, width: usize, height: usize) {
        let requested = PresentRegion {
            left: left.min(self.width),
            top: top.min(self.height),
            right: left.saturating_add(width).min(self.width),
            bottom: top.saturating_add(height).min(self.height),
        };
        self.render_clip = Some(match self.render_clip {
            Some(current) => PresentRegion {
                left: current.left.max(requested.left),
                top: current.top.max(requested.top),
                right: current.right.min(requested.right),
                bottom: current.bottom.min(requested.bottom),
            },
            None => requested,
        });
    }

    // ------------------------=
    // FUNC: clear_render_clip
    // DESC: Restores unrestricted rendering after one bounded scene reconstruction.
    // ------------------=
    fn clear_render_clip(&mut self) {
        self.render_clip = None;
    }

    // ------------------------=
    // FUNC: render_point_visible
    // DESC: Reports whether one framebuffer point lies inside the active reconstruction clip.
    // ------------------=
    const fn render_point_visible(&self, x: usize, y: usize) -> bool {
        match self.render_clip {
            Some(clip) => x >= clip.left && x < clip.right && y >= clip.top && y < clip.bottom,
            None => true,
        }
    }

    // ------------------------=
    // FUNC: clipped_render_region
    // DESC: Intersects one requested rectangle with the display and active reconstruction clip.
    // ------------------=
    fn clipped_render_region(
        &self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) -> Option<PresentRegion> {
        if width == 0 || height == 0 || left >= self.width || top >= self.height {
            return None;
        }
        let mut region = PresentRegion {
            left,
            top,
            right: left.saturating_add(width).min(self.width),
            bottom: top.saturating_add(height).min(self.height),
        };
        if let Some(clip) = self.render_clip {
            region.left = region.left.max(clip.left);
            region.top = region.top.max(clip.top);
            region.right = region.right.min(clip.right);
            region.bottom = region.bottom.min(clip.bottom);
        }
        (region.left < region.right && region.top < region.bottom).then_some(region)
    }

    // ------------------------=
    // FUNC: mark_dirty_rect
    // DESC: Adds a clipped framebuffer rectangle to the pending coherent presentation union.
    // ------------------=
    fn mark_dirty_rect(&mut self, left: usize, top: usize, width: usize, height: usize) {
        if self.recording_surface {
            return;
        }
        let Some(mut submitted) = self.clipped_render_region(left, top, width, height) else {
            return;
        };
        let mut count = self.dirty_count as usize;
        let before = count;
        self.submitted_regions = self.submitted_regions.saturating_add(1);
        let overflow =
            crate::ui::present_damage::insert(&mut self.dirty_regions, &mut count, submitted);
        self.merged_regions = self
            .merged_regions
            .saturating_add((before + 1).saturating_sub(count) as u32);
        self.dirty_count = count as u8;
        if overflow {
            self.damage_collapses = self.damage_collapses.saturating_add(1);
        }
    }

    // ------------------------=
    // FUNC: present_damage
    // DESC: Atomically exposes the completed dirty union from software back buffer to physical framebuffer.
    // ------------------=
    fn present_damage(&mut self) -> u64 {
        if self.dirty_count == 0 {
            return 0;
        }
        let mut pixels = 0u64;
        let mut submitted = true;
        let present_started = crate::ui::performance::monotonic_ns();
        for index in 0..self.dirty_count as usize {
            let region = self.dirty_regions[index];
            let width = region.right.saturating_sub(region.left);
            let height = region.bottom.saturating_sub(region.top);
            if self.back_buffered {
                for y in region.top..region.bottom {
                    if y & 31 == 0 { crate::ui::input_capture::poll(); }
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            self.buffer.add(y * self.stride + region.left),
                            self.front_buffer.add(y * self.stride + region.left),
                            width,
                        );
                    }
                }
            }
            pixels = pixels.saturating_add((width as u64).saturating_mul(height as u64));
            submitted &= crate::drivers::display::update(region.left, region.top, width, height);
        }
        crate::drivers::display::flush();
        // Retry unchanged scanout damage when the native FIFO cannot accept the complete batch.
        if !submitted {
            return pixels;
        }
        self.presented_frames = self.presented_frames.wrapping_add(1);
        self.presented_pixels = self.presented_pixels.saturating_add(pixels);
        let completed = crate::ui::performance::monotonic_ns();
        let screen_pixels = self.width as u64 * self.height as u64;
        let fallback = if pixels >= screen_pixels {
            if self.fallback_reason == crate::ui::performance::FallbackReason::None {
                crate::ui::performance::FallbackReason::Unclassified
            } else {
                self.fallback_reason
            }
        } else {
            crate::ui::performance::FallbackReason::None
        };
        if fallback != crate::ui::performance::FallbackReason::None {
            self.full_frame_fallbacks = self.full_frame_fallbacks.saturating_add(1);
        }
        crate::ui::performance::publish(crate::ui::performance::FramePerformanceSample {
            timestamp_ns: completed,
            frame_ns: crate::ui::performance::elapsed(self.frame_started_ns, completed),
            compose_ns: crate::ui::performance::elapsed(self.frame_started_ns, present_started),
            present_ns: crate::ui::performance::elapsed(present_started, completed),
            submitted_regions: self.submitted_regions,
            merged_regions: self.merged_regions,
            damaged_pixels: pixels,
            screen_pixels,
            fallback,
            budget_ns: 16_666_667,
            ..crate::ui::performance::FramePerformanceSample::default()
        });
        self.frame_started_ns = None;
        self.submitted_regions = 0;
        self.merged_regions = 0;
        self.fallback_reason = crate::ui::performance::FallbackReason::None;
        self.dirty_count = 0;
        pixels
    }

    // ------------------------=
    // FUNC: force_full_present
    // DESC: Requests and presents a complete frame for initialization or explicit recovery.
    // ------------------=
    fn force_full_present(&mut self) -> u64 {
        if self.fallback_reason == crate::ui::performance::FallbackReason::None {
            self.fallback_reason = crate::ui::performance::FallbackReason::Recovery;
        }
        self.mark_dirty_rect(0, 0, self.width, self.height);
        self.present_damage()
    }

    // ------------------------=
    // FUNC: uses_software_back_buffer
    // DESC: Reports whether the active display fits the bounded coherent software presentation buffer.
    // ------------------=
    pub const fn uses_software_back_buffer(&self) -> bool {
        self.back_buffered
    }

    // ------------------------=
    // FUNC: diagnostics
    // DESC: Projects physical presentation behavior as structured machine-readable counters.
    // ------------------=
    pub const fn diagnostics(&self) -> DisplayPresentDiagnostics {
        DisplayPresentDiagnostics {
            software_back_buffered: self.back_buffered,
            back_buffer_bytes: if self.back_buffered {
                self.stride as u64 * self.height as u64 * 4
            } else {
                0
            },
            presented_frames: self.presented_frames,
            presented_pixels: self.presented_pixels,
            full_frame_fallbacks: self.full_frame_fallbacks,
            damage_collapses: self.damage_collapses,
        }
    }
}

struct ConsoleSurface {
    display: DisplayDevice,
    particle_phase: usize,
    particles_initialized: bool,
    bootstrap_scene: bool,
    split_layout: bool,
    layout_initialized: bool,
    cursor_x: i32,
    cursor_y: i32,
    pointer_pressed: bool,
    installer_scene: bool,
    last_installer_screen: u8,
    last_installer_focus: usize,
    last_installer_choice: usize,
    last_installer_date_time: crate::storage::DateTimeConfiguration,
    last_installer_date_time_part: usize,
    installer_progress: usize,
    installer_animation_phase: usize,
    last_system_screen: u8,
    last_system_step: usize,
    last_system_focus: usize,
    last_system_menu: usize,
    last_icon_theme: u8,
    last_accent_rgb: u32,
    last_primary_rgb: u32,
    last_background_opacity: u8,
    last_background_blur: u8,
    last_system_content: u32,
    last_system_static_content: u32,
    last_launcher_state: u64,
    last_launcher_interaction_state: u64,
    last_launcher_transition: u8,
    last_system_validation_error: bool,
    last_home_window_x: i32,
    last_home_window_y: i32,
    last_home_window_width: i32,
    last_home_window_height: i32,
    last_home_window_visible: bool,
    last_home_window_maximized: bool,
    last_home_location: usize,
    last_home_selected_item: Option<usize>,
    last_home_dragging_item: Option<usize>,
    last_home_note_location: usize,
    last_file_navigator_state: Option<crate::runtime::object_navigation::FileNavigatorState>,
    last_desktop_items: u8,
    last_desktop_item_positions: [[i32; 2]; 7],
    last_system_clock: crate::storage::DateTimeConfiguration,
    last_settings_window: crate::ui::system_layout::SettingsWindowState,
    last_node_settings: Option<crate::runtime::node_client::NodePresentation>,
    last_pool_settings_revision: Option<u64>,
    last_network_settings: Option<(crate::runtime::network::NetworkStatus, Option<crate::runtime::network::types::NetworkInterface>, Option<crate::runtime::network::types::IpAddress>, Option<crate::runtime::network::types::IpAddress>)>,
    last_app_window_x: i32,
    last_app_window_y: i32,
    last_app_window_width: i32,
    last_app_window_height: i32,
    last_app_window_maximized: bool,
    last_editor_saved: bool,
    system_ui_active: bool,
    cursor_saved: bool,
    cursor_left: usize,
    cursor_top: usize,
    cursor_width: usize,
    cursor_height: usize,
    cursor_backing: [u32; 128 * 128],
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    menu_saved: bool,
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    menu_left: usize,
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    menu_top: usize,
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    menu_width: usize,
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    menu_height: usize,
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    menu_backing: [u32; 640 * 800],
}

static mut CONSOLE: Option<ConsoleSurface> = None;
static mut POINTER_ACTIVITY_PENDING: bool = false;
static mut POINTER_ACTIVITY_GRACE_TICKS: u8 = 0;
static mut SYSTEM_CLOCK_TICKS: u8 = 0;

// ------------------------=
// FUNC: presentation_diagnostics
// DESC: Returns structured compositor presentation counters for IOP inspection and diagnostics.
// ------------------=
pub fn presentation_diagnostics() -> Option<DisplayPresentDiagnostics> {
    unsafe {
        let slot = &raw const CONSOLE;
        (*slot)
            .as_ref()
            .map(|console| console.display.diagnostics())
    }
}
