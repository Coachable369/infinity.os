mod storage;

use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use storage::{
    object::{crc32, ObjectError, ObjectStore, STORE_RELATIVE_LBA},
    BlockDevice,
};

#[derive(Clone)]
struct SparseDisk {
    sectors: u64,
    bytes: Rc<RefCell<BTreeMap<u64, [u8; 512]>>>,
}

impl SparseDisk {
    // ------------------------=
    // FUNC: downgrade_metadata_to_v5
    // DESC: Converts a fresh empty fixture's metadata headers into the exact compact v5 shape for migration testing.
    // ------------------=
    fn downgrade_metadata_to_v5(&self) {
        for sector in self.bytes.borrow_mut().values_mut() {
            if !sector.starts_with(b"INFO") { continue; }
            sector[8..12].copy_from_slice(&5u32.to_le_bytes());
            if sector.starts_with(b"INFOSTAT") {
                sector[32..36].copy_from_slice(&15_744u32.to_le_bytes());
                sector[36..40].copy_from_slice(&1_968u32.to_le_bytes());
            }
            sector[508..512].fill(0);
            let checksum = crc32(sector);
            sector[508..512].copy_from_slice(&checksum.to_le_bytes());
        }
    }
}

impl BlockDevice for SparseDisk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Reports a compiler-capable sparse test device without allocating its logical capacity in host memory.
    // ------------------=
    fn block_count(&self) -> u64 { self.sectors }

    // ------------------------=
    // FUNC: read_sector
    // DESC: Returns durable sparse sectors and physical zeroes for unwritten capacity.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
        if lba >= self.sectors { return false; }
        *out = self.bytes.borrow().get(&lba).copied().unwrap_or([0; 512]);
        true
    }

    // ------------------------=
    // FUNC: write_sector
    // DESC: Persists one in-range sector while eliding physical all-zero storage.
    // ------------------=
    fn write_sector(&mut self, lba: u64, bytes: &[u8; 512]) -> bool {
        if lba >= self.sectors { return false; }
        if bytes.iter().all(|byte| *byte == 0) { self.bytes.borrow_mut().remove(&lba); }
        else { self.bytes.borrow_mut().insert(lba, *bytes); }
        true
    }

    // ------------------------=
    // FUNC: flush
    // DESC: Completes the in-memory durable write boundary synchronously.
    // ------------------=
    fn flush(&mut self) -> bool { true }
}

// ------------------------=
// FUNC: main
// DESC: Verifies v6 capacity, compiler-sized reservation, sealed streaming, ordinary-object isolation and reboot persistence.
// ------------------=
fn main() {
    let disk = SparseDisk {
        sectors: STORE_RELATIVE_LBA + 1_200_000,
        bytes: Rc::new(RefCell::new(BTreeMap::new())),
    };
    let mut store = ObjectStore::format(disk.clone(), 0, disk.sectors, [0x61; 16]).unwrap();
    assert!(store.total_blocks() as u64 * 4096 >= 128 * 1024 * 1024);

    let clang = store.create_system_extent(b"clang", 100 * 1024 * 1024).unwrap();
    assert!(store.staging_reserved_bytes() >= 100 * 1024 * 1024);
    assert_eq!(store.read_system_extent(clang, 0, &mut [0; 1]), Err(ObjectError::InvalidObject));

    let size = 2usize * 1024 * 1024;
    let content: Vec<u8> = (0..size).map(|index| (index.wrapping_mul(31) % 251) as u8).collect();
    let component = store.create_system_extent(b"resource", size as u32).unwrap();
    for (index, chunk) in content.chunks(1024).enumerate() {
        store.write_staging_range(component, (index * 1024) as u64, chunk).unwrap();
    }
    store.seal_system_extent(component, b"/system/toolchain/resource", crc32(&content)).unwrap();
    assert_eq!(store.read(component, None, &mut [0; 16]), Err(ObjectError::InsufficientCapacity));
    let mut sample = [0u8; 4096];
    store.read_system_extent(component, 1_048_123, &mut sample).unwrap();
    assert_eq!(&sample, &content[1_048_123..1_048_123 + sample.len()]);
    drop(store);

    let mut mounted = ObjectStore::mount(disk, 0).unwrap();
    let resolved = mounted.resolve(b"/system/toolchain/resource").unwrap();
    assert_eq!(resolved, component);
    mounted.read_system_extent(resolved, (size - sample.len()) as u64, &mut sample).unwrap();
    assert_eq!(&sample, &content[size - sample.len()..]);

    let migration_disk = SparseDisk {
        sectors: STORE_RELATIVE_LBA + 1_200_000,
        bytes: Rc::new(RefCell::new(BTreeMap::new())),
    };
    drop(ObjectStore::format(migration_disk.clone(), 0, migration_disk.sectors, [0x62; 16]).unwrap());
    migration_disk.downgrade_metadata_to_v5();
    let mut legacy = ObjectStore::mount(migration_disk.clone(), 0).unwrap();
    assert_eq!(legacy.total_blocks(), 15_744);
    let documents = legacy.resolve(b"/home/default/documents").unwrap();
    legacy.attach(b"/home/default/v5-migrated", documents).unwrap();
    drop(legacy);
    let upgraded = ObjectStore::mount(migration_disk, 0).unwrap();
    assert!(upgraded.total_blocks() as u64 * 4096 >= 128 * 1024 * 1024);
    assert_eq!(upgraded.resolve(b"/home/default/v5-migrated").unwrap(), documents);
}
