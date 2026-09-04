use super::{BlockDevice, StorageDevice};
use core::arch::asm;

const DATA: u16 = 0x1f0;
const FEATURES: u16 = 0x1f1;
const COUNT: u16 = 0x1f2;
const LBA0: u16 = 0x1f3;
const LBA1: u16 = 0x1f4;
const LBA2: u16 = 0x1f5;
const DRIVE: u16 = 0x1f6;
const COMMAND_STATUS: u16 = 0x1f7;
const BUSY: u8 = 0x80;
const READY: u8 = 0x40;
const DATA_REQUEST: u8 = 0x08;
const ERROR: u8 = 0x01;
const TIMEOUT: usize = 2_000_000;

pub struct AtaDevice {
    blocks: u64,
}

// ------------------------=
// FUNC: outb
// DESC: Implements the outb operation.
// ------------------=
unsafe fn outb(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack));
}
// ------------------------=
// FUNC: inb
// DESC: Implements the inb operation.
// ------------------=
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    asm!("in al, dx", in("dx") port, out("al") value, options(nomem, nostack));
    value
}
// ------------------------=
// FUNC: outw
// DESC: Implements the outw operation.
// ------------------=
unsafe fn outw(port: u16, value: u16) {
    asm!("out dx, ax", in("dx") port, in("ax") value, options(nomem, nostack));
}
// ------------------------=
// FUNC: inw
// DESC: Implements the inw operation.
// ------------------=
unsafe fn inw(port: u16) -> u16 {
    let value: u16;
    asm!("in ax, dx", in("dx") port, out("ax") value, options(nomem, nostack));
    value
}

// ------------------------=
// FUNC: wait_not_busy
// DESC: Implements the wait not busy operation.
// ------------------=
fn wait_not_busy() -> Option<u8> {
    for _ in 0..TIMEOUT {
        let status = unsafe { inb(COMMAND_STATUS) };
        if status & BUSY == 0 {
            return Some(status);
        }
        core::hint::spin_loop();
    }
    None
}

// ------------------------=
// FUNC: wait_data
// DESC: Implements the wait data operation.
// ------------------=
fn wait_data() -> bool {
    for _ in 0..TIMEOUT {
        let status = unsafe { inb(COMMAND_STATUS) };
        if status & ERROR != 0 {
            return false;
        }
        if status & BUSY == 0 && status & DATA_REQUEST != 0 {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}

// ------------------------=
// FUNC: select_lba
// DESC: Implements the select lba operation.
// ------------------=
fn select_lba(lba: u64, command: u8) -> bool {
    if lba > 0x0fff_ffff {
        return false;
    }
    if wait_not_busy().is_none() {
        return false;
    }
    unsafe {
        outb(DRIVE, 0xe0 | ((lba >> 24) as u8 & 0x0f));
        outb(FEATURES, 0);
        outb(COUNT, 1);
        outb(LBA0, lba as u8);
        outb(LBA1, (lba >> 8) as u8);
        outb(LBA2, (lba >> 16) as u8);
        outb(COMMAND_STATUS, command);
    }
    wait_data()
}

impl AtaDevice {
    // ------------------------=
    // FUNC: open
    // DESC: Implements the open operation.
    // ------------------=
    pub fn open() -> Option<Self> {
        identify().map(|(_, blocks)| Self { blocks })
    }
}

impl BlockDevice for AtaDevice {
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
        if lba >= self.blocks || !select_lba(lba, 0x20) {
            return false;
        }
        for word in 0..256 {
            let value = unsafe { inw(DATA) }.to_le_bytes();
            sector[word * 2] = value[0];
            sector[word * 2 + 1] = value[1];
        }
        true
    }

    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes or updates write sector data.
    // ------------------=
    fn write_sector(&mut self, lba: u64, sector: &[u8; 512]) -> bool {
        if lba >= self.blocks || !select_lba(lba, 0x30) {
            return false;
        }
        for word in 0..256 {
            unsafe {
                outw(
                    DATA,
                    u16::from_le_bytes([sector[word * 2], sector[word * 2 + 1]]),
                );
            }
        }
        wait_not_busy()
            .map(|status| status & ERROR == 0)
            .unwrap_or(false)
    }

    // ------------------------=
    // FUNC: flush
    // DESC: Implements the flush operation.
    // ------------------=
    fn flush(&mut self) -> bool {
        if wait_not_busy().is_none() {
            return false;
        }
        unsafe {
            outb(COMMAND_STATUS, 0xe7);
        }
        wait_not_busy()
            .map(|status| status & ERROR == 0)
            .unwrap_or(false)
    }
}

// ------------------------=
// FUNC: identify
// DESC: Implements the identify operation.
// ------------------=
fn identify() -> Option<([u16; 256], u64)> {
    if unsafe { inb(COMMAND_STATUS) } == 0xff {
        return None;
    }
    unsafe {
        outb(DRIVE, 0xa0);
        outb(COUNT, 0);
        outb(LBA0, 0);
        outb(LBA1, 0);
        outb(LBA2, 0);
        outb(COMMAND_STATUS, 0xec);
    }
    let initial = unsafe { inb(COMMAND_STATUS) };
    if initial == 0 || !wait_data() {
        return None;
    }
    let mut words = [0u16; 256];
    for word in &mut words {
        *word = unsafe { inw(DATA) };
    }
    let blocks = words[60] as u64 | ((words[61] as u64) << 16);
    if blocks < 32768 {
        None
    } else {
        Some((words, blocks))
    }
}

// ------------------------=
// FUNC: discover
// DESC: Implements the discover operation.
// ------------------=
pub fn discover() -> Option<StorageDevice> {
    let (words, blocks) = identify()?;
    let mut model = [0u8; 40];
    for index in 0..20 {
        let bytes = words[27 + index].to_be_bytes();
        model[index * 2] = bytes[0];
        model[index * 2 + 1] = bytes[1];
    }
    let mut length = 40;
    while length > 0 && (model[length - 1] == b' ' || model[length - 1] == 0) {
        length -= 1;
    }
    let mut device = AtaDevice { blocks };
    let mut header = [0u8; 512];
    let has_gpt = device.read_sector(1, &mut header) && &header[..8] == b"EFI PART";
    Some(StorageDevice {
        identity: b"storage0",
        model,
        model_length: length,
        blocks,
        logical_block_size: 512,
        physical_block_size: 512,
        removable: words[0] & 0x0080 != 0,
        bus: b"ATA PIO",
        has_gpt,
    })
}
