//! Read-only recipient inventory. Physical identities must be captured before retirement.
use super::*;
use storage::object::{ObjectId,ObjectError};
use runtime::fabric::{replica::{Checkpoint,ReplicaDescriptor,ReplicaState,Transfer},resources::ResourceId};
pub struct Entry {pub version:u64,pub manifest:u64,pub backing:[u8;16],pub extent:Option<[u8;16]>}
pub struct Report {pub catalog_present:bool,pub entries:Vec<Entry>,pub physical:Vec<([u8;16],bool)>}
// ------------------------=
// FUNC: identity
// DESC: Encodes a fixed identity for structured observation.
// ------------------=
fn identity(b:&[u8])->String{b.iter().map(|v|format!("{v:02x}")).collect()}
// ------------------------=
// FUNC: inspect
// DESC: Validates every bounded native recipient binding and inventories matching ownership without initializing state.
// ------------------=
pub fn inspect<D:BlockDevice>(s:&mut ObjectStore<D>,owner:[u8;32],object:[u8;16],ids:&[[u8;16]])->Result<Report,String>{
    if owner==[0;32]||object==[0;16]||ids.len()>64||ids.contains(&[0;16]){return Err("identity_invalid".into())}
    let mut report=Report{catalog_present:false,entries:vec![],physical:ids.iter().map(|id|(*id,s.object_exists(ObjectId(*id)))).collect()};
    let id=match s.resolve(b"/system/storage/replicas"){Ok(id)=>id,Err(ObjectError::NotFound|ObjectError::NamespaceNotFound)=>return Ok(report),Err(_)=>return Err("catalog_namespace".into())};
    let mut raw=[0;1696];let n=s.read(id,None,&mut raw).map_err(|_|"catalog_read")?;
    let slots=match(n,&raw[..8]){(864,b"INFREP01")=>4,(1696,b"INFREP02")=>8,_=>return Err("catalog_format".into())};
    if raw[16..32]!=[0;16]{return Err("catalog_reserved".into())}report.catalog_present=true;
    let mut known:Vec<(ReplicaDescriptor,[u8;16],[u8;32],u64,u64,u64)>=vec![];
    for row in raw[32..32+slots*208].chunks_exact(208){
        if row.iter().all(|b|*b==0){continue}
        if &row[..8]!=b"INFREPL1"||row[8]!=1||row[9..16]!=[0;7]||row[112..128]!=[0;16]||row[201..]!=[0;7]{return Err("binding_format".into())}
        let d=ReplicaDescriptor{job:number(row,16),object:row[24..40].try_into().unwrap(),version:number(row,40),resource:ResourceId(row[48..64].try_into().unwrap()),generation:number(row,64),bytes:number(row,72),hash:row[80..112].try_into().unwrap()};
        Transfer::resume(Checkpoint{descriptor:d,copied:0,state:ReplicaState::Planned}).map_err(|_|"binding_descriptor")?;
        let backing:[u8;16]=row[128..144].try_into().unwrap();let issuer:[u8;32]=row[144..176].try_into().unwrap();
        let authority=number(row,176);let manifest=number(row,184);let scope=number(row,192);
        if issuer==[0;32]||authority==0||manifest==0||row[200]!=u8::from(backing==[0;16]){return Err("binding_authority".into())}
        if known.iter().any(|(old,b,o,a,m,sc)|(*b==backing&&backing!=[0;16])||(old.object==d.object&&(*o!=issuer||*a!=authority||*sc!=scope||old.version==d.version||*m==manifest||(old.version<d.version)!=(*m<manifest)))){return Err("binding_conflict".into())}
        known.push((d,backing,issuer,authority,manifest,scope));
        if issuer!=owner||d.object!=object{continue}
        let extent=if backing==[0;16]{None}else{
            let mut record=[0;160];if s.read(ObjectId(backing),None,&mut record).map_err(|_|"checkpoint_read")?!=160||&record[144..152]!=b"EXTENT01"||record[152..]!=[0;8]{return Err("checkpoint_format".into())}
            let e:[u8;16]=record[128..144].try_into().unwrap();
            if e==[0;16]||e==backing||!s.object_exists(ObjectId(e)){return Err("extent_missing".into())}
            // Reuse native recovery validation rather than trusting an independently decoded checkpoint.
            let replica=native_fabric::extent::NativeExtentReplica::open(s,ObjectId(backing),d.resource,d.generation).map_err(|_|"checkpoint_invalid")?;
            if replica.inspect().is_none_or(|cp|cp.descriptor!=d){return Err("checkpoint_identity".into())}Some(e)
        };
        report.entries.push(Entry{version:d.version,manifest,backing,extent});
    }Ok(report)
}
impl Report{
    // ------------------------=
    // FUNC: json
    // DESC: Reports explicit retirement markers and requested physical identity presence; never equates missing catalog with proof of deletion.
    // ------------------=
    pub fn json(&self)->String{
        let entries=self.entries.iter().map(|e|format!("{{\"version\":{},\"manifest_generation\":{},\"retired\":{},\"backing\":\"{}\",\"extent\":{}}}",e.version,e.manifest,e.backing==[0;16],identity(&e.backing),e.extent.map_or("null".into(),|id|format!("\"{}\"",identity(&id))))).collect::<Vec<_>>().join(",");
        let physical=self.physical.iter().map(|(id,p)|format!("{{\"id\":\"{}\",\"present\":{p}}}",identity(id))).collect::<Vec<_>>().join(",");
        format!("{{\"inspected\":true,\"read_only\":true,\"catalog_present\":{},\"bindings\":[{entries}],\"physical_objects\":[{physical}]}}",self.catalog_present)
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    use std::{cell::{Cell,RefCell},rc::Rc,collections::BTreeMap};
    #[derive(Clone)]struct Disk{sectors:Rc<RefCell<BTreeMap<u64,[u8;512]>>>,writes:Rc<Cell<bool>>}
    impl BlockDevice for Disk{
        // ------------------------=
        // FUNC: block_count
        // DESC: Supplies bounded sparse native fixture geometry.
        // ------------------=
        fn block_count(&self)->u64{1_000_000}
        // ------------------------=
        // FUNC: read_sector
        // DESC: Reads durable fixture sectors.
        // ------------------=
        fn read_sector(&mut self,l:u64,b:&mut[u8;512])->bool{*b=*self.sectors.borrow().get(&l).unwrap_or(&[0;512]);true}
        // ------------------------=
        // FUNC: write_sector
        // DESC: Permits production setup but rejects any inspection mutation.
        // ------------------=
        fn write_sector(&mut self,l:u64,b:&[u8;512])->bool{assert!(self.writes.get());self.sectors.borrow_mut().insert(l,*b);true}
        // ------------------------=
        // FUNC: flush
        // DESC: Models stable native commits.
        // ------------------=
        fn flush(&mut self)->bool{true}
    }
    // ------------------------=
    // FUNC: recipient_retirement_inventory_survives_reboot
    // DESC: Exercises real two-version native transfer/deletion and independently checks physical retirement, cold recovery, missing catalog and corruption.
    // ------------------=
    #[test]
    fn recipient_retirement_inventory_survives_reboot(){
        std::thread::Builder::new().stack_size(32*1024*1024).spawn(run_retirement).unwrap().join().unwrap();
    }
    // ------------------------=
    // FUNC: run_retirement
    // DESC: Runs bounded large native fixtures outside the default test thread stack.
    // ------------------=
    fn run_retirement(){
        use runtime::{iop::{remote::AuthenticatedStorageRequest,storage_protocol::{Operation,StorageOperationV1}},node::types::NodeId};
        use sha2::{Digest,Sha256};
        let disk=Disk{sectors:Default::default(),writes:Rc::new(Cell::new(true))};
        let mut s=ObjectStore::format(disk.clone(),0,1_000_000,[7;16]).unwrap();
        disk.writes.set(false);assert!(!inspect(&mut s,[19;32],[2;16],&[]).unwrap().catalog_present);disk.writes.set(true);
        let mut svc=native_fabric::service::ReplicaService::new(ResourceId([4;16]),5);
        svc.attach_device_identity(Some([6;16]));
        let mut r=AuthenticatedStorageRequest{local:NodeId([18;32]),peer:NodeId([19;32]),session_reference:[20;16],grant:1,request_id:7,correlation:8,causation:9,payload:StorageOperationV1{operation:Operation::TransferBegin,object:[2;16],authority_generation:1,manifest_generation:2,object_version:1,offset:63,scope:42,value:1,length:56,data:[0;64]}};
        for version in 1..=2{
            let payload=[version as u8;63];let hash:[u8;32]=Sha256::digest(payload).into();
            r.payload.operation=Operation::TransferBegin;r.payload.object_version=version;r.payload.manifest_generation=version+1;r.payload.offset=63;r.payload.length=56;r.payload.data.fill(0);
            r.payload.data[..16].copy_from_slice(&[4;16]);r.payload.data[16..24].copy_from_slice(&5u64.to_le_bytes());r.payload.data[24..56].copy_from_slice(&hash);
            svc.execute(&mut s,r).unwrap();r.payload.operation=Operation::TransferChunk;r.payload.offset=0;r.payload.length=63;r.payload.data[..63].copy_from_slice(&payload);svc.execute(&mut s,r).unwrap();
            r.payload.operation=Operation::TransferCommit;r.payload.length=0;r.payload.data.fill(0);
            for _ in 0..3{svc.execute(&mut s,r).unwrap();}
            assert_eq!(native_fabric::service::verify_persisted_replica(&mut s,[19;32],[2;16],version,hash),Ok(63));
        }
        disk.writes.set(false);let before=inspect(&mut s,[19;32],[2;16],&[]).unwrap();assert_eq!(before.entries.len(),2);
        let ids:Vec<[u8;16]>=before.entries.iter().flat_map(|e|[e.backing,e.extent.unwrap()]).collect();assert_eq!(ids.len(),4);
        assert!(inspect(&mut s,[19;32],[2;16],&ids).unwrap().physical.iter().all(|p|p.1));
        assert!(inspect(&mut s,[21;32],[2;16],&[]).unwrap().entries.is_empty());
        disk.writes.set(true);r.payload.operation=Operation::ReplicaDelete;svc.execute(&mut s,r).unwrap();svc.execute(&mut s,r).unwrap();disk.writes.set(false);
        drop(s);let mut s=ObjectStore::mount(disk.clone(),0).unwrap();let after=inspect(&mut s,[19;32],[2;16],&ids).unwrap();
        assert_eq!(after.entries.len(),2);assert!(after.entries.iter().all(|e|e.backing==[0;16]&&e.extent.is_none()));assert!(after.physical.iter().all(|p|!p.1));
        disk.writes.set(true);s.replace_named_state(b"/system/storage/replicas",&[0;1696]).unwrap();disk.writes.set(false);assert!(inspect(&mut s,[19;32],[2;16],&ids).is_err());
    }
}
