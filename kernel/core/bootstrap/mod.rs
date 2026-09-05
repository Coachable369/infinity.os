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
    buffer: *mut u32,
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    format: u32,
}

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
        Some(Self {
            buffer: info.framebuffer_address as usize as *mut u32,
            width: info.framebuffer_width as usize,
            height: info.framebuffer_height as usize,
            stride: info.framebuffer_stride as usize,
            format: info.framebuffer_format,
        })
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
    last_settings_maximized: bool,
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
