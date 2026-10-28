//! Per-process native extension mappings. No host loader or ambient file access.
#[path = "native_extension_image.rs"]
mod elf;
use super::native_memory::*;
use super::native_job_files::File;

#[derive(Clone, Copy)]
struct Loaded { file: usize, base: u64, allocation: FrameAllocation, global: bool }

pub struct Libraries { owner: u32, loaded: [Option<Loaded>; 8] }

// ------------------------=
// FUNC: number
// DESC: Decodes bounded executable symbol metadata without trusting section offsets.
// ------------------=
fn number(bytes: &[u8], at: usize, width: usize) -> Option<u64> {
    let field = bytes.get(at..at.checked_add(width)?)?;
    Some(field.iter().enumerate().fold(0, |v, (i,b)| v | ((*b as u64) << (i * 8))))
}

// ------------------------=
// FUNC: executable_export
// DESC: Resolves a defined interpreter export from its immutable ELF without invoking or mapping untrusted extension code.
// ------------------=
fn executable_export(bytes: &[u8], name: &[u8]) -> Option<u64> {
    let sections = number(bytes, 40, 8)? as usize;
    let count = number(bytes, 60, 2)? as usize;
    if count > 256 || number(bytes, 58, 2)? != 64 { return None; }
    bytes.get(sections..sections.checked_add(count * 64)?)?;
    for index in 0..count {
        let section = sections + index * 64;
        if !matches!(number(bytes, section + 4, 4)?, 2 | 11) { continue; }
        let link = number(bytes, section + 40, 4)? as usize;
        if link >= count || number(bytes, section + 56, 8)? != 24 { return None; }
        let offset = number(bytes, section + 24, 8)? as usize;
        let length = number(bytes, section + 32, 8)? as usize;
        if length % 24 != 0 || length / 24 > 65536 { return None; }
        let symbols = bytes.get(offset..offset.checked_add(length)?)?;
        let string_section = sections + link * 64;
        if number(bytes, string_section + 4, 4)? != 3 { return None; }
        let offset = number(bytes, string_section + 24, 8)? as usize;
        let length = number(bytes, string_section + 32, 8)? as usize;
        let strings = bytes.get(offset..offset.checked_add(length)?)?;
        for symbol in symbols.chunks_exact(24) {
            if !matches!(symbol[4] >> 4, 1 | 2) || symbol[4] & 15 > 2 || symbol[5] & 3 != 0
                || number(symbol, 6, 2)? == 0 { continue; }
            let offset = number(symbol, 0, 4)? as usize;
            let rest = strings.get(offset..)?;
            let Some(end) = rest.iter().take(256).position(|b| *b == 0) else { continue; };
            if &rest[..end] != name { continue; }
            let value = number(symbol, 8, 8)?;
            return Some(value);
        }
    }
    None
}

// ------------------------=
// FUNC: executable_symbol
// DESC: Resolves interpreter exports only when their actual calling-process mapping is user accessible.
// ------------------=
fn executable_symbol<T: PageTable>(bytes: &[u8], tables: &T, name: &[u8]) -> Option<u64> {
    let value = executable_export(bytes, name)?;
    tables.mapping(value).filter(|mapping| mapping.permissions.user).map(|_| value)
}

// ------------------------=
// FUNC: validate_python_extension
// DESC: Rejects incompatible architecture, dependencies, TLS, constructors, relocations or unresolved imports before committing a native package transaction.
// ------------------=
pub fn validate_python_extension(main: &[u8], source: &[u8], machine: u16) -> Result<(), i64> {
    let image = elf::Image::parse(source, machine).map_err(|_| -8i64)?;
    image.relocations(|relocation| {
        if relocation.relative { return Ok(()); }
        let symbol = image.symbol(relocation.symbol)?;
        if symbol.defined || symbol.weak || executable_export(main, symbol.name).is_some() { Ok(()) }
        else { Err(elf::Error::Symbol) }
    }).map_err(|_| -8)
}

impl Libraries {
    // ------------------------=
    // FUNC: new
    // DESC: Starts with no native libraries and binds allocations to the calling process identity.
    // ------------------=
    pub const fn new(owner: u32) -> Self { Self { owner, loaded: [None; 8] } }

