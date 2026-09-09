use crate::{native_fabric::NativeReplica, runtime::fabric::{replica::*, resources::ResourceId},
    storage::{BlockDevice, object::{ObjectStore, ObjectType, Space, STORE_RELATIVE_LBA}}};
use sha2::{Digest, Sha256};
use std::{cell::RefCell, rc::Rc, collections::BTreeMap};

// ------------------------=
// FUNC: pool_mutation_execution_gate_preserves_plan_and_domain_boundaries
// DESC: Exercises production dispatch admission using parsed operations, including explicit no-execute plans and foreign object commands.
// ------------------=
#[test]
fn pool_mutation_execution_gate_preserves_plan_and_domain_boundaries() {
    use crate::runtime::console_language::{parse, ParseOutcome};
    for command in [b"pool create nonce=1 policy=critical content=Example".as_slice(),
        b"pool write obj:01010101010101010101010101010101 generation=1 version=1 content=Changed",
        b"pool copy obj:01010101010101010101010101010101 generation=1 version=1 nonce=2",
        b"pool policy obj:01010101010101010101010101010101 generation=1 version=1 policy=protected",
        b"pool advertise peer=node:0101010101010101010101010101010101010101010101010101010101010101 grant=1"] {
        let ParseOutcome::Graph(mut graph) = parse(command).unwrap() else { panic!("expected typed graph") };
        assert!(graph.executable_pool_mutation());
        graph.plan_only = true;
        assert!(!graph.executable_pool_mutation());
        graph.plan_only = false;
        graph.node_count = 2;
        assert!(!graph.executable_pool_mutation());
    }
    for command in [b"plan pool create nonce=1 policy=critical content=Example".as_slice(),
        b"pool list", b"object inspect obj:01010101010101010101010101010101"] {
        match parse(command).unwrap() {
            ParseOutcome::Graph(graph) => assert!(!graph.executable_pool_mutation()),
            ParseOutcome::OperationDiscovery(_) => {},
            _ => panic!("unexpected parser state"),
        }
    }
}

#[derive(Clone, Default)]
struct Disk(Rc<RefCell<DiskState>>);
#[derive(Default)]
struct DiskState { sectors: BTreeMap<u64, [u8; 512]>, writes_left: Option<usize>, writes: usize }

// ------------------------=
// FUNC: replay_captured_installed_pool_capacity_failure
// DESC: Mounts an explicitly supplied stopped installed disk read-only, redirects every write to RAM, and replays the exact rejected creation against its real persisted objects.
// ------------------=
#[test]
#[ignore = "requires an explicitly supplied stopped installed disk"]
fn replay_captured_installed_pool_capacity_failure() {
    use std::io::{Read, Seek, SeekFrom};
    use crate::runtime::{fabric::placement::StorageClass, node::types::NodeId};
    struct Overlay { file: std::fs::File, writes: BTreeMap<u64, [u8; 512]> }
    impl BlockDevice for Overlay {
        // ------------------------=
        // FUNC: block_count
        // DESC: Reports the captured physical disk size without altering the artifact.
        // ------------------=
        fn block_count(&self) -> u64 { self.file.metadata().unwrap().len() / 512 }
        // ------------------------=
        // FUNC: read_sector
        // DESC: Reads the overlay first and otherwise the exact original installed sector.
        // ------------------=
        fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
            if let Some(bytes) = self.writes.get(&lba) { *out = *bytes; return true; }
            self.file.seek(SeekFrom::Start(lba * 512)).is_ok() && self.file.read_exact(out).is_ok()
        }
        // ------------------------=
        // FUNC: write_sector
        // DESC: Restricts all replay mutations to process memory; the source file is opened read-only.
        // ------------------=
        fn write_sector(&mut self, lba: u64, bytes: &[u8; 512]) -> bool { self.writes.insert(lba, *bytes); true }
        // ------------------------=
        // FUNC: flush
        // DESC: Models stable overlay writes without flushing or changing the captured file.
        // ------------------=
        fn flush(&mut self) -> bool { true }
    }
    let mut overlay = Overlay { file: std::fs::File::open(std::env::var("MS10_REPLAY_DISK").unwrap()).unwrap(), writes: BTreeMap::new() };
    let (container, _, resource) = crate::storage::object::find_container(&mut overlay).unwrap();
    let mut root = [0; 512]; assert!(overlay.read_sector(1, &mut root));
    let device = root[56..72].try_into().unwrap();
    let mut store = ObjectStore::mount(overlay, container).unwrap();
    // Decode the existing manifest's authority from the persisted catalog.
    let catalog = store.resolve(b"/system/storage/pool-manifests").unwrap();
    let mut bytes = [0; 1056]; assert_eq!(store.read(catalog, None, &mut bytes), Ok(1056));
    let owner = NodeId(bytes[64..96].try_into().unwrap());
    assert_eq!(store.pool_inspect(owner, 0, 0).unwrap().0, 1);
    let empty = store.pool_create(owner, 0, 2, StorageClass::Temporary, &[], owner, ResourceId(resource), device, 1).unwrap();
    assert_eq!(empty.length, 0);
    assert_eq!(store.pool_inspect(owner, 0, 0).unwrap().0, 2);
}

