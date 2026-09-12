#[path = "../../kernel/runtime/native_c_image.rs"] mod image;
use image::{Image, ImageError, MAX_IMAGE};

// ------------------------=
// FUNC: set64
// DESC: Mutates one binary ELF field for structural rejection tests.
// ------------------=
fn set64(bytes: &mut [u8], at: usize, value: u64) { bytes[at..at + 8].copy_from_slice(&value.to_le_bytes()); }

// ------------------------=
// FUNC: main
// DESC: Tests actual Clang ELF images, zero initialization, cross-architecture rejection and malformed inputs.
// ------------------=
fn main() {
    for (path, machine) in [("build/native-c/hello-x86_64.elf", 62), ("build/native-c/hello-aarch64.elf", 183)] {
        let bytes = std::fs::read(path).unwrap();
        let metadata = Image::parse(&bytes, machine).unwrap();
        let mut memory = vec![0xa5; MAX_IMAGE];
        assert_eq!(Image::load(&bytes, machine, &mut memory), Ok(metadata));
        for segment in &metadata.segments[..metadata.segment_count] {
            assert_eq!(&memory[segment.address..segment.address + segment.file_size],
                &bytes[segment.source..segment.source + segment.file_size]);
            assert!(memory[segment.address + segment.file_size..segment.address + segment.memory_size].iter().all(|b| *b == 0));
        }
        assert_eq!(Image::parse(&bytes, if machine == 62 { 183 } else { 62 }), Err(ImageError::Architecture));
        assert_eq!(Image::load(&bytes, machine, &mut [0;1]), Err(ImageError::Bounds));
        for cut in 0..bytes.len().min(4096) { assert!(Image::parse(&bytes[..cut], machine).is_err()); }
        let mut invalid = bytes.clone();
        set64(&mut invalid, 32, u64::MAX);
        assert!(Image::parse(&invalid, machine).is_err());
        invalid.copy_from_slice(&bytes);
        invalid[68..72].copy_from_slice(&7u32.to_le_bytes());
        assert_eq!(Image::parse(&invalid, machine), Err(ImageError::Permissions));
        invalid.copy_from_slice(&bytes);
        set64(&mut invalid, 24, 0x2000);
        assert_eq!(Image::parse(&invalid, machine), Err(ImageError::Entry));
        invalid.copy_from_slice(&bytes);
        set64(&mut invalid, 64 + 56 + 16, 0);
        assert_eq!(Image::parse(&invalid, machine), Err(ImageError::Overlap));
        invalid.copy_from_slice(&bytes);
        set64(&mut invalid, 64 + 40, u64::MAX);
        assert_eq!(Image::parse(&invalid, machine), Err(ImageError::Bounds));
        invalid.copy_from_slice(&bytes);
        invalid[64..68].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(Image::parse(&invalid, machine), Err(ImageError::Unsupported));
    }
}
