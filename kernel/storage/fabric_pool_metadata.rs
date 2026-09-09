//! Native fixed-capacity durability for metadata-quorum primitives only.
//! No namespace, application read, network authority or writer election is added.
use crate::storage::{BlockDevice,object::{ObjectStore,ObjectError,ObjectType,Space,ObjectId}};
use crate::runtime::{fabric::metadata::{Group,Replica,SignedRecord,Record,Certificate,Receipt,Error},node::types::NodeId};
pub(crate) const CAPACITY:usize=8;
const ENTRY:usize=1120;
const BYTES:usize=32+CAPACITY*ENTRY;
const PATH:&[u8]=b"/system/storage/pool-quorum-replicas";
const RESERVED:&[u8]=b"@pool-quorum-replicas";
#[path="fabric_pool_metadata_payload.rs"]
mod payload;
pub(crate) use payload::{load_bundle,stage_bundle,publish_bundle,publish_received_bundle,read_bundle};
pub(crate) use payload::{read_repair_payload,write_repair_payload};
pub(crate) use payload::{read_mutation_payload,write_mutation_payload,is_shared_object};

// ------------------------=
// FUNC: group_bytes
// DESC: Canonically preserves explicit membership keys and owner without deriving authority from discovery.
// ------------------=
fn group_bytes(g:&Group)->[u8;240] {
    let mut b=[0;240];b[..8].copy_from_slice(&g.epoch.to_le_bytes());b[8..40].copy_from_slice(&g.owner.0);
    for i in 0..3 {b[40+i*32..72+i*32].copy_from_slice(&g.members[i].0);b[136+i*32..168+i*32].copy_from_slice(&g.keys[i]);}b
}
// ------------------------=
// FUNC: decode_group
// DESC: Rejects malformed or identity/key-inconsistent persisted group configuration.
// ------------------=
fn decode_group(b:&[u8])->Result<Group,Error> {
    let g=Group {epoch:u64::from_le_bytes(b[..8].try_into().unwrap()),owner:NodeId(b[8..40].try_into().unwrap()),
        members:core::array::from_fn(|i|NodeId(b[40+i*32..72+i*32].try_into().unwrap())),
        keys:core::array::from_fn(|i|b[136+i*32..168+i*32].try_into().unwrap())};
    g.validate()?;if group_bytes(&g)!=b {return Err(Error::Invalid);}Ok(g)
}
// ------------------------=
// FUNC: signed_bytes
// DESC: Encodes the immutable canonical record and exact owner signature.
// ------------------=
fn signed_bytes(v:SignedRecord)->[u8;320] {let mut b=[0;320];b[..256].copy_from_slice(&v.record.encode());b[256..].copy_from_slice(&v.signature);b}
// ------------------------=
// FUNC: decode_signed
// DESC: Verifies canonical metadata bytes and the configured owner's signature on recovery.
// ------------------=
fn decode_signed(b:&[u8],g:&Group)->Result<SignedRecord,Error> {
    let v=SignedRecord {record:Record::decode(&b[..256])?,signature:b[256..320].try_into().unwrap()};v.validate(g)?;Ok(v)
}
// ------------------------=
// FUNC: replica_bytes
// DESC: Encodes bounded staged and committed records with explicit presence bits and reserved zeros.
// ------------------=
fn replica_bytes(r:Replica)->[u8;880] {
    let mut b=[0;880];
    if let Some(s)=r.staged {b[0]=1;b[16..336].copy_from_slice(&signed_bytes(s));}
    if let Some(c)=r.committed {b[1]=1;b[336..656].copy_from_slice(&signed_bytes(c.value));
        for i in 0..2 {let at=656+i*112;b[at..at+48].copy_from_slice(&c.prepared[i].transcript());b[at+48..at+112].copy_from_slice(&c.prepared[i].signature);}}
    b
}
// ------------------------=
// FUNC: decode_replica
// DESC: Recovers only canonical owner-signed state and a valid distinct-member quorum certificate; staged and committed identities cannot disagree.
// ------------------=
fn decode_replica(b:&[u8],g:&Group)->Result<Replica,Error> {
    if b[0]>1 || b[1]>1 {return Err(Error::Invalid);}
    let staged=if b[0]==1 {Some(decode_signed(&b[16..336],g)?)}else{None};
    let committed=if b[1]==1 {
        let value=decode_signed(&b[336..656],g)?;
        let mut prepared=[Receipt {member:0,digest:[0;32],published:false,signature:[0;64]};2];
        for (i,r) in prepared.iter_mut().enumerate() {
            let at=656+i*112;
            *r=Receipt {member:b[at+8],published:b[at+9]!=0,digest:b[at+16..at+48].try_into().unwrap(),signature:b[at+48..at+112].try_into().unwrap()};
            if r.transcript()!=b[at..at+48] {return Err(Error::Invalid);}
        }
        let c=Certificate {value,prepared};c.validate(g)?;Some(c)
    }else{None};
    if let (Some(s),Some(c))=(staged,committed) {
        if s.record.object!=c.value.record.object || s.record.generation<=c.value.record.generation
            || s.record.version<c.value.record.version || s.record.revocation<c.value.record.revocation || c.value.record.deleted {return Err(Error::Conflict);}
    }
    let r=Replica {staged,committed};if replica_bytes(r)!=b {return Err(Error::Invalid);}Ok(r)
}
// ------------------------=
// FUNC: read_catalog
// DESC: Reads native metadata without creating anything; absent backing is an empty replica set, corruption is never reset.
// ------------------=
fn read_catalog<D:BlockDevice>(store:&mut ObjectStore<D>)->Result<([u8;BYTES],Option<ObjectId>),Error> {
    let mut bytes=[0;BYTES];bytes[..8].copy_from_slice(b"INFQDB01");
    let id=match store.reserved_system_metadata_id(RESERVED,PATH).map_err(|_|Error::Persistence)? {Some(id)=>id,None=>return Ok((bytes,None))};
    if store.read(id,None,&mut bytes).map_err(|_|Error::Persistence)?!=BYTES || &bytes[..8]!=b"INFQDB01" || bytes[8..32]!=[0;24] {return Err(Error::Invalid);}
    let mut seen=[[0;16];CAPACITY];
    for i in 0..CAPACITY {
        let at=32+i*ENTRY;let entry=&bytes[at..at+ENTRY];if entry.iter().all(|b|*b==0) {continue;}
        let g=decode_group(&entry[..240])?;let r=decode_replica(&entry[240..],&g)?;
        let object=r.staged.or(r.committed.map(|c|c.value)).ok_or(Error::Invalid)?.record.object;
        if seen[..i].contains(&object) {return Err(Error::Conflict);}seen[i]=object;
    }
    Ok((bytes,Some(id)))
}
// ------------------------=
// FUNC: locate
// DESC: Finds exact object identity and rejects a different configured group instead of silently replacing authority.
// ------------------=
fn locate(bytes:&[u8;BYTES],g:&Group,object:[u8;16])->Result<Option<usize>,Error> {
    for i in 0..CAPACITY {let at=32+i*ENTRY;let entry=&bytes[at..at+ENTRY];if entry.iter().all(|b|*b==0){continue;}
        let stored=decode_group(&entry[..240])?;let r=decode_replica(&entry[240..],&stored)?;
        if r.staged.or(r.committed.map(|c|c.value)).unwrap().record.object==object {
            if group_bytes(g)!=group_bytes(&stored) {return Err(Error::Denied);}return Ok(Some(i));
        }}Ok(None)
}
// ------------------------=
// FUNC: load
// DESC: Supplies a read-only recovered replica to the explicit metadata state machine; no freshness or application-access claim is made.
// ------------------=
pub(crate) fn load<D:BlockDevice>(store:&mut ObjectStore<D>,g:&Group,object:[u8;16])->Result<Replica,Error> {
    g.validate()?;if object==[0;16] {return Err(Error::Invalid);}
    let (bytes,_)=read_catalog(store)?;
    match locate(&bytes,g,object)? {Some(i)=>decode_replica(&bytes[32+i*ENTRY+240..32+(i+1)*ENTRY],g),None=>Ok(Replica::default())}
}
// ------------------------=
// FUNC: persist
// DESC: Implements an atomic state-machine persistence callback with exact CAS, idempotent retries and eight-object capacity; success precedes any receipt issuance.
// ------------------=
pub(crate) fn persist<D:BlockDevice>(store:&mut ObjectStore<D>,g:&Group,object:[u8;16],expected:Replica,next:Replica)->Result<(),Error> {
    g.validate()?;if object==[0;16] {return Err(Error::Invalid);}
    let encoded=replica_bytes(next);let validated=decode_replica(&encoded,g)?;
    let candidate=validated.staged.or(validated.committed.map(|c|c.value)).ok_or(Error::Invalid)?;
    if candidate.record.object!=object {return Err(Error::Denied);}
    let (mut bytes,id)=read_catalog(store)?;let slot=locate(&bytes,g,object)?;
    let current=match slot {Some(i)=>decode_replica(&bytes[32+i*ENTRY+240..32+(i+1)*ENTRY],g)?,None=>Replica::default()};
    if replica_bytes(current)==encoded {return Ok(());}
    if replica_bytes(current)!=replica_bytes(expected) {return Err(Error::Stale);}
    let mut checked=current;
    if current.committed==next.committed {checked.prepare(g,next.staged.ok_or(Error::Invalid)?,|_|Ok(()))?;}
    else {checked.publish(g,next.committed.ok_or(Error::Invalid)?,|_|Ok(()))?;}
    if replica_bytes(checked)!=encoded {return Err(Error::Conflict);}
    let slot=slot.or_else(||(0..CAPACITY).find(|i|bytes[32+i*ENTRY..32+(i+1)*ENTRY].iter().all(|b|*b==0))).ok_or(Error::ResourceLimit)?;
    let at=32+slot*ENTRY;bytes[at..at+240].copy_from_slice(&group_bytes(g));bytes[at+240..at+ENTRY].copy_from_slice(&encoded);
    match id {Some(id)=>{store.replace_state(id,&bytes).map_err(|_|Error::Persistence)?;},None=>{
        store.replace_reserved_state(RESERVED,PATH,&bytes).map_err(|_|Error::Persistence)?;
    }}Ok(())
}

#[cfg(test)]
#[path="../../tools/pool_metadata_native_tests.rs"]
mod tests;
