#![allow(dead_code)]
#[path = "../kernel/runtime/mod.rs"] mod runtime;
#[path = "storage.rs"] mod storage;
#[path = "../kernel/ui/mod.rs"] mod ui;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use storage::{BlockDevice, object::{ObjectStore, ObjectType, ObjectError, Space, STORE_RELATIVE_LBA, SSH_SERVICE_OWNER}};
const SECTORS: u64 = STORE_RELATIVE_LBA + 32768;
#[derive(Clone)]
struct Disk(Rc<RefCell<BTreeMap<u64, [u8; 512]>>>);
// ------------------------=
// FUNC: output_text
// DESC: Supplies diagnostics without using text output as test evidence.
// ------------------=
fn output_text(_: &[u8]) {}
impl BlockDevice for Disk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Supplies fixed-capacity native sector storage.
    // ------------------=
    fn block_count(&self) -> u64 { SECTORS }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads persisted sparse sectors through the production BlockDevice contract.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
        if lba >= SECTORS { return false; }
        *out = self.0.borrow().get(&lba).copied().unwrap_or([0; 512]); true
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Records native sector writes for cold-remount verification.
    // ------------------=
    fn write_sector(&mut self, lba: u64, bytes: &[u8; 512]) -> bool {
        if lba >= SECTORS { return false; }
        self.0.borrow_mut().insert(lba, *bytes); true
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Completes synchronous persistence in the deterministic block device.
    // ------------------=
    fn flush(&mut self) -> bool { true }
}
// ------------------------=
// FUNC: main
// DESC: Exercises service-private host-key persistence on the real object store, generic access denial and independent remount.
// ------------------=
fn main() {
    let disk = Disk(Rc::new(RefCell::new(BTreeMap::new())));
    let mut store = ObjectStore::format(disk.clone(), 0, SECTORS, [0x73; 16]).unwrap();
    let path = b"/system/security/ssh/host-key";
    assert_eq!(store.resolve(path), Err(ObjectError::NamespaceNotFound));
    let bytes = [42; 74];
    let id = store.create_owned_attached(b"@ssh-host-key", ObjectType::Metadata, Space::System, &bytes, path, SSH_SERVICE_OWNER).unwrap();
    let mut out = [0; 74];
    assert_eq!(store.read(id, None, &mut out), Err(ObjectError::Unauthorized));
    assert_eq!(store.read(id, Some(1), &mut out), Err(ObjectError::Unauthorized));
    assert_eq!(store.write(id, &[1; 74]), Err(ObjectError::Unauthorized));
    assert_eq!(store.attach(b"/personal/key-alias", id), Err(ObjectError::Unauthorized));
    assert_eq!(store.copy_attached(id, b"/personal/key-copy"), Err(ObjectError::Unauthorized));
    assert_eq!(store.remove(id), Err(ObjectError::Unauthorized));
    assert!(store.create_attached(b"@ssh-host-key", ObjectType::Metadata, Space::System, &[1; 74], path).is_err());
    assert_eq!(store.read_ssh_private(id, b"@ssh-host-key", &mut out), Ok(74));
    assert_eq!(out, bytes);
    drop(store);
    let cold = Disk(Rc::new(RefCell::new(disk.0.borrow().clone())));
    let mut store = ObjectStore::mount(cold, 0).unwrap();
    assert_eq!(store.resolve(path), Ok(id));
    assert_eq!(store.read_ssh_private(id, b"@ssh-host-key", &mut out), Ok(74));
    assert_eq!(out, bytes);
    assert_eq!(store.read(id, None, &mut out), Err(ObjectError::Unauthorized));
    assert_eq!(store.write(id, &[1; 74]), Err(ObjectError::Unauthorized));
    assert_eq!(store.read_ssh_private(id, b"identity-state", &mut out), Err(ObjectError::Unauthorized));
    let impostor = store.create_attached(b"@ssh-host-key", ObjectType::Metadata, Space::System,
        &bytes, b"/system/security/ssh/wrong-owner").unwrap();
    assert_eq!(store.read_ssh_private(impostor, b"@ssh-host-key", &mut out), Err(ObjectError::Unauthorized));
}
