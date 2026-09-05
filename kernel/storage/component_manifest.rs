//! Versioned multi-sector System Generation component manifest encoding.

pub const MANIFEST_BYTES: usize = 1024;
pub const MANIFEST_SECTORS: usize = MANIFEST_BYTES / 512;
pub const COMPONENT_COUNT: u32 = 11;
const MANIFEST_VERSION: u32 = 2;
const HEADER_BYTES: usize = 32;
const ENTRY_BYTES: usize = 48;
const CRC_OFFSET: usize = MANIFEST_BYTES - 4;
const INSTALL_CLASS_CORE: u32 = 1;

#[derive(Clone, Copy)]
struct ComponentRegistration {
    id: u32,
    kind: u32,
    architecture_specific: bool,
    reference_kind: u32,
}

// The installer consumes this single registry. Statically linked components
// deliberately share the verified installed-kernel extent.
const SYSTEM_COMPONENT_REGISTRY: [ComponentRegistration; COMPONENT_COUNT as usize] = [
    ComponentRegistration {
        id: 1,
        kind: 1,
        architecture_specific: true,
        reference_kind: 1,
    },
    ComponentRegistration {
        id: 2,
        kind: 2,
        architecture_specific: true,
        reference_kind: 2,
    },
    ComponentRegistration {
        id: 3,
        kind: 3,
        architecture_specific: false,
        reference_kind: 1,
    },
    ComponentRegistration {
        id: 4,
        kind: 4,
        architecture_specific: false,
        reference_kind: 1,
    },
    ComponentRegistration {
        id: 5,
        kind: 5,
        architecture_specific: false,
        reference_kind: 1,
    },
    ComponentRegistration {
        id: 6,
        kind: 6,
        architecture_specific: false,
        reference_kind: 1,
    },
    ComponentRegistration {
        id: 7,
        kind: 7,
        architecture_specific: false,
        reference_kind: 1,
    },
    ComponentRegistration {
        id: 8,
        kind: 8,
        architecture_specific: false,
        reference_kind: 1,
    },
    ComponentRegistration {
        id: 9,
        kind: 9,
        architecture_specific: false,
        reference_kind: 1,
    },
    ComponentRegistration {
        id: 10,
        kind: 10,
        architecture_specific: false,
        reference_kind: 1,
    },
    ComponentRegistration {
        id: 11,
        kind: 11,
        architecture_specific: false,
        reference_kind: 1,
    },
];

// ------------------------=
// FUNC: encode
// DESC: Encodes every required System Generation component into a bounded two-sector record.
// ------------------=
pub fn encode(
    architecture: u32,
    kernel_crc: u32,
    esp_crc: u32,
    kernel_lba: u64,
    kernel_bytes: u64,
    esp_lba: u64,
    esp_bytes: u64,
) -> [u8; MANIFEST_BYTES] {
    let mut output = [0u8; MANIFEST_BYTES];
    output[..8].copy_from_slice(b"INFCOMP1");
    put_u32(&mut output, 8, MANIFEST_VERSION);
    put_u32(&mut output, 12, MANIFEST_BYTES as u32);
    put_u32(&mut output, 16, COMPONENT_COUNT);
    put_u32(&mut output, 20, ENTRY_BYTES as u32);
    put_u32(&mut output, 24, MANIFEST_SECTORS as u32);
    for (index, entry) in SYSTEM_COMPONENT_REGISTRY.iter().enumerate() {
        let esp = entry.reference_kind == 2;
        write_component(
            &mut output,
            index,
            entry.id,
            entry.kind,
            if entry.architecture_specific {
                architecture
            } else {
                0
            },
            if esp { esp_crc } else { kernel_crc },
            entry.reference_kind,
            if esp { esp_lba } else { kernel_lba },
            if esp { esp_bytes } else { kernel_bytes },
        );
    }
    put_u32(&mut output, CRC_OFFSET, 0);
    let checksum = crc32(&output);
    put_u32(&mut output, CRC_OFFSET, checksum);
    output
}

