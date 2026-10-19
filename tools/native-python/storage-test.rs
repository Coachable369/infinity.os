#[path = "../storage.rs"] mod storage;
#[path = "../../kernel/storage/spatial_path.rs"] mod spatial_path;
#[path = "../../sdk/infinity-browser-core/favorites.rs"] mod favorites;
#[path = "../../kernel/runtime/font_preferences.rs"] mod font_preferences;
use std::{cell::RefCell, rc::Rc};
use storage::{BlockDevice, object::{ObjectStore, STORE_RELATIVE_LBA}};

#[derive(Clone)]
struct Disk(Rc<RefCell<Vec<[u8; 512]>>>);
impl BlockDevice for Disk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Reports the bounded fixture disk geometry.
    // ------------------=
    fn block_count(&self) -> u64 { self.0.borrow().len() as u64 }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads exactly one native sector from the fixture disk.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
        let disk = self.0.borrow();
        let Some(sector) = disk.get(lba as usize) else { return false; };
        *out = *sector; true
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Persists exactly one native sector within the fixture disk.
    // ------------------=
    fn write_sector(&mut self, lba: u64, bytes: &[u8; 512]) -> bool {
        let mut disk = self.0.borrow_mut();
        let Some(sector) = disk.get_mut(lba as usize) else { return false; };
        *sector = *bytes; true
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Completes synchronous fixture writes.
    // ------------------=
    fn flush(&mut self) -> bool { true }
}

// ------------------------=
// FUNC: main
// DESC: Seeds actual release artifacts through the installer helper and compares full bytes after remount.
// ------------------=
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let extension = std::fs::read(&args[1]).unwrap();
    let license = std::fs::read(&args[2]).unwrap();
    assert!(license.len() > storage::object::MAX_CONTENT);
    let sectors = STORE_RELATIVE_LBA as usize + 32768;
    let disk = Disk(Rc::new(RefCell::new(vec![[0; 512]; sectors])));
    let mut store = ObjectStore::format(disk.clone(), 0, sectors as u64, [0x51; 16]).unwrap();
    store.seed_python_examples(&extension, &license).unwrap();
    // The bootstrap helper participates in the formatter's final metadata commit.
    // A normal namespace transaction commits that same pending state in this fixture.
    let license_id = store.resolve(b"/system/licenses/python").unwrap();
    store.attach(b"/system/licenses/python-test-proof", license_id).unwrap();
    drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    for (path, expected) in [
        (b"/home/default/documents/hello.py".as_slice(), include_bytes!("../../sdk/python/examples/hello.py").as_slice()),
        (b"/home/default/documents/native_demo.py".as_slice(), include_bytes!("../../sdk/python/examples/native_demo.py").as_slice()),
        (b"/home/default/documents/infinity_native_test.so".as_slice(), extension.as_slice()),
        (b"/system/licenses/python".as_slice(), license.as_slice()),
    ] {
        let id = store.resolve(path).unwrap();
        let mut actual = vec![0; expected.len()];
        assert_eq!(store.read(id, None, &mut actual).unwrap(), expected.len());
        assert_eq!(actual, expected);
    }
}
