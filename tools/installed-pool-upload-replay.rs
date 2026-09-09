//! Explicit read-only host artifact replay with all transaction writes confined to RAM.
use super::*;
struct Overlay{base:FileDisk, sectors:std::collections::BTreeMap<u64,[u8;512]>}
impl BlockDevice for Overlay{
    // ------------------------=
    // FUNC: block_count
    // DESC: Preserves real installed disk geometry.
    // ------------------=
    fn block_count(&self)->u64{self.base.block_count()}
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads replay writes before immutable installed disk bytes.
    // ------------------=
    fn read_sector(&mut self,l:u64,b:&mut[u8;512])->bool{if let Some(s)=self.sectors.get(&l){*b=*s;true}else{self.base.read_sector(l,b)}}
    // ------------------------=
    // FUNC: write_sector
    // DESC: Keeps all replay mutations solely in memory, never in the opened read-only artifact.
    // ------------------=
    fn write_sector(&mut self,l:u64,b:&[u8;512])->bool{self.sectors.insert(l,*b);true}
    // ------------------------=
    // FUNC: flush
    // DESC: Completes only the volatile replay transaction.
    // ------------------=
    fn flush(&mut self)->bool{true}
}
// ------------------------=
// FUNC: run
// DESC: Replays an explicitly supplied installed upload through the native service without modifying its raw disk.
// ------------------=
pub fn run(path:String)->Result<String,String>{
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(move||replay(path)).map_err(|_|"replay_thread")?.join().map_err(|_|"native_replay_failed")?;
    Ok("{\"replayed\":true,\"artifact_read_only\":true,\"writes_in_memory_only\":true,\"upload_completed\":true}".into())
}
// ------------------------=
// FUNC: replay
// DESC: Reports the exact native commit outcome and validates canonical successful response encoding.
// ------------------=
fn replay(path:String){
    use runtime::{node::types::NodeId,fabric::resources::ResourceId,iop::{remote::AuthenticatedStorageRequest,storage_protocol::{Operation,StorageOperationV1}}};
    let file=File::open(path).unwrap();let sectors=file.metadata().unwrap().len()/512;
    let mut disk=Overlay{base:FileDisk{file,sectors},sectors:Default::default()};let start=container(&mut disk).unwrap();let mut s=ObjectStore::mount(disk,start).unwrap();
    let id=s.resolve(b"/system/storage/pool-upload").unwrap();let mut bytes=[0;432];assert_eq!(s.read(id,None,&mut bytes).unwrap(),432);
    eprintln!("upload complete={} length={} offset={} namespace_count={} generation={}",bytes[392],u32::from_le_bytes(bytes[56..60].try_into().unwrap()),u32::from_le_bytes(bytes[60..64].try_into().unwrap()),(0..32).filter(|i|s.namespace_entry(*i).is_some()).count(),s.generation());
    eprintln!("audit={:?}",s.resolve(b"/system/storage/pool-audit"));
    for i in 0..32{if let Some((path,_))=s.namespace_entry(i){eprintln!("native namespace={}",String::from_utf8_lossy(path));}}
    let owner=NodeId(bytes[8..40].try_into().unwrap());let scope=number(&bytes,40);
    let mut svc=native_fabric::service::ReplicaService::new(ResourceId([4;16]),1);svc.attach_device_identity(Some([6;16]));
    let p=StorageOperationV1{operation:Operation::PoolUploadCommit,object:id.0,authority_generation:1,manifest_generation:0,object_version:0,offset:0,scope,value:0,length:0,data:[0;64]};
    let r=AuthenticatedStorageRequest{local:owner,peer:owner,session_reference:[0;16],grant:1,request_id:1,correlation:1,causation:1,payload:p};
    for step in 0..260{let result=svc.execute(&mut s,r);eprintln!("commit step={step} result={result:?}");let reply=result.unwrap();assert!(reply.encode().is_ok());if reply.value==1{return}}
    panic!("bounded verification did not complete");
}
