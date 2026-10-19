#[path = "../../kernel/runtime/native_extension_image.rs"]
mod image;

// ------------------------=
// FUNC: verify
// DESC: Exercises real cross-compiled extension metadata and rejects truncation, wrong architectures and writable executable segments.
// ------------------=
fn verify(path: &str, machine: u16) {
    let bytes = std::fs::read(path).unwrap();
    let parsed = image::Image::parse(&bytes, machine).unwrap();
    assert!(parsed.relro.is_some());
    let init = parsed.lookup(b"PyInit_infinity_native_test").unwrap().unwrap();
    assert!(init.defined && init.exported && init.size > 0);
    assert!(parsed.lookup(b"missing_symbol").unwrap().is_none());
    let mut relocations = 0;
    parsed.relocations(|r| {
        assert!(r.address < parsed.memory_bytes);
        if !r.relative { assert!(!parsed.symbol(r.symbol)?.name.is_empty()); }
        relocations += 1;
        Ok(())
    }).unwrap();
    assert!(relocations > 0);
    assert!(image::Image::parse(&bytes, if machine == 183 { 62 } else { 183 }).is_err());
    for length in [0, 6, 16, 63, bytes.len() / 2, bytes.len() - 1] {
        assert!(image::Image::parse(&bytes[..length], machine).is_err());
    }
    let mut corrupt = bytes.clone();
    let ph = u64::from_le_bytes(bytes[32..40].try_into().unwrap()) as usize;
    let count = u16::from_le_bytes(bytes[56..58].try_into().unwrap()) as usize;
    let load = (0..count).map(|i| ph + i * 56)
        .find(|p| u32::from_le_bytes(bytes[*p..*p + 4].try_into().unwrap()) == 1).unwrap();
    corrupt[load + 4..load + 8].copy_from_slice(&7u32.to_le_bytes());
    assert!(matches!(image::Image::parse(&corrupt, machine), Err(image::Error::Permissions)));
    corrupt = bytes.clone();
    corrupt[load + 32..load + 40].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(image::Image::parse(&corrupt, machine).is_err());
    let relro = (0..count).map(|i| ph + i * 56)
        .find(|p| u32::from_le_bytes(bytes[*p..*p + 4].try_into().unwrap()) == 0x6474e552).unwrap();
    let stack = (0..count).map(|i| ph + i * 56)
        .find(|p| u32::from_le_bytes(bytes[*p..*p + 4].try_into().unwrap()) == 0x6474e551).unwrap();
    let executable = (0..count).map(|i| ph + i * 56)
        .find(|p| u32::from_le_bytes(bytes[*p..*p + 4].try_into().unwrap()) == 1
            && u32::from_le_bytes(bytes[*p + 4..*p + 8].try_into().unwrap()) & 1 != 0).unwrap();
    for flags in [1u32, 2, 5, 6, 8] {
        corrupt = bytes.clone();
        corrupt[relro + 4..relro + 8].copy_from_slice(&flags.to_le_bytes());
        assert!(matches!(image::Image::parse(&corrupt, machine), Err(image::Error::Permissions)));
    }
    for length in [0u64, u64::MAX, image::MAX_BYTES as u64] {
        corrupt = bytes.clone();
        corrupt[relro + 40..relro + 48].copy_from_slice(&length.to_le_bytes());
        assert!(matches!(image::Image::parse(&corrupt, machine), Err(image::Error::Bounds)));
    }
    for address in [u64::MAX, image::MAX_BYTES as u64,
        u64::from_le_bytes(bytes[executable + 16..executable + 24].try_into().unwrap())] {
        corrupt = bytes.clone();
        corrupt[relro + 16..relro + 24].copy_from_slice(&address.to_le_bytes());
        corrupt[relro + 40..relro + 48].copy_from_slice(&1u64.to_le_bytes());
        assert!(matches!(image::Image::parse(&corrupt, machine), Err(image::Error::Bounds)));
    }
    corrupt = bytes.clone();
    corrupt[stack..stack + 56].copy_from_slice(&bytes[relro..relro + 56]);
    assert!(matches!(image::Image::parse(&corrupt, machine), Err(image::Error::Permissions)));
    // RELRO is optional, but its presence must never be silently ignored.
    corrupt = bytes.clone();
    corrupt[relro..relro + 4].copy_from_slice(&0u32.to_le_bytes());
    assert!(image::Image::parse(&corrupt, machine).unwrap().relro.is_none());
}

// ------------------------=
// FUNC: main
// DESC: Checks both native architecture artifacts without treating output prose as evidence.
// ------------------=
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3);
    verify(&args[1], 183);
    verify(&args[2], 62);
}
