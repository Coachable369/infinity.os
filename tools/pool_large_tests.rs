use super::*;
#[path = "pool_retirement_tests.rs"]
mod retirement;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
#[derive(Clone, Default)]
struct Disk(Rc<RefCell<State>>);
#[derive(Clone, Default)]
struct State {
    sectors: BTreeMap<u64, [u8; 512]>,
    remaining: Option<usize>,
    writes: usize,
    reads: usize,
}
impl BlockDevice for Disk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Supplies bounded sparse native media for behavioral persistence tests.
    // ------------------=
    fn block_count(&self) -> u64 {
        crate::storage::object::STORE_RELATIVE_LBA + 65536
    }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads exactly persisted bytes without interpreting diagnostics.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
        let mut s = self.0.borrow_mut();
        s.reads += 1;
        *out = *s.sectors.get(&lba).unwrap_or(&[0; 512]);
        true
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Injects deterministic persistence cuts at exact sector boundaries.
    // ------------------=
    fn write_sector(&mut self, lba: u64, bytes: &[u8; 512]) -> bool {
        let mut s = self.0.borrow_mut();
        if s.remaining == Some(0) {
            return false;
        }
        if let Some(n) = s.remaining.as_mut() {
            *n -= 1;
        }
        s.writes += 1;
        s.sectors.insert(lba, *bytes);
        true
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Models successful stable-storage flush after accepted sectors.
    // ------------------=
    fn flush(&mut self) -> bool {
        true
    }
}
const OWNER: NodeId = NodeId([1; 32]);
const RESOURCE: ResourceId = ResourceId([2; 16]);
// ------------------------=
// FUNC: fresh
// DESC: Creates native metadata and the production Pool catalog.
// ------------------=
fn fresh() -> (Disk, ObjectStore<Disk>) {
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    store.initialize_pool_catalog().unwrap();
    (disk, store)
}
// ------------------------=
// FUNC: upload
// DESC: Exercises persistent bounded append and one-KiB verification to produce a committed application manifest.
// ------------------=
fn upload(
    store: &mut ObjectStore<Disk>,
    bytes: &[u8],
    nonce: u64,
    target: ObjectId,
    expected: u64,
) -> Manifest {
    let (id, offset) = store
        .pool_upload_begin(
            OWNER,
            0,
            nonce,
            StorageClass::Critical,
            bytes.len() as u32,
            Sha256::digest(bytes).into(),
            target,
            expected,
        )
        .unwrap();
    assert_eq!(offset, 0);
    for (at, window) in bytes.chunks(1024).enumerate() {
        assert_eq!(
            store
                .pool_upload_append(id, OWNER, 0, (at * 1024) as u32, window)
                .unwrap(),
            (at * 1024 + window.len()) as u32
        );
    }
    let mut verifier = UploadVerifier::new();
    for _ in 0..=bytes.len().div_ceil(1024) {
        if let Some(m) = store
            .pool_upload_commit_step(id, OWNER, 0, &mut verifier, OWNER, RESOURCE, [3; 16], 1)
            .unwrap()
        {
            return m;
        }
    }
    panic!("bounded verification failed to publish")
}
// ------------------------=
// FUNC: verify
// DESC: Checks returned binary content over every transport-sized range.
// ------------------=
fn verify(store: &mut ObjectStore<Disk>, manifest: &Manifest, bytes: &[u8]) {
    for (at, expected) in bytes.chunks(64).enumerate() {
        let mut out = [0; 64];
        store
            .pool_read(manifest, (at * 64) as u64, &mut out[..expected.len()])
            .unwrap();
        assert_eq!(&out[..expected.len()], expected);
    }
}
// ------------------------=
// FUNC: large_pool_resume_copy_diverge_delete_reboot
// DESC: Proves multi-extent identity, durable upload resume, independent COW edits, retained-version reachability and final reclamation.
// ------------------=
#[test]
fn large_pool_resume_copy_diverge_delete_reboot() {
    let (disk, mut store) = fresh();
    let bytes: Vec<u8> = (0..40000).map(|n| (n % 251) as u8).collect();
    let (id, _) = store
        .pool_upload_begin(
            OWNER,
            0,
            1,
            StorageClass::Critical,
            bytes.len() as u32,
            Sha256::digest(&bytes).into(),
            ObjectId([0; 16]),
            0,
        )
        .unwrap();
    for (at, window) in bytes[..20480].chunks(1024).enumerate() {
        store
            .pool_upload_append(id, OWNER, 0, (at * 1024) as u32, window)
            .unwrap();
    }
    assert_eq!(store.pool_inspect(OWNER, 0, 0).unwrap().0, 0);
    drop(store);
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    assert_eq!(
        store
            .pool_upload_begin(
                OWNER,
                0,
                1,
                StorageClass::Critical,
                40000,
                Sha256::digest(&bytes).into(),
                ObjectId([0; 16]),
                0
            )
            .unwrap(),
        (id, 20480)
    );
    for (at, window) in bytes[20480..].chunks(1024).enumerate() {
        store
            .pool_upload_append(id, OWNER, 0, (20480 + at * 1024) as u32, window)
            .unwrap();
    }
    let mut verifier = UploadVerifier::new();
    let mut first = None;
    for _ in 0..41 {
        let next = store
            .pool_upload_commit_step(id, OWNER, 0, &mut verifier, OWNER, RESOURCE, [3; 16], 1)
            .unwrap();
        if next.is_some() {
            first = next;
            break;
        }
    }
    let first = first.unwrap();
    assert_eq!(first.chunks.iter().flatten().count(), 3);
    verify(&mut store, &first, &bytes);
    let copied = store
        .pool_copy(
            ObjectId(first.object),
            OWNER,
            0,
            first.generation,
            2,
            OWNER,
            RESOURCE,
            [3; 16],
            1,
        )
        .unwrap();
    assert_ne!(first.object, copied.object);
    assert_eq!(first.chunks, copied.chunks);
    let changed = vec![9; 50000];
    let next = upload(&mut store, &changed, 3, ObjectId(copied.object), 1);
    assert_eq!(next.object, copied.object);
    assert_eq!(next.version, 2);
    verify(&mut store, &first, &bytes);
    verify(&mut store, &next, &changed);
    store
        .pool_delete(ObjectId(first.object), OWNER, 0, 1)
        .unwrap();
    verify(&mut store, &next, &changed);
    drop(store);
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    let current = store
        .pool_manifest(ObjectId(next.object), OWNER, 0)
        .unwrap();
    verify(&mut store, &current, &changed);
    store
        .pool_delete(ObjectId(next.object), OWNER, 0, 2)
        .unwrap();
    assert_eq!(store.pool_inspect(OWNER, 0, 0).unwrap().0, 0);
    for chunk in first.chunks.iter().chain(next.chunks.iter()).flatten() {
        assert!(store
            .read_extent_range(ObjectId(chunk.content), 0, &mut [0; 1])
            .is_err());
    }
    let mut mounted = ObjectStore::mount(disk, 0).unwrap();
    assert_eq!(mounted.pool_inspect(OWNER, 0, 0).unwrap().0, 0);
}
// ------------------------=
// FUNC: large_pool_integrity_prefix_and_abort
// DESC: Rejects foreign upload access, gaps, changed duplicates and corrupt final content; abort durably releases staging capacity.
// ------------------=
#[test]
fn large_pool_integrity_prefix_and_abort() {
    let (disk, mut store) = fresh();
    let bytes = vec![8; 20000];
    let (id, _) = store
        .pool_upload_begin(
            OWNER,
            0,
            1,
            StorageClass::Temporary,
            20000,
            Sha256::digest(&bytes).into(),
            ObjectId([0; 16]),
            0,
        )
        .unwrap();
    assert!(store
        .pool_upload_append(id, NodeId([9; 32]), 0, 0, &[8; 64])
        .is_err());
    assert!(store
        .pool_upload_append(id, OWNER, 0, 64, &[8; 64])
        .is_err());
    store.pool_upload_append(id, OWNER, 0, 0, &[8; 64]).unwrap();
    assert_eq!(
        store.pool_upload_append(id, OWNER, 0, 0, &[8; 64]).unwrap(),
        64
    );
    assert!(store.pool_upload_append(id, OWNER, 0, 0, &[7; 64]).is_err());
    assert!(store
        .pool_upload_commit_step(
            id,
            OWNER,
            0,
            &mut UploadVerifier::new(),
            OWNER,
            RESOURCE,
            [3; 16],
            1
        )
        .is_err());
    store.pool_upload_abort(id, OWNER, 0).unwrap();
    drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    assert_eq!(store.staging_reserved_bytes(), 0);
    let mut altered = bytes.clone();
    altered[17000] = 4;
    let (id, _) = store
        .pool_upload_begin(
            OWNER,
            0,
            2,
            StorageClass::Temporary,
            20000,
            Sha256::digest(&bytes).into(),
            ObjectId([0; 16]),
            0,
        )
        .unwrap();
    for (at, b) in altered.chunks(1024).enumerate() {
        store
            .pool_upload_append(id, OWNER, 0, (at * 1024) as u32, b)
            .unwrap();
    }
    let mut v = UploadVerifier::new();
    for _ in 0..20 {
        assert!(store
            .pool_upload_commit_step(id, OWNER, 0, &mut v, OWNER, RESOURCE, [3; 16], 1)
            .unwrap()
            .is_none());
    }
    assert!(store
        .pool_upload_commit_step(id, OWNER, 0, &mut v, OWNER, RESOURCE, [3; 16], 1)
        .is_err());
    assert_eq!(store.pool_inspect(OWNER, 0, 0).unwrap().0, 0);
}
// ------------------------=
// FUNC: large_pool_publication_power_cuts_preserve_unpublished_state
// DESC: Cuts every final-publication sector write and proves remount never exposes uncommitted extents as an application object.
// ------------------=
#[test]
fn large_pool_publication_power_cuts_preserve_unpublished_state() {
    let (disk, mut store) = fresh();
    let bytes = vec![17; 18000];
    let (id, _) = store
        .pool_upload_begin(
            OWNER,
            0,
            1,
            StorageClass::Protected,
            18000,
            Sha256::digest(&bytes).into(),
            ObjectId([0; 16]),
            0,
        )
        .unwrap();
    for (at, b) in bytes.chunks(1024).enumerate() {
        store
            .pool_upload_append(id, OWNER, 0, (at * 1024) as u32, b)
            .unwrap();
    }
    let mut v = UploadVerifier::new();
    for _ in 0..18 {
        assert!(store
            .pool_upload_commit_step(id, OWNER, 0, &mut v, OWNER, RESOURCE, [3; 16], 1)
            .unwrap()
            .is_none());
    }
    let snapshot = disk.0.borrow().clone();
    let writes = snapshot.writes;
    let manifest = store
        .pool_upload_commit_step(id, OWNER, 0, &mut v.clone(), OWNER, RESOURCE, [3; 16], 1)
        .unwrap()
        .unwrap();
    let cost = disk.0.borrow().writes - writes;
    assert!(cost > 0);
    for cut in 0..cost {
        let trial = Disk(Rc::new(RefCell::new(snapshot.clone())));
        trial.0.borrow_mut().remaining = Some(cut);
        let mut interrupted = ObjectStore::mount(trial.clone(), 0).unwrap();
        assert!(interrupted
            .pool_upload_commit_step(id, OWNER, 0, &mut v.clone(), OWNER, RESOURCE, [3; 16], 1)
            .is_err());
        trial.0.borrow_mut().remaining = None;
        let mut recovered = ObjectStore::mount(trial, 0).unwrap();
        assert_eq!(recovered.pool_inspect(OWNER, 0, 0).unwrap().0, 0);
        assert!(recovered
            .pool_manifest(ObjectId(manifest.object), OWNER, 0)
            .is_err());
        assert!(recovered.staging_reserved_bytes() >= 18000);
    }
}
// ------------------------=
// FUNC: large_pool_delete_power_cuts_preserve_shared_content
// DESC: Cuts each deletion commit and checks both independent identities and their shared immutable bytes survive failed transactions.
// ------------------=
#[test]
fn large_pool_delete_power_cuts_preserve_shared_content() {
    let (disk, mut store) = fresh();
    let bytes = vec![31; 18000];
    let first = upload(&mut store, &bytes, 1, ObjectId([0; 16]), 0);
    let copy = store
        .pool_copy(
            ObjectId(first.object),
            OWNER,
            0,
            1,
            2,
            OWNER,
            RESOURCE,
            [3; 16],
            1,
        )
        .unwrap();
    let snapshot = disk.0.borrow().clone();
    let writes = snapshot.writes;
    store
        .pool_delete(ObjectId(first.object), OWNER, 0, 1)
        .unwrap();
    let cost = disk.0.borrow().writes - writes;
    for cut in 0..cost {
        let trial = Disk(Rc::new(RefCell::new(snapshot.clone())));
        trial.0.borrow_mut().remaining = Some(cut);
        let mut interrupted = ObjectStore::mount(trial.clone(), 0).unwrap();
        assert!(interrupted
            .pool_delete(ObjectId(first.object), OWNER, 0, 1)
            .is_err());
        trial.0.borrow_mut().remaining = None;
        let mut recovered = ObjectStore::mount(trial, 0).unwrap();
        assert_eq!(recovered.pool_inspect(OWNER, 0, 0).unwrap().0, 2);
        let mut b = [0; 64];
        recovered.pool_read(&first, 16384, &mut b).unwrap();
        assert_eq!(b, [31; 64]);
        recovered.pool_read(&copy, 16384, &mut b).unwrap();
        assert_eq!(b, [31; 64]);
    }
}
// ------------------------=
// FUNC: large_pool_zero_and_small_divergence_keep_old_extent_ownership
// DESC: Covers empty upload publication and large-to-small updates without premature reclamation of retained history.
// ------------------=
#[test]
fn large_pool_zero_and_small_divergence_keep_old_extent_ownership() {
    let (_, mut store) = fresh();
    let empty = upload(&mut store, &[], 1, ObjectId([0; 16]), 0);
    store.pool_read(&empty, 0, &mut []).unwrap();
    assert_eq!(empty.length, 0);
    let original = upload(&mut store, &vec![4; 17000], 2, ObjectId([0; 16]), 0);
    let copy = store
        .pool_copy(
            ObjectId(original.object),
            OWNER,
            0,
            1,
            3,
            OWNER,
            RESOURCE,
            [3; 16],
            1,
        )
        .unwrap();
    let small = store
        .pool_update(
            ObjectId(copy.object),
            OWNER,
            0,
            1,
            b"small",
            OWNER,
            RESOURCE,
            [3; 16],
            1,
        )
        .unwrap();
    verify(&mut store, &small, b"small");
    store
        .pool_delete(ObjectId(original.object), OWNER, 0, 1)
        .unwrap();
    for c in original.chunks.iter().flatten() {
        store
            .read_extent_range(ObjectId(c.content), 0, &mut [0; 1])
            .unwrap();
    }
    store
        .pool_delete(ObjectId(copy.object), OWNER, 0, 2)
        .unwrap();
    for c in original.chunks.iter().flatten() {
        assert!(store
            .read_extent_range(ObjectId(c.content), 0, &mut [0; 1])
            .is_err());
    }
}
// ------------------------=
// FUNC: large_pool_authenticated_wire_upload_and_delete
// DESC: Exercises production canonical IOP payloads and recipient ownership checks across every window, reboot, incremental completion and deletion.
// ------------------=
#[test]
fn large_pool_authenticated_wire_upload_and_delete() {
    use crate::native_fabric::service::ReplicaService;
    use crate::runtime::iop::{
        remote::AuthenticatedStorageRequest,
        storage_protocol::{Operation, StorageOperationV1},
    };
    let (disk, mut store) = fresh();
    let mut service = ReplicaService::mount(&mut store, RESOURCE, 1).unwrap();
    service.attach_device_identity(Some([3; 16]));
    let bytes: Vec<u8> = (0..20000).map(|i| (i % 233) as u8).collect();
    let hash: [u8; 32] = Sha256::digest(&bytes).into();
    let mut request = AuthenticatedStorageRequest {
        local: OWNER,
        peer: OWNER,
        session_reference: [6; 16],
        grant: 1,
        request_id: 1,
        correlation: 2,
        causation: 3,
        payload: StorageOperationV1 {
            operation: Operation::PoolUploadBegin,
            object: [0; 16],
            authority_generation: 1,
            manifest_generation: 0,
            object_version: 0,
            offset: 0,
            scope: 9,
            value: 0,
            length: 45,
            data: [0; 64],
        },
    };
    request.payload.data[..4].copy_from_slice(&20000u32.to_le_bytes());
    request.payload.data[4..36].copy_from_slice(&hash);
    request.payload.data[36] = 3;
    request.payload.data[37..45].copy_from_slice(&42u64.to_le_bytes());
    request.payload = StorageOperationV1::decode(&request.payload.encode().unwrap()).unwrap();
    let begin = service.execute(&mut store, request).unwrap();
    let upload_id: [u8; 16] = begin.data[..16].try_into().unwrap();
    request.payload.operation = Operation::PoolUploadAppend;
    request.payload.object = upload_id;
    for (at, b) in bytes.chunks(60).enumerate() {
        request.payload.data = [0; 64];
        request.payload.data[..4].copy_from_slice(&((at * 60) as u32).to_le_bytes());
        request.payload.data[4..4 + b.len()].copy_from_slice(b);
        request.payload.length = (4 + b.len()) as u16;
        request.payload = StorageOperationV1::decode(&request.payload.encode().unwrap()).unwrap();
        if at == 0 {
            let mut denied = request;
            denied.peer = NodeId([8; 32]);
            assert!(service.execute(&mut store, denied).is_err());
            denied = request;
            denied.payload.scope = 10;
            assert!(service.execute(&mut store, denied).is_err());
        }
        assert_eq!(
            service.execute(&mut store, request).unwrap().offset,
            (at * 60 + b.len()) as u64
        );
    }
    drop(store);
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    let mut service = ReplicaService::mount(&mut store, RESOURCE, 1).unwrap();
    service.attach_device_identity(Some([3; 16]));
    request.payload.operation = Operation::PoolUploadCommit;
    request.payload.length = 0;
    request.payload.data = [0; 64];
    let mut finished = None;
    for step in 0..21 {
        let reply = service.execute(&mut store, request).unwrap();
        if reply.value == 1 {
            assert_eq!(step, 20);
            finished = Some(reply);
            break;
        }
        assert_eq!(reply.value, 0);
    }
    let done = finished.unwrap();
    assert_eq!(done.length, 48);
    assert_eq!(&done.data[16..48], &hash);
    assert_eq!(done.offset, 20000);
    let application: [u8; 16] = done.data[..16].try_into().unwrap();
    assert_ne!(application, [0; 16]);
    assert_eq!(done.object, upload_id);
    assert_eq!(
        service.execute(&mut store, request).unwrap().object,
        done.object
    );
    let m = store
        .pool_manifest(ObjectId(application), OWNER, 9)
        .unwrap();
    verify(&mut store, &m, &bytes);
    request.payload.operation = Operation::ObjectDelete;
    request.payload.object = application;
    request.payload.manifest_generation = done.manifest_generation;
    let mut denied = request;
    denied.peer = NodeId([8; 32]);
    assert!(service.execute(&mut store, denied).is_err());
    service.execute(&mut store, request).unwrap();
    drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    assert_eq!(store.pool_inspect(OWNER, 9, 0).unwrap().0, 0);
}
// ------------------------=
// FUNC: large_pool_source_cache_bounds_io_and_revalidates_owner
// DESC: Measures actual block reads to prove repeated transfer windows reuse verified immutable extents while rejecting foreign cached access.
// ------------------=
#[test]
fn large_pool_source_cache_bounds_io_and_revalidates_owner() {
    use crate::native_fabric::service::ReplicaService;
    use crate::runtime::storage_coordinator::{NativeReply, NativeRequest};
    let (disk, mut store) = fresh();
    let bytes = vec![5; 20000];
    let m = upload(&mut store, &bytes, 1, ObjectId([0; 16]), 0);
    let mut service = ReplicaService::mount(&mut store, RESOURCE, 1).unwrap();
    service.attach_device_identity(Some([3; 16]));
    let start = disk.0.borrow().reads;
    let first = service
        .coordinator_operation(
            &mut store,
            NativeRequest::Read {
                object: m.object,
                owner: OWNER,
                scope: 0,
                version: 1,
                offset: 0,
                length: 64,
            },
        )
        .unwrap();
    assert!(matches!(
        first,
        NativeReply::Bytes {
            data,
            length: 64
        } if data == [5;64]
    ));
    let miss = disk.0.borrow().reads - start;
    let start = disk.0.borrow().reads;
    let hit = service
        .coordinator_operation(
            &mut store,
            NativeRequest::Read {
                object: m.object,
                owner: OWNER,
                scope: 0,
                version: 1,
                offset: 64,
                length: 64,
            },
        )
        .unwrap();
    assert!(matches!(
        hit,
        NativeReply::Bytes {
            data,
            length: 64
        } if data == [5;64]
    ));
    let hit_reads = disk.0.borrow().reads - start;
    assert!(miss >= hit_reads + 32);
    assert!(service
        .coordinator_operation(
            &mut store,
            NativeRequest::Read {
                object: m.object,
                owner: NodeId([9; 32]),
                scope: 0,
                version: 1,
                offset: 64,
                length: 64
            }
        )
        .is_err());
    let next = store
        .pool_update(
            ObjectId(m.object),
            OWNER,
            0,
            1,
            &[7; 128],
            OWNER,
            RESOURCE,
            [3; 16],
            1,
        )
        .unwrap();
    let updated = service
        .coordinator_operation(
            &mut store,
            NativeRequest::Read {
                object: m.object,
                owner: OWNER,
                scope: 0,
                version: next.version,
                offset: 0,
                length: 64,
            },
        )
        .unwrap();
    assert!(matches!(
        updated,
        NativeReply::Bytes {
            data,
            length: 64
        } if data == [7;64]
    ));
}
// ------------------------=
// FUNC: large_pool_reservation_and_append_power_cuts_resume_exact_prefix
// DESC: Cuts reservation and append writes individually; remount preserves capacity ownership and the last acknowledged upload prefix.
// ------------------=
#[test]
fn large_pool_reservation_and_append_power_cuts_resume_exact_prefix() {
    let (disk, mut store) = fresh();
    let content = vec![12; 18000];
    let digest = Sha256::digest(&content).into();
    let first = store
        .pool_upload_begin(
            OWNER,
            0,
            99,
            StorageClass::Temporary,
            0,
            Sha256::digest([]).into(),
            ObjectId([0; 16]),
            0,
        )
        .unwrap()
        .0;
    store.pool_upload_abort(first, OWNER, 0).unwrap();
    let snapshot = disk.0.borrow().clone();
    let before = snapshot.writes;
    let (id, _) = store
        .pool_upload_begin(
            OWNER,
            0,
            1,
            StorageClass::Protected,
            18000,
            digest,
            ObjectId([0; 16]),
            0,
        )
        .unwrap();
    let cost = disk.0.borrow().writes - before;
    for cut in 0..cost {
        let trial = Disk(Rc::new(RefCell::new(snapshot.clone())));
        trial.0.borrow_mut().remaining = Some(cut);
        let mut interrupted = ObjectStore::mount(trial.clone(), 0).unwrap();
        assert!(interrupted
            .pool_upload_begin(
                OWNER,
                0,
                1,
                StorageClass::Protected,
                18000,
                digest,
                ObjectId([0; 16]),
                0
            )
            .is_err());
        trial.0.borrow_mut().remaining = None;
        let mut recovered = ObjectStore::mount(trial, 0).unwrap();
        assert_eq!(recovered.staging_reserved_bytes(), 0);
        assert_eq!(recovered.pool_inspect(OWNER, 0, 0).unwrap().0, 0);
    }
    let snapshot = disk.0.borrow().clone();
    let before = snapshot.writes;
    store
        .pool_upload_append(id, OWNER, 0, 0, &content[..60])
        .unwrap();
    let cost = disk.0.borrow().writes - before;
    for cut in 0..cost {
        let trial = Disk(Rc::new(RefCell::new(snapshot.clone())));
        trial.0.borrow_mut().remaining = Some(cut);
        let mut interrupted = ObjectStore::mount(trial.clone(), 0).unwrap();
        assert!(interrupted
            .pool_upload_append(id, OWNER, 0, 0, &content[..60])
            .is_err());
        trial.0.borrow_mut().remaining = None;
        let mut recovered = ObjectStore::mount(trial, 0).unwrap();
        assert_eq!(
            recovered
                .pool_upload_begin(
                    OWNER,
                    0,
                    1,
                    StorageClass::Protected,
                    18000,
                    digest,
                    ObjectId([0; 16]),
                    0
                )
                .unwrap()
                .1,
            0
        );
        assert_eq!(
            recovered
                .pool_upload_append(id, OWNER, 0, 0, &content[..60])
                .unwrap(),
            60
        );
    }
}
