#![allow(dead_code)]
#[path = "../kernel/runtime/mod.rs"] mod runtime;
#[path = "../kernel/ui/mod.rs"] mod ui;
#[path = "../kernel/storage/spatial_path.rs"] mod spatial_path;
mod storage;
use runtime::automation::{self, Catalog, Kind, State};
use std::{fs::{File, OpenOptions}, io::{Read, Seek, SeekFrom, Write}};
use storage::{BlockDevice, object::{ObjectStore, ObjectError, STORE_RELATIVE_LBA}};

// ------------------------=
// FUNC: output_text
// DESC: Supplies the runtime diagnostic sink without using prose as an acceptance oracle.
// ------------------=
fn output_text(_: &[u8]) {}

struct Disk(File, u64);
impl BlockDevice for Disk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Exposes the actual file-backed test disk geometry.
    // ------------------=
    fn block_count(&self) -> u64 { self.1 }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads one bounded disk sector across independent mounts.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
        lba < self.1 && self.0.seek(SeekFrom::Start(lba * 512)).is_ok() && self.0.read_exact(out).is_ok()
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes one actual native Object Store sector.
    // ------------------=
    fn write_sector(&mut self, lba: u64, bytes: &[u8; 512]) -> bool {
        lba < self.1 && self.0.seek(SeekFrom::Start(lba * 512)).is_ok() && self.0.write_all(bytes).is_ok()
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Requires durable disk synchronization before a test checkpoint is acknowledged.
    // ------------------=
    fn flush(&mut self) -> bool { self.0.sync_all().is_ok() }
}

// ------------------------=
// FUNC: main
// DESC: Verifies fresh-install catalog seeding, bounded checkpoints, corrupt-write rejection, and cold disk remount without replaying a one-time task.
// ------------------=
fn main() {
    std::fs::create_dir_all("build/tests").unwrap();
    let path = format!("build/tests/automation-reboot-{}.img", std::process::id());
    let sectors = STORE_RELATIVE_LBA + 32768;
    let file = OpenOptions::new().create_new(true).read(true).write(true).open(&path).unwrap();
    file.set_len(sectors * 512).unwrap();
    let mut store = ObjectStore::format(Disk(file, sectors), 0, sectors, [0x73; 16]).unwrap();
    let id = store.resolve(automation::PATH).unwrap();
    let mut bytes = [0; automation::STATE_BYTES];
    assert_eq!(store.read_automation(&mut bytes), Ok(bytes.len()));
    let mut catalog = Catalog::decode(&bytes).unwrap();
    assert_eq!(catalog.count(Kind::Daemon), 0);
    assert_eq!(catalog.count(Kind::Schedule), 0);
    let owner = [0x17; 16];
    let daemon = catalog.apply(&automation::parse(b"service create worker /worker.py start=boot delay=5 restart=on-failure").unwrap(), owner, true, 100).unwrap().unwrap();
    let once = catalog.apply(&automation::parse(b"schedule create once /once.py at=110").unwrap(), owner, true, 100).unwrap().unwrap();
    catalog.started(once, 14, 110).unwrap();
    catalog.finished(once, 0, 111).unwrap();
    catalog.started(daemon, 15, 111).unwrap();
    catalog.encode(&mut bytes).unwrap();
    store.checkpoint_automation(&bytes).unwrap();
    let before_generation = store.generation();
    let mut corrupt = bytes;
    corrupt[100] ^= 1;
    assert_eq!(store.checkpoint_automation(&corrupt), Err(ObjectError::CorruptContent));
    assert_eq!(store.generation(), before_generation);
    assert_eq!(store.write(id, &bytes), Err(ObjectError::Unauthorized));
    assert_eq!(store.read(id, None, &mut bytes), Err(ObjectError::Unauthorized));
    assert_eq!(store.remove(id), Err(ObjectError::Unauthorized));
    assert_eq!(store.detach(automation::PATH), Err(ObjectError::Unauthorized));
    for _ in 0..storage::object::MAX_VERSIONS + 8 { store.checkpoint_automation(&bytes).unwrap(); }
    drop(store);
    let file = OpenOptions::new().read(true).write(true).open(&path).unwrap();
    let mut mounted = ObjectStore::mount(Disk(file, sectors), 0).unwrap();
    let id = mounted.resolve(automation::PATH).unwrap();
    assert_eq!(mounted.read_automation(&mut bytes), Ok(bytes.len()));
    let restored = Catalog::decode(&bytes).unwrap();
    assert_eq!(restored.get(once).unwrap().state, State::Completed);
    assert_eq!(restored.get(once).unwrap().runs, 1);
    assert_eq!(restored.get(once).unwrap().last_exit, Some(0));
    assert_eq!(restored.get(daemon).unwrap().state, State::Failed);
    assert_eq!(restored.get(daemon).unwrap().process_id, 0);
    assert_eq!(restored.due(500, 4), None);
    assert_eq!(restored.due(500, 5).unwrap().id, daemon);
    drop(mounted);
    std::fs::remove_file(path).unwrap();
}
