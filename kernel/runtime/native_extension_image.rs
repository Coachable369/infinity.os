//! Bounded ELF metadata for native CPython extensions. Parsing never executes code.
//! Dynamic libraries must be built for InfinityOS; foreign OS dependencies are rejected.

pub const MAX_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SYMBOLS: usize = 4096;
pub const MAX_RELOCATIONS: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { Header, Architecture, Bounds, Unsupported, Permissions, Symbol, Relocation }

#[derive(Clone, Copy, Default, Debug)]
pub struct Segment {
    pub offset: usize,
    pub address: usize,
    pub file_bytes: usize,
    pub memory_bytes: usize,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region { pub address: usize, pub length: usize }

pub struct Image<'a> {
    bytes: &'a [u8],
    pub machine: u16,
    pub segments: [Segment; 8],
    pub segment_count: usize,
    pub memory_bytes: usize,
    pub relro: Option<Region>,
    sections: usize,
    section_count: usize,
    symbols: usize,
    symbol_count: usize,
    strings: &'a [u8],
    symbol_section: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct Symbol<'a> {
    pub name: &'a [u8],
    pub value: u64,
    pub size: u64,
    pub defined: bool,
    pub weak: bool,
    pub exported: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Relocation {
    pub address: usize,
    pub symbol: usize,
    pub relative: bool,
    pub addend: i64,
}

// ------------------------=
// FUNC: integer
// DESC: Reads one little-endian integer after validating its complete byte range.
// ------------------=
fn integer(bytes: &[u8], offset: usize, width: usize) -> Result<u64, Error> {
    let value = bytes.get(offset..offset.checked_add(width).ok_or(Error::Bounds)?).ok_or(Error::Bounds)?;
    Ok(value.iter().enumerate().fold(0, |v, (i, b)| v | ((*b as u64) << (8 * i))))
}

// ------------------------=
// FUNC: range
// DESC: Bounds a complete file-backed table before any entry is inspected.
// ------------------=
fn range(bytes: &[u8], start: u64, length: u64) -> Result<&[u8], Error> {
    let end = start.checked_add(length).ok_or(Error::Bounds)?;
    if end > bytes.len() as u64 { return Err(Error::Bounds); }
    Ok(&bytes[start as usize..end as usize])
}

impl<'a> Image<'a> {
    // ------------------------=
    // FUNC: parse
    // DESC: Validates native ELF shared objects, disjoint W^X segments, symbol tables and bounded relocations.
    // ------------------=
    pub fn parse(bytes: &'a [u8], machine: u16) -> Result<Self, Error> {
        if bytes.len() > MAX_BYTES || bytes.get(..7) != Some(b"\x7fELF\x02\x01\x01")
            || integer(bytes, 16, 2)? != 3 || integer(bytes, 20, 4)? != 1
            || integer(bytes, 52, 2)? != 64 || integer(bytes, 54, 2)? != 56
            || integer(bytes, 58, 2)? != 64 { return Err(Error::Header); }
        if !matches!(machine, 62 | 183) || integer(bytes, 18, 2)? != machine as u64 {
            return Err(Error::Architecture);
        }
        let programs = integer(bytes, 32, 8)?;
        let program_count = integer(bytes, 56, 2)?;
        if program_count == 0 || program_count > 16 { return Err(Error::Bounds); }
        let programs = range(bytes, programs, program_count * 56)?;
        let sections = integer(bytes, 40, 8)?;
        let section_count = integer(bytes, 60, 2)?;
        if section_count == 0 || section_count > 128 { return Err(Error::Bounds); }
        range(bytes, sections, section_count * 64)?;
        let mut image = Self { bytes, machine, segments: [Segment::default(); 8],
            segment_count: 0, memory_bytes: 0, sections: sections as usize,
            section_count: section_count as usize, symbols: 0, symbol_count: 0,
            strings: &[], symbol_section: 0, relro: None };
        for header in programs.chunks_exact(56) {
            let kind = integer(header, 0, 4)?;
            let flags = integer(header, 4, 4)? as u32;
            let offset = integer(header, 8, 8)?;
            let address = integer(header, 16, 8)?;
            let file_bytes = integer(header, 32, 8)?;
            let memory_bytes = integer(header, 40, 8)?;
            if kind == 3 || kind == 7 || (kind == 0x6474e551 && flags & 1 != 0) {
                return Err(Error::Unsupported);
            }
            if kind == 0x6474e552 {
                if image.relro.is_some() || flags & !4 != 0 { return Err(Error::Permissions); }
                if memory_bytes == 0 || address > MAX_BYTES as u64
                    || memory_bytes > MAX_BYTES as u64 - address { return Err(Error::Bounds); }
                image.relro = Some(Region { address: address as usize, length: memory_bytes as usize });
                continue;
            }
            if kind == 2 {
                if file_bytes % 16 != 0 || file_bytes > 4096 { return Err(Error::Bounds); }
                for dynamic in range(bytes, offset, file_bytes)?.chunks_exact(16) {
                    // Reject foreign dependencies, text relocations, TLS and constructors
                    // until their native lifecycle contracts have been implemented.
                    if matches!(integer(dynamic, 0, 8)?, 1 | 12 | 13 | 22 | 25 | 26 | 32) {
                        return Err(Error::Unsupported);
                    }
                }
            }
            if kind != 1 || memory_bytes == 0 { continue; }
            if image.segment_count == 8 || file_bytes > memory_bytes || address > MAX_BYTES as u64
                || memory_bytes > MAX_BYTES as u64 - address || offset % 4096 != address % 4096 {
                return Err(Error::Bounds);
            }
            if flags & !7 != 0 || flags & 4 == 0 || flags & 3 == 3 { return Err(Error::Permissions); }
            range(bytes, offset, file_bytes)?;
            let start = address & !4095;
            let end = (address + memory_bytes + 4095) & !4095;
            for previous in &image.segments[..image.segment_count] {
                let other_start = previous.address as u64 & !4095;
                let other_end = (previous.address as u64 + previous.memory_bytes as u64 + 4095) & !4095;
                if start < other_end && other_start < end { return Err(Error::Permissions); }
            }
            image.segments[image.segment_count] = Segment { offset: offset as usize, address: address as usize,
                file_bytes: file_bytes as usize, memory_bytes: memory_bytes as usize, flags };
            image.segment_count += 1;
            image.memory_bytes = image.memory_bytes.max(end as usize);
        }
        if image.segment_count == 0 { return Err(Error::Header); }
        // Match the native executable loader: one nonempty interval must be
        // contained in writable, non-executable load storage. Page protection
        // rounds outward only after these byte bounds have been validated.
        if let Some(relro) = image.relro {
            if !image.contains(relro.address as u64, relro.length as u64, true) {
                return Err(Error::Bounds);
            }
        }
        for index in 0..image.section_count {
            let header = image.section(index)?;
            if integer(header, 4, 4)? != 11 { continue; }
            if image.symbol_count != 0 || integer(header, 56, 8)? != 24 { return Err(Error::Symbol); }
            let size = integer(header, 32, 8)?;
            if size == 0 || size % 24 != 0 || size / 24 > MAX_SYMBOLS as u64 { return Err(Error::Symbol); }
            let offset = integer(header, 24, 8)?;
            range(bytes, offset, size)?;
            let strings = image.section(integer(header, 40, 4)? as usize)?;
            if integer(strings, 4, 4)? != 3 { return Err(Error::Symbol); }
            image.strings = range(bytes, integer(strings, 24, 8)?, integer(strings, 32, 8)?)?;
            image.symbols = offset as usize;
            image.symbol_count = (size / 24) as usize;
            image.symbol_section = index;
        }
        if image.symbol_count == 0 { return Err(Error::Symbol); }
        for index in 0..image.symbol_count { image.symbol(index)?; }
        image.relocations(|_| Ok(()))?;
        Ok(image)
    }

