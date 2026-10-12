#[path = "../storage.rs"] mod storage;
#[path = "../../kernel/runtime/native_c_io.rs"] mod io;
use io as native_c_io;
#[path = "../../kernel/runtime/execution.rs"] mod execution;
#[path = "../../kernel/runtime/capability.rs"] mod capability;
#[path = "../../kernel/runtime/native_c_service.rs"] mod service;
use service as native_c_service;
#[path = "../../kernel/runtime/native_c_image.rs"] mod native_c_image;
#[path = "../../kernel/runtime/native_memory.rs"] mod native_memory;
#[path = "../../kernel/runtime/native_memory_arm.rs"] mod native_memory_arm;
#[path = "../../kernel/runtime/native_c_syscall.rs"] mod native_c_syscall;
use std::{rc::Rc, cell::RefCell};
use storage::{BlockDevice, object::{ObjectStore, ObjectType, Space, STORE_RELATIVE_LBA, MAX_CONTENT}};
use io::{ObjectIo, Grant, IoError, READ, WRITE, CREATE, TRUNCATE, APPEND, EXCLUSIVE, TEXT};

// ------------------------=
// FUNC: resource_stream_acceptance
// DESC: Streams sealed compiler resources across allocation boundaries after remount, with independent cursors and immutable grants.
// ------------------=
fn resource_stream_acceptance() {
    let disk = Disk(Rc::new(RefCell::new(vec![[0;512];16384])));
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [0x72;16]).unwrap();
    let content: Vec<u8> = (0..131071usize).map(|i| ((i * 31) % 251) as u8).collect();
    let resource = store.create_system_extent(b"sdk", content.len() as u32).unwrap();
    for (i, bytes) in content.chunks(1024).enumerate() {
        store.write_staging_range(resource, (i * 1024) as u64, bytes).unwrap();
    }
    store.seal_system_extent(resource, b"/system/toolchain/sdk", storage::object::crc32(&content)).unwrap();
    drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    let grants = [Grant { root: b"/system/toolchain", write: true }];
    let mut io = ObjectIo::<2>::new(b"/system/toolchain", &grants).unwrap();
    assert_eq!(io.open(&mut store, b"sdk", WRITE), Err(IoError::Denied));
    let first = io.open(&mut store, b"sdk", READ).unwrap();
    let second = io.open(&mut store, b"sdk", READ).unwrap();
    assert_eq!(io.inspect(first).unwrap().size, content.len() as u64);
    assert_eq!(io.inspect(first).unwrap().access, READ);
    let mut out = vec![0; 70000];
    assert_eq!(io.read(&mut store, first, &mut out).unwrap(), 65536);
    assert_eq!(&out[..65536], &content[..65536]);
    assert_eq!(io.seek(second, 0, 1).unwrap(), 0);
    assert_eq!(io.read_at(&mut store, first, 4093, &mut out[..31]).unwrap(), 31);
    assert_eq!(&out[..31], &content[4093..4124]);
    assert_eq!(io.seek(first, 0, 1).unwrap(), 65536);
    assert_eq!(io.read(&mut store, first, &mut out).unwrap(), content.len() - 65536);
    assert_eq!(&out[..content.len()-65536], &content[65536..]);
    assert_eq!(io.read(&mut store, first, &mut out).unwrap(), 0);
    assert_eq!(io.read_at(&mut store, second, usize::MAX, &mut out).unwrap(), 0);
    assert_eq!(io.seek(first, -7, 2).unwrap(), content.len()-7);
    assert_eq!(io.read(&mut store, first, &mut out).unwrap(), 7);
    assert_eq!(&out[..7], &content[content.len()-7..]);
    assert_eq!(io.write(first, b"replace"), Err(IoError::Denied));
    assert_eq!(io.truncate(first, 0), Err(IoError::Denied));
    io.close(&mut store, first).unwrap();
    io.close(&mut store, second).unwrap();
    assert_eq!(store.metadata(resource).unwrap().current_version, 1);
    let restricted = [Grant { root: b"/home/default/documents", write: false }];
    let mut denied = ObjectIo::<1>::new(b"/", &restricted).unwrap();
    assert_eq!(denied.open(&mut store, b"/system/toolchain/sdk", READ), Err(IoError::Denied));
}

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
    assert_eq!(io.read_at(&mut store, read, 2, &mut bytes[..3]).unwrap(), 3);
    assert_eq!(&bytes[..3], &source[2..5]);
    assert_eq!(io.read_at(&mut store, read, usize::MAX, &mut bytes[..3]).unwrap(), 0);
    assert_eq!(io.seek(read, 0, 1).unwrap(), 0);
    let length = io.read(&mut store, read, &mut bytes).unwrap();
    assert_eq!(&bytes[..length], source); // opened version remains coherent
    assert_eq!(io.read(&mut store, read, &mut bytes).unwrap(), 0);
    assert_eq!(io.write(read, b"bad"), Err(IoError::Denied));
    assert_eq!(io.truncate(read, 0), Err(IoError::Denied));
    assert_eq!(io.seek(read, -1, 0), Err(IoError::Invalid));
    assert_eq!(io.seek(read, i64::MAX, 2), Err(IoError::Invalid));
    io.close(&mut store, read).unwrap();
    assert_eq!(io.read(&mut store, read, &mut bytes), Err(IoError::BadHandle));
    let current = io.open(&mut store, b"hello.c", READ).unwrap();
    assert_ne!(current, read);
    let length = io.read(&mut store, current, &mut bytes).unwrap();
    assert_eq!(&bytes[..length], b"int main(void) { return 17; }\n");
    io.close(&mut store, current).unwrap();
    let output = io.open(&mut store, b"output.txt", READ | WRITE | CREATE | TRUNCATE | TEXT).unwrap();
    let output_id = store.resolve(b"/home/default/documents/output.txt").unwrap();
    assert_eq!(store.metadata(output_id).unwrap().kind, ObjectType::Text);
    io.write(output, b"abc").unwrap();
    io.truncate(output, 6).unwrap();
    assert_eq!(io.seek(output, 0, 1).unwrap(), 3);
    assert_eq!(io.read_at(&mut store, output, 3, &mut bytes[..3]).unwrap(), 3);
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
    resource_stream_acceptance();
    service_acceptance(&mut store);
    syscall_acceptance::<native_memory::X86AddressSpace>(&mut store);
    syscall_acceptance::<native_memory_arm::ArmAddressSpace>(&mut store);
}