// ------------------------=
// FUNC: installed_density_empty_pool_creation_survives_version_table_expansion
// DESC: Reproduces thirty-one occupied version slots, then verifies empty-object admission past the old limit and every sector-cut rollback boundary.
// ------------------=
#[test]
fn installed_density_empty_pool_creation_survives_version_table_expansion() {
    use crate::runtime::{fabric::placement::StorageClass, node::types::NodeId};
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    store.initialize_pool_catalog().unwrap();
    for n in 0..14 {
        store.create(b"Core state", ObjectType::Metadata, Space::System, &[n]).unwrap();
    }
    let owner = NodeId([3; 32]); let resource = ResourceId([4; 16]);
    let first = store.pool_create(owner, 0, 1, StorageClass::Critical, b"retained", owner, resource, [5; 16], 1).unwrap();
    drop(store);
    // Format-4 banks have the same original-sector layout and only 32 slots.
    // Recreate those exact binary headers and checksums, not a prose fixture.
    {
        let mut persisted = disk.0.borrow_mut();
        for (_, bytes) in persisted.sectors.range_mut(STORE_RELATIVE_LBA..STORE_RELATIVE_LBA + 72) {
            bytes[8..12].copy_from_slice(&4u32.to_le_bytes());
            bytes[508..512].fill(0);
            let crc = crate::storage::object::crc32(bytes);
            bytes[508..512].copy_from_slice(&crc.to_le_bytes());
        }
        let roots = [0, 1].map(|i| {
            let root = persisted.sectors.get(&(STORE_RELATIVE_LBA + i)).unwrap();
            (u64::from_le_bytes(root[16..24].try_into().unwrap()), u64::from_le_bytes(root[24..32].try_into().unwrap()))
        });
        let bank = roots.into_iter().max().unwrap().1;
        let occupied: usize = (0..4).map(|s| {
            let bytes = persisted.sectors.get(&(STORE_RELATIVE_LBA + bank + 11 + s)).unwrap();
            (0..8).filter(|n| bytes[16 + n * 60] != 0).count()
        }).sum();
        assert_eq!(occupied, 31);
    }
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    let baseline = disk.0.borrow().sectors.clone();
    let prior_writes = disk.0.borrow().writes;
    let empty = store.pool_create(owner, 0, 2, StorageClass::Temporary, &[], owner, resource, [5; 16], 1).unwrap();
    let writes = disk.0.borrow().writes - prior_writes;
    assert_ne!(empty.object, first.object);
    drop(store);
    let mut mounted = ObjectStore::mount(disk.clone(), 0).unwrap();
    assert_eq!(mounted.pool_manifest(crate::storage::object::ObjectId(empty.object), owner, 0), Ok(empty));
    assert_eq!(mounted.pool_inspect(owner, 0, 0).unwrap().0, 2);
    for cut in 0..writes {
        let trial = Disk(Rc::new(RefCell::new(DiskState { sectors: baseline.clone(), writes_left: Some(cut), writes: 0 })));
        let mut interrupted = ObjectStore::mount(trial.clone(), 0).unwrap();
        assert!(interrupted.pool_create(owner, 0, 2, StorageClass::Temporary, &[], owner, resource, [5; 16], 1).is_err());
        trial.0.borrow_mut().writes_left = None;
        let mut recovered = ObjectStore::mount(trial, 0).unwrap();
        assert_eq!(recovered.pool_inspect(owner, 0, 0).unwrap().0, 1);
        assert_eq!(recovered.pool_manifest(crate::storage::object::ObjectId(first.object), owner, 0), Ok(first));
    }
}
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
// FUNC: replica_request
// DESC: Builds an authenticated-boundary host fixture; separate router tests exercise actual session, grant, revocation and correlation validation.
// ------------------=
fn replica_request(payload: &[u8]) -> crate::runtime::iop::remote::AuthenticatedStorageRequest {
    use crate::runtime::{node::types::NodeId, iop::{remote::AuthenticatedStorageRequest,
        storage_protocol::{Operation, StorageOperationV1}}};
    let d = descriptor(payload);
    let mut data = [0; 64]; data[..16].copy_from_slice(&d.resource.0);
    data[16..24].copy_from_slice(&d.generation.to_le_bytes()); data[24..56].copy_from_slice(&d.hash);
    AuthenticatedStorageRequest { local: NodeId([18; 32]), peer: NodeId([19; 32]), session_reference: [20; 16], grant: 1,
        request_id: 7, correlation: 8, causation: 9,
        payload: StorageOperationV1 { operation: Operation::TransferBegin, object: d.object,
            authority_generation: 1, manifest_generation: 2, object_version: d.version,
            offset: d.bytes, scope: 42, value: d.job, length: 56, data } }
}

// ------------------------=
// FUNC: pool_request
// DESC: Builds a bounded native create fixture whose nonce is persisted independently from application identity and namespace.
// ------------------=
fn pool_request(content: &[u8]) -> crate::runtime::iop::remote::AuthenticatedStorageRequest {
    use crate::runtime::iop::storage_protocol::Operation;
    let mut request = replica_request(&[]);
    request.payload.operation = Operation::ObjectCreate;
    request.payload.object = [0; 16]; request.payload.object_version = 0;
    request.payload.manifest_generation = 0; request.payload.offset = 77;
    request.payload.value = 3; request.payload.data = [0; 64]; request.payload.length = content.len() as u16;
    request.payload.data[..content.len()].copy_from_slice(content);
    request
}