    // ------------------------=
    // FUNC: section
    // DESC: Returns one previously bounded section header, rejecting invalid linked indexes.
    // ------------------=
    fn section(&self, index: usize) -> Result<&'a [u8], Error> {
        if index >= self.section_count { return Err(Error::Bounds); }
        Ok(&self.bytes[self.sections + index * 64..self.sections + (index + 1) * 64])
    }

    // ------------------------=
    // FUNC: symbol
    // DESC: Validates symbol names and native data/function types without accepting TLS or resolver execution.
    // ------------------=
    pub fn symbol(&self, index: usize) -> Result<Symbol<'a>, Error> {
        if index >= self.symbol_count { return Err(Error::Symbol); }
        let entry = &self.bytes[self.symbols + index * 24..self.symbols + (index + 1) * 24];
        let start = integer(entry, 0, 4)? as usize;
        let rest = self.strings.get(start..).ok_or(Error::Symbol)?;
        let end = rest.iter().take(256).position(|b| *b == 0).ok_or(Error::Symbol)?;
        let kind = entry[4] & 15;
        let binding = entry[4] >> 4;
        if kind > 2 || binding > 2 { return Err(Error::Unsupported); }
        let section = integer(entry, 6, 2)? as usize;
        if section >= self.section_count { return Err(Error::Symbol); }
        let value = integer(entry, 8, 8)?;
        let size = integer(entry, 16, 8)?;
        if section != 0 && !self.contains(value, size.max(1), false) { return Err(Error::Symbol); }
        Ok(Symbol { name: &rest[..end], value, size, defined: section != 0,
            weak: binding == 2, exported: binding != 0 && entry[5] & 3 == 0 })
    }

    // ------------------------=
    // FUNC: lookup
    // DESC: Resolves only defined exported native symbols with exact bounded names.
    // ------------------=
    pub fn lookup(&self, name: &[u8]) -> Result<Option<Symbol<'a>>, Error> {
        for index in 1..self.symbol_count {
            let symbol = self.symbol(index)?;
            if symbol.defined && symbol.exported && symbol.name == name { return Ok(Some(symbol)); }
        }
        Ok(None)
    }

    // ------------------------=
    // FUNC: contains
    // DESC: Requires an entire symbol or relocation target to belong to an appropriate load segment.
    // ------------------=
    fn contains(&self, address: u64, size: u64, writable: bool) -> bool {
        let Some(end) = address.checked_add(size) else { return false; };
        self.segments[..self.segment_count].iter().any(|s| address >= s.address as u64
            && end <= (s.address + s.memory_bytes) as u64 && (!writable || s.flags & 2 != 0))
    }

    // ------------------------=
    // FUNC: relocations
    // DESC: Visits only bounded native 64-bit data relocations and rejects text writes and unknown relocation kinds.
    // ------------------=
    pub fn relocations(&self, mut visit: impl FnMut(Relocation) -> Result<(), Error>) -> Result<(), Error> {
        let mut count = 0;
        for index in 0..self.section_count {
            let header = self.section(index)?;
            let kind = integer(header, 4, 4)?;
            if integer(header, 8, 8)? & 2 == 0 { continue; }
            if kind == 9 || kind == 19 { return Err(Error::Unsupported); }
            if kind != 4 { continue; }
            if integer(header, 56, 8)? != 24 || integer(header, 40, 4)? as usize != self.symbol_section {
                return Err(Error::Relocation);
            }
            let size = integer(header, 32, 8)?;
            if size % 24 != 0 || size / 24 > MAX_RELOCATIONS as u64 { return Err(Error::Relocation); }
            count += (size / 24) as usize;
            if count > MAX_RELOCATIONS { return Err(Error::Relocation); }
            for entry in range(self.bytes, integer(header, 24, 8)?, size)?.chunks_exact(24) {
                let address = integer(entry, 0, 8)?;
                let info = integer(entry, 8, 8)?;
                let code = info as u32;
                let symbol = (info >> 32) as usize;
                let relative = if self.machine == 183 { code == 1027 } else { code == 8 };
                let supported = if self.machine == 183 { matches!(code, 257 | 1025 | 1026 | 1027) }
                    else { matches!(code, 1 | 6 | 7 | 8) };
                if !supported || address & 7 != 0 || !self.contains(address, 8, true)
                    || symbol >= self.symbol_count || (relative && symbol != 0) { return Err(Error::Relocation); }
                visit(Relocation { address: address as usize, symbol, relative,
                    addend: integer(entry, 16, 8)? as i64 })?;
            }
        }
        Ok(())
    }
}
