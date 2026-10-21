//! Bounded I/O adapter for foreground provisioning. It never owns or re-enters an installer.
use super::BlockDevice;

pub struct InteractiveDevice<'a, D, F> {
    pub device: &'a mut D,
    pub service: F,
    pub sectors: usize,
}
impl<D, F: FnMut()> InteractiveDevice<'_, D, F> {
    // ------------------------=
    // FUNC: progress
    // DESC: Services a bounded input drain after at most 64 KiB of completed sector work.
    // ------------------=
    fn progress(&mut self, sectors: usize) {
        self.sectors += sectors;
        if self.sectors >= 128 { self.sectors = 0; (self.service)(); }
    }
}
impl<D: BlockDevice, F: FnMut()> BlockDevice for InteractiveDevice<'_, D, F> {
    // ------------------------=
    // FUNC: block_count
    // DESC: Preserves the exact target geometry used by destructive-plan validation.
    // ------------------=
    fn block_count(&self) -> u64 { self.device.block_count() }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Preserves storage results and yields only after the underlying device borrow returns.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
        let result = self.device.read_sector(lba, out); self.progress(1); result
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Completes each write before servicing input, without dispatching application or storage actions.
    // ------------------=
    fn write_sector(&mut self, lba: u64, bytes: &[u8; 512]) -> bool {
        let result = self.device.write_sector(lba, bytes); self.progress(1); result
    }
    // ------------------------=
    // FUNC: read_blocks
    // DESC: Retains native bulk reads while bounding uninterruptible spans to 64 KiB.
    // ------------------=
    fn read_blocks(&mut self, lba: u64, bytes: &mut [u8]) -> bool {
        if bytes.len() % 512 != 0 { return false; }
        for (index, chunk) in bytes.chunks_mut(65536).enumerate() {
            let result = self.device.read_blocks(lba + (index * 128) as u64, chunk);
            self.progress(chunk.len() / 512);
            if !result { return false; }
        }
        true
    }
    // ------------------------=
    // FUNC: write_blocks
    // DESC: Retains native bulk writes and never retries a failed device operation.
    // ------------------=
    fn write_blocks(&mut self, lba: u64, bytes: &[u8]) -> bool {
        if bytes.len() % 512 != 0 { return false; }
        for (index, chunk) in bytes.chunks(65536).enumerate() {
            let result = self.device.write_blocks(lba + (index * 128) as u64, chunk);
            self.progress(chunk.len() / 512);
            if !result { return false; }
        }
        true
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Services input after the device has completed its durability barrier.
    // ------------------=
    fn flush(&mut self) -> bool {
        let result = self.device.flush(); self.sectors = 0; (self.service)(); result
    }
}