// ------------------------=
// FUNC: native_pool_creation_policy_and_manifest_are_authoritative_after_reboot
// DESC: Exercises native service creation, exact retry identity, paginated canonical manifests, policy fencing, namespace independence and durable empty/nonempty content.
// ------------------=
#[test]
fn native_pool_creation_policy_and_manifest_are_authoritative_after_reboot() {
    use crate::{native_fabric::service::ReplicaService,
        runtime::{fabric::manifest::{Manifest, MANIFEST_BYTES}, iop::{remote::RemoteError, storage_protocol::Operation}},
        storage::object::ObjectId};
    for content in [&[][..], &[29; 63][..]] {
        let disk = Disk::default();
        let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
        let mut service = ReplicaService::mount(&mut store, ResourceId([4; 16]), 5).unwrap();
        service.attach_device_identity(Some([6; 16]));
        let request = pool_request(content);
        let created = service.execute(&mut store, request).unwrap();
        let id = ObjectId(created.data[..16].try_into().unwrap());
        assert_ne!(id.0, [0; 16]); assert_eq!(created.data[48..50], [3, 2]);
        let generation = store.generation(); let usage = store.usage_blocks();
        assert_eq!(service.execute(&mut store, request), Ok(created));
        assert_eq!(store.generation(), generation); assert_eq!(store.usage_blocks(), usage);
        let mut changed_retry = request; changed_retry.payload.value = 1;
        assert_eq!(service.execute(&mut store, changed_retry), Err(RemoteError::Conflict));
        store.attach(b"/personal/first", id).unwrap();
        let mut inspect = request; inspect.payload.operation = Operation::ObjectInspect;
        inspect.payload.object = id.0; inspect.payload.object_version = 1; inspect.payload.manifest_generation = 1;
        inspect.payload.data = [0; 64]; inspect.payload.length = 0; inspect.payload.value = 64;
        let mut bytes = [0; MANIFEST_BYTES];
        for offset in (0..MANIFEST_BYTES).step_by(64) {
            inspect.payload.offset = offset as u64;
            let reply = service.execute(&mut store, inspect).unwrap();
            assert_eq!(reply.manifest_generation, 1); assert_eq!(reply.value, MANIFEST_BYTES as u64);
            bytes[offset..offset+reply.length as usize].copy_from_slice(&reply.data[..reply.length as usize]);
        }
        let manifest = Manifest::decode(&bytes).unwrap();
        assert_eq!(manifest.object, id.0); assert_eq!(manifest.length, content.len() as u64);
        assert_eq!(manifest.hash, <[u8; 32]>::from(Sha256::digest(content)));
        assert_eq!(manifest.placements[0].unwrap().node, request.local);
        assert_eq!(manifest.authority, request.peer);
        if !content.is_empty() { assert_ne!(manifest.chunks[0].unwrap().content, id.0); }
        let mut policy = inspect; policy.payload.operation = Operation::ObjectSetPolicy;
        policy.payload.offset = 0; policy.payload.value = 2;
        let updated = service.execute(&mut store, policy).unwrap();
        assert_eq!(updated.manifest_generation, 2); assert_eq!(updated.data[48..50], [2, 2]);
        let committed = store.generation();
        assert_eq!(service.execute(&mut store, policy), Err(RemoteError::Conflict));
        policy.payload.manifest_generation = 2; policy.payload.value = 1;
        let mut foreign = policy; foreign.peer.0[31] ^= 1;
        assert_eq!(service.execute(&mut store, foreign), Err(RemoteError::AccessDenied));
        assert_eq!(store.generation(), committed);
        assert_eq!(service.execute(&mut store, policy).unwrap().data[48..50], [1, 1]);
        drop(service); drop(store);
        let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
        let restored = store.pool_manifest(id, request.peer, request.payload.scope).unwrap();
        assert_eq!(restored.generation, 3); assert_eq!(restored.object, id.0);
        assert_eq!(restored.placements[0].unwrap().admission_generation, 1);
        assert_eq!(store.resolve(b"/personal/first"), Ok(id));
        let mut out = [0; 64]; let len = store.read(id, None, &mut out).unwrap();
        assert_eq!(&out[..len], content);
        let mut service = ReplicaService::mount(&mut store, ResourceId([4; 16]), 5).unwrap();
        service.attach_device_identity(Some([6; 16]));
        let mut read = policy; read.payload.operation = Operation::ObjectRead;
        read.payload.manifest_generation = 3; read.payload.value = content.len() as u64;
        assert_eq!(&service.execute(&mut store, read).unwrap().data[..content.len()], content);
        let mut update = read; update.payload.operation = Operation::ObjectUpdate;
        update.payload.value = 0; update.payload.length = 64; update.payload.data = [73; 64];
        let result = service.execute(&mut store, update).unwrap();
        assert_eq!(result.object_version, 2); assert_eq!(result.manifest_generation, 4);
        assert_eq!(store.resolve(b"/personal/first"), Ok(id));
        assert_eq!(service.execute(&mut store, read), Err(RemoteError::Conflict));
        read.payload.manifest_generation = 4; read.payload.object_version = 2; read.payload.value = 64;
        assert_eq!(service.execute(&mut store, read).unwrap().data, [73; 64]);
        let len = store.read(id, Some(1), &mut out).unwrap(); assert_eq!(&out[..len], content);
        let mut listing = request; listing.payload.operation = Operation::PoolInspect;
        listing.payload.object = [0; 16]; listing.payload.offset = 0; listing.payload.value = 0;
        listing.payload.length = 0; listing.payload.data = [0; 64];
        let listed = service.execute(&mut store, listing).unwrap();
        assert_eq!(listed.value, 1); assert_eq!(listed.data[..16], id.0);
        listing.peer.0[31] ^= 1;
        let hidden = service.execute(&mut store, listing).unwrap();
        assert_eq!(hidden.value, 0); assert_eq!(hidden.length, 0); assert_eq!(hidden.data, [0; 64]);
    }
}

// ------------------------=
// FUNC: native_pool_creation_is_atomic_at_every_sector_cut
// DESC: Interrupts each write of content plus manifest plus ownership admission and requires recovery to contain either the complete object or no admitted object.
// ------------------=
#[test]
fn native_pool_creation_is_atomic_at_every_sector_cut() {
    use crate::native_fabric::service::ReplicaService;
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let mut service = ReplicaService::mount(&mut store, ResourceId([4; 16]), 5).unwrap();
    service.attach_device_identity(Some([6; 16]));
    let request = pool_request(&[31; 63]);
    let baseline = disk.0.borrow().sectors.clone(); let old_usage = store.usage_blocks();
    disk.0.borrow_mut().writes = 0;
    let created = service.execute(&mut store, request).unwrap();
    let writes = disk.0.borrow().writes; let new_usage = store.usage_blocks();
    for cut in 0..=writes {
        let disk = Disk(Rc::new(RefCell::new(DiskState { sectors: baseline.clone(), writes_left: None, writes: 0 })));
        let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
        let mut service = ReplicaService::new(ResourceId([4; 16]), 5);
        service.attach_device_identity(Some([6; 16]));
        disk.0.borrow_mut().writes_left = Some(cut);
        let _ = service.execute(&mut store, request);
        drop(service); drop(store); disk.0.borrow_mut().writes_left = None;
        let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
        assert!(store.usage_blocks() == old_usage || store.usage_blocks() == new_usage);
        let mut service = ReplicaService::new(ResourceId([4; 16]), 5);
        service.attach_device_identity(Some([6; 16]));
        assert_eq!(service.execute(&mut store, request), Ok(created));
        assert_eq!(store.usage_blocks(), new_usage);
    }
}

