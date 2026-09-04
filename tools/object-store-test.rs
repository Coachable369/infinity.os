mod storage;

use std::{cell::{Cell,RefCell},rc::Rc};
use storage::{BlockDevice,DateTimeConfiguration,object::{ContentType,ObjectCapabilityPolicy,ObjectCreateRequest,ObjectError,ObjectMetadataUpdateRequest,ObjectOperation,ObjectQueryRequest,ObjectRef,ObjectService,
    ObjectStore,ObjectType,RelationshipAttachRequest,RelationshipDetachRequest,RelationshipType,Space,
    BANK_A,BANK_B,FORMAT_VERSION,ROOT_A,ROOT_B,STORE_RELATIVE_LBA,crc32}};

#[derive(Clone)] struct MemoryDisk(Rc<RefCell<Vec<[u8;512]>>>);
impl MemoryDisk {
    // ------------------------=
    // FUNC: new
    // DESC: Creates and initializes a new instance.
    // ------------------=
    fn new(sectors:usize)->Self{Self(Rc::new(RefCell::new(vec![[0;512];sectors])))}
    // ------------------------=
    // FUNC: flip
    // DESC: Implements the flip operation.
    // ------------------=
    fn flip(&self,lba:usize,offset:usize){self.0.borrow_mut()[lba][offset]^=0x5a;}
    // ------------------------=
    // FUNC: set_version_and_rechecksum
    // DESC: Writes or updates set version and rechecksum data.
    // ------------------=
    fn set_version_and_rechecksum(&self,lba:usize,version:u32){let mut disk=self.0.borrow_mut();let s=&mut disk[lba];
        s[8..12].copy_from_slice(&version.to_le_bytes());s[508..512].fill(0);let sum=crc32(s);s[508..512].copy_from_slice(&sum.to_le_bytes());}
}
impl BlockDevice for MemoryDisk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Implements the block count operation.
    // ------------------=
    fn block_count(&self)->u64{self.0.borrow().len() as u64}
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads read sector data.
    // ------------------=
    fn read_sector(&mut self,lba:u64,out:&mut[u8;512])->bool{let disk=self.0.borrow();let Some(s)=disk.get(lba as usize)else{return false};*out=*s;true}
    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes or updates write sector data.
    // ------------------=
    fn write_sector(&mut self,lba:u64,input:&[u8;512])->bool{let mut disk=self.0.borrow_mut();let Some(s)=disk.get_mut(lba as usize)else{return false};*s=*input;true}
    // ------------------------=
    // FUNC: flush
    // DESC: Implements the flush operation.
    // ------------------=
    fn flush(&mut self)->bool{true}
}

#[derive(Clone)] struct FailingDisk { inner:MemoryDisk,remaining:Rc<Cell<Option<usize>>> }
impl FailingDisk { fn new(inner:MemoryDisk)->Self{Self{inner,remaining:Rc::new(Cell::new(None))}}
    // ------------------------=
    // FUNC: arm
    // DESC: Implements the arm operation.
    // ------------------=
    fn arm(&self,writes:usize){self.remaining.set(Some(writes))}fn disarm(&self){self.remaining.set(None)} }
impl BlockDevice for FailingDisk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Implements the block count operation.
    // ------------------=
    fn block_count(&self)->u64{self.inner.block_count()}
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads read sector data.
    // ------------------=
    fn read_sector(&mut self,lba:u64,out:&mut[u8;512])->bool{self.inner.read_sector(lba,out)}
    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes or updates write sector data.
    // ------------------=
    fn write_sector(&mut self,lba:u64,input:&[u8;512])->bool{if let Some(left)=self.remaining.get(){if left==0{return false}
        self.remaining.set(Some(left-1));}self.inner.write_sector(lba,input)}
    // ------------------------=
    // FUNC: flush
    // DESC: Implements the flush operation.
    // ------------------=
    fn flush(&mut self)->bool{true}
}
struct Deny;
impl ObjectCapabilityPolicy for Deny{fn authorize(&self,_:ObjectOperation,_:Option<ObjectRef>)->bool{false}}

