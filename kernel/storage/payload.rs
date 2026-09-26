//! Bounded installation transfers independent of executable and FAT file sizes.
use super::BlockDevice;
pub struct Image {
    pub bytes: &'static [u8],
    pub length: usize,
    pub crc: u32,
    pub slot: u32,
    pub sha256: [u8; 32],
}
static mut BRIDGE: u64 = 0;
// ------------------------=
// FUNC: initialize
// DESC: Retains the bootloader's target-specific bounded payload read capability.
// ------------------=
pub fn initialize(address: u64) {
    unsafe {
        BRIDGE = address;
    }
}
impl Image {
    // ------------------------=
    // FUNC: embedded
    // DESC: Preserves the legacy in-executable payload for non-streaming builds.
    // ------------------=
    pub const fn embedded(bytes: &'static [u8]) -> Self {
        Self {
            bytes,
            length: bytes.len(),
            crc: 0,
            slot: 0,
            sha256: [0; 32],
        }
    }
    // ------------------------=
    // FUNC: len
    // DESC: Returns the complete logical payload size, not its shard size.
    // ------------------=
    pub const fn len(&self) -> usize {
        self.length
    }
    // ------------------------=
    // FUNC: checksum
    // DESC: Returns the manifest CRC used by the existing boot generation contract.
    // ------------------=
    pub fn checksum(&self) -> u32 {
        if self.bytes.is_empty() {
            return self.crc;
        }
        let mut crc = !0u32;
        for byte in self.bytes {
            crc ^= *byte as u32;
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(crc & 1));
            }
        }
        !crc
    }
    // ------------------------=
    // FUNC: read
    // DESC: Reads one bounded logical range without allocating payload-sized memory.
    // ------------------=
    fn read(&self, offset: usize, output: &mut [u8]) -> Result<(), ()> {
        if offset.checked_add(output.len()).ok_or(())? > self.length {
            return Err(());
        }
        if !self.bytes.is_empty() {
            output.copy_from_slice(&self.bytes[offset..offset + output.len()]);
            return Ok(());
        }
        #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
        unsafe {
            if BRIDGE == 0 {
                return Err(());
            }
            let read: extern "efiapi" fn(u32, u64, usize, *mut u8) -> u64 =
                core::mem::transmute(BRIDGE as usize);
            if read(self.slot, offset as u64, output.len(), output.as_mut_ptr()) == 0 {
                return Ok(());
            }
        }
        Err(())
    }
    // ------------------------=
    // FUNC: transfer
    // DESC: Streams or verifies sectors and checks the full source SHA before activation can proceed.
    // ------------------=
    pub fn transfer<D: BlockDevice, F: FnMut(usize)>(
        &self,
        device: &mut D,
        lba: u64,
        verify: bool,
        mut progress: F,
    ) -> Result<(), ()> {
        use sha2::{Digest, Sha256};
        let blocks = (self.length as u64).checked_add(511).ok_or(())? / 512;
        if lba.checked_add(blocks).ok_or(())? > device.block_count() {
            return Err(());
        }
        let mut source = [0u8; 65536];
        let mut actual = [0u8; 65536];
        let mut hash = Sha256::new();
        let mut offset = 0usize;
        while offset < self.length {
            let count = (self.length - offset).min(source.len());
            self.read(offset, &mut source[..count])?;
            hash.update(&source[..count]);
            let padded = (count + 511) / 512 * 512;
            source[count..padded].fill(0);
            if verify {
                if !device.read_blocks(lba + offset as u64 / 512, &mut actual[..padded])
                    || source[..padded] != actual[..padded]
                {
                    return Err(());
                }
            } else if !device.write_blocks(lba + offset as u64 / 512, &source[..padded]) {
                return Err(());
            }
            offset += count;
            progress(offset / 512);
        }
        if self.bytes.is_empty() && hash.finalize().as_slice() != self.sha256 {
            return Err(());
        }
        Ok(())
    }
}
