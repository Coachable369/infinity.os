use super::{BlockDevice, StorageDevice};

#[repr(C)]
struct BlockMedia {
    media_id: u32,
    removable_media: u8,
    media_present: u8,
    logical_partition: u8,
    read_only: u8,
    write_caching: u8,
    block_size: u32,
    io_align: u32,
    last_block: u64,
    lowest_aligned_lba: u64,
    logical_blocks_per_physical_block: u32,
    optimal_transfer_length_granularity: u32,
}

#[repr(C)]
struct BlockIo {
    revision: u64,
    media: *mut BlockMedia,
    reset: usize,
    read_blocks: usize,
    write_blocks: usize,
    flush_blocks: usize,
}

type BlockTransfer = unsafe extern "efiapi" fn(*mut BlockIo, u32, u64, usize, *mut u8) -> u64;
type FlushBlocks = unsafe extern "efiapi" fn(*mut BlockIo) -> u64;

pub struct UefiBlockDevice {
    protocol: *mut BlockIo,
    media_id: u32,
    blocks: u64,
}

impl UefiBlockDevice {
    // ------------------------=
    // FUNC: open
    // DESC: Implements the open operation.
    // ------------------=
    pub fn open() -> Option<Self> {
        let protocol = crate::drivers::input::uefi::block_io(0)? as *mut BlockIo;
        let media = unsafe { protocol.as_ref()?.media.as_ref()? };
        if media.media_present == 0 || media.read_only != 0 || media.block_size != 512 {
            return None;
        }
        Some(Self {
            protocol,
            media_id: media.media_id,
            blocks: media.last_block + 1,
        })
    }
}

impl BlockDevice for UefiBlockDevice {
    // ------------------------=
    // FUNC: block_count
    // DESC: Implements the block count operation.
    // ------------------=
    fn block_count(&self) -> u64 {
        self.blocks
    }

    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads read sector data.
    // ------------------=
    fn read_sector(&mut self, lba: u64, sector: &mut [u8; 512]) -> bool {
        if lba >= self.blocks {
            return false;
        }
        let address = unsafe { (*self.protocol).read_blocks };
        if address == 0 {
            return false;
        }
        let read: BlockTransfer = unsafe { core::mem::transmute(address) };
        unsafe {
            read(
                self.protocol,
                self.media_id,
                lba,
                sector.len(),
                sector.as_mut_ptr(),
            ) == 0
        }
    }

    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes or updates write sector data.
    // ------------------=
    fn write_sector(&mut self, lba: u64, sector: &[u8; 512]) -> bool {
        if lba >= self.blocks {
            return false;
        }
        let address = unsafe { (*self.protocol).write_blocks };
        if address == 0 {
            return false;
        }
        let write: BlockTransfer = unsafe { core::mem::transmute(address) };
        unsafe {
            write(
                self.protocol,
                self.media_id,
                lba,
                sector.len(),
                sector.as_ptr() as *mut u8,
            ) == 0
        }
    }

    // ------------------------=
    // FUNC: flush
    // DESC: Implements the flush operation.
    // ------------------=
    fn flush(&mut self) -> bool {
        let address = unsafe { (*self.protocol).flush_blocks };
        if address == 0 {
            return true;
        }
        let flush: FlushBlocks = unsafe { core::mem::transmute(address) };
        unsafe { flush(self.protocol) == 0 }
    }
}

// ------------------------=
// FUNC: discover
// DESC: Implements the discover operation.
// ------------------=
pub fn discover() -> Option<StorageDevice> {
    let mut device = UefiBlockDevice::open()?;
    if device.blocks < 32_768 {
        return None;
    }
    let mut header = [0u8; 512];
    let has_gpt = device.read_sector(1, &mut header) && &header[..8] == b"EFI PART";
    let mut model = [0u8; 40];
    let name = b"VirtualBox UEFI Virtual Disk";
    model[..name.len()].copy_from_slice(name);
    Some(StorageDevice {
        identity: b"storage0",
        model,
        model_length: name.len(),
        blocks: device.blocks,
        logical_block_size: 512,
        physical_block_size: 512,
        removable: false,
        bus: b"UEFI Block I/O",
        has_gpt,
    })
}
