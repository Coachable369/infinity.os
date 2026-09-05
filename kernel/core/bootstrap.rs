//! Shared framebuffer infrastructure for early and installed-system presentation.
//!
//! Screen-family rendering lives in the `first_boot`, `installer`, and
//! `post_install` child modules. Hardware discovery supplies the framebuffer;
//! this module retains shared bitmap scaling, text, pointer composition,
//! animation timing, and damage-limited presentation mechanisms.

use crate::boot_info::BootInfo;
use core::ptr::{read_volatile, write_volatile};

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const SPLASH_BMP: &[u8] = include_bytes!("../../assets/boot/infinity-eclipse-header-v1.bmp");
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
const CONSOLE_BACKGROUND_BMP: &[u8] =
    include_bytes!("../../assets/boot/infinity-console-background-v1.bmp");
#[cfg(not(feature = "installer"))]
const CONSOLE_BACKGROUND_BMP: &[u8] = &[];
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const AUTHENTICATION_BMP: &[u8] = include_bytes!("../../assets/desktop/infinity-default-dark-wallpaper-v2.bmp");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const DESKTOP_BMP: &[u8] = include_bytes!("../../assets/desktop/infinity-shell-wallpaper-v3.bmp");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const ONBOARDING_BMP: &[u8] = include_bytes!("../../assets/desktop/infinity-onboarding-wallpaper-v1.bmp");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const UI_FONT_ATLAS: &[u8] = include_bytes!("../../assets/fonts/InfinityUI-Regular-24.atlas");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const UI_FONT_SEMIBOLD_ATLAS: &[u8] = include_bytes!("../../assets/fonts/InfinityUI-Semibold-24.atlas");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const UI_FONT_METRICS: &[u8] = include_bytes!("../../assets/fonts/InfinityUI-Regular-24.metrics");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const UI_FONT_SEMIBOLD_METRICS: &[u8] =
    include_bytes!("../../assets/fonts/InfinityUI-Semibold-24.metrics");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const UI_FONT_KERN: &[u8] = include_bytes!("../../assets/fonts/InfinityUI-Regular-24.kern");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const UI_FONT_SEMIBOLD_KERN: &[u8] =
    include_bytes!("../../assets/fonts/InfinityUI-Semibold-24.kern");
#[cfg(target_arch = "x86")]
const UI_FONT_ATLAS: &[u8] = &[];
#[cfg(target_arch = "x86")]
const UI_FONT_SEMIBOLD_ATLAS: &[u8] = &[];
#[cfg(target_arch = "x86")]
const UI_FONT_METRICS: &[u8] = &[];
#[cfg(target_arch = "x86")]
const UI_FONT_SEMIBOLD_METRICS: &[u8] = &[];
#[cfg(target_arch = "x86")]
const UI_FONT_KERN: &[u8] = &[];
#[cfg(target_arch = "x86")]
const UI_FONT_SEMIBOLD_KERN: &[u8] = &[];
const UI_FONT_CELL_WIDTH: usize = 24;
const UI_FONT_CELL_HEIGHT: usize = 28;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const INSTALLER_FONT_ATLAS: &[u8] =
    include_bytes!("../../assets/fonts/InfinityInstaller-Regular-24.atlas");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const INSTALLER_FONT_SEMIBOLD_ATLAS: &[u8] =
    include_bytes!("../../assets/fonts/InfinityInstaller-Semibold-24.atlas");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const INSTALLER_FONT_METRICS: &[u8] =
    include_bytes!("../../assets/fonts/InfinityInstaller-Regular-24.metrics");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const INSTALLER_FONT_SEMIBOLD_METRICS: &[u8] =
    include_bytes!("../../assets/fonts/InfinityInstaller-Semibold-24.metrics");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const INSTALLER_FONT_KERN: &[u8] =
    include_bytes!("../../assets/fonts/InfinityInstaller-Regular-24.kern");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const INSTALLER_FONT_SEMIBOLD_KERN: &[u8] =
    include_bytes!("../../assets/fonts/InfinityInstaller-Semibold-24.kern");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const INSTALLER_HEADLINE_FONT_ATLAS: &[u8] =
    include_bytes!("../../assets/fonts/InfinityInstaller-Semibold-32.atlas");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const INSTALLER_HEADLINE_FONT_METRICS: &[u8] =
    include_bytes!("../../assets/fonts/InfinityInstaller-Semibold-32.metrics");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const INSTALLER_HEADLINE_FONT_KERN: &[u8] =
    include_bytes!("../../assets/fonts/InfinityInstaller-Semibold-32.kern");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const INSTALLER_COMPACT_FONT_ATLAS: &[u8] =
    include_bytes!("../../assets/fonts/InfinityInstaller-Regular-19.atlas");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const INSTALLER_COMPACT_FONT_SEMIBOLD_ATLAS: &[u8] =
    include_bytes!("../../assets/fonts/InfinityInstaller-Semibold-19.atlas");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const INSTALLER_COMPACT_FONT_METRICS: &[u8] =
    include_bytes!("../../assets/fonts/InfinityInstaller-Regular-19.metrics");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const INSTALLER_COMPACT_FONT_SEMIBOLD_METRICS: &[u8] =
    include_bytes!("../../assets/fonts/InfinityInstaller-Semibold-19.metrics");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const INSTALLER_COMPACT_FONT_KERN: &[u8] =
    include_bytes!("../../assets/fonts/InfinityInstaller-Regular-19.kern");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const INSTALLER_COMPACT_FONT_SEMIBOLD_KERN: &[u8] =
    include_bytes!("../../assets/fonts/InfinityInstaller-Semibold-19.kern");
#[cfg(target_arch = "x86")]
const INSTALLER_FONT_ATLAS: &[u8] = &[];
#[cfg(target_arch = "x86")]
const INSTALLER_FONT_SEMIBOLD_ATLAS: &[u8] = &[];
#[cfg(target_arch = "x86")]
const INSTALLER_FONT_METRICS: &[u8] = &[];
#[cfg(target_arch = "x86")]
const INSTALLER_FONT_SEMIBOLD_METRICS: &[u8] = &[];
#[cfg(target_arch = "x86")]
const INSTALLER_FONT_KERN: &[u8] = &[];
#[cfg(target_arch = "x86")]
const INSTALLER_FONT_SEMIBOLD_KERN: &[u8] = &[];
#[cfg(target_arch = "x86")]
const INSTALLER_COMPACT_FONT_ATLAS: &[u8] = &[];
#[cfg(target_arch = "x86")]
const INSTALLER_COMPACT_FONT_SEMIBOLD_ATLAS: &[u8] = &[];
#[cfg(target_arch = "x86")]
const INSTALLER_COMPACT_FONT_METRICS: &[u8] = &[];
#[cfg(target_arch = "x86")]
const INSTALLER_COMPACT_FONT_SEMIBOLD_METRICS: &[u8] = &[];
#[cfg(target_arch = "x86")]
const INSTALLER_COMPACT_FONT_KERN: &[u8] = &[];
#[cfg(target_arch = "x86")]
const INSTALLER_COMPACT_FONT_SEMIBOLD_KERN: &[u8] = &[];
const INSTALLER_FONT_CELL_WIDTH: usize = 24;
const INSTALLER_FONT_CELL_HEIGHT: usize = 28;
const INSTALLER_HEADLINE_FONT_CELL_WIDTH: usize = 32;
const INSTALLER_HEADLINE_FONT_CELL_HEIGHT: usize = 38;
const INSTALLER_COMPACT_FONT_CELL_WIDTH: usize = 20;
const INSTALLER_COMPACT_FONT_CELL_HEIGHT: usize = 24;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const EMBLEM_BMP: &[u8] = include_bytes!("../../assets/boot/infinity-emblem-v2.bmp");
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
const INSTALLER_BMP: &[u8] =
    include_bytes!("../../assets/boot/infinity-installer-background-v2.bmp");
#[cfg(not(feature = "installer"))]
const INSTALLER_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
const INSTALLER_MASTHEAD_BMP: &[u8] =
    include_bytes!("../../assets/boot/infinity-installer-masthead-v2.bmp");
#[cfg(not(feature = "installer"))]
const INSTALLER_MASTHEAD_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
const INSTALLER_WELCOME_MASTHEAD_BMP: &[u8] =
    include_bytes!("../../assets/boot/infinity-installer-masthead-v1.bmp");
#[cfg(not(feature = "installer"))]
const INSTALLER_WELCOME_MASTHEAD_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
const INSTALLER_MESH_HERO_BMP: &[u8] =
    include_bytes!("../../assets/boot/infinity-installer-mesh-hero-v1.bmp");
#[cfg(not(feature = "installer"))]
const INSTALLER_MESH_HERO_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
const INSTALLER_MESH_OVERVIEW_BMP: &[u8] =
    include_bytes!("../../assets/boot/infinity-installer-mesh-overview-v1.bmp");
#[cfg(not(feature = "installer"))]
const INSTALLER_MESH_OVERVIEW_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
const INSTALLER_ACTIVATION_BMP: &[u8] =
    include_bytes!("../../assets/boot/infinity-installer-activation-v2.bmp");
#[cfg(not(feature = "installer"))]
const INSTALLER_ACTIVATION_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
const STORAGE_HIERARCHY_BMP: &[u8] =
    include_bytes!("../../assets/boot/infinity-storage-hierarchy-v3.bmp");
#[cfg(not(feature = "installer"))]
const STORAGE_HIERARCHY_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
const DISK_DISCOVERY_VISION_BMP: &[u8] =
    include_bytes!("../../assets/boot/infinity-disk-discovery-vision-v1.bmp");
#[cfg(not(feature = "installer"))]
const DISK_DISCOVERY_VISION_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
const STORAGE_DEVICE_BMP: &[u8] =
    include_bytes!("../../assets/boot/infinity-storage-device-v1.bmp");
#[cfg(not(feature = "installer"))]
const STORAGE_DEVICE_BMP: &[u8] = &[];
#[cfg(all(
    feature = "installer",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
const DATE_TIME_WORLD_BMP: &[u8] =
    include_bytes!("../../assets/boot/infinity-time-zone-map-v1.bmp");
