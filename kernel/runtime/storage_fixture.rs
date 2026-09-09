//! Explicit operator-launched acceptance producer. It uses public typed upload
//! operations only; it cannot create placements, satisfy policy, or bypass IOP.
use super::*;
use identity::StableId;
use sha2::{Digest,Sha256};
use iop::{IopError, storage_protocol::{Operation,StorageOperationV1}};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Observation {
    pub phase:u8, pub offset:u32, pub length:u32, pub object:[u8;16], pub hash:[u8;32],
    pub version:u64, pub generation:u64, pub error:Option<IopError>,
}
pub struct Producer {
    owner:Option<(StableId,StableId)>, seed:u64, policy:u8, nonce:u64,
    upload:[u8;16], digest:Option<Sha256>, pub observation:Observation,
}
impl Producer {
    // ------------------------=
    // FUNC: new
    // DESC: Starts idle with no generated objects or implicit test activity.
    // ------------------=
    pub const fn new()->Self {Self{owner:None,seed:0,policy:0,nonce:0,upload:[0;16],digest:None,
        observation:Observation{phase:0,offset:0,length:0,object:[0;16],hash:[0;32],version:0,generation:0,error:None}}}
}
// ------------------------=
// FUNC: byte_at
// DESC: Defines public deterministic fixture bytes so the installed acceptance host can independently verify actual stored content.
// ------------------=
pub fn byte_at(seed:u64,offset:u32)->u8 {offset.wrapping_mul(73).wrapping_add((seed as u32).wrapping_mul(19)).wrapping_add(offset>>8) as u8}
// ------------------------=
// FUNC: start
// DESC: Admits an explicitly requested bounded acceptance object for the current privileged operator; no replica or health state is synthesized.
// ------------------=
pub fn start(user:StableId,session:StableId,length:u32,seed:u64,policy:u8)->Result<(),IopError> {
    with_runtime(|r| {
        storage_client::authorize(r,user,session)?;
        if length>262144 || !(1..=3).contains(&policy) || r.storage_fixture.owner.is_some(){return Err(IopError::InvalidPayload);}
        let nonce=r.iop.next_node_request()?;
        let f=&mut r.storage_fixture;*f=Producer::new();f.owner=Some((user,session));f.seed=seed;f.policy=policy;f.nonce=nonce;
        f.digest=Some(Sha256::new());f.observation.phase=1;f.observation.length=length;Ok(())
    }).ok_or(IopError::UnknownEndpoint)?
}
// ------------------------=
// FUNC: poll
// DESC: Produces at most one KiB of hash input or one canonical sixty-byte upload window per runtime iteration and revalidates the owner every time.
// ------------------=
pub(super) fn poll(r:&mut InfinityRuntime,now:u64) {
    let Some((user,session))=r.storage_fixture.owner else{return;};
    if let Err(error)=storage_client::authorize(r,user,session){r.storage_fixture.observation.phase=255;r.storage_fixture.observation.error=Some(error);r.storage_fixture.owner=None;return;}
    let phase=r.storage_fixture.observation.phase;
    if phase==1 {
        let f=&mut r.storage_fixture;let mut bytes=[0;1024];let length=(f.observation.length-f.observation.offset).min(1024) as usize;
        for(i,b)in bytes[..length].iter_mut().enumerate(){*b=byte_at(f.seed,f.observation.offset+i as u32);}
        f.digest.as_mut().unwrap().update(&bytes[..length]);f.observation.offset+=length as u32;
        if f.observation.offset==f.observation.length {f.observation.hash=f.digest.take().unwrap().finalize().into();f.observation.offset=0;f.observation.phase=2;}
        return;
    }
    let f=&r.storage_fixture;
    let mut p=StorageOperationV1{operation:Operation::PoolUploadBegin,object:[0;16],authority_generation:1,
        manifest_generation:0,object_version:0,offset:0,scope:0,value:0,length:0,data:[0;64]};
    match phase {
        2=>{p.length=45;p.data[..4].copy_from_slice(&f.observation.length.to_le_bytes());p.data[4..36].copy_from_slice(&f.observation.hash);
            p.data[36]=f.policy;p.data[37..45].copy_from_slice(&f.nonce.to_le_bytes());},
        3=>{p.operation=Operation::PoolUploadAppend;p.object=f.upload;let length=(f.observation.length-f.observation.offset).min(60) as usize;
            p.length=(length+4) as u16;p.data[..4].copy_from_slice(&f.observation.offset.to_le_bytes());
            for(i,b)in p.data[4..4+length].iter_mut().enumerate(){*b=byte_at(f.seed,f.observation.offset+i as u32);}},
        4=>{p.operation=Operation::PoolUploadCommit;p.object=f.upload;},
        _=>return,
    }
    let result=storage_client::perform(r,now,p);
    let f=&mut r.storage_fixture;
    match result {
        Ok(reply)=>match phase {
            2 if reply.length==16=>{f.upload=reply.data[..16].try_into().unwrap();f.observation.offset=reply.offset as u32;
                f.observation.phase=if f.observation.offset==f.observation.length {4}else{3};},
            3 if reply.offset>f.observation.offset as u64 && reply.offset<=f.observation.length as u64=>{
                f.observation.offset=reply.offset as u32;if f.observation.offset==f.observation.length{f.observation.phase=4;}},
            4 if reply.value==0=>{},
            4 if reply.value==1 && reply.length==48 && reply.data[16..48]==f.observation.hash=>{
                f.observation.object=reply.data[..16].try_into().unwrap();f.observation.version=reply.object_version;
                f.observation.generation=reply.manifest_generation;f.observation.phase=5;f.owner=None;},
            _=>{f.observation.phase=255;f.observation.error=Some(IopError::InvalidPayload);f.owner=None;},
        },
        Err(error)=>{f.observation.phase=255;f.observation.error=Some(error);f.owner=None;},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: installed_and_live_register_identical_native_pool_operations
    // DESC: Exercises both runtime boot profiles and verifies the same typed upload, retirement and inspection contract is installed rather than live-only.
    // ------------------=
    #[test]
    fn installed_and_live_register_identical_native_pool_operations() {
        let mut contracts=[([0u32;24],0u8);2];
        for (index,live) in [false,true].into_iter().enumerate() {
            let mut r=InfinityRuntime::new(live);r.define_bootstrap().unwrap();
            let m=&r.services.inspect(SERVICE_REPLICA_STORAGE).unwrap().manifest;
            contracts[index]=(m.operations,m.operation_count);
            for op in [Operation::PoolUploadBegin,Operation::PoolUploadAppend,Operation::PoolUploadCommit,Operation::PoolUploadAbort,
                Operation::ObjectDelete,Operation::ReplicaDelete,Operation::PoolInspect] {
                assert!(m.operations[..m.operation_count as usize].contains(&(op as u32)));
                assert_eq!(Operation::decode(op as u32),Ok(op));
            }
        }
        assert_eq!(contracts[0],contracts[1]);
        assert_eq!(byte_at(17,0),67);assert_eq!(byte_at(17,256),68);
        let mut denied=InfinityRuntime::new(false);
        denied.storage_fixture.owner=Some((StableId([1;16]),StableId([2;16])));
        poll(&mut denied,1);
        assert_eq!(denied.storage_fixture.observation.error,Some(IopError::AccessDenied));
        assert_eq!(denied.storage_fixture.observation.phase,255);
        assert!(denied.storage_fixture.owner.is_none());
    }
}
