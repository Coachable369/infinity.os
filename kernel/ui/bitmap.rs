//! Bounded runtime decoding for bitmap assets embedded by InfinityStudio.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeBitmap<'a> {
    data: &'a [u8],
    pixel_offset: usize,
    width: usize,
    height: usize,
    row_bytes: usize,
    bytes_per_pixel: usize,
    top_down: bool,
    uses_alpha: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeBitmapPlacement {
    pub destination_left: usize,
    pub destination_top: usize,
    pub destination_width: usize,
    pub destination_height: usize,
    pub source_left: usize,
    pub source_top: usize,
    pub source_width: usize,
    pub source_height: usize,
}

impl<'a> RuntimeBitmap<'a> {
    // ------------------------=
    // FUNC: parse
    // DESC: Validates a 24-bit RGB or standard 32-bit BGRA bitmap before framebuffer sampling.
    // ------------------=
    pub fn parse(data: &'a [u8]) -> Option<Self> {
        if data.len() < 54 || &data[0..2] != b"BM" {
            return None;
        }
        let dib_size = little_u32(data, 14)? as usize;
        let pixel_offset = little_u32(data, 10)? as usize;
        let width = little_i32(data, 18)?;
        let signed_height = little_i32(data, 22)?;
        let planes = little_u16(data, 26)?;
        let bits_per_pixel = little_u16(data, 28)?;
        let compression = little_u32(data, 30)?;
        if dib_size < 40 || width <= 0 || signed_height == 0 || planes != 1 {
            return None;
        }
        let (bytes_per_pixel, uses_alpha) = match (bits_per_pixel, compression) {
            (24, 0) => (3usize, false),
            (32, 0) => (4usize, false),
            (32, 3) if standard_bgra_masks(data, dib_size) => (4usize, true),
            _ => return None,
        };
        let width = width as usize;
        let height = signed_height.unsigned_abs() as usize;
        let unpadded = width.checked_mul(bytes_per_pixel)?;
        let row_bytes = unpadded.checked_add(3)? & !3;
        let byte_count = row_bytes.checked_mul(height)?;
        if pixel_offset < 14usize.checked_add(dib_size)?
            || pixel_offset.checked_add(byte_count)? > data.len()
        {
            return None;
        }
        Some(Self {
            data,
            pixel_offset,
            width,
            height,
            row_bytes,
            bytes_per_pixel,
            top_down: signed_height < 0,
            uses_alpha,
        })
    }

    // ------------------------=
    // FUNC: width
    // DESC: Returns the validated source width in pixels.
    // ------------------=
    pub const fn width(self) -> usize {
        self.width
    }

    // ------------------------=
    // FUNC: height
    // DESC: Returns the validated source height in pixels.
    // ------------------=
    pub const fn height(self) -> usize {
        self.height
    }

    // ------------------------=
    // FUNC: placement
    // DESC: Resolves explicit crop plus either aspect-fill or whole-image aspect-fit sampling geometry.
    // ------------------=
    pub fn placement(
        self,
        destination_width: usize,
        destination_height: usize,
        crop: [u8; 4],
        aspect_fill: bool,
    ) -> Option<RuntimeBitmapPlacement> {
        if destination_width == 0 || destination_height == 0 {
            return None;
        }
        let mut source_left = self.width * crop[0] as usize / 100;
        let mut source_top = self.height * crop[1] as usize / 100;
        let mut source_width = self.width
            * (100usize.saturating_sub(crop[0] as usize + crop[2] as usize))
            / 100;
        let mut source_height = self.height
            * (100usize.saturating_sub(crop[1] as usize + crop[3] as usize))
            / 100;
        if source_width == 0 || source_height == 0 {
            return None;
        }
        let mut placement = RuntimeBitmapPlacement {
            destination_left: 0,
            destination_top: 0,
            destination_width,
            destination_height,
            source_left,
            source_top,
            source_width,
            source_height,
        };
        if aspect_fill {
            if source_width * destination_height > source_height * destination_width {
                let fitted_width = source_height * destination_width / destination_height;
                source_left += source_width.saturating_sub(fitted_width) / 2;
                source_width = fitted_width;
            } else {
                let fitted_height = source_width * destination_height / destination_width;
                source_top += source_height.saturating_sub(fitted_height) / 2;
                source_height = fitted_height;
            }
            placement.source_left = source_left;
            placement.source_top = source_top;
            placement.source_width = source_width;
            placement.source_height = source_height;
        } else if source_width * destination_height > source_height * destination_width {
            placement.destination_height =
                (source_height * destination_width / source_width).max(1);
            placement.destination_top =
                destination_height.saturating_sub(placement.destination_height) / 2;
        } else {
            placement.destination_width =
                (source_width * destination_height / source_height).max(1);
            placement.destination_left =
                destination_width.saturating_sub(placement.destination_width) / 2;
        }
        Some(placement)
    }

    // ------------------------=
    // FUNC: rgba
    // DESC: Returns one logical top-down source pixel with preserved Studio alpha.
    // ------------------=
    pub fn rgba(self, x: usize, y: usize) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let source_y = if self.top_down {
            y
        } else {
            self.height.saturating_sub(1 + y)
        };
        let index = self
            .pixel_offset
            .checked_add(source_y.checked_mul(self.row_bytes)?)?
            .checked_add(x.checked_mul(self.bytes_per_pixel)?)?;
        let blue = *self.data.get(index)?;
        let green = *self.data.get(index + 1)?;
        let red = *self.data.get(index + 2)?;
        let alpha = if self.uses_alpha {
            *self.data.get(index + 3)?
        } else {
            255
        };
        Some([red, green, blue, alpha])
    }
}

// ------------------------=
// FUNC: standard_bgra_masks
// DESC: Accepts only the deterministic BGRA bitfield layout emitted by InfinityStudio.
// ------------------=
fn standard_bgra_masks(data: &[u8], dib_size: usize) -> bool {
    dib_size >= 56
        && little_u32(data, 54) == Some(0x00ff_0000)
        && little_u32(data, 58) == Some(0x0000_ff00)
        && little_u32(data, 62) == Some(0x0000_00ff)
        && little_u32(data, 66) == Some(0xff00_0000)
}

// ------------------------=
// FUNC: little_u16
// DESC: Reads one bounded little-endian 16-bit bitmap field.
// ------------------=
fn little_u16(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes([
        *data.get(offset)?,
        *data.get(offset + 1)?,
    ]))
}

// ------------------------=
// FUNC: little_u32
// DESC: Reads one bounded little-endian 32-bit bitmap field.
// ------------------=
fn little_u32(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *data.get(offset)?,
        *data.get(offset + 1)?,
        *data.get(offset + 2)?,
        *data.get(offset + 3)?,
    ]))
}

// ------------------------=
// FUNC: little_i32
// DESC: Reads one bounded signed little-endian 32-bit bitmap field.
// ------------------=
fn little_i32(data: &[u8], offset: usize) -> Option<i32> {
    Some(i32::from_le_bytes(little_u32(data, offset)?.to_le_bytes()))
}
