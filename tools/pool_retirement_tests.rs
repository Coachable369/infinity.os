use super::*;
use crate::native_fabric::service::ReplicaService;
use crate::runtime::iop::{
    remote::AuthenticatedStorageRequest,
    storage_protocol::{Operation, StorageOperationV1},
};
// ------------------------=
// FUNC: persisted_verifier_checks_actual_bytes_without_mutation
// DESC: Verifies native recipient storage after mount and rejects identity/hash/version mismatch and actual payload corruption without any writes.
// ------------------=
#[test]
fn persisted_verifier_checks_actual_bytes_without_mutation() {
    use crate::native_fabric::service::verify_persisted_replica;
    let (disk, mut store) = fresh();
    let mut service = ReplicaService::mount(&mut store, RESOURCE, 1).unwrap();
    replica(&mut store, &mut service, 3);
    drop(store);
    let mut store = ObjectStore::mount(disk.clone(),0).unwrap();
    let hash = Sha256::digest([3;32]).into();
    let writes = disk.0.borrow().writes;
    assert_eq!(verify_persisted_replica(&mut store,OWNER.0,[90;16],3,hash),Ok(32));
    assert!(verify_persisted_replica(&mut store,[8;32],[90;16],3,hash).is_err());
    assert!(verify_persisted_replica(&mut store,OWNER.0,[90;16],2,hash).is_err());
    assert!(verify_persisted_replica(&mut store,OWNER.0,[90;16],3,[0;32]).is_err());
    assert_eq!(disk.0.borrow().writes,writes);
    let mut state = disk.0.borrow_mut();
    let sector = state.sectors.values_mut().find(|s| s[..32]==[3;32] && s[32..]==[0;480]).unwrap();
    sector[4] ^= 1; drop(state);
    assert!(verify_persisted_replica(&mut store,OWNER.0,[90;16],3,hash).is_err());
    assert_eq!(disk.0.borrow().writes,writes);
}
// ------------------------=
// FUNC: recipient_catalog_migrates_readonly_and_bounds_eight_versions
// DESC: Mounts a legacy four-slot catalog without writes, migrates on mutation, retains eight immutable versions and rejects ninth admission without corrupting history.
// ------------------=
#[test]
fn recipient_catalog_migrates_readonly_and_bounds_eight_versions() {
    use crate::native_fabric::service::verify_persisted_replica;
    let (disk,mut store)=fresh();
    let mut service=ReplicaService::mount(&mut store,RESOURCE,1).unwrap();
    for version in 1..=3 { replica(&mut store,&mut service,version); }
    let catalog=store.resolve(b"/system/storage/replicas").unwrap();
    let mut encoded=[0;1696];store.read(catalog,None,&mut encoded).unwrap();
    encoded[..8].copy_from_slice(b"INFREP01");
    store.replace_state(catalog,&encoded[..864]).unwrap();
    drop(service);drop(store);
    let mut store=ObjectStore::mount(disk.clone(),0).unwrap();
    let writes=disk.0.borrow().writes;
    for version in 1..=3 {
        assert_eq!(verify_persisted_replica(&mut store,OWNER.0,[90;16],version,Sha256::digest([version as u8;32]).into()),Ok(32));
    }
    assert_eq!(disk.0.borrow().writes,writes);
    let mut service=ReplicaService::mount(&mut store,RESOURCE,1).unwrap();
    let mut last=None;
    for version in 4..=8 { last=Some(replica(&mut store,&mut service,version)); }
    assert_eq!(store.read(catalog,None,&mut encoded).unwrap(),1696);
    let mut ninth=last.unwrap();ninth.payload.object_version=9;ninth.payload.manifest_generation=9;ninth.payload.value=109;
    ninth.payload.data[24..56].copy_from_slice(&Sha256::digest([9;32]));
    let generation=store.generation();
    assert!(service.execute(&mut store,ninth).is_err());assert_eq!(store.generation(),generation);
    drop(service);drop(store);
    let mut store=ObjectStore::mount(disk,0).unwrap();
    for version in 1..=8 {
        assert_eq!(verify_persisted_replica(&mut store,OWNER.0,[90;16],version,Sha256::digest([version as u8;32]).into()),Ok(32));
    }
}
// ------------------------=
// FUNC: replica
// DESC: Publishes a real native recipient extent through bounded typed transfer operations.
// ------------------=
fn replica(
    store: &mut ObjectStore<Disk>,
    service: &mut ReplicaService,
    version: u64,
) -> AuthenticatedStorageRequest {
    let bytes = [version as u8; 32];
    let mut data = [0; 64];
    data[..16].copy_from_slice(&RESOURCE.0);
    data[16..24].copy_from_slice(&1u64.to_le_bytes());
    data[24..56].copy_from_slice(&Sha256::digest(bytes));
    let request = AuthenticatedStorageRequest {
        local: NodeId([9; 32]),
        peer: OWNER,
        session_reference: [6; 16],
        grant: 1,
        request_id: 1,
        correlation: 2,
        causation: 3,
        payload: StorageOperationV1 {
            operation: Operation::TransferBegin,
            object: [90; 16],
            authority_generation: 1,
            manifest_generation: version,
            object_version: version,
            offset: 32,
            scope: 7,
            value: 100 + version,
            length: 56,
            data,
        },
    };
    service.execute(store, request).unwrap();
    let mut chunk = request;
    chunk.payload.operation = Operation::TransferChunk;
    chunk.payload.offset = 0;
    chunk.payload.length = 32;
    chunk.payload.data = [0; 64];
    chunk.payload.data[..32].copy_from_slice(&bytes);
    service.execute(store, chunk).unwrap();
    let mut commit = request;
    commit.payload.operation = Operation::TransferCommit;
    commit.payload.offset = 0;
    commit.payload.length = 0;
    commit.payload.data = [0; 64];
    for _ in 0..3 {
        if service.execute(store, commit).unwrap().data[0] == 4 {
            return request;
        }
    }
    panic!("bounded recipient publication incomplete")
}
// ------------------------=
// FUNC: recipient_retirement_fences_versions_retries_and_reboot
// DESC: Retires all historical recipient extents atomically, rejects foreign ownership and obsolete requests, and prevents stale transfer resurrection.
// ------------------=
#[test]
fn recipient_retirement_fences_versions_retries_and_reboot() {
    let (disk, mut store) = fresh();
    let mut service = ReplicaService::mount(&mut store, RESOURCE, 1).unwrap();
    let first = replica(&mut store, &mut service, 1);
    let second = replica(&mut store, &mut service, 2);
    let last = replica(&mut store, &mut service, 3);
    let mut delete = last;
    delete.payload.operation = Operation::ReplicaDelete;
    delete.payload.data = [0; 64];
    delete.payload.length = 0;
    let before = store.generation();
    for attack in 0..5 {
        let mut wrong = delete;
        match attack {
            0 => wrong.peer = NodeId([7; 32]),
            1 => wrong.payload.scope += 1,
            2 => wrong.payload.authority_generation += 1,
            3 => wrong.payload.value += 1,
            _ => wrong.payload.object_version = 2,
        };
        assert!(service.execute(&mut store, wrong).is_err());
        assert_eq!(store.generation(), before);
    }
    let mut old = second;
    old.payload.operation = Operation::ReplicaDelete;
    old.payload.length = 0;
    old.payload.data = [0; 64];
    assert!(service.execute(&mut store, old).is_err());
    let snapshot = disk.0.borrow().clone();
    let writes = snapshot.writes;
    service.execute(&mut store, delete).unwrap();
    let cost = disk.0.borrow().writes - writes;
    let committed = store.generation();
    assert_eq!(service.execute(&mut store, delete).unwrap().value, 1);
    assert_eq!(store.generation(), committed);
    drop(store);
    let mut store = ObjectStore::mount(disk.clone(), 0).unwrap();
    let mut service = ReplicaService::mount(&mut store, RESOURCE, 1).unwrap();
    assert_eq!(service.execute(&mut store, delete).unwrap().value, 1);
    for request in [first, second, last] {
        assert!(service.execute(&mut store, request).is_err());
        let mut inspect = request;
        inspect.payload.operation = Operation::ReplicaInspect;
        inspect.payload.length = 0;
        inspect.payload.data = [0; 64];
        assert!(service.execute(&mut store, inspect).is_err());
    }
    for cut in 0..cost {
        let trial = Disk(Rc::new(RefCell::new(snapshot.clone())));
        trial.0.borrow_mut().remaining = Some(cut);
        let mut interrupted = ObjectStore::mount(trial.clone(), 0).unwrap();
        let mut svc = ReplicaService::new(RESOURCE, 1);
        assert!(svc.execute(&mut interrupted, delete).is_err());
        trial.0.borrow_mut().remaining = None;
        let mut recovered = ObjectStore::mount(trial, 0).unwrap();
        let mut svc = ReplicaService::new(RESOURCE, 1);
        for request in [first, second, last] {
            let mut inspect = request;
            inspect.payload.operation = Operation::ReplicaInspect;
            inspect.payload.length = 0;
            inspect.payload.data = [0; 64];
            assert_eq!(svc.execute(&mut recovered, inspect).unwrap().data[0], 4);
        }
    }
}
// ------------------------=
// FUNC: deletion_outbox_retains_offline_placements_and_cow_after_reboot
// DESC: Proves logical deletion, persistent retirement obligations and shared-content retention are committed together until exact acknowledgements arrive.
// ------------------=
#[test]
fn deletion_outbox_retains_offline_placements_and_cow_after_reboot() {
    let (disk, mut store) = fresh();
    let bytes = vec![29; 18000];
    let original = upload(&mut store, &bytes, 1, ObjectId([0; 16]), 0);
    let copy = store
        .pool_copy(
            ObjectId(original.object),
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
    let mut next = original;
    next.generation = 2;
    next.placements[1] = Some(Placement {
        node: NodeId([7; 32]),
        resource: ResourceId([7; 16]),
        device: [8; 16],
        generation: 1,
        version: 1,
        hash: original.hash,
        state: PlacementState::Offline,
        admission_generation: 1,
    });
    next.placements[2] = Some(Placement {
        node: NodeId([8; 32]),
        resource: ResourceId([9; 16]),
        device: [10; 16],
        generation: 1,
        version: 1,
        hash: original.hash,
        state: PlacementState::Stale,
        admission_generation: 1,
    });
    store
        .pool_commit_manifest(ObjectId(original.object), OWNER, 0, 1, &next)
        .unwrap();
    assert!(store.pool_deletion(0, OWNER, 0).unwrap().is_none());
    let snapshot = disk.0.borrow().clone();
    let before = snapshot.writes;
    store
        .pool_delete(ObjectId(original.object), OWNER, 0, 2)
        .unwrap();
    let cost = disk.0.borrow().writes - before;
    drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    let deletion = store.pool_deletion(0, OWNER, 0).unwrap().unwrap();
    assert_eq!(deletion.placements[1], next.placements[1]);
    assert_eq!(deletion.placements[2], next.placements[2]);
    assert_eq!(deletion.acknowledged & 6, 0);
    verify(&mut store, &copy, &bytes);
    assert!(store
        .pool_ack_deletion(original.object, OWNER, 0, 1, 1)
        .is_err());
    store
        .pool_ack_deletion(original.object, OWNER, 0, 2, 1)
        .unwrap();
    store
        .pool_ack_deletion(original.object, OWNER, 0, 2, 1)
        .unwrap();
    assert!(store.pool_deletion(0, OWNER, 0).unwrap().is_some());
    store
        .pool_ack_deletion(original.object, OWNER, 0, 2, 2)
        .unwrap();
    assert!(store.pool_deletion(0, OWNER, 0).unwrap().is_none());
    verify(&mut store, &copy, &bytes);
    for cut in 0..cost {
        let trial = Disk(Rc::new(RefCell::new(snapshot.clone())));
        trial.0.borrow_mut().remaining = Some(cut);
        let mut interrupted = ObjectStore::mount(trial.clone(), 0).unwrap();
        assert!(interrupted
            .pool_delete(ObjectId(original.object), OWNER, 0, 2)
            .is_err());
        trial.0.borrow_mut().remaining = None;
        let mut recovered = ObjectStore::mount(trial, 0).unwrap();
        assert_eq!(
            recovered
                .pool_manifest(ObjectId(original.object), OWNER, 0)
                .unwrap()
                .generation,
            2
        );
        assert!(recovered.pool_deletion(0, OWNER, 0).unwrap().is_none());
        let mut out = [0; 64];
        recovered.pool_read(&copy, 0, &mut out).unwrap();
        assert_eq!(out, [29; 64]);
    }
}
// ------------------------=
// FUNC: manifest_and_audit_commit_share_power_loss_boundary
// DESC: Cuts every manifest/audit paired write and verifies recovery never observes a transition without its matching durable audit record.
// ------------------=
#[test]
fn manifest_and_audit_commit_share_power_loss_boundary() {
    let (disk, mut store) = fresh();
    let initial = store
        .pool_create(
            OWNER,
            0,
            1,
            StorageClass::Protected,
            b"audit",
            OWNER,
            RESOURCE,
            [3; 16],
            1,
        )
        .unwrap();
    let mut next = initial;
    next.generation = 2;
    next.placements[0].as_mut().unwrap().state = PlacementState::Offline;
    store
        .pool_commit_manifest(ObjectId(initial.object), OWNER, 0, 1, &next)
        .unwrap();
    let mut final_manifest = next;
    final_manifest.generation = 3;
    final_manifest.placements[0].as_mut().unwrap().state = PlacementState::Verified;
    let snapshot = disk.0.borrow().clone();
    let before = snapshot.writes;
    store
        .pool_commit_manifest(ObjectId(initial.object), OWNER, 0, 2, &final_manifest)
        .unwrap();
    let cost = disk.0.borrow().writes - before;
    let audit = store.resolve(b"/system/storage/pool-audit").unwrap();
    let mut bytes = [0; 2080];
    store.read(audit, None, &mut bytes).unwrap();
    assert_eq!(u64::from_le_bytes(bytes[8..16].try_into().unwrap()), 2);
    assert_eq!(
        u64::from_le_bytes(bytes[32 + 128 + 24..32 + 128 + 32].try_into().unwrap()),
        3
    );
    for cut in 0..cost {
        let trial = Disk(Rc::new(RefCell::new(snapshot.clone())));
        trial.0.borrow_mut().remaining = Some(cut);
        let mut interrupted = ObjectStore::mount(trial.clone(), 0).unwrap();
        assert!(interrupted
            .pool_commit_manifest(ObjectId(initial.object), OWNER, 0, 2, &final_manifest)
            .is_err());
        trial.0.borrow_mut().remaining = None;
        let mut recovered = ObjectStore::mount(trial, 0).unwrap();
        assert_eq!(
            recovered
                .pool_manifest(ObjectId(initial.object), OWNER, 0)
                .unwrap()
                .generation,
            2
        );
        recovered.read(audit, None, &mut bytes).unwrap();
        assert_eq!(u64::from_le_bytes(bytes[8..16].try_into().unwrap()), 1);
        assert_eq!(u64::from_le_bytes(bytes[56..64].try_into().unwrap()), 2);
    }
}
// ------------------------=
// FUNC: full_remote_outbox_does_not_block_local_only_reclamation
// DESC: Preserves all eight pending offline retirement obligations while permitting a deletion requiring no additional remote entry.
// ------------------=
#[test]
fn full_remote_outbox_does_not_block_local_only_reclamation() {
    let (_, mut store) = fresh();
    for nonce in 1..=8 {
        let original = store
            .pool_create(
                OWNER,
                0,
                nonce,
                StorageClass::Protected,
                b"remote",
                OWNER,
                RESOURCE,
                [3; 16],
                1,
            )
            .unwrap();
        let mut next = original;
        next.generation = 2;
        next.placements[1] = Some(Placement {
            node: NodeId([7; 32]),
            resource: ResourceId([7; 16]),
            device: [8; 16],
            generation: 1,
            version: 1,
            hash: original.hash,
            state: PlacementState::Offline,
            admission_generation: 1,
        });
        store
            .pool_commit_manifest(ObjectId(original.object), OWNER, 0, 1, &next)
            .unwrap();
        store
            .pool_delete(ObjectId(original.object), OWNER, 0, 2)
            .unwrap();
    }
    for index in 0..8 {
        assert!(store.pool_deletion(index, OWNER, 0).unwrap().is_some());
    }
    let local = store
        .pool_create(
            OWNER,
            0,
            99,
            StorageClass::Temporary,
            b"local",
            OWNER,
            RESOURCE,
            [3; 16],
            1,
        )
        .unwrap();
    store
        .pool_delete(ObjectId(local.object), OWNER, 0, 1)
        .unwrap();
    for index in 0..8 {
        assert!(store.pool_deletion(index, OWNER, 0).unwrap().is_some());
    }
}