// ------------------------=
// FUNC: validate
// DESC: Validates manifest bounds, checksum, architecture, required flags, and installed-kernel declaration.
// ------------------=
pub fn validate(
    input: &[u8; MANIFEST_BYTES],
    architecture: u32,
    kernel_crc: u32,
    kernel_lba: u64,
    kernel_bytes: u64,
) -> bool {
    if &input[..8] != b"INFCOMP1"
        || get_u32(input, 8) != MANIFEST_VERSION
        || get_u32(input, 12) != MANIFEST_BYTES as u32
        || get_u32(input, 16) != COMPONENT_COUNT
        || get_u32(input, 20) != ENTRY_BYTES as u32
        || get_u32(input, 24) != MANIFEST_SECTORS as u32
        || !valid_checksum(input)
    {
        return false;
    }
    let mut kernel_declared = false;
    for index in 0..COMPONENT_COUNT as usize {
        let Some(at) = HEADER_BYTES.checked_add(index.saturating_mul(ENTRY_BYTES)) else {
            return false;
        };
        if at
            .checked_add(ENTRY_BYTES)
            .is_none_or(|end| end > CRC_OFFSET)
            || get_u32(input, at + 16) != INSTALL_CLASS_CORE
            || get_u32(input, at + 20) & 1 == 0
        {
            return false;
        }
        let entry_architecture = get_u32(input, at + 12);
        if entry_architecture != 0 && entry_architecture != architecture {
            return false;
        }
        if get_u32(input, at) == 1
            && get_u32(input, at + 24) == kernel_crc
            && get_u64(input, at + 32) == kernel_lba
            && get_u64(input, at + 40) == kernel_bytes
        {
            kernel_declared = true;
        }
    }
    kernel_declared
}

// ------------------------=
// FUNC: write_component
// DESC: Writes one fixed-width required component entry after proving it fits before the record checksum.
// ------------------=
fn write_component(
    output: &mut [u8; MANIFEST_BYTES],
    index: usize,
    id: u32,
    kind: u32,
    architecture: u32,
    checksum: u32,
    reference_kind: u32,
    lba: u64,
    bytes: u64,
) {
    let at = HEADER_BYTES + index * ENTRY_BYTES;
    debug_assert!(at + ENTRY_BYTES <= CRC_OFFSET);
    put_u32(output, at, id);
    put_u32(output, at + 4, kind);
    put_u32(output, at + 8, 1);
    put_u32(output, at + 12, architecture);
    put_u32(output, at + 16, INSTALL_CLASS_CORE);
    put_u32(output, at + 20, 1);
    put_u32(output, at + 24, checksum);
    put_u32(output, at + 28, reference_kind);
    put_u64(output, at + 32, lba);
    put_u64(output, at + 40, bytes);
}

// ------------------------=
// FUNC: valid_checksum
// DESC: Verifies the record checksum without mutating the caller's bytes.
// ------------------=
fn valid_checksum(input: &[u8; MANIFEST_BYTES]) -> bool {
    let expected = get_u32(input, CRC_OFFSET);
    let mut copy = *input;
    put_u32(&mut copy, CRC_OFFSET, 0);
    crc32(&copy) == expected
}

// ------------------------=
// FUNC: crc32
// DESC: Calculates the IEEE CRC-32 used by native System Generation records.
// ------------------=
fn crc32(input: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in input {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb8_8320 } else { 0 };
        }
    }
    !crc
}

// ------------------------=
// FUNC: put_u32
// DESC: Writes one little-endian 32-bit manifest field.
// ------------------=
fn put_u32(output: &mut [u8], at: usize, value: u32) {
    output[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

// ------------------------=
// FUNC: put_u64
// DESC: Writes one little-endian 64-bit manifest field.
// ------------------=
fn put_u64(output: &mut [u8], at: usize, value: u64) {
    output[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

// ------------------------=
// FUNC: get_u32
// DESC: Reads one little-endian 32-bit manifest field.
// ------------------=
fn get_u32(input: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([input[at], input[at + 1], input[at + 2], input[at + 3]])
}

// ------------------------=
// FUNC: get_u64
// DESC: Reads one little-endian 64-bit manifest field.
// ------------------=
fn get_u64(input: &[u8], at: usize) -> u64 {
    u64::from_le_bytes([
        input[at],
        input[at + 1],
        input[at + 2],
        input[at + 3],
        input[at + 4],
        input[at + 5],
        input[at + 6],
        input[at + 7],
    ])
}