    // ------------------------=
    // FUNC: syscall
    // DESC: Validates user strings and result slots before loading or resolving an explicitly granted native module.
    // ------------------=
    pub fn syscall<T: PageTable>(&mut self, memory: &mut NativeMemory, tables: &mut T,
        main: &[u8], files: &[File<'_>], machine: u16, operation: u64, args: [u64; 6],
        clean_code: impl FnMut(u64, usize)) -> i64 {
        if !matches!(operation, 0x1230 | 0x1231) { return -88; }
        if check_user_range(tables, args[2], 8, true).is_err() { return -14; }
        let pointer = if operation == 0x1230 { args[0] } else { args[1] };
        let mut name = [0; 256];
        let mut length = 0;
        loop {
            if length == name.len() { return -36; }
            let Some(address) = pointer.checked_add(length as u64) else { return -14; };
            if copy_from_user(tables, address, &mut name[length..length + 1]).is_err() { return -14; }
            if name[length] == 0 { break; }
            length += 1;
        }
        if length == 0 { return -22; }
        let result = if operation == 0x1230 {
            let Some(file) = files.iter().position(|file| file.path == &name[..length]) else { return -2; };
            self.open(memory, tables, main, files, machine, file, args[1], clean_code)
        } else {
            self.symbol(files, machine, args[0], &name[..length])
        };
        match result {
            Ok(value) => copy_to_user(tables, args[2], &value.to_le_bytes()).map_or(-14, |_| 0),
            Err(error) => error,
        }
    }

    // ------------------------=
    // FUNC: resolve
    // DESC: Resolves against the interpreter and explicitly global previously loaded native libraries.
    // ------------------=
    fn resolve<T: PageTable>(&self, main: &[u8], tables: &T, files: &[File<'_>], machine: u16, name: &[u8]) -> Option<u64> {
        if let Some(value) = executable_symbol(main, tables, name) { return Some(value); }
        for entry in self.loaded.iter().flatten().filter(|entry| entry.global) {
            let image = elf::Image::parse(files.get(entry.file)?.bytes, machine).ok()?;
            if let Some(symbol) = image.lookup(name).ok()? { return entry.base.checked_add(symbol.value); }
        }
        None
    }

    // ------------------------=
    // FUNC: open
    // DESC: Relocates an explicitly granted extension in private non-executable storage, then publishes final W^X mappings.
    // ------------------=
    pub fn open<T: PageTable>(&mut self, memory: &mut NativeMemory, tables: &mut T,
        main: &[u8], files: &[File<'_>], machine: u16, file: usize, flags: u64,
        mut clean_code: impl FnMut(u64, usize)) -> Result<u64, i64> {
        if !matches!(flags, 1 | 2 | 5 | 6) { return Err(-22); }
        if let Some((index, entry)) = self.loaded.iter_mut().enumerate()
            .find(|(_, entry)| entry.is_some_and(|entry| entry.file == file)) {
            if flags & 4 != 0 { entry.as_mut().unwrap().global = true; }
            return Ok(index as u64 + 1);
        }
        let slot = self.loaded.iter().position(Option::is_none).ok_or(-24i64)?;
        let source = files.get(file).ok_or(-2i64)?.bytes;
        let image = elf::Image::parse(source, machine).map_err(|_| -8i64)?;
        let base = 0x500000000u64 + slot as u64 * 16 * 1024 * 1024;
        let pages = (image.memory_bytes / 4096) as u32;
        // Check the entire reserved arena before allocating or changing mappings.
        for page in 0..pages {
            if tables.mapping(base + page as u64 * 4096).is_some() { return Err(-13); }
        }
        let allocation = memory.allocate(pages, self.owner).map_err(|_| -12i64)?;
        let destination = unsafe { core::slice::from_raw_parts_mut(allocation.physical_address as *mut u8, image.memory_bytes) };
        for segment in &image.segments[..image.segment_count] {
            destination[segment.address..segment.address + segment.file_bytes]
                .copy_from_slice(&source[segment.offset..segment.offset + segment.file_bytes]);
        }
        let relocated = image.relocations(|relocation| {
            let value = if relocation.relative { base } else {
                let symbol = image.symbol(relocation.symbol)?;
                if symbol.defined { base.checked_add(symbol.value).ok_or(elf::Error::Bounds)? }
                else if let Some(value) = self.resolve(main, tables, files, machine, symbol.name) { value }
                else if symbol.weak { 0 } else { return Err(elf::Error::Symbol); }
            };
            let value = (value as i128).checked_add(relocation.addend as i128)
                .filter(|v| *v >= 0 && *v <= u64::MAX as i128).ok_or(elf::Error::Relocation)? as u64;
            destination[relocation.address..relocation.address + 8].copy_from_slice(&value.to_le_bytes());
            Ok(())
        });
        if relocated.is_err() { memory.release(allocation).map_err(|_| -5i64)?; return Err(-8); }
        let mut mapped = 0;
        for segment in &image.segments[..image.segment_count] {
            let start = segment.address & !4095;
            let end = (segment.address + segment.memory_bytes + 4095) & !4095;
            if segment.flags & 1 != 0 { clean_code(allocation.physical_address + start as u64, end - start); }
            // One page at a time makes rollback exact even when page-table allocation fails.
            for page in (start..end).step_by(4096) {
                // Relocations are already complete in private kernel storage.
                // Publish RELRO with the same outward page rounding as the
                // native executable loader, never as a writable user mapping.
                let readonly = image.relro.is_some_and(|region| page >= (region.address & !4095)
                    && page < ((region.address + region.length + 4095) & !4095));
                let permissions = PagePermissions { user: true,
                    writable: segment.flags & 2 != 0 && !readonly, executable: segment.flags & 1 != 0 };
                if tables.map_range(memory, base + page as u64, allocation.physical_address + page as u64, 4096, permissions).is_err() {
                    for previous in &image.segments[..image.segment_count] {
                        let first = previous.address & !4095;
                        let last = (previous.address + previous.memory_bytes + 4095) & !4095;
                        for old in (first..last).step_by(4096).take(mapped) {
                            tables.unmap_user_range(base + old as u64, 4096).map_err(|_| -5i64)?;
                            mapped -= 1;
                        }
                        if mapped == 0 { break; }
                    }
                    memory.release(allocation).map_err(|_| -5i64)?;
                    return Err(-12);
                }
                mapped += 1;
            }
        }
        self.loaded[slot] = Some(Loaded { file, base, allocation, global: flags & 4 != 0 });
        Ok(slot as u64 + 1)
    }

    // ------------------------=
    // FUNC: symbol
    // DESC: Resolves an exact export from a live process-owned library handle.
    // ------------------=
    pub fn symbol(&self, files: &[File<'_>], machine: u16, handle: u64, name: &[u8]) -> Result<u64, i64> {
        let index = handle.checked_sub(1).filter(|i| *i < 8).ok_or(-9i64)? as usize;
        let entry = self.loaded[index].ok_or(-9i64)?;
        let image = elf::Image::parse(files.get(entry.file).ok_or(-2i64)?.bytes, machine).map_err(|_| -8i64)?;
        let symbol = image.lookup(name).map_err(|_| -8i64)?.ok_or(-2i64)?;
        entry.base.checked_add(symbol.value).ok_or(-8)
    }

    // ------------------------=
    // FUNC: release_all
    // DESC: Reclaims extension frames only after process execution has stopped, preserving live CPython function pointers.
    // ------------------=
    pub fn release_all<T: PageTable>(&mut self, tables: &mut T, memory: &mut NativeMemory) -> Result<(), MemoryError> {
        for entry in &mut self.loaded {
            if let Some(loaded) = *entry {
                for page in 0..loaded.allocation.page_count {
                    let address = loaded.base + page as u64 * 4096;
                    if let Some(mapping) = tables.mapping(address) {
                        if !mapping.permissions.user || mapping.physical_address != loaded.allocation.physical_address + page as u64 * 4096 {
                            return Err(MemoryError::AccessDenied);
                        }
                        tables.unmap_user_range(address, 4096)?;
                    }
                }
                memory.release(loaded.allocation)?;
                *entry = None;
            }
        }
        Ok(())
    }
}