// ------------------------=
// FUNC: native_recipient_keeps_versions_immutable_across_restart
// DESC: Commits two physical versions under one stable application identity, rejects stale and conflicting successor admissions, and reads each verified version after cold mount.
// ------------------=
#[test]
fn native_recipient_keeps_versions_immutable_across_restart() {
    use crate::{native_fabric::service::ReplicaService,
        runtime::iop::{remote::RemoteError, storage_protocol::Operation}};
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let d = descriptor(&[31; 64]);
    let mut service = ReplicaService::mount(&mut store, d.resource, d.generation).unwrap();
    let mut requests = [replica_request(&[31; 64]), replica_request(&[47; 64])];
    requests[1].payload.object_version += 1;
    requests[1].payload.manifest_generation += 1;
    requests[1].payload.value += 1;
    for (index, request) in requests.iter().copied().enumerate() {
        assert_eq!(service.execute(&mut store, request).unwrap().data[0], 1);
        let mut chunk = request; chunk.payload.operation = Operation::TransferChunk;
        chunk.payload.offset = 0; chunk.payload.length = 64;
        chunk.payload.data = [if index == 0 { 31 } else { 47 }; 64];
        assert_eq!(service.execute(&mut store, chunk).unwrap().offset, 64);
        let mut commit = request; commit.payload.operation = Operation::TransferCommit;
        commit.payload.length = 0; commit.payload.data = [0; 64];
        assert_eq!(service.execute(&mut store, commit).unwrap().data[0], 4);
    }
    let committed = store.generation(); let usage = store.usage_blocks();
    for attack in 0..4 {
        let mut stale = requests[1];
        match attack {
            0 => stale.payload.object_version -= 2,
            1 => { stale.payload.object_version += 1; },
            2 => { stale.payload.object_version += 1; stale.payload.manifest_generation += 1; stale.peer.0[0] ^= 1; },
            _ => stale.payload.data[24] ^= 1,
        }
        assert!(matches!(service.execute(&mut store, stale), Err(RemoteError::Conflict | RemoteError::AccessDenied)));
        assert_eq!(store.generation(), committed); assert_eq!(store.usage_blocks(), usage);
    }
    drop(service); drop(store);
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    let mut service = ReplicaService::mount(&mut store, d.resource, d.generation).unwrap();
    for (index, mut read) in requests.into_iter().enumerate() {
        let expected = [if index == 0 { 31 } else { 47 }; 64];
        read.payload.operation = Operation::ObjectRead; read.payload.offset = 0;
        read.payload.value = 64; read.payload.length = 32; read.payload.data = [0; 64];
        read.payload.data[..32].copy_from_slice(&Sha256::digest(expected));
        assert_eq!(service.execute(&mut store, read).unwrap().data, expected);
    }
}

// ------------------------=
// FUNC: native_pool_update_keeps_manifest_and_content_on_one_transaction_root
// DESC: Cuts every sector write of an actual content successor and requires the cold-mounted reader to return exactly the old or new verified version, never mixed manifest/content state.
// ------------------=
#[test]
fn native_pool_update_keeps_manifest_and_content_on_one_transaction_root() {
    use crate::{native_fabric::service::ReplicaService,
        runtime::iop::storage_protocol::Operation, storage::object::ObjectId};
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let mut service = ReplicaService::mount(&mut store, ResourceId([4; 16]), 5).unwrap();
    service.attach_device_identity(Some([6; 16]));
    let create = pool_request(&[31; 63]);
    let created = service.execute(&mut store, create).unwrap();
    let object = ObjectId(created.data[..16].try_into().unwrap());
    let mut update = create; update.payload.operation = Operation::ObjectUpdate;
    update.payload.object = object.0; update.payload.object_version = 1; update.payload.manifest_generation = 1;
    update.payload.offset = 0; update.payload.value = 0; update.payload.length = 64; update.payload.data = [49; 64];
    let baseline = disk.0.borrow().sectors.clone();
    disk.0.borrow_mut().writes = 0;
    service.execute(&mut store, update).unwrap();
    let writes = disk.0.borrow().writes;
    for cut in 0..=writes {
        let disk = Disk(Rc::new(RefCell::new(DiskState { sectors: baseline.clone(), writes_left: None, writes: 0 })));
        let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
        let mut service = ReplicaService::new(ResourceId([4; 16]), 5);
        service.attach_device_identity(Some([6; 16]));
        disk.0.borrow_mut().writes_left = Some(cut);
        let _ = service.execute(&mut store, update);
        drop(service); drop(store); disk.0.borrow_mut().writes_left = None;
        let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
        let manifest = store.pool_manifest(object, create.peer, create.payload.scope).unwrap();
        assert!(manifest.version == 1 || manifest.version == 2);
        assert_eq!(manifest.generation, manifest.version);
        let mut bytes = [0; 64];
        let length = manifest.length as usize;
        store.pool_read(&manifest, 0, &mut bytes[..length]).unwrap();
        assert_eq!(&bytes[..length], if manifest.version == 1 { &[31; 63][..] } else { &[49; 64][..] });
    }
}

