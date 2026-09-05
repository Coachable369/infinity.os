#[path = "../kernel/storage/component_manifest.rs"]
mod component_manifest;

// ------------------------=
// FUNC: read_u32
// DESC: Decodes one structured little-endian field from the emitted manifest artifact.
// ------------------=
fn read_u32(input: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([input[at], input[at + 1], input[at + 2], input[at + 3]])
}

// ------------------------=
// FUNC: main
// DESC: Proves the full component registry fits, validates, and detects second-sector corruption.
// ------------------=
fn main() {
    let architecture = 3;
    let kernel_crc = 0x1122_3344;
    let kernel_lba = 2048;
    let kernel_bytes = 78_901_234;
    let manifest = component_manifest::encode(
        architecture,
        kernel_crc,
        0x5566_7788,
        kernel_lba,
        kernel_bytes,
        2048,
        134_217_728,
    );
    assert_eq!(manifest.len(), component_manifest::MANIFEST_BYTES);
    assert_eq!(read_u32(&manifest, 16), component_manifest::COMPONENT_COUNT);
    assert_eq!(
        read_u32(&manifest, 24),
        component_manifest::MANIFEST_SECTORS as u32
    );
    let final_entry = 32 + (component_manifest::COMPONENT_COUNT as usize - 1) * 48;
    assert!(final_entry >= 512);
    assert_eq!(read_u32(&manifest, final_entry), 11);
    assert!(component_manifest::validate(
        &manifest,
        architecture,
        kernel_crc,
        kernel_lba,
        kernel_bytes,
    ));

    let mut corrupted = manifest;
    corrupted[700] ^= 0x5a;
    assert!(!component_manifest::validate(
        &corrupted,
        architecture,
        kernel_crc,
        kernel_lba,
        kernel_bytes,
    ));
}
