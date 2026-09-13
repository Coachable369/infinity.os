//! Checked loader for the first Clang/LLD static-position-independent C ABI.
//! Parsing never executes code. Execution/isolation is owned by the caller.
pub const MAX_SEGMENTS: usize = 8;
pub const MAX_IMAGE: usize = 256 * 1024;
#[path = "native_tls.rs"]
pub mod tls;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageError { Header, Architecture, Unsupported, Bounds, Overlap, Permissions, Entry }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Segment {
    pub source: usize,
    pub address: usize,
    pub file_size: usize,
    pub memory_size: usize,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Image {
    pub entry: usize,
    pub memory_size: usize,
    pub segments: [Segment; MAX_SEGMENTS],
    pub segment_count: usize,
    pub tls: Option<tls::Template>,
}

// ------------------------=
// FUNC: integer
// DESC: Decodes a bounded little-endian ELF integer without unchecked indexing.
// ------------------=
fn integer(bytes: &[u8], at: usize, width: usize) -> Result<u64, ImageError> {
    let value = bytes.get(at..at.checked_add(width).ok_or(ImageError::Bounds)?)
        .ok_or(ImageError::Bounds)?;
    Ok(value.iter().enumerate().fold(0, |sum, (i, byte)| sum | (u64::from(*byte) << (i * 8))))
}

// ------------------------=
// FUNC: index
// DESC: Rejects ELF addresses that do not fit the native address width.
// ------------------=
fn index(bytes: &[u8], at: usize) -> Result<usize, ImageError> {
    usize::try_from(integer(bytes, at, 8)?).map_err(|_| ImageError::Bounds)
}

impl Image {
    // ------------------------=
    // FUNC: parse
    // DESC: Validates a bounded ELF64 ET_DYN image with disjoint non-WX segments and no dynamic runtime.
    // ------------------=
    pub fn parse(bytes: &[u8], machine: u16) -> Result<Self, ImageError> {
        if bytes.get(..7) != Some(b"\x7fELF\x02\x01\x01") || integer(bytes, 16, 2)? != 3
            || integer(bytes, 20, 4)? != 1 || integer(bytes, 52, 2)? != 64 {
            return Err(ImageError::Header);
        }
        if !matches!(machine, 62 | 183) || integer(bytes, 18, 2)? != u64::from(machine) {
            return Err(ImageError::Architecture);
        }
        let count = integer(bytes, 56, 2)? as usize;
        let table = index(bytes, 32)?;
        if count == 0 || count > MAX_SEGMENTS || table < 64 || integer(bytes, 54, 2)? != 56 {
            return Err(ImageError::Unsupported);
        }
        let end = table.checked_add(count * 56).ok_or(ImageError::Bounds)?;
        if end > bytes.len() { return Err(ImageError::Bounds); }
        let sections = index(bytes, 40)?;
        let section_count = integer(bytes, 60, 2)? as usize;
        if section_count != 0 {
            if sections < 64 || integer(bytes, 58, 2)? != 64
                || integer(bytes, 62, 2)? as usize >= section_count {
                return Err(ImageError::Header);
            }
            if sections.checked_add(section_count * 64).ok_or(ImageError::Bounds)? > bytes.len() {
                return Err(ImageError::Bounds);
            }
            for number in 0..section_count {
                let section = sections + number * 64;
                if matches!(integer(bytes, section + 4, 4)?, 4 | 9) && index(bytes, section + 32)? != 0 {
                    return Err(ImageError::Unsupported);
                }
            }
        } else if sections != 0 { return Err(ImageError::Unsupported); }
        let mut image = Self { entry: index(bytes, 24)?, memory_size: 0,
            segments: [Segment::default(); MAX_SEGMENTS], segment_count: 0, tls: None };
        let mut tls_seen = false;
        for number in 0..count {
            let p = table + number * 56;
            match integer(bytes, p, 4)? {
                0 | 6 => continue,
                1 => (),
                7 => {
                    if tls_seen { return Err(ImageError::Unsupported); }
                    tls_seen = true;
                    let template = tls::Template { source: index(bytes, p + 8)?,
                        address: index(bytes, p + 16)?, file_size: index(bytes, p + 32)?,
                        memory_size: index(bytes, p + 40)?, alignment: index(bytes, p + 48)?.max(1) };
                    template.validate(bytes)?;
                    if integer(bytes, p + 4, 4)? & !6 != 0 { return Err(ImageError::Permissions); }
                    if template.memory_size == 0 { continue; }
                    image.tls = Some(template);
                    continue;
                }
                // PT_DYNAMIC, PT_INTERP and unknown records require later ABI revisions.
                _ => return Err(ImageError::Unsupported),
            }
            let segment = Segment { source: index(bytes, p + 8)?, address: index(bytes, p + 16)?,
                file_size: index(bytes, p + 32)?, memory_size: index(bytes, p + 40)?,
                flags: integer(bytes, p + 4, 4)? as u32 };
            let alignment = index(bytes, p + 48)?;
            if segment.flags & !7 != 0 || segment.flags & 4 == 0 || segment.flags & 3 == 3 {
                return Err(ImageError::Permissions);
            }
            if alignment > 1 && (!alignment.is_power_of_two()
                || segment.source % alignment != segment.address % alignment
                || alignment > 4096) { return Err(ImageError::Bounds); }
            let memory_end = segment.address.checked_add(segment.memory_size).ok_or(ImageError::Bounds)?;
            let file_end = segment.source.checked_add(segment.file_size).ok_or(ImageError::Bounds)?;
            if segment.file_size > segment.memory_size || memory_end > MAX_IMAGE || file_end > bytes.len() {
                return Err(ImageError::Bounds);
            }
            if segment.memory_size == 0 { continue; }
            for previous in &image.segments[..image.segment_count] {
                if segment.address < previous.address + previous.memory_size && previous.address < memory_end {
                    return Err(ImageError::Overlap);
                }
            }
            image.memory_size = image.memory_size.max(memory_end);
            image.segments[image.segment_count] = segment;
            image.segment_count += 1;
        }
        if let Some(tls) = image.tls {
            if !image.segments[..image.segment_count].iter().any(|s|
                tls.address >= s.address && tls.address + tls.file_size <= s.address + s.file_size
                && tls.source >= s.source && tls.source - s.source == tls.address - s.address) {
                return Err(ImageError::Bounds);
            }
        }
        if !image.segments[..image.segment_count].iter().any(|s| s.flags & 1 != 0
            && image.entry >= s.address && image.entry < s.address + s.file_size) {
            return Err(ImageError::Entry);
        }
        Ok(image)
    }

    // ------------------------=
    // FUNC: load
    // DESC: Revalidates the immutable image before copying initialized data and zeroing BSS into caller-owned memory.
    // ------------------=
    pub fn load(bytes: &[u8], machine: u16, destination: &mut [u8]) -> Result<Self, ImageError> {
        let image = Self::parse(bytes, machine)?;
        if destination.len() < image.memory_size { return Err(ImageError::Bounds); }
        destination[..image.memory_size].fill(0);
        for segment in &image.segments[..image.segment_count] {
            destination[segment.address..segment.address + segment.file_size]
                .copy_from_slice(&bytes[segment.source..segment.source + segment.file_size]);
        }
        Ok(image)
    }
}