// ------------------------=
// FUNC: native_recipient_fences_every_chunk_and_recovers_ownership
// DESC: Exercises real native transactions across service loss, owner/version/scope attacks, duplicate retries, empty content and bounded verification.
// ------------------=
#[test]
fn native_recipient_fences_every_chunk_and_recovers_ownership() {
    use crate::{native_fabric::service::ReplicaService,
        runtime::iop::{remote::RemoteError, storage_protocol::Operation}};
    for length in [0, 32769] {
        let disk = Disk::default();
        let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
        let payload: Vec<u8> = (0..length).map(|n| (n % 251) as u8).collect();
        let request = replica_request(&payload); let d = descriptor(&payload);
        let mut service = ReplicaService::mount(&mut store, d.resource, d.generation).unwrap();
        let initial_usage = store.usage_blocks();
        let initial_generation = store.generation();
        let mut wrong = request; wrong.payload.data[0] ^= 1;
        assert_eq!(service.execute(&mut store, wrong), Err(RemoteError::Conflict));
        assert_eq!(store.usage_blocks(), initial_usage); assert_eq!(store.generation(), initial_generation);
        assert_eq!(service.execute(&mut store, request).unwrap().data[0], 1);
        let admitted = store.generation(); let usage = store.usage_blocks();
        assert_eq!(service.execute(&mut store, request).unwrap().offset, 0);
        assert_eq!(store.generation(), admitted); assert_eq!(store.usage_blocks(), usage);
        for attack in 0..5 {
            let mut denied = request; denied.payload.operation = Operation::TransferChunk;
            denied.payload.length = 1; denied.payload.offset = 0; denied.payload.data = [0; 64];
            match attack { 0 => denied.peer.0[31] ^= 1, 1 => denied.payload.authority_generation += 1,
                2 => denied.payload.scope += 1, 3 => denied.payload.object_version += 1,
                _ => denied.payload.manifest_generation += 1 }
            assert!(matches!(service.execute(&mut store, denied), Err(RemoteError::AccessDenied | RemoteError::Conflict)));
            assert_eq!(store.generation(), admitted);
        }
        for at in (0..payload.len()).step_by(64) {
            let mut chunk = request; chunk.payload.operation = Operation::TransferChunk;
            chunk.payload.offset = at as u64; chunk.payload.data = [0; 64];
            let end = (at+64).min(payload.len()); chunk.payload.length = (end-at) as u16;
            chunk.payload.data[..end-at].copy_from_slice(&payload[at..end]);
            assert_eq!(service.execute(&mut store, chunk).unwrap().offset, end as u64);
            let committed = store.generation();
            assert_eq!(service.execute(&mut store, chunk).unwrap().offset, end as u64);
            assert_eq!(store.generation(), committed);
        }
        let mut commit = request; commit.payload.operation = Operation::TransferCommit;
        commit.payload.data = [0; 64]; commit.payload.length = 0; commit.payload.offset = 0;
        let first = service.execute(&mut store, commit).unwrap();
        assert_eq!(first.data[0], if length == 0 { 4 } else { 3 });
        drop(service); drop(store);
        let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
        let mut service = ReplicaService::mount(&mut store, d.resource, d.generation).unwrap();
        let mut foreign = commit; foreign.peer.0[0] ^= 1;
        assert_eq!(service.execute(&mut store, foreign), Err(RemoteError::AccessDenied));
        let ticks = ((length + 1023) / 1024).max(1);
        for step in 1..=ticks {
            let response = service.execute(&mut store, commit).unwrap();
            assert_eq!(response.offset, length as u64);
            assert_eq!(response.data[0], if step == ticks { 4 } else { 3 });
            assert_eq!(u64::from_le_bytes(response.data[41..49].try_into().unwrap()), (step * 1024).min(length) as u64);
        }
        let published = store.generation();
        assert_eq!(service.execute(&mut store, commit).unwrap().data[0], 4);
        assert_eq!(store.generation(), published);
        for offset in (0..length).step_by(64) {
            let mut read = request; read.payload.operation = Operation::ObjectRead;
            read.payload.offset = offset as u64; read.payload.value = (length-offset).min(64) as u64;
            let base = offset / 1024 * 1024; let end = (base+1024).min(length);
            read.payload.data = [0; 64]; read.payload.length = 32;
            read.payload.data[..32].copy_from_slice(&Sha256::digest(&payload[base..end]));
            let response = service.execute(&mut store, read).unwrap();
            assert_eq!(&response.data[..response.length as usize], &payload[offset..offset+response.length as usize]);
            read.payload.data[0] ^= 1;
            assert_eq!(service.execute(&mut store, read), Err(RemoteError::RemoteFailure));
        }
        assert_eq!(store.generation(), published);
    }
}

// ------------------------=
// FUNC: recipient_admission_has_no_orphan_reservation_at_any_sector_cut
// DESC: Cuts every write in the actual recipient admission transaction and proves retry/remount preserve ownership and exact physical allocation.
// ------------------=
#[test]
fn recipient_admission_has_no_orphan_reservation_at_any_sector_cut() {
    use crate::native_fabric::service::ReplicaService;
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let payload = [41; 1025]; let request = replica_request(&payload); let d = descriptor(&payload);
    let mut service = ReplicaService::mount(&mut store, d.resource, d.generation).unwrap();
    let baseline = disk.0.borrow().sectors.clone(); let old_usage = store.usage_blocks();
    disk.0.borrow_mut().writes = 0;
    service.execute(&mut store, request).unwrap();
    let writes = disk.0.borrow().writes; let new_usage = store.usage_blocks();
    assert!(new_usage > old_usage);
    for cut in 0..=writes {
        let disk = Disk(Rc::new(RefCell::new(DiskState { sectors: baseline.clone(), writes_left: None, writes: 0 })));
        let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
        let mut service = ReplicaService::mount(&mut store, d.resource, d.generation).unwrap();
        disk.0.borrow_mut().writes_left = Some(cut);
        let _ = service.execute(&mut store, request);
        drop(service); drop(store); disk.0.borrow_mut().writes_left = None;
        let mut store = ObjectStore::mount(disk, 0).unwrap();
        assert!(store.usage_blocks() == old_usage || store.usage_blocks() == new_usage);
        let mut service = ReplicaService::mount(&mut store, d.resource, d.generation).unwrap();
        assert_eq!(service.execute(&mut store, request).unwrap().data[0], 1);
        assert_eq!(store.usage_blocks(), new_usage);
    }
}

