#![allow(dead_code)]
mod storage;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use storage::object::ObjectStore;
use storage::BlockDevice;

struct FileDisk {
    file: File,
    sectors: u64,
}

impl FileDisk {
    // ------------------------=
    // FUNC: open
    // DESC: Opens a raw test-disk artifact as a read-only block device.
    // ------------------=
    fn open(path: &str) -> Self {
        let file = File::open(path).expect("open installed test disk");
        let sectors = file.metadata().expect("disk metadata").len() / 512;
        Self { file, sectors }
    }
}

impl BlockDevice for FileDisk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Returns the raw artifact capacity in 512-byte sectors.
    // ------------------=
    fn block_count(&self) -> u64 {
        self.sectors
    }

    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads one sector from the raw installed-disk artifact.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
        self.file.seek(SeekFrom::Start(lba.saturating_mul(512))).is_ok()
            && self.file.read_exact(out).is_ok()
    }

    // ------------------------=
    // FUNC: write_sector
    // DESC: Rejects mutation because installed-object verification is read-only.
    // ------------------=
    fn write_sector(&mut self, _: u64, _: &[u8; 512]) -> bool {
        false
    }

    // ------------------------=
    // FUNC: flush
    // DESC: Completes the read-only BlockDevice contract without mutating storage.
    // ------------------=
    fn flush(&mut self) -> bool {
        true
    }
}

// ------------------------=
// FUNC: read_u64
// DESC: Decodes one little-endian 64-bit field from an on-disk record.
// ------------------=
fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("u64 field"))
}

// ------------------------=
// FUNC: main
// DESC: Verifies installed native runtime, networking, AI, organization, and date/time System objects.
// ------------------=
fn main() {
    let path = std::env::args().nth(1).expect("raw disk path");
    let mut disk = FileDisk::open(&path);
    let mut entries = [0u8; 512];
    assert!(disk.read_sector(2, &mut entries), "read GPT entries");
    let container_lba = read_u64(&entries, 128 + 32);
    assert!(container_lba > 0, "Infinity Container partition missing");
    let mut store = ObjectStore::mount(disk, container_lba).expect("mount native Object Store");
    let date_time = store.date_time_configuration().expect("installed date/time configuration");
    assert!(date_time.is_valid(), "installed date/time configuration invalid");
    assert!(store.runtime_bootstrap_valid(), "native runtime/AI objects invalid");
    let organization = store.resolve(b"/system/organization/schema").expect("organization schema object");
    let mut bytes = [0u8; 256];
    assert_eq!(store.read(organization, None, &mut bytes).expect("read organization schema"), 256);
    assert!(storage::organization::organization_schema_valid(&bytes));
    println!("PASS: installed native runtime, AI, organization schema, and date/time settings mount and validate");
}
