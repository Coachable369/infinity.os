#[path = "../storage.rs"] mod storage;
#[path = "../../kernel/runtime/native_c_io.rs"] mod io;
use std::{rc::Rc, cell::RefCell};
use storage::{BlockDevice, object::{ObjectStore, ObjectType, Space, STORE_RELATIVE_LBA, MAX_CONTENT}};
use io::{ObjectIo, Grant, IoError, READ, WRITE, CREATE, TRUNCATE, APPEND, EXCLUSIVE, TEXT};

#[derive(Clone)]
struct Disk(Rc<RefCell<Vec<[u8;512]>>>);
impl BlockDevice for Disk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Reports sparse fixture capacity at the native store offset.
    // ------------------=
    fn block_count(&self) -> u64 { STORE_RELATIVE_LBA + self.0.borrow().len() as u64 }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads production object sectors without an emulated filesystem.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8;512]) -> bool {
        let Some(index) = lba.checked_sub(STORE_RELATIVE_LBA) else { return false; };
        let disk = self.0.borrow();
        let Some(bytes) = disk.get(index as usize) else { return false; };
        *out = *bytes; true
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Persists one sector in the shared remountable block fixture.
    // ------------------=
    fn write_sector(&mut self, lba: u64, input: &[u8;512]) -> bool {
        let Some(index) = lba.checked_sub(STORE_RELATIVE_LBA) else { return false; };
        let mut disk = self.0.borrow_mut();
        let Some(bytes) = disk.get_mut(index as usize) else { return false; };
        *bytes = *input; true
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Completes synchronous test writes.
    // ------------------=
    fn flush(&mut self) -> bool { true }
}

// ------------------------=
// FUNC: main
// DESC: Exercises native object stream rights, snapshots, conflict preservation, bounded operations and persisted bytes.
// ------------------=
fn main() {
    let disk = Disk(Rc::new(RefCell::new(vec![[0;512];4096])));
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [0x68;16]).unwrap();
    let grants = [Grant { root: b"/home/default/documents", write: true }];
    let mut io = ObjectIo::<4>::new(b"/home/default/documents", &grants).unwrap();
    let id = store.resolve(b"/home/default/documents/hello.c").unwrap();
    let source = include_bytes!("../../sdk/c/examples/hello.c");
    for path in [b"../pictures/secret".as_slice(), b"/system/runtime", b"/home/default/documents-sibling/a", b"../../../../escape"] {
        assert_eq!(io.open(&mut store, path, READ | WRITE | CREATE), Err(IoError::Denied));
    }
    assert_eq!(io.open(&mut store, b"hello.c\0evil", READ), Err(IoError::Invalid));
    assert_eq!(io.open(&mut store, b"hello.c", READ | TRUNCATE), Err(IoError::Invalid));
    assert_eq!(io.open(&mut store, b".", READ), Err(IoError::IsDirectory));
    assert_eq!(io.open(&mut store, b"hello.c", WRITE | CREATE | EXCLUSIVE), Err(IoError::Exists));
    let read = io.open(&mut store, b"./hello.c", READ).unwrap();
    store.write(id, b"int main(void) { return 17; }\n").unwrap();
    let mut bytes = [0;MAX_CONTENT];
    assert_eq!(io.read_at(read, 2, &mut bytes[..3]).unwrap(), 3);
    assert_eq!(&bytes[..3], &source[2..5]);
    assert_eq!(io.read_at(read, usize::MAX, &mut bytes[..3]).unwrap(), 0);
    assert_eq!(io.seek(read, 0, 1).unwrap(), 0);
    let length = io.read(read, &mut bytes).unwrap();
    assert_eq!(&bytes[..length], source); // opened version remains coherent
    assert_eq!(io.read(read, &mut bytes).unwrap(), 0);
    assert_eq!(io.write(read, b"bad"), Err(IoError::Denied));
    assert_eq!(io.truncate(read, 0), Err(IoError::Denied));
    assert_eq!(io.seek(read, -1, 0), Err(IoError::Invalid));
    assert_eq!(io.seek(read, i64::MAX, 2), Err(IoError::Invalid));
    io.close(&mut store, read).unwrap();
    assert_eq!(io.read(read, &mut bytes), Err(IoError::BadHandle));
    let current = io.open(&mut store, b"hello.c", READ).unwrap();
    assert_ne!(current, read);
    let length = io.read(current, &mut bytes).unwrap();
    assert_eq!(&bytes[..length], b"int main(void) { return 17; }\n");
    io.close(&mut store, current).unwrap();
    let output = io.open(&mut store, b"output.txt", READ | WRITE | CREATE | TRUNCATE | TEXT).unwrap();
    let output_id = store.resolve(b"/home/default/documents/output.txt").unwrap();
    assert_eq!(store.metadata(output_id).unwrap().kind, ObjectType::Text);
    io.write(output, b"abc").unwrap();
    io.truncate(output, 6).unwrap();
    assert_eq!(io.seek(output, 0, 1).unwrap(), 3);
    assert_eq!(io.read_at(output, 3, &mut bytes[..3]).unwrap(), 3);
    assert_eq!(&bytes[..3], &[0, 0, 0]);
    io.truncate(output, 3).unwrap();
    assert_eq!(io.truncate(output, MAX_CONTENT + 1), Err(IoError::Capacity));
    io.seek(output, 5, 0).unwrap();
    io.write(output, b"z").unwrap();
    let before = store.metadata(output_id).unwrap().current_version;
    io.flush(&mut store, output).unwrap();
    assert_eq!(store.metadata(output_id).unwrap().current_version, before + 1);
    io.flush(&mut store, output).unwrap();
    assert_eq!(store.metadata(output_id).unwrap().current_version, before + 1);
    assert_eq!(io.open(&mut store, b"output.txt", WRITE), Err(IoError::Conflict));
    io.seek(output, MAX_CONTENT as i64, 0).unwrap();
    assert_eq!(io.write(output, b"x"), Err(IoError::Capacity));
    io.close(&mut store, output).unwrap();
    let append = io.open(&mut store, b"output.txt", WRITE | APPEND).unwrap();
    io.seek(append, 0, 0).unwrap();
    io.write(append, b"!").unwrap();
    io.close(&mut store, append).unwrap();
    drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    let length = store.read(output_id, None, &mut bytes).unwrap();
    assert_eq!(&bytes[..length], b"abc\0\0z!");
    let pending = io.open(&mut store, b"output.txt", WRITE | TRUNCATE).unwrap();
    io.write(pending, b"would overwrite").unwrap();
    store.write(output_id, b"editor changes").unwrap();
    assert_eq!(io.close(&mut store, pending), Err(IoError::Conflict));
    let length = store.read(output_id, None, &mut bytes).unwrap();
    assert_eq!(&bytes[..length], b"editor changes");
    let readonly = [Grant { root: b"/home/default/documents", write: false }];
    let mut restricted = ObjectIo::<1>::new(b"/home/default/documents", &readonly).unwrap();
    assert_eq!(restricted.open(&mut store, b"output.txt", WRITE), Err(IoError::Denied));
    restricted.open(&mut store, b"hello.c", READ).unwrap();
    assert_eq!(restricted.open(&mut store, b"output.txt", READ), Err(IoError::Capacity));
    // A writable path alias cannot grant mutation of a System object.
    let system = store.create(b"test-system", ObjectType::Blob, Space::System, b"immutable").unwrap();
    store.attach(b"/home/default/documents/system-alias", system).unwrap();
    assert_eq!(io.open(&mut store, b"system-alias", WRITE), Err(IoError::Denied));
    assert_eq!(io.unlink(&mut store, b"system-alias"), Err(IoError::Denied));
    assert_eq!(io.make_directory(&mut store, b"../pictures/forbidden"), Err(IoError::Denied));
    io.make_directory(&mut store, b"compile-work").unwrap();
    io.change_directory(&store, b"compile-work").unwrap();
    let mut path = [0; 95];
    let length = io.current_directory(&mut path).unwrap();
    assert_eq!(&path[..length], b"/home/default/documents/compile-work");
    assert_eq!(io.change_directory(&store, b"../hello.c"), Err(IoError::Invalid));
    let canonical = io.canonical_path(&store, b"../hello.c", &mut path).unwrap();
    assert_eq!(&path[..canonical], b"/home/default/documents/hello.c");
    let created = io.open(&mut store, b"temporary.o", WRITE | CREATE).unwrap();
    io.write(created, b"object fixture").unwrap();
    assert_eq!(io.unlink(&mut store, b"temporary.o"), Err(IoError::Conflict));
    io.close(&mut store, created).unwrap();
    io.unlink(&mut store, b"temporary.o").unwrap();
    assert!(store.resolve(b"/home/default/documents/compile-work/temporary.o").is_err());
    io.change_directory(&store, b"..").unwrap();
    let hello = store.resolve(b"/home/default/documents/hello.c").unwrap();
    io.link(&mut store, b"hello.c", b"hello-reference.c", false).unwrap();
    assert_eq!(store.resolve(b"/home/default/documents/hello-reference.c").unwrap(), hello);
    assert_eq!(io.link(&mut store, b"hello.c", b"hello-reference.c", false), Err(IoError::Exists));
    assert_eq!(io.link(&mut store, b"hello.c", b"symbolic.c", true), Err(IoError::Unsupported));
    io.unlink(&mut store, b"hello-reference.c").unwrap();
    assert_eq!(store.resolve(b"/home/default/documents/hello.c").unwrap(), hello);
}
