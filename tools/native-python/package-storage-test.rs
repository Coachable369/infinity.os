#[path = "../storage.rs"] mod storage;
#[path = "../../kernel/storage/spatial_path.rs"] mod spatial_path;
#[path = "../../sdk/infinity-browser-core/favorites.rs"] mod favorites;
#[path = "../../kernel/runtime/font_preferences.rs"] mod font_preferences;
#[path = "../../kernel/runtime/native_c_image.rs"] mod native_c_image;
#[path = "../../kernel/runtime/native_memory.rs"] mod native_memory;
#[path = "../../kernel/runtime/native_python_packages.rs"] mod packages;
use std::{cell::RefCell, rc::Rc};
use storage::{BlockDevice, object::{ObjectStore, ObjectId, ObjectError, STORE_RELATIVE_LBA}};

#[derive(Clone)]
struct Disk { bytes: Rc<RefCell<Vec<[u8; 512]>>>, fail: Rc<RefCell<Option<usize>>> }
impl BlockDevice for Disk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Reports exact fixture geometry to the production ObjectStore.
    // ------------------=
    fn block_count(&self) -> u64 { self.bytes.borrow().len() as u64 }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads persisted bytes without bypassing on-disk checksums or metadata roots.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
        let bytes = self.bytes.borrow(); let Some(sector) = bytes.get(lba as usize) else { return false; };
        *out = *sector; true
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Injects a bounded physical write failure at a caller-selected transaction step.
    // ------------------=
    fn write_sector(&mut self, lba: u64, value: &[u8; 512]) -> bool {
        if let Some(left) = self.fail.borrow_mut().as_mut() { if *left == 0 { return false; } *left -= 1; }
        let mut bytes = self.bytes.borrow_mut(); let Some(sector) = bytes.get_mut(lba as usize) else { return false; };
        *sector = *value; true
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Completes synchronous fixture writes.
    // ------------------=
    fn flush(&mut self) -> bool { true }
}

static mut VFS: packages::Vfs = packages::Vfs::new();

// ------------------------=
// FUNC: syscall_checks
// DESC: Exercises pointer validation, descriptor generations and write semantics through real user page tables.
// ------------------=
fn syscall_checks(vfs: &mut packages::Vfs) {
    use native_memory::{NativeMemory, X86AddressSpace, PagePermissions, copy_to_user, copy_from_user};
    let layout = std::alloc::Layout::from_size_align(4 * 1024 * 1024, 4096).unwrap();
    let arena = unsafe { std::alloc::alloc_zeroed(layout) }; assert!(!arena.is_null());
    let mut memory = NativeMemory::empty(); memory.initialize(arena as u64, layout.size() as u64).unwrap();
    let mut tables = X86AddressSpace::new(&mut memory, 9).unwrap();
    let pages = memory.allocate(4, 9).unwrap(); let base = 0x1_0000_0000;
    tables.map_range(&mut memory, base, pages.physical_address, 16384, PagePermissions::USER_RW).unwrap();
    vfs.reset(true);
    copy_to_user(&mut tables, base, b"/packages/probe.py\0").unwrap();
    assert_eq!(vfs.syscall(&mut tables, 0x120b, [base, 7, base + 65536, 0, 0, 0]), Some(-14));
    assert!(vfs.read(b"/packages/probe.py").is_err());
    assert_eq!(vfs.syscall(&mut tables, 0x120b, [base, (1u64 << 32) | 7, base + 256, 0, 0, 0]), Some(-22));
    assert!(!vfs.dirty());
    assert_eq!(vfs.syscall(&mut tables, 0x120b, [base, 7 | 128, base + 256, 0, 0, 0]), Some(0));
    let mut bytes = [0; 8]; copy_from_user(&tables, base + 256, &mut bytes).unwrap();
    let handle = u64::from_le_bytes(bytes);
    copy_to_user(&mut tables, base + 1024, b"abc").unwrap();
    assert_eq!(vfs.syscall(&mut tables, 0x120d, [handle, base + 1024, 3, base + 512, 0, 0]), Some(0));
    assert_eq!(vfs.read(b"/packages/probe.py").unwrap(), b"abc");
    assert_eq!(vfs.syscall(&mut tables, 0x120d, [handle, base + 65536, 3, base + 512, 0, 0]), Some(-14));
    assert_eq!(vfs.read(b"/packages/probe.py").unwrap(), b"abc");
    assert_eq!(vfs.syscall(&mut tables, 0x120e, [handle, 1000, 0, base + 512, 0, 0]), Some(0));
    assert_eq!(vfs.syscall(&mut tables, 0x120d, [handle, 0, 0, base + 512, 0, 0]), Some(0));
    assert_eq!(vfs.read(b"/packages/probe.py").unwrap(), b"abc");
    assert_eq!(vfs.syscall(&mut tables, 0x1213, [handle, 6, 0, 0, 0, 0]), Some(0));
    assert_eq!(vfs.read(b"/packages/probe.py").unwrap(), b"abc\0\0\0");
    assert_eq!(vfs.syscall(&mut tables, 0x120f, [handle, 0, 0, 0, 0, 0]), Some(0));
    assert_eq!(vfs.syscall(&mut tables, 0x120b, [base, 1, base + 256, 0, 0, 0]), Some(0));
    copy_from_user(&tables, base + 256, &mut bytes).unwrap(); assert_ne!(u64::from_le_bytes(bytes), handle);
    assert_eq!(vfs.syscall(&mut tables, 0x120c, [handle, base + 1024, 3, base + 512, 0, 0]), Some(-9));
    copy_to_user(&mut tables, base, b"/packages\0").unwrap();
    assert_eq!(vfs.syscall(&mut tables, 0x1206, [base, base + 256, 0, 0, 0, 0]), Some(0));
    copy_from_user(&tables, base + 256, &mut bytes).unwrap(); let directory = u64::from_le_bytes(bytes);
    assert_eq!(vfs.syscall(&mut tables, 0x1207, [directory, base + 65536, 256, 0, 0, 0]), Some(-14));
    assert_eq!(vfs.syscall(&mut tables, 0x1207, [directory, base + 1024, 256, 0, 0, 0]), Some(0));
    let mut name = [0; 9]; copy_from_user(&tables, base + 1024, &mut name).unwrap(); assert_eq!(&name, b"probe.py\0");
    assert!(vfs.put(b"/packages/pip", b"replace").is_err());
    assert_eq!(vfs.syscall(&mut tables, 0x1200, [0; 6]), None);
    tables.release(&mut memory).unwrap(); memory.release(pages).unwrap();
    unsafe { std::alloc::dealloc(arena, layout); }
}

