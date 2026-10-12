#[path = "../kernel/runtime/native_c_image.rs"] mod image;
use image::{Image, ImageError};

// ------------------------=
// FUNC: integer
// DESC: Reads an ELF fixture field independently of the loader implementation.
// ------------------=
fn integer(bytes: &[u8], at: usize) -> usize {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap()) as usize
}

// ------------------------=
// FUNC: main
// DESC: Exercises real relative C callback tables and atomic rejection of forged relocation authority.
// ------------------=
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let machine: u16 = std::env::args().nth(2).unwrap().parse().unwrap();
    let bytes = std::fs::read(path).unwrap();
    let metadata = Image::parse(&bytes, machine).unwrap();
    let relocation = metadata.relative_relocations.unwrap();
    let base = 0x100000000usize;
    let mut memory = vec![0xa5; metadata.memory_size];
    assert_eq!(Image::load(&bytes, machine, &mut memory), Err(ImageError::Unsupported));
    assert!(memory.iter().all(|byte| *byte == 0xa5));
    Image::load_at(&bytes, machine, &mut memory, base).unwrap();
    for at in (relocation.address..relocation.address + relocation.length).step_by(24) {
        assert_eq!(integer(&memory, integer(&bytes, at)), base + integer(&bytes, at + 16));
    }
    for (offset, value) in [(relocation.address, 0u64),
        (relocation.address + 8, (1u64 << 32) | if machine == 62 { 8 } else { 1027 }),
        (relocation.address + 16, u64::MAX),
        (relocation.address + 24, integer(&bytes, relocation.address) as u64)] {
        let mut invalid = bytes.clone();
        invalid[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        memory.fill(0xa5);
        assert!(Image::load_at(&invalid, machine, &mut memory, base).is_err());
        assert!(memory.iter().all(|byte| *byte == 0xa5));
    }
    memory.fill(0xa5);
    assert_eq!(Image::load_at(&bytes, machine, &mut memory, usize::MAX), Err(ImageError::Bounds));
    assert!(memory.iter().all(|byte| *byte == 0xa5));
}
