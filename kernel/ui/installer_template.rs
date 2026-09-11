//! Bounded parser for installer layouts authored by InfinityOS Installer Studio.

use super::bitmap::RuntimeBitmap;

const MAGIC: &[u8; 4] = b"IUIT";
const FORMAT_VERSION: u16 = 5;
const MAX_SCREEN_COUNT: u16 = 32;
const MAX_ASSET_COUNT: u16 = 128;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallerTemplateRole {
    Masthead = 1,
    Console = 2,
    Content = 3,
    Title = 4,
    Body = 5,
    Image = 6,
    BackButton = 7,
    PrimaryButton = 8,
    Footer = 9,
    Input = 10,
    SectionLabel = 11,
    DateField = 12,
    TimeField = 13,
    TimeZoneSelector = 14,
    OffsetBadge = 15,
    TimeZoneMap = 16,
    Metadata = 17,
    ProgressSegment = 18,
    ProgressBar = 19,
    ProgressHero = 20,
    LiveDetails = 21,
    SettingsNavigationItem = 22,
    SettingsNavigationIcon = 23,
    SettingsArtwork = 24,
    SettingsNavigationLabel = 25,
}

// ------------------------=
// FUNC: template_image_uses_aspect_fill
// DESC: Keeps scene-filling wallpaper and map roles edge-to-edge while ordinary artwork remains wholly visible.
// ------------------=
pub const fn template_image_uses_aspect_fill(role: u8) -> bool {
    role == InstallerTemplateRole::Masthead as u8
        || role == InstallerTemplateRole::TimeZoneMap as u8
}

