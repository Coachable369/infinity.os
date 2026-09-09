use super::*;
use crate::native_fabric::service::ReplicaService;
use crate::runtime::iop::{
    remote::AuthenticatedStorageRequest,
    storage_protocol::{Operation, StorageOperationV1},
};
// ------------------------=
// FUNC: full_namespace_upload_audit_legacy_and_reserved_authority
// DESC: Publishes real 32-KiB content at a full namespace, preserves named legacy audit identity and denies caller-created reserved System metadata.
// ------------------=
#[test]
fn full_namespace_upload_audit_legacy_and_reserved_authority(){
    use crate::storage::object::{ObjectType,Space,ObjectService,ObjectCapabilityPolicy,ObjectOperation,ObjectRef,ObjectCreateRequest};
    let (disk,mut s)=fresh();let bytes:Vec<u8>=(0..32768).map(|n|crate::runtime::storage_fixture::byte_at(17,n)).collect();
    s.pool_upload_begin(OWNER,0,1,StorageClass::Critical,32768,Sha256::digest(&bytes).into(),ObjectId([0;16]),0).unwrap();
    for i in 0..32 {let path=format!("/audit-capacity-{i}");if s.create_attached(b"filler",ObjectType::Metadata,Space::Personal,&[],path.as_bytes()).is_err(){break}}
    assert_eq!((0..32).filter(|i|s.namespace_entry(*i).is_some()).count(),32);
    let m=upload(&mut s,&bytes,1,ObjectId([0;16]),0);verify(&mut s,&m,&bytes);assert_eq!(audit(&mut s).0,1);
    s.pool_create(OWNER,0,2,StorageClass::Protected,b"small",OWNER,RESOURCE,[3;16],1).unwrap();assert_eq!(audit(&mut s).0,2);
    assert!(s.resolve(b"/system/storage/pool-audit").is_err());
    let id=s.reserved_system_metadata_id(b"@pool-audit",b"/system/storage/pool-audit").unwrap().unwrap();
    let mut reboot=ObjectStore::mount(disk,0).unwrap();assert_eq!(reboot.reserved_system_metadata_id(b"@pool-audit",b"/system/storage/pool-audit").unwrap(),Some(id));assert_eq!(audit(&mut reboot).0,2);
    struct Allow;
    impl ObjectCapabilityPolicy for Allow{
        // ------------------------=
        // FUNC: authorize
        // DESC: Grants normal fixture operations so reserved authority denial is independently exercised.
        // ------------------=
        fn authorize(&self,_:ObjectOperation,_:Option<ObjectRef>)->bool{true}
    }
    let mut api=ObjectService::new(&mut reboot,&Allow);
    assert_eq!(api.create(ObjectCreateRequest{name:b"@pool-audit",kind:ObjectType::Metadata,space:Space::System,content:&[]}),Err(ObjectError::Unauthorized));
    assert_eq!(api.update(crate::storage::object::ObjectUpdateRequest{object:ObjectRef{id},content:b"forged"}),Err(ObjectError::Unauthorized));
    assert_eq!(api.delete(crate::storage::object::ObjectDeleteRequest{object:ObjectRef{id}}),Err(ObjectError::Unauthorized));
    let (_,mut legacy)=fresh();let mut ring=[0;2080];ring[..8].copy_from_slice(b"INFPAD01");
    let old=legacy.create_attached(b"pool-audit",ObjectType::Metadata,Space::System,&ring,b"/system/storage/pool-audit").unwrap();
    legacy.pool_create(OWNER,0,2,StorageClass::Protected,b"legacy",OWNER,RESOURCE,[3;16],1).unwrap();
    assert_eq!(legacy.reserved_system_metadata_id(b"@pool-audit",b"/system/storage/pool-audit").unwrap(),Some(old));assert_eq!(audit(&mut legacy).0,1);
    {
        use crate::storage::object::{NamespaceAttachRequest,NamespaceMoveRequest,NamespaceDetachRequest,RelationshipAttachRequest,RelationshipDetachRequest,RelationshipType};
        let mut api=ObjectService::new(&mut legacy,&Allow);
        assert_eq!(api.create(ObjectCreateRequest{name:b"pool-audit",kind:ObjectType::Metadata,space:Space::System,content:&ring}),Err(ObjectError::Unauthorized));
        let personal=api.create(ObjectCreateRequest{name:b"@pool-audit",kind:ObjectType::Metadata,space:Space::Personal,content:b"personal"}).unwrap();
        api.update(crate::storage::object::ObjectUpdateRequest{object:personal,content:b"updated"}).unwrap();
        api.attach(NamespaceAttachRequest{path:b"/system/storage-other",object:personal}).unwrap();
        assert_eq!(api.attach(NamespaceAttachRequest{path:b"/system/storage/forged",object:personal}),Err(ObjectError::Unauthorized));
        assert_eq!(api.move_entry(NamespaceMoveRequest{from:b"/system/storage-other",to:b"/system/storage/pool-audit"}),Err(ObjectError::Unauthorized));
        assert_eq!(api.move_entry(NamespaceMoveRequest{from:b"/system/storage/pool-audit",to:b"/moved"}),Err(ObjectError::Unauthorized));
        assert_eq!(api.detach(NamespaceDetachRequest{path:b"/system/storage/pool-audit"}),Err(ObjectError::Unauthorized));
        assert_eq!(api.relationship_attach(RelationshipAttachRequest{source:personal,kind:RelationshipType::References,target:ObjectRef{id:old},flags:0}),Err(ObjectError::Unauthorized));
        assert_eq!(api.relationship_detach(RelationshipDetachRequest{source:personal,kind:RelationshipType::References,target:ObjectRef{id:old}}),Err(ObjectError::Unauthorized));
        api.move_entry(NamespaceMoveRequest{from:b"/system/storage-other",to:b"/normal"}).unwrap();api.detach(NamespaceDetachRequest{path:b"/normal"}).unwrap();api.delete(crate::storage::object::ObjectDeleteRequest{object:personal}).unwrap();
    }
    legacy.create(b"@pool-audit",ObjectType::Metadata,Space::Personal,&[]).unwrap();assert_eq!(audit(&mut legacy).0,1);
    legacy.create(b"@pool-audit",ObjectType::Metadata,Space::System,&ring).unwrap();assert_eq!(legacy.reserved_system_metadata_id(b"@pool-audit",b"/system/storage/pool-audit"),Err(ObjectError::CorruptContent));
}
// ------------------------=
// FUNC: audit
// DESC: Reads structured ring sequence and most recent binary lifecycle record.
// ------------------=
pub(super) fn audit(store:&mut ObjectStore<Disk>)->(u64,[u8;128]) {
    let id=store.reserved_system_metadata_id(b"@pool-audit",b"/system/storage/pool-audit").unwrap().unwrap();let mut bytes=[0;2080];
    assert_eq!(store.read(id,None,&mut bytes),Ok(2080));
    let sequence=u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    let at=32+((sequence-1)%16) as usize*128;(sequence,bytes[at..at+128].try_into().unwrap())
}
// ------------------------=
// FUNC: lifecycle_audit_is_bounded_and_retries_do_not_duplicate
// DESC: Exercises create/copy/update/upload/delete/recipient publication and verifies exact durable kinds and retry behavior without per-window audit spam.
// ------------------=
#[test]
fn lifecycle_audit_is_bounded_and_retries_do_not_duplicate() {
    let (disk,mut store)=fresh();
    let m=store.pool_create(OWNER,0,1,StorageClass::Protected,b"first",OWNER,RESOURCE,[3;16],1).unwrap();
    assert_eq!(audit(&mut store).0,1);assert_eq!(audit(&mut store).1[98],2);
    store.pool_create(OWNER,0,1,StorageClass::Protected,b"first",OWNER,RESOURCE,[3;16],1).unwrap();
    assert_eq!(audit(&mut store).0,1);
    let copy=store.pool_copy(ObjectId(m.object),OWNER,0,1,2,OWNER,RESOURCE,[3;16],1).unwrap();
    assert_eq!(audit(&mut store).1[98],3);
    store.pool_copy(ObjectId(m.object),OWNER,0,1,2,OWNER,RESOURCE,[3;16],1).unwrap();assert_eq!(audit(&mut store).0,2);
    let mut m=m;
    for n in 0..18 {
        m=store.pool_update(ObjectId(m.object),OWNER,0,m.generation,&[n],OWNER,RESOURCE,[3;16],1).unwrap();
        assert_eq!(audit(&mut store).1[98],4);
    }
    assert_eq!(audit(&mut store).0,20);
    let uploaded=upload(&mut store,&[8;17000],3,ObjectId([0;16]),0);
    assert_eq!(audit(&mut store).0,21);assert_eq!(audit(&mut store).1[98],5);
    store.pool_delete(ObjectId(copy.object),OWNER,0,copy.generation).unwrap();
    assert_eq!(audit(&mut store).0,22);assert_eq!(audit(&mut store).1[98],6);
    let mut service=ReplicaService::mount(&mut store,RESOURCE,1).unwrap();let mut request=replica(&mut store,&mut service,1);
    let before=audit(&mut store);assert_eq!(before.0,23);assert_eq!(before.1[98],7);assert_eq!(&before.1[48..80],&OWNER.0);
    request.payload.operation=Operation::TransferCommit;request.payload.length=0;request.payload.data=[0;64];request.payload.offset=0;
    service.execute(&mut store,request).unwrap();assert_eq!(audit(&mut store),before);
    drop(store);let mut store=ObjectStore::mount(disk,0).unwrap();assert_eq!(audit(&mut store),before);
    verify(&mut store,&uploaded,&[8;17000]);
}
// ------------------------=
// FUNC: create_and_update_audits_share_every_power_cut
// DESC: Cuts every transaction sector and requires application publication and audit to recover together, including initial ring creation.
// ------------------=
#[test]
fn create_and_update_audits_share_every_power_cut() {
    let (disk,mut store)=fresh();let baseline=disk.0.borrow().clone();let before=baseline.writes;
    let m=store.pool_create(OWNER,0,1,StorageClass::Protected,b"one",OWNER,RESOURCE,[3;16],1).unwrap();
    let cost=disk.0.borrow().writes-before;
    for cut in 0..cost {
        let d=Disk(Rc::new(RefCell::new(baseline.clone())));let mut s=ObjectStore::mount(d.clone(),0).unwrap();d.0.borrow_mut().remaining=Some(cut);
        assert!(s.pool_create(OWNER,0,1,StorageClass::Protected,b"one",OWNER,RESOURCE,[3;16],1).is_err());
        d.0.borrow_mut().remaining=None;let mut s=ObjectStore::mount(d,0).unwrap();
        assert_eq!(s.pool_inspect(OWNER,0,0).unwrap().0,0);assert_eq!(s.reserved_system_metadata_id(b"@pool-audit",b"/system/storage/pool-audit").unwrap(),None);
    }
    let baseline=disk.0.borrow().clone();let before=baseline.writes;
    store.pool_update(ObjectId(m.object),OWNER,0,1,b"two",OWNER,RESOURCE,[3;16],1).unwrap();let cost=disk.0.borrow().writes-before;
    for cut in 0..cost {
        let d=Disk(Rc::new(RefCell::new(baseline.clone())));let mut s=ObjectStore::mount(d.clone(),0).unwrap();d.0.borrow_mut().remaining=Some(cut);
        assert!(s.pool_update(ObjectId(m.object),OWNER,0,1,b"two",OWNER,RESOURCE,[3;16],1).is_err());
        d.0.borrow_mut().remaining=None;let mut s=ObjectStore::mount(d,0).unwrap();
        assert_eq!(s.pool_manifest(ObjectId(m.object),OWNER,0).unwrap().version,1);assert_eq!(audit(&mut s).0,1);
    }
}
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
    let audit = store.reserved_system_metadata_id(b"@pool-audit",b"/system/storage/pool-audit").unwrap().unwrap();
    let mut bytes = [0; 2080];
    store.read(audit, None, &mut bytes).unwrap();
    assert_eq!(u64::from_le_bytes(bytes[8..16].try_into().unwrap()), 3);
    assert_eq!(
        u64::from_le_bytes(bytes[32 + 256 + 24..32 + 256 + 32].try_into().unwrap()),
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
        assert_eq!(u64::from_le_bytes(bytes[8..16].try_into().unwrap()), 2);
        assert_eq!(u64::from_le_bytes(bytes[184..192].try_into().unwrap()), 2);
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
