use crate::{native_fabric::NativeReplica, runtime::fabric::{replica::*, resources::ResourceId},
    storage::{BlockDevice, object::{ObjectStore, ObjectType, Space, STORE_RELATIVE_LBA}}};
use sha2::{Digest, Sha256};
use std::{cell::RefCell, rc::Rc, collections::BTreeMap};

#[derive(Clone, Default)]
struct Disk(Rc<RefCell<DiskState>>);
#[derive(Default)]
struct DiskState { sectors: BTreeMap<u64, [u8; 512]>, writes_left: Option<usize>, writes: usize }
impl BlockDevice for Disk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Exposes a sparse host fixture with enough blocks for native object formatting.
    // ------------------=
    fn block_count(&self) -> u64 { STORE_RELATIVE_LBA + 32768 }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads actual persisted binary sectors from the fixture device.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
        if lba >= self.block_count() { return false; }
        *out = self.0.borrow().sectors.get(&lba).copied().unwrap_or([0; 512]); true
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Injects a deterministic power-loss boundary after a selected count of successful sector writes.
    // ------------------=
    fn write_sector(&mut self, lba: u64, bytes: &[u8; 512]) -> bool {
        if lba >= self.block_count() { return false; }
        let mut state = self.0.borrow_mut();
        if let Some(left) = state.writes_left.as_mut() { if *left == 0 { return false; } *left -= 1; }
        state.sectors.insert(lba, *bytes); state.writes += 1; true
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Models synchronous stable-sector persistence in the host fixture.
    // ------------------=
    fn flush(&mut self) -> bool { true }
}

// ------------------------=
// FUNC: descriptor
// DESC: Constructs an exact immutable replica transfer fixture with a real SHA-256 content digest.
// ------------------=
fn descriptor(bytes: &[u8]) -> ReplicaDescriptor {
    ReplicaDescriptor { job: 1, object: [2; 16], version: 3, resource: ResourceId([4; 16]),
        generation: 5, bytes: bytes.len() as u64, hash: Sha256::digest(bytes).into() }
}

// ------------------------=
// FUNC: manifest_commit_is_generation_fenced_and_crash_atomic
// DESC: Cuts every native manifest commit write, remounts, and proves readers only see a coherent old or new policy/version generation.
// ------------------=
#[test]
fn manifest_commit_is_generation_fenced_and_crash_atomic() {
    use crate::{native_fabric::{commit_manifest, load_manifest}, runtime::{fabric::{manifest::*, placement::*}, node::types::NodeId}};
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let app = store.create(b"application", ObjectType::Metadata, Space::Personal, &[]).unwrap();
    let backing = store.create(b"manifest", ObjectType::Metadata, Space::System, &[]).unwrap();
    assert_ne!(app, backing);
    let first = Manifest { object: app.0, version: 1, length: 0, hash: Sha256::digest([]).into(),
        policy: StorageClass::Critical, generation: 1, authority: NodeId([8; 32]), authority_generation: 1,
        chunks: [None; MAX_CHUNKS], placements: [None; MAX_PLACEMENTS], healing: None };
    commit_manifest(&mut store, backing, 0, &first).unwrap();
    assert_eq!(load_manifest(&mut store, backing, backing), Err(ManifestError::Conflict));
    let mut next = first; next.generation = 2; next.policy = StorageClass::Protected;
    assert_eq!(commit_manifest(&mut store, backing, 0, &next), Err(ManifestError::Stale));
    let baseline = disk.0.borrow().sectors.clone(); disk.0.borrow_mut().writes = 0;
    commit_manifest(&mut store, backing, 1, &next).unwrap();
    let count = disk.0.borrow().writes; assert!(count > 0);
    commit_manifest(&mut store, backing, 1, &next).unwrap();
    assert_eq!(disk.0.borrow().writes, count);
    let mut changed = next; changed.hash = [9; 32];
    assert!(commit_manifest(&mut store, backing, 2, &changed).is_err());
    for cut in 0..=count {
        let disk = Disk(Rc::new(RefCell::new(DiskState { sectors: baseline.clone(), writes_left: Some(cut), writes: 0 })));
        let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
        let _ = commit_manifest(&mut store, backing, 1, &next);
        drop(store); disk.0.borrow_mut().writes_left = None;
        let mut store = ObjectStore::mount(disk, 0).unwrap();
        let loaded = load_manifest(&mut store, backing, app).unwrap();
        assert!(loaded == first || loaded == next);
        assert_eq!(loaded.availability(), Availability::Offline);
        commit_manifest(&mut store, backing, 1, &next).unwrap();
        assert_eq!(load_manifest(&mut store, backing, app), Ok(next));
    }
}