// ------------------------=
// FUNC: discovered_resource_observes_capacity_reservations_and_reboot_ordering
// DESC: Uses an explicit GPT fixture and actual native allocations to verify authoritative resource measurements, bounded advertisements, lease expiry and monotonic reboot recovery.
// ------------------=
#[test]
fn discovered_resource_observes_capacity_reservations_and_reboot_ordering() {
    use crate::{native_fabric::service::ReplicaService,
        runtime::{fabric::resources::{Directory, ResourceError}, iop::storage_protocol::Operation},
        storage::object::{crc32, device_identity}};
    let mut disk = Disk::default();
    let mut header = [0; 512]; header[..8].copy_from_slice(b"EFI PART");
    header[12..16].copy_from_slice(&92u32.to_le_bytes()); header[24..32].copy_from_slice(&1u64.to_le_bytes());
    header[32..40].copy_from_slice(&(disk.block_count()-1).to_le_bytes()); header[56..72].fill(21);
    let checksum = crc32(&header[..92]); header[16..20].copy_from_slice(&checksum.to_le_bytes());
    assert!(disk.write_sector(1, &header)); assert_eq!(device_identity(&mut disk), Some([21; 16]));
    let mut damaged = header; damaged[12..16].copy_from_slice(&513u32.to_le_bytes());
    assert!(disk.write_sector(1, &damaged)); assert_eq!(device_identity(&mut disk), None);
    damaged = header; damaged[56] ^= 1;
    assert!(disk.write_sector(1, &damaged)); assert_eq!(device_identity(&mut disk), None);
    assert!(disk.write_sector(1, &header));
    let identity = device_identity(&mut disk);
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let request = replica_request(&[41; 8193]); let d = descriptor(&[41; 8193]);
    let mut service = ReplicaService::mount(&mut store, d.resource, d.generation).unwrap();
    service.attach_device_identity(identity);
    let mut query = request; query.payload.operation = Operation::ResourceInspect;
    query.payload.object = [0; 16]; query.payload.length = 0; query.payload.data = [0; 64];
    let first = service.execute(&mut store, query).unwrap();
    assert_eq!(first.object_version, store.total_blocks() as u64 * 4096);
    assert_eq!(first.offset, (store.total_blocks()-store.usage_blocks()) as u64 * 4096);
    assert_eq!(&first.data[16..32], &[21; 16]);
    assert_eq!(u64::from_le_bytes(first.data[32..40].try_into().unwrap()), 0);
    service.execute(&mut store, request).unwrap();
    let second = service.execute(&mut store, query).unwrap();
    assert!(second.offset < first.offset); assert!(second.manifest_generation > first.manifest_generation);
    assert_eq!(u64::from_le_bytes(second.data[32..40].try_into().unwrap()), 12288);
    let mut advertisement = query; advertisement.payload = second;
    advertisement.payload.operation = Operation::ResourceAdvertise;
    advertisement.payload.object = second.data[..16].try_into().unwrap(); advertisement.payload.value = 60;
    let mut directory = Directory::new();
    assert_eq!(directory.accept_storage_advertisement(advertisement, 10), Ok(true));
    assert_eq!(directory.accept_storage_advertisement(advertisement, 11), Ok(false));
    assert_eq!(directory.entries()[0].unwrap().expires, 70);
    assert_eq!(directory.entries()[0].unwrap().reserved, 12288);
    assert!(directory.usable(0, 69) > 0); directory.expire(70);
    assert_eq!(directory.usable(0, 70), 0); assert_eq!(directory.entries().iter().flatten().count(), 1);
    assert_eq!(directory.accept_storage_advertisement(advertisement, 71), Err(ResourceError::Conflict));
    drop(service); drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    let mut service = ReplicaService::mount(&mut store, d.resource, d.generation).unwrap();
    service.attach_device_identity(identity);
    let rebooted = service.execute(&mut store, query).unwrap();
    assert!(rebooted.manifest_generation > second.manifest_generation);
    advertisement.payload = rebooted; advertisement.payload.operation = Operation::ResourceAdvertise;
    advertisement.payload.object = rebooted.data[..16].try_into().unwrap(); advertisement.payload.value = 60;
    assert_eq!(directory.accept_storage_advertisement(advertisement, 72), Ok(true));
    assert!(directory.usable(0, 72) > 0);
}

// ------------------------=
// FUNC: oversized_extent_cannot_allocate_beyond_native_capacity
// DESC: Rejects a valid-sized transfer larger than the actual native pool without modifying allocation or corrupting the rebootable root.
// ------------------=
#[test]
fn oversized_extent_cannot_allocate_beyond_native_capacity() {
    use crate::storage::object::ObjectError;
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, STORE_RELATIVE_LBA + 80 + 100 * 8, [8; 16]).unwrap();
    assert_eq!(store.total_blocks(), 100);
    let used = store.usage_blocks();
    let writes = disk.0.borrow().writes;
    assert_eq!(store.create_staging_extent(1024 * 1024), Err(ObjectError::InsufficientCapacity));
    assert_eq!(store.usage_blocks(), used);
    assert_eq!(disk.0.borrow().writes, writes);
    drop(store);
    let restored = ObjectStore::mount(disk, 0).unwrap();
    assert_eq!(restored.total_blocks(), 100);
    assert_eq!(restored.usage_blocks(), used);
}

