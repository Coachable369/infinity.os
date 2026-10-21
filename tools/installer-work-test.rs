#[path="../kernel/core/installer_work.rs"]
mod installer_work;
#[path="../kernel/ui/input_batch.rs"]
mod input_batch;
#[path="../kernel/storage/interactive_device.rs"]
mod interactive_device;
use std::cell::Cell;

// Minimal typed disk contract; the production I/O adapter itself is imported above.
trait BlockDevice {
    // ------------------------=
    // FUNC: block_count
    // DESC: Supplies the test disk geometry.
    // ------------------=
    fn block_count(&self) -> u64;
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads one test sector.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool;
    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes one test sector.
    // ------------------=
    fn write_sector(&mut self, lba: u64, bytes: &[u8; 512]) -> bool;
    // ------------------------=
    // FUNC: flush
    // DESC: Flushes the test disk.
    // ------------------=
    fn flush(&mut self) -> bool;
    // ------------------------=
    // FUNC: read_blocks
    // DESC: Exposes bulk read boundaries for the production adapter fixture.
    // ------------------=
    fn read_blocks(&mut self, lba: u64, out: &mut [u8]) -> bool;
    // ------------------------=
    // FUNC: write_blocks
    // DESC: Exposes bulk write boundaries for the production adapter fixture.
    // ------------------=
    fn write_blocks(&mut self, lba: u64, bytes: &[u8]) -> bool;
}
struct Disk { bytes: Vec<u8>, operations: Vec<(u64, usize)>, fail: bool }
impl BlockDevice for Disk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Reports stable sector geometry.
    // ------------------=
    fn block_count(&self) -> u64 { (self.bytes.len() / 512) as u64 }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads exact bytes through the bulk test implementation.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8;512]) -> bool { self.read_blocks(lba, out) }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes exact bytes through the bulk test implementation.
    // ------------------=
    fn write_sector(&mut self, lba: u64, bytes: &[u8;512]) -> bool { self.write_blocks(lba, bytes) }
    // ------------------------=
    // FUNC: flush
    // DESC: Returns an injected durability failure without masking it.
    // ------------------=
    fn flush(&mut self) -> bool { !self.fail }
    // ------------------------=
    // FUNC: read_blocks
    // DESC: Records native I/O sizes and returns persisted bytes.
    // ------------------=
    fn read_blocks(&mut self, lba: u64, out: &mut [u8]) -> bool {
        self.operations.push((lba, out.len()));
        if self.fail { return false; }
        out.copy_from_slice(&self.bytes[lba as usize * 512..lba as usize * 512 + out.len()]); true
    }
    // ------------------------=
    // FUNC: write_blocks
    // DESC: Records each attempt once and injects write failures before mutation.
    // ------------------=
    fn write_blocks(&mut self, lba: u64, bytes: &[u8]) -> bool {
        self.operations.push((lba, bytes.len()));
        if self.fail { return false; }
        self.bytes[lba as usize * 512..lba as usize * 512 + bytes.len()].copy_from_slice(bytes); true
    }
}

// ------------------------=
// FUNC: run
// DESC: Verifies presentation-before-write, single confirmation, non-reentrancy, cursor service cadence, exact storage bytes, and failure propagation.
// ------------------=
pub fn run() {
    use installer_work::Work;
    let mut work = Work::Idle;
    let mut batch = input_batch::PresentationBatch::new();
    assert!(!work.begin(false));
    let owner = batch.enter();
    assert!(work.confirm());
    assert!(!batch.request());
    assert!(!work.confirm());
    assert!(!work.begin(batch.active()));
    assert_eq!(batch.leave(owner), input_batch::Presentation::Scene);
    assert!(work.begin(batch.active()));
    assert!(!work.begin(false));
    let services = Cell::new(0);
    let mut disk = Disk { bytes: vec![0; 256 * 1024], operations: Vec::new(), fail: false };
    let content = vec![0x73; 192 * 1024];
    {
        let mut device = interactive_device::InteractiveDevice { device: &mut disk, service: || {
            assert!(work.busy());
            assert!(!work.confirm());
            assert!(!work.begin(false));
            services.set(services.get() + 1);
        }, sectors: 0 };
        assert_eq!(device.block_count(), 512);
        assert!(device.write_blocks(0, &content));
        assert_eq!(services.get(), 3);
        let mut readback = vec![0; content.len()];
        assert!(device.read_blocks(0, &mut readback));
        assert_eq!(readback, content);
        assert_eq!(services.get(), 6);
        assert!(device.flush());
        assert_eq!(services.get(), 7);
        for _ in 0..127 { assert!(device.write_sector(400, &[0x42;512])); }
        assert_eq!(services.get(), 7);
        assert!(device.read_sector(400, &mut [0;512]));
        assert_eq!(services.get(), 8);
        assert!(!device.write_blocks(0, &[0;513]));
        assert_eq!(services.get(), 8);
    }
    assert!(disk.operations.iter().all(|(_,size)| *size <= 65536));
    assert_eq!(&disk.operations[..3], &[(0,65536), (128,65536), (256,65536)]);
    let before = disk.bytes.clone();
    disk.fail = true;
    let count = disk.operations.len();
    {
        let mut device = interactive_device::InteractiveDevice { device: &mut disk, service: || {}, sectors: 0 };
        assert!(!device.write_blocks(0, &content));
        assert!(!device.flush());
    }
    assert_eq!(disk.operations.len(), count + 1);
    assert_eq!(disk.bytes, before);
    work.finish();
    assert!(!work.busy());
    assert!(!work.begin(false));
}
