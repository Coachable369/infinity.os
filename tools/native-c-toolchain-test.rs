use std::env;
use std::fs;
use std::path::Path;

// ------------------------=
// FUNC: read_u16
// DESC: Reads one little-endian ELF half word from a validated byte range.
// ------------------=
fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}

// ------------------------=
// FUNC: read_u32
// DESC: Reads one little-endian ELF word from a validated byte range.
// ------------------=
fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

// ------------------------=
// FUNC: read_u64
// DESC: Reads one little-endian ELF address or offset from a validated byte range.
// ------------------=
fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}

// ------------------------=
// FUNC: verify_artifact
// DESC: Verifies a linked compiler component is a loadable x86-64 ELF executable with static TLS.
// ------------------=
fn verify_artifact(path: &Path) {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    assert!(bytes.len() >= 64, "{}: truncated ELF header", path.display());
    assert_eq!(&bytes[0..4], b"\x7fELF", "{}: bad ELF magic", path.display());
    assert_eq!(bytes[4], 2, "{}: not ELF64", path.display());
    assert_eq!(bytes[5], 1, "{}: not little endian", path.display());
    assert_eq!(read_u16(&bytes, 16), 2, "{}: not ET_EXEC", path.display());
    assert_eq!(read_u16(&bytes, 18), 62, "{}: not x86-64", path.display());
    assert_ne!(read_u64(&bytes, 24), 0, "{}: missing entry point", path.display());

    let program_offset = usize::try_from(read_u64(&bytes, 32)).unwrap();
    let entry_size = usize::from(read_u16(&bytes, 54));
    let entry_count = usize::from(read_u16(&bytes, 56));
    assert!(entry_size >= 56, "{}: malformed program header size", path.display());
    assert!(entry_count > 0, "{}: no program headers", path.display());
    let table_size = entry_size.checked_mul(entry_count).unwrap();
    assert!(program_offset.checked_add(table_size).unwrap() <= bytes.len(), "{}: truncated program table", path.display());

    let mut executable_load = false;
    let mut static_tls = false;
    for index in 0..entry_count {
        let offset = program_offset + index * entry_size;
        let segment_type = read_u32(&bytes, offset);
        let flags = read_u32(&bytes, offset + 4);
        let memory_size = read_u64(&bytes, offset + 40);
        executable_load |= segment_type == 1 && flags & 1 != 0 && memory_size > 0;
        static_tls |= segment_type == 7 && memory_size > 0;
    }
    assert!(executable_load, "{}: no executable load segment", path.display());
    assert!(static_tls, "{}: no static TLS segment", path.display());

    let section_offset = usize::try_from(read_u64(&bytes, 40)).unwrap();
    let section_size = usize::from(read_u16(&bytes, 58));
    let section_count = usize::from(read_u16(&bytes, 60));
    assert!(section_size >= 64, "{}: malformed section header size", path.display());
    let section_table_size = section_size.checked_mul(section_count).unwrap();
    assert!(section_offset.checked_add(section_table_size).unwrap() <= bytes.len(), "{}: truncated section table", path.display());
    let mut symbol_table_found = false;
    let mut unresolved_strong = 0usize;
    for index in 0..section_count {
        let offset = section_offset + index * section_size;
        if read_u32(&bytes, offset + 4) != 2 {
            continue;
        }
        symbol_table_found = true;
        let symbols_offset = usize::try_from(read_u64(&bytes, offset + 24)).unwrap();
        let symbols_size = usize::try_from(read_u64(&bytes, offset + 32)).unwrap();
        let symbol_size = usize::try_from(read_u64(&bytes, offset + 56)).unwrap();
        assert!(symbol_size >= 24, "{}: malformed symbol size", path.display());
        assert!(symbols_offset.checked_add(symbols_size).unwrap() <= bytes.len(), "{}: truncated symbol table", path.display());
        assert_eq!(symbols_size % symbol_size, 0, "{}: partial symbol entry", path.display());
        for symbol in (symbols_offset..symbols_offset + symbols_size).step_by(symbol_size) {
            let binding = bytes[symbol + 4] >> 4;
            let section = read_u16(&bytes, symbol + 6);
            if section == 0 && binding != 0 && binding != 2 {
                unresolved_strong += 1;
            }
        }
    }
    assert!(symbol_table_found, "{}: missing static symbol table", path.display());
    assert_eq!(unresolved_strong, 0, "{}: unresolved strong symbols", path.display());
}

// ------------------------=
// FUNC: main
// DESC: Verifies the exact Clang and LLD artifacts supplied by the reproducible toolchain build.
// ------------------=
fn main() {
    let paths: Vec<_> = env::args_os().skip(1).collect();
    assert_eq!(paths.len(), 2, "expected Clang and LLD artifact paths");
    for path in paths {
        verify_artifact(Path::new(&path));
    }
}
