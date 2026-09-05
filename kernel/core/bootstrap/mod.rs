//! Thin framebuffer/UI facade preserving the public display and presentation API.

use crate::boot_info::BootInfo;

mod bootstrap;
mod desktop;
mod installer;
mod primitives;

use self::primitives::*;

pub use self::bootstrap::{animation_tick, console_present, note_pointer_activity, show_splash};
pub use self::desktop::system_ui_present;
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
}

const MAX_PRESENT_REGIONS: usize = 8;

#[derive(Clone, Copy, Default)]
struct PresentRegion {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
}

impl PresentRegion {
    // ------------------------=
    // FUNC: intersects_or_touches
    // DESC: Reports whether two presentation regions can be safely coalesced without a spatial gap.
    // ------------------=
    const fn intersects_or_touches(self, other: Self) -> bool {
        self.left <= other.right
            && other.left <= self.right
            && self.top <= other.bottom
            && other.top <= self.bottom
    }

    // ------------------------=
    // FUNC: union
    // DESC: Returns the smallest presentation region containing both inputs.
    // ------------------=
    const fn union(self, other: Self) -> Self {
        Self {
            left: if self.left < other.left {
                self.left
            } else {
                other.left
            },
            top: if self.top < other.top {
                self.top
            } else {
                other.top
            },
            right: if self.right > other.right {
                self.right
            } else {
                other.right
            },
            bottom: if self.bottom > other.bottom {
                self.bottom
            } else {
                other.bottom
            },
        }
    }

    // ------------------------=
    // FUNC: contains
    // DESC: Reports whether a pending region already covers a newly submitted region.
    // ------------------=
    const fn contains(self, other: Self) -> bool {
        self.left <= other.left
            && self.top <= other.top
            && self.right >= other.right
            && self.bottom >= other.bottom
    }
}

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
const MAX_SOFTWARE_BACK_BUFFER_PIXELS: usize = 2560 * 1600;

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
        let required = info.framebuffer_stride as u64 * info.framebuffer_height as u64 * 4;
        if info.framebuffer_address == 0
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
        })
    }

    // ------------------------=
    // FUNC: mark_dirty_rect
    // DESC: Adds a clipped framebuffer rectangle to the pending coherent presentation union.
    // ------------------=
    fn mark_dirty_rect(&mut self, left: usize, top: usize, width: usize, height: usize) {
        if width == 0 || height == 0 || left >= self.width || top >= self.height {
            return;
        }
        let mut submitted = PresentRegion {
            left,
            top,
            right: left.saturating_add(width).min(self.width),
            bottom: top.saturating_add(height).min(self.height),
        };
        let count = self.dirty_count as usize;
        for index in 0..count {
            if self.dirty_regions[index].contains(submitted) {
                return;
            }
            if self.dirty_regions[index].intersects_or_touches(submitted) {
                submitted = self.dirty_regions[index].union(submitted);
                self.dirty_regions[index] = submitted;
                return;
            }
        }
        if count < MAX_PRESENT_REGIONS {
            self.dirty_regions[count] = submitted;
            self.dirty_count += 1;
            return;
        }
        let mut collapsed = submitted;
        for region in &self.dirty_regions {
            collapsed = collapsed.union(*region);
        }
        self.dirty_regions[0] = collapsed;
        self.dirty_count = 1;
        self.damage_collapses = self.damage_collapses.saturating_add(1);
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
        for index in 0..self.dirty_count as usize {
            let region = self.dirty_regions[index];
            let width = region.right.saturating_sub(region.left);
            let height = region.bottom.saturating_sub(region.top);
            if self.back_buffered {
                for y in region.top..region.bottom {
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
        }
        self.presented_frames = self.presented_frames.wrapping_add(1);
        self.presented_pixels = self.presented_pixels.saturating_add(pixels);
        self.dirty_count = 0;
        pixels
    }

    // ------------------------=
    // FUNC: force_full_present
    // DESC: Requests and presents a complete frame for initialization or explicit recovery.
    // ------------------=
    fn force_full_present(&mut self) -> u64 {
        self.full_frame_fallbacks = self.full_frame_fallbacks.saturating_add(1);
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
    last_system_content: u32,
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
    last_desktop_items: u8,
    last_desktop_item_positions: [[i32; 2]; 7],
    last_system_clock: crate::storage::DateTimeConfiguration,
    last_settings_window: crate::ui::system_layout::SettingsWindowState,
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
    menu_backing: [u32; 600 * 800],
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