// ------------------------=
// FUNC: verification_releases_store_between_bounded_ticks_and_rejects_stale_jobs
// DESC: Runs independent native work between verification ticks, proves forward progress, and rejects an obsolete verifier after another job publishes.
// ------------------=
#[test]
fn verification_releases_store_between_bounded_ticks_and_rejects_stale_jobs() {
    use crate::native_fabric::extent::{ExtentVerification, NativeExtentReplica};
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let backing = store.create(b"stream", ObjectType::Metadata, Space::System, &[]).unwrap();
    let other = store.create(b"local-work", ObjectType::Metadata, Space::Personal, &[]).unwrap();
    let payload: Vec<u8> = (0..65553).map(|n| (n % 251) as u8).collect();
    let d = descriptor(&payload);
    {
        let mut native = NativeExtentReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
        let mut transfer = Transfer::begin(&mut native, d).unwrap();
        for at in (0..payload.len()).step_by(1024) {
            transfer.receive(&mut native, at as u64, &payload[at..(at+1024).min(payload.len())]).unwrap();
        }
    }
    let mut old = ExtentVerification::resume(&mut store, backing, d.resource, d.generation).unwrap();
    assert_eq!(old.tick(&mut store), Ok(ReplicaState::Verifying));
    let mut job = ExtentVerification::resume(&mut store, backing, d.resource, d.generation).unwrap();
    assert_eq!(job.verified_bytes(), 0);
    for step in 1..=65u64 {
        assert_eq!(job.tick(&mut store), Ok(if step == 65 { ReplicaState::Available } else { ReplicaState::Verifying }));
        assert_eq!(job.verified_bytes(), (step * 1024).min(d.bytes));
        store.replace_state(other, &step.to_le_bytes()).unwrap();
        let mut out = [0; 8]; assert_eq!(store.read(other, None, &mut out), Ok(8));
        assert_eq!(u64::from_le_bytes(out), step);
    }
    assert_eq!(old.tick(&mut store), Err(ReplicaError::Stale));
    drop(store);
    let mut restored = ObjectStore::mount(disk, 0).unwrap();
    let mut native = NativeExtentReplica::open(&mut restored, backing, d.resource, d.generation).unwrap();
    assert_eq!(native.inspect().unwrap().state, ReplicaState::Available);
    let mut out = [0; 1024];
    native.read_verified_chunk(0, &mut out, Sha256::digest(&payload[..1024]).into()).unwrap();
    assert_eq!(out, payload[..1024]);
}

// ------------------------=
// FUNC: streamed_large_replica_recovers_and_seals_without_content_sized_buffers
// DESC: Transfers across the former 16-KiB boundary, loses all process state, resumes, verifies in 1-KiB steps and rejects corrupt chunk reads.
// ------------------=
#[test]
fn streamed_large_replica_recovers_and_seals_without_content_sized_buffers() {
    use crate::native_fabric::extent::NativeExtentReplica;
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let backing = store.create(b"stream", ObjectType::Metadata, Space::System, &[]).unwrap();
    let payload: Vec<u8> = (0..65553).map(|n| (n % 251) as u8).collect();
    let d = descriptor(&payload);
    {
        let mut native = NativeExtentReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
        let mut transfer = Transfer::begin(&mut native, d).unwrap();
        for at in (0..32768).step_by(1024) { transfer.receive(&mut native, at as u64, &payload[at..at+1024]).unwrap(); }
        assert_eq!(native.read_verified_chunk(0, &mut [0; 1024], [0; 32]), Err(ReplicaError::Incomplete));
    }
    let mut record = [0; 160]; store.read(backing, None, &mut record).unwrap();
    let extent = crate::storage::object::ObjectId(record[128..144].try_into().unwrap());
    assert_eq!(store.read(extent, None, &mut [0; 1]), Err(crate::storage::object::ObjectError::Busy));
    drop(store);
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    {
        let mut native = NativeExtentReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
        let mut transfer = Transfer::resume(native.inspect().unwrap()).unwrap();
        assert_eq!(transfer.inspect().copied, 32768);
        transfer.receive(&mut native, 0, &payload[..1024]).unwrap();
        for at in (32768..payload.len()).step_by(1024) { transfer.receive(&mut native, at as u64, &payload[at..(at+1024).min(payload.len())]).unwrap(); }
        for _ in 0..3 { assert_eq!(transfer.verify_tick(&mut native), Ok(ReplicaState::Verifying)); }
        transfer.receive(&mut native, 0, &payload[..1024]).unwrap();
        assert_eq!(transfer.verify_tick(&mut native), Ok(ReplicaState::Verifying));
    }
    drop(store);
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    {
        let mut native = NativeExtentReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
        let mut transfer = Transfer::resume(native.inspect().unwrap()).unwrap();
        for _ in 0..64 { assert_eq!(transfer.verify_tick(&mut native), Ok(ReplicaState::Verifying)); }
        assert_eq!(transfer.verify_tick(&mut native), Ok(ReplicaState::Available));
    }
    drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    assert!(store.write_staging_range(extent, 0, &[0; 1024]).is_err());
    {
        let mut native = NativeExtentReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
        for at in (0..payload.len()).step_by(1024) {
            let part = &payload[at..(at+1024).min(payload.len())];
            let mut out = [0; 1024];
            native.read_verified_chunk(at as u64, &mut out[..part.len()], Sha256::digest(part).into()).unwrap();
            assert_eq!(&out[..part.len()], part);
        }
        let mut out = [0xff; 1024];
        assert_eq!(native.read_verified_chunk(0, &mut out, [0; 32]), Err(ReplicaError::Integrity));
        assert_eq!(out, [0; 1024]);
    }
    let mut out = vec![0; payload.len()];
    assert_eq!(store.read(extent, None, &mut out), Ok(payload.len()));
    assert_eq!(out, payload);
}

