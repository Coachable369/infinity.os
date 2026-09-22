mod storage;
#[path = "../kernel/storage/spatial_path.rs"]
mod spatial_path;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use storage::{
    object::{
        crc32, ContentType, ObjectCapabilityPolicy, ObjectCreateRequest, ObjectError,
        ObjectMetadataUpdateRequest, ObjectOperation, ObjectQueryRequest, ObjectRef, ObjectService,
        ObjectStore, ObjectType, RelationshipAttachRequest, RelationshipDetachRequest,
        RelationshipType, Space, BANK_A, BANK_B, FORMAT_VERSION, ROOT_A, ROOT_B,
        STORE_RELATIVE_LBA,
    },
    BlockDevice, DateTimeConfiguration,
};

#[derive(Clone)]
struct MemoryDisk(Rc<RefCell<Vec<[u8; 512]>>>);
impl MemoryDisk {
    // ------------------------=
    // FUNC: new
    // DESC: Creates and initializes a new instance.
    // ------------------=
    fn new(sectors: usize) -> Self {
        Self(Rc::new(RefCell::new(vec![[0; 512]; sectors])))
    }
    // ------------------------=
    // FUNC: flip
    // DESC: Implements the flip operation.
    // ------------------=
    fn flip(&self, lba: usize, offset: usize) {
        self.0.borrow_mut()[lba][offset] ^= 0x5a;
    }
    // ------------------------=
    // FUNC: set_version_and_rechecksum
    // DESC: Writes or updates set version and rechecksum data.
    // ------------------=
    fn set_version_and_rechecksum(&self, lba: usize, version: u32) {
        let mut disk = self.0.borrow_mut();
        let s = &mut disk[lba];
        s[8..12].copy_from_slice(&version.to_le_bytes());
        s[508..512].fill(0);
        let sum = crc32(s);
        s[508..512].copy_from_slice(&sum.to_le_bytes());
    }
}
impl BlockDevice for MemoryDisk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Implements the block count operation.
    // ------------------=
    fn block_count(&self) -> u64 {
        self.0.borrow().len() as u64
    }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads read sector data.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
        let disk = self.0.borrow();
        let Some(s) = disk.get(lba as usize) else {
            return false;
        };
        *out = *s;
        true
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes or updates write sector data.
    // ------------------=
    fn write_sector(&mut self, lba: u64, input: &[u8; 512]) -> bool {
        let mut disk = self.0.borrow_mut();
        let Some(s) = disk.get_mut(lba as usize) else {
            return false;
        };
        *s = *input;
        true
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Implements the flush operation.
    // ------------------=
    fn flush(&mut self) -> bool {
        true
    }
}

// ------------------------=
// FUNC: editor_documents_recovery
// DESC: Verifies saved document identity, collision-safe recovery, idempotence, and durable bytes after remount.
// ------------------=
fn editor_documents_recovery(sectors: usize) {
    let disk = MemoryDisk::new(sectors);
    let mut store = ObjectStore::format(disk.clone(), 0, sectors as u64, [0x92;16]).unwrap();
    let legacy = b"/personal/documents/helloworld.txt";
    let canonical = b"/home/default/documents/helloworld.txt";
    let original = store.create_attached(b"helloworld.txt", ObjectType::Text, Space::Personal, b"hello world", legacy).unwrap();
    assert_eq!(store.recover_editor_documents().unwrap(), 1);
    assert_eq!(store.resolve(canonical).unwrap(), original);
    assert!(store.resolve(legacy).is_err());
    let version = store.generation();
    assert_eq!(store.recover_editor_documents().unwrap(), 0);
    assert_eq!(store.generation(), version);
    // A distinct older file with the same name must survive alongside the canonical file.
    let conflicting = store.create_attached(b"helloworld.txt", ObjectType::Text, Space::Personal, b"older text", legacy).unwrap();
    assert_eq!(store.recover_editor_documents().unwrap(), 1);
    assert_eq!(store.resolve(canonical).unwrap(), original);
    assert_eq!(store.recover_editor_documents().unwrap(), 0);
    let long_name = [b'x';63];
    let mut long_legacy = b"/personal/documents/".to_vec();
    long_legacy.extend_from_slice(&long_name);
    let mut long_canonical = b"/home/default/documents/".to_vec();
    long_canonical.extend_from_slice(&long_name);
    let long_original = store.create_attached(b"long-original", ObjectType::Text, Space::Personal, b"keep", &long_canonical).unwrap();
    store.create_attached(b"long-legacy", ObjectType::Text, Space::Personal, b"recover", &long_legacy).unwrap();
    assert_eq!(store.recover_editor_documents().unwrap(), 1);
    assert_eq!(store.resolve(&long_canonical).unwrap(), long_original);
    assert!(store.resolve(&long_legacy).is_err());
    drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    let mut ids = Vec::new();
    for index in 0..256 {
        let Some(entry) = store.namespace_list_nth(storage::object::DOCUMENTS_PATH, index) else { break; };
        if entry.path_len as usize > storage::object::DOCUMENTS_PATH.len() { ids.push(entry.object.id); }
    }
    assert!(ids.contains(&original) && ids.contains(&conflicting));
    let mut bytes = [0;32];
    assert_eq!(store.read(original, None, &mut bytes).unwrap(), 11);
    assert_eq!(&bytes[..11], b"hello world");
    assert_eq!(store.read(conflicting, None, &mut bytes).unwrap(), 10);
    assert_eq!(&bytes[..10], b"older text");
}