// ------------------------=
// FUNC: empty_replica_requires_verified_durable_publication
// DESC: Proves empty content remains staged until its real empty digest is verified and survives remount without fabricated chunks.
// ------------------=
#[test]
fn empty_replica_requires_verified_durable_publication() {
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let backing = store.create(b"empty-replica", ObjectType::Metadata, Space::System, &[]).unwrap();
    let d = descriptor(&[]);
    {
        let mut native = NativeReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
        let mut transfer = Transfer::begin(&mut native, d).unwrap();
        assert_eq!(native.read_verified(&mut []), Err(ReplicaError::Incomplete));
        assert_eq!(transfer.receive(&mut native, 0, &[]), Err(ReplicaError::Invalid));
        assert_eq!(transfer.verify_tick(&mut native), Ok(ReplicaState::Available));
    }
    drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    {
        let mut native = NativeReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
        assert_eq!(native.read_verified(&mut []), Ok(0));
        assert_eq!(native.inspect().unwrap().descriptor, d);
    }
    let corrupt = store.create(b"bad-empty-digest", ObjectType::Metadata, Space::System, &[]).unwrap();
    let mut native = NativeReplica::open(&mut store, corrupt, d.resource, d.generation).unwrap();
    let mut transfer = Transfer::begin(&mut native, ReplicaDescriptor { hash: [0; 32], ..d }).unwrap();
    assert_eq!(transfer.verify_tick(&mut native), Err(ReplicaError::Integrity));
    assert_eq!(native.read_verified(&mut []), Err(ReplicaError::Incomplete));
}

// ------------------------=
// FUNC: native_copy_shares_content_and_reclaims_only_last_reference
// DESC: Proves physical allocation sharing, independent versions, checkpoint replacement, deletion and remount recovery through the native store.
// ------------------=
#[test]
fn native_copy_shares_content_and_reclaims_only_last_reference() {
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let source = store.create(b"source", ObjectType::Metadata, Space::Personal, &[7; 8192]).unwrap();
    let used = store.usage_blocks();
    let space_used = store.usage_by_space(Space::Personal);
    let copy = store.copy_attached(source, b"/home/copy").unwrap();
    assert_ne!(copy, source);
    assert_eq!(store.usage_blocks(), used);
    assert_eq!(store.usage_by_space(Space::Personal), space_used);
    assert_eq!(store.current_version(copy), Ok(1));
    store.replace_state(source, &[8; 8192]).unwrap();
    let mut content = [0; 8192];
    assert_eq!(store.read(copy, None, &mut content), Ok(8192));
    assert_eq!(content, [7; 8192]);
    store.write(copy, &[9; 8192]).unwrap();
    store.read(source, None, &mut content).unwrap();
    assert_eq!(content, [8; 8192]);
    drop(store);
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    store.read(copy, Some(1), &mut content).unwrap();
    assert_eq!(content, [7; 8192]);
    store.destroy_explicit(source, true).unwrap();
    assert_eq!(store.collect(), Ok(2));
    store.read(copy, None, &mut content).unwrap();
    assert_eq!(content, [9; 8192]);
    let shared = store.copy_attached(copy, b"/home/shared").unwrap();
    store.destroy_explicit(copy, true).unwrap();
    assert_eq!(store.collect(), Ok(2)); // Only the obsolete v1 extent is reclaimed.
    drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    store.read(shared, None, &mut content).unwrap();
    assert_eq!(content, [9; 8192]);
    store.destroy_explicit(shared, true).unwrap();
    assert_eq!(store.collect(), Ok(2));
}