// ------------------------=
// FUNC: streamed_reservation_and_seal_are_atomic
// DESC: Injects every native write failure during initial reservation and final publication; no lost reservation or readable partial content survives remount.
// ------------------=
#[test]
fn streamed_reservation_and_seal_are_atomic() {
    use crate::native_fabric::extent::NativeExtentReplica;
    for sealing in [false, true] {
        let disk = Disk::default();
        let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
        let backing = store.create(b"stream", ObjectType::Metadata, Space::System, &[]).unwrap();
        let d = descriptor(&[]);
        if sealing {
            let mut native = NativeExtentReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
            Transfer::begin(&mut native, d).unwrap();
            native.checkpoint(&Checkpoint { descriptor: d, copied: 0, state: ReplicaState::Verifying }).unwrap();
        }
        let baseline = disk.0.borrow().sectors.clone(); let original_usage = store.usage_blocks();
        disk.0.borrow_mut().writes = 0;
        {
            let mut native = NativeExtentReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
            if sealing { Transfer::resume(native.inspect().unwrap()).unwrap().verify_tick(&mut native).unwrap(); }
            else { Transfer::begin(&mut native, d).unwrap(); }
        }
        let count = disk.0.borrow().writes;
        for cut in 0..=count {
            let disk = Disk(Rc::new(RefCell::new(DiskState { sectors: baseline.clone(), writes_left: Some(cut), writes: 0 })));
            let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
            {
                let mut native = NativeExtentReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
                if sealing { let _ = Transfer::resume(native.inspect().unwrap()).unwrap().verify_tick(&mut native); }
                else { let _ = Transfer::begin(&mut native, d); }
            }
            drop(store); disk.0.borrow_mut().writes_left = None;
            let mut store = ObjectStore::mount(disk, 0).unwrap();
            let usage = store.usage_blocks();
            if sealing {
                let published = NativeExtentReplica::open(&mut store,backing,d.resource,d.generation).unwrap()
                    .inspect().unwrap().state == ReplicaState::Available;
                let audit=store.reserved_system_metadata_id(b"@pool-audit",b"/system/storage/pool-audit").and_then(|id|id.ok_or(crate::storage::object::ObjectError::NotFound));
                assert_eq!(audit.is_ok(),published);
                if let Ok(id)=audit {
                    let mut bytes=[0;2080];assert_eq!(store.read(id,None,&mut bytes),Ok(2080));
                    assert_eq!(u64::from_le_bytes(bytes[8..16].try_into().unwrap()),1);
                    assert_eq!(bytes[32+98],7);
                }
            }
            let mut native = NativeExtentReplica::open(&mut store, backing, d.resource, d.generation).unwrap();
            if let Some(checkpoint) = native.inspect() {
                let mut transfer = Transfer::resume(checkpoint).unwrap();
                assert_eq!(transfer.verify_tick(&mut native), Ok(ReplicaState::Available));
                native.read_verified_chunk(0, &mut [], d.hash).unwrap();
            } else { assert!(!sealing); assert_eq!(usage, original_usage); }
        }
    }
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
        policy: StorageClass::Critical, minimum_available: 1, generation: 1, authority: NodeId([8; 32]), authority_generation: 1,
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
// FUNC: native_pool_copy_shares_content_but_not_identity_or_remote_placements
// DESC: Exercises Pool-level COW over actual immutable native extents, independent divergence, cold-mount ownership and honest protection without duplicate remote placement claims.
// ------------------=
#[test]
fn native_pool_copy_shares_content_but_not_identity_or_remote_placements() {
    use crate::runtime::{fabric::placement::{StorageClass, Availability}, node::types::NodeId};
    use crate::storage::object::ObjectId;
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    store.initialize_pool_catalog().unwrap();
    let owner = NodeId([19; 32]); let local = NodeId([18; 32]); let resource = ResourceId([4; 16]);
    let original = store.pool_create(owner, 42, 1, StorageClass::Critical, &[31; 8192], local, resource, [6; 16], 5).unwrap();
    let source = ObjectId(original.object);
    let used = store.usage_by_space(Space::Personal);
    let copied = store.pool_copy(source, owner, 42, 1, 2, local, resource, [6; 16], 5).unwrap();
    let copy = ObjectId(copied.object);
    assert_ne!(source, copy); assert_eq!(copied.chunks, original.chunks);
    assert_eq!(store.usage_by_space(Space::Personal), used);
    assert_eq!(copied.placements.iter().flatten().count(), 1);
    assert_eq!(copied.availability(), Availability::Degraded);
    let generation = store.generation();
    assert_eq!(store.pool_copy(source, owner, 42, 1, 2, local, resource, [6; 16], 5), Ok(copied));
    assert_eq!(store.generation(), generation);
    let source_next = store.pool_update(source, owner, 42, 1, &[41; 8192], local, resource, [6; 16], 5).unwrap();
    let mut bytes = [0; 64]; store.pool_read(&copied, 0, &mut bytes).unwrap(); assert_eq!(bytes, [31; 64]);
    let copy_next = store.pool_update(copy, owner, 42, 1, &[51; 8192], local, resource, [6; 16], 5).unwrap();
    store.pool_read(&source_next, 0, &mut bytes).unwrap(); assert_eq!(bytes, [41; 64]);
    assert_ne!(source_next.chunks, copy_next.chunks);
    drop(store);
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    assert_eq!(store.pool_manifest(copy, owner, 42), Ok(copy_next));
    assert_eq!(store.pool_manifest(source, owner, 42), Ok(source_next));
    store.pool_read(&copy_next, 0, &mut bytes).unwrap(); assert_eq!(bytes, [51; 64]);
    store.destroy_explicit(source, true).unwrap();
    store.collect().unwrap();
    let mut old = [0; 8192]; assert_eq!(store.read(copy, Some(1), &mut old), Ok(8192)); assert_eq!(old, [31; 8192]);
    store.pool_read(&copy_next, 0, &mut bytes).unwrap(); assert_eq!(bytes, [51; 64]);
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
