//! Checked loader for the first Clang/LLD static-position-independent C ABI.
//! Parsing never executes code. Execution/isolation is owned by the caller.
pub const MAX_SEGMENTS: usize = 8;
pub const MAX_IMAGE: usize = 256 * 1024;
pub const MAX_COMPILER_IMAGE: usize = 128 * 1024 * 1024;
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Region {
    pub address: usize,
    pub length: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Image {
    pub entry: usize,
    pub memory_size: usize,
    pub virtual_base: usize,
    pub segments: [Segment; MAX_SEGMENTS],
    pub segment_count: usize,
    pub tls: Option<tls::Template>,
    pub relro: Option<Region>,
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
        Self::parse_with_policy(bytes, machine, false)
    }

    // ------------------------=
    // FUNC: parse_compiler
    // DESC: Validates a bounded fixed-address native compiler image and normalizes its virtual addresses for isolated loading.
    // ------------------=
    pub fn parse_compiler(bytes: &[u8], machine: u16) -> Result<Self, ImageError> {
        Self::parse_with_policy(bytes, machine, true)
    }

    // ------------------------=
    // FUNC: parse_with_policy
    // DESC: Applies distinct executable type and memory limits without weakening the ordinary native application contract.
    // ------------------=
    fn parse_with_policy(bytes: &[u8], machine: u16, compiler: bool) -> Result<Self, ImageError> {
        let expected_type = if compiler { 2 } else { 3 };
        let memory_limit = if compiler { MAX_COMPILER_IMAGE } else { MAX_IMAGE };
        if bytes.len() > memory_limit
            || bytes.get(..7) != Some(b"\x7fELF\x02\x01\x01")
            || integer(bytes, 16, 2)? != expected_type
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
        let raw_entry = index(bytes, 24)?;
        let mut raw_segments = [Segment::default(); MAX_SEGMENTS];
        let mut raw_segment_count = 0usize;
        let mut virtual_base = usize::MAX;
        let mut virtual_end = 0usize;
        for number in 0..count {
            let p = table + number * 56;
            if integer(bytes, p, 4)? != 1 { continue; }
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
            if segment.file_size > segment.memory_size || file_end > bytes.len()
                || (!compiler && memory_end > memory_limit) {
                return Err(ImageError::Bounds);
            }
            if segment.memory_size == 0 { continue; }
            for previous in &raw_segments[..raw_segment_count] {
                if segment.address < previous.address + previous.memory_size && previous.address < memory_end {
                    return Err(ImageError::Overlap);
                }
            }
            virtual_base = virtual_base.min(segment.address);
            virtual_end = virtual_end.max(memory_end);
            raw_segments[raw_segment_count] = segment;
            raw_segment_count += 1;
        }
        if raw_segment_count == 0 || virtual_base == usize::MAX
            || virtual_end.checked_sub(virtual_base).is_none_or(|span| span > memory_limit) {
            return Err(ImageError::Bounds);
        }
        let load_bias = if compiler { virtual_base } else { 0 };
        let entry = raw_entry.checked_sub(load_bias).ok_or(ImageError::Entry)?;
        let mut image = Self { entry, memory_size: virtual_end - load_bias, virtual_base: load_bias,
            segments: [Segment::default(); MAX_SEGMENTS], segment_count: 0, tls: None, relro: None };
        let mut tls_seen = false;
        for number in 0..count {
            let p = table + number * 56;
            match integer(bytes, p, 4)? {
                0 | 6 => continue,
                1 => continue,
                7 => {
                    if tls_seen { return Err(ImageError::Unsupported); }
                    tls_seen = true;
                    let template = tls::Template { source: index(bytes, p + 8)?,
                        address: index(bytes, p + 16)?, file_size: index(bytes, p + 32)?,
                        memory_size: index(bytes, p + 40)?, alignment: index(bytes, p + 48)?.max(1) };
                    template.validate(bytes)?;
                    if integer(bytes, p + 4, 4)? & !6 != 0 { return Err(ImageError::Permissions); }
                    if template.memory_size == 0 { continue; }
                    image.tls = Some(TemplateAddress::normalize(template, load_bias)?);
                    continue;
                }
                0x6474e551 if compiler => {
                    if integer(bytes, p + 4, 4)? & 1 != 0 || index(bytes, p + 32)? != 0
                        || index(bytes, p + 40)? != 0 { return Err(ImageError::Permissions); }
                    continue;
                }
                0x6474e552 if compiler => {
                    if image.relro.is_some() || integer(bytes, p + 4, 4)? & !4 != 0 {
                        return Err(ImageError::Permissions);
                    }
                    let address = index(bytes, p + 16)?;
                    let length = index(bytes, p + 40)?;
                    let end = address.checked_add(length).ok_or(ImageError::Bounds)?;
                    if length == 0 || !raw_segments[..raw_segment_count].iter().any(|segment| {
                        segment.flags & 2 != 0 && address >= segment.address
                            && end <= segment.address + segment.memory_size
                    }) {
                        return Err(ImageError::Bounds);
                    }
                    image.relro = Some(Region {
                        address: address.checked_sub(load_bias).ok_or(ImageError::Bounds)?,
                        length,
                    });
                    continue;
                }
                // PT_DYNAMIC, PT_INTERP and unknown records require later ABI revisions.
                _ => return Err(ImageError::Unsupported),
            }
        }
        for raw in &raw_segments[..raw_segment_count] {
            let mut segment = *raw;
            segment.address = segment.address.checked_sub(load_bias).ok_or(ImageError::Bounds)?;
            image.segments[image.segment_count] = segment;
            image.segment_count += 1;
        }
        if let Some(tls) = image.tls {
            if tls.file_size != 0 && !image.segments[..image.segment_count].iter().any(|s|
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
        Self::copy_into(bytes, destination, image)
    }

    // ------------------------=
    // FUNC: load_compiler
    // DESC: Loads a validated compiler executable into caller-owned isolated memory using normalized segment offsets.
    // ------------------=
    pub fn load_compiler(bytes: &[u8], machine: u16, destination: &mut [u8]) -> Result<Self, ImageError> {
        let image = Self::parse_compiler(bytes, machine)?;
        Self::copy_into(bytes, destination, image)
    }

    // ------------------------=
    // FUNC: copy_into
    // DESC: Copies validated segments and zeroes their uninitialized tails without retaining the source image.
    // ------------------=
    fn copy_into(bytes: &[u8], destination: &mut [u8], image: Self) -> Result<Self, ImageError> {
        if destination.len() < image.memory_size { return Err(ImageError::Bounds); }
        destination[..image.memory_size].fill(0);
        for segment in &image.segments[..image.segment_count] {
            destination[segment.address..segment.address + segment.file_size]
                .copy_from_slice(&bytes[segment.source..segment.source + segment.file_size]);
        }
        Ok(image)
    }
}

struct TemplateAddress;

impl TemplateAddress {
    // ------------------------=
    // FUNC: normalize
    // DESC: Converts a validated TLS virtual address to the same isolated-memory coordinate system as load segments.
    // ------------------=
    fn normalize(mut template: tls::Template, load_bias: usize) -> Result<tls::Template, ImageError> {
        template.address = if template.file_size == 0 {
            0
        } else {
            template.address.checked_sub(load_bias).ok_or(ImageError::Bounds)?
        };
        Ok(template)
    }
}