// ------------------------=
// FUNC: installer_live_details
// DESC: Collects bounded runtime rows for the authored details layer, excluding the separate screen heading.
// ------------------=
pub fn installer_live_details(
    lines: &[[u8; 96]; 6],
    lengths: &[usize; 6],
    line_count: usize,
    prompt: &[u8],
    command: &[u8],
) -> ([u8; 768], usize) {
    let mut text = [0u8; 768];
    let mut length = 0usize;
    let start = if line_count > 1 { 1 } else { 0 };
    for row in start..line_count.min(6) {
        if length > 0 {
            text[length] = b'\n';
            length += 1;
        }
        let count = lengths[row].min(96);
        text[length..length + count].copy_from_slice(&lines[row][..count]);
        length += count;
    }
    if !prompt.is_empty() {
        if length > 0 {
            text[length] = b'\n';
            length += 1;
        }
        for bytes in [prompt, command] {
            let count = bytes.len().min(text.len() - length);
            text[length..length + count].copy_from_slice(&bytes[..count]);
            length += count;
        }
    }
    (text, length)
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallerTemplateVariable {
    None = 0,
    MachineNodeName = 1,
    ProfileName = 2,
    DisplayName = 3,
    Password = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstallerTemplateRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstallerTemplateElement<'a> {
    pub id: [u8; 16],
    pub kind: u8,
    pub role: u8,
    pub input_variable: u8,
    pub locked: bool,
    pub hidden: bool,
    pub z_index: i16,
    pub frame: InstallerTemplateRect,
    pub fill: [u8; 4],
    pub border: [u8; 4],
    pub opacity: u8,
    pub corner_radius: u8,
    pub font_size: u16,
    pub name: &'a [u8],
    pub text: &'a [u8],
    pub image_asset: &'a [u8],
    pub crop: [u8; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallerTemplateError {
    Truncated,
    InvalidHeader,
    InvalidVersion,
    InvalidScreenSet,
    InvalidElement,
    InvalidNavigation,
    TrailingData,
}

#[derive(Clone, Copy)]
pub struct InstallerTemplate<'a> {
    data: &'a [u8],
    screen_count: u16,
    asset_count: u16,
}

impl<'a> InstallerTemplate<'a> {
    // ------------------------=
    // FUNC: parse
    // DESC: Validates a complete bounded installer template before it can affect runtime UI.
    // ------------------=
    pub fn parse(data: &'a [u8]) -> Result<Self, InstallerTemplateError> {
        Self::parse_profile(data, true)
    }

    // ------------------------=
    // FUNC: parse_settings
    // DESC: Validates a System Settings design without imposing installer navigation controls.
    // ------------------=
    pub fn parse_settings(data: &'a [u8]) -> Result<Self, InstallerTemplateError> {
        Self::parse_profile(data, false)
    }

    // ------------------------=
    // FUNC: parse_profile
    // DESC: Parses the shared IUIT format against installer or logged-in Settings structural requirements.
    // ------------------=
    fn parse_profile(
        data: &'a [u8],
        requires_navigation: bool,
    ) -> Result<Self, InstallerTemplateError> {
        let mut reader = Reader::new(data);
        if reader.take(4)? != MAGIC {
            return Err(InstallerTemplateError::InvalidHeader);
        }
        if reader.u16()? != FORMAT_VERSION {
            return Err(InstallerTemplateError::InvalidVersion);
        }
        let screen_count = reader.u16()?;
        if screen_count == 0 || screen_count > MAX_SCREEN_COUNT {
            return Err(InstallerTemplateError::InvalidScreenSet);
        }

        let mut seen_screens = 0u32;
        for _ in 0..screen_count {
            let screen = reader.u8()?;
            if screen == 0 || screen as u16 > screen_count {
                return Err(InstallerTemplateError::InvalidScreenSet);
            }
            let screen_bit = 1u32 << (screen - 1);
            if seen_screens & screen_bit != 0 {
                return Err(InstallerTemplateError::InvalidScreenSet);
            }
            seen_screens |= screen_bit;

            let screen_title = reader.short_string()?;
            if screen_title.is_empty() || core::str::from_utf8(screen_title).is_err() {
                return Err(InstallerTemplateError::InvalidScreenSet);
            }
            let count = reader.u16()?;
            let mut back_count = 0u8;
            let mut primary_count = 0u8;
            let mut console_count = 0u8;
            let mut input_count = 0u8;
            let mut details_count = 0u8;
            let mut content_count = 0u8;
            let mut title_count = 0u8;
            let mut body_count = 0u8;
            let mut section_count = 0u8;
            let mut row_count = 0u8;
            let mut settings_navigation_count = 0u8;
            let mut settings_icon_count = 0u8;
            let mut settings_artwork_count = 0u8;
            let mut settings_label_count = 0u8;
            for _ in 0..count {
                let element = reader.element()?;
                if !element_is_bounded(element.frame) {
                    return Err(InstallerTemplateError::InvalidElement);
                }
                if element.role == InstallerTemplateRole::Console as u8 {
                    console_count = console_count.saturating_add(1);
                }
                if element.role == InstallerTemplateRole::Content as u8 {
                    content_count = content_count.saturating_add(1);
                }
                if element.role == InstallerTemplateRole::Title as u8 {
                    title_count = title_count.saturating_add(1);
                }
                if element.role == InstallerTemplateRole::Body as u8 {
                    body_count = body_count.saturating_add(1);
                }
                if element.role == InstallerTemplateRole::SectionLabel as u8 {
                    section_count = section_count.saturating_add(1);
                }
                if element.role == InstallerTemplateRole::Metadata as u8 {
                    row_count = row_count.saturating_add(1);
                }
                if element.role == InstallerTemplateRole::SettingsNavigationItem as u8 {
                    settings_navigation_count = settings_navigation_count.saturating_add(1);
                }
                if element.role == InstallerTemplateRole::SettingsNavigationIcon as u8 {
                    settings_icon_count = settings_icon_count.saturating_add(1);
                }
                if element.role == InstallerTemplateRole::SettingsArtwork as u8 {
                    settings_artwork_count = settings_artwork_count.saturating_add(1);
                }
                if element.role == InstallerTemplateRole::SettingsNavigationLabel as u8 {
                    settings_label_count = settings_label_count.saturating_add(1);
                }
                if element.role == InstallerTemplateRole::Input as u8 && !element.hidden {
                    input_count = input_count.saturating_add(1);
                }
                if element.role == InstallerTemplateRole::LiveDetails as u8 {
                    details_count = details_count.saturating_add(1);
                }
                if element.role == InstallerTemplateRole::BackButton as u8 {
                    back_count = back_count.saturating_add(1);
                    if !functional_button(&element) {
                        return Err(InstallerTemplateError::InvalidNavigation);
                    }
                }
                if element.role == InstallerTemplateRole::PrimaryButton as u8 {
                    primary_count = primary_count.saturating_add(1);
                    if !functional_button(&element) {
                        return Err(InstallerTemplateError::InvalidNavigation);
                    }
                }
            }
            let invalid_installer = requires_navigation
                && (back_count != 1 || primary_count != 1 || console_count == 0);
            let invalid_settings = !requires_navigation
                && (console_count != 1 || content_count != 1 || title_count == 0
                    || body_count == 0 || section_count != 1 || row_count == 0
                    || settings_navigation_count != 11 || settings_icon_count != 11
                    || settings_label_count != 11 || settings_artwork_count != 1);
            if invalid_installer || invalid_settings || input_count > 1 || details_count > 1 {
                return Err(InstallerTemplateError::InvalidNavigation);
            }
        }
        let expected_screens = if screen_count == MAX_SCREEN_COUNT {
            u32::MAX
        } else {
            (1u32 << screen_count) - 1
        };
        if seen_screens != expected_screens {
            return Err(InstallerTemplateError::InvalidScreenSet);
        }
        let asset_count = reader.u16()?;
        if asset_count > MAX_ASSET_COUNT {
            return Err(InstallerTemplateError::InvalidElement);
        }
        for _ in 0..asset_count {
            let name = reader.short_string()?;
            let bytes = reader.long_bytes()?;
            if name.is_empty()
                || core::str::from_utf8(name).is_err()
                || RuntimeBitmap::parse(bytes).is_none()
            {
                return Err(InstallerTemplateError::InvalidElement);
            }
        }
        if !reader.is_at_end() {
            return Err(InstallerTemplateError::TrailingData);
        }
        Ok(Self {
            data,
            screen_count,
            asset_count,
        })
    }

    // ------------------------=
    // FUNC: screen_count
    // DESC: Reports the validated number of saved installer screens.
    // ------------------=
    pub const fn screen_count(&self) -> u16 {
        self.screen_count
    }

    // ------------------------=
    // FUNC: asset
    // DESC: Returns a packaged image by its editor-visible asset path.
    // ------------------=
    pub fn asset(&self, target_name: &[u8]) -> Option<&'a [u8]> {
        let mut reader = Reader::new(self.data);
        reader.take(8).ok()?;
        for _ in 0..self.screen_count {
            reader.u8().ok()?;
            reader.short_string().ok()?;
            let count = reader.u16().ok()?;
            for _ in 0..count {
                reader.element().ok()?;
            }
        }
        let count = reader.u16().ok()?;
        if count != self.asset_count {
            return None;
        }
        for _ in 0..count {
            let name = reader.short_string().ok()?;
            let bytes = reader.long_bytes().ok()?;
            if name == target_name {
                return Some(bytes);
            }
        }
        None
    }

    // ------------------------=
    // FUNC: element
    // DESC: Finds a role in one validated screen without allocation.
    // ------------------=
    pub fn element(
        &self,
        target_screen: u8,
        target_role: InstallerTemplateRole,
    ) -> Option<InstallerTemplateElement<'a>> {
        let mut reader = Reader::new(self.data);
        reader.take(8).ok()?;
        for _ in 0..self.screen_count {
            let screen = reader.u8().ok()?;
            reader.short_string().ok()?;
            let count = reader.u16().ok()?;
            for _ in 0..count {
                let element = reader.element().ok()?;
                if screen == target_screen && element.role == target_role as u8 {
                    return Some(element);
                }
            }
        }
        None
    }

    // ------------------------=
    // FUNC: element_count
    // DESC: Reports the number of saved layers in one validated screen.
    // ------------------=
    pub fn element_count(&self, target_screen: u8) -> Option<u16> {
        let mut reader = Reader::new(self.data);
        reader.take(8).ok()?;
        for _ in 0..self.screen_count {
            let screen = reader.u8().ok()?;
            reader.short_string().ok()?;
            let count = reader.u16().ok()?;
            if screen == target_screen {
                return Some(count);
            }
            for _ in 0..count {
                reader.element().ok()?;
            }
        }
        None
    }

    // ------------------------=
    // FUNC: element_at
    // DESC: Returns one saved screen layer by document order without allocation.
    // ------------------=
    pub fn element_at(
        &self,
        target_screen: u8,
        target_index: u16,
    ) -> Option<InstallerTemplateElement<'a>> {
        let mut reader = Reader::new(self.data);
        reader.take(8).ok()?;
        for _ in 0..self.screen_count {
            let screen = reader.u8().ok()?;
            reader.short_string().ok()?;
            let count = reader.u16().ok()?;
            for index in 0..count {
                let element = reader.element().ok()?;
                if screen == target_screen && index == target_index {
                    return Some(element);
                }
            }
        }
        None
    }

    // ------------------------=
    // FUNC: layer_at
    // DESC: Returns a screen layer in the same z-index and UUID order used by the visual editor.
    // ------------------=
    pub fn layer_at(
        &self,
        target_screen: u8,
        target_layer: u16,
    ) -> Option<InstallerTemplateElement<'a>> {
        let count = self.element_count(target_screen)?;
        if target_layer >= count {
            return None;
        }
        for candidate_index in 0..count {
            let candidate = self.element_at(target_screen, candidate_index)?;
            let mut earlier = 0u16;
            for comparison_index in 0..count {
                let comparison = self.element_at(target_screen, comparison_index)?;
                if comparison.z_index < candidate.z_index
                    || (comparison.z_index == candidate.z_index && comparison.id < candidate.id)
                {
                    earlier = earlier.saturating_add(1);
                }
            }
            if earlier == target_layer {
                return Some(candidate);
            }
        }
        None
    }
}

