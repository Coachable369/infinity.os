//! Framebuffer-backed bootstrap and installer presentation.
//!
//! This module owns the early graphical surface from the first splash frame
//! through the startup console and installation workflow. Hardware discovery
//! supplies the framebuffer; this module handles bitmap scaling, text, focus,
//! pointer composition, animation timing, and damage-limited presentation.

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

    // ------------------------=
    // FUNC: authentication_frame
    // DESC: Renders the default skin authentication scene to the supplied gold-standard geometry and copy.
    // ------------------=
    fn authentication_frame(&mut self, locked: bool, selected_user: usize, input: &[u8], focus: usize) {
        let fit = (self.width.saturating_mul(1000) / 1536).min(self.height.saturating_mul(1000) / 1024).max(1);
        let content_width = 1536usize.saturating_mul(fit) / 1000;
        let content_height = 1024usize.saturating_mul(fit) / 1000;
        let offset_x = self.width.saturating_sub(content_width) / 2;
        let offset_y = self.height.saturating_sub(content_height) / 2;
        let sx = |value: usize| offset_x + value.saturating_mul(fit) / 1000;
        let sy = |value: usize| offset_y + value.saturating_mul(fit) / 1000;
        let sw = |value: usize| value.saturating_mul(fit) / 1000;

        let top_height = sw(64).max(38);
        self.fill_rect_alpha(0, 0, self.width, top_height, 1, 6, 13, 238);
        self.outline_rect(0, top_height.saturating_sub(1), self.width, 1, 15, 31, 48);
        self.small_infinity_mark(sx(57), top_height / 2, sw(48));
        self.ui_text(sx(92), top_height / 2 - UI_FONT_CELL_HEIGHT / 2, b"I N F I N I T Y O S", 239, 244, 249, 1);
        self.ui_text(sx(1302), top_height / 2 - UI_FONT_CELL_HEIGHT / 2, b"LOCAL   |   PRIVATE", 229, 235, 241, 1);
        self.authentication_icon(sx(1497), top_height / 2, 1, sw(21), false);

        let card_x = sx(54);
        let card_y = sy(123);
        let card_w = sw(521);
        let card_h = sw(754);
        self.glass_panel(card_x, card_y, card_w, card_h, true);
        self.small_infinity_mark(card_x + card_w / 2, card_y + sw(91), sw(122));
        self.ui_text_centered(card_x, card_w, card_y + sw(148), if locked { b"Welcome Back" } else { b"Welcome to InfinityOS" }, 244, 247, 251, 2);
        self.ui_text_centered(card_x, card_w, card_y + sw(198), b"Secure. Private. Limitless.", 188, 198, 211, 1);

        let inner_x = card_x + sw(49);
        let inner_w = card_w.saturating_sub(sw(98));
        let user_y = card_y + sw(253);
        self.fill_rounded_rect_alpha(inner_x, user_y, inner_w, sw(86), sw(13), 2, 10, 20, if focus == 0 { 248 } else { 224 });
        self.outline_rounded_rect(inner_x, user_y, inner_w, sw(86), sw(13), if focus == 0 { 37 } else { 44 }, if focus == 0 { 183 } else { 66 }, if focus == 0 { 234 } else { 89 });
        self.authentication_icon(inner_x + sw(45), user_y + sw(43), 2, sw(50), focus == 0);
        let user = crate::runtime::with_runtime(|runtime| runtime.identity.user_nth(selected_user)).flatten();
        if let Some(value) = user {
            self.ui_text(inner_x + sw(91), user_y + sw(25), value.display_name.as_bytes(), 237, 242, 248, 1);
        } else {
            self.ui_text(inner_x + sw(91), user_y + sw(25), b"Aurelius", 237, 242, 248, 1);
        }
        self.ui_text(inner_x + sw(91), user_y + sw(53), b"Last signed in 2 minutes ago", 151, 162, 177, 1);
        self.authentication_icon(inner_x + inner_w - sw(28), user_y + sw(43), 3, sw(15), focus == 0);

        let password_y = card_y + sw(356);
        self.authentication_password_field(inner_x, password_y, inner_w, sw(57), input, focus == 1);

        let sign_y = card_y + sw(439);
        self.fill_rounded_rect_alpha(inner_x, sign_y, inner_w, sw(56), sw(13), 7, 48, 79, if focus == 2 { 248 } else { 226 });
        self.outline_rounded_rect(inner_x, sign_y, inner_w, sw(56), sw(13), 34, 182, 235);
        self.ui_text_centered(inner_x, inner_w, sign_y + sw(18), if locked { b"Unlock" } else { b"Sign In" }, 247, 250, 253, 1);
        self.authentication_icon(inner_x + inner_w - sw(30), sign_y + sw(28), 4, sw(18), focus == 2);

        let separator_y = card_y + sw(527);
        self.fill_rect(inner_x, separator_y, inner_w, 1, 18, 31, 45);
        self.fill_rect_alpha(inner_x + inner_w / 2 - sw(23), separator_y - sw(10), sw(46), sw(20), 4, 12, 22, 255);
        self.ui_text_centered(inner_x + inner_w / 2 - sw(23), sw(46), separator_y - sw(8), b"or", 157, 168, 183, 1);

        let key_y = card_y + sw(557);
        self.fill_rounded_rect_alpha(inner_x, key_y, inner_w, sw(55), sw(13), 5, 16, 28, if focus == 3 { 248 } else { 224 });
        self.outline_rounded_rect(inner_x, key_y, inner_w, sw(55), sw(13), if focus == 3 { 34 } else { 37 }, if focus == 3 { 182 } else { 59 }, if focus == 3 { 235 } else { 79 });
        self.ui_text_centered(inner_x, inner_w, key_y + sw(17), b"Sign In with Security Key", 235, 240, 246, 1);
        self.authentication_icon(inner_x + inner_w - sw(31), key_y + sw(27), 5, sw(17), focus == 3);

        let actions_y = card_y + sw(661);
        let third = inner_w / 3;
        let action_labels: [&[u8]; 3] = [b"Add User", b"Forgot Password?", b"Options"];
        for (index, label) in action_labels.iter().enumerate() {
            let center = inner_x + third * index + third / 2;
            self.authentication_icon(center, actions_y, 6 + index, sw(27), focus == index + 4);
            self.ui_text_centered(inner_x + third * index, third, actions_y + sw(31), label, if focus == index + 4 { 255 } else { 209 }, if focus == index + 4 { 255 } else { 217 }, if focus == index + 4 { 255 } else { 226 }, 1);
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
            self.authentication_icon(center, tray_y + sw(31), 9 + index, sw(25), focus == index + 7);
            self.ui_text_centered(tray_x + cell * index, cell, tray_y + sw(57), label, if focus == index + 7 { 255 } else { 221 }, if focus == index + 7 { 255 } else { 228 }, if focus == index + 7 { 255 } else { 236 }, 1);
        }
    }

    // ------------------------=
    // FUNC: authentication_password_field
    // DESC: Draws the secure password control without retaining or displaying plaintext pixels.
    // ------------------=
    fn authentication_password_field(&mut self, x: usize, y: usize, width: usize, height: usize, input: &[u8], focused: bool) {
        self.fill_rounded_rect_alpha(x, y, width, height, height / 4, 2, 9, 18, 255);
        self.outline_rounded_rect(x, y, width, height, height / 4, if focused { 34 } else { 37 }, if focused { 182 } else { 58 }, if focused { 235 } else { 78 });
        self.authentication_icon(x + height / 2, y + height / 2, 5, height / 3, focused);
        if input.is_empty() {
            self.ui_text(x + height, y + height / 2 - 8, b"Enter your password", 115, 126, 142, 1);
        } else {
            let mut masked = [0u8; 64];
            let length = input.len().min(masked.len());
            masked[..length].fill(b'*');
            self.ui_text(x + height, y + height / 2 - 8, &masked[..length], 238, 244, 249, 1);
        }
        self.authentication_icon(x + width - height / 2, y + height / 2, 13, height / 3, focused);
    }

    // ------------------------=
    // FUNC: authentication_icon
    // DESC: Rasterizes the default skin's semantic authentication icons from scalable primitives.
    // ------------------=
    fn authentication_icon(&mut self, center_x: usize, center_y: usize, kind: usize, size: usize, active: bool) {
        let color = if active { (139, 225, 255) } else { (226, 235, 243) };
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
                    stroke!(cx - span, cy - rise + level * half / 2, cx, cy + level * half / 3, color.0, color.1, color.2);
                    stroke!(cx, cy + level * half / 3, cx + span, cy - rise + level * half / 2, color.0, color.1, color.2);
                }
                self.icon_circle(cx, cy + half * 3 / 4, (half / 7).max(1), color, size);
            }
            2 => {
                self.icon_circle(cx, cy, half, (24, 178, 232), size);
                self.star_orb(cx, cy - half / 4, half / 3, 255, true);
                self.fill_rounded_rect_alpha((cx - half * 2 / 3).max(0) as usize, cy as usize, (half * 4 / 3) as usize, (half * 2 / 3) as usize, (half / 3).max(1) as usize, 73, 198, 241, 240);
            }
            3 => {
                stroke!(cx - half, cy - half / 3, cx, cy + half / 3, color.0, color.1, color.2);
                stroke!(cx, cy + half / 3, cx + half, cy - half / 3, color.0, color.1, color.2);
            }
            4 => {
                stroke!(cx - half, cy, cx + half, cy, color.0, color.1, color.2);
                stroke!(cx + half / 3, cy - half / 2, cx + half, cy, color.0, color.1, color.2);
                stroke!(cx + half / 3, cy + half / 2, cx + half, cy, color.0, color.1, color.2);
            }
            5 => {
                self.outline_rounded_rect((cx - half * 2 / 3).max(0) as usize, (cy - half / 6).max(0) as usize, (half * 4 / 3) as usize, half as usize, (half / 6).max(1) as usize, color.0, color.1, color.2);
                self.icon_circle(cx, cy - half / 4, half / 2, color, size);
            }
            6 => {
                self.icon_circle(cx - half / 3, cy - half / 3, half / 3, color, size);
                stroke!(cx - half, cy + half, cx - half / 3, cy, color.0, color.1, color.2);
                stroke!(cx - half / 3, cy, cx + half / 3, cy + half, color.0, color.1, color.2);
                stroke!(cx + half / 2, cy - half / 4, cx + half / 2, cy + half / 2, color.0, color.1, color.2);
                stroke!(cx + half / 6, cy + half / 8, cx + half * 5 / 6, cy + half / 8, color.0, color.1, color.2);
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
                    stroke!(cx + dx * 2 / 3, cy + dy * 2 / 3, cx + dx, cy + dy, color.0, color.1, color.2);
                }
            }
            9 => {
                self.icon_circle(cx, cy + half / 8, half * 3 / 4, color, size);
                stroke!(cx, cy - half, cx, cy + half / 6, color.0, color.1, color.2);
            }
            10 => {
                self.icon_circle(cx, cy, half * 3 / 4, color, size);
                stroke!(cx - half, cy - half / 4, cx - half / 4, cy - half / 2, color.0, color.1, color.2);
                stroke!(cx - half, cy - half / 4, cx - half * 3 / 4, cy - half, color.0, color.1, color.2);
            }
            11 => {
                stroke!(cx - half, cy, cx, cy - half / 2, color.0, color.1, color.2);
                stroke!(cx, cy - half / 2, cx + half, cy, color.0, color.1, color.2);
                stroke!(cx - half / 2, cy + half / 3, cx, cy, color.0, color.1, color.2);
                stroke!(cx, cy, cx + half / 2, cy + half / 3, color.0, color.1, color.2);
            }
            12 => {
                self.icon_circle(cx, cy - half * 2 / 3, half / 5, color, size);
                stroke!(cx, cy, cx, cy + half, color.0, color.1, color.2);
                stroke!(cx - half, cy, cx + half, cy, color.0, color.1, color.2);
                stroke!(cx, cy + half / 3, cx - half * 2 / 3, cy + half, color.0, color.1, color.2);
                stroke!(cx, cy + half / 3, cx + half * 2 / 3, cy + half, color.0, color.1, color.2);
            }
            13 => {
                stroke!(cx - half, cy, cx - half / 3, cy - half / 3, color.0, color.1, color.2);
                stroke!(cx - half / 3, cy - half / 3, cx + half / 3, cy - half / 3, color.0, color.1, color.2);
                stroke!(cx + half / 3, cy - half / 3, cx + half, cy, color.0, color.1, color.2);
                stroke!(cx + half, cy, cx + half / 3, cy + half / 3, color.0, color.1, color.2);
                stroke!(cx + half / 3, cy + half / 3, cx - half / 3, cy + half / 3, color.0, color.1, color.2);
                stroke!(cx - half / 3, cy + half / 3, cx - half, cy, color.0, color.1, color.2);
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
    fn system_top_bar(&mut self, active_menu: Option<usize>, clock: crate::storage::DateTimeConfiguration) -> usize {
        let scale = self.ui_scale().max(1);
        let height = (46 * scale).min(self.height / 12).max(40);
        self.fill_rect_alpha(0, 0, self.width, height, 0, 4, 10, 218);
        self.fill_rect_alpha(0, 0, self.width, height / 2, 14, 26, 39, 58);
        self.fill_rect_alpha(0, height.saturating_sub(1), self.width, 1, 50, 70, 87, 170);

        let brand_width = (150 * scale).min(self.width / 5);
        if active_menu == Some(0) {
            self.fill_rounded_rect_alpha(8 * scale, 5 * scale, brand_width, height.saturating_sub(10 * scale), 9 * scale, 15, 57, 83, 214);
        }
        self.small_infinity_mark(22 * scale, height / 2, 24 * scale);
        self.ui_text_strong(40 * scale, height / 2 - 10 * scale, b"InfinityOS", 247, 249, 252, 1);

        let menu_positions = [178usize, 238, 298, 360, 444];
        for (index, label) in [b"File".as_slice(), b"Edit", b"View", b"Window", b"Help"].iter().enumerate() {
            let menu_x = menu_positions[index] * scale;
            if active_menu == Some(index + 1) {
                let active_width = self.ui_text_width(label, 1) + 18 * scale;
                self.fill_rounded_rect_alpha(menu_x.saturating_sub(9 * scale), 5 * scale, active_width, height.saturating_sub(10 * scale), 8 * scale, 18, 55, 78, 210);
            }
            self.ui_text(menu_x, height / 2 - 10 * scale, label, 213, 222, 231, 1);
        }

        let icon_size = 18 * scale;
        let status_y = height / 2;
        let status_width = 32 * scale;
        let clock_width = 88 * scale;
        let status_left = self.width.saturating_sub(7 * status_width + clock_width + 10 * scale);
        let private_width = self.ui_text_width(b"LOCAL  |  PRIVATE", 1);
        self.ui_text(status_left.saturating_sub(private_width + 18 * scale), height / 2 - 10 * scale, b"LOCAL  |  PRIVATE", 211, 221, 231, 1);
        for (index, kind) in [0usize, 1, 2, 3, 13, 4, 5].iter().enumerate() {
            let center = status_left + index * status_width + status_width / 2;
            self.system_status_icon(center, status_y, *kind, icon_size);
        }
        let mut time = *b"00:00:00";
        time[0] = b'0' + clock.hour / 10;
        time[1] = b'0' + clock.hour % 10;
        time[3] = b'0' + clock.minute / 10;
        time[4] = b'0' + clock.minute % 10;
        time[6] = b'0' + clock.second / 10;
        time[7] = b'0' + clock.second % 10;
        let time_width = self.ui_text_width(&time, 1);
        self.ui_text_strong(self.width.saturating_sub(time_width + 16 * scale), height / 2 - 10 * scale, &time, 235, 242, 248, 1);
        height
    }

    // ------------------------=
    // FUNC: system_identity_bar
    // DESC: Draws the minimal trusted first-boot bar without presenting desktop commands before a session exists.
    // ------------------=
    fn system_identity_bar(&mut self) -> usize {
        let scale = self.ui_scale().max(1);
        let height = (46 * scale).min(self.height / 12).max(40);
        self.fill_rect_alpha(0, 0, self.width, height, 0, 4, 10, 232);
        self.fill_rect_alpha(0, 0, self.width, height / 2, 14, 26, 39, 54);
        self.fill_rect_alpha(0, height.saturating_sub(1), self.width, 1, 50, 70, 87, 170);
        self.small_infinity_mark(35 * scale, height / 2, 48 * scale);
        self.ui_text_strong(64 * scale, height / 2 - 10 * scale, b"INFINITYOS", 241, 246, 250, 1);
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
    fn system_menu_panel(&mut self, menu_kind: usize, focus: usize, scale: usize) {
        let (anchor, width, items): (usize, usize, &[&[u8]]) = match menu_kind {
            1 => (176, 248, &[b"Open Console", b"Open Personal Space", b"New Project...", b"System Settings...", b"Close Home Window"]),
            2 => (236, 230, &[b"Undo", b"Redo", b"Cut", b"Copy", b"Paste", b"Select All"]),
            3 => (296, 238, &[b"Show Home Window", b"Center Home Window", b"Icon View", b"Refresh", b"Appearance..."]),
            4 => (358, 242, &[b"Minimize Home", b"Restore Home", b"Center Window", b"System Settings..."]),
            5 => (442, 252, &[b"InfinityOS Help", b"Keyboard Shortcuts", b"System Status", b"About InfinityOS"]),
            _ => (16, 268, &[b"About InfinityOS", b"System Settings...", b"Users & Accounts...", b"AI & Voice...", b"Network...", b"Privacy & Security...", b"Lock Screen", b"Log Out...", b"Restart...", b"Shut Down..."]),
        };
        let menu_x = (anchor * scale).min(self.width.saturating_sub(width * scale + 8));
        let menu_y = (46 * scale).min(self.height / 12).max(40) + 6 * scale;
        let menu_w = (width * scale).min(self.width.saturating_sub(menu_x + 8));
        let menu_h = (22 + items.len() * 34) * scale;
        self.glass_panel(menu_x, menu_y, menu_w, menu_h, true);
        self.fill_rounded_rect_alpha(menu_x + 2 * scale, menu_y + 2 * scale, menu_w.saturating_sub(4 * scale), 20 * scale, 10 * scale, 18, 35, 50, 118);
        for (index, item) in items.iter().enumerate() {
            let row_y = menu_y + (11 + index * 34) * scale;
            if focus == index {
                self.fill_rounded_rect_alpha(menu_x + 7 * scale, row_y, menu_w.saturating_sub(14 * scale), 30 * scale, 7 * scale, 20, 87, 125, 224);
            }
            self.authentication_icon(menu_x + 23 * scale, row_y + 15 * scale, (index + menu_kind * 2) % 13 + 1, 15 * scale, focus == index);
            self.ui_text(menu_x + 43 * scale, row_y + 5 * scale, item, if focus == index { 249 } else { 218 }, if focus == index { 252 } else { 227 }, if focus == index { 255 } else { 236 }, 1);
        }
    }

    // ------------------------=
    // FUNC: system_status_icon
    // DESC: Draws the coherent small-size volume, radio, power, search, and menu vector family for the top bar.
    // ------------------=
    fn system_status_icon(&mut self, center_x: usize, center_y: usize, kind: usize, size: usize) {
        let cx = center_x as i32;
        let cy = center_y as i32;
        let half = size.max(10) as i32 / 2;
        let color = (230, 238, 245);
        match kind {
            0 => {
                self.icon_line(cx - half, cy - half / 3, cx - half / 2, cy - half / 3, color, size);
                self.icon_line(cx - half / 2, cy - half / 3, cx, cy - half * 3 / 4, color, size);
                self.icon_line(cx, cy - half * 3 / 4, cx, cy + half * 3 / 4, color, size);
                self.icon_line(cx, cy + half * 3 / 4, cx - half / 2, cy + half / 3, color, size);
                self.icon_line(cx - half / 2, cy + half / 3, cx - half, cy + half / 3, color, size);
                self.icon_line(cx + half / 3, cy - half / 2, cx + half * 2 / 3, cy, color, size);
                self.icon_line(cx + half * 2 / 3, cy, cx + half / 3, cy + half / 2, color, size);
            }
            1 => self.authentication_icon(center_x, center_y, 1, size, false),
            2 => {
                self.icon_line(cx, cy - half, cx, cy + half, color, size);
                self.icon_line(cx, cy - half, cx + half * 2 / 3, cy - half / 3, color, size);
                self.icon_line(cx + half * 2 / 3, cy - half / 3, cx - half / 2, cy + half * 2 / 3, color, size);
                self.icon_line(cx - half / 2, cy - half * 2 / 3, cx + half * 2 / 3, cy + half / 3, color, size);
                self.icon_line(cx + half * 2 / 3, cy + half / 3, cx, cy + half, color, size);
            }
            3 => {
                let left = center_x.saturating_sub(size / 2);
                let top = center_y.saturating_sub(size * 3 / 10);
                self.outline_rounded_rect(left, top, size.saturating_sub(3), size * 3 / 5, 2, color.0, color.1, color.2);
                self.fill_rect_alpha(left + size.saturating_sub(2), top + size / 5, 3, size / 5, 210, 224, 235, 255);
                self.fill_rounded_rect_alpha(left + 3, top + 3, size * 3 / 5, size * 3 / 5 - 6, 1, 216, 231, 241, 255);
            }
            4 => {
                self.icon_circle(cx - half / 5, cy - half / 5, half * 3 / 5, color, size);
                self.icon_line(cx + half / 4, cy + half / 4, cx + half, cy + half, color, size);
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
    fn onboarding_step_indicator(&mut self, left: usize, top: usize, width: usize, step: usize) {
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
    fn polished_button(&mut self, left: usize, top: usize, width: usize, height: usize, label: &[u8], primary: bool, focused: bool) {
        let radius = (height / 4).clamp(8, 14);
        if primary {
            self.fill_rounded_rect_alpha(left, top, width, height, radius, 8, 54, 84, if focused { 252 } else { 232 });
            self.fill_rounded_rect_alpha(left + 2, top + 2, width.saturating_sub(4), height / 2, radius.saturating_sub(2), 25, 111, 159, if focused { 116 } else { 72 });
            self.outline_rounded_rect(left, top, width, height, radius, if focused { 151 } else { 40 }, if focused { 229 } else { 181 }, if focused { 255 } else { 231 });
        } else {
            self.fill_rounded_rect_alpha(left, top, width, height, radius, 5, 15, 27, if focused { 246 } else { 218 });
            self.outline_rounded_rect(left, top, width, height, radius, if focused { 105 } else { 42 }, if focused { 197 } else { 69 }, if focused { 236 } else { 91 });
        }
        self.ui_text_centered_strong(left, width, top + height / 2 - 10, label, 242, 248, 252, 1);
        if primary {
            self.authentication_icon(left + width.saturating_sub(height / 2), top + height / 2, 4, height / 3, focused);
        }
    }

    // ------------------------=
    // FUNC: onboarding_input_field
    // DESC: Draws one polished first-boot text field with placeholder, focus, and secure masking.
    // ------------------=
    fn onboarding_input_field(&mut self, left: usize, top: usize, width: usize, height: usize, input: &[u8], masked: bool, focused: bool, placeholder: &[u8]) {
        let radius = (height / 4).clamp(8, 14);
        self.fill_rounded_rect_alpha(left, top, width, height, radius, 2, 10, 20, 245);
        self.outline_rounded_rect(left, top, width, height, radius, if focused { 67 } else { 40 }, if focused { 192 } else { 72 }, if focused { 241 } else { 95 });
        let mut shown = [0u8; 64];
        let shown_len = input.len().min(shown.len());
        if masked {
            shown[..shown_len].fill(b'*');
        } else {
            shown[..shown_len].copy_from_slice(&input[..shown_len]);
        }
        let display = if shown_len == 0 { placeholder } else { &shown[..shown_len] };
        let color = if shown_len == 0 { (119, 133, 149) } else { (239, 245, 250) };
        self.ui_text(left + 16, top + height / 2 - 10, display, color.0, color.1, color.2, 1);
    }

    // ------------------------=
    // FUNC: onboarding_frame
    // DESC: Renders the premium progressive first-boot card from shared controls and semantic copy.
    // ------------------=
    fn onboarding_frame(&mut self, step: usize, input: &[u8], masked: bool, focus: usize, validation_error: bool) {
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
        self.ui_text_strong(inner_left + 55 * scale, card_top + 24 * scale, b"INFINITYOS", 237, 244, 249, 1);
        self.ui_text(card_left + card_width.saturating_sub(102 * scale), card_top + 24 * scale, b"LOCAL  |  PRIVATE", 135, 155, 174, 1);
        self.onboarding_step_indicator(inner_left, card_top + 66 * scale, inner_width, step);

        let (eyebrow, title, description, placeholder): (&[u8], &[u8], &[u8], &[u8]) = match step {
            0 => (b"WELCOME", b"Welcome to InfinityOS", b"A private system shaped around you.", b""),
            1 => (b"MACHINE", b"Name this Infinity Node", b"Choose a friendly name for this device. You can change it later.", b"InfinityNode"),
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
                (b"Yours from the start", b"Identity and Personal Space are built in.", 2),
                (b"Private by design", b"Explicit capability controls stay local.", 8),
                (b"Ready to grow", b"Objects, apps, and AI share one system.", 7),
            ];
            for (index, (heading, detail, icon)) in items.iter().enumerate() {
                let row_top = body_top + index * 68 * scale;
                self.fill_rounded_rect_alpha(inner_left, row_top, inner_width, 54 * scale, 12 * scale, 8, 23, 38, 208);
                self.authentication_icon(inner_left + 24 * scale, row_top + 27 * scale, *icon, 24 * scale, true);
                self.ui_text_strong(inner_left + 50 * scale, row_top + 8 * scale, heading, 227, 239, 247, 1);
                self.ui_text(inner_left + 50 * scale, row_top + 29 * scale, detail, 145, 162, 178, 1);
            }
        } else if (1..=4).contains(&step) {
            self.ui_text_strong(inner_left, body_top, if step == 4 { b"PASSWORD" } else { b"PROFILE DETAIL" }, 151, 168, 184, 1);
            self.onboarding_input_field(inner_left, body_top + 28 * scale, inner_width, 50 * scale, input, masked, focus == 1, placeholder);
            let helper = if validation_error {
                if step == 4 { b"Use at least eight characters.".as_slice() } else { b"This field is required before continuing.".as_slice() }
            } else if step == 4 {
                b"Stored as a salted verifier; plaintext is never persisted.".as_slice()
            } else {
                b"You can revise this value from Settings later.".as_slice()
            };
            self.ui_text(inner_left, body_top + 88 * scale, helper, if validation_error { 255 } else { 132 }, if validation_error { 118 } else { 151 }, if validation_error { 126 } else { 168 }, 1);
        } else if step == 5 {
            let privacy_rows: [(&[u8], &[u8], usize); 4] = [
                (b"Local AI", b"On", 7usize),
                (b"Remote processing", b"Off", 11usize),
                (b"Voice and microphone", b"Off", 5usize),
                (b"Appearance", b"InfinityOS Default Dark", 8usize),
            ];
            for (index, (label, value, icon)) in privacy_rows.iter().enumerate() {
                let row_top = body_top + index * 54 * scale;
                self.fill_rounded_rect_alpha(inner_left, row_top, inner_width, 44 * scale, 10 * scale, 6, 19, 32, 216);
                self.authentication_icon(inner_left + 22 * scale, row_top + 22 * scale, *icon, 20 * scale, false);
                self.ui_text_strong(inner_left + 46 * scale, row_top + 12 * scale, label, 220, 231, 239, 1);
                let value_width = self.ui_text_width(value, 1);
                self.ui_text(inner_left + inner_width.saturating_sub(value_width + 16 * scale), row_top + 12 * scale, value, 105, 204, 240, 1);
            }
        } else {
            self.fill_rounded_rect_alpha(inner_left, body_top, inner_width, 150 * scale, 14 * scale, 6, 21, 35, 220);
            self.small_infinity_mark(inner_left + inner_width / 2, body_top + 48 * scale, 82 * scale);
            self.ui_text_centered_strong(inner_left, inner_width, body_top + 82 * scale, b"Everything is ready", 235, 243, 249, 1);
            self.ui_text_centered(inner_left, inner_width, body_top + 110 * scale, b"Enter a secure, local-first InfinityOS session.", 151, 166, 181, 1);
        }

        let button_height = 48 * scale;
        let button_top = card_top + card_height.saturating_sub(72 * scale);
        if step > 0 {
            let back_width = inner_width * 30 / 100;
            self.polished_button(inner_left, button_top, back_width, button_height, b"Back", false, focus == 0);
            let primary_left = inner_left + back_width + 12 * scale;
            self.polished_button(primary_left, button_top, inner_width.saturating_sub(back_width + 12 * scale), button_height, if step >= 6 { b"Enter InfinityOS" } else { b"Continue" }, true, focus == 1);
        } else {
            self.polished_button(inner_left, button_top, inner_width, button_height, b"Continue", true, true);
        }
    }

    // ------------------------=
    // FUNC: system_ui_frame
    // DESC: Renders onboarding, authentication, desktop, menu, lock, and Settings from shared state.
    // ------------------=
    fn system_ui_frame(
        &mut self,
        screen: u8,
        step: usize,
        input: &[u8],
        masked: bool,
        focus: usize,
        validation_error: bool,
        window_x: i32,
        window_y: i32,
        window_visible: bool,
        window_maximized: bool,
        home_location: usize,
        selected_item: Option<usize>,
        dragging_item: Option<usize>,
        note_location: usize,
        clock: crate::storage::DateTimeConfiguration,
        settings_maximized: bool,
        menu_kind: usize,
    ) {
        if matches!(screen, 5 | 6) {
            self.paint_authentication_background();
            self.authentication_frame(screen == 6, step, input, focus);
            return;
        }
        if screen == 1 {
            self.paint_first_boot_background();
            self.onboarding_frame(step, input, masked, focus, validation_error);
            return;
        }
        if matches!(screen, 2 | 3 | 4) {
            self.paint_desktop_background();
        } else {
            self.paint_first_boot_background();
        }
        let scale = self.ui_scale().max(1);
        let margin = self.width * 4 / 100;
        let top_bar = self.system_top_bar((screen == 3).then_some(menu_kind), clock);

        if matches!(screen, 2 | 3) {
            self.desktop_shell(scale, window_x, window_y, window_visible, window_maximized, home_location, selected_item, dragging_item, note_location);
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
                (inset, top_bar + inset, self.width.saturating_sub(inset * 2), self.height.saturating_sub(top_bar + inset * 2))
            } else {
                (self.width.saturating_sub(restored_width) / 2, top_bar + self.height.saturating_sub(top_bar + restored_height) / 2, restored_width, restored_height)
            };
            self.glass_panel(left, top, width, height, true);
            let title_height = 54 * scale;
            self.fill_rect_alpha(left, top, width, title_height, 6, 17, 29, 222);
            self.small_infinity_mark(left + 25 * scale, top + title_height / 2, 31 * scale);
            self.ui_text_strong(left + 50 * scale, top + 15 * scale, b"System Settings", 241, 246, 250, 1);
            for index in 0..3usize {
                let control_left = left + width.saturating_sub((26 + (2 - index) * 25) * scale);
                self.fill_rounded_rect_alpha(control_left, top + 15 * scale, 18 * scale, 18 * scale, 5 * scale, 14, 28, 42, 225);
                self.outline_rounded_rect(control_left, top + 15 * scale, 18 * scale, 18 * scale, 5 * scale, 56, 78, 96);
            }
            let nav_w = width * 28 / 100;
            self.fill_rect_alpha(left, top + title_height, nav_w, height.saturating_sub(title_height), 4, 15, 27, 214);
            self.fill_rect_alpha(left + nav_w, top + title_height, 1, height.saturating_sub(title_height), 36, 58, 76, 180);
            let sections: [&[u8]; 8] = [
                b"General",
                b"Appearance",
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
                    self.fill_rounded_rect_alpha(left + 10 * scale, y - 10 * scale, nav_w.saturating_sub(20 * scale), 36 * scale, 9 * scale, 15, 66, 100, 226);
                }
                self.authentication_icon(left + 27 * scale, y + 8 * scale, [8usize, 13, 6, 7, 8, 11, 11, 12][index], 17 * scale, focus == index);
                self.ui_text_strong(left + 48 * scale, y, section, if focus == index { 237 } else { 180 }, if focus == index { 245 } else { 198 }, if focus == index { 251 } else { 211 }, 1);
            }
            let content_x = left + nav_w + 34 * scale;
            let content_width = width.saturating_sub(nav_w + 68 * scale);
            let content_y = top + title_height + 29 * scale;
            self.ui_text_strong(content_x, content_y, sections[focus.min(7)], 238, 244, 249, 2);
            self.ui_text(content_x, content_y + 36 * scale, b"Changes apply through typed InfinityOS settings operations.", 143, 160, 176, 1);
            let rows: [(&[u8], &[u8]); 4] = match focus.min(7) {
                0 => [
                    (b"Machine Name", input),
                    (b"Language", b"English (US)"),
                    (b"Region", b"United States"),
                    (b"System Generation", b"Active"),
                ],
                1 => [
                    (b"Skin", b"InfinityOS Default Dark"),
                    (b"UI Scale", b"Automatic"),
                    (b"Accent", b"Infinity Blue"),
                    (b"Wallpaper", b"Cosmic Horizon"),
                ],
                2 => [
                    (b"Current User", b"Active"),
                    (b"Credential", b"Password"),
                    (b"Session", b"Authenticated"),
                    (b"Personal Space", b"Private"),
                ],
                3 => [
                    (b"AI Provider", b"Local only"),
                    (b"Remote Processing", b"Off"),
                    (b"Voice", b"Off"),
                    (b"Activation", b"Disabled"),
                ],
                4 => [
                    (b"Ambient Authority", b"Denied"),
                    (b"Microphone", b"Not granted"),
                    (b"Remote AI", b"Denied"),
                    (b"Session Auth", b"Verified"),
                ],
                5 => [
                    (b"Display", b"Ready"),
                    (b"Keyboard", b"Ready"),
                    (b"Pointer", b"Ready"),
                    (b"Audio Input", b"Unavailable"),
                ],
                6 => [
                    (b"Infinity Pool", b"Online"),
                    (b"System Space", b"Ready"),
                    (b"Personal Space", b"Owned"),
                    (b"Recovery Space", b"Ready"),
                ],
                _ => [
                    (b"InfinityOS", b"Development"),
                    (b"Architecture", b"Native"),
                    (b"Boot", b"Verified"),
                    (b"Identity Format", b"Version 1"),
                ],
            };
            for (index, (label, value)) in rows.iter().enumerate() {
                let y = content_y + (78 + index * 58) * scale;
                self.fill_rounded_rect_alpha(content_x, y, content_width, 46 * scale, 10 * scale, 6, 20, 33, 218);
                self.outline_rounded_rect(content_x, y, content_width, 46 * scale, 10 * scale, 35, 57, 74);
                self.ui_text_strong(content_x + 16 * scale, y + 13 * scale, label, 190, 205, 217, 1);
                let value_width = self.ui_text_width(value, 1);
                self.ui_text(content_x + content_width.saturating_sub(value_width + 16 * scale), y + 13 * scale, value, 220, 232, 240, 1);
                self.authentication_icon(content_x + content_width.saturating_sub(12 * scale), y + 23 * scale, 4, 12 * scale, false);
            }
        }
    }

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
    // FUNC: glass_panel
    // DESC: Builds a layered translucent panel with restrained shadow, highlight, and cyan edge treatment.
    // ------------------=
    fn glass_panel(&mut self, left: usize, top: usize, width: usize, height: usize, strong: bool) {
        let radius = (width.min(height) / 12).clamp(8, 18);
        if self.skin_visual_mode() == 1 {
            self.fill_rounded_rect_alpha(left, top, width, height, radius, 248, 251, 255, if strong { 244 } else { 226 });
            self.outline_rounded_rect(left, top, width, height, radius, 122, 145, 166);
            return;
        }
        if self.skin_visual_mode() == 2 {
            self.fill_rounded_rect_alpha(left, top, width, height, radius, 8, 8, 8, 255);
            self.outline_rounded_rect(left, top, width, height, radius, 255, 255, 255);
            return;
        }
        for inset in (1..=5usize).rev() {
            self.fill_rounded_rect_alpha(
                left.saturating_add(inset * 2),
                top.saturating_add(inset * 2),
                width,
                height,
                radius,
                0,
                4,
                10,
                18,
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
            self.outline_rounded_rect(left + 2, top + 2, width - 4, height - 4, radius.saturating_sub(2), 9, 35, 54);
        }
    }

    // ------------------------=
    // FUNC: desktop_icon
    // DESC: Draws one scalable glass application or collection tile and its label.
    // ------------------=
    fn desktop_icon(&mut self, left: usize, top: usize, label: &[u8], kind: usize) {
        let size = (self.height / 21).max(44);
        if kind == 3 {
            let paper_x = left + size / 7;
            let paper_w = size * 5 / 7;
            self.fill_rounded_rect_alpha(left + 4, top + 6, size, size, size / 9, 0, 3, 9, 150);
            self.fill_rounded_rect_alpha(paper_x, top, paper_w, size, size / 12, 237, 243, 248, 255);
            self.outline_rounded_rect(paper_x, top, paper_w, size, size / 12, 151, 170, 184);
            self.fill_rect_alpha(paper_x + paper_w * 2 / 3, top, paper_w / 3, size / 3, 188, 205, 218, 255);
            for row in 0..3usize {
                self.fill_rect_alpha(paper_x + size / 8, top + size * (5 + row * 2) / 12, paper_w * 2 / 3, 2, 104, 125, 141, 210);
            }
        } else {
            let tab_w = size * 9 / 16;
            self.fill_rounded_rect_alpha(left + 4, top + 7, size, size * 7 / 8, size / 8, 0, 4, 12, 160);
            self.fill_rounded_rect_alpha(left, top + size / 7, size, size * 6 / 7, size / 8, 27, 137, 219, 255);
            self.fill_rounded_rect_alpha(left + size / 14, top, tab_w, size / 3, size / 10, 68, 187, 246, 255);
            self.fill_rounded_rect_alpha(left + 2, top + size / 4, size.saturating_sub(4), size / 3, size / 12, 89, 200, 250, 190);
            self.outline_rounded_rect(left, top + size / 7, size, size * 6 / 7, size / 8, 127, 224, 255);
            let glyph_color = (7, 75, 129);
            if kind == 1 {
                self.icon_line((left + size / 2) as i32, (top + size / 3) as i32, (left + size / 2) as i32, (top + size * 2 / 3) as i32, glyph_color, size);
                self.icon_line((left + size / 3) as i32, (top + size * 7 / 12) as i32, (left + size / 2) as i32, (top + size * 3 / 4) as i32, glyph_color, size);
                self.icon_line((left + size * 2 / 3) as i32, (top + size * 7 / 12) as i32, (left + size / 2) as i32, (top + size * 3 / 4) as i32, glyph_color, size);
            } else if kind == 2 {
                self.small_infinity_mark(left + size / 2, top + size / 2, size * 2 / 3);
            } else {
                self.outline_rounded_rect(left + size / 4, top + size * 5 / 12, size / 2, size / 3, size / 14, glyph_color.0, glyph_color.1, glyph_color.2);
            }
        }
        let label_width = self.ui_text_width(label, 1);
        self.ui_text(left + size.saturating_sub(label_width) / 2, top + size + 8, label, 221, 235, 243, 1);
    }

    // ------------------------=
    // FUNC: desktop_app_icon
    // DESC: Draws one original dimensional InfinityOS application icon for dock-scale presentation.
    // ------------------=
    fn desktop_app_icon(&mut self, left: usize, top: usize, size: usize, kind: usize, active: bool) {
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
        self.fill_rounded_rect_alpha(left, top, size, size, radius, palette.0, palette.1, palette.2, 250);
        self.fill_rounded_rect_alpha(left + 2, top + 2, size.saturating_sub(4), size / 2, radius.saturating_sub(2), palette.3, palette.4, palette.5, 155);
        self.outline_rounded_rect(left, top, size, size, radius, if active { 153 } else { 75 }, if active { 228 } else { 147 }, if active { 255 } else { 190 });
        let center_x = left + size / 2;
        let center_y = top + size / 2;
        let stroke = (222, 245, 255);
        match kind {
            0 => {
                self.icon_line((left + size / 4) as i32, (top + size * 2 / 5) as i32, (left + size * 2 / 5) as i32, (top + size / 2) as i32, stroke, size);
                self.icon_line((left + size * 2 / 5) as i32, (top + size / 2) as i32, (left + size / 4) as i32, (top + size * 3 / 5) as i32, stroke, size);
                self.icon_line((left + size / 2) as i32, (top + size * 3 / 5) as i32, (left + size * 3 / 4) as i32, (top + size * 3 / 5) as i32, stroke, size);
            }
            1 => {
                self.fill_rounded_rect_alpha(left + size / 5, top + size * 7 / 20, size * 3 / 5, size * 2 / 5, size / 12, 88, 202, 250, 255);
                self.fill_rounded_rect_alpha(left + size / 4, top + size / 4, size / 3, size / 4, size / 12, 131, 225, 255, 255);
                self.outline_rounded_rect(left + size / 5, top + size * 7 / 20, size * 3 / 5, size * 2 / 5, size / 12, 182, 239, 255);
            }
            2 => {
                self.icon_circle(center_x as i32, center_y as i32, size as i32 / 3, (151, 226, 255), size);
                self.icon_circle(center_x as i32, center_y as i32, size as i32 / 5, (211, 246, 255), size);
                self.icon_line((left + size / 5) as i32, center_y as i32, (left + size * 4 / 5) as i32, center_y as i32, stroke, size);
                self.icon_line(center_x as i32, (top + size / 5) as i32, center_x as i32, (top + size * 4 / 5) as i32, stroke, size);
            }
            3 => {
                self.ui_text_centered_strong(left, size, top + size / 2 - 10, b"AI", 255, 255, 255, 1);
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
                self.icon_line((left + size * 2 / 5) as i32, center_y as i32, center_x as i32, (top + size * 3 / 5) as i32, stroke, size);
                self.icon_line(center_x as i32, (top + size * 3 / 5) as i32, (left + size * 3 / 4) as i32, (top + size * 2 / 5) as i32, stroke, size);
            }
            _ => {
                self.fill_rounded_rect_alpha(left + size / 3, top + size / 3, size / 3, size / 2, size / 10, 126, 151, 169, 240);
                self.icon_line((left + size / 4) as i32, (top + size / 3) as i32, (left + size * 3 / 4) as i32, (top + size / 3) as i32, stroke, size);
            }
        }
        if active {
            self.fill_rounded_rect_alpha(left + size / 2 - 3, top + size + 5, 6, 3, 2, 125, 220, 255, 255);
        }
    }

    // ------------------------=
    // FUNC: restore_desktop_window
    // DESC: Restores only the previous Home window rectangle from the cached desktop wallpaper during dragging.
    // ------------------=
    fn restore_desktop_window(&mut self, window_x: i32, window_y: i32) {
        let left = self.width * window_x.clamp(10, 540) as usize / 1000;
        let top = self.height * window_y.clamp(80, 550) as usize / 1000;
        let width = self.width * 43 / 100;
        let height = (self.height * 38 / 100).min(430 * self.ui_scale().max(1));
        let padding = 8usize;
        self.paint_desktop_background_rect(
            left.saturating_sub(padding),
            top.saturating_sub(padding),
            width.saturating_add(padding * 2),
            height.saturating_add(padding * 2),
        );
    }

    // ------------------------=
    // FUNC: desktop_shell
    // DESC: Renders the screenshot-matched InfinityOS desktop, home browser, status cards, and application dock.
    // ------------------=
    fn desktop_shell(&mut self, scale: usize, window_x: i32, window_y: i32, window_visible: bool, window_maximized: bool, home_location: usize, selected_item: Option<usize>, dragging_item: Option<usize>, note_location: usize) {
        if window_visible {
        let (browser_left, browser_top, browser_width, browser_height) = if window_maximized {
            let left = 10 * scale;
            let top = (46 * scale).min(self.height / 12).max(40) + 10 * scale;
            let bottom = self.height.saturating_sub(90 * scale);
            (left, top, self.width.saturating_sub(left * 2), bottom.saturating_sub(top))
        } else {
            (self.width * window_x.clamp(10, 540) as usize / 1000, self.height * window_y.clamp(80, 550) as usize / 1000, self.width * 43 / 100, (self.height * 38 / 100).min(430 * scale))
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
        self.small_infinity_mark(browser_left + 20 * scale, browser_top + title_h / 2, 24 * scale);
        self.ui_text_strong(browser_left + 38 * scale, browser_top + 7 * scale, b"Home", 226, 237, 245, 1);
        for index in 0..3usize {
            let control_size = 20 * scale;
            let control_left = browser_left + browser_width.saturating_sub((28 + (2 - index) * 27) * scale);
            self.fill_rounded_rect_alpha(control_left, browser_top + 7 * scale, control_size, control_size, 6 * scale, 13, 28, 43, 235);
            self.outline_rounded_rect(control_left, browser_top + 7 * scale, control_size, control_size, 6 * scale, 63, 84, 101);
            let center_x = control_left + control_size / 2;
            let center_y = browser_top + 17 * scale;
            if index == 0 {
                self.icon_line((center_x - 5 * scale) as i32, center_y as i32, (center_x + 5 * scale) as i32, center_y as i32, (181, 199, 212), control_size);
            } else if index == 1 {
                self.outline_rounded_rect(center_x - 5 * scale, center_y - 5 * scale, 10 * scale, 10 * scale, 2 * scale, 181, 199, 212);
            } else {
                self.icon_line((center_x - 5 * scale) as i32, (center_y - 5 * scale) as i32, (center_x + 5 * scale) as i32, (center_y + 5 * scale) as i32, (209, 220, 229), control_size);
                self.icon_line((center_x + 5 * scale) as i32, (center_y - 5 * scale) as i32, (center_x - 5 * scale) as i32, (center_y + 5 * scale) as i32, (209, 220, 229), control_size);
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
        self.authentication_icon(browser_left + 20 * scale, tool_top + 19 * scale, 3, 15 * scale, false);
        self.authentication_icon(browser_left + 48 * scale, tool_top + 19 * scale, 4, 15 * scale, false);
        let location_left = browser_left + 72 * scale;
        let location_width = browser_width.saturating_sub(124 * scale);
        self.fill_rounded_rect_alpha(location_left, tool_top + 5 * scale, location_width, 28 * scale, 8 * scale, 4, 15, 27, 238);
        self.outline_rounded_rect(location_left, tool_top + 5 * scale, location_width, 28 * scale, 8 * scale, 38, 62, 81);
        let location_names: [&[u8]; 9] = [b"Home", b"Personal Space", b"Documents", b"Downloads", b"Pictures", b"Music", b"Videos", b"Projects", b"Recycle Bin"];
        self.ui_text(location_left + 14 * scale, tool_top + 9 * scale, location_names[home_location.min(8)], 193, 211, 224, 1);
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
                self.authentication_icon(browser_left + 16 * scale, item_y + 8 * scale, icon_kind, 13 * scale, index == home_location);
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
            if (index < 6 && home_location != 0) || (index == 6 && note_location != home_location) {
                continue;
            }
            let column = index % 4;
            let row = index / 4;
            if selected_item == Some(index) {
                self.fill_rounded_rect_alpha(grid_x + column * gap.saturating_sub(6 * scale), grid_y + row * tile_step.saturating_sub(8 * scale), gap.max(44 * scale), tile_step.max(54 * scale), 8 * scale, 17, 79, 112, 190);
            }
            self.desktop_icon(grid_x + column * gap, grid_y + row * tile_step, name, *kind);
        }
        if dragging_item == Some(6) {
            self.ui_text(browser_left + sidebar_w + 28 * scale, browser_top + browser_height.saturating_sub(28 * scale), b"DROP NOTES.TXT ON A SIDEBAR LOCATION", 91, 211, 250, 1);
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
        let machine_name = machine.map(|value| value.display_name).unwrap_or(crate::runtime::identity::ShortText::empty());
        for (index, (label, value)) in [
            (b"Machine Name".as_slice(), machine_name.as_bytes()),
            (b"System Generation", b"Active".as_slice()),
            (b"Runtime", b"Online".as_slice()),
            (b"Storage Service", b"Ready".as_slice()),
            (b"Input", b"Ready".as_slice()),
            (b"Appearance", b"Default Dark".as_slice()),
        ].iter().enumerate() {
            let row_y = overview_top + (48 + index * 34) * scale;
            self.ui_text(widget_left + 18 * scale, row_y, label, 158, 174, 190, 1);
            let value_width = self.ui_text_width(value, 1);
            self.ui_text_strong(widget_left + widget_width.saturating_sub(value_width + 18 * scale), row_y, value, 223, 234, 242, 1);
            if matches!(index, 2 | 3 | 4) {
                self.fill_rounded_rect_alpha(widget_left + widget_width.saturating_sub(value_width + 30 * scale), row_y + 7 * scale, 6 * scale, 6 * scale, 3 * scale, 73, 210, 122, 255);
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
        ].iter().enumerate() {
            let row_y = ai_top + (50 + index * 34) * scale;
            self.ui_text(widget_left + 18 * scale, row_y, label, 158, 174, 190, 1);
            let value_width = self.ui_text_width(value, 1);
            self.ui_text_strong(widget_left + widget_width.saturating_sub(value_width + 18 * scale), row_y, value, 209, 225, 235, 1);
        }

        let dock_width = self.width * 54 / 100;
        let dock_height = 72 * scale;
        let dock_left = self.width.saturating_sub(dock_width) / 2;
        let dock_top = self.height.saturating_sub(dock_height + 10 * scale);
        self.glass_panel(dock_left, dock_top, dock_width, dock_height, false);
        let icon_gap = dock_width / 9;
        for index in 0..8usize {
            let size = 46 * scale;
            let x = dock_left + icon_gap / 2 + index * icon_gap;
            self.desktop_app_icon(x, dock_top + 10 * scale, size, index, matches!(index, 0 | 1));
            if index == 6 {
                self.fill_rect_alpha(x.saturating_sub(icon_gap / 3), dock_top + 10 * scale, 1, size, 69, 91, 108, 180);
            }
        }
    }

    // ------------------------=
    // FUNC: system_login_animation
    // DESC: Redraws only a narrow background strip and animated orbit lights on login to avoid full-frame flicker.
    // ------------------=
    fn system_login_animation(&mut self, phase: usize) {
        let old_phase = (phase + 382) % 384;
        let radius = (self.height / 180).max(3);
        for index in 0..7usize {
            let old_position = (old_phase + index * 47) % 384;
            let (old_x, old_y) = infinity_point(old_position);
            let old_screen_x = self.width as i32 * 75 / 100
                + old_x * self.width as i32 * 43 / 22_400;
            let old_screen_y = self.height as i32 * 34 / 100
                + old_y * self.height as i32 * 30 / 12_400;
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
            let x = self.width as i32 * 75 / 100
                + path_x * self.width as i32 * 43 / 22_400;
            let y = self.height as i32 * 34 / 100
                + path_y * self.height as i32 * 30 / 12_400;
            let orb_radius = if index == 0 { radius } else { (radius / 2).max(2) };
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
    fn system_ui_input_field(&mut self, screen: u8, step: usize, input: &[u8], masked: bool) {
        if matches!(screen, 5 | 6) {
            let fit = (self.width.saturating_mul(1000) / 1536).min(self.height.saturating_mul(1000) / 1024).max(1);
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
            self.authentication_password_field(inner_x, password_y, inner_w, 57usize.saturating_mul(fit) / 1000, input, true);
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

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_installer_background
    // DESC: Scales and paints the installer artwork across the complete display.
    // ------------------=
    fn paint_installer_background(&mut self) {
        self.paint_bitmap_cover_rect(INSTALLER_BMP, 0, 0, self.width, self.height);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_installer_masthead
    // DESC: Centers the generated InfinityOS brand masthead above every setup step.
    // ------------------=
    fn paint_installer_masthead(&mut self) {
        let height = self.height * 30 / 100;
        let width = (height * 3).min(self.width * 82 / 100);
        let left = self.width.saturating_sub(width) / 2;
        self.paint_bitmap_fit_rect(INSTALLER_MASTHEAD_BMP, left, 0, width, height);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_installer_welcome_masthead
    // DESC: Draws the taller Welcome to InfinityOS brand lockup used only by setup step one.
    // ------------------=
    fn paint_installer_welcome_masthead(&mut self) {
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
    fn paint_installer_background_rect(
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
    fn restore_installer_panel(&mut self, screen: u8) {
        // Wizard panels intentionally vary slightly by content, so repainting
        // only the next panel rectangle can leave the wider previous panel's
        // edges behind. A step transition is infrequent; restore the complete
        // scene to guarantee that two wizard screens are never composited.
        self.paint_installer_background();
        if screen == 1 {
            self.paint_installer_welcome_masthead();
        } else {
            self.paint_installer_masthead();
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_bitmap_cover_rect
    // DESC: Samples a bitmap with cover scaling and paints only the requested rectangle.
    // ------------------=
    fn paint_bitmap_cover_rect(
        &mut self,
        bitmap: &[u8],
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        if bitmap.len() < 54 || &bitmap[0..2] != b"BM" {
            return;
        }
        let offset = le32(bitmap, 10) as usize;
        let source_width = le32(bitmap, 18) as usize;
        let signed_height = le32(bitmap, 22) as i32;
        let source_height = signed_height.unsigned_abs() as usize;
        if le16(bitmap, 28) != 24 || source_width == 0 || source_height == 0 {
            return;
        }
        let row_bytes = (source_width * 3 + 3) & !3;
        let (crop_x, crop_y, crop_width, crop_height) =
            if source_width * self.height > source_height * self.width {
                let width = source_height * self.width / self.height;
                ((source_width - width) / 2, 0, width, source_height)
            } else {
                let height = source_width * self.height / self.width;
                (0, (source_height - height) / 2, source_width, height)
            };
        for y in top..(top + height).min(self.height) {
            let logical_y = crop_y + y * crop_height / self.height;
            let source_y = if signed_height < 0 {
                logical_y
            } else {
                source_height - 1 - logical_y
            };
            for x in left..(left + width).min(self.width) {
                let source_x = crop_x + x * crop_width / self.width;
                let index = offset + source_y * row_bytes + source_x * 3;
                if index + 2 >= bitmap.len() {
                    return;
                }
                self.pixel(
                    x as i32,
                    y as i32,
                    bitmap[index + 2],
                    bitmap[index + 1],
                    bitmap[index],
                );
            }
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_bitmap_fit_rect
    // DESC: Samples a bitmap with aspect-fit scaling inside the requested rectangle.
    // ------------------=
    fn paint_bitmap_fit_rect(
        &mut self,
        bitmap: &[u8],
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        if bitmap.len() < 54 || &bitmap[0..2] != b"BM" || le16(bitmap, 28) != 24 {
            return;
        }
        let offset = le32(bitmap, 10) as usize;
        let source_width = le32(bitmap, 18) as usize;
        let signed_height = le32(bitmap, 22) as i32;
        let source_height = signed_height.unsigned_abs() as usize;
        if source_width == 0 || source_height == 0 || width == 0 || height == 0 {
            return;
        }
        let row_bytes = (source_width * 3 + 3) & !3;
        for y in 0..height.min(self.height.saturating_sub(top)) {
            let sy = y * source_height / height;
            let source_y = if signed_height < 0 {
                sy
            } else {
                source_height - 1 - sy
            };
            for x in 0..width.min(self.width.saturating_sub(left)) {
                let sx = x * source_width / width;
                let index = offset + source_y * row_bytes + sx * 3;
                if index + 2 >= bitmap.len() {
                    return;
                }
                self.pixel(
                    (left + x) as i32,
                    (top + y) as i32,
                    bitmap[index + 2],
                    bitmap[index + 1],
                    bitmap[index],
                );
            }
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: paint_bitmap_cover_box
    // DESC: Aspect-crops a bitmap into a fixed destination box without painting beyond its bounds.
    // ------------------=
    fn paint_bitmap_cover_box(
        &mut self,
        bitmap: &[u8],
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        if bitmap.len() < 54 || &bitmap[0..2] != b"BM" || le16(bitmap, 28) != 24 {
            return;
        }
        let offset = le32(bitmap, 10) as usize;
        let source_width = le32(bitmap, 18) as usize;
        let signed_height = le32(bitmap, 22) as i32;
        let source_height = signed_height.unsigned_abs() as usize;
        if source_width == 0 || source_height == 0 || width == 0 || height == 0 {
            return;
        }
        let row_bytes = (source_width * 3 + 3) & !3;
        let (crop_x, crop_y, crop_width, crop_height) =
            if source_width * height > source_height * width {
                let cropped_width = source_height * width / height;
                (
                    (source_width.saturating_sub(cropped_width)) / 2,
                    0,
                    cropped_width,
                    source_height,
                )
            } else {
                let cropped_height = source_width * height / width;
                (
                    0,
                    (source_height.saturating_sub(cropped_height)) / 2,
                    source_width,
                    cropped_height,
                )
            };
        for y in 0..height.min(self.height.saturating_sub(top)) {
            let sy = crop_y + y * crop_height / height;
            let source_y = if signed_height < 0 {
                sy
            } else {
                source_height - 1 - sy
            };
            for x in 0..width.min(self.width.saturating_sub(left)) {
                let sx = crop_x + x * crop_width / width;
                let index = offset + source_y * row_bytes + sx * 3;
                if index + 2 >= bitmap.len() {
                    return;
                }
                self.pixel(
                    (left + x) as i32,
                    (top + y) as i32,
                    bitmap[index + 2],
                    bitmap[index + 1],
                    bitmap[index],
                );
            }
        }
    }

    #[cfg(target_arch = "x86")]
    // ------------------------=
    // FUNC: paint_bitmap_cover_box
    // DESC: Supplies a bounded dark hero fallback when legacy x86 omits high-resolution bitmap decoding.
    // ------------------=
    fn paint_bitmap_cover_box(
        &mut self,
        _bitmap: &[u8],
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        self.fill_rect(left, top, width, height, 2, 10, 18);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: emblem_reveal_band
    // DESC: Reveals one vertical animation band of the infinity emblem.
    // ------------------=
    fn emblem_reveal_band(&mut self, step: usize, total: usize) {
        if EMBLEM_BMP.len() < 138 || &EMBLEM_BMP[0..2] != b"BM" {
            return;
        }
        let source_width = le32(EMBLEM_BMP, 18) as usize;
        let source_height = (le32(EMBLEM_BMP, 22) as i32).unsigned_abs() as usize;
        let target_width =
            (self.width * 52 / 100).min((self.height * 70 / 100) * source_width / source_height);
        let half = target_width / 2;
        let start = half * step / total;
        let end = half * (step + 1) / total;
        self.emblem_range(half.saturating_sub(end), half.saturating_sub(start), 255);
        self.emblem_range(half + start, (half + end).min(target_width), 255);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: emblem_range
    // DESC: Blends an opacity-controlled horizontal range of the infinity emblem.
    // ------------------=
    fn emblem_range(&mut self, first_x: usize, last_x: usize, opacity: u8) {
        let source_width = le32(EMBLEM_BMP, 18) as usize;
        let source_height = (le32(EMBLEM_BMP, 22) as i32).unsigned_abs() as usize;
        let target_width =
            (self.width * 52 / 100).min((self.height * 70 / 100) * source_width / source_height);
        let target_height = target_width * source_height / source_width;
        self.emblem_rect(first_x, 0, last_x, target_height, opacity);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: emblem_rect
    // DESC: Blends a clipped rectangular region of the infinity emblem artwork.
    // ------------------=
    fn emblem_rect(
        &mut self,
        first_x: usize,
        first_y: usize,
        last_x: usize,
        last_y: usize,
        opacity: u8,
    ) {
        if EMBLEM_BMP.len() < 138 || &EMBLEM_BMP[0..2] != b"BM" || le16(EMBLEM_BMP, 28) != 32 {
            return;
        }
        let source_width = le32(EMBLEM_BMP, 18) as usize;
        let source_height = (le32(EMBLEM_BMP, 22) as i32).unsigned_abs() as usize;
        let target_width =
            (self.width * 52 / 100).min((self.height * 70 / 100) * source_width / source_height);
        let target_height = target_width * source_height / source_width;
        let left = (self.width - target_width) / 2;
        let top = self.height * BOOT_EMBLEM_TOP_PERCENT / 100;
        self.emblem_rect_at(
            left,
            top,
            target_width,
            target_height,
            first_x,
            first_y,
            last_x,
            last_y,
            opacity,
        );
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: emblem_rect_at
    // DESC: Blends a clipped emblem region into an explicitly bounded destination rectangle.
    // ------------------=
    fn emblem_rect_at(
        &mut self,
        left: usize,
        top: usize,
        target_width: usize,
        target_height: usize,
        first_x: usize,
        first_y: usize,
        last_x: usize,
        last_y: usize,
        opacity: u8,
    ) {
        if EMBLEM_BMP.len() < 138 || &EMBLEM_BMP[0..2] != b"BM" || le16(EMBLEM_BMP, 28) != 32 {
            return;
        }
        let offset = le32(EMBLEM_BMP, 10) as usize;
        let source_width = le32(EMBLEM_BMP, 18) as usize;
        let signed_height = le32(EMBLEM_BMP, 22) as i32;
        let source_height = signed_height.unsigned_abs() as usize;
        for y in first_y..last_y.min(target_height) {
            let sy = y * source_height / target_height;
            let source_y = if signed_height < 0 {
                sy
            } else {
                source_height - 1 - sy
            };
            for x in first_x..last_x.min(target_width) {
                let sx = x * source_width / target_width;
                let index = offset + (source_y * source_width + sx) * 4;
                if index + 3 >= EMBLEM_BMP.len() {
                    return;
                }
                let alpha = ((EMBLEM_BMP[index + 3] as u16 * opacity as u16) / 255) as u8;
                if alpha < 3 {
                    continue;
                }
                let blue = EMBLEM_BMP[index] as u16;
                let green = EMBLEM_BMP[index + 1] as u16;
                let red = EMBLEM_BMP[index + 2] as u16;
                let a = alpha as u16;
                let address = unsafe { self.buffer.add((top + y) * self.stride + left + x) };
                let current = unsafe { read_volatile(address) };
                let (cr, cg, cb) = if self.format == 0 {
                    (
                        (current & 255) as u16,
                        ((current >> 8) & 255) as u16,
                        ((current >> 16) & 255) as u16,
                    )
                } else {
                    (
                        ((current >> 16) & 255) as u16,
                        ((current >> 8) & 255) as u16,
                        (current & 255) as u16,
                    )
                };
                self.pixel(
                    (left + x) as i32,
                    (top + y) as i32,
                    ((red * a + cr * (255 - a)) / 255) as u8,
                    ((green * a + cg * (255 - a)) / 255) as u8,
                    ((blue * a + cb * (255 - a)) / 255) as u8,
                );
            }
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: finish_emblem
    // DESC: Draws the complete infinity emblem after its reveal animation finishes.
    // ------------------=
    fn finish_emblem(&mut self) {
        let source_width = le32(EMBLEM_BMP, 18) as usize;
        let source_height = (le32(EMBLEM_BMP, 22) as i32).unsigned_abs() as usize;
        let target_width =
            (self.width * 52 / 100).min((self.height * 70 / 100) * source_width / source_height);
        let target_height = target_width * source_height / source_width;
        let left = (self.width - target_width) / 2;
        let top = self.height * BOOT_EMBLEM_TOP_PERCENT / 100;
        let padding = self.ui_scale() * 32;
        self.paint_background_rect(
            left.saturating_sub(padding),
            top.saturating_sub(padding),
            target_width + padding * 2,
            target_height + padding * 2,
        );
        self.emblem_range(0, target_width, 255);
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: finish_console_emblem
    // DESC: Draws a complete uncropped emblem over the installed-system background.
    // ------------------=
    fn finish_console_emblem(&mut self) {
        let (left, top, target_width, target_height) = self.console_emblem_geometry();
        let padding = self.ui_scale() * 32;
        self.paint_console_background_rect(
            left.saturating_sub(padding),
            top.saturating_sub(padding),
            target_width + padding * 2,
            target_height + padding * 2,
        );
        self.emblem_rect_at(
            left,
            top,
            target_width,
            target_height,
            0,
            0,
            target_width,
            target_height,
            255,
        );
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: restore_emblem_particle
    // DESC: Restores artwork beneath a moving emblem particle before its next frame.
    // ------------------=
    fn restore_emblem_particle(&mut self, x: i32, y: i32, radius: i32, bootstrap: bool) {
        let (emblem_left, emblem_top, target_width, target_height) = if bootstrap {
            let source_width = le32(EMBLEM_BMP, 18) as usize;
            let source_height = (le32(EMBLEM_BMP, 22) as i32).unsigned_abs() as usize;
            let target_width = (self.width * 52 / 100)
                .min((self.height * 70 / 100) * source_width / source_height);
            let target_height = target_width * source_height / source_width;
            (
                (self.width - target_width) / 2,
                self.height * BOOT_EMBLEM_TOP_PERCENT / 100,
                target_width,
                target_height,
            )
        } else {
            self.console_emblem_geometry()
        };
        let left = (x - radius - 2).max(0) as usize;
        let top = (y - radius - 2).max(0) as usize;
        let size = (radius * 2 + 5) as usize;
        if bootstrap {
            self.paint_background_rect(left, top, size, size);
        } else {
            self.paint_console_background_rect(left, top, size, size);
        }
        let local_left = left.saturating_sub(emblem_left);
        let local_top = top.saturating_sub(emblem_top);
        let local_right = (left + size).saturating_sub(emblem_left).min(target_width);
        let local_bottom = (top + size).saturating_sub(emblem_top).min(target_height);
        if local_left < local_right && local_top < local_bottom {
            self.emblem_rect_at(
                emblem_left,
                emblem_top,
                target_width,
                target_height,
                local_left,
                local_top,
                local_right,
                local_bottom,
                255,
            );
        }
    }

    // ------------------------=
    // FUNC: infinity_pulse
    // DESC: Draws the luminous pulse and trail along the infinity-shaped path.
    // ------------------=
    fn infinity_pulse(&mut self, head: usize) {
        let scale_x = (self.width.min(self.height * 16 / 9) as i32 / 400).max(1);
        let scale_y = (scale_x * 2 / 3).max(1);
        let center_x = self.width as i32 / 2;
        let center_y = self.height as i32 * 37 / 100 + BOOT_PARTICLE_Y_OFFSET;
        let (head_x, head_y) = infinity_point(head);
        self.glow(
            center_x + head_x * scale_x,
            center_y + head_y * scale_y,
            scale_x * 2,
            80,
        );
        self.glow(
            center_x + head_x * scale_x,
            center_y + head_y * scale_y,
            (scale_x / 2).max(2),
            235,
        );
        for age in (2..14usize).rev() {
            let index = (head + 384 - age * 3) % 384;
            let (px, py) = infinity_point(index);
            let strength = 18 + ((14 - age) * 6) as u8;
            self.glow(
                center_x + px * scale_x,
                center_y + py * scale_y,
                (scale_x / 2).max(2),
                strength,
            );
            if age % 4 == 0 {
                let jitter_x = (((index * 17 + age * 11) % 9) as i32 - 4) * scale_x;
                let jitter_y = (((index * 7 + age * 13) % 7) as i32 - 3) * scale_y;
                self.glow(
                    center_x + px * scale_x + jitter_x,
                    center_y + py * scale_y + jitter_y,
                    (scale_x / 3).max(1),
                    strength.saturating_add(55),
                );
            }
        }
    }

    // ------------------------=
    // FUNC: glow
    // DESC: Draws a soft white radial glow centered at the requested point.
    // ------------------=
    fn glow(&mut self, x: i32, y: i32, radius: i32, intensity: u8) {
        self.glow_color(x, y, radius, 255, 255, 255, intensity);
    }

    // ------------------------=
    // FUNC: glow_color
    // DESC: Draws a soft colored radial glow with distance-based opacity.
    // ------------------=
    fn glow_color(
        &mut self,
        x: i32,
        y: i32,
        radius: i32,
        red: u8,
        green: u8,
        blue: u8,
        intensity: u8,
    ) {
        for oy in -radius..=radius {
            for ox in -radius..=radius {
                let distance = ox * ox + oy * oy;
                if distance <= radius * radius {
                    let alpha = ((radius * radius - distance) as u32 * intensity as u32
                        / (radius * radius).max(1) as u32) as u8;
                    self.blend_color(x + ox, y + oy, red, green, blue, alpha);
                }
            }
        }
    }

    // ------------------------=
    // FUNC: star_orb
    // DESC: Draws a star-like orbiting particle and optional bright trail head.
    // ------------------=
    fn star_orb(&mut self, x: i32, y: i32, radius: i32, intensity: u8, head: bool) {
        self.glow_color(x, y, radius * 2, 108, 176, 255, intensity / 3);
        self.glow_color(x, y, radius, 220, 238, 255, intensity);
        self.glow_color(x, y, (radius / 3).max(1), 255, 255, 255, 255);
        if head {
            for distance in 1..=radius * 3 {
                let alpha = (180i32.saturating_sub(distance * 150 / (radius * 3).max(1))) as u8;
                self.blend_white(x + distance, y, alpha);
                self.blend_white(x - distance, y, alpha);
                self.blend_white(x, y + distance, alpha);
                self.blend_white(x, y - distance, alpha);
                if distance <= radius * 2 {
                    self.blend_color(x + distance, y + distance, 176, 216, 255, alpha / 2);
                    self.blend_color(x - distance, y - distance, 176, 216, 255, alpha / 2);
                    self.blend_color(x + distance, y - distance, 176, 216, 255, alpha / 2);
                    self.blend_color(x - distance, y + distance, 176, 216, 255, alpha / 2);
                }
            }
        }
    }

    // ------------------------=
    // FUNC: progress
    // DESC: Draws the loading label and bounded progress bar for the boot sequence.
    // ------------------=
    fn progress(&mut self, percent: usize, label: &[u8]) {
        let ui_scale = self.ui_scale();
        let width = (self.width * 58 / 100).max(240);
        let height = 28usize * ui_scale;
        let left = (self.width - width) / 2;
        let top = self.height * 82 / 100;
        for y in 0..height {
            for x in 0..width {
                let border = x < 2 || y < 2 || x >= width - 2 || y >= height - 2;
                let filled = x < (width - 4) * percent / 100 + 2;
                let (r, g, b) = if border {
                    (184, 184, 188)
                } else if filled {
                    (112, 112, 118)
                } else {
                    (30, 30, 34)
                };
                self.pixel((left + x) as i32, (top + y) as i32, r, g, b);
            }
        }
        let text_width = self.installer_text_width(label, false);
        self.text(
            (self.width.saturating_sub(text_width)) / 2,
            top + height.saturating_sub(INSTALLER_FONT_CELL_HEIGHT) / 2,
            label,
            235,
            235,
            238,
        );
    }

    // ------------------------=
    // FUNC: terminal_box
    // DESC: Draws the seven-row console panel, command input, and selection state.
    // ------------------=
    fn terminal_box(
        &mut self,
        lines: &[[u8; 96]; 6],
        lengths: &[usize; 6],
        line_count: usize,
        prompt: &[u8],
        command: &[u8],
        _split_menu: bool,
    ) {
        let ui_scale = self.ui_scale();
        let width = (self.width * 58 / 100)
            .max(420)
            .min(self.width.saturating_sub(24));
        let row_height = INSTALLER_FONT_CELL_HEIGHT + 4;
        let height = row_height * 7 + 16 * ui_scale;
        let left = (self.width - width) / 2;
        let top = self.height * 68 / 100;
        for y in 0..height {
            for x in 0..width {
                let border = x < 2 || y < 2 || x >= width - 2 || y >= height - 2;
                let (r, g, b) = if border {
                    (166, 170, 180)
                } else {
                    (13, 15, 20)
                };
                self.pixel((left + x) as i32, (top + y) as i32, r, g, b);
            }
        }
        for row in 0..line_count.min(6) {
            let (red, green, blue) = (216, 222, 232);
            self.text(
                left + 14 * ui_scale,
                top + 8 * ui_scale + row * row_height,
                &lines[row][..lengths[row]],
                red,
                green,
                blue,
            );
        }
        if !prompt.is_empty() && line_count < 7 {
            let y = top + 8 * ui_scale + line_count * row_height;
            self.text(left + 14 * ui_scale, y, prompt, 238, 241, 247);
            let prompt_width = self.installer_text_width(prompt, false);
            let command_width = self.installer_text_width(command, false);
            self.text(
                left + 14 * ui_scale + prompt_width,
                y,
                command,
                255,
                255,
                255,
            );
            self.text(
                left + 14 * ui_scale + prompt_width + command_width,
                y,
                b"_",
                255,
                255,
                255,
            );
        }
    }

    // ------------------------=
    // FUNC: startup_bbs
    // DESC: Composes the BBS-style startup console over the boot artwork.
    // ------------------=
    fn startup_bbs(&mut self, command: &[u8], cursor_x: i32, cursor_y: i32, pressed: bool) {
        let scale = self.ui_scale();
        let left = self.width * 13 / 100;
        let top = self.height * 58 / 100;
        let width = self.width * 74 / 100;
        let height = self.height * 39 / 100;
        self.fill_rect(left, top, width, height, 7, 11, 18);
        self.outline_rect(left, top, width, height, 142, 164, 190);
        self.outline_rect(
            left + 4 * scale,
            top + 4 * scale,
            width.saturating_sub(8 * scale),
            height.saturating_sub(8 * scale),
            42,
            68,
            94,
        );
        let text_left = left + 22 * scale;

        let graffiti: [&[u8]; 6] = [
            b".___ _______  ___________.___ _______  .___________________.___.________    _________",
            b"|   |\\      \\ \\_   _____/|   |\\      \\ |   \\__    ___/\\__  |   |\\_____  \\  /   _____/",
            b"|   |/   |   \\ |    __)  |   |/   |   \\|   | |    |    /   |   | /   |   \\ \\_____  \\",
            b"|   /    |    \\|     \\   |   /    |    \\   | |    |    \\____   |/    |    \\/        \\",
            b"|___\\____|__  /\\___  /   |___\\____|__  /___| |____|    / ______|\\_______  /_______  /",
            b"            \\/     \\/                \\/                \\/               \\/        \\/",
        ];
        for (row, line) in graffiti.iter().enumerate() {
            let (red, green, blue) = if row == 0 || row == 5 {
                (143, 215, 255)
            } else if row & 1 == 0 {
                (248, 251, 255)
            } else {
                (214, 233, 249)
            };
            // Keep the wide FIGlet banner at its native cell size; ordinary
            // interface text below it uses the larger readable UI scale.
            self.text_scaled(
                text_left,
                top + 13 * scale + row * 11,
                line,
                red,
                green,
                blue,
                1,
                false,
            );
        }
        self.text(
            text_left,
            top + 82 * scale,
            b"--[ Welcome to infinityOS ] -- SYSTEM READY --",
            112,
            181,
            224,
        );
        self.startup_option_rows(cursor_x, cursor_y, pressed);
        let prompt_y = self.height * 88 / 100;
        let prompt_x = self.width * 18 / 100;
        self.infinity_prompt_mark(prompt_x, prompt_y);
        let prompt_label = b" -> ";
        let prompt_label_x = prompt_x + 21 * scale;
        self.text(prompt_label_x, prompt_y, prompt_label, 216, 229, 242);
        let command_x = prompt_label_x + self.installer_text_width(prompt_label, false);
        self.text(command_x, prompt_y, command, 248, 250, 255);
        self.text(
            command_x + self.installer_text_width(command, false),
            prompt_y,
            b"_",
            248,
            250,
            255,
        );
        self.text(
            self.width * 18 / 100,
            self.height * 93 / 100,
            b"TAB / ARROWS: MOVE     ENTER: SELECT     TYPE: install | repair | console",
            121,
            158,
            191,
        );
    }

    // ------------------------=
    // FUNC: infinity_prompt_mark
    // DESC: Draws the compact infinity mark used beside the command prompt.
    // ------------------=
    fn infinity_prompt_mark(&mut self, left: usize, top: usize) {
        let scale = self.ui_scale();
        for &(path_x, path_y) in &INFINITY_PATH {
            let x = left + ((path_x + 112) as usize * 18 * scale / 224);
            let y = top + ((62 - path_y) as usize * 8 * scale / 124);
            self.fill_rect(x, y, scale.max(1), scale.max(1), 216, 229, 242);
        }
    }

    // ------------------------=
    // FUNC: startup_option_rows
    // DESC: Draws and highlights the mouse-selectable startup actions.
    // ------------------=
    fn startup_option_rows(&mut self, cursor_x: i32, cursor_y: i32, pressed: bool) {
        let scale = self.ui_scale();
        let options: [(&[u8], &[u8], i32); 3] = [
            (b"[1]  INSTALL INFINITYOS", b"Begin the installation", 735),
            (
                b"[2]  REPAIR AN EXISTING INSTALLATION",
                b"Inspect and repair",
                785,
            ),
            (b"[3]  OPEN SYSTEM CONSOLE", b"Enter system console", 835),
        ];
        for (title, subtitle, center) in options {
            let row_top = self.height * (center as usize - 23) / 1000;
            let row_height = self.height * 46 / 1000;
            let selected = (cursor_y - center).abs() <= 25;
            let hovered = selected && (180..=820).contains(&cursor_x);
            let (r, g, b) = if hovered && pressed {
                (32, 54, 76)
            } else if selected {
                (20, 39, 58)
            } else {
                (10, 17, 27)
            };
            self.fill_rect(
                self.width * 18 / 100,
                row_top,
                self.width * 64 / 100,
                row_height,
                r,
                g,
                b,
            );
            if selected {
                self.fill_rect(
                    self.width * 18 / 100,
                    row_top,
                    3 * scale,
                    row_height,
                    201,
                    225,
                    246,
                );
            }
            let color = if selected {
                (248, 251, 255)
            } else {
                (207, 220, 234)
            };
            self.text(
                self.width * 20 / 100,
                row_top + 4 * scale,
                title,
                color.0,
                color.1,
                color.2,
            );
            self.text(
                self.width * 49 / 100,
                row_top + 4 * scale,
                subtitle,
                104,
                151,
                190,
            );
        }
    }

    // ------------------------=
    // FUNC: installer_navigation
    // DESC: Draws installer Back and Continue controls with pointer and focus states.
    // ------------------=
    fn installer_navigation(
        &mut self,
        screen: u8,
        focus: usize,
        has_primary: bool,
        cursor_x: i32,
        cursor_y: i32,
        pressed: bool,
        redraw_foundation: bool,
    ) {
        let modal_or_progress = screen == 7 || screen == 8;
        let navigation_top = match screen {
            1 => 87,
            2 => 90,
            _ => 89,
        };
        let button_top = match screen {
            1 => 810,
            2 => 820,
            _ => 830,
        };
        let hint_top = match screen {
            1 => 895,
            2 => 935,
            _ => 910,
        };
        // Rebuild the shared navigation rail when a step changes so labels do
        // not accumulate over the translucent photographic background.
        if redraw_foundation {
            // One continuous footer rail matches the reference and prevents
            // the three formerly separate foundations from looking clipped.
            self.fill_rect_alpha(
                self.width * 8 / 100,
                self.height * navigation_top / 100,
                self.width * 84 / 100,
                self.height * 4 / 100,
                7,
                12,
                19,
                232,
            );
            self.fill_rect(
                self.width * 10 / 100,
                self.height * navigation_top / 100,
                self.width * 80 / 100,
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
            let (button_left, button_width) = match screen {
                1 => (145, 325),
                2 => (135, 350),
                _ => (190, 300),
            };
            self.installer_button(
                button_left,
                button_top,
                button_width,
                55,
                back,
                back_subtitle,
                focus == 0,
                cursor_x,
                cursor_y,
                pressed,
            );
        }
        if has_primary && !modal_or_progress {
            let (button_left, button_width) = match screen {
                1 => (480, 350),
                2 => (510, 350),
                _ => (510, 300),
            };
            self.installer_button(
                button_left,
                button_top,
                button_width,
                55,
                primary,
                subtitle,
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
                b"INSTALLATION IN PROGRESS    PLEASE KEEP THIS DEVICE POWERED"
            } else if screen == 5 {
                b"LEFT / RIGHT: FIELD    UP / DOWN: CHANGE    TAB: MOVE    ENTER: SELECT"
            } else {
                b"TAB / ARROWS: MOVE FOCUS    ENTER: SELECT    ESC: CANCEL    F1: HELP"
            };
            let hint_width = self.installer_text_width(hint, false);
            self.installer_text(
                self.width.saturating_sub(hint_width) / 2,
                self.height * hint_top / 1000,
                hint,
                156,
                174,
                202,
            );
        }
    }

    // ------------------------=
    // FUNC: installer_confirmation_popup
    // DESC: Draws the modal destructive-action warning and its two explicit choices.
    // ------------------=
    fn installer_confirmation_popup(
        &mut self,
        focus: usize,
        cursor_x: i32,
        cursor_y: i32,
        pressed: bool,
    ) {
        let scale = self.ui_scale();
        let left = self.width * 25 / 100;
        let top = self.height * 34 / 100;
        let width = self.width * 50 / 100;
        let height = self.height * 32 / 100;
        let radius = 16 * scale;
        // Repaint from an opaque glass base on every state change. This keeps
        // alpha highlights deterministic instead of accumulating luminance
        // when the pointer crosses a control boundary.
        self.fill_rounded_rect_alpha(
            left + 9 * scale,
            top + 11 * scale,
            width,
            height,
            radius,
            0,
            3,
            9,
            170,
        );
        self.fill_rounded_rect_alpha(left, top, width, height, radius, 4, 14, 27, 255);
        self.fill_rounded_rect_alpha(
            left + 2 * scale,
            top + 2 * scale,
            width.saturating_sub(4 * scale),
            height / 3,
            radius.saturating_sub(2 * scale),
            20,
            44,
            66,
            88,
        );
        self.outline_rounded_rect(left, top, width, height, radius, 69, 122, 156);
        self.fill_rounded_rect_alpha(
            left + 24 * scale,
            top + 25 * scale,
            42 * scale,
            42 * scale,
            12 * scale,
            91,
            43,
            39,
            255,
        );
        self.authentication_icon(
            left + 45 * scale,
            top + 46 * scale,
            8,
            23 * scale,
            true,
        );
        self.ui_text_strong(
            left + 82 * scale,
            top + 26 * scale,
            b"Erase this disk?",
            250,
            247,
            244,
            1,
        );
        self.ui_text(
            left + 82 * scale,
            top + 50 * scale,
            b"This action permanently removes all existing data.",
            188,
            203,
            216,
            1,
        );
        self.ui_text(
            left + 24 * scale,
            top + 88 * scale,
            b"InfinityOS will create a new Infinity Pool on the selected disk.",
            221,
            231,
            239,
            1,
        );
        self.ui_text_strong(
            left + 24 * scale,
            top + 116 * scale,
            b"This cannot be undone.",
            246,
            158,
            139,
            1,
        );
        self.installer_button(
            310,
            570,
            170,
            60,
            b"CANCEL",
            b"Keep the disk unchanged",
            focus == 0,
            cursor_x,
            cursor_y,
            pressed,
        );
        self.installer_button(
            520,
            570,
            210,
            60,
            b"ERASE & INSTALL",
            b"Begin installation",
            focus == 1,
            cursor_x,
            cursor_y,
            pressed,
        );
        let hint = b"Tab / arrows: choose     Enter: confirm     Esc: cancel";
        self.ui_text_centered(
            left,
            width,
            top + height.saturating_sub(27 * scale),
            hint,
            137,
            159,
            179,
            1,
        );
    }

    // ------------------------=
    // FUNC: installer_activation_art
    // DESC: Draws the generated system-activation artwork used by installation progress and completion.
    // ------------------=
    fn installer_activation_art(&mut self, compact: bool) {
        #[cfg(target_arch = "x86")]
        let _ = compact;

        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        {
            let left = self.width * if compact { 52 } else { 22 } / 100;
            let top = self.height * if compact { 40 } else { 37 } / 100;
            let width = self.width * if compact { 35 } else { 56 } / 100;
            let height = (width * 9 / 16).min(self.height * if compact { 34 } else { 32 } / 100);
            self.paint_bitmap_fit_rect(
                INSTALLER_ACTIVATION_BMP,
                left,
                top,
                width,
                height,
            );
            self.outline_rect(
                left,
                top,
                width,
                height,
                48,
                118,
                164,
            );
        }
    }

    // ------------------------=
    // FUNC: installer_progress_frame
    // DESC: Draws one verified installation-progress frame over the activation artwork.
    // ------------------=
    fn installer_progress_frame(&mut self, percent: usize, label: &[u8], phase: usize) {
        let scale = self.ui_scale();
        let left = self.width * 18 / 100;
        let width = self.width * 64 / 100;
        let top = self.height * 72 / 100;
        let height = self.height * 11 / 100;
        self.fill_rounded_rect_alpha(left, top, width, height, 14 * scale, 3, 13, 25, 255);
        self.fill_rounded_rect_alpha(left + 2, top + 2, width.saturating_sub(4), height / 2, 12 * scale, 24, 49, 70, 90);
        self.outline_rounded_rect(left, top, width, height, 14 * scale, 52, 116, 155);

        self.ui_text_centered_strong(
            left,
            width,
            top + 14 * scale,
            label,
            225,
            238,
            248,
            1,
        );

        let track_left = left + 28 * scale;
        let track_top = top + 43 * scale;
        let track_width = width.saturating_sub(56 * scale);
        let track_height = 10 * scale;
        self.fill_rect(track_left, track_top, track_width, track_height, 68, 75, 85);
        self.fill_rect(
            track_left,
            track_top,
            track_width * percent.min(100) / 100,
            track_height,
            185,
            207,
            224,
        );
        self.outline_rect(
            track_left,
            track_top,
            track_width,
            track_height,
            126,
            164,
            191,
        );

        for marker in 0..=4usize {
            let marker_x = track_left + track_width * marker / 4;
            let active = percent >= marker * 25;
            self.star_orb(
                marker_x as i32,
                (track_top + track_height / 2) as i32,
                if active {
                    3 * scale as i32
                } else {
                    2 * scale as i32
                },
                if active { 238 } else { 74 },
                active,
            );
        }

        let (path_x, path_y) = infinity_point(phase);
        let orb_x = self.width as i32 / 2 + path_x * (self.width as i32 / 820).max(1);
        let orb_y = self.height as i32 * 55 / 100 + path_y * (self.height as i32 / 900).max(1);
        self.star_orb(orb_x, orb_y, 4 * scale as i32, 244, true);

        let mut percent_text = [b'0'; 4];
        let value = percent.min(100);
        let digits = if value == 100 {
            percent_text[0] = b'1';
            percent_text[1] = b'0';
            percent_text[2] = b'0';
            3
        } else if value >= 10 {
            percent_text[0] = b'0' + (value / 10) as u8;
            percent_text[1] = b'0' + (value % 10) as u8;
            2
        } else {
            percent_text[0] = b'0' + value as u8;
            1
        };
        percent_text[digits] = b'%';
        let percent_slice = &percent_text[..digits + 1];
        let percent_width = self.ui_text_width(percent_slice, 1);
        self.ui_text_strong(
            track_left + track_width.saturating_sub(percent_width),
            top + 14 * scale,
            percent_slice,
            111,
            204,
            248,
            1,
        );
    }

    // ------------------------=
    // FUNC: installer_countdown_frame
    // DESC: Draws a cinematic, bounded reboot countdown frame after installation succeeds.
    // ------------------=
    fn installer_countdown_frame(&mut self, remaining: usize, frame: usize, phase: usize) {
        let scale = self.ui_scale();
        let left = self.width * 31 / 100;
        let top = self.height * 48 / 100;
        let width = self.width * 38 / 100;
        let height = self.height * 27 / 100;
        self.fill_rect(left + 7 * scale, top + 7 * scale, width, height, 1, 3, 7);
        self.fill_rect(left, top, width, height, 6, 13, 22);
        self.outline_rect(left, top, width, height, 87, 180, 230);

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
    fn installer_panel(
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
        let scale = self.ui_scale();
        let (left_percent, top_percent, width_percent, height_percent) = match screen {
            1 => (2, 41, 96, 52),
            2 | 3 => (2, 32, 96, 66),
            _ => (8, 33, 84, 61),
        };
        let left = self.width * left_percent / 100;
        // The reference layout uses one tall, reusable setup card beneath the
        // brand masthead. Every step inherits this geometry so navigation does
        // not jump while the user moves through the installer.
        let top = self.height * top_percent / 100;
        let width = self.width * width_percent / 100;
        let height = self.height * height_percent / 100;
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
        let title: &[u8] = match screen {
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
        self.text(left + 18 * scale, top + 15 * scale, title, 232, 240, 249);
        let section: &[u8] = if screen == 11 {
            b"HELP / F1 TO RETURN"
        } else {
            b"GUIDED SETUP"
        };
        let section_width = self.installer_text_width(section, false);
        self.text(
            left + width.saturating_sub(section_width + 18 * scale),
            top + 15 * scale,
            section,
            108,
            174,
            216,
        );
        self.fill_rect(
            left + 18 * scale,
            top + 30 * scale,
            width.saturating_sub(36 * scale),
            scale,
            50,
            86,
            116,
        );
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
        for row in 0..line_count.min(6) {
            if screen == 8 {
                break;
            }
            let row_spacing = if screen == 1 { 15 } else { 20 };
            let y = top + 48 * scale + row * row_spacing * scale;
            let color = if row == 0 {
                (244, 248, 253)
            } else {
                (216, 226, 237)
            };
            self.text(
                left + 24 * scale,
                y,
                &lines[row][..lengths[row]],
                color.0,
                color.1,
                color.2,
            );
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
            self.text(
                left + 24 * scale + prompt_width,
                y,
                command,
                255,
                255,
                255,
            );
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
            let step_x = if screen == 1 {
                self.width * 88 / 100
            } else {
                left + width.saturating_sub(44 * scale)
            };
            let step_y = if screen == 1 {
                self.height * 79 / 100
            } else {
                top + height.saturating_sub(82 * scale)
            };
            self.text(step_x, step_y, &step, 105, 154, 193);
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: installer_pool_panel
    // DESC: Composes the approved second-step Infinity Pool explanation and topology.
    // ------------------=
    fn installer_pool_panel(
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
        self.installer_text_strong(
            left + inset,
            header_y,
            b"HOW INFINITY POOL WORKS",
            220,
            230,
            241,
        );
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

        self.fill_rect_alpha(left_panel, content_top, left_width, content_height, 2, 11, 20, 202);
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
        self.installer_headline_text(
            copy_x,
            headline_y,
            b"ONE DISK BECOMES",
            246,
            249,
            253,
        );
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
        for (index, line) in [
            b"The Infinity Pool organizes your disk into four".as_slice(),
            b"protected areas that keep your data safe, isolated,",
            b"and easy to recover.",
        ]
        .iter()
        .enumerate()
        {
            self.installer_text(
                copy_x,
                body_y + index * 26,
                line,
                193,
                207,
                222,
            );
        }
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
        self.installer_pool_space_cards(copy_x, cards_top, left_width.saturating_sub(width * 50 / 1000), cards_height);

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
    fn installer_pool_space_cards(
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
    fn installer_pool_space_icon(
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
                self.line(cx - half / 2, cy - half / 2, cx, cy - half * 3 / 4, color.0, color.1, color.2);
                self.line(cx, cy - half * 3 / 4, cx + half / 2, cy - half / 2, color.0, color.1, color.2);
                self.line(cx - half / 2, cy - half / 2, cx - half / 2, cy + half / 5, color.0, color.1, color.2);
                self.line(cx + half / 2, cy - half / 2, cx + half / 2, cy + half / 5, color.0, color.1, color.2);
                self.line(cx - half / 2, cy + half / 5, cx, cy + half * 3 / 4, color.0, color.1, color.2);
                self.line(cx + half / 2, cy + half / 5, cx, cy + half * 3 / 4, color.0, color.1, color.2);
            }
            1 => {
                self.icon_circle(cx, cy - half / 3, half / 3, color, size);
                self.line(cx - half / 2, cy + half * 2 / 3, cx - half / 2, cy + half / 3, color.0, color.1, color.2);
                self.line(cx - half / 2, cy + half / 3, cx, cy + half / 8, color.0, color.1, color.2);
                self.line(cx + half / 2, cy + half / 3, cx, cy + half / 8, color.0, color.1, color.2);
                self.line(cx + half / 2, cy + half / 3, cx + half / 2, cy + half * 2 / 3, color.0, color.1, color.2);
            }
            2 => {
                let cell = (half * 2 / 3).max(4) as usize;
                for (dx, dy) in [(-half * 2 / 3, -half * 2 / 3), (half / 6, -half * 2 / 3), (-half * 2 / 3, half / 6), (half / 6, half / 6)] {
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
                self.line(cx - half * 3 / 4, cy - half / 3, cx - half / 3, cy - half / 2, color.0, color.1, color.2);
                self.line(cx - half * 3 / 4, cy - half / 3, cx - half * 2 / 3, cy + half / 8, color.0, color.1, color.2);
            }
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: installer_disk_discovery_panel
    // DESC: Composes the live disk-discovery step from detected device facts and approved artwork.
    // ------------------=
    fn installer_disk_discovery_panel(
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
        self.installer_text_strong(left + inset, header_y, b"DISKS DISCOVERED", 220, 230, 241);
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

        self.fill_rect_alpha(left_panel, content_top, left_width, content_height, 1, 10, 19, 225);
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
        self.installer_compact_text(
            left_panel + panel_inset,
            available_y + INSTALLER_FONT_CELL_HEIGHT,
            b"Select a disk to add to the Infinity Pool.",
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
            self.fill_rect_alpha(card_left, card_top, card_width, card_height / 2, 20, 67, 92, 44);
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
            self.installer_compact_text_strong(
                badge_left + 8,
                title_y,
                badge,
                189,
                225,
                248,
            );

            self.installer_compact_text(
                detail_x,
                title_y + 30,
                device.model(),
                126,
                192,
                234,
            );
            let (capacity, capacity_length) = Self::installer_capacity_label(device.capacity_mib());
            self.installer_compact_text(
                detail_x,
                title_y + 55,
                &capacity[..capacity_length],
                126,
                192,
                234,
            );
            let capacity_width = self.installer_compact_text_width(&capacity[..capacity_length], false);
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
            self.installer_compact_text_strong(
                vision_x + 42,
                row_y,
                title,
                164,
                215,
                246,
            );
            self.installer_compact_text(
                vision_x + 42,
                row_y + 22,
                subtitle,
                87,
                177,
                229,
            );
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
    fn installer_discovery_benefit_icon(
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
                self.line(cx - half / 2, cy - half / 2, cx, cy - half * 3 / 4, color.0, color.1, color.2);
                self.line(cx, cy - half * 3 / 4, cx + half / 2, cy - half / 2, color.0, color.1, color.2);
                self.line(cx - half / 2, cy - half / 2, cx - half / 2, cy + half / 5, color.0, color.1, color.2);
                self.line(cx + half / 2, cy - half / 2, cx + half / 2, cy + half / 5, color.0, color.1, color.2);
                self.line(cx - half / 2, cy + half / 5, cx, cy + half * 3 / 4, color.0, color.1, color.2);
                self.line(cx + half / 2, cy + half / 5, cx, cy + half * 3 / 4, color.0, color.1, color.2);
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
                self.line(cx - half * 2 / 3, cy - half / 3, cx + half * 2 / 3, cy + half / 3, color.0, color.1, color.2);
                self.line(cx - half * 2 / 3, cy + half / 3, cx + half * 2 / 3, cy - half / 3, color.0, color.1, color.2);
            }
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    // ------------------------=
    // FUNC: installer_disk_kind
    // DESC: Maps discovered hardware traits to a concise user-facing device class.
    // ------------------=
    fn installer_disk_kind(device: &crate::storage::StorageDevice) -> &'static [u8] {
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
    fn installer_capacity_label(capacity_mib: u64) -> ([u8; 24], usize) {
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
    fn installer_append_decimal(buffer: &mut [u8], length: &mut usize, mut value: u64) {
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
    fn installer_append_label(buffer: &mut [u8], length: &mut usize, suffix: &[u8]) {
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
    fn installer_welcome_panel(
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
        self.installer_text_strong(
            left + inset,
            header_y,
            b"WELCOME TO INFINITYOS",
            220,
            230,
            241,
        );
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

        // The 24px Roboto body face needs a full four-line text block in the
        // narrow overview cards. Keep the band above the navigation rail and
        // allocate enough vertical room for descenders on the final line.
        let cards_top = top + height * 540 / 1000;
        let cards_height = height * 225 / 1000;
        self.installer_mesh_overview_row(left_column, cards_top, left_width, cards_height);
        self.installer_mesh_feature_row(hero_left, cards_top, hero_width, cards_height);
    }

    // ------------------------=
    // FUNC: installer_corner_accents
    // DESC: Draws the short cyan corner brackets used by the approved setup frame.
    // ------------------=
    fn installer_corner_accents(&mut self, left: usize, top: usize, width: usize, height: usize) {
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
    fn installer_welcome_callout(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
    ) {
        self.fill_rect_alpha(left, top, width, height, 1, 10, 19, 194);
        self.outline_rect(left, top, width, height, 18, 83, 119);
        self.installer_corner_accents(left, top, width, height);
        self.installer_text_strong(
            left + 42,
            top + 15,
            b"WELCOME TO INFINITYOS",
            50,
            203,
            246,
        );
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
        for (index, line) in [
            b"InfinityOS is a distributed operating system.".as_slice(),
            b"One simple home for your system, your apps,",
            b"and everything you create.",
            b"",
            b"We'll guide you through every choice.",
            b"Nothing changes until you approve it.",
        ]
        .iter()
        .enumerate()
        {
            self.installer_text(
                body_x,
                body_y + index * (INSTALLER_FONT_CELL_HEIGHT + 2),
                line,
                193,
                207,
                222,
            );
        }
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
    // DESC: Draws the four compact mesh benefits in one equally spaced lower-left row.
    // ------------------=
    fn installer_mesh_overview_row(
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
        self.fill_rect(left, label_y + 10, label_x.saturating_sub(left + 14), 1, 23, 91, 128);
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

        let items: [(&[u8], &[u8], usize); 4] = [
            (
                b"SHARE RESOURCES",
                b"CPU, GPU, storage, and bandwidth across your devices.",
                0,
            ),
            (
                b"SYNC SECURELY",
                b"Fast, private sync that keeps your data safe.",
                1,
            ),
            (
                b"YOU CONTROL",
                b"Every device is yours. Your data. Your rules.",
                2,
            ),
            (
                b"BUILT FOR TOMORROW",
                b"Scale from one device to thousands. InfinityOS grows with you.",
                3,
            ),
        ];
        let gap = 12usize;
        let card_width = width.saturating_sub(gap * 3) / 4;
        for (index, (title, body, icon)) in items.iter().enumerate() {
            let x = left + index * (card_width + gap);
            self.fill_rect_alpha(x, top, card_width, height, 2, 13, 23, 204);
            self.outline_rect(x, top, card_width, height, 27, 75, 104);
            self.installer_mesh_icon(x + 27, top + 28, *icon, 26);
            self.installer_compact_text_strong(x + 16, top + 43, title, 52, 198, 241);
            self.fill_rect(x + 14, top + 74, card_width.saturating_sub(28), 1, 22, 81, 112);
            self.installer_compact_text_wrapped(
                x + 16,
                top + 78,
                card_width.saturating_sub(32),
                body,
                181,
                199,
                217,
                4,
            );
        }
    }

    // ------------------------=
    // FUNC: installer_mesh_feature_row
    // DESC: Places the four approved mesh principles as one aligned strip beneath the hero.
    // ------------------=
    fn installer_mesh_feature_row(
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
    fn installer_mesh_icon(
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
                    self.line(cx + offset, cy - half / 2, cx + offset * 2 / 3, cy - half / 2, color.0, color.1, color.2);
                    self.line(cx + offset, cy + half / 2, cx + offset * 2 / 3, cy + half / 2, color.0, color.1, color.2);
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
                self.line(cx, cy + half / 4, cx, cy + half / 2, color.0, color.1, color.2);
            }
            2 => {
                let nodes = [(cx, cy - half), (cx - half, cy + half / 2), (cx + half, cy + half / 2), (cx, cy)];
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
    fn installer_welcome_top(&mut self) {
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
    fn installer_mesh_feature_overlay(&mut self) {
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
    fn installer_feature_card(
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
    fn installer_mesh_node(&mut self, center_x: usize, center_y: usize, kind: usize) {
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
    fn installer_mesh_overview(&mut self) {
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

    // ------------------------=
    // FUNC: installer_date_time_panel
    // DESC: Draws the functional date, time, and typed time-zone installer screen.
    // ------------------=
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn installer_date_time_panel(
        &mut self,
        left: usize,
        top: usize,
        width: usize,
        height: usize,
        date_time: crate::storage::DateTimeConfiguration,
        active_part: usize,
        focus: usize,
        cursor_x: i32,
        cursor_y: i32,
        pressed: bool,
    ) {
        let scale = self.ui_scale();
        self.fill_rect_alpha(left, top, width, height, 2, 10, 18, 232);
        self.fill_rect_alpha(left, top, width, height / 3, 4, 20, 33, 42);
        self.outline_rect(left, top, width, height, 16, 137, 190);
        self.outline_rect(
            left + 4 * scale,
            top + 4 * scale,
            width.saturating_sub(8 * scale),
            height.saturating_sub(8 * scale),
            20,
            73,
            104,
        );
        self.installer_corner_accents(left, top, width, height);

        let inset = width * 20 / 1000;
        let header_y = top + height * 27 / 1000;
        self.installer_text_strong(left + inset, header_y, b"DATE & TIME", 220, 230, 241);
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
            scale,
            20,
            88,
            124,
        );

        let form_left = self.width * 10 / 100;
        let form_top = self.height * 405 / 1000;
        let form_width = self.width * 38 / 100;
        let form_height = self.height * 365 / 1000;
        self.fill_rect_alpha(form_left, form_top, form_width, form_height, 2, 13, 23, 212);
        self.outline_rounded_rect(form_left, form_top, form_width, form_height, 12 * scale, 25, 95, 132);
        self.installer_corner_accents(form_left, form_top, form_width, form_height);
        self.installer_headline_text(
            form_left + 20 * scale,
            form_top + 20 * scale,
            b"SET YOUR LOCAL TIME",
            244,
            249,
            254,
        );
        self.installer_text(
            form_left + 20 * scale,
            form_top + 58 * scale,
            b"Used by your clock, calendar, events, and activity history.",
            172,
            195,
            215,
        );

        let mut date = *b"0000-00-00";
        date[0] = b'0' + ((date_time.year / 1000) % 10) as u8;
        date[1] = b'0' + ((date_time.year / 100) % 10) as u8;
        date[2] = b'0' + ((date_time.year / 10) % 10) as u8;
        date[3] = b'0' + (date_time.year % 10) as u8;
        Self::installer_two_digits(&mut date, 5, date_time.month);
        Self::installer_two_digits(&mut date, 8, date_time.day);
        let mut time = *b"00:00";
        Self::installer_two_digits(&mut time, 0, date_time.hour);
        Self::installer_two_digits(&mut time, 3, date_time.minute);
        let fields: [(&[u8], &[u8], usize, usize); 3] = [
            (b"DATE", &date, 465, 2),
            (b"TIME", &time, 555, 3),
            (b"TIME ZONE", Self::installer_time_zone_label(date_time.time_zone_id), 645, 4),
        ];
        for (label, value, normalized_y, field_focus) in fields {
            let field_top = self.height * normalized_y / 1000;
            let field_height = self.height * 70 / 1000;
            let focused = focus == field_focus;
            let hovered = (100..=475).contains(&cursor_x)
                && (normalized_y as i32..=(normalized_y + 70) as i32).contains(&cursor_y);
            self.fill_rounded_rect_alpha(
                self.width * 10 / 100,
                field_top,
                self.width * 38 / 100,
                field_height,
                9 * scale,
                if focused || hovered { 9 } else { 4 },
                if focused || hovered { 31 } else { 20 },
                if focused || hovered { 48 } else { 33 },
                if hovered && pressed { 250 } else { 226 },
            );
            self.outline_rounded_rect(
                self.width * 10 / 100,
                field_top,
                self.width * 38 / 100,
                field_height,
                9 * scale,
                if focused { 67 } else { 31 },
                if focused { 196 } else { 92 },
                if focused { 241 } else { 126 },
            );
            self.installer_compact_text_strong(
                self.width * 115 / 1000,
                field_top + 8 * scale,
                label,
                60,
                199,
                244,
            );
            self.installer_text(
                self.width * 17 / 100,
                field_top + 29 * scale,
                value,
                232,
                241,
                250,
            );
            self.installer_text_strong(
                self.width * 125 / 1000,
                field_top + 28 * scale,
                b"-",
                77,
                203,
                246,
            );
            self.installer_text_strong(
                self.width * 44 / 100,
                field_top + 28 * scale,
                b"+",
                77,
                203,
                246,
            );
        }

        let active_y = if focus == 2 { 465 } else if focus == 3 { 555 } else { 645 };
        if focus == 2 || focus == 3 {
            let (part_x, part_width) = if focus == 2 {
                match active_part.min(2) {
                    0 => (170, 82),
                    1 => (270, 43),
                    _ => (330, 43),
                }
            } else if active_part == 4 {
                (305, 48)
            } else {
                (220, 48)
            };
            self.fill_rect(
                self.width * part_x / 1000,
                self.height * (active_y + 57) / 1000,
                self.width * part_width / 1000,
                2 * scale,
                63,
                202,
                246,
            );
        }

        let art_left = self.width * 50 / 100;
        let art_top = self.height * 405 / 1000;
        let art_width = self.width * 40 / 100;
        let art_height = self.height * 365 / 1000;
        self.fill_rect_alpha(art_left, art_top, art_width, art_height, 1, 9, 18, 226);
        let map_width = art_width.saturating_sub(8 * scale);
        let map_height = (map_width / 2).min(art_height.saturating_sub(8 * scale));
        let map_left = art_left + (art_width.saturating_sub(map_width)) / 2;
        let map_top = art_top + (art_height.saturating_sub(map_height)) / 2;
        self.paint_bitmap_fit_rect(
            DATE_TIME_WORLD_BMP,
            map_left,
            map_top,
            map_width,
            map_height,
        );
        let (longitude, latitude) = Self::installer_time_zone_map_coordinates(date_time.time_zone_id);
        let marker_x = map_left as i32 + (longitude as i32 + 180) * map_width as i32 / 360;
        let marker_y = map_top as i32 + (90 - latitude as i32) * map_height as i32 / 180;
        let band_width = (map_width / 24).max(4 * scale);
        let band_left = (marker_x - band_width as i32 / 2)
            .max(map_left as i32)
            .min((map_left + map_width.saturating_sub(band_width)) as i32) as usize;
        self.fill_rect_alpha(
            band_left,
            map_top,
            band_width,
            map_height,
            32,
            173,
            238,
            54,
        );
        self.fill_rect(band_left, map_top, scale, map_height, 63, 196, 241);
        self.fill_rect(
            band_left + band_width.saturating_sub(scale),
            map_top,
            scale,
            map_height,
            63,
            196,
            241,
        );
        self.fill_rect(
            map_left,
            marker_y.max(map_top as i32) as usize,
            map_width,
            scale,
            45,
            139,
            190,
        );
        self.star_orb(marker_x, marker_y, 4 * scale as i32, 245, true);
        self.fill_rect_alpha(map_left, map_top, map_width, 45 * scale, 1, 8, 17, 196);
        self.fill_rect_alpha(
            map_left,
            map_top + map_height.saturating_sub(38 * scale),
            map_width,
            38 * scale,
            1,
            8,
            17,
            208,
        );
        self.outline_rounded_rect(art_left, art_top, art_width, art_height, 12 * scale, 38, 111, 151);
        self.installer_corner_accents(art_left, art_top, art_width, art_height);
        self.installer_text_strong(
            art_left + 22 * scale,
            art_top + 20 * scale,
            b"SELECT YOUR TIME ZONE",
            64,
            202,
            247,
        );
        self.installer_text(
            art_left + 22 * scale,
            art_top + art_height.saturating_sub(31 * scale),
            Self::installer_time_zone_label(date_time.time_zone_id),
            184,
            204,
            221,
        );
        let instruction = b"CLICK MAP  |  LEFT / RIGHT";
        let instruction_width = self.installer_text_width(instruction, false);
        self.installer_text(
            art_left + art_width.saturating_sub(22 * scale + instruction_width),
            art_top + art_height.saturating_sub(31 * scale),
            instruction,
            84,
            187,
            229,
        );
        self.installer_text(
            left + width.saturating_sub(inset + self.installer_text_width(b"05", false)),
            self.height * 80 / 100,
            b"05",
            105,
            194,
            232,
        );
    }

    // ------------------------=
    // FUNC: installer_two_digits
    // DESC: Writes one zero-padded two-digit value into a fixed installer label.
    // ------------------=
    fn installer_two_digits(output: &mut [u8], offset: usize, value: u8) {
        output[offset] = b'0' + (value / 10) % 10;
        output[offset + 1] = b'0' + value % 10;
    }

    // ------------------------=
    // FUNC: installer_time_zone_label
    // DESC: Projects a stable time-zone ID into its human-readable installer label.
    // ------------------=
    fn installer_time_zone_label(id: u16) -> &'static [u8] {
        match id {
            1 => b"UTC-08:00  Pacific",
            2 => b"UTC-07:00  Mountain",
            3 => b"UTC-06:00  Central",
            4 => b"UTC-05:00  Eastern",
            5 => b"UTC-04:00  Atlantic",
            7 => b"UTC+01:00  Central Europe",
            8 => b"UTC+05:30  India",
            9 => b"UTC+08:00  Singapore",
            10 => b"UTC+09:00  Japan",
            11 => b"UTC+10:00  Eastern Australia",
            _ => b"UTC+00:00  Universal",
        }
    }

    // ------------------------=
    // FUNC: installer_time_zone_map_coordinates
    // DESC: Returns a representative longitude and latitude for a typed time-zone marker.
    // ------------------=
    fn installer_time_zone_map_coordinates(id: u16) -> (i16, i16) {
        match id {
            1 => (-122, 37),
            2 => (-111, 40),
            3 => (-95, 40),
            4 => (-74, 40),
            5 => (-63, 45),
            7 => (10, 50),
            8 => (78, 22),
            9 => (104, 1),
            10 => (139, 36),
            11 => (151, -33),
            _ => (0, 51),
        }
    }

    // ------------------------=
    // FUNC: installer_button
    // DESC: Draws one normalized installer button with hover and keyboard focus styling.
    // ------------------=
    fn installer_button(
        &mut self,
        nx: usize,
        ny: usize,
        nw: usize,
        nh: usize,
        title: &[u8],
        subtitle: &[u8],
        focused: bool,
        cursor_x: i32,
        cursor_y: i32,
        pressed: bool,
    ) {
        let left = self.width * nx / 1000;
        let top = self.height * ny / 1000;
        let width = self.width * nw / 1000;
        let height = self.height * nh / 1000;
        let hovered = cursor_x >= nx as i32
            && cursor_x <= (nx + nw) as i32
            && cursor_y >= ny as i32
            && cursor_y <= (ny + nh) as i32;
        let depressed = hovered && pressed;
        let (r, g, b) = if hovered && pressed {
            (48, 67, 91)
        } else if focused || hovered {
            (25, 42, 63)
        } else {
            (14, 20, 30)
        };
        let radius = (height / 7).clamp(7, 13);
        self.fill_rounded_rect_alpha(left, top, width, height, radius, r, g, b, 255);
        self.fill_rounded_rect_alpha(
            left + 2,
            top + 2,
            width.saturating_sub(4),
            height / 2,
            radius.saturating_sub(2),
            55,
            92,
            125,
            if focused || hovered { 78 } else { 28 },
        );
        let (or, og, ob) = if focused {
            (210, 234, 255)
        } else if hovered {
            (150, 205, 255)
        } else {
            (76, 94, 120)
        };
        self.outline_rounded_rect(left, top, width, height, radius, or, og, ob);
        if focused {
            self.fill_rect(left, top, self.ui_scale() * 4, height, 235, 246, 255);
        }
        let scale = self.ui_scale();
        let press_offset = if depressed { 2 * scale } else { 0 };
        let block_height = INSTALLER_FONT_CELL_HEIGHT * 2 + 4;
        let title_y = top + height.saturating_sub(block_height) / 2 + press_offset;
        let subtitle_y = title_y + INSTALLER_FONT_CELL_HEIGHT + 4;
        let title_width = self.installer_text_width(title, true);
        let subtitle_width = self.installer_text_width(subtitle, false);
        let title_x = left + width.saturating_sub(title_width) / 2;
        let subtitle_x = left + width.saturating_sub(subtitle_width) / 2;
        self.installer_text_strong(title_x, title_y, title, 244, 248, 255);
        self.installer_text(subtitle_x, subtitle_y, subtitle, 83, 187, 230);
        if focused || hovered {
            let arrow_x = if nx < 480 {
                (left + 26 * scale) as i32
            } else {
                (left + width.saturating_sub(24 * scale)) as i32
            };
            let arrow_y = (top + height / 2 + press_offset) as i32;
            self.line(
                arrow_x - 8 * scale as i32,
                arrow_y,
                arrow_x,
                arrow_y,
                106,
                207,
                255,
            );
            self.line(
                arrow_x - 4 * scale as i32,
                arrow_y - 4 * scale as i32,
                arrow_x,
                arrow_y,
                106,
                207,
                255,
            );
            self.line(
                arrow_x - 4 * scale as i32,
                arrow_y + 4 * scale as i32,
                arrow_x,
                arrow_y,
                106,
                207,
                255,
            );
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
    last_home_window_visible: bool,
    last_home_window_maximized: bool,
    last_home_location: usize,
    last_home_selected_item: Option<usize>,
    last_home_dragging_item: Option<usize>,
    last_home_note_location: usize,
    last_system_clock: crate::storage::DateTimeConfiguration,
    last_settings_maximized: bool,
    system_ui_active: bool,
    cursor_saved: bool,
    cursor_left: usize,
    cursor_top: usize,
    cursor_width: usize,
    cursor_height: usize,
    cursor_backing: [u32; 128 * 128],
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
            last_home_window_visible: false,
            last_home_window_maximized: false,
            last_home_location: usize::MAX,
            last_home_selected_item: None,
            last_home_dragging_item: None,
            last_home_note_location: usize::MAX,
            last_system_clock: crate::storage::DateTimeConfiguration::utc_default(),
            last_settings_maximized: false,
            system_ui_active: false,
            cursor_saved: false,
            cursor_left: 0,
            cursor_top: 0,
            cursor_width: 0,
            cursor_height: 0,
            cursor_backing: [0; 128 * 128],
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
    window_visible: bool,
    window_maximized: bool,
    home_location: usize,
    selected_item: Option<usize>,
    dragging_item: Option<usize>,
    note_location: usize,
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
            let structural_change_without_window = console.last_system_screen != screen
                || console.last_system_step != step
                || console.last_system_focus != focus
                || console.last_system_menu != menu_kind
                || console.last_system_validation_error != validation_error
                || console.last_home_window_visible != window_visible
                || console.last_home_window_maximized != window_maximized
                || console.last_home_location != home_location
                || console.last_home_selected_item != selected_item
                || console.last_home_dragging_item != dragging_item
                || console.last_home_note_location != note_location
                || console.last_system_clock != clock
                || console.last_settings_maximized != settings_maximized;
            let window_moved = console.last_home_window_x != window_x
                || console.last_home_window_y != window_y;
            let content_changed = console.last_system_content != content;
            if window_moved
                && !structural_change_without_window
                && !content_changed
                && screen == 2
                && console.last_system_screen == 2
                && window_visible
                && !window_maximized
            {
                console
                    .display
                    .restore_desktop_window(console.last_home_window_x, console.last_home_window_y);
                let scale = console.display.ui_scale().max(1);
                console.display.desktop_shell(scale, window_x, window_y, window_visible, window_maximized, home_location, selected_item, dragging_item, note_location);
            } else if structural_change_without_window || window_moved {
                console
                    .display
                    .system_ui_frame(screen, step, input, masked, focus, validation_error, window_x, window_y, window_visible, window_maximized, home_location, selected_item, dragging_item, note_location, clock, settings_maximized, menu_kind);
            } else if content_changed
                && (matches!(screen, 5 | 6) || (screen == 1 && (1..=4).contains(&step)))
            {
                console
                    .display
                    .system_ui_input_field(screen, step, input, masked);
            } else if content_changed {
                console
                    .display
                    .system_ui_frame(screen, step, input, masked, focus, validation_error, window_x, window_y, window_visible, window_maximized, home_location, selected_item, dragging_item, note_location, clock, settings_maximized, menu_kind);
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
            console.last_home_window_visible = window_visible;
            console.last_home_window_maximized = window_maximized;
            console.last_home_location = home_location;
            console.last_home_selected_item = selected_item;
            console.last_home_dragging_item = dragging_item;
            console.last_home_note_location = note_location;
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
    _window_visible: bool,
    _window_maximized: bool,
    _home_location: usize,
    _selected_item: Option<usize>,
    _dragging_item: Option<usize>,
    _note_location: usize,
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
