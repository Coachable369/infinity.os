//! Versioned pluggable skin packages and transactional appearance activation.

use super::geometry::{Insets, Scale};

pub const SKIN_PACKAGE_VERSION: u16 = 1;
pub const MAX_SKINS: usize = 4;
pub const SKIN_ID_BYTES: usize = 32;
pub const SKIN_HEADER_BYTES: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum ColorRole {
    Canvas = 0,
    Panel = 1,
    PanelRaised = 2,
    Border = 3,
    TextPrimary = 4,
    TextSecondary = 5,
    Accent = 6,
    AccentBright = 7,
    Focus = 8,
    Success = 9,
    Warning = 10,
    Danger = 11,
    Scrim = 12,
}

pub const COLOR_ROLE_COUNT: usize = 13;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Color(pub u32);

impl Color {
    // ------------------------=
    // FUNC: rgb
    // DESC: Creates one opaque semantic color from a bounded 24-bit RGB value.
    // ------------------=
    pub const fn rgb(value: u32) -> Self {
        Self(0xff00_0000 | (value & 0x00ff_ffff))
    }

    // ------------------------=
    // FUNC: channels
    // DESC: Returns the red, green, and blue channels used by framebuffer renderers.
    // ------------------=
    pub const fn channels(self) -> (u8, u8, u8) {
        (
            ((self.0 >> 16) & 0xff) as u8,
            ((self.0 >> 8) & 0xff) as u8,
            (self.0 & 0xff) as u8,
        )
    }

