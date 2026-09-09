use super::*;
use crate::runtime::{crypto::{NodeCrypto,KeyRef},fabric::metadata::*};
use sha2::{Digest,Sha256};
use std::{rc::Rc,cell::RefCell,collections::BTreeMap};
#[derive(Clone,Default)]
struct Disk(Rc<RefCell<Media>>);
#[derive(Clone,Default)]
struct Media {sectors:BTreeMap<u64,[u8;512]>,writes:usize,cut:Option<usize>}
impl BlockDevice for Disk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Defines bounded sparse native test media.
    // ------------------=
    fn block_count(&self)->u64 {400000}
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads actual persisted fixture sectors.
    // ------------------=
    fn read_sector(&mut self,l:u64,o:&mut[u8;512])->bool {*o=*self.0.borrow().sectors.get(&l).unwrap_or(&[0;512]);true}
    // ------------------------=
    // FUNC: write_sector
    // DESC: Injects exact sector failures without mutating rejected sectors.
    // ------------------=
    fn write_sector(&mut self,l:u64,b:&[u8;512])->bool {let mut m=self.0.borrow_mut();if m.cut==Some(0){return false;}if let Some(n)=m.cut.as_mut(){*n-=1;}m.writes+=1;m.sectors.insert(l,*b);true}
    // ------------------------=
    // FUNC: flush
    // DESC: Models stable writes for native commit ordering.
    // ------------------=
    fn flush(&mut self)->bool {true}
}
// ------------------------=
// FUNC: group
// DESC: Creates explicit real signing identities without inferring membership from discovery.
// ------------------=
fn group()->(Group,[(NodeCrypto,KeyRef);3]) {
    let keys=core::array::from_fn(|i|{let mut c=NodeCrypto::new();let k=c.initialize(&[i as u8+1;32],true).unwrap();(c,k)});
    let public=core::array::from_fn(|i|keys[i].0.public_identity().unwrap());
    let members=public.map(|key|{let mut h=Sha256::new();h.update(b"InfinityOS NodeId v1");h.update(key);NodeId(h.finalize().into())});
    (Group {epoch:1,owner:members[0],members,keys:public},keys)
}
// ------------------------=
// FUNC: signed
// DESC: Signs an immutable metadata descriptor for one object.
// ------------------=
fn signed(g:&Group,keys:&[(NodeCrypto,KeyRef);3],object:u8)->SignedRecord {
    let r=Record {group:g.digest(),object:[object;16],generation:1,version:1,previous:[0;32],manifest:[3;32],namespace:[4;32],policy:[5;32],revocation:1,deleted:false};
    SignedRecord {record:r,signature:keys[0].0.sign(keys[0].1,&r.encode()).unwrap()}
}
// ------------------------=
// FUNC: certificate
// DESC: Produces two distinct cryptographically valid durable-staging receipts.
// ------------------=
fn certificate(value:SignedRecord,keys:&[(NodeCrypto,KeyRef);3])->Certificate {
    Certificate {value,prepared:core::array::from_fn(|i|{let mut p=Receipt {member:i as u8,digest:value.record.digest(),published:false,signature:[0;64]};p.signature=keys[i].0.sign(keys[i].1,&p.transcript()).unwrap();p})}
}
// ------------------------=
// FUNC: persistence_callback_roundtrip_and_capacity
// DESC: Verifies native staging/publication recovery, read-only load, eight-object bounds and replay idempotency using actual signatures.
// ------------------=
#[test]
fn persistence_callback_roundtrip_and_capacity() {
    let (g,keys)=group();let disk=Disk::default();let mut store=ObjectStore::format(disk.clone(),0,disk.block_count(),[7;16]).unwrap();
    let writes=disk.0.borrow().writes;assert!(load(&mut store,&g,[1;16]).unwrap().staged.is_none());assert_eq!(disk.0.borrow().writes,writes);
    for n in 1..=8 {
        let value=signed(&g,&keys,n);let mut r=load(&mut store,&g,[n;16]).unwrap();let old=r;
        r.prepare(&g,value,|next|persist(&mut store,&g,[n;16],old,next)).unwrap();
        let generation=store.generation();persist(&mut store,&g,[n;16],old,r).unwrap();assert_eq!(store.generation(),generation);
        let old=r;r.publish(&g,certificate(value,&keys),|next|persist(&mut store,&g,[n;16],old,next)).unwrap();
    }
    let ninth=Replica {staged:Some(signed(&g,&keys,9)),committed:None};let generation=store.generation();
    assert_eq!(persist(&mut store,&g,[9;16],Replica::default(),ninth),Err(Error::ResourceLimit));assert_eq!(store.generation(),generation);
    drop(store);let mut store=ObjectStore::mount(disk,0).unwrap();
    for n in 1..=8 {let r=load(&mut store,&g,[n;16]).unwrap();assert!(r.staged.is_none());assert_eq!(r.committed.unwrap(),certificate(signed(&g,&keys,n),&keys));}
}
// ------------------------=
// FUNC: every_sector_cut_keeps_old_or_new_canonical_replica
// DESC: Cuts initial catalog creation and committed publication; no acknowledgement state advances unless the complete native transaction survives reboot.
// ------------------=
#[test]
fn every_sector_cut_keeps_old_or_new_canonical_replica() {
    let (g,keys)=group();let disk=Disk::default();let mut store=ObjectStore::format(disk.clone(),0,disk.block_count(),[7;16]).unwrap();
    let value=signed(&g,&keys,1);let stage=Replica {staged:Some(value),committed:None};let published=Replica {staged:None,committed:Some(certificate(value,&keys))};
    for (old,next) in [(Replica::default(),stage),(stage,published)] {
        let baseline=disk.0.borrow().clone();let before=baseline.writes;persist(&mut store,&g,[1;16],old,next).unwrap();let cost=disk.0.borrow().writes-before;
        for cut in 0..=cost {
            let d=Disk(Rc::new(RefCell::new(baseline.clone())));let mut s=ObjectStore::mount(d.clone(),0).unwrap();d.0.borrow_mut().cut=Some(cut);
            let result=persist(&mut s,&g,[1;16],old,next);assert_eq!(result.is_ok(),cut==cost);
            d.0.borrow_mut().cut=None;drop(s);let mut s=ObjectStore::mount(d,0).unwrap();
            assert_eq!(replica_bytes(load(&mut s,&g,[1;16]).unwrap()),replica_bytes(if cut==cost{next}else{old}));
        }
    }
}
// ------------------------=
// FUNC: malformed_quorum_and_stale_callback_fail_closed
// DESC: Rejects forged signatures, duplicate receipt members, mixed identities, stale CAS and noncanonical persisted fields rather than resetting durable metadata.
// ------------------=
#[test]
fn malformed_quorum_and_stale_callback_fail_closed() {
    let (g,keys)=group();let disk=Disk::default();let mut store=ObjectStore::format(disk.clone(),0,disk.block_count(),[7;16]).unwrap();
    let value=signed(&g,&keys,1);let stage=Replica {staged:Some(value),committed:None};persist(&mut store,&g,[1;16],Replica::default(),stage).unwrap();
    let mut cert=certificate(value,&keys);cert.prepared[1]=cert.prepared[0];
    assert_eq!(persist(&mut store,&g,[1;16],stage,Replica {staged:None,committed:Some(cert)}),Err(Error::Quorum));
    cert=certificate(value,&keys);cert.value.signature[0]^=1;
    assert_eq!(persist(&mut store,&g,[1;16],stage,Replica {staged:None,committed:Some(cert)}),Err(Error::Signature));
    let published=Replica {staged:None,committed:Some(certificate(value,&keys))};
    assert_eq!(persist(&mut store,&g,[1;16],Replica::default(),published),Err(Error::Stale));
    assert_eq!(persist(&mut store,&g,[2;16],stage,published),Err(Error::Denied));
    let mut other=g;other.epoch+=1;assert!(matches!(load(&mut store,&other,[1;16]),Err(Error::Denied)));
    let (mut bytes,id)=read_catalog(&mut store).unwrap();bytes[32+240+2]=1;store.replace_state(id.unwrap(),&bytes).unwrap();
    assert!(matches!(load(&mut store,&g,[1;16]),Err(Error::Invalid)));
}
