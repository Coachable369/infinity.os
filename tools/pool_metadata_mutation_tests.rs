use super::*;
use crate::runtime::{storage_metadata::{NativeRequest as N,NativeReply as R},iop::storage_protocol::{StorageOperationV1,Operation},fabric::{placement::StorageClass,resources::ResourceId}};
use crate::fabric_pool_metadata_service::Service;
// ------------------------=
// FUNC: legacy_named_deletion_outbox_migrates_without_replacing_content
// DESC: Preserves the legacy native outbox identity while moving its lookup into the canonical Pool catalog and recovering that reference after reboot.
// ------------------=
#[test]
fn legacy_named_deletion_outbox_migrates_without_replacing_content(){
    let clean=Disk::default();let mut pristine=ObjectStore::format(clean.clone(),0,clean.block_count(),[6;16]).unwrap();
    assert!(pristine.pool_deletion(0,crate::runtime::node::types::NodeId([9;32]),0).unwrap().is_none());
    let root=pristine.generation();assert!(pristine.pool_deletion(0,crate::runtime::node::types::NodeId([9;32]),0).unwrap().is_none());assert_eq!(pristine.generation(),root);
    let(g,_)=group();let disk=Disk::default();let mut store=ObjectStore::format(disk.clone(),0,disk.block_count(),[7;16]).unwrap();store.initialize_pool_catalog().unwrap();
    let mut bytes=[0;9120];bytes[..8].copy_from_slice(b"INFPDEL1");
    let original=store.create_attached(b"pool-deletions",crate::storage::object::ObjectType::Metadata,crate::storage::object::Space::System,&bytes,b"/system/storage/pool-deletions").unwrap();
    assert!(store.pool_deletion(0,g.owner,0).unwrap().is_none());let generation=store.generation();
    assert!(store.pool_deletion(0,g.owner,0).unwrap().is_none());assert_eq!(store.generation(),generation);
    drop(store);let mut store=ObjectStore::mount(disk,0).unwrap();assert_eq!(store.resolve(b"/system/storage/pool-deletions").unwrap(),original);
    assert!(store.pool_deletion(0,g.owner,0).unwrap().is_none());let mut read=[0;9120];assert_eq!(store.read(original,None,&mut read).unwrap(),bytes.len());assert_eq!(read,bytes);
}
// ------------------------=
// FUNC: returned_owner_mutations_require_certified_overlay_and_preserve_bytes
// DESC: Exercises update, copy and delete from a stale owner's local manifest using the durable repaired head and retries every operation after cold mount.
// ------------------=
#[test]
fn returned_owner_mutations_require_certified_overlay_and_preserve_bytes(){
    use crate::runtime::{fabric::metadata_repair::RepairCertificate,storage_metadata_repair::NativeRequest as RepairRequest};
    let(g,keys)=group();
    for operation in [Operation::ObjectUpdate,Operation::ObjectCopy,Operation::ObjectDelete]{
        let disk=Disk::default();let mut store=ObjectStore::format(disk.clone(),0,disk.block_count(),[7;16]).unwrap();
        let mut base=actual_bundle(&mut store,&g,&keys);stage_bundle(&mut store,base).unwrap();base.certificate=Some(certificate(base.value,&keys));publish_bundle(&mut store,base.manifest.object,base.certificate.unwrap()).unwrap();
        let stage=repair_authorization(base,&keys);let mut published=stage;
        published.repair.certificate=Some(RepairCertificate{value:stage.repair.value,prepared:core::array::from_fn(|i|{let mut receipt=Receipt{member:i as u8,digest:stage.repair.value.digest(),published:false,signature:[0;64]};receipt.signature=keys[i].0.sign(keys[i].1,&receipt.transcript()).unwrap();receipt})});
        let mut replica=crate::native_fabric::service::ReplicaService::mount(&mut store,ResourceId([8;16]),1).unwrap();let mut repair=crate::fabric_pool_repair::Service::new();
        repair.execute(&mut store,&mut replica,RepairRequest::Stage{authorization:stage,now:1}).unwrap();
        repair.execute(&mut store,&mut replica,RepairRequest::Publish{authorization:published,now:1}).unwrap();
        assert_eq!(store.pool_manifest(ObjectId(base.manifest.object),g.owner,0).unwrap().generation,1);
        let mut request=StorageOperationV1{operation,object:base.manifest.object,authority_generation:1,manifest_generation:2,object_version:1,offset:0,scope:0,value:0,length:0,data:[0;64]};
        if operation==Operation::ObjectUpdate{request.length=9;request.data[..9].copy_from_slice(b"recovered");}else if operation==Operation::ObjectCopy{request.value=321;}
        let mut service=Service::new();
        let bad=if operation==Operation::ObjectCopy{N::Copy{anchor:base,request,overlay:None}}else{N::Mutate{anchor:base,request}};
        assert!(service.execute(&mut store,bad).is_err());
        let run=||if operation==Operation::ObjectCopy{N::Copy{anchor:base,request,overlay:Some(published.repair)}}else{N::MutateOverlay{anchor:base,request,overlay:published.repair}};
        if operation==Operation::ObjectDelete{assert_eq!((0..32).filter(|i|store.namespace_entry(*i).is_some()).count(),32);}
        let attempted=service.execute(&mut store,run());
        let result=match attempted.unwrap(){R::Mutation{manifest,..}=>manifest,_=>panic!()};
        drop(store);let mut store=ObjectStore::mount(disk.clone(),0).unwrap();let mut service=Service::new();
        match service.execute(&mut store,run()).unwrap(){R::Mutation{manifest,..}=>assert_eq!(manifest,result),_=>panic!()}
        match operation{
            Operation::ObjectUpdate=>{assert_eq!(result.object,base.manifest.object);assert_eq!(result.version,2);let mut bytes=[0;9];store.pool_read(&result,0,&mut bytes).unwrap();assert_eq!(&bytes,b"recovered");},
            Operation::ObjectCopy=>{assert_ne!(result.object,base.manifest.object);let mut bytes=[0;24];store.pool_read(&result,0,&mut bytes).unwrap();assert_eq!(&bytes,b"immutable actual payload");store.pool_read(&base.manifest,0,&mut bytes).unwrap();assert_eq!(&bytes,b"immutable actual payload");},
            Operation::ObjectDelete=>{
                assert!(store.pool_manifest(ObjectId(base.manifest.object),g.owner,0).is_err());
                let obligation=store.pool_deletion(0,g.owner,0).unwrap().unwrap();assert_eq!(obligation.object,base.manifest.object);
                assert!(obligation.placements[1].is_some());
                store.pool_ack_deletion(obligation.object,g.owner,0,obligation.manifest_generation,1).unwrap();
                assert!(store.pool_deletion(0,g.owner,0).unwrap().is_none());
                assert!(store.pool_ack_deletion(obligation.object,g.owner,0,obligation.manifest_generation,1).is_err());
                drop(store);store=ObjectStore::mount(disk.clone(),0).unwrap();assert!(store.pool_deletion(0,g.owner,0).unwrap().is_none());
            },_=>unreachable!()
        }
        assert_eq!(read_bundle(&mut store,base.manifest.object).unwrap().value,base.value);
    }
}
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
