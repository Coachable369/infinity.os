//! Bounded parser for installer layouts authored by InfinityOS Installer Studio.

const MAGIC: &[u8; 4] = b"IUIT";
const FORMAT_VERSION: u16 = 3;
const MAX_SCREEN_COUNT: u16 = 32;

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
    pub kind: u8,
    pub role: u8,
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
}

impl<'a> InstallerTemplate<'a> {
    // ------------------------=
    // FUNC: parse
    // DESC: Validates a complete bounded installer template before it can affect runtime UI.
    // ------------------=
    pub fn parse(data: &'a [u8]) -> Result<Self, InstallerTemplateError> {
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
            for _ in 0..count {
                let element = reader.element()?;
                if !element_is_bounded(element.frame) {
                    return Err(InstallerTemplateError::InvalidElement);
                }
                if element.role == InstallerTemplateRole::Console as u8 {
                    console_count = console_count.saturating_add(1);
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
            if back_count != 1 || primary_count != 1 || console_count == 0 {
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
        if !reader.is_at_end() {
            return Err(InstallerTemplateError::TrailingData);
        }
        Ok(Self { data, screen_count })
    }

    // ------------------------=
    // FUNC: screen_count
    // DESC: Reports the validated number of saved installer screens.
    // ------------------=
    pub const fn screen_count(&self) -> u16 {
        self.screen_count
    }

    // ------------------------=
    // FUNC: element
    // DESC: Finds a role in one validated screen without allocation.
    // ------------------=
    pub fn element(&self, target_screen: u8, target_role: InstallerTemplateRole) -> Option<InstallerTemplateElement<'a>> {
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
        let end = self.offset.checked_add(count).ok_or(InstallerTemplateError::Truncated)?;
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
    // FUNC: element
    // DESC: Decodes one bounded element record from the runtime template.
    // ------------------=
    fn element(&mut self) -> Result<InstallerTemplateElement<'a>, InstallerTemplateError> {
        self.take(16)?;
        let kind = self.u8()?;
        let role = self.u8()?;
        let flags = self.u8()?;
        if !(1..=5).contains(&kind) || role > InstallerTemplateRole::Input as u8 || flags & !3 != 0
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
            kind,
            role,
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