// ------------------------=
// FUNC: native_replica_remount_and_publication
// DESC: Exercises real native storage commits, process-state loss, resumed transfer and integrity-checked publication.
// ------------------=
#[test]
fn native_replica_remount_and_publication() {
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let backing = store.create(b"replica-fixture", ObjectType::Metadata, Space::System, &[]).unwrap();
    let payload = vec![0xa7; 3073]; let d = descriptor(&payload);
    {
        let mut native = NativeReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
        let mut transfer = Transfer::begin(&mut native, d).unwrap();
        transfer.receive(&mut native, 0, &payload[..1024]).unwrap();
        assert_eq!(native.read_verified(&mut [0; 4096]), Err(ReplicaError::Incomplete));
        // Persisting an invented offset or Available state directly must fail.
        let mut forged = transfer.inspect(); forged.copied = 2048;
        assert_eq!(native.checkpoint(&forged), Err(ReplicaError::Conflict));
        forged.copied = d.bytes; forged.state = ReplicaState::Available;
        assert_eq!(native.publish_verified(&forged), Err(ReplicaError::Conflict));
    }
    drop(store);
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    assert!(matches!(NativeReplica::open(&mut store, backing, d.resource, 6), Err(ReplicaError::Stale)));
    {
        let mut native = NativeReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
        let mut transfer = Transfer::resume(native.inspect().unwrap()).unwrap();
        assert_eq!(transfer.inspect().copied, 1024);
        transfer.receive(&mut native, 0, &payload[..1024]).unwrap();
        for offset in (1024..payload.len()).step_by(1024) {
            transfer.receive(&mut native, offset as u64, &payload[offset..(offset + 1024).min(payload.len())]).unwrap();
        }
        assert_eq!(transfer.verify_tick(&mut native), Ok(ReplicaState::Verifying));
    }
    drop(store);
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    {
        let mut native = NativeReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
        let mut transfer = Transfer::resume(native.inspect().unwrap()).unwrap();
        for _ in 0..3 { assert_eq!(transfer.verify_tick(&mut native), Ok(ReplicaState::Verifying)); }
        assert_eq!(transfer.verify_tick(&mut native), Ok(ReplicaState::Available));
    }
    drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    let mut native = NativeReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
    let mut read = vec![0; payload.len()];
    assert_eq!(native.read_verified(&mut read), Ok(payload.len())); assert_eq!(read, payload);
    assert_eq!(native.inspect().unwrap().descriptor.object, d.object);
}

// ------------------------=
// FUNC: native_replica_atomic_chunk_power_loss
// DESC: Cuts every sector-write boundary of one native checkpoint commit and requires coherent remount and recovery.
// ------------------=
#[test]
fn native_replica_atomic_chunk_power_loss() {
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [8; 16]).unwrap();
    let backing = store.create(b"replica-fixture", ObjectType::Metadata, Space::System, &[]).unwrap();
    let payload = [0x32; 1024]; let d = descriptor(&payload);
    {
        let mut native = NativeReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
        Transfer::begin(&mut native, d).unwrap();
    }
    let baseline = disk.0.borrow().sectors.clone();
    disk.0.borrow_mut().writes = 0;
    {
        let mut native = NativeReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
        let mut transfer = Transfer::resume(native.inspect().unwrap()).unwrap();
        transfer.receive(&mut native, 0, &payload).unwrap();
    }
    let total = disk.0.borrow().writes; assert!(total > 1);
    for cut in 0..=total {
        let disk = Disk(Rc::new(RefCell::new(DiskState { sectors: baseline.clone(), writes_left: Some(cut), writes: 0 })));
        let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
        {
            let mut native = NativeReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
            let mut transfer = Transfer::resume(native.inspect().unwrap()).unwrap();
            let _ = transfer.receive(&mut native, 0, &payload);
        }
        drop(store); disk.0.borrow_mut().writes_left = None;
        let mut store = ObjectStore::mount(disk, 0).unwrap();
        let mut native = NativeReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
        let checkpoint = native.inspect().unwrap();
        assert!(checkpoint.copied == 0 || checkpoint.copied == 1024);
        assert_ne!(checkpoint.state, ReplicaState::Available);
        let mut transfer = Transfer::resume(checkpoint).unwrap();
        transfer.receive(&mut native, 0, &payload).unwrap();
        assert_eq!(transfer.verify_tick(&mut native), Ok(ReplicaState::Available));
        let mut out = [0; 1024]; assert_eq!(native.read_verified(&mut out), Ok(1024)); assert_eq!(out, payload);
    }
}