    // ------------------------=
    // FUNC: rgb24
    // DESC: Returns the portable 24-bit representation stored in a user profile.
    // ------------------=
    pub const fn rgb24(self) -> u32 {
        self.0 & 0x00ff_ffff
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AccentSurface {
    WindowOutline,
    Header,
    TopBar,
    Dock,
    Widget,
    Focus,
    Selection,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SkinId {
    bytes: [u8; SKIN_ID_BYTES],
    length: u8,
}

impl SkinId {
    // ------------------------=
    // FUNC: from_bytes
    // DESC: Creates a bounded stable skin identifier from ASCII bytes.
    // ------------------=
    pub const fn from_bytes(value: &[u8]) -> Self {
        let mut bytes = [0; SKIN_ID_BYTES];
        let mut index = 0;
        while index < value.len() && index < SKIN_ID_BYTES {
            bytes[index] = value[index];
            index += 1;
        }
        Self {
            bytes,
            length: index as u8,
        }
    }

    // ------------------------=
    // FUNC: as_bytes
    // DESC: Exposes the stable identifier without allocation.
    // ------------------=
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.length as usize]
    }
}

#[derive(Clone, Copy)]
pub struct SkinTokens {
    pub colors: [Color; COLOR_ROLE_COUNT],
    pub spacing: [u16; 8],
    pub corner_radius: [u16; 4],
    pub panel_insets: Insets,
    pub default_scale: Scale,
    pub motion_duration_ms: [u16; 4],
}

impl SkinTokens {
    // ------------------------=
    // FUNC: color
    // DESC: Resolves a semantic color role into an ARGB color value.
    // ------------------=
    pub const fn color(&self, role: ColorRole) -> Color {
        self.colors[role as usize]
    }
}

#[derive(Clone, Copy)]
pub struct SkinPackage {
    pub id: SkinId,
    pub parent: Option<SkinId>,
    pub format_version: u16,
    pub minimum_ui_abi: u16,
    pub content_hash: u64,
    pub tokens: SkinTokens,
    pub trusted: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SkinError {
    Full,
    Duplicate,
    Unknown,
    InvalidMagic,
    InvalidVersion,
    InvalidScale,
    InvalidHash,
    MissingParent,
    InheritanceCycle,
    ActivationFailed,
    InvalidAccent,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppearanceScope {
    Machine,
    User,
    Session,
}

pub struct SkinRegistry {
    packages: [Option<SkinPackage>; MAX_SKINS],
    active: SkinId,
    previous: SkinId,
    last_known_good: SkinId,
    safe: SkinId,
    generation: u32,
    accent_override: Option<Color>,
}

impl SkinRegistry {
    // ------------------------=
    // FUNC: new
    // DESC: Creates the registry with official dark, alternate diagnostic, and SafeSkin packages.
    // ------------------=
    pub const fn new() -> Self {
        let dark = default_dark_skin();
        let alternate = diagnostic_light_skin();
        let safe = safe_skin();
        Self {
            packages: [Some(dark), Some(alternate), Some(safe), None],
            active: dark.id,
            previous: dark.id,
            last_known_good: dark.id,
            safe: safe.id,
            generation: 1,
            accent_override: None,
        }
    }

    // ------------------------=
    // FUNC: register
    // DESC: Validates and registers a bounded skin package without activating it.
    // ------------------=
    pub fn register(&mut self, package: SkinPackage) -> Result<(), SkinError> {
        validate_package(&package)?;
        if self.find(package.id).is_some() {
            return Err(SkinError::Duplicate);
        }
        if let Some(parent) = package.parent {
            if self.find(parent).is_none() {
                return Err(SkinError::MissingParent);
            }
            if parent == package.id {
                return Err(SkinError::InheritanceCycle);
            }
        }
        let slot = self
            .packages
            .iter_mut()
            .find(|slot| slot.is_none())
            .ok_or(SkinError::Full)?;
        *slot = Some(package);
        Ok(())
    }

    // ------------------------=
    // FUNC: find
    // DESC: Resolves a skin package by stable identifier.
    // ------------------=
    pub fn find(&self, id: SkinId) -> Option<&SkinPackage> {
        self.packages
            .iter()
            .flatten()
            .find(|package| package.id == id)
    }

    // ------------------------=
    // FUNC: active
    // DESC: Returns the currently active validated skin package.
    // ------------------=
    pub fn active(&self) -> &SkinPackage {
        self.find(self.active)
            .or_else(|| self.find(self.safe))
            .unwrap()
    }

    // ------------------------=
    // FUNC: activate
    // DESC: Transactionally switches skins and rolls back if validation fails.
    // ------------------=
    pub fn activate(&mut self, id: SkinId, _scope: AppearanceScope) -> Result<u32, SkinError> {
        let candidate = *self.find(id).ok_or(SkinError::Unknown)?;
        validate_package(&candidate)?;
        let old = self.active;
        self.previous = old;
        self.active = id;
        if self.active().id != id {
            self.active = old;
            return Err(SkinError::ActivationFailed);
        }
        self.last_known_good = id;
        self.generation = self.generation.wrapping_add(1);
        Ok(self.generation)
    }

    // ------------------------=
    // FUNC: rollback
    // DESC: Restores the previously active package after a failed appearance transaction.
    // ------------------=
    pub fn rollback(&mut self) {
        self.active = self.previous;
        self.generation = self.generation.wrapping_add(1);
    }

    // ------------------------=
    // FUNC: enter_safe_mode
    // DESC: Activates the built-in dependency-free SafeSkin recovery package.
    // ------------------=
    pub fn enter_safe_mode(&mut self) {
        self.previous = self.active;
        self.active = self.safe;
        self.generation = self.generation.wrapping_add(1);
    }

    // ------------------------=
    // FUNC: generation
    // DESC: Returns the monotonic appearance generation used to invalidate caches.
    // ------------------=
    pub const fn generation(&self) -> u32 {
        self.generation
    }

    // ------------------------=
    // FUNC: color
    // DESC: Resolves a semantic role while honoring the current user accent override.
    // ------------------=
    pub fn color(&self, role: ColorRole) -> Color {
        match (role, self.accent_override) {
            (ColorRole::Accent, Some(accent)) => accent,
            (ColorRole::AccentBright | ColorRole::Focus, Some(accent)) => {
                mix_color(accent, Color::rgb(0x00ff_ffff), 112)
            }
            _ => self.active().tokens.color(role),
        }
    }

    // ------------------------=
    // FUNC: set_accent
    // DESC: Applies one user-scoped accent and invalidates every semantic appearance surface.
    // ------------------=
    pub fn set_accent(
        &mut self,
        accent_rgb: u32,
        _scope: AppearanceScope,
    ) -> Result<u32, SkinError> {
        if accent_rgb == 0 || accent_rgb > 0x00ff_ffff {
            return Err(SkinError::InvalidAccent);
        }
        self.accent_override = Some(Color::rgb(accent_rgb));
        self.generation = self.generation.wrapping_add(1);
        Ok(self.generation)
    }

    // ------------------------=
    // FUNC: accent_rgb
    // DESC: Returns the active user accent as a portable 24-bit RGB value.
    // ------------------=
    pub fn accent_rgb(&self) -> u32 {
        self.color(ColorRole::Accent).rgb24()
    }

    // ------------------------=
    // FUNC: accent_surface
    // DESC: Derives consistent window, navigation, dock, widget, focus, and selection colors.
    // ------------------=
    pub fn accent_surface(&self, surface: AccentSurface) -> Color {
        let accent = self.color(ColorRole::Accent);
        match surface {
            AccentSurface::WindowOutline => mix_color(accent, Color::rgb(0x00ff_ffff), 34),
            AccentSurface::Header => mix_color(accent, Color::rgb(0x0002_0c18), 184),
            AccentSurface::TopBar => mix_color(accent, Color::rgb(0x0000_0710), 208),
            AccentSurface::Dock => mix_color(accent, Color::rgb(0x0002_0c18), 194),
            AccentSurface::Widget => mix_color(accent, Color::rgb(0x0005_1524), 166),
            AccentSurface::Focus => mix_color(accent, Color::rgb(0x00ff_ffff), 92),
            AccentSurface::Selection => mix_color(accent, Color::rgb(0x0005_1b2a), 132),
        }
    }
}

// ------------------------=
// FUNC: mix_color
// DESC: Blends two opaque colors with an integer amount suitable for no-std rendering.
// ------------------=
fn mix_color(source: Color, target: Color, target_amount: u8) -> Color {
    let (source_r, source_g, source_b) = source.channels();
    let (target_r, target_g, target_b) = target.channels();
    let amount = target_amount as u32;
    let inverse = 255u32.saturating_sub(amount);
    Color::rgb(
        (((source_r as u32 * inverse + target_r as u32 * amount) / 255) << 16)
            | (((source_g as u32 * inverse + target_g as u32 * amount) / 255) << 8)
            | ((source_b as u32 * inverse + target_b as u32 * amount) / 255),
    )
}

// ------------------------=
// FUNC: hsv_to_rgb
// DESC: Converts picker hue, saturation, and value coordinates into a portable RGB accent.
// ------------------=
pub fn hsv_to_rgb(hue: u16, saturation: u8, value: u8) -> u32 {
    let hue = hue.min(359) as u32;
    let saturation = saturation as u32;
    let value = value as u32;
    if saturation == 0 {
        return (value << 16) | (value << 8) | value;
    }
    let region = hue / 60;
    let remainder = (hue % 60) * 255 / 60;
    let p = value * (255 - saturation) / 255;
    let q = value * (255 - saturation * remainder / 255) / 255;
    let t = value * (255 - saturation * (255 - remainder) / 255) / 255;
    let (red, green, blue) = match region {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        _ => (value, p, q),
    };
    (red << 16) | (green << 8) | blue
}

// ------------------------=
// FUNC: rgb_to_hsv
// DESC: Converts a persisted RGB accent into stable picker coordinates.
// ------------------=
pub fn rgb_to_hsv(rgb: u32) -> (u16, u8, u8) {
    let red = ((rgb >> 16) & 0xff) as i32;
    let green = ((rgb >> 8) & 0xff) as i32;
    let blue = (rgb & 0xff) as i32;
    let maximum = red.max(green).max(blue);
    let minimum = red.min(green).min(blue);
    let delta = maximum - minimum;
    let value = maximum as u8;
    let saturation = if maximum == 0 {
        0
    } else {
        (delta * 255 / maximum) as u8
    };
    if delta == 0 {
        return (0, saturation, value);
    }
    let mut hue = if maximum == red {
        60 * (green - blue) / delta
    } else if maximum == green {
        120 + 60 * (blue - red) / delta
    } else {
        240 + 60 * (red - green) / delta
    };
    if hue < 0 {
        hue += 360;
    }
    (hue as u16, saturation, value)
}

// ------------------------=
// FUNC: validate_package
// DESC: Enforces native package schema, scaling, identity, and content-integrity constraints.
// ------------------=
pub fn validate_package(package: &SkinPackage) -> Result<(), SkinError> {
    if package.format_version != SKIN_PACKAGE_VERSION
        || package.minimum_ui_abi > super::INFINITY_UI_ABI_VERSION
    {
        return Err(SkinError::InvalidVersion);
    }
    if package.id.as_bytes().is_empty() || !package.tokens.default_scale.valid() {
        return Err(SkinError::InvalidScale);
    }
    if package.content_hash == 0 {
        return Err(SkinError::InvalidHash);
    }
    Ok(())
}

// ------------------------=
// FUNC: encode_header
// DESC: Serializes the explicit little-endian native skin header without Rust ABI layout.
// ------------------=
pub fn encode_header(package: &SkinPackage, out: &mut [u8; SKIN_HEADER_BYTES]) {
    out.fill(0);
    out[..8].copy_from_slice(b"INFSKIN1");
    out[8..10].copy_from_slice(&package.format_version.to_le_bytes());
    out[10..12].copy_from_slice(&package.minimum_ui_abi.to_le_bytes());
    out[12..20].copy_from_slice(&package.content_hash.to_le_bytes());
    out[20] = package.id.length;
    out[21..53].copy_from_slice(&package.id.bytes);
    out[53] = package.parent.map(|value| value.length).unwrap_or(0);
    out[54] = package.trusted as u8;
    let checksum = header_checksum(&out[..60]);
    out[60..64].copy_from_slice(&checksum.to_le_bytes());
}

// ------------------------=
// FUNC: decode_header
// DESC: Parses and validates a bounded native skin header before asset loading.
// ------------------=
pub fn decode_header(
    data: &[u8; SKIN_HEADER_BYTES],
    tokens: SkinTokens,
) -> Result<SkinPackage, SkinError> {
    if &data[..8] != b"INFSKIN1" {
        return Err(SkinError::InvalidMagic);
    }
    if u32::from_le_bytes([data[60], data[61], data[62], data[63]]) != header_checksum(&data[..60])
    {
        return Err(SkinError::InvalidHash);
    }
    let length = data[20] as usize;
    if length == 0 || length > SKIN_ID_BYTES {
        return Err(SkinError::InvalidHash);
    }
    let mut id = [0; SKIN_ID_BYTES];
    id.copy_from_slice(&data[21..53]);
    let package = SkinPackage {
        id: SkinId {
            bytes: id,
            length: length as u8,
        },
        parent: None,
        format_version: u16::from_le_bytes([data[8], data[9]]),
        minimum_ui_abi: u16::from_le_bytes([data[10], data[11]]),
        content_hash: u64::from_le_bytes([
            data[12], data[13], data[14], data[15], data[16], data[17], data[18], data[19],
        ]),
        tokens,
        trusted: data[54] != 0,
    };
    validate_package(&package)?;
    Ok(package)
}

// ------------------------=
// FUNC: header_checksum
// DESC: Produces the deterministic FNV-1a package-header checksum.
// ------------------=
fn header_checksum(bytes: &[u8]) -> u32 {
    let mut hash = 2_166_136_261u32;
    for byte in bytes {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(16_777_619);
    }
    hash
}

// ------------------------=
// FUNC: default_dark_skin
// DESC: Defines the screenshot-accurate official InfinityOS dark visual system.
// ------------------=
pub const fn default_dark_skin() -> SkinPackage {
    SkinPackage {
        id: SkinId::from_bytes(b"infinity.default.dark"),
        parent: None,
        format_version: 1,
        minimum_ui_abi: 1,
        content_hash: 0x9f43_8a5b_8de7_2101,
        trusted: true,
        tokens: SkinTokens {
            colors: [
                Color(0xff02070f),
                Color(0xe60a121d),
                Color(0xf0121d2a),
                Color(0xff33475b),
                Color(0xfff1f5fa),
                Color(0xffaeb8c6),
                Color(0xff20bfff),
                Color(0xff9ce8ff),
                Color(0xffffffff),
                Color(0xff50d890),
                Color(0xffffc857),
                Color(0xffff6170),
                Color(0x99000000),
            ],
            spacing: [4, 8, 12, 16, 20, 24, 32, 48],
            corner_radius: [4, 10, 16, 24],
            panel_insets: Insets {
                top: 32,
                right: 32,
                bottom: 32,
                left: 32,
            },
            default_scale: Scale::ONE,
            motion_duration_ms: [90, 160, 240, 420],
        },
    }
}

// ------------------------=
// FUNC: diagnostic_light_skin
// DESC: Defines an intentionally different alternate package used to prove true skin replacement.
// ------------------=
pub const fn diagnostic_light_skin() -> SkinPackage {
    SkinPackage {
        id: SkinId::from_bytes(b"infinity.diagnostic.light"),
        parent: Some(SkinId::from_bytes(b"infinity.default.dark")),
        format_version: 1,
        minimum_ui_abi: 1,
        content_hash: 0x6b7a_f45c_91e2_1142,
        trusted: true,
        tokens: SkinTokens {
            colors: [
                Color(0xffe8edf3),
                Color(0xfaf8fbff),
                Color(0xffffffff),
                Color(0xff8ca0b4),
                Color(0xff101821),
                Color(0xff526170),
                Color(0xff0a74b9),
                Color(0xff38a9ea),
                Color(0xff062a45),
                Color(0xff16834f),
                Color(0xff8d6500),
                Color(0xffb32436),
                Color(0x66000000),
            ],
            spacing: [4, 8, 12, 16, 20, 24, 32, 48],
            corner_radius: [2, 6, 10, 14],
            panel_insets: Insets {
                top: 28,
                right: 28,
                bottom: 28,
                left: 28,
            },
            default_scale: Scale::ONE,
            motion_duration_ms: [0, 0, 0, 0],
        },
    }
}

// ------------------------=
// FUNC: safe_skin
// DESC: Defines the dependency-free high-contrast recovery skin compiled into InfinityUI.
// ------------------=
pub const fn safe_skin() -> SkinPackage {
    SkinPackage {
        id: SkinId::from_bytes(b"infinity.safe"),
        parent: None,
        format_version: 1,
        minimum_ui_abi: 1,
        content_hash: 0x51af_e001_77c0_0001,
        trusted: true,
        tokens: SkinTokens {
            colors: [
                Color(0xff000000),
                Color(0xff080808),
                Color(0xff101010),
                Color(0xffffffff),
                Color(0xffffffff),
                Color(0xffd8d8d8),
                Color(0xff00d8ff),
                Color(0xffffffff),
                Color(0xffffff00),
                Color(0xff00ff80),
                Color(0xffffff00),
                Color(0xffff4040),
                Color(0xcc000000),
            ],
            spacing: [4, 8, 12, 16, 20, 24, 32, 48],
            corner_radius: [0, 0, 0, 0],
            panel_insets: Insets {
                top: 24,
                right: 24,
                bottom: 24,
                left: 24,
            },
            default_scale: Scale::ONE,
            motion_duration_ms: [0, 0, 0, 0],
        },
    }
}