// ------------------------=
// FUNC: syscall_acceptance
// DESC: Exercises real hardware page encodings and ObjectStore through the compiler ABI, including failure atomicity.
// ------------------=
fn syscall_acceptance<T: native_memory::PageTable>(store: &mut ObjectStore<Disk>) {
    use native_memory::*;
    use capability::{CapabilityManager, CapabilityType};
    use execution::SecurityIdentity;
    let layout = std::alloc::Layout::from_size_align(1024 * 1024, 4096).unwrap();
    let arena = unsafe { std::alloc::alloc_zeroed(layout) };
    assert!(!arena.is_null());
    let mut memory = NativeMemory::empty();
    memory.initialize(arena as u64, layout.size() as u64).unwrap();
    let initial = memory.available_pages();
    let data = memory.allocate(3, 1).unwrap();
    let mut tables = T::new(&mut memory, 1).unwrap();
    let user = 0x1_0000_0000;
    tables.map_range(&mut memory, user, data.physical_address, 8192, PagePermissions::USER_RW).unwrap();
    tables.map_range(&mut memory, user + 8192, data.physical_address + 8192, 4096, PagePermissions::USER_R).unwrap();
    tables.map_range(&mut memory, user + 0x10000, data.physical_address, 4096, PagePermissions::KERNEL_RW).unwrap();
    let owner = SecurityIdentity([71;16]);
    let mut caps = CapabilityManager::new();
    let read = caps.grant(CapabilityType::PersonalSpaceRead, 90, 1, 0, owner, owner, None, 0).unwrap();
    let write = caps.grant(CapabilityType::PersonalSpaceWrite, 90, 1, 0, owner, owner, None, 0).unwrap();
    let mut session = service::Session::<2>::new(ObjectIo::<2>::documents(), owner, 90, read, write);
    copy_to_user(&mut tables, user + 4090, b"abi-output.c\0").unwrap();
    let flags = (READ | WRITE | CREATE | TRUNCATE) as u64;
    // Invalid result pointers cannot create an object or consume a handle.
    assert_eq!(session.file_syscall(owner, &caps, store, &mut tables, 0x120b,
        [user + 4090, flags, user + 8192, 0, 0, 0], 1), -14);
    assert_eq!(session.file_syscall(owner, &caps, store, &mut tables, 0x120b,
        [user + 4090, flags, user + 128, 0, 0, 0], 1), 0);
    let mut number = [0u8;8];
    copy_from_user(&tables, user + 128, &mut number).unwrap();
    let handle = u64::from_le_bytes(number);
    assert_eq!(handle, 3);
    assert_eq!(session.file_syscall(owner, &caps, store, &mut tables, 0x1214,
        [user + 2048, 95, user + 136, 0, 0, 0], 1), 0);
    copy_from_user(&tables, user + 136, &mut number).unwrap();
    let mut cwd = [0; b"/home/default/documents".len()];
    copy_from_user(&tables, user + 2048, &mut cwd).unwrap();
    assert_eq!(&cwd, b"/home/default/documents");
    assert_eq!(u64::from_le_bytes(number), cwd.len() as u64);
    copy_to_user(&mut tables, user + 256, b"int main(){return 54;}").unwrap();
    let args = [handle, user + 256, 22, user + 136, 0, 0];
    assert_eq!(session.file_syscall(owner, &caps, store, &mut tables, 0x120d,
        [handle, user + 256, 22, user + 8192, 0, 0], 1), -14);
    assert_eq!(session.file_syscall(owner, &caps, store, &mut tables, 0x120d, args, 1), 0);
    assert_eq!(session.file_syscall(owner, &caps, store, &mut tables, 0x1211,
        [handle, user + 1024, 0, 0, 0, 0], 1), 0);
    copy_from_user(&tables, user + 1032, &mut number).unwrap();
    assert_eq!(u64::from_le_bytes(number), 22);
    assert_eq!(session.file_syscall(owner, &caps, store, &mut tables, 0x120e,
        [handle, 0, 0, user + 136, 0, 0], 1), 0);
    // A read spanning a read-only page cannot advance the cursor or partly copy.
    assert_eq!(session.file_syscall(owner, &caps, store, &mut tables, 0x120c,
        [handle, user + 8190, 22, user + 136, 0, 0], 1), -14);
    assert_eq!(session.file_syscall(owner, &caps, store, &mut tables, 0x120c,
        [handle, user + 512, 22, user + 136, 0, 0], 1), 0);
    let mut text = [0u8;22];
    copy_from_user(&tables, user + 512, &mut text).unwrap();
    assert_eq!(&text, b"int main(){return 54;}");
    assert_eq!(session.file_syscall(owner, &caps, store, &mut tables, 0x1210,
        [handle, 18, user + 512, 2, user + 136, 0], 1), 0);
    let mut pair = [0;2];
    copy_from_user(&tables, user + 512, &mut pair).unwrap();
    assert_eq!(&pair, b"54");
    // Positional reads leave the stream at its previous end.
    assert_eq!(session.file_syscall(owner, &caps, store, &mut tables, 0x120c,
        [handle, user + 512, 1, user + 136, 0, 0], 1), 0);
    copy_from_user(&tables, user + 136, &mut number).unwrap();
    assert_eq!(u64::from_le_bytes(number), 0);
    assert_eq!(session.file_syscall(owner, &caps, store, &mut tables, 0x120c,
        [handle + (1 << 32), user + 512, 1, user + 136, 0, 0], 1), -9);
    assert_eq!(session.file_syscall(owner, &caps, store, &mut tables, 0x120d,
        [handle, user + 0x10000, 1, user + 136, 0, 0], 1), -14);
    assert_eq!(session.file_syscall(owner, &caps, store, &mut tables, 0x120f,
        [handle, 0, 0, 0, 0, 0], 1), 0);
    let id = store.resolve(b"/home/default/documents/abi-output.c").unwrap();
    let mut saved = [0u8;22];
    assert_eq!(store.read(id, None, &mut saved).unwrap(), 22);
    assert_eq!(saved, text);
    assert_eq!(session.file_syscall(owner, &caps, store, &mut tables, 0x1212,
        [user + 4090, user + 1024, 0, 0, 0, 0], 1), 0);
    let mut metadata = [0u8;40];
    copy_from_user(&tables, user + 1024, &mut metadata).unwrap();
    assert_eq!(u64::from_le_bytes(metadata[8..16].try_into().unwrap()), 22);
    assert_eq!(u32::from_le_bytes(metadata[28..32].try_into().unwrap()), 1);
    assert_eq!(u32::from_le_bytes(metadata[32..36].try_into().unwrap()), READ | WRITE);
    caps.revoke(write).unwrap();
    assert_eq!(session.file_syscall(owner, &caps, store, &mut tables, 0x1212,
        [user + 4090, user + 1024, 0, 0, 0, 0], 1), 0);
    copy_from_user(&tables, user + 1024, &mut metadata).unwrap();
    assert_eq!(u32::from_le_bytes(metadata[32..36].try_into().unwrap()), READ);
    assert_eq!(session.file_syscall(owner, &caps, store, &mut tables, 0x121a, [0;6], 1), -88);
    session.cancel();
    tables.release(&mut memory).unwrap();
    memory.release(data).unwrap();
    assert_eq!(memory.available_pages(), initial);
    unsafe { std::alloc::dealloc(arena, layout); }
}