// ------------------------=
// FUNC: native_replica_publication_power_loss
// DESC: Cuts every publication write boundary; after remount readers see either verified Available data or inaccessible Verifying data.
// ------------------=
#[test]
fn native_replica_publication_power_loss() {
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [9; 16]).unwrap();
    let backing = store.create(b"replica-fixture", ObjectType::Metadata, Space::System, &[]).unwrap();
    let payload = [0x91; 1024]; let d = descriptor(&payload);
    {
        let mut native = NativeReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
        let mut transfer = Transfer::begin(&mut native, d).unwrap();
        transfer.receive(&mut native, 0, &payload).unwrap();
        native.checkpoint(&Checkpoint { state: ReplicaState::Verifying, ..transfer.inspect() }).unwrap();
    }
    let baseline = disk.0.borrow().sectors.clone(); disk.0.borrow_mut().writes = 0;
    {
        let mut native = NativeReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
        let mut transfer = Transfer::resume(native.inspect().unwrap()).unwrap();
        assert_eq!(transfer.verify_tick(&mut native), Ok(ReplicaState::Available));
    }
    let total = disk.0.borrow().writes; assert!(total > 1);
    for cut in 0..=total {
        let disk = Disk(Rc::new(RefCell::new(DiskState { sectors: baseline.clone(), writes_left: Some(cut), writes: 0 })));
        let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
        {
            let mut native = NativeReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
            let mut transfer = Transfer::resume(native.inspect().unwrap()).unwrap();
            let _ = transfer.verify_tick(&mut native);
        }
        drop(store); disk.0.borrow_mut().writes_left = None;
        let mut store = ObjectStore::mount(disk, 0).unwrap();
        let mut native = NativeReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
        let checkpoint = native.inspect().unwrap();
        assert!(matches!(checkpoint.state, ReplicaState::Verifying | ReplicaState::Available));
        let mut out = [0; 1024];
        if checkpoint.state == ReplicaState::Verifying {
            assert_eq!(native.read_verified(&mut out), Err(ReplicaError::Incomplete));
        } else { assert_eq!(native.read_verified(&mut out), Ok(1024)); assert_eq!(out, payload); }
        let mut transfer = Transfer::resume(checkpoint).unwrap();
        assert_eq!(transfer.verify_tick(&mut native), Ok(ReplicaState::Available));
        assert_eq!(native.read_verified(&mut out), Ok(1024)); assert_eq!(out, payload);
    }
}

// ------------------------=
// FUNC: native_replica_rejects_malformed_and_corrupt_objects
// DESC: Rejects truncated and invalid native envelopes without panic and rejects content whose native CRC passes but SHA-256 differs.
// ------------------=
#[test]
fn native_replica_rejects_malformed_and_corrupt_objects() {
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [10; 16]).unwrap();
    let backing = store.create(b"replica-fixture", ObjectType::Metadata, Space::System, &[]).unwrap();
    let payload = [0xb4; 32]; let d = descriptor(&payload);
    {
        let mut native = NativeReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
        let mut transfer = Transfer::begin(&mut native, d).unwrap();
        transfer.receive(&mut native, 0, &payload).unwrap();
        transfer.verify_tick(&mut native).unwrap();
    }
    let mut encoded = [0; 1024]; let length = store.read(backing, None, &mut encoded).unwrap();
    for size in 1..length {
        store.replace_state(backing, &encoded[..size]).unwrap();
        assert!(matches!(NativeReplica::open(&mut store, backing, d.resource, d.generation), Err(ReplicaError::Invalid)));
    }
    for offset in [0, 8, 9, 120] {
        let mut invalid = encoded; invalid[offset] ^= 0xff;
        store.replace_state(backing, &invalid[..length]).unwrap();
        assert!(matches!(NativeReplica::open(&mut store, backing, d.resource, d.generation), Err(ReplicaError::Invalid)));
    }
    encoded[length - 1] ^= 1;
    store.replace_state(backing, &encoded[..length]).unwrap();
    drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    let mut native = NativeReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
    assert_eq!(native.read_verified(&mut [0; 32]), Err(ReplicaError::Integrity));
}