// ------------------------=
// FUNC: hello_document_provisioning
// DESC: Verifies the fresh-store C document is discoverable, typed as editable text, and preserves saved changes across remounts.
// ------------------=
fn hello_document_provisioning(sectors: usize) {
    let disk = MemoryDisk::new(sectors);
    let mut store = ObjectStore::format(disk.clone(), 0, sectors as u64, [0x93;16]).unwrap();
    let id = store.resolve(b"/home/default/documents/hello.c").unwrap();
    let metadata = store.metadata(id).unwrap();
    assert_eq!(metadata.kind, ObjectType::Text);
    assert_eq!(metadata.content_type, ContentType::Utf8Text);
    assert_eq!(metadata.space, Space::Personal);
    let listed = (0..32).filter_map(|index|
        store.namespace_list_nth(storage::object::DOCUMENTS_PATH, index)
    ).any(|entry| entry.object.id == id);
    assert!(listed);
    let mut bytes = [0u8;1024];
    let len = store.read(id, None, &mut bytes).unwrap();
    assert_eq!(&bytes[..len], include_bytes!("../sdk/c/examples/hello.c"));
    let edited = b"int main(void) { return 23; }\n";
    let version = store.write(id, edited).unwrap();
    drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    assert_eq!(store.resolve(b"/home/default/documents/hello.c").unwrap(), id);
    assert_eq!(store.metadata(id).unwrap().current_version, version);
    let len = store.read(id, None, &mut bytes).unwrap();
    assert_eq!(&bytes[..len], edited);
}

#[derive(Clone)]
struct FailingDisk {
    inner: MemoryDisk,
    remaining: Rc<Cell<Option<usize>>>,
}
impl FailingDisk {
    fn new(inner: MemoryDisk) -> Self {
        Self {
            inner,
            remaining: Rc::new(Cell::new(None)),
        }
    }
    // ------------------------=
    // FUNC: arm
    // DESC: Implements the arm operation.
    // ------------------=
    fn arm(&self, writes: usize) {
        self.remaining.set(Some(writes))
    }
    fn disarm(&self) {
        self.remaining.set(None)
    }
}
impl BlockDevice for FailingDisk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Implements the block count operation.
    // ------------------=
    fn block_count(&self) -> u64 {
        self.inner.block_count()
    }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads read sector data.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
        self.inner.read_sector(lba, out)
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes or updates write sector data.
    // ------------------=
    fn write_sector(&mut self, lba: u64, input: &[u8; 512]) -> bool {
        if let Some(left) = self.remaining.get() {
            if left == 0 {
                return false;
            }
            self.remaining.set(Some(left - 1));
        }
        self.inner.write_sector(lba, input)
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Implements the flush operation.
    // ------------------=
    fn flush(&mut self) -> bool {
        true
    }
}
struct Deny;
impl ObjectCapabilityPolicy for Deny {
    fn authorize(&self, _: ObjectOperation, _: Option<ObjectRef>) -> bool {
        false
    }
}

