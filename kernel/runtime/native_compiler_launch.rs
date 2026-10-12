//! Shared launch-record marshalling for the native compiler and linker.
//! All pointers refer to checked private user memory; no kernel callbacks cross the ABI.
use super::native_memory::{check_user_range, copy_to_user, MemoryError, PageTable};

pub const RECORD_BYTES: usize = 4096;
pub const MAX_ARGUMENTS: usize = 32;

pub struct Invocation<'a> {
    pub arguments: &'a [&'a [u8]],
    pub executable: &'a [u8],
    pub home: &'a [u8],
    pub node: &'a [u8],
    pub process: u32,
    pub session: u32,
    pub heap: u64,
    pub heap_bytes: u64,
}

// ------------------------=
// FUNC: write
// DESC: Validates and atomically copies a bounded compiler launch record into mapped user memory.
// ------------------=
pub fn write<T: PageTable>(tables: &mut T, address: u64, invocation: Invocation<'_>) -> Result<(), MemoryError> {
    let args = invocation.arguments;
    if args.is_empty() || args.len() > MAX_ARGUMENTS || invocation.process == 0 || invocation.session == 0
        || address & 15 != 0 || invocation.heap & 15 != 0 || invocation.heap_bytes == 0 {
        return Err(MemoryError::InvalidAddress);
    }
    let end = address.checked_add(RECORD_BYTES as u64).ok_or(MemoryError::AddressOverflow)?;
    let heap_end = invocation.heap.checked_add(invocation.heap_bytes).ok_or(MemoryError::AddressOverflow)?;
    if invocation.heap < end && heap_end > address { return Err(MemoryError::InvalidAddress); }
    check_user_range(tables, address, RECORD_BYTES, true)?;
    check_user_range(tables, invocation.heap, usize::try_from(invocation.heap_bytes).map_err(|_| MemoryError::AddressOverflow)?, true)?;
    let mut bytes = [0u8; RECORD_BYTES];
    bytes[0..4].copy_from_slice(&1u32.to_le_bytes());
    bytes[4..8].copy_from_slice(&72u32.to_le_bytes());
    bytes[8..12].copy_from_slice(&(args.len() as u32).to_le_bytes());
    bytes[12..16].copy_from_slice(&invocation.process.to_le_bytes());
    bytes[16..20].copy_from_slice(&invocation.session.to_le_bytes());
    bytes[24..32].copy_from_slice(&(address + 72).to_le_bytes());
    bytes[32..40].copy_from_slice(&invocation.heap.to_le_bytes());
    bytes[40..48].copy_from_slice(&invocation.heap_bytes.to_le_bytes());
    let mut next = 72 + (args.len() + 1) * 8;
    for (index, value) in args.iter().enumerate() {
        let pointer = append(&mut bytes, &mut next, address, value)?;
        bytes[72 + index * 8..80 + index * 8].copy_from_slice(&pointer.to_le_bytes());
    }
    for (offset, value) in [(48, invocation.executable), (56, invocation.home), (64, invocation.node)] {
        if value.is_empty() { return Err(MemoryError::InvalidAddress); }
        let pointer = append(&mut bytes, &mut next, address, value)?;
        bytes[offset..offset + 8].copy_from_slice(&pointer.to_le_bytes());
    }
    copy_to_user(tables, address, &bytes)
}

// ------------------------=
// FUNC: append
// DESC: Encodes one NUL-terminated argument without accepting embedded terminators or overflowing the launch page.
// ------------------=
fn append(bytes: &mut [u8; RECORD_BYTES], next: &mut usize, base: u64, value: &[u8]) -> Result<u64, MemoryError> {
    let end = next.checked_add(value.len()).and_then(|n| n.checked_add(1)).ok_or(MemoryError::AddressOverflow)?;
    if end > bytes.len() || value.contains(&0) { return Err(MemoryError::InvalidAddress); }
    let pointer = base + *next as u64;
    bytes[*next..end - 1].copy_from_slice(value);
    *next = end;
    Ok(pointer)
}