// ------------------------=
// FUNC: main
// DESC: Verifies staged filesystem confinement, private archive remount, version fences and failed-write rollback using production implementations.
// ------------------=
fn main() {
    use packages::{admit_pip, PipMode, PolicyError};
    assert_eq!(admit_pip(&[b"list", b"--format=json"]), Ok(PipMode::ReadOnly));
    assert_eq!(admit_pip(&[b"freeze", b"--all"]), Ok(PipMode::ReadOnly));
    assert_eq!(admit_pip(&[b"install", b"/home/default/documents/demo.whl"]), Ok(PipMode::Install));
    assert_eq!(admit_pip(&[b"uninstall", b"-y", b"demo"]), Ok(PipMode::Uninstall));
    assert_eq!(admit_pip(&[b"uninstall", b"demo"]), Err(PolicyError::Confirmation));
    assert_eq!(admit_pip(&[b"install", b"--target=/system"]), Err(PolicyError::Option));
    assert_eq!(admit_pip(&[b"install", b"https://example.org/demo.whl"]), Err(PolicyError::Input));
    assert_eq!(admit_pip(&[b"install", b"PIP==26.2.1"]), Err(PolicyError::SelfModification));
    assert_eq!(admit_pip(&[b"install", b"pip-tools"]), Ok(PipMode::Install));
    assert_eq!(admit_pip(&[b"install", b"/work/pip-26.2.1-py3-none-any.whl"]), Err(PolicyError::SelfModification));
    assert_eq!(admit_pip(&[b"list", b"--outdated"]), Err(PolicyError::Option));
    let vfs = unsafe { &mut *(&raw mut VFS) };
    syscall_checks(vfs);
    let mut archive = vec![0; packages::MAX_ARCHIVE];
    vfs.reset(true);
    vfs.mkdir(b"/packages/demo").unwrap();
    let source = vec![0x5a; 96 * 1024];
    vfs.put(b"/packages/demo/__init__.py", &source).unwrap();
    vfs.mkdir(b"/tmp/stage").unwrap();
    vfs.put(b"/tmp/stage/module.py", b"answer = 42\n").unwrap();
    vfs.rename(b"/tmp/stage", b"/packages/staged").unwrap();
    vfs.mount_readonly(b"/work/demo.whl", b"wheel bytes").unwrap();
    assert!(vfs.put(b"/work/demo.whl", b"changed").is_err());
    assert!(vfs.put(b"/packages/../work/escape.py", b"bad").is_err());
    assert!(vfs.rename(b"/packages/staged", b"/work/escape").is_err());
    assert!(vfs.remove(b"/packages/staged", true).is_err());
    assert_eq!(vfs.read(b"/packages/staged/module.py").unwrap(), b"answer = 42\n");
    assert!(vfs.dirty());
    let length = vfs.encode(&mut archive).unwrap();
    vfs.reset(false); vfs.load_archive(&archive[..length]).unwrap();
    assert!(!vfs.dirty());
    assert_eq!(vfs.read(b"/packages/demo/__init__.py").unwrap(), source);
    assert!(vfs.read(b"/work/demo.whl").is_err());
    assert!(vfs.read(b"/tmp/stage/module.py").is_err());
    assert!(vfs.put(b"/packages/demo/__init__.py", b"inject").is_err());
    assert!(vfs.remove(b"/packages/staged/module.py", false).is_err());
    vfs.put(b"/tmp/runtime-data", b"transient").unwrap();
    assert!(!vfs.dirty());
    let mut malformed = archive[..length].to_vec(); malformed[16..18].copy_from_slice(&65535u16.to_le_bytes());
    vfs.reset(false); assert!(vfs.load_archive(&malformed).is_err());
    vfs.reset(true); vfs.load_archive(&archive[..length]).unwrap();
    vfs.remove(b"/packages/staged/module.py", false).unwrap(); vfs.remove(b"/packages/staged", true).unwrap();
    assert!(vfs.read(b"/packages/staged/module.py").is_err());

    let owner = ObjectId([0x41; 16]); let foreign = ObjectId([0x42; 16]);
    let sectors = STORE_RELATIVE_LBA as usize + 65536;
    let disk = Disk { bytes: Rc::new(RefCell::new(vec![[0; 512]; sectors])), fail: Rc::new(RefCell::new(None)) };
    let mut store = ObjectStore::format(disk.clone(), 0, sectors as u64, [0x53; 16]).unwrap();
    let mut out = vec![0; packages::MAX_ARCHIVE];
    assert_eq!(store.read_python_packages(owner, &mut out).unwrap(), (None, 0));
    let revision = store.save_python_packages(owner, None, &archive[..length]).unwrap();
    assert_eq!(revision.version, 1);
    assert_eq!(store.read_python_packages(foreign, &mut out).unwrap(), (None, 0));
    assert_eq!(store.save_python_packages(foreign, Some(revision), &archive[..length]), Err(ObjectError::InvalidVersion));
    assert_eq!(store.save_python_packages(owner, None, &archive[..length]), Err(ObjectError::InvalidVersion));
    assert_eq!(store.read(revision.id, None, &mut out), Err(ObjectError::Unauthorized));
    assert_eq!(store.write(revision.id, b"bad"), Err(ObjectError::Unauthorized));
    assert_eq!(store.attach(b"/home/default/documents/alias", revision.id), Err(ObjectError::Unauthorized));
    let private_path = store.namespace_ref_nth(revision.id, 0).unwrap().to_vec();
    let protected_generation = store.generation();
    assert_eq!(store.destroy_explicit(revision.id, true), Err(ObjectError::Unauthorized));
    assert_eq!(store.copy_attached(revision.id, b"/home/default/documents/copy"), Err(ObjectError::Unauthorized));
    assert_eq!(store.copy_path_attached(&private_path, b"/home/default/documents/path-copy"), Err(ObjectError::Unauthorized));
    assert_eq!(store.detach(&private_path), Err(ObjectError::Unauthorized));
    assert_eq!(store.remove_path(&private_path), Err(ObjectError::Unauthorized));
    assert_eq!(store.remove(revision.id), Err(ObjectError::Unauthorized));
    assert_eq!(store.move_entry(&private_path, b"/home/default/documents/moved"), Err(ObjectError::Unauthorized));
    assert_eq!(store.move_entry(b"/system", b"/moved-system"), Err(ObjectError::Unauthorized));
    assert_eq!(store.restore(revision.id, revision.version), Err(ObjectError::Unauthorized));
    let metadata = store.metadata(revision.id).unwrap();
    assert_eq!(store.update_metadata(storage::object::ObjectMetadataUpdateRequest {
        object: metadata.object, owner: foreign, content_type: metadata.content_type,
        tags: b"changed", flags: 0,
    }), Err(ObjectError::Unauthorized));
    assert_eq!(store.generation(), protected_generation);
    assert_eq!(store.resolve(&private_path).unwrap(), revision.id);
    assert!(store.copy_path_attached(b"/system", b"/copied-system").is_err());
    assert_eq!(store.generation(), protected_generation);
    assert!(store.resolve(b"/copied-system").is_err());
    let (protected, protected_length) = store.read_python_packages(owner, &mut out).unwrap();
    assert_eq!(protected, Some(revision));
    assert_eq!(&out[..protected_length], &archive[..length]);
    drop(store);
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    let (loaded, count) = store.read_python_packages(owner, &mut out).unwrap();
    assert_eq!(loaded, Some(revision)); assert_eq!(&out[..count], &archive[..length]);
    let initial_blocks = store.usage_blocks(); let mut current = revision;
    for _ in 0..80 { current = store.save_python_packages(owner, Some(current), &archive[..length]).unwrap(); }
    assert_eq!(store.usage_blocks(), initial_blocks);
    assert_eq!(current.id, revision.id); assert_eq!(current.version, 81);
    assert_eq!(store.save_python_packages(owner, Some(revision), &archive[..length]), Err(ObjectError::InvalidVersion));
    for fail_after in [0, 4, 33, 100] {
        *disk.fail.borrow_mut() = Some(fail_after);
        assert_eq!(store.save_python_packages(owner, Some(current), &archive[..length]), Err(ObjectError::TransactionFailed));
        *disk.fail.borrow_mut() = None;
        drop(store); store = ObjectStore::mount(disk.clone(), 0).unwrap();
        let (loaded, count) = store.read_python_packages(owner, &mut out).unwrap();
        assert_eq!(loaded, Some(current)); assert_eq!(&out[..count], &archive[..length]);
    }
    // The archive contract is eight MiB, independent of smaller download and
    // ordinary-object limits. Exercise the exact bound, not only a small wheel.
    vfs.reset(true);
    let full_file = vec![0x6b; 4 * 1024 * 1024];
    vfs.put(b"/packages/a", &full_file).unwrap();
    let overhead = 16 + 2 * (8 + b"/packages/a".len());
    let tail = vec![0x7c; packages::MAX_ARCHIVE - overhead - full_file.len()];
    vfs.put(b"/packages/b", &tail).unwrap();
    assert_eq!(vfs.encode(&mut archive).unwrap(), packages::MAX_ARCHIVE);
    current = store.save_python_packages(owner, Some(current), &archive).unwrap();
    drop(store); store = ObjectStore::mount(disk.clone(), 0).unwrap();
    let (loaded, count) = store.read_python_packages(owner, &mut out).unwrap();
    assert_eq!(loaded, Some(current)); assert_eq!(count, packages::MAX_ARCHIVE); assert_eq!(out, archive);
    vfs.reset(false); vfs.load_archive(&out).unwrap();
    assert_eq!(vfs.read(b"/packages/a").unwrap(), full_file);
    assert_eq!(vfs.read(b"/packages/b").unwrap(), tail);
    let generation = store.generation(); let blocks = store.usage_blocks();
    let mut oversized = archive.clone(); oversized.push(0);
    assert_eq!(store.save_python_packages(owner, Some(current), &oversized), Err(ObjectError::CorruptContent));
    assert_eq!(store.generation(), generation); assert_eq!(store.usage_blocks(), blocks);
    vfs.reset(true); vfs.load_archive(&archive).unwrap();
    let mut larger_tail = tail.clone(); larger_tail.push(0);
    vfs.put(b"/packages/b", &larger_tail).unwrap();
    assert_eq!(vfs.encode(&mut oversized[..packages::MAX_ARCHIVE]), Err(-28));
    let mut changed = archive.clone(); *changed.last_mut().unwrap() ^= 1;
    *disk.fail.borrow_mut() = Some(100);
    assert_eq!(store.save_python_packages(owner, Some(current), &changed), Err(ObjectError::TransactionFailed));
    *disk.fail.borrow_mut() = None;
    drop(store); store = ObjectStore::mount(disk.clone(), 0).unwrap();
    let (loaded, count) = store.read_python_packages(owner, &mut out).unwrap();
    assert_eq!(loaded, Some(current)); assert_eq!(count, packages::MAX_ARCHIVE); assert_eq!(out, archive);
    assert_eq!(store.generation(), generation); assert_eq!(store.usage_blocks(), blocks);
    assert_eq!(store.create(b"oversized-public", storage::object::ObjectType::Blob,
        storage::object::Space::Personal, &full_file), Err(ObjectError::InsufficientCapacity));
    assert!(store.save_python_packages(ObjectId([0; 16]), None, &archive[..length]).is_err());
    println!("Private package storage and filesystem behavior passed");
}
