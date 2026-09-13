#[path = "../../kernel/runtime/native_c_image.rs"]
mod image;
use image::{Image, ImageError};

// ------------------------=
// FUNC: main
// DESC: Verifies real cross-compiled TLS ELF templates, ABI alignment, initialized data, zero fill and malformed record rejection.
// ------------------=
fn main() {
    for (arch, machine) in [("x86_64", 62), ("aarch64", 183)] {
        let bytes = std::fs::read(format!("build/native-c/tls-{arch}.elf")).unwrap();
        let image = Image::parse(&bytes, machine).unwrap();
        let template = image.tls.unwrap();
        assert_eq!(template.alignment, 64);
        assert!(template.memory_size > template.file_size);
        let (mut a, mut b) = ([0xa5; 1024], [0xb6; 1024]);
        let first = template.initialize(&bytes, machine, &mut a).unwrap();
        let second = template.initialize(&bytes, machine, &mut b).unwrap();
        assert_ne!(first.thread_pointer(), second.thread_pointer());
        assert_eq!(first.thread_pointer() % 64, 0);
        assert_eq!(first.data(), second.data());
        assert_eq!(
            &first.data()[..template.file_size],
            &bytes[template.source..template.source + template.file_size]
        );
        assert!(first.data()[template.file_size..].iter().all(|b| *b == 0));
        assert!(template.initialize(&bytes, machine, &mut [0; 1]).is_err());
        let mut bad = template;
        bad.alignment = 3;
        assert_eq!(bad.validate(&bytes), Err(ImageError::Bounds));
        bad = template;
        bad.file_size = bad.memory_size + 1;
        assert_eq!(bad.validate(&bytes), Err(ImageError::Bounds));
        bad = template;
        bad.source = usize::MAX;
        assert_eq!(bad.validate(&bytes), Err(ImageError::Bounds));
    }
}