// ------------------------=
// FUNC: service_acceptance
// DESC: Verifies real object mutations, revocation, expiry, cross-process rejection and cancellation through the typed service.
// ------------------=
fn service_acceptance(store: &mut ObjectStore<Disk>) {
    use capability::{CapabilityManager, CapabilityType};
    use execution::SecurityIdentity;
    use service::{Request, Session};
    let owner = SecurityIdentity([41; 16]);
    let other = SecurityIdentity([42; 16]);
    let mut caps = CapabilityManager::new();
    let read = caps.grant(CapabilityType::PersonalSpaceRead, 71, 1, 0, owner, owner, Some(100), 0).unwrap();
    let write = caps.grant(CapabilityType::PersonalSpaceWrite, 71, 1, 0, owner, owner, None, 0).unwrap();
    let mut session = Session::<4>::new(ObjectIo::<4>::documents(), owner, 71, read, write);
    let mut output = [0u8; 128];
    let path = b"/home/default/documents/service.c";
    let handle = session.execute(owner, &caps, store,
        Request::Open { path, flags: READ | WRITE | CREATE }, &mut output, 1).unwrap() as i32;
    session.execute(owner, &caps, store, Request::Write { handle, bytes: b"int main(){return 9;}" }, &mut output, 1).unwrap();
    session.execute(owner, &caps, store, Request::Flush(handle), &mut output, 1).unwrap();
    let id = store.resolve(path).unwrap();
    let version = store.metadata(id).unwrap().current_version;
    let size = store.read(id, None, &mut output).unwrap();
    assert_eq!(&output[..size], b"int main(){return 9;}");
    assert_eq!(session.execute(other, &caps, store, Request::Close(handle), &mut output, 1), Err(IoError::Denied));
    session.execute(owner, &caps, store, Request::Seek { handle, offset: 0, origin: 0 }, &mut output, 1).unwrap();
    session.execute(owner, &caps, store, Request::Write { handle, bytes: b"DO NOT COMMIT" }, &mut output, 1).unwrap();
    caps.revoke(write).unwrap();
    for request in [Request::Flush(handle), Request::Truncate { handle, length: 0 },
        Request::Write { handle, bytes: b"x" }, Request::Close(handle)] {
        assert_eq!(session.execute(owner, &caps, store, request, &mut output, 1), Err(IoError::Denied));
        assert_eq!(store.metadata(id).unwrap().current_version, version);
    }
    assert_eq!(session.execute(owner, &caps, store, Request::Read(handle), &mut output, 1), Err(IoError::BadHandle));
    let handle = session.execute(owner, &caps, store, Request::Open { path, flags: READ }, &mut output, 99).unwrap() as i32;
    output.fill(0xA5);
    assert_eq!(session.execute(owner, &caps, store, Request::Read(handle), &mut output, 100), Err(IoError::Denied));
    assert_eq!(output, [0xA5; 128]);
    assert_eq!(session.execute(owner, &caps, store, Request::Remove(path), &mut output, 1), Err(IoError::Denied));
    assert_eq!(store.resolve(path).unwrap(), id);
    let write = caps.grant(CapabilityType::PersonalSpaceWrite, 71, 1, 0, owner, owner, None, 0).unwrap();
    let mut cancelled = Session::<1>::new(ObjectIo::<1>::documents(), owner, 71, read, write);
    let handle = cancelled.execute(owner, &caps, store, Request::Open { path, flags: WRITE | TRUNCATE }, &mut output, 1).unwrap() as i32;
    cancelled.execute(owner, &caps, store, Request::Write { handle, bytes: b"cancelled" }, &mut output, 1).unwrap();
    cancelled.cancel();
    assert_eq!(cancelled.execute(owner, &caps, store, Request::Close(handle), &mut output, 1), Err(IoError::Denied));
    assert_eq!(store.metadata(id).unwrap().current_version, version);
    let mut wrong_scope = Session::<1>::new(ObjectIo::<1>::documents(), owner, 72, read, write);
    assert_eq!(wrong_scope.execute(owner, &caps, store, Request::Open { path, flags: WRITE }, &mut output, 1), Err(IoError::Denied));
}
