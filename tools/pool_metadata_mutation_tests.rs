use super::*;
use crate::runtime::{storage_metadata::{NativeRequest as N,NativeReply as R},iop::storage_protocol::{StorageOperationV1,Operation},fabric::{placement::StorageClass,resources::ResourceId}};
use crate::fabric_pool_metadata_service::Service;
// ------------------------=
// FUNC: shared_copy_has_independent_identity_and_cow_content
// DESC: Copies a certified owner object once per nonce without advancing its chain and proves later copy writes preserve the source across reboot.
// ------------------=
#[test]
fn shared_copy_has_independent_identity_and_cow_content() {
    let (g, keys) = group();
    let disk = Disk::default();
    let mut store = ObjectStore::format(disk.clone(), 0, disk.block_count(), [7; 16]).unwrap();
    let mut base = actual_bundle(&mut store, &g, &keys);
    stage_bundle(&mut store, base).unwrap();
    base.certificate = Some(certificate(base.value, &keys));
    publish_bundle(&mut store, base.manifest.object, base.certificate.unwrap()).unwrap();
    let request = StorageOperationV1 { operation: Operation::ObjectCopy, object: base.manifest.object,
        authority_generation: 1, manifest_generation: 1, object_version: 1, offset: 0, scope: 0,
        value: 123, length: 0, data: [0; 64] };
    let mut service = Service::new();
    let copy = match service.execute(&mut store, N::Copy { anchor: base, request, overlay: None }).unwrap() {
        R::Mutation { manifest, .. } => manifest, _ => panic!(),
    };
    assert_ne!(copy.object, base.manifest.object);
    assert_eq!(copy.hash, base.manifest.hash);
    let generation = store.generation();
    match service.execute(&mut store, N::Copy { anchor: base, request, overlay: None }).unwrap() {
        R::Mutation { manifest, .. } => assert_eq!(manifest, copy), _ => panic!(),
    }
    assert_eq!(store.generation(), generation);
    let mut content = [0; 24];
    store.pool_read(&copy, 0, &mut content).unwrap();
    assert_eq!(&content, b"immutable actual payload");
    let changed = store.pool_update(ObjectId(copy.object), g.owner, 0, copy.generation,
        b"copy only", g.owner, ResourceId([8; 16]), [9; 16], 1).unwrap();
    assert_ne!(changed.hash, base.manifest.hash);
    assert_eq!(store.pool_manifest(ObjectId(base.manifest.object), g.owner, 0).unwrap(), base.manifest);
    assert_eq!(read_bundle(&mut store, base.manifest.object).unwrap().value, base.value);
    drop(store);
    let mut store = ObjectStore::mount(disk, 0).unwrap();
    store.pool_read(&base.manifest, 0, &mut content).unwrap();
    assert_eq!(&content, b"immutable actual payload");
    let mut updated = [0; 9];
    store.pool_read(&changed, 0, &mut updated).unwrap();
    assert_eq!(&updated, b"copy only");
    assert_eq!(read_bundle(&mut store, base.manifest.object).unwrap().value, base.value);
}
// ------------------------=
// FUNC: shared_update_intent_reboot_and_every_sector_cut
// DESC: Exercises guarded direct writes, exact-once native mutation recovery across every WAL/apply/result cut, and certified finalization.
// ------------------=
#[test]
fn shared_update_intent_reboot_and_every_sector_cut(){
    let(g,keys)=group();let disk=Disk::default();let mut store=ObjectStore::format(disk.clone(),0,disk.block_count(),[7;16]).unwrap();let mut base=actual_bundle(&mut store,&g,&keys);stage_bundle(&mut store,base).unwrap();base.certificate=Some(certificate(base.value,&keys));publish_bundle(&mut store,base.manifest.object,base.certificate.unwrap()).unwrap();let id=ObjectId(base.manifest.object);
    assert!(store.pool_set_policy(id,g.owner,0,1,StorageClass::Critical).is_err());assert!(store.pool_delete(id,g.owner,0,1).is_err());assert!(store.pool_update(id,g.owner,0,1,b"wrong",g.owner,ResourceId([8;16]),[9;16],1).is_err());assert!(store.pool_copy(id,g.owner,0,1,99,g.owner,ResourceId([8;16]),[9;16],1).is_err());
    let mut p=StorageOperationV1{operation:Operation::ObjectUpdate,object:id.0,authority_generation:1,manifest_generation:1,object_version:1,offset:0,scope:0,value:0,length:14,data:[0;64]};p.data[..14].copy_from_slice(b"exactly once!!");
    let baseline=disk.0.borrow().clone();let before=baseline.writes;let mut service=Service::new();let (expected,response)=match service.execute(&mut store,N::Mutate{anchor:base,request:p}).unwrap(){R::Mutation{manifest,response}=>(manifest,response),_=>panic!()};let cost=disk.0.borrow().writes-before;assert_eq!(expected.version,2);
    for cut in 0..=cost{let d=Disk(Rc::new(RefCell::new(baseline.clone())));let mut s=ObjectStore::mount(d.clone(),0).unwrap();let mut svc=Service::new();d.0.borrow_mut().cut=Some(cut);let attempted=svc.execute(&mut s,N::Mutate{anchor:base,request:p});assert_eq!(attempted.is_ok(),cut==cost);d.0.borrow_mut().cut=None;drop(s);let mut s=ObjectStore::mount(d.clone(),0).unwrap();let mut svc=Service::new();
        match svc.execute(&mut s,N::Mutate{anchor:base,request:p}).unwrap(){R::Mutation{manifest,response:r}=>{assert_eq!(manifest,expected);assert_eq!(r,response)},_=>panic!()}
        let mut actual=[0;14];s.pool_read(&expected,0,&mut actual).unwrap();assert_eq!(&actual,b"exactly once!!");let generation=s.generation();svc.execute(&mut s,N::Mutate{anchor:base,request:p}).unwrap();assert_eq!(s.generation(),generation);
        match svc.execute(&mut s,N::PendingMutation{index:0}).unwrap(){R::Pending(Some(intent))=>{assert_eq!(intent.request,p);assert_eq!(intent.anchor.value,base.value)},_=>panic!()}
    }
    assert!(service.execute(&mut store,N::FinalizeMutation{object:id.0,request:p,record:base.value.record}).is_err());
    let mut next=base;next.manifest=expected;next.certificate=None;next.repair_grants=[None;2];next.value.record.generation=2;next.value.record.version=expected.version;next.value.record.previous=base.value.record.digest();let mut raw=[0;5376];expected.encode(&mut raw).unwrap();next.value.record.manifest=Sha256::digest(raw).into();next.value.signature=keys[0].0.sign(keys[0].1,&next.value.record.encode()).unwrap();stage_bundle(&mut store,next).unwrap();publish_bundle(&mut store,id.0,certificate(next.value,&keys)).unwrap();
    service.execute(&mut store,N::FinalizeMutation{object:id.0,request:p,record:next.value.record}).unwrap();match service.execute(&mut store,N::PendingMutation{index:0}).unwrap(){R::Pending(None)=>{},_=>panic!()}
    let finalized=store.generation();service.execute(&mut store,N::FinalizeMutation{object:id.0,request:p,record:next.value.record}).unwrap();assert_eq!(store.generation(),finalized);
    drop(store);let mut store=ObjectStore::mount(disk,0).unwrap();assert_eq!(store.pool_manifest(id,g.owner,0).unwrap(),expected);
}