struct Reader<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a bounded cursor over immutable runtime template bytes.
    // ------------------=
    const fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    // ------------------------=
    // FUNC: is_at_end
    // DESC: Reports whether the parser consumed the full template.
    // ------------------=
    fn is_at_end(&self) -> bool {
        self.offset == self.data.len()
    }

    // ------------------------=
    // FUNC: take
    // DESC: Returns the next bounded byte slice and advances the cursor.
    // ------------------=
    fn take(&mut self, count: usize) -> Result<&'a [u8], InstallerTemplateError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(InstallerTemplateError::Truncated)?;
        if end > self.data.len() {
            return Err(InstallerTemplateError::Truncated);
        }
        let value = &self.data[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    // ------------------------=
    // FUNC: u8
    // DESC: Reads one unsigned byte from the template.
    // ------------------=
    fn u8(&mut self) -> Result<u8, InstallerTemplateError> {
        Ok(self.take(1)?[0])
    }

    // ------------------------=
    // FUNC: u16
    // DESC: Reads one little-endian unsigned 16-bit value.
    // ------------------=
    fn u16(&mut self) -> Result<u16, InstallerTemplateError> {
        let value = self.take(2)?;
        Ok(u16::from_le_bytes([value[0], value[1]]))
    }

    // ------------------------=
    // FUNC: u32
    // DESC: Reads one little-endian unsigned 32-bit value.
    // ------------------=
    fn u32(&mut self) -> Result<u32, InstallerTemplateError> {
        let value = self.take(4)?;
        Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
    }

    // ------------------------=
    // FUNC: i16
    // DESC: Reads one little-endian signed 16-bit value.
    // ------------------=
    fn i16(&mut self) -> Result<i16, InstallerTemplateError> {
        Ok(i16::from_le_bytes(self.u16()?.to_le_bytes()))
    }

    // ------------------------=
    // FUNC: color
    // DESC: Reads one runtime RGBA token.
    // ------------------=
    fn color(&mut self) -> Result<[u8; 4], InstallerTemplateError> {
        let value = self.take(4)?;
        Ok([value[0], value[1], value[2], value[3]])
    }

    // ------------------------=
    // FUNC: short_string
    // DESC: Reads a one-byte length-prefixed string slice.
    // ------------------=
    fn short_string(&mut self) -> Result<&'a [u8], InstallerTemplateError> {
        let count = self.u8()? as usize;
        self.take(count)
    }

    // ------------------------=
    // FUNC: long_string
    // DESC: Reads a two-byte length-prefixed string slice.
    // ------------------=
    fn long_string(&mut self) -> Result<&'a [u8], InstallerTemplateError> {
        let count = self.u16()? as usize;
        self.take(count)
    }

    // ------------------------=
    // FUNC: long_bytes
    // DESC: Reads a four-byte length-prefixed binary asset.
    // ------------------=
    fn long_bytes(&mut self) -> Result<&'a [u8], InstallerTemplateError> {
        let count = self.u32()? as usize;
        self.take(count)
    }

    // ------------------------=
    // FUNC: element
    // DESC: Decodes one bounded element record from the runtime template.
    // ------------------=
    fn element(&mut self) -> Result<InstallerTemplateElement<'a>, InstallerTemplateError> {
        let id_bytes = self.take(16)?;
        let mut id = [0u8; 16];
        id.copy_from_slice(id_bytes);
        let kind = self.u8()?;
        let role = self.u8()?;
        let input_variable = self.u8()?;
        let flags = self.u8()?;
        if !(1..=6).contains(&kind)
            || role > InstallerTemplateRole::SettingsNavigationLabel as u8
            || input_variable > InstallerTemplateVariable::Password as u8
            || (kind == 6 && role != InstallerTemplateRole::ProgressBar as u8)
            || (role == InstallerTemplateRole::ProgressBar as u8 && kind != 6)
            || (role == InstallerTemplateRole::ProgressHero as u8 && kind != 2)
            || (role == InstallerTemplateRole::LiveDetails as u8 && kind != 3)
            || (role == InstallerTemplateRole::SettingsNavigationItem as u8 && kind != 1)
            || (role == InstallerTemplateRole::SettingsNavigationIcon as u8 && kind != 2)
            || (role == InstallerTemplateRole::SettingsArtwork as u8 && kind != 2)
            || (role == InstallerTemplateRole::SettingsNavigationLabel as u8 && kind != 3)
            || (role != InstallerTemplateRole::Input as u8
                && input_variable != InstallerTemplateVariable::None as u8)
            || (role == InstallerTemplateRole::Input as u8
                && flags & 2 == 0
                && input_variable == InstallerTemplateVariable::None as u8)
            || flags & !3 != 0
        {
            return Err(InstallerTemplateError::InvalidElement);
        }
        let z_index = self.i16()?;
        let frame = InstallerTemplateRect {
            x: self.u16()?,
            y: self.u16()?,
            width: self.u16()?,
            height: self.u16()?,
        };
        let fill = self.color()?;
        let border = self.color()?;
        let opacity = self.u8()?;
        let corner_radius = self.u8()?;
        let font_size = self.u16()?;
        if opacity > 100 || font_size < 6 {
            return Err(InstallerTemplateError::InvalidElement);
        }
        let name = self.short_string()?;
        let text = self.long_string()?;
        let image_asset = self.short_string()?;
        let crop = [self.u8()?, self.u8()?, self.u8()?, self.u8()?];
        if core::str::from_utf8(name).is_err()
            || core::str::from_utf8(text).is_err()
            || core::str::from_utf8(image_asset).is_err()
        {
            return Err(InstallerTemplateError::InvalidElement);
        }
        if crop[0] > 90
            || crop[1] > 90
            || crop[2] > 90
            || crop[3] > 90
            || crop[0] as u16 + crop[2] as u16 > 95
            || crop[1] as u16 + crop[3] as u16 > 95
        {
            return Err(InstallerTemplateError::InvalidElement);
        }
        Ok(InstallerTemplateElement {
            id,
            kind,
            role,
            input_variable,
            locked: flags & 1 != 0,
            hidden: flags & 2 != 0,
            z_index,
            frame,
            fill,
            border,
            opacity,
            corner_radius,
            font_size,
            name,
            text,
            image_asset,
            crop,
        })
    }
}

// ------------------------=
// FUNC: element_is_bounded
// DESC: Ensures normalized geometry stays inside the 1000 by 1000 artboard.
// ------------------=
fn element_is_bounded(frame: InstallerTemplateRect) -> bool {
    frame.width >= 20
        && frame.height >= 20
        && frame.x as u32 + frame.width as u32 <= 1000
        && frame.y as u32 + frame.height as u32 <= 1000
}

// ------------------------=
// FUNC: functional_button
// DESC: Preserves a visible installer navigation action while allowing authored lock and geometry state.
// ------------------=
fn functional_button(element: &InstallerTemplateElement<'_>) -> bool {
    element.kind == 5 && !element.hidden
}