// ------------------------=
// FUNC: main
// DESC: Runs the program entry point.
// ------------------=
fn main(){
    let disk=MemoryDisk::new(80_000);let seed=[0x41;16];
    let mut store=ObjectStore::format(disk.clone(),0,80_000,seed).expect("format");
    assert!(store.runtime_bootstrap_valid());
    assert!(store.resolve(b"/home/default/documents").is_ok());
    let id=store.create(b"hello",ObjectType::Text,Space::Personal,b"Hello Infinity").expect("create");
    store.attach(b"/home/default/documents/hello",id).expect("attach");
    assert_eq!(store.write(id,b"Version 2").unwrap(),2);assert_eq!(store.write(id,b"Version 3").unwrap(),3);
    store.attach(b"/home/default/archive/hello",id).expect("second reference");
    store.move_entry(b"/home/default/documents/hello",b"/home/default/projects/hello").expect("move");
    assert_eq!(store.resolve(b"/home/default/documents/hello"),Err(ObjectError::NamespaceNotFound));
    assert_eq!(store.resolve(b"/home/default/projects/hello").unwrap(),id);
    assert_eq!(store.resolve(b"/home/default/archive/hello").unwrap(),id);
    assert_eq!(store.namespace_refs(id),2);assert_eq!(store.history_count(id),3);

    let owner=store.create(b"owner",ObjectType::Metadata,Space::System,b"identity").unwrap();
    store.update_metadata(ObjectMetadataUpdateRequest{object:ObjectRef{id},owner,content_type:ContentType::Utf8Text,
        tags:b"welcome",flags:0x21}).unwrap();let metadata=store.metadata(id).unwrap();
    assert_eq!(metadata.owner,owner);assert_eq!(metadata.tags_len,7);assert_eq!(&metadata.tags[..7],b"welcome");assert_eq!(metadata.flags,0x21);
    store.relationship_attach(RelationshipAttachRequest{source:ObjectRef{id},kind:RelationshipType::GeneratedBy,
        target:ObjectRef{id:owner},flags:7}).unwrap();let relation=store.relationship_nth(id,0).unwrap();
    assert_eq!(relation.target.id,owner);assert_eq!(relation.kind,RelationshipType::GeneratedBy);assert_eq!(relation.flags,7);
    let query=ObjectQueryRequest{kind:Some(ObjectType::Text),space:Some(Space::Personal),include_tombstones:false};
    assert_eq!(store.query_nth(query,0).unwrap().object.id,id);

    let large=[0xA5u8;9000];let large_id=store.create(b"large",ObjectType::Blob,Space::Applications,&large).unwrap();
    let mut large_out=[0u8;9000];assert_eq!(store.read(large_id,None,&mut large_out).unwrap(),large.len());assert_eq!(large_out,large);
    assert_eq!(store.usage_by_space(Space::Applications),3);assert!(store.usage_by_space(Space::Personal)>=3);
    assert!(store.usage_by_space(Space::System)>=2);assert_eq!(store.usage_by_space(Space::Recovery),1);

    let used_before=store.usage_blocks();let generation_before=store.generation();
    assert_eq!(store.create_attached(b"rejected",ObjectType::Text,Space::Personal,b"must roll back",b"/home"),Err(ObjectError::NameConflict));
    assert_eq!(store.usage_blocks(),used_before);assert_eq!(store.generation(),generation_before);

    let disposable=store.create_attached(b"disposable",ObjectType::Text,Space::Personal,b"temporary",
        b"/home/default/documents/disposable").unwrap();
    store.attach(b"/home/default/archive/disposable",disposable).unwrap();
    assert_eq!(store.remove_path(b"/home/default/documents/disposable").unwrap(),disposable);
    assert!(store.object_exists(disposable));assert_eq!(store.namespace_refs(disposable),1);
    assert_eq!(store.remove_path(b"/home/default/archive/disposable").unwrap(),disposable);assert!(!store.object_exists(disposable));
    let generation=store.generation();drop(store);

    let mut rebooted=ObjectStore::mount(disk.clone(),0).expect("reboot mount");
    assert_eq!(rebooted.generation(),generation);assert_eq!(rebooted.resolve(b"/home/default/archive/hello").unwrap(),id);
    let mut content=[0u8;4096];let n=rebooted.read(id,None,&mut content).unwrap();assert_eq!(&content[..n],b"Version 3");
    assert_eq!(rebooted.restore(id,1).unwrap(),4);drop(rebooted);
    let mut rebooted=ObjectStore::mount(disk.clone(),0).expect("restore reboot");
    let n=rebooted.read(id,None,&mut content).unwrap();assert_eq!(&content[..n],b"Hello Infinity");assert_eq!(rebooted.history_count(id),4);
    assert_eq!(rebooted.metadata(id).unwrap().owner,owner);assert_eq!(rebooted.relationship_nth(id,0).unwrap().target.id,owner);
    rebooted.relationship_detach(RelationshipDetachRequest{source:ObjectRef{id},kind:RelationshipType::GeneratedBy,target:ObjectRef{id:owner}}).unwrap();

    let garbage=rebooted.create(b"garbage",ObjectType::Blob,Space::Recovery,b"old").unwrap();let before_gc=rebooted.usage_blocks();
    rebooted.remove(garbage).unwrap();assert!(rebooted.query_nth(ObjectQueryRequest{kind:None,space:Some(Space::Recovery),include_tombstones:true},0).is_some());
    let reclaimed=rebooted.collect().unwrap();assert!(reclaimed>=1);assert_eq!(rebooted.usage_blocks()+reclaimed,before_gc);

    let committed=rebooted.generation();let inactive=if committed&1==1{BANK_A}else{BANK_B};disk.flip((STORE_RELATIVE_LBA+inactive+7) as usize,100);
    drop(rebooted);let rebooted=ObjectStore::mount(disk.clone(),0).expect("ignore uncommitted bank");assert_eq!(rebooted.generation(),committed);drop(rebooted);
    disk.flip((STORE_RELATIVE_LBA+inactive+7) as usize,100);

    let newest_root=if committed&1==1{ROOT_B}else{ROOT_A};disk.flip((STORE_RELATIVE_LBA+newest_root) as usize,24);
    let fallback=ObjectStore::mount(disk.clone(),0).expect("root fallback");assert!(fallback.generation()<committed);drop(fallback);
    disk.flip((STORE_RELATIVE_LBA+BANK_A+5) as usize,70);disk.flip((STORE_RELATIVE_LBA+BANK_B+5) as usize,70);
    assert!(matches!(ObjectStore::mount(disk.clone(),0),Err(ObjectError::CorruptMetadata)));

    let content_disk=MemoryDisk::new(80_000);let mut content_store=ObjectStore::format(content_disk.clone(),0,80_000,seed).unwrap();
    let content_id=content_store.create(b"corrupt",ObjectType::Text,Space::Personal,b"verified bytes").unwrap();
    // Eleven bootstrap contents precede this object: kernel, recovery, runtime,
    // service registry, capability policy, local model, AI bootstrap, voice
    // framework, agent policy, organization schema, and identity state.
    content_disk.flip((STORE_RELATIVE_LBA+80+11*8) as usize,0);let mut out=[0u8;4096];
    assert_eq!(content_store.read(content_id,None,&mut out),Err(ObjectError::CorruptContent));

    for (offset,label) in [(1u64,"allocation"),(5,"object"),(15,"namespace"),(23,"relationship")]{
        let corrupt=MemoryDisk::new(80_000);drop(ObjectStore::format(corrupt.clone(),0,80_000,seed).unwrap());
        corrupt.flip((STORE_RELATIVE_LBA+BANK_B+offset) as usize,80);
        assert!(matches!(ObjectStore::mount(corrupt,0),Err(ObjectError::CorruptMetadata)),"{label}");}

    let unsupported=MemoryDisk::new(80_000);drop(ObjectStore::format(unsupported.clone(),0,80_000,seed).unwrap());
    unsupported.set_version_and_rechecksum((STORE_RELATIVE_LBA+ROOT_B) as usize,FORMAT_VERSION+1);
    assert!(matches!(ObjectStore::mount(unsupported,0),Err(ObjectError::UnsupportedFormat)));

    for writes in [8usize,20,33]{let backing=MemoryDisk::new(80_000);let failing=FailingDisk::new(backing.clone());
        let mut crash=ObjectStore::format(failing.clone(),0,80_000,seed).unwrap();let stable=crash.create(b"stable",ObjectType::Text,Space::Personal,b"before").unwrap();
        let generation=crash.generation();failing.arm(writes);assert_eq!(crash.write(stable,b"after"),Err(ObjectError::TransactionFailed));
        failing.disarm();drop(crash);let mut recovered=ObjectStore::mount(backing,0).unwrap();let mut data=[0u8;32];let n=recovered.read(stable,None,&mut data).unwrap();
        assert_eq!(&data[..n],b"before");assert_eq!(recovered.generation(),generation);}
    for writes in [5usize,25]{let backing=MemoryDisk::new(80_000);let failing=FailingDisk::new(backing.clone());
        let mut crash=ObjectStore::format(failing.clone(),0,80_000,seed).unwrap();let stable=crash.create(b"stable",ObjectType::Text,Space::Personal,b"before").unwrap();
        let generation=crash.generation();failing.arm(writes);assert_eq!(crash.attach(b"/home/default/documents/crash",stable),Err(ObjectError::TransactionFailed));
        failing.disarm();drop(crash);let recovered=ObjectStore::mount(backing,0).unwrap();assert_eq!(recovered.generation(),generation);
        assert_eq!(recovered.resolve(b"/home/default/documents/crash"),Err(ObjectError::NamespaceNotFound));}

    let policy_disk=MemoryDisk::new(80_000);let mut policy_store=ObjectStore::format(policy_disk,0,80_000,seed).unwrap();
    let mut denied=ObjectService::new(&mut policy_store,&Deny);assert_eq!(denied.create(ObjectCreateRequest{name:b"blocked",
        kind:ObjectType::Text,space:Space::Personal,content:b"no"}),Err(ObjectError::Unauthorized));

    let time_disk=MemoryDisk::new(80_000);let mut time_store=ObjectStore::format(time_disk.clone(),0,80_000,seed).unwrap();
    let configured_time=DateTimeConfiguration{year:2026,month:9,day:4,hour:14,minute:30,second:0,time_zone_id:3,utc_offset_minutes:-360};
    time_store.install_date_time_configuration(configured_time).expect("install date/time configuration");
    assert_eq!(time_store.date_time_configuration(),Some(configured_time));drop(time_store);
    let mut time_rebooted=ObjectStore::mount(time_disk,0).expect("date/time reboot mount");
    assert_eq!(time_rebooted.date_time_configuration(),Some(configured_time));

    println!("PASS: native IDs, typed metadata/query, persistent date/time settings, persistent relationships, multi-extent COW, per-Space accounting, conservative GC, namespace identity, reboot/restore, five crash boundaries, format rejection, root/allocation/object/namespace/relationship/content corruption detection");
}