// ------------------------=
// FUNC: main
// DESC: Runs the program entry point.
// ------------------=
fn main() {
    let test_sectors = STORE_RELATIVE_LBA as usize + 32_768;
    private_spatial_checkpoint(test_sectors);
    legacy_store_mount(test_sectors);
    checkpoint_replacement(test_sectors);
    editor_documents_recovery(test_sectors);
    hello_document_provisioning(test_sectors);
    let disk = MemoryDisk::new(test_sectors);
    let seed = [0x41; 16];
    let mut completed_stages = Vec::new();
    let mut store = ObjectStore::format_with_progress(disk.clone(), 0, test_sectors as u64, seed,
        &mut |stage, _| completed_stages.push(stage)).expect("format");
    assert_eq!(completed_stages, [56, 57, 59, 60, 62, 64, 65, 66]);
    assert!(completed_stages.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(store.runtime_bootstrap_valid());
    // This fixture exercises all five free namespace slots. The independent
    // provisioning test above verifies the sample; delete it here as a user
    // may do, retaining the same namespace budget for the lifecycle tests.
    store.remove_path(b"/home/default/documents/hello.c").unwrap();
    assert!(store.resolve(b"/home/default/documents").is_ok());
    let documents = store
        .resolve(b"/home/default/documents")
        .expect("documents reference");
    store
        .attach(b"/home/default/desktop/documents", documents)
        .expect("desktop reference");
    let id = store
        .create(
            b"hello",
            ObjectType::Text,
            Space::Personal,
            b"Hello Infinity",
        )
        .expect("create");
    store
        .attach(b"/home/default/documents/hello", id)
        .expect("attach");
    assert_eq!(store.write(id, b"Version 2").unwrap(), 2);
    assert_eq!(store.write(id, b"Version 3").unwrap(), 3);
    store
        .attach(b"/home/default/archive/hello", id)
        .expect("second reference");
    store
        .move_entry(
            b"/home/default/documents/hello",
            b"/home/default/projects/hello",
        )
        .expect("move");
    assert_eq!(
        store.resolve(b"/home/default/documents/hello"),
        Err(ObjectError::NamespaceNotFound)
    );
    assert_eq!(store.resolve(b"/home/default/projects/hello").unwrap(), id);
    assert_eq!(store.resolve(b"/home/default/archive/hello").unwrap(), id);
    assert_eq!(store.namespace_refs(id), 2);
    assert_eq!(store.history_count(id), 3);

    let owner = store
        .create(b"owner", ObjectType::Metadata, Space::System, b"identity")
        .unwrap();
    store
        .update_metadata(ObjectMetadataUpdateRequest {
            object: ObjectRef { id },
            owner,
            content_type: ContentType::Utf8Text,
            tags: b"welcome",
            flags: 0x21,
        })
        .unwrap();
    let metadata = store.metadata(id).unwrap();
    assert_eq!(metadata.owner, owner);
    assert_eq!(metadata.tags_len, 7);
    assert_eq!(&metadata.tags[..7], b"welcome");
    assert_eq!(metadata.flags, 0x21);
    store
        .relationship_attach(RelationshipAttachRequest {
            source: ObjectRef { id },
            kind: RelationshipType::GeneratedBy,
            target: ObjectRef { id: owner },
            flags: 7,
        })
        .unwrap();
    let relation = store.relationship_nth(id, 0).unwrap();
    assert_eq!(relation.target.id, owner);
    assert_eq!(relation.kind, RelationshipType::GeneratedBy);
    assert_eq!(relation.flags, 7);
    let query = ObjectQueryRequest {
        kind: Some(ObjectType::Text),
        space: Some(Space::Personal),
        include_tombstones: false,
    };
    let matches: Vec<_> = (0..52).filter_map(|index| store.query_nth(query, index))
        .map(|entry| entry.object.id).collect();
    assert!(matches.contains(&id));

    let large = [0xA5u8; 9000];
    let large_id = store
        .create(b"large", ObjectType::Blob, Space::Applications, &large)
        .unwrap();
    let mut large_out = [0u8; 9000];
    assert_eq!(
        store.read(large_id, None, &mut large_out).unwrap(),
        large.len()
    );
    assert_eq!(large_out, large);
    assert_eq!(store.usage_by_space(Space::Applications), 3);
    assert!(store.usage_by_space(Space::Personal) >= 3);
    assert!(store.usage_by_space(Space::System) >= 2);
    assert_eq!(store.usage_by_space(Space::Recovery), 1);

    let used_before = store.usage_blocks();
    let generation_before = store.generation();
    assert_eq!(
        store.create_attached(
            b"rejected",
            ObjectType::Text,
            Space::Personal,
            b"must roll back",
            b"/home"
        ),
        Err(ObjectError::NameConflict)
    );
    assert_eq!(store.usage_blocks(), used_before);
    assert_eq!(store.generation(), generation_before);

    let disposable = store
        .create_attached(
            b"disposable",
            ObjectType::Text,
            Space::Personal,
            b"temporary",
            b"/home/default/documents/disposable",
        )
        .unwrap();
    store
        .attach(b"/home/default/archive/disposable", disposable)
        .unwrap();
    assert_eq!(
        store
            .remove_path(b"/home/default/documents/disposable")
            .unwrap(),
        disposable
    );
    assert!(store.object_exists(disposable));
    assert_eq!(store.namespace_refs(disposable), 1);
    assert_eq!(
        store
            .remove_path(b"/home/default/archive/disposable")
            .unwrap(),
        disposable
    );
    assert!(!store.object_exists(disposable));
    // Reclaim the deleted fixture before creating the later garbage-collection
    // case. CORE System Generation components intentionally consume most of the
    // compact bootstrap catalog, so tests must respect the same bounded capacity.
    assert!(store.collect().unwrap() >= 1);
    let generation = store.generation();
    drop(store);

    let mut rebooted = ObjectStore::mount(disk.clone(), 0).expect("reboot mount");
    assert_eq!(
        rebooted
            .resolve(b"/home/default/desktop/documents")
            .unwrap(),
        documents
    );
    assert_eq!(rebooted.generation(), generation);
    assert_eq!(
        rebooted.resolve(b"/home/default/archive/hello").unwrap(),
        id
    );
    let mut content = [0u8; 4096];
    let n = rebooted.read(id, None, &mut content).unwrap();
    assert_eq!(&content[..n], b"Version 3");
    assert_eq!(rebooted.restore(id, 1).unwrap(), 4);
    drop(rebooted);
    let mut rebooted = ObjectStore::mount(disk.clone(), 0).expect("restore reboot");
    let n = rebooted.read(id, None, &mut content).unwrap();
    assert_eq!(&content[..n], b"Hello Infinity");
    assert_eq!(rebooted.history_count(id), 4);
    assert_eq!(rebooted.metadata(id).unwrap().owner, owner);
    assert_eq!(rebooted.relationship_nth(id, 0).unwrap().target.id, owner);
    rebooted
        .relationship_detach(RelationshipDetachRequest {
            source: ObjectRef { id },
            kind: RelationshipType::GeneratedBy,
            target: ObjectRef { id: owner },
        })
        .unwrap();

    let garbage = rebooted
        .create(b"garbage", ObjectType::Blob, Space::Recovery, b"old")
        .unwrap();
    let before_gc = rebooted.usage_blocks();
    rebooted.remove(garbage).unwrap();
    assert!(rebooted
        .query_nth(
            ObjectQueryRequest {
                kind: None,
                space: Some(Space::Recovery),
                include_tombstones: true
            },
            0
        )
        .is_some());
    let reclaimed = rebooted.collect().unwrap();
    assert!(reclaimed >= 1);
    assert_eq!(rebooted.usage_blocks() + reclaimed, before_gc);

    let committed = rebooted.generation();
    let inactive = if committed & 1 == 1 { BANK_A } else { BANK_B };
    disk.flip((STORE_RELATIVE_LBA + inactive + 7) as usize, 100);
    drop(rebooted);
    let rebooted = ObjectStore::mount(disk.clone(), 0).expect("ignore uncommitted bank");
    assert_eq!(rebooted.generation(), committed);
    drop(rebooted);
    disk.flip((STORE_RELATIVE_LBA + inactive + 7) as usize, 100);

    let newest_root = if committed & 1 == 1 { ROOT_B } else { ROOT_A };
    disk.flip((STORE_RELATIVE_LBA + newest_root) as usize, 24);
    let fallback = ObjectStore::mount(disk.clone(), 0).expect("root fallback");
    assert!(fallback.generation() < committed);
    drop(fallback);
    disk.flip((STORE_RELATIVE_LBA + BANK_A + 5) as usize, 70);
    disk.flip((STORE_RELATIVE_LBA + BANK_B + 5) as usize, 70);
    assert!(matches!(
        ObjectStore::mount(disk.clone(), 0),
        Err(ObjectError::CorruptMetadata)
    ));

    let content_disk = MemoryDisk::new(test_sectors);
    let mut content_store =
        ObjectStore::format(content_disk.clone(), 0, test_sectors as u64, seed).unwrap();
    let content_id = content_store
        .create(
            b"corrupt",
            ObjectType::Text,
            Space::Personal,
            b"verified bytes",
        )
        .unwrap();
    // Bootstrap contents precede this object: kernel, recovery, runtime,
    // service registry, capability policy, local model, AI bootstrap, voice
    // framework, agent policy, organization schema, identity state, and native
    // network state, and declarative shell-profile state.
    content_disk.flip(
        (STORE_RELATIVE_LBA + 80 + storage::object::BOOTSTRAP_CONTENT_OBJECTS * 8) as usize,
        0,
    );
    let mut out = [0u8; 4096];
    assert_eq!(
        content_store.read(content_id, None, &mut out),
        Err(ObjectError::CorruptContent)
    );

    for (offset, label) in [
        (1u64, "allocation"),
        (5, "object"),
        (15, "namespace"),
        (23, "relationship"),
    ] {
        let corrupt = MemoryDisk::new(test_sectors);
        drop(ObjectStore::format(corrupt.clone(), 0, test_sectors as u64, seed).unwrap());
        corrupt.flip((STORE_RELATIVE_LBA + BANK_B + offset) as usize, 80);
        assert!(
            matches!(
                ObjectStore::mount(corrupt, 0),
                Err(ObjectError::CorruptMetadata)
            ),
            "{label}"
        );
    }

    let unsupported = MemoryDisk::new(test_sectors);
    drop(ObjectStore::format(unsupported.clone(), 0, test_sectors as u64, seed).unwrap());
    unsupported
        .set_version_and_rechecksum((STORE_RELATIVE_LBA + ROOT_B) as usize, FORMAT_VERSION + 1);
    assert!(matches!(
        ObjectStore::mount(unsupported, 0),
        Err(ObjectError::UnsupportedFormat)
    ));

    for writes in [8usize, 20, 33] {
        let backing = MemoryDisk::new(test_sectors);
        let failing = FailingDisk::new(backing.clone());
        let mut crash = ObjectStore::format(failing.clone(), 0, test_sectors as u64, seed).unwrap();
        let stable = crash
            .create(b"stable", ObjectType::Text, Space::Personal, b"before")
            .unwrap();
        let generation = crash.generation();
        failing.arm(writes);
        assert_eq!(
            crash.write(stable, b"after"),
            Err(ObjectError::TransactionFailed)
        );
        failing.disarm();
        drop(crash);
        let mut recovered = ObjectStore::mount(backing, 0).unwrap();
        let mut data = [0u8; 32];
        let n = recovered.read(stable, None, &mut data).unwrap();
        assert_eq!(&data[..n], b"before");
        assert_eq!(recovered.generation(), generation);
    }
    for writes in [5usize, 25] {
        let backing = MemoryDisk::new(test_sectors);
        let failing = FailingDisk::new(backing.clone());
        let mut crash = ObjectStore::format(failing.clone(), 0, test_sectors as u64, seed).unwrap();
        let stable = crash
            .create(b"stable", ObjectType::Text, Space::Personal, b"before")
            .unwrap();
        let generation = crash.generation();
        failing.arm(writes);
        assert_eq!(
            crash.attach(b"/home/default/documents/crash", stable),
            Err(ObjectError::TransactionFailed)
        );
        failing.disarm();
        drop(crash);
        let recovered = ObjectStore::mount(backing, 0).unwrap();
        assert_eq!(recovered.generation(), generation);
        assert_eq!(
            recovered.resolve(b"/home/default/documents/crash"),
            Err(ObjectError::NamespaceNotFound)
        );
    }

    let policy_disk = MemoryDisk::new(test_sectors);
    let mut policy_store = ObjectStore::format(policy_disk, 0, test_sectors as u64, seed).unwrap();
    let mut denied = ObjectService::new(&mut policy_store, &Deny);
    assert_eq!(
        denied.create(ObjectCreateRequest {
            name: b"blocked",
            kind: ObjectType::Text,
            space: Space::Personal,
            content: b"no"
        }),
        Err(ObjectError::Unauthorized)
    );

    let time_disk = MemoryDisk::new(test_sectors);
    let mut time_store =
        ObjectStore::format(time_disk.clone(), 0, test_sectors as u64, seed).unwrap();
    let configured_time = DateTimeConfiguration {
        year: 2026,
        month: 9,
        day: 4,
        hour: 14,
        minute: 30,
        second: 0,
        time_zone_id: 3,
        utc_offset_minutes: -360,
    };
    time_store
        .install_date_time_configuration(configured_time)
        .expect("install date/time configuration");
    assert_eq!(time_store.date_time_configuration(), Some(configured_time));
    drop(time_store);
    let mut time_rebooted = ObjectStore::mount(time_disk, 0).expect("date/time reboot mount");
    assert_eq!(
        time_rebooted.date_time_configuration(),
        Some(configured_time)
    );

    let editor_disk = MemoryDisk::new(test_sectors);
    let mut editor_store =
        ObjectStore::format(editor_disk.clone(), 0, test_sectors as u64, seed).unwrap();
    let first_document = editor_store
        .create_attached(
            b"first",
            ObjectType::Text,
            Space::Personal,
            b"alpha",
            b"/personal/documents/first",
        )
        .unwrap();
    let second_document = editor_store
        .create_attached(
            b"second",
            ObjectType::Text,
            Space::Personal,
            b"beta",
            b"/personal/documents/second",
        )
        .unwrap();
    assert_ne!(first_document, second_document);
    assert_eq!(
        editor_store
            .namespace_list_nth(b"/personal/documents/", 0)
            .unwrap()
            .object
            .id,
        first_document
    );
    assert_eq!(
        editor_store
            .namespace_list_nth(b"/personal/documents/", 1)
            .unwrap()
            .object
            .id,
        second_document
    );
    assert_eq!(
        editor_store
            .write(first_document, b"alpha revised")
            .unwrap(),
        2
    );
    drop(editor_store);
    let mut editor_rebooted = ObjectStore::mount(editor_disk, 0).unwrap();
    let mut editor_content = [0u8; 32];
    let editor_length = editor_rebooted
        .read(first_document, None, &mut editor_content)
        .unwrap();
    assert_eq!(&editor_content[..editor_length], b"alpha revised");
    assert_eq!(
        editor_rebooted
            .resolve(b"/personal/documents/second")
            .unwrap(),
        second_document
    );

    println!("PASS: native IDs, typed metadata/query, persistent date/time settings, persistent relationships, multi-extent COW, per-Space accounting, conservative GC, namespace identity, reboot/restore, five crash boundaries, format rejection, root/allocation/object/namespace/relationship/content corruption detection");
}

// ------------------------=
// FUNC: private_spatial_checkpoint
// DESC: Exercises fresh-store spatial persistence, alias/copy isolation, replacement and cold remount.
// ------------------=
fn private_spatial_checkpoint(sectors:usize) {
    let disk=MemoryDisk::new(sectors);
    let mut store=ObjectStore::format(disk.clone(),0,sectors as u64,[0x94;16]).unwrap();
    let checkpoint_path = spatial_path::owner_path([0x11; 16]);
    let path = checkpoint_path.as_slice();
    let id=store.create_attached(b"@spatial-state",ObjectType::Metadata,Space::System,&[7;8192],path).unwrap();
    let mut bytes=[0;8192];
    assert_eq!(store.read(id,None,&mut bytes),Err(ObjectError::Unauthorized));
    assert_eq!(store.write(id,b"replace"),Err(ObjectError::Unauthorized));
    assert_eq!(store.remove(id),Err(ObjectError::Unauthorized));
    assert_eq!(store.copy_attached(id,b"/home/default/leaked"),Err(ObjectError::Unauthorized));
    store.attach(b"/home/default/alias",id).unwrap();
    assert_eq!(store.read(store.resolve(b"/home/default/alias").unwrap(),None,&mut bytes),Err(ObjectError::Unauthorized));
    assert_eq!(store.read_spatial_state(id,&mut bytes),Ok(8192));assert_eq!(bytes,[7;8192]);
    for byte in 8..12 {store.replace_state(id,&[byte;8192]).unwrap();}
    assert_eq!(store.history_count(id),1);
    drop(store);
    let mut mounted=ObjectStore::mount(disk,0).unwrap();
    assert_eq!(mounted.resolve(path),Ok(id));
    assert_eq!(mounted.read_spatial_state(id,&mut bytes),Ok(8192));assert_eq!(bytes,[11;8192]);
    assert_eq!(mounted.read(id,None,&mut bytes),Err(ObjectError::Unauthorized));
}

// ------------------------=
// FUNC: legacy_store_mount
// DESC: Verifies reads and durable mutations at the historical store offset without migrating or overwriting it.
// ------------------=
fn legacy_store_mount(sectors: usize) {
    let disk = MemoryDisk::new(sectors);
    drop(ObjectStore::format(disk.clone(), 0, sectors as u64, [0x42; 16]).unwrap());
    let old = storage::layout::LEGACY_STORE_RELATIVE_LBA as usize;
    let new = STORE_RELATIVE_LBA as usize;
    {
        let mut data = disk.0.borrow_mut();
        data.copy_within(new..sectors, old);
        data[new..].fill([0; 512]);
    }
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    assert!(store.runtime_bootstrap_valid());
    let documents = store.resolve(b"/home/default/documents").unwrap();
    store.attach(b"/home/default/legacy-link", documents).unwrap();
    drop(store);
    let mounted = ObjectStore::mount(disk.clone(), 0).unwrap();
    assert_eq!(mounted.resolve(b"/home/default/legacy-link").unwrap(), documents);
    assert_eq!(disk.0.borrow()[new], [0; 512]);
}

// ------------------------=
// FUNC: checkpoint_replacement
// DESC: Verifies saturated checkpoint replacement, retained user history, and interrupted-write recovery.
// ------------------=
fn checkpoint_replacement(sectors: usize) {
    for failure in [None, Some(0usize), Some(8), Some(20), Some(33)] {
        let backing = MemoryDisk::new(sectors);
        let disk = FailingDisk::new(backing.clone());
        let mut store = ObjectStore::format(disk.clone(), 0, sectors as u64, [0x42; 16]).unwrap();
        let user = store.create(b"user", ObjectType::Text, Space::Personal, b"original").unwrap();
        store.write(user, b"edited").unwrap();
        let state = store.resolve(b"/system/security/nodes/state").unwrap();
        // Reproduce the installed failure through the existing versioned API.
        while store.write(state, b"before").is_ok() {}
        assert_eq!(store.write(state, b"after"), Err(ObjectError::InsufficientCapacity));
        let generation = store.generation();
        if let Some(writes) = failure { disk.arm(writes); }
        let result = store.replace_state(state, b"after");
        disk.disarm();
        if failure.is_some() {
            assert_eq!(result, Err(ObjectError::TransactionFailed));
            assert_eq!(store.generation(), generation);
        } else {
            result.unwrap();
            for _ in 0..64 { store.replace_state(state, b"after").unwrap(); }
            assert_eq!(store.history_count(state), 1);
        }
        drop(store);
        let mut recovered = ObjectStore::mount(backing, 0).unwrap();
        let mut data = [0u8; 32];
        let length = recovered.read(state, None, &mut data).unwrap();
        assert_eq!(&data[..length], if failure.is_some() { &b"before"[..] } else { &b"after"[..] });
        assert_eq!(recovered.history_count(user), 2);
        let length = recovered.read(user, Some(1), &mut data).unwrap();
        assert_eq!(&data[..length], b"original");
        recovered.replace_state(state, b"retry").unwrap();
        let length = recovered.read(state, None, &mut data).unwrap();
        assert_eq!(&data[..length], b"retry");
    }
}