#[cfg(not(feature = "installer"))]
const DATE_TIME_WORLD_BMP: &[u8] = &[];
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const CURSOR_BMP: &[u8] = include_bytes!("../../assets/boot/infinity-cursor-v1.bmp");
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const BOOT_EMBLEM_TOP_PERCENT: usize = 23;
// Installer-only calibration: the ISO reveal pulse follows the visible ribbon
// centerline in the composited bootstrap artwork, which sits 50 pixels below
// the original mathematical path origin.
const BOOT_PARTICLE_Y_OFFSET: i32 = 50;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const CONSOLE_EMBLEM_TOP_PERCENT: usize = 18;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const CONSOLE_EMBLEM_WIDTH_PERCENT: usize = 44;
const INFINITY_PATH: [(i32, i32); 97] = [
    (0, 0),
    (7, 8),
    (15, 16),
    (22, 24),
    (29, 31),
    (36, 38),
    (43, 44),
    (50, 49),
    (56, 54),
    (62, 57),
    (68, 60),
    (74, 61),
    (79, 62),
    (84, 61),
    (89, 60),
    (93, 57),
    (97, 54),
    (100, 49),
    (103, 44),
    (106, 38),
    (108, 31),
    (110, 24),
    (111, 16),
    (112, 8),
    (112, 0),
    (112, -8),
    (111, -16),
    (110, -24),
    (108, -31),
    (106, -38),
    (103, -44),
    (100, -49),
    (97, -54),
    (93, -57),
    (89, -60),
    (84, -61),
    (79, -62),
    (74, -61),
    (68, -60),
    (62, -57),
    (56, -54),
    (50, -49),
    (43, -44),
    (36, -38),
    (29, -31),
    (22, -24),
    (15, -16),
    (7, -8),
    (0, 0),
    (-7, 8),
    (-15, 16),
    (-22, 24),
    (-29, 31),
    (-36, 38),
    (-43, 44),
    (-50, 49),
    (-56, 54),
    (-62, 57),
    (-68, 60),
    (-74, 61),
    (-79, 62),
    (-84, 61),
    (-89, 60),
    (-93, 57),
    (-97, 54),
    (-100, 49),
    (-103, 44),
    (-106, 38),
    (-108, 31),
    (-110, 24),
    (-111, 16),
    (-112, 8),
    (-112, 0),
    (-112, -8),
    (-111, -16),
    (-110, -24),
    (-108, -31),
    (-106, -38),
    (-103, -44),
    (-100, -49),
    (-97, -54),
    (-93, -57),
    (-89, -60),
    (-84, -61),
    (-79, -62),
    (-74, -61),
    (-68, -60),
    (-62, -57),
    (-56, -54),
    (-50, -49),
    (-43, -44),
    (-36, -38),
    (-29, -31),
    (-22, -24),
    (-15, -16),
    (-7, -8),
    (0, 0),
];

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

    // ------------------------=
    // FUNC: pixel
    // DESC: Writes one clipped RGB pixel using the firmware-provided channel format.
    // ------------------=
    fn pixel(&mut self, x: i32, y: i32, red: u8, green: u8, blue: u8) {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return;
        }
        let color = if self.format == 0 {
            red as u32 | (green as u32) << 8 | (blue as u32) << 16
        } else {
            blue as u32 | (green as u32) << 8 | (red as u32) << 16
        };
        unsafe {
            write_volatile(
                self.buffer.add(y as usize * self.stride + x as usize),
                color,
            );
        }
    }

    // ------------------------=
    // FUNC: blend_white
    // DESC: Alpha-blends white over one existing framebuffer pixel.
    // ------------------=
    fn blend_white(&mut self, x: i32, y: i32, alpha: u8) {
        self.blend_color(x, y, 255, 255, 255, alpha);
    }

    // ------------------------=
    // FUNC: blend_color
    // DESC: Alpha-blends an RGB color over one existing framebuffer pixel.
    // ------------------=
    fn blend_color(
        &mut self,
        x: i32,
        y: i32,
        target_red: u8,
        target_green: u8,
        target_blue: u8,
        alpha: u8,
    ) {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return;
        }
        let address = unsafe { self.buffer.add(y as usize * self.stride + x as usize) };
        let color = unsafe { read_volatile(address) };
        let (red, green, blue) = if self.format == 0 {
            (
                (color & 255) as u8,
                ((color >> 8) & 255) as u8,
                ((color >> 16) & 255) as u8,
            )
        } else {
            (
                ((color >> 16) & 255) as u8,
                ((color >> 8) & 255) as u8,
                (color & 255) as u8,
            )
        };
        let mix = |value: u8, target: u8| {
            ((value as u16 * (255 - alpha as u16) + target as u16 * alpha as u16) / 255) as u8
        };
        self.pixel(
            x,
            y,
            mix(red, target_red),
            mix(green, target_green),
            mix(blue, target_blue),
        );
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_background
    // DESC: Scales and paints the boot artwork across the complete display.
    // ------------------=
    fn paint_background(&mut self) {
        self.paint_background_rect(0, 0, self.width, self.height);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_background_rect
    // DESC: Restores a clipped display rectangle from the scaled boot artwork.
    // ------------------=
    fn paint_background_rect(&mut self, left: usize, top: usize, width: usize, height: usize) {
        self.paint_bitmap_cover_rect(SPLASH_BMP, left, top, width, height);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_console_background
    // DESC: Paints the distinct installed-system scene after bootstrap completes.
    // ------------------=
    fn paint_console_background(&mut self) {
        self.paint_bitmap_cover_rect(CONSOLE_BACKGROUND_BMP, 0, 0, self.width, self.height);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_console_background_rect
    // DESC: Restores a damaged console rectangle from the installed-system artwork.
    // ------------------=
    fn paint_console_background_rect(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        self.paint_bitmap_cover_rect(CONSOLE_BACKGROUND_BMP, left, top, width, height);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_first_boot_background
    // DESC: Paints the dedicated premium onboarding environment with left-side card contrast.
    // ------------------=
    fn paint_first_boot_background(&mut self) {
        self.paint_bitmap_cover_rect(ONBOARDING_BMP, 0, 0, self.width, self.height);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_desktop_background
    // DESC: Paints the high-resolution authenticated-session wallpaper without baked interface elements.
    // ------------------=
    fn paint_desktop_background(&mut self) {
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
    fn paint_authentication_background(&mut self) {
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
    fn paint_authentication_background_rect(
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
    fn paint_desktop_background_rect(
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
    fn skin_visual_mode(&self) -> u8 {
        crate::runtime::with_runtime(|runtime| {
            let id = runtime.ui.skins.active().id;
            if id == crate::ui::skin::SkinId::from_bytes(b"infinity.diagnostic.light") {
                1
            } else if id == crate::ui::skin::SkinId::from_bytes(b"infinity.safe") {
                2
            } else {
                0
            }
        }).unwrap_or(0)
    }

    #[cfg(target_arch = "x86")]
    // ------------------------=
    // FUNC: paint_desktop_background
    // DESC: Provides a dark legacy fallback when the high-resolution desktop surface is unavailable.
    // ------------------=
    fn paint_desktop_background(&mut self) {
        self.fill_rect(0, 0, self.width, self.height, 1, 7, 15);
    }

    #[cfg(target_arch = "x86")]
    // ------------------------=
    // FUNC: paint_authentication_background
    // DESC: Provides the legacy text-compatible sign-in canvas when high-resolution artwork is unavailable.
    // ------------------=
    fn paint_authentication_background(&mut self) {
        self.fill_rect(0, 0, self.width, self.height, 1, 7, 15);
    }

    #[cfg(target_arch = "x86")]
    // ------------------------=
    // FUNC: paint_authentication_background_rect
    // DESC: Restores a bounded legacy authentication region beneath animated particles.
    // ------------------=
    fn paint_authentication_background_rect(
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
    fn paint_desktop_background_rect(
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
    fn paint_first_boot_background(&mut self) {
        self.fill_rect(0, 0, self.width, self.height, 1, 7, 15);
    }

    // ------------------------=
    // FUNC: small_infinity_mark
    // DESC: Draws the native top-bar Infinity pill without baking UI into artwork.
    // ------------------=
    fn small_infinity_mark(&mut self, center_x: usize, center_y: usize, size: usize) {
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
    // FUNC: ui_text_centered
    // DESC: Centers smooth proportional UI text within a fixed horizontal region.
    // ------------------=
    fn ui_text_centered(&mut self, left: usize, width: usize, y: usize, text: &[u8], red: u8, green: u8, blue: u8, scale: usize) {
        let text_width = self.ui_text_width(text, scale);
        self.ui_text(left + width.saturating_sub(text_width) / 2, y, text, red, green, blue, scale);
    }

    // ------------------------=
    // FUNC: ui_text_centered_strong
    // DESC: Centers semibold UI text using the shared proportional metrics.
    // ------------------=
    fn ui_text_centered_strong(&mut self, left: usize, width: usize, y: usize, text: &[u8], red: u8, green: u8, blue: u8, scale: usize) {
        let text_width = self.ui_text_width_weighted(text, scale, true);
        self.ui_text_strong(left + width.saturating_sub(text_width) / 2, y, text, red, green, blue, scale);
    }

    // ------------------------=
    // FUNC: ui_text_fit_strong
    // DESC: Draws a heading at its preferred hierarchy and safely falls back when the measured glyphs exceed the container.
    // ------------------=
    fn ui_text_fit_strong(&mut self, x: usize, y: usize, max_width: usize, text: &[u8], red: u8, green: u8, blue: u8, preferred_scale: usize) {
        let scale = if self.ui_text_width_weighted(text, preferred_scale, true) <= max_width {
            preferred_scale
        } else {
            1
        };
        self.ui_text_strong(x, y, text, red, green, blue, scale);
    }

    // ------------------------=
    // FUNC: ui_text_wrapped
    // DESC: Wraps proportional UI copy at word boundaries without allocating or drawing beyond the supplied width.
    // ------------------=
    fn ui_text_wrapped(&mut self, x: usize, y: usize, max_width: usize, text: &[u8], red: u8, green: u8, blue: u8, max_lines: usize) {
        let mut start = 0usize;
        let mut line = 0usize;
        while start < text.len() && line < max_lines {
            while start < text.len() && text[start] == b' ' {
                start += 1;
            }
            if start >= text.len() {
                break;
            }
            let mut end = start + 1;
            let mut last_space = None;
            while end <= text.len() {
                if end < text.len() && text[end] == b' ' {
                    last_space = Some(end);
                }
                if self.ui_text_width(&text[start..end], 1) > max_width {
                    end = last_space.unwrap_or(end.saturating_sub(1).max(start + 1));
                    break;
                }
                if end == text.len() {
                    break;
                }
                end += 1;
            }
            self.ui_text(
                x,
                y + line * (UI_FONT_CELL_HEIGHT + 4) * self.ui_scale(),
                &text[start..end],
                red,
                green,
                blue,
                1,
            );
            start = end.saturating_add((end < text.len() && text[end] == b' ') as usize);
            line += 1;
        }
    }

    // ------------------------=
    // FUNC: ui_text_width
    // DESC: Measures smooth UI copy using the same proportional glyph advances used by the rasterizer.
    // ------------------=
    fn ui_text_width(&self, text: &[u8], scale: usize) -> usize {
        self.ui_text_width_weighted(text, scale, false)
    }

    // ------------------------=
    // FUNC: ui_text_width_weighted
    // DESC: Measures Roboto UI text with the same proportional advances and pair kerning used for drawing.
    // ------------------=
    fn ui_text_width_weighted(&self, text: &[u8], scale: usize, semibold: bool) -> usize {
        let scale = self.ui_effective_text_scale(scale);
        let metrics = if semibold {
            UI_FONT_SEMIBOLD_METRICS
        } else {
            UI_FONT_METRICS
        };
        let kerning = if semibold {
            UI_FONT_SEMIBOLD_KERN
        } else {
            UI_FONT_KERN
        };
        let mut width = 0usize;
        let mut previous = None;
        for byte in text {
            if !(32..=126).contains(byte) {
                previous = None;
                continue;
            }
            width = Self::font_position_advance(
                width,
                Self::font_pair_adjustment(kerning, previous, *byte) * scale as isize,
            );
            width = width.saturating_add(metrics[*byte as usize - 32] as usize * scale);
            previous = Some(*byte);
        }
        width
    }

    // ------------------------=
    // FUNC: ui_effective_text_scale
    // DESC: Selects a legible anti-aliased text scale for the active display while preserving title hierarchy.
    // ------------------=
    fn ui_effective_text_scale(&self, requested: usize) -> usize {
        requested.max(1).min(3)
    }

    // ------------------------=
    // FUNC: font_pair_adjustment
    // DESC: Decodes one signed Roboto pair-kerning adjustment from a compact lookup table.
    // ------------------=
    fn font_pair_adjustment(table: &[u8], previous: Option<u8>, current: u8) -> isize {
        let Some(previous) = previous else {
            return 0;
        };
        if !(32..=126).contains(&previous) || !(32..=126).contains(&current) {
            return 0;
        }
        table[(previous as usize - 32) * 95 + current as usize - 32] as isize - 128
    }

    // ------------------------=
    // FUNC: font_position_advance
    // DESC: Applies a signed kerning delta without allowing a text coordinate to underflow.
    // ------------------=
    fn font_position_advance(position: usize, adjustment: isize) -> usize {
        if adjustment < 0 {
            position.saturating_sub(adjustment.unsigned_abs())
        } else {
            position.saturating_add(adjustment as usize)
        }
    }

}

mod first_boot;
mod installer;
mod post_install;

impl DisplayDevice {

    // ------------------------=
    // FUNC: ui_text
    // DESC: Alpha-rasterizes bundled Roboto Regular for smooth bootstrap and desktop typography.
    // ------------------=
    fn ui_text(
        &mut self,
        x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        scale: usize,
    ) {
        self.ui_text_weighted(x, y, text, red, green, blue, scale, false);
    }

    // ------------------------=
    // FUNC: ui_text_strong
    // DESC: Renders headings and controls with bundled Roboto Medium.
    // ------------------=
    fn ui_text_strong(
        &mut self,
        x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        scale: usize,
    ) {
        self.ui_text_weighted(x, y, text, red, green, blue, scale, true);
    }

    // ------------------------=
    // FUNC: ui_text_weighted
    // DESC: Alpha-rasterizes one of the bundled proportional UI font weights.
    // ------------------=
    fn ui_text_weighted(
        &mut self,
        mut x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        scale: usize,
        semibold: bool,
    ) {
        let scale = self.ui_effective_text_scale(scale);
        let atlas = if semibold {
            UI_FONT_SEMIBOLD_ATLAS
        } else {
            UI_FONT_ATLAS
        };
        let metrics = if semibold {
            UI_FONT_SEMIBOLD_METRICS
        } else {
            UI_FONT_METRICS
        };
        let kerning = if semibold {
            UI_FONT_SEMIBOLD_KERN
        } else {
            UI_FONT_KERN
        };
        let mut previous = None;
        for byte in text {
            if !(32..=126).contains(byte) {
                previous = None;
                continue;
            }
            x = Self::font_position_advance(
                x,
                Self::font_pair_adjustment(kerning, previous, *byte) * scale as isize,
            );
            let glyph = (*byte as usize - 32) * UI_FONT_CELL_WIDTH;
            for row in 0..UI_FONT_CELL_HEIGHT {
                for column in 0..UI_FONT_CELL_WIDTH {
                    let alpha = atlas[row * UI_FONT_CELL_WIDTH * 95 + glyph + column];
                    if alpha == 0 {
                        continue;
                    }
                    for py in 0..scale {
                        for px in 0..scale {
                            self.blend_color(
                                (x + column * scale + px) as i32,
                                (y + row * scale + py) as i32,
                                red,
                                green,
                                blue,
                                alpha,
                            );
                        }
                    }
                }
            }
            x = x.saturating_add(metrics[*byte as usize - 32] as usize * scale);
            previous = Some(*byte);
        }
    }

    // ------------------------=
    // FUNC: installer_text_width
    // DESC: Measures installer copy at the reference panel's fixed antialiased type size.
    // ------------------=
    fn installer_text_width(&self, text: &[u8], semibold: bool) -> usize {
        let metrics = if semibold {
            INSTALLER_FONT_SEMIBOLD_METRICS
        } else {
            INSTALLER_FONT_METRICS
        };
        let kerning = if semibold {
            INSTALLER_FONT_SEMIBOLD_KERN
        } else {
            INSTALLER_FONT_KERN
        };
        let mut width = 0usize;
        let mut previous = None;
        for byte in text {
            if !(32..=126).contains(byte) {
                previous = None;
                continue;
            }
            width = Self::font_position_advance(
                width,
                Self::font_pair_adjustment(kerning, previous, *byte),
            );
            width = width.saturating_add(metrics[*byte as usize - 32] as usize);
            previous = Some(*byte);
        }
        width
    }

    // ------------------------=
    // FUNC: installer_text
    // DESC: Draws installer body copy with the fixed-size smooth reference typography.
    // ------------------=
    fn installer_text(
        &mut self,
        x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
    ) {
        self.installer_text_weighted(x, y, text, red, green, blue, false);
    }

    // ------------------------=
    // FUNC: installer_text_strong
    // DESC: Draws installer headings with the fixed-size smooth semibold reference typography.
    // ------------------=
    fn installer_text_strong(
        &mut self,
        x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
    ) {
        self.installer_text_weighted(x, y, text, red, green, blue, true);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: installer_headline_text_width
    // DESC: Measures the larger semibold face reserved for installer hero statements.
    // ------------------=
    fn installer_headline_text_width(&self, text: &[u8]) -> usize {
        let mut width = 0usize;
        let mut previous = None;
        for byte in text {
            if !(32..=126).contains(byte) {
                previous = None;
                continue;
            }
            width = Self::font_position_advance(
                width,
                Self::font_pair_adjustment(INSTALLER_HEADLINE_FONT_KERN, previous, *byte),
            );
            width = width.saturating_add(
                INSTALLER_HEADLINE_FONT_METRICS[*byte as usize - 32] as usize,
            );
            previous = Some(*byte);
        }
        width
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: installer_headline_text
    // DESC: Alpha-rasterizes the 32px semibold installer hero face.
    // ------------------=
    fn installer_headline_text(
        &mut self,
        mut x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
    ) {
        let mut previous = None;
        for byte in text {
            if !(32..=126).contains(byte) {
                previous = None;
                continue;
            }
            x = Self::font_position_advance(
                x,
                Self::font_pair_adjustment(INSTALLER_HEADLINE_FONT_KERN, previous, *byte),
            );
            let glyph = (*byte as usize - 32) * INSTALLER_HEADLINE_FONT_CELL_WIDTH;
            for row in 0..INSTALLER_HEADLINE_FONT_CELL_HEIGHT {
                for column in 0..INSTALLER_HEADLINE_FONT_CELL_WIDTH {
                    let alpha = INSTALLER_HEADLINE_FONT_ATLAS[
                        row * INSTALLER_HEADLINE_FONT_CELL_WIDTH * 95 + glyph + column
                    ];
                    if alpha != 0 {
                        self.blend_color(
                            (x + column) as i32,
                            (y + row) as i32,
                            red,
                            green,
                            blue,
                            alpha,
                        );
                    }
                }
            }
            x = x.saturating_add(
                INSTALLER_HEADLINE_FONT_METRICS[*byte as usize - 32] as usize,
            );
            previous = Some(*byte);
        }
    }

    // ------------------------=
    // FUNC: installer_text_weighted
    // DESC: Alpha-rasterizes installer text without inheriting desktop DPI multiplication.
    // ------------------=
    fn installer_text_weighted(
        &mut self,
        mut x: usize,
        mut y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        semibold: bool,
    ) {
        let line_start = x;
        let atlas = if semibold {
            INSTALLER_FONT_SEMIBOLD_ATLAS
        } else {
            INSTALLER_FONT_ATLAS
        };
        let metrics = if semibold {
            INSTALLER_FONT_SEMIBOLD_METRICS
        } else {
            INSTALLER_FONT_METRICS
        };
        let kerning = if semibold {
            INSTALLER_FONT_SEMIBOLD_KERN
        } else {
            INSTALLER_FONT_KERN
        };
        let mut previous = None;
        for byte in text {
            if *byte == b'\n' {
                x = line_start;
                y = y.saturating_add(INSTALLER_FONT_CELL_HEIGHT + 4);
                previous = None;
                continue;
            }
            if !(32..=126).contains(byte) {
                previous = None;
                continue;
            }
            x = Self::font_position_advance(
                x,
                Self::font_pair_adjustment(kerning, previous, *byte),
            );
            let glyph = (*byte as usize - 32) * INSTALLER_FONT_CELL_WIDTH;
            for row in 0..INSTALLER_FONT_CELL_HEIGHT {
                for column in 0..INSTALLER_FONT_CELL_WIDTH {
                    let alpha = atlas
                        [row * INSTALLER_FONT_CELL_WIDTH * 95 + glyph + column];
                    if alpha != 0 {
                        self.blend_color(
                            (x + column) as i32,
                            (y + row) as i32,
                            red,
                            green,
                            blue,
                            alpha,
                        );
                    }
                }
            }
            x += metrics[*byte as usize - 32] as usize;
            previous = Some(*byte);
        }
    }

    // ------------------------=
    // FUNC: installer_text_wrapped
    // DESC: Wraps installer copy into a bounded card using the reference's compact line rhythm.
    // ------------------=
    fn installer_text_wrapped(
        &mut self,
        x: usize,
        y: usize,
        max_width: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        max_lines: usize,
    ) {
        let mut start = 0usize;
        let mut line = 0usize;
        while start < text.len() && line < max_lines {
            while start < text.len() && text[start] == b' ' {
                start += 1;
            }
            if start >= text.len() {
                break;
            }
            let mut end = start + 1;
            let mut last_space = None;
            while end <= text.len() {
                if end < text.len() && text[end] == b' ' {
                    last_space = Some(end);
                }
                if self.installer_text_width(&text[start..end], false) > max_width {
                    end = last_space.unwrap_or(end.saturating_sub(1).max(start + 1));
                    break;
                }
                if end == text.len() {
                    break;
                }
                end += 1;
            }
            self.installer_text(
                x,
                y + line * (INSTALLER_FONT_CELL_HEIGHT + 2),
                &text[start..end],
                red,
                green,
                blue,
            );
            start = end;
            line += 1;
        }
    }

    // ------------------------=
    // FUNC: installer_compact_text_width
    // DESC: Measures the smaller Roboto companion used only by dense four-column information cards.
    // ------------------=
    fn installer_compact_text_width(&self, text: &[u8], semibold: bool) -> usize {
        let metrics = if semibold {
            INSTALLER_COMPACT_FONT_SEMIBOLD_METRICS
        } else {
            INSTALLER_COMPACT_FONT_METRICS
        };
        let kerning = if semibold {
            INSTALLER_COMPACT_FONT_SEMIBOLD_KERN
        } else {
            INSTALLER_COMPACT_FONT_KERN
        };
        let mut width = 0usize;
        let mut previous = None;
        for byte in text {
            if !(32..=126).contains(byte) {
                previous = None;
                continue;
            }
            width = Self::font_position_advance(
                width,
                Self::font_pair_adjustment(kerning, previous, *byte),
            );
            width = width.saturating_add(metrics[*byte as usize - 32] as usize);
            previous = Some(*byte);
        }
        width
    }

    // ------------------------=
    // FUNC: installer_compact_text
    // DESC: Draws compact card body copy with the 19px Roboto companion face.
    // ------------------=
    fn installer_compact_text(
        &mut self,
        x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
    ) {
        self.installer_compact_text_weighted(x, y, text, red, green, blue, false);
    }

    // ------------------------=
    // FUNC: installer_compact_text_strong
    // DESC: Draws compact card labels with the 19px Roboto Medium companion face.
    // ------------------=
    fn installer_compact_text_strong(
        &mut self,
        x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
    ) {
        self.installer_compact_text_weighted(x, y, text, red, green, blue, true);
    }

    // ------------------------=
    // FUNC: installer_compact_text_weighted
    // DESC: Alpha-rasterizes one compact Roboto weight with proportional metrics and pair kerning.
    // ------------------=
    fn installer_compact_text_weighted(
        &mut self,
        mut x: usize,
        y: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        semibold: bool,
    ) {
        let atlas = if semibold {
            INSTALLER_COMPACT_FONT_SEMIBOLD_ATLAS
        } else {
            INSTALLER_COMPACT_FONT_ATLAS
        };
        let metrics = if semibold {
            INSTALLER_COMPACT_FONT_SEMIBOLD_METRICS
        } else {
            INSTALLER_COMPACT_FONT_METRICS
        };
        let kerning = if semibold {
            INSTALLER_COMPACT_FONT_SEMIBOLD_KERN
        } else {
            INSTALLER_COMPACT_FONT_KERN
        };
        let mut previous = None;
        for byte in text {
            if !(32..=126).contains(byte) {
                previous = None;
                continue;
            }
            x = Self::font_position_advance(
                x,
                Self::font_pair_adjustment(kerning, previous, *byte),
            );
            let glyph = (*byte as usize - 32) * INSTALLER_COMPACT_FONT_CELL_WIDTH;
            for row in 0..INSTALLER_COMPACT_FONT_CELL_HEIGHT {
                for column in 0..INSTALLER_COMPACT_FONT_CELL_WIDTH {
                    let alpha = atlas[row * INSTALLER_COMPACT_FONT_CELL_WIDTH * 95 + glyph + column];
                    if alpha != 0 {
                        self.blend_color(
                            (x + column) as i32,
                            (y + row) as i32,
                            red,
                            green,
                            blue,
                            alpha,
                        );
                    }
                }
            }
            x = x.saturating_add(metrics[*byte as usize - 32] as usize);
            previous = Some(*byte);
        }
    }

    // ------------------------=
    // FUNC: installer_compact_text_wrapped
    // DESC: Wraps compact card copy without shrinking or clipping individual glyphs.
    // ------------------=
    fn installer_compact_text_wrapped(
        &mut self,
        x: usize,
        y: usize,
        max_width: usize,
        text: &[u8],
        red: u8,
        green: u8,
        blue: u8,
        max_lines: usize,
    ) {
        let mut start = 0usize;
        let mut line = 0usize;
        while start < text.len() && line < max_lines {
            while start < text.len() && text[start] == b' ' {
                start += 1;
            }
            if start >= text.len() {
                break;
            }
            let mut end = start + 1;
            let mut last_space = None;
            while end <= text.len() {
                if end < text.len() && text[end] == b' ' {
                    last_space = Some(end);
                }
                if self.installer_compact_text_width(&text[start..end], false) > max_width {
                    end = last_space.unwrap_or(end.saturating_sub(1).max(start + 1));
                    break;
                }
                if end == text.len() {
                    break;
                }
                end += 1;
            }
            self.installer_compact_text(
                x,
                y + line * (INSTALLER_COMPACT_FONT_CELL_HEIGHT + 1),
                &text[start..end],
                red,
                green,
                blue,
            );
            start = end;
            line += 1;
        }
    }



    // ------------------------=
    // FUNC: pointer_cursor
    // DESC: Draws the scaled cursor artwork at a clipped screen position.
    // ------------------=
    fn pointer_cursor(&mut self, cursor_x: i32, cursor_y: i32) {
        let x = self.width as i32 * cursor_x / 1000;
        let y = self.height as i32 * cursor_y / 1000;
        let scale = self.ui_scale();
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        {
            let offset = le32(CURSOR_BMP, 10) as usize;
            let source_width = le32(CURSOR_BMP, 18) as usize;
            let signed_height = le32(CURSOR_BMP, 22) as i32;
            let source_height = signed_height.unsigned_abs() as usize;
            let size = 28 * scale;
            for py in 0..size {
                for px in 0..size {
                    let sx = px * source_width / size;
                    let sy = py * source_height / size;
                    let source_y = if signed_height < 0 {
                        sy
                    } else {
                        source_height - 1 - sy
                    };
                    let index = offset + (source_y * source_width + sx) * 4;
                    if index + 3 >= CURSOR_BMP.len() {
                        return;
                    }
                    let alpha = CURSOR_BMP[index + 3];
                    if alpha > 2 {
                        self.blend_color(
                            x + px as i32,
                            y + py as i32,
                            CURSOR_BMP[index + 2],
                            CURSOR_BMP[index + 1],
                            CURSOR_BMP[index],
                            alpha,
                        );
                    }
                }
            }
        }
        #[cfg(target_arch = "x86")]
        for row in 0..14 * scale as i32 {
            for column in 0..=(row / 2) {
                self.pixel(x + column + 2, y + row + 3, 20, 24, 32);
                self.pixel(x + column, y + row, 248, 250, 255);
            }
        }
    }

    // ------------------------=
    // FUNC: fill_rect
    // DESC: Fills a clipped rectangle with a solid RGB color.
    // ------------------=
    fn fill_rect(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        red: u8,
        green: u8,
        blue: u8,
    ) {
        for y in top..(top + height).min(self.height) {
            for x in left..(left + width).min(self.width) {
                self.pixel(x as i32, y as i32, red, green, blue);
            }
        }
    }

    // ------------------------=
    // FUNC: fill_rect_alpha
    // DESC: Applies a translucent RGB fill while retaining the scene beneath a panel.
    // ------------------=
    fn fill_rect_alpha(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        red: u8,
        green: u8,
        blue: u8,
        alpha: u8,
    ) {
        for y in top..(top + height).min(self.height) {
            for x in left..(left + width).min(self.width) {
                self.blend_color(x as i32, y as i32, red, green, blue, alpha);
            }
        }
    }

    // ------------------------=
    // FUNC: fill_rounded_rect_alpha
    // DESC: Applies a clipped translucent fill with geometrically smooth rounded corners.
    // ------------------=
    fn fill_rounded_rect_alpha(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        radius: usize,
        red: u8,
        green: u8,
        blue: u8,
        alpha: u8,
    ) {
        let radius = radius.min(width / 2).min(height / 2).max(1);
        let right = left.saturating_add(width).min(self.width);
        let bottom = top.saturating_add(height).min(self.height);
        let radius_squared = (radius * radius) as i64;
        for y in top..bottom {
            for x in left..right {
                let dx = if x < left + radius {
                    left + radius - x
                } else if x >= left + width.saturating_sub(radius) {
                    x.saturating_sub(left + width.saturating_sub(radius) - 1)
                } else {
                    0
                };
                let dy = if y < top + radius {
                    top + radius - y
                } else if y >= top + height.saturating_sub(radius) {
                    y.saturating_sub(top + height.saturating_sub(radius) - 1)
                } else {
                    0
                };
                if dx == 0 || dy == 0 || (dx * dx + dy * dy) as i64 <= radius_squared {
                    self.blend_color(x as i32, y as i32, red, green, blue, alpha);
                }
            }
        }
    }

    // ------------------------=
    // FUNC: outline_rounded_rect
    // DESC: Draws a one-pixel rounded border while preserving transparent panel corners.
    // ------------------=
    fn outline_rounded_rect(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        radius: usize,
        red: u8,
        green: u8,
        blue: u8,
    ) {
        let radius = radius.min(width / 2).min(height / 2).max(1);
        let outer = (radius * radius) as i64;
        let inner_radius = radius.saturating_sub(1);
        let inner = (inner_radius * inner_radius) as i64;
        let right = left.saturating_add(width).min(self.width);
        let bottom = top.saturating_add(height).min(self.height);
        for y in top..bottom {
            for x in left..right {
                let edge = x == left || y == top || x + 1 == left + width || y + 1 == top + height;
                let dx = if x < left + radius { left + radius - x } else if x >= left + width.saturating_sub(radius) { x.saturating_sub(left + width.saturating_sub(radius) - 1) } else { 0 };
                let dy = if y < top + radius { top + radius - y } else if y >= top + height.saturating_sub(radius) { y.saturating_sub(top + height.saturating_sub(radius) - 1) } else { 0 };
                let corner_distance = (dx * dx + dy * dy) as i64;
                let corner_edge = dx > 0 && dy > 0 && corner_distance <= outer && corner_distance >= inner;
                if (edge && (dx == 0 || dy == 0)) || corner_edge {
                    self.pixel(x as i32, y as i32, red, green, blue);
                }
            }
        }
    }

    // ------------------------=
    // FUNC: outline_rect
    // DESC: Draws a one-pixel rectangular border in the requested RGB color.
    // ------------------=
    fn outline_rect(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        red: u8,
        green: u8,
        blue: u8,
    ) {
        for x in left..(left + width).min(self.width) {
            self.pixel(x as i32, top as i32, red, green, blue);
            self.pixel(
                x as i32,
                (top + height.saturating_sub(1)) as i32,
                red,
                green,
                blue,
            );
        }
        for y in top..(top + height).min(self.height) {
            self.pixel(left as i32, y as i32, red, green, blue);
            self.pixel(
                (left + width.saturating_sub(1)) as i32,
                y as i32,
                red,
                green,
                blue,
            );
        }
    }

    // ------------------------=
    // FUNC: line
    // DESC: Rasterizes a clipped straight line for installer icons and accents.
    // ------------------=
    fn line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, red: u8, green: u8, blue: u8) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        loop {
            self.pixel(x0, y0, red, green, blue);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let twice = error * 2;
            if twice >= dy {
                error += dy;
                x0 += sx;
            }
            if twice <= dx {
                error += dx;
                y0 += sy;
            }
        }
    }

    // ------------------------=
    // FUNC: icon_line
    // DESC: Draws a scale-aware anti-aliased-looking icon stroke that remains legible after VM display scaling.
    // ------------------=
    fn icon_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: (u8, u8, u8), icon_size: usize) {
        let radius = (icon_size / 18).clamp(1, 3) as i32;
        for offset in -radius..=radius {
            self.line(x0 + offset, y0, x1 + offset, y1, color.0, color.1, color.2);
            self.line(x0, y0 + offset, x1, y1 + offset, color.0, color.1, color.2);
        }
    }

    // ------------------------=
    // FUNC: icon_circle
    // DESC: Rasterizes a scale-aware circular vector stroke for semantic skin icons.
    // ------------------=
    fn icon_circle(&mut self, center_x: i32, center_y: i32, radius: i32, color: (u8, u8, u8), icon_size: usize) {
        let mut x = radius.max(1);
        let mut y = 0i32;
        let mut error = 1 - x;
        let weight = (icon_size / 20).clamp(1, 2) as i32;
        while x >= y {
            for offset in -weight..=weight {
                for (px, py) in [
                    (x, y), (y, x), (-y, x), (-x, y),
                    (-x, -y), (-y, -x), (y, -x), (x, -y),
                ] {
                    self.blend_color(center_x + px + offset, center_y + py, color.0, color.1, color.2, 235);
                }
            }
            y += 1;
            if error < 0 {
                error += 2 * y + 1;
            } else {
                x -= 1;
                error += 2 * (y - x) + 1;
            }
        }
    }

    // ------------------------=
    // FUNC: text
    // DESC: Draws text using the standard interface scale and bitmap font.
    // ------------------=
    fn text(&mut self, x: usize, y: usize, text: &[u8], r: u8, g: u8, b: u8) {
        self.installer_text(x, y, text, r, g, b);
    }

    // ------------------------=
    // FUNC: text_scaled
    // DESC: Rasterizes glyphs at an explicit integer scale with newline support.
    // ------------------=
    fn text_scaled(
        &mut self,
        mut x: usize,
        y: usize,
        text: &[u8],
        r: u8,
        g: u8,
        b: u8,
        scale: usize,
        embolden: bool,
    ) {
        for &character in text {
            let glyph = glyph(character);
            for (row, bits) in glyph.iter().enumerate() {
                for column in 0..8 {
                    if bits & (1 << (7 - column)) != 0 {
                        for py in 0..scale {
                            for px in 0..scale {
                                self.pixel(
                                    (x + column * scale + px) as i32,
                                    (y + row * scale + py) as i32,
                                    r,
                                    g,
                                    b,
                                );
                            }
                        }
                        // At compact resolutions a one-pixel stem becomes faint
                        // after VirtualBox scales the guest surface. Give 1x text
                        // a second horizontal sample without changing its metrics.
                        if embolden && scale == 1 {
                            self.pixel((x + column + 1) as i32, (y + row) as i32, r, g, b);
                        }
                    }
                }
            }
            x += 9 * scale;
        }
    }

    // The bundled atlas already contains 16-pixel anti-aliased glyphs. Keep its
    // native scale through ordinary 720p, 1080p, and square HiDPI VM modes so
    // text metrics remain proportional to the component geometry. Scale the
    // complete UI only on true 1440p-or-larger surfaces.
    // ------------------------=
    // FUNC: ui_scale
    // DESC: Selects a readable integer UI scale from the current display resolution.
    // ------------------=
    fn ui_scale(&self) -> usize {
        if self.width >= 2560 && self.height >= 1440 {
            2
        } else {
            1
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: infinity_screen_position
    // DESC: Converts an infinity-path phase into scaled screen coordinates.
    // ------------------=
    fn infinity_screen_position(&self, phase: usize, bootstrap: bool) -> (i32, i32) {
        let (left, top, width, height) = if bootstrap {
            let source_width = le32(EMBLEM_BMP, 18) as usize;
            let source_height = (le32(EMBLEM_BMP, 22) as i32).unsigned_abs() as usize;
            let width = (self.width * 52 / 100)
                .min((self.height * 70 / 100) * source_width / source_height);
            let height = width * source_height / source_width;
            (
                (self.width - width) / 2,
                self.height * BOOT_EMBLEM_TOP_PERCENT / 100,
                width,
                height,
            )
        } else {
            self.console_emblem_geometry()
        };
        let (px, py) = infinity_point(phase);
        let center_x = left as i32 + width as i32 / 2;
        let center_y = top as i32 + height as i32 / 2;
        (
            center_x + px * width as i32 / 224,
            center_y + py * height as i32 / 124,
        )
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: console_emblem_geometry
    // DESC: Reserves a resolution-independent emblem rectangle above the Console panel.
    // ------------------=
    fn console_emblem_geometry(&self) -> (usize, usize, usize, usize) {
        let source_width = le32(EMBLEM_BMP, 18) as usize;
        let source_height = (le32(EMBLEM_BMP, 22) as i32).unsigned_abs() as usize;
        let panel_top = self.height * 68 / 100;
        let top = self.height * CONSOLE_EMBLEM_TOP_PERCENT / 100;
        let safe_height = panel_top.saturating_sub(top + self.height * 4 / 100);
        let width_from_height = safe_height * source_width / source_height;
        let target_width = (self.width * CONSOLE_EMBLEM_WIDTH_PERCENT / 100)
            .min(width_from_height)
            .max(1);
        let target_height = target_width * source_height / source_width;
        (
            (self.width - target_width) / 2,
            top,
            target_width,
            target_height,
        )
    }
}

// ------------------------=
// FUNC: infinity_point
// DESC: Looks up a normalized point along the closed infinity animation path.
// ------------------=
fn infinity_point(phase: usize) -> (i32, i32) {
    let step = (phase / 4) % 96;
    let fraction = (phase % 4) as i32;
    let (ax, ay) = INFINITY_PATH[step];
    let (bx, by) = INFINITY_PATH[step + 1];
    (ax + (bx - ax) * fraction / 4, ay + (by - ay) * fraction / 4)
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

// ------------------------=
// FUNC: note_pointer_activity
// DESC: Marks recent pointer motion so bootstrap effects yield the current frame to interactive input.
// ------------------=
pub fn note_pointer_activity() {
    unsafe {
        POINTER_ACTIVITY_PENDING = true;
        // Keep the software-rendered particle pass out of the short interval
        // between positioning the pointer and pressing a button. VirtualBox's
        // firmware pointer exposes only its current button state; if a costly
        // frame spans both press and release, the click cannot be recovered.
        // Thirty 60 Hz deadlines provide a 500 ms interaction window while
        // leaving the animation completely smooth whenever the pointer rests.
        POINTER_ACTIVITY_GRACE_TICKS = 30;
    }
}

// ------------------------=
// FUNC: activate_console
// DESC: Transfers the display device into the persistent interactive console surface.
// ------------------=
fn activate_console(display: DisplayDevice) {
    unsafe {
        CONSOLE = Some(ConsoleSurface {
            display,
            particle_phase: 0,
            particles_initialized: false,
            bootstrap_scene: true,
            split_layout: false,
            layout_initialized: false,
            cursor_x: 0,
            cursor_y: 0,
            pointer_pressed: false,
            installer_scene: false,
            last_installer_screen: 0,
            last_installer_focus: usize::MAX,
            last_installer_choice: usize::MAX,
            last_installer_date_time: crate::storage::DateTimeConfiguration::utc_default(),
            last_installer_date_time_part: usize::MAX,
            installer_progress: 0,
            installer_animation_phase: 0,
            last_system_screen: 0,
            last_system_step: usize::MAX,
            last_system_focus: usize::MAX,
            last_system_menu: usize::MAX,
            last_system_content: 0,
            last_system_validation_error: false,
            last_home_window_x: i32::MIN,
            last_home_window_y: i32::MIN,
            last_home_window_width: i32::MIN,
            last_home_window_height: i32::MIN,
            last_home_window_visible: false,
            last_home_window_maximized: false,
            last_home_location: usize::MAX,
            last_home_selected_item: None,
            last_home_dragging_item: None,
            last_home_note_location: usize::MAX,
            last_desktop_items: 0,
            last_desktop_item_positions: [[i32::MIN; 2]; 7],
            last_system_clock: crate::storage::DateTimeConfiguration::utc_default(),
            last_settings_maximized: false,
            system_ui_active: false,
            cursor_saved: false,
            cursor_left: 0,
            cursor_top: 0,
            cursor_width: 0,
            cursor_height: 0,
            cursor_backing: [0; 128 * 128],
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            menu_saved: false,
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            menu_left: 0,
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            menu_top: 0,
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            menu_width: 0,
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            menu_height: 0,
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            menu_backing: [0; 600 * 800],
        });
    }
}

impl ConsoleSurface {
    // ------------------------=
    // FUNC: restore_cursor
    // DESC: Restores the saved pixels underneath the cursor from the previous frame.
    // ------------------=
    fn restore_cursor(&mut self) {
        if !self.cursor_saved {
            return;
        }
        for y in 0..self.cursor_height {
            for x in 0..self.cursor_width {
                unsafe {
                    write_volatile(
                        self.display.buffer.add(
                            (self.cursor_top + y) * self.display.stride + self.cursor_left + x,
                        ),
                        self.cursor_backing[y * self.cursor_width + x],
                    );
                }
            }
        }
        self.cursor_saved = false;
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: restore_menu
    // DESC: Restores the exact desktop pixels saved beneath the active top-level menu.
    // ------------------=
    fn restore_menu(&mut self) {
        if !self.menu_saved {
            return;
        }
        for y in 0..self.menu_height {
            for x in 0..self.menu_width {
                unsafe {
                    write_volatile(
                        self.display.buffer.add(
                            (self.menu_top + y) * self.display.stride + self.menu_left + x,
                        ),
                        self.menu_backing[y * self.menu_width + x],
                    );
                }
            }
        }
        self.menu_saved = false;
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: save_menu
    // DESC: Captures the bounded desktop region underneath one translucent top-level menu.
    // ------------------=
    fn save_menu(&mut self, menu_kind: usize) {
        let layout = crate::ui::system_layout::SystemLayout::new(
            self.display.width,
            self.display.height,
        );
        let (left, top, width, height, _) = layout.system_menu_geometry(menu_kind);
        let width = width.min(600);
        let height = height.min(800);
        for y in 0..height {
            for x in 0..width {
                unsafe {
                    self.menu_backing[y * width + x] = read_volatile(
                        self.display.buffer.add((top + y) * self.display.stride + left + x),
                    );
                }
            }
        }
        self.menu_left = left;
        self.menu_top = top;
        self.menu_width = width;
        self.menu_height = height;
        self.menu_saved = true;
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: present_desktop_menu
    // DESC: Opens, updates, switches, or closes a menu using only its saved damage region.
    // ------------------=
    fn present_desktop_menu(
        &mut self,
        previous_screen: u8,
        previous_menu: usize,
        screen: u8,
        menu_kind: usize,
        focus: usize,
        clock: crate::storage::DateTimeConfiguration,
    ) {
        if previous_screen == 3 {
            self.restore_menu();
        }
        if screen == 3 {
            self.save_menu(menu_kind);
            let scale = self.display.ui_scale().max(1);
            self.display.system_menu_panel(menu_kind, focus, scale);
        }
        if previous_screen != screen || previous_menu != menu_kind {
            self.display.system_top_bar((screen == 3).then_some(menu_kind), clock);
        }
    }

    // ------------------------=
    // FUNC: save_and_draw_cursor
    // DESC: Saves pixels beneath the cursor and then draws the cursor artwork.
    // ------------------=
    fn save_and_draw_cursor(&mut self, cursor_x: i32, cursor_y: i32) {
        let left = (self.display.width as i32 * cursor_x / 1000).max(0) as usize;
        let top = (self.display.height as i32 * cursor_y / 1000).max(0) as usize;
        let size = (28 * self.display.ui_scale()).min(128);
        let width = size.min(self.display.width.saturating_sub(left));
        let height = size.min(self.display.height.saturating_sub(top));
        for y in 0..height {
            for x in 0..width {
                self.cursor_backing[y * width + x] = unsafe {
                    read_volatile(
                        self.display
                            .buffer
                            .add((top + y) * self.display.stride + left + x),
                    )
                };
            }
        }
        self.cursor_left = left;
        self.cursor_top = top;
        self.cursor_width = width;
        self.cursor_height = height;
        self.cursor_saved = width != 0 && height != 0;
        self.display.pointer_cursor(cursor_x, cursor_y);
    }

    // ------------------------=
    // FUNC: prepare_layout
    // DESC: Recomputes scaled geometry when the screen or display mode changes.
    // ------------------=
    fn prepare_layout(&mut self, split: bool, installer: bool) {
        // The startup BBS intentionally masks the lower portion of the large
        // bootstrap emblem. Once the user enters Console or Repair, replace
        // that composition with the dedicated console scene and its bounded
        // emblem before drawing the shorter terminal panel. Reusing the boot
        // emblem here exposed its masked edge and produced the chopped image.
        if self.bootstrap_scene && !installer && !split && self.split_layout {
            self.particles_initialized = false;
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            {
                self.display.paint_console_background();
                self.display.finish_console_emblem();
            }
            self.bootstrap_scene = false;
            self.layout_initialized = false;
        }
        if self.installer_scene != installer {
            self.particles_initialized = false;
            #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
            if installer {
                self.display.paint_installer_background();
                self.display.paint_installer_masthead();
                self.bootstrap_scene = false;
            } else {
                self.display.paint_console_background();
                self.display.finish_console_emblem();
            }
            #[cfg(target_arch = "x86")]
            self.display
                .fill_rect(0, 0, self.display.width, self.display.height, 0, 0, 0);
            self.installer_scene = installer;
            self.layout_initialized = false;
        }
        if self.layout_initialized && self.split_layout == split {
            return;
        }
        let left = self.display.width * 3 / 100;
        // Invalidate the complete union of the startup BBS and installer
        // surfaces.  The startup panel begins higher and ends lower than the
        // ordinary console; a smaller rectangle leaves its frame behind when
        // changing modes.
        let top = self.display.height * 57 / 100;
        let width = self.display.width * 95 / 100;
        let height = self.display.height * 42 / 100;
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        if installer {
            self.display
                .paint_installer_background_rect(left, top, width, height);
        } else if self.bootstrap_scene {
            self.display.paint_background_rect(left, top, width, height);
        } else {
            self.display
                .paint_console_background_rect(left, top, width, height);
        }
        #[cfg(target_arch = "x86")]
        self.display.fill_rect(left, top, width, height, 0, 0, 0);
        self.split_layout = split;
        self.layout_initialized = true;
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: animate
    // DESC: Advances and redraws one frame of console particle animation.
    // ------------------=
    fn animate(&mut self) {
        if self.installer_scene || self.system_ui_active {
            return;
        }
        let scale = self.display.ui_scale() as i32;
        // The startup BBS begins above the ordinary Console panel. Animation
        // restoration must use the active panel edge, otherwise it restores
        // emblem/background pixels over the opaque BBS and creates the dark
        // scallops visible along its top border.
        let panel_clip = if self.split_layout {
            self.display.height * 58 / 100
        } else {
            self.display.height * 68 / 100
        } as i32;
        let animation_clip = panel_clip - (self.display.height * 2 / 100) as i32;
        // Three comet-like streams share the emblem's exact 384-step path.
        // Each head pulls a tapered five-particle wake through both loops and
        // the center crossover; this continues independently of boot progress.
        if self.particles_initialized {
            for stream in 0..3usize {
                for trail in 0..6usize {
                    let old_phase = (self.particle_phase + stream * 128 + 384 - trail * 5) % 384;
                    let (x, y) = self
                        .display
                        .infinity_screen_position(old_phase, self.bootstrap_scene);
                    let radius = ((7 - trail as i32) * scale / 2).max(scale);
                    let extent = if trail == 0 { radius * 3 } else { radius * 2 };
                    if y + extent + 2 < animation_clip {
                        self.display
                            .restore_emblem_particle(x, y, extent, self.bootstrap_scene);
                    }
                }
            }
        }
        self.particle_phase = (self.particle_phase + 2) % 384;
        for stream in 0..3usize {
            for trail in 0..6usize {
                let phase = (self.particle_phase + stream * 128 + 384 - trail * 5) % 384;
                let (x, y) = self
                    .display
                    .infinity_screen_position(phase, self.bootstrap_scene);
                let radius = ((7 - trail as i32) * scale / 2).max(scale);
                let intensity = (230usize.saturating_sub(trail * 31)) as u8;
                let extent = if trail == 0 { radius * 3 } else { radius * 2 };
                if y + extent + 2 < animation_clip {
                    self.display.star_orb(x, y, radius, intensity, trail == 0);
                }
            }
        }
        self.particles_initialized = true;
    }

    #[cfg(target_arch = "x86")]
    // ------------------------=
    // FUNC: animate
    // DESC: Provides a no-op animation implementation when graphics are unavailable.
    // ------------------=
    fn animate(&mut self) {}
}

// ------------------------=
// FUNC: console_present
// DESC: Presents console or installer state while minimizing damaged-region redraws.
// ------------------=
pub fn console_present(
    lines: &[[u8; 96]; 6],
    lengths: &[usize; 6],
    line_count: usize,
    prompt: &[u8],
    command: &[u8],
    split_menu: bool,
    installer_screen: u8,
    installer_focus: usize,
    installer_choice: usize,
    installer_date_time: crate::storage::DateTimeConfiguration,
    installer_date_time_part: usize,
    installer_has_primary: bool,
    storage_device: Option<crate::storage::StorageDevice>,
    cursor_x: i32,
    cursor_y: i32,
    pointer_pressed: bool,
) {
    unsafe {
        let slot = &raw mut CONSOLE;
        if let Some(console) = (*slot).as_mut() {
            console.system_ui_active = false;
            let full_redraw = !console.layout_initialized
                || console.split_layout != split_menu
                || console.installer_scene != (installer_screen != 0);
            let screen_changed = console.last_installer_screen != installer_screen;
            let focus_changed = console.last_installer_focus != installer_focus;
            let choice_changed = console.last_installer_choice != installer_choice;
            let date_time_changed = console.last_installer_date_time != installer_date_time;
            let date_time_part_changed =
                console.last_installer_date_time_part != installer_date_time_part;
            let old_cursor_y = console.cursor_y;
            let pressed_changed = console.pointer_pressed != pointer_pressed;
            let pointer_changed =
                console.cursor_x != cursor_x || console.cursor_y != cursor_y || pressed_changed;
            let startup_row = |y: i32| -> u8 {
                if (710..760).contains(&y) {
                    0
                } else if (760..810).contains(&y) {
                    1
                } else if (810..860).contains(&y) {
                    2
                } else {
                    3
                }
            };
            let startup_row_changed = startup_row(old_cursor_y) != startup_row(cursor_y);
            // Restore only the pixels under the old cursor. Repainting the
            // full photographic background for each key or HID report caused
            // the visible flash seen in VirtualBox.
            console.restore_cursor();
            console.cursor_x = cursor_x;
            console.cursor_y = cursor_y;
            console.pointer_pressed = pointer_pressed;
            console.prepare_layout(split_menu, installer_screen != 0);
            if installer_screen != 0 {
                let content_redraw = full_redraw
                    || screen_changed
                    || (installer_screen == 5
                        && (choice_changed
                            || date_time_changed
                            || date_time_part_changed
                            || (focus_changed && !pointer_changed)));
                if content_redraw {
                    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
                    console.display.restore_installer_panel(installer_screen);
                    console.display.installer_panel(
                        lines,
                        lengths,
                        line_count,
                        prompt,
                        command,
                        installer_screen,
                        storage_device,
                        installer_date_time,
                        installer_date_time_part,
                        installer_focus,
                        cursor_x,
                        cursor_y,
                        pointer_pressed,
                    );
                }
                if installer_screen == 8 && content_redraw {
                    console.installer_progress = 0;
                    console.installer_animation_phase = 0;
                    console
                        .display
                        .installer_progress_frame(0, b"PREPARING INSTALLATION", 0);
                }
                if installer_screen == 7 && (content_redraw || focus_changed || pressed_changed) {
                    console.display.installer_confirmation_popup(
                        installer_focus,
                        cursor_x,
                        cursor_y,
                        pointer_pressed,
                    );
                }
                if full_redraw || screen_changed || focus_changed || pressed_changed {
                    console.display.installer_navigation(
                        installer_screen,
                        installer_focus,
                        installer_has_primary,
                        cursor_x,
                        cursor_y,
                        pointer_pressed,
                        full_redraw || screen_changed,
                    );
                }
            } else if split_menu {
                // Text edits still repaint the BBS, but ordinary mouse motion
                // only restores and redraws the small cursor sprite. Repaint
                // option rows when their hover state actually changes.
                if full_redraw || !pointer_changed {
                    console
                        .display
                        .startup_bbs(command, cursor_x, cursor_y, pointer_pressed);
                } else if startup_row_changed || pressed_changed {
                    console
                        .display
                        .startup_option_rows(cursor_x, cursor_y, pointer_pressed);
                }
            } else {
                console
                    .display
                    .terminal_box(lines, lengths, line_count, prompt, command, false);
            }
            if installer_screen != 0 || split_menu {
                console.save_and_draw_cursor(cursor_x, cursor_y);
            }
            console.last_installer_screen = installer_screen;
            console.last_installer_focus = installer_focus;
            console.last_installer_choice = installer_choice;
            console.last_installer_date_time = installer_date_time;
            console.last_installer_date_time_part = installer_date_time_part;
        }
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
            let bounded_menu_change = crate::ui::redraw::desktop_menu_change_requires_bounded_redraw(
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
            let window_moved = console.last_home_window_x != window_x
                || console.last_home_window_y != window_y;
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
                && screen == 2
                && console.last_system_screen == 2
                && window_visible
                && !window_maximized
            {
                console
                    .display
                    .move_desktop_window(
                        console.last_home_window_x,
                        console.last_home_window_y,
                        window_x,
                        window_y,
                        window_width,
                        window_height,
                    );
            } else if structural_change_without_window || window_move_requires_structural_redraw || window_resized {
                console.menu_saved = false;
                console
                    .display
                    .system_ui_frame(screen, step, input, masked, focus, validation_error, window_x, window_y, window_width, window_height, window_visible, window_maximized, home_location, selected_item, dragging_item, note_location, desktop_items, desktop_item_positions, clock, settings_maximized, menu_kind);
                full_surface_redrawn = true;
            } else if crate::ui::redraw::onboarding_controls_require_repaint(
                screen,
                pointer_changed,
                focus_changed,
            ) {
                console
                    .display
                    .onboarding_focus_controls(step, input, masked, focus);
            } else if content_changed
                && (matches!(screen, 5 | 6) || (screen == 1 && (1..=4).contains(&step)))
            {
                console
                    .display
                    .system_ui_input_field(screen, step, input, masked);
            } else if content_changed {
                console.menu_saved = false;
                console
                    .display
                    .system_ui_frame(screen, step, input, masked, focus, validation_error, window_x, window_y, window_width, window_height, window_visible, window_maximized, home_location, selected_item, dragging_item, note_location, desktop_items, desktop_item_positions, clock, settings_maximized, menu_kind);
                full_surface_redrawn = true;
            }
            if !full_surface_redrawn
                && crate::ui::redraw::desktop_clock_requires_bounded_redraw(
                    screen,
                    clock_changed,
                )
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

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: installer_progress_update
// DESC: Animates the graphical installer from its last verified checkpoint to the next one.
// ------------------=
pub fn installer_progress_update(percent: u8, label: &[u8]) {
    unsafe {
        let slot = &raw mut CONSOLE;
        if let Some(console) = (*slot).as_mut() {
            console.restore_cursor();
            let target = (percent as usize).min(100);
            let start = console.installer_progress.min(target);
            for value in start..=target {
                console.installer_animation_phase = (console.installer_animation_phase + 5) % 384;
                console.display.installer_progress_frame(
                    value,
                    label,
                    console.installer_animation_phase,
                );
                wait_frame(12);
            }
            console.installer_progress = target;
            console.save_and_draw_cursor(console.cursor_x, console.cursor_y);
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
                    wait_frame(33);
                }
            }
            console
                .display
                .installer_countdown_frame(0, 30, console.installer_animation_phase);
            wait_frame(220);
        }
    }
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: installer_reboot_countdown
// DESC: Provides an immediate text-mode transition on architectures without graphical countdown support.
// ------------------=
pub fn installer_reboot_countdown() {}

// ------------------------=
// FUNC: animation_tick
// DESC: Advances the active console animation on an input-driver timer tick.
// ------------------=
pub fn animation_tick() {
    if !animation_due() {
        return;
    }
    unsafe {
        SYSTEM_CLOCK_TICKS = SYSTEM_CLOCK_TICKS.saturating_add(1);
        if SYSTEM_CLOCK_TICKS >= 60 {
            SYSTEM_CLOCK_TICKS = 0;
            crate::console::clock_tick();
        }
    }
    // Keep the full-quality 60 Hz particle path while idle. A pointer update
    // owns its current presentation deadline completely; effects resume at the
    // first idle deadline (normally within 16 ms). On the software-rendered ARM
    // bootstrap this prevents any expensive glow frame from landing between a
    // HID sample and its cursor presentation. Installer and desktop paths are
    // unchanged.
    let defer_for_pointer = unsafe {
        let active = POINTER_ACTIVITY_PENDING;
        POINTER_ACTIVITY_PENDING = false;
        if POINTER_ACTIVITY_GRACE_TICKS != 0 {
            POINTER_ACTIVITY_GRACE_TICKS -= 1;
            true
        } else {
            active
        }
    };
    if defer_for_pointer {
        return;
    }
    unsafe {
        let slot = &raw mut CONSOLE;
        if let Some(console) = (*slot).as_mut() {
            if console.system_ui_active && matches!(console.last_system_screen, 5 | 6) {
                console.restore_cursor();
                console.particle_phase = (console.particle_phase + 2) % 384;
                console
                    .display
                    .system_login_animation(console.particle_phase);
                console.save_and_draw_cursor(console.cursor_x, console.cursor_y);
                return;
            }
            // Animation and cursor share the front buffer. Restore the cursor
            // before updating particles, then recapture the finished pixels;
            // otherwise cursor motion writes stale emblem pixels back and
            // produces flashing trails.
            let cursor_over_animation = !console.installer_scene
                && console.split_layout
                && (180..=820).contains(&console.cursor_x)
                && (180..=560).contains(&console.cursor_y);
            if cursor_over_animation {
                console.restore_cursor();
            }
            console.animate();
            if cursor_over_animation {
                console.save_and_draw_cursor(console.cursor_x, console.cursor_y);
            }
        }
    }
}

#[cfg(target_arch = "aarch64")]
// ------------------------=
// FUNC: animation_due
// DESC: Uses the x86 time-stamp counter to decide when the next frame is due.
// ------------------=
fn animation_due() -> bool {
    static mut NEXT: u64 = 0;
    let now: u64;
    let frequency: u64;
    unsafe {
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) now);
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) frequency);
        if now < NEXT {
            return false;
        }
        NEXT = now.saturating_add((frequency / 60).max(1));
    }
    true
}

#[cfg(target_arch = "x86_64")]
// ------------------------=
// FUNC: animation_due
// DESC: Uses the AArch64 virtual counter to decide when the next frame is due.
// ------------------=
fn animation_due() -> bool {
    static mut NEXT: u64 = 0;
    let now = unsafe { core::arch::x86_64::_rdtsc() };
    unsafe {
        if now < NEXT {
            return false;
        }
        let leaf = core::arch::x86_64::__cpuid(0x16).eax as u64;
        let frequency = leaf.max(1000) * 1_000_000;
        NEXT = now.saturating_add(frequency / 60);
    }
    true
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: animation_due
// DESC: Provides a conservative frame cadence on targets without a hardware timer path.
// ------------------=
fn animation_due() -> bool {
    false
}

#[cfg(target_arch = "aarch64")]
// ------------------------=
// FUNC: wait_frame
// DESC: Uses the AArch64 architectural counter for stable animation timing independent of CPU speed.
// ------------------=
fn wait_frame(milliseconds: u64) {
    let start: u64;
    let frequency: u64;
    unsafe {
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) start);
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) frequency);
    }
    let target = start.saturating_add(frequency.saturating_mul(milliseconds) / 1000);
    loop {
        let now: u64;
        unsafe {
            core::arch::asm!("mrs {}, cntvct_el0", out(reg) now);
        }
        if now >= target {
            break;
        }
        core::hint::spin_loop();
    }
}

#[cfg(target_arch = "x86_64")]
// ------------------------=
// FUNC: wait_frame
// DESC: Uses the x86 time-stamp counter for stable animation timing independent of CPU speed.
// ------------------=
fn wait_frame(milliseconds: u64) {
    let megahertz = core::arch::x86_64::__cpuid(0x16).eax as u64;
    let megahertz = megahertz.max(1000);
    let start = unsafe { core::arch::x86_64::_rdtsc() };
    let target = start.saturating_add(megahertz.saturating_mul(1000).saturating_mul(milliseconds));
    while unsafe { core::arch::x86_64::_rdtsc() } < target {
        core::hint::spin_loop();
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: show_splash
// DESC: Runs the graphical boot splash and hands the framebuffer to the console.
// ------------------=
pub fn show_splash(info: &BootInfo) -> bool {
    let Some(mut display) = DisplayDevice::from_boot_info(info) else {
        return false;
    };
    display.paint_background();
    let stages: [(usize, &[u8]); 4] = [
        (25, b"GRAPHICS SURFACE READY"),
        (50, b"KEYBOARD DRIVER PROBE"),
        (75, b"MOUSE DRIVER PROBE"),
        (100, b"INFINITYOS READY"),
    ];
    for (stage_index, (stage, label)) in stages.into_iter().enumerate() {
        for frame in 0..30 {
            let sequence = stage_index * 30 + frame;
            display.emblem_reveal_band(sequence, 120);
            display.infinity_pulse(sequence * 3);
            display.progress(stage.saturating_sub(24) + frame * 24 / 30, label);
            wait_frame(35);
        }
    }
    // Keep the completed bootstrap composition intact for the startup menu.
    // The BBS panel owns its lower rectangle, while particle restoration above
    // it now reads from the same boot artwork and full-size emblem geometry.
    display.finish_emblem();
    activate_console(display);
    true
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: show_splash
// DESC: Reports that no graphical splash is available on unsupported targets.
// ------------------=
pub fn show_splash(_info: &BootInfo) -> bool {
    false
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: le16
// DESC: Decodes a little-endian 16-bit integer from bitmap bytes.
// ------------------=
fn le16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
// ------------------------=
// FUNC: le32
// DESC: Decodes a little-endian 32-bit integer from bitmap bytes.
// ------------------=
fn le32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

// ------------------------=
// FUNC: glyph
// DESC: Returns the eight-row bitmap glyph used by the built-in ASCII font.
// ------------------=
fn glyph(c: u8) -> [u8; 8] {
    match c {
        b'A' => [0x18, 0x24, 0x42, 0x7e, 0x42, 0x42, 0x42, 0],
        b'B' => [0x7c, 0x42, 0x42, 0x7c, 0x42, 0x42, 0x7c, 0],
        b'C' => [0x3c, 0x42, 0x40, 0x40, 0x40, 0x42, 0x3c, 0],
        b'D' => [0x78, 0x44, 0x42, 0x42, 0x42, 0x44, 0x78, 0],
        b'E' => [0x7e, 0x40, 0x40, 0x7c, 0x40, 0x40, 0x7e, 0],
        b'F' => [0x7e, 0x40, 0x40, 0x7c, 0x40, 0x40, 0x40, 0],
        b'G' => [0x3c, 0x42, 0x40, 0x4e, 0x42, 0x42, 0x3c, 0],
        b'H' => [0x42, 0x42, 0x42, 0x7e, 0x42, 0x42, 0x42, 0],
        b'I' => [0x7e, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7e, 0],
        b'J' => [0x0e, 0x04, 0x04, 0x04, 0x44, 0x44, 0x38, 0],
        b'K' => [0x42, 0x44, 0x48, 0x70, 0x48, 0x44, 0x42, 0],
        b'L' => [0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x7e, 0],
        b'M' => [0x42, 0x66, 0x5a, 0x5a, 0x42, 0x42, 0x42, 0],
        b'N' => [0x42, 0x62, 0x52, 0x4a, 0x46, 0x42, 0x42, 0],
        b'O' => [0x3c, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3c, 0],
        b'P' => [0x7c, 0x42, 0x42, 0x7c, 0x40, 0x40, 0x40, 0],
        b'Q' => [0x3c, 0x42, 0x42, 0x42, 0x4a, 0x44, 0x3a, 0],
        b'R' => [0x7c, 0x42, 0x42, 0x7c, 0x48, 0x44, 0x42, 0],
        b'S' => [0x3c, 0x42, 0x40, 0x3c, 0x02, 0x42, 0x3c, 0],
        b'T' => [0x7e, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0],
        b'U' => [0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3c, 0],
        b'V' => [0x42, 0x42, 0x42, 0x42, 0x24, 0x24, 0x18, 0],
        b'W' => [0x42, 0x42, 0x42, 0x5a, 0x5a, 0x66, 0x42, 0],
        b'X' => [0x42, 0x24, 0x18, 0x18, 0x18, 0x24, 0x42, 0],
        b'Y' => [0x42, 0x24, 0x18, 0x18, 0x18, 0x18, 0x18, 0],
        b'Z' => [0x7e, 0x04, 0x08, 0x10, 0x20, 0x40, 0x7e, 0],
        b'0' => [0x3c, 0x42, 0x46, 0x4a, 0x52, 0x62, 0x3c, 0],
        b'1' => [0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x7e, 0],
        b'2' => [0x3c, 0x42, 0x02, 0x0c, 0x30, 0x40, 0x7e, 0],
        b'3' => [0x7c, 0x02, 0x02, 0x3c, 0x02, 0x02, 0x7c, 0],
        b'4' => [0x0c, 0x14, 0x24, 0x44, 0x7e, 0x04, 0x04, 0],
        b'5' => [0x7e, 0x40, 0x40, 0x7c, 0x02, 0x02, 0x7c, 0],
        b'6' => [0x3c, 0x40, 0x40, 0x7c, 0x42, 0x42, 0x3c, 0],
        b'7' => [0x7e, 0x02, 0x04, 0x08, 0x10, 0x10, 0x10, 0],
        b'8' => [0x3c, 0x42, 0x42, 0x3c, 0x42, 0x42, 0x3c, 0],
        b'9' => [0x3c, 0x42, 0x42, 0x3e, 0x02, 0x02, 0x3c, 0],
        b' ' => [0; 8],
        b'.' => [0, 0, 0, 0, 0, 0x18, 0x18, 0],
        b'-' => [0, 0, 0, 0x7e, 0, 0, 0, 0],
        b'=' => [0, 0, 0x7e, 0, 0x7e, 0, 0, 0],
        b'+' => [0, 0x18, 0x18, 0x7e, 0x18, 0x18, 0, 0],
        b'>' => [0x40, 0x20, 0x10, 0x08, 0x10, 0x20, 0x40, 0],
        b'_' => [0, 0, 0, 0, 0, 0, 0x7e, 0],
        b'!' => [0x18, 0x18, 0x18, 0x18, 0x18, 0, 0x18, 0],
        b':' => [0, 0x18, 0x18, 0, 0x18, 0x18, 0, 0],
        b'[' => [0x3c, 0x20, 0x20, 0x20, 0x20, 0x20, 0x3c, 0],
        b']' => [0x3c, 0x04, 0x04, 0x04, 0x04, 0x04, 0x3c, 0],
        b'/' => [0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0, 0],
        b'\\' => [0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0, 0],
        b'|' => [0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0],
        b'<' => [0x04, 0x08, 0x10, 0x20, 0x10, 0x08, 0x04, 0],
        b',' => [0, 0, 0, 0, 0, 0x18, 0x18, 0x10],
        b'\'' => [0x18, 0x18, 0x10, 0, 0, 0, 0, 0],
        b'?' => [0x3c, 0x42, 0x02, 0x0c, 0x10, 0, 0x10, 0],
        b'(' => [0x0c, 0x10, 0x20, 0x20, 0x20, 0x10, 0x0c, 0],
        b')' => [0x30, 0x08, 0x04, 0x04, 0x04, 0x08, 0x30, 0],
        b'#' => [0x24, 0x24, 0x7e, 0x24, 0x7e, 0x24, 0x24, 0],
        b'*' => [0, 0x42, 0x24, 0x7e, 0x24, 0x42, 0, 0],
        b'a'..=b'z' => glyph(c - 32),
        _ => [0x7e, 0x42, 0x5a, 0x5a, 0x42, 0x42, 0x7e, 0],
    }
}
