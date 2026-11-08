#![allow(dead_code)]
mod storage;
#[path="storage-metadata-fixture.rs"]mod metadata_fixture;
use std::{cell::{Cell,RefCell},collections::BTreeMap,rc::Rc};
use storage::{BlockDevice,object::{ObjectStore,ObjectType,ObjectError,Space,ObjectId,STORE_RELATIVE_LBA,BANK_A,BANK_B,crc32}};
#[derive(Clone)]struct Disk{sectors:u64,bytes:Rc<RefCell<BTreeMap<u64,[u8;512]>>>,events:Rc<Cell<usize>>,fail:Rc<Cell<Option<usize>>>}
impl Disk{
    // ------------------------=
    // FUNC: event
    // DESC: Injects failure at an exact write or flush boundary rather than assuming a fixed metadata size.
    // ------------------=
    fn event(&self)->bool{let n=self.events.get();self.events.set(n+1);self.fail.get()!=Some(n)}
    // ------------------------=
    // FUNC: fork
    // DESC: Makes an independent disk image for interruption recovery without touching any guest artifact.
    // ------------------=
    fn fork(&self)->Self{Self{sectors:self.sectors,bytes:Rc::new(RefCell::new(self.bytes.borrow().clone())),events:Rc::new(Cell::new(0)),fail:Rc::new(Cell::new(None))}}
}
impl BlockDevice for Disk{
    // ------------------------=
    // FUNC: block_count
    // DESC: Exposes the sparse native fixture capacity.
    // ------------------=
    fn block_count(&self)->u64{self.sectors}
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads stable fixture bytes with unwritten sectors represented as physical zeroes.
    // ------------------=
    fn read_sector(&mut self,lba:u64,out:&mut[u8;512])->bool{if lba>=self.sectors{return false;}*out=self.bytes.borrow().get(&lba).copied().unwrap_or([0;512]);true}
    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes exactly one fixture sector or fails before changing it at the selected boundary.
    // ------------------=
    fn write_sector(&mut self,lba:u64,bytes:&[u8;512])->bool{if lba>=self.sectors||!self.event(){return false;}self.bytes.borrow_mut().insert(lba,*bytes);true}
    // ------------------------=
    // FUNC: flush
    // DESC: Exercises both pre-root and post-root flush failure semantics.
    // ------------------=
    fn flush(&mut self)->bool{self.event()}
}
// ------------------------=
// FUNC: verify
// DESC: Checks unchanged object identity, content versions and owner-private key bytes after every remount.
// ------------------=
fn verify(store:&mut ObjectStore<Disk>,id:ObjectId,key:ObjectId){
    assert_eq!(store.resolve(b"/preserved"),Ok(id));assert_eq!(store.resolve(b"/home/Owner/.ssh/id_ed25519"),Ok(key));
    let mut bytes=[0;64];let n=store.read(id,None,&mut bytes).unwrap();assert_eq!(&bytes[..n],b"second version");
    let n=store.read(id,Some(1),&mut bytes).unwrap();assert_eq!(&bytes[..n],b"first version");
    let n=store.read_user_ssh_key(key,ObjectId([9;16]),&mut bytes).unwrap();assert_eq!(&bytes[..n],b"fixture-private-key");
    assert_eq!(store.read(key,None,&mut bytes),Err(ObjectError::Unauthorized));
}
// ------------------------=
// FUNC: active_bank
// DESC: Reads the newest fixture root's binary bank pointer for physical allocation assertions.
// ------------------=
fn active_bank(disk:&Disk)->u64{
    let bytes=disk.bytes.borrow();
    let root=(0..2).max_by_key(|slot|u64::from_le_bytes(bytes[&(STORE_RELATIVE_LBA+slot)][16..24].try_into().unwrap())).unwrap();
    u64::from_le_bytes(bytes[&(STORE_RELATIVE_LBA+root)][24..32].try_into().unwrap())
}
// ------------------------=
// FUNC: reserved
// DESC: Checks the committed allocation bitmap for every block of the retired metadata extent.
// ------------------=
fn reserved(disk:&Disk,start:u32,expected:bool){
    let bank=active_bank(disk);let rows=disk.bytes.borrow();
    for block in start..start+20{
        let byte=block as usize/8;assert!(byte/492<4);
        let sector=&rows[&(STORE_RELATIVE_LBA+bank+1+(byte/492)as u64)];
        assert_eq!(sector[16+byte%492]&(1<<(block%8))!=0,expected);
    }
}
// ------------------------=
// FUNC: migration_case
// DESC: Exercises both metadata publications under every write and flush interruption, preserving old sectors until retirement.
// ------------------=
fn migration_case(mut disk:Disk,id:ObjectId,key:ObjectId){
    metadata_fixture::downgrade_v10(&mut disk,STORE_RELATIVE_LBA);
    let bank=active_bank(&disk);
    let old_extent=u32::from_le_bytes(disk.bytes.borrow()[&(STORE_RELATIVE_LBA+bank)][40..44].try_into().unwrap());
    let old_sectors:Vec<_>=(0..160).map(|offset|{let lba=STORE_RELATIVE_LBA+80+u64::from(old_extent)*8+offset;(lba,disk.bytes.borrow()[&lba])}).collect();
    let mut old=ObjectStore::mount(disk.clone(),0).unwrap();verify(&mut old,id,key);let generation=old.generation();let used=old.usage_blocks();drop(old);
    let success=disk.fork();let mut store=ObjectStore::mount(success.clone(),0).unwrap();success.events.set(0);store.attach(b"/migrated",id).unwrap();let events=success.events.get();
    assert_eq!(store.usage_blocks(),used+96);drop(store);reserved(&success,old_extent,true);
    for(lba,bytes)in &old_sectors{assert_eq!(success.bytes.borrow()[lba],*bytes);}
    let migrated=success.fork();
    let mut store=ObjectStore::mount(success.clone(),0).unwrap();verify(&mut store,id,key);success.events.set(0);
    let during=store.create_attached(b"during",ObjectType::Blob,Space::Personal,&[0xd6;4096],b"/during").unwrap();let retire_events=success.events.get();
    assert_eq!(store.usage_blocks(),used+77);drop(store);reserved(&success,old_extent,false);
    for(lba,bytes)in &old_sectors{assert_eq!(success.bytes.borrow()[lba],*bytes);}
    let mut store=ObjectStore::mount(success.clone(),0).unwrap();verify(&mut store,id,key);let mut content=[0;4096];assert_eq!(store.read(during,None,&mut content),Ok(4096));assert_eq!(content,[0xd6;4096]);
    let reused=store.create_attached(b"reused",ObjectType::Blob,Space::Personal,&[0xa5;4096],b"/reused").unwrap();assert_eq!(store.usage_blocks(),used+78);drop(store);
    for offset in 0..8{assert_eq!(success.bytes.borrow()[&(STORE_RELATIVE_LBA+80+u64::from(old_extent)*8+offset)],[0xa5;512]);}
    let mut store=ObjectStore::mount(success,0).unwrap();verify(&mut store,id,key);assert_eq!(store.read(reused,None,&mut content),Ok(4096));assert_eq!(content,[0xa5;4096]);drop(store);
    for phase in 0..2{let(base,event_count,before_generation,before_used,after_used,path)=if phase==0{(&disk,events,generation,used,used+96,b"/migrated".as_slice())}else{(&migrated,retire_events,generation+1,used+96,used+77,b"/during".as_slice())};
        for cut in 0..=event_count{
            let interrupted=base.fork();let mut store=ObjectStore::mount(interrupted.clone(),0).unwrap();interrupted.events.set(0);interrupted.fail.set(Some(cut));
            let result=if phase==0{store.attach(path,id)}else{store.create_attached(b"during",ObjectType::Blob,Space::Personal,&[0xd6;4096],path).map(|_|())};
            interrupted.fail.set(None);
            if result==Err(ObjectError::CommitUncertain){assert_eq!(store.attach(b"/unsafe-retry",id),Err(ObjectError::CommitUncertain));}
            drop(store);let mut mounted=ObjectStore::mount(interrupted.clone(),0).unwrap();verify(&mut mounted,id,key);
            let published=cut>=event_count-1;assert_eq!(mounted.generation(),before_generation+u64::from(published));assert_eq!(mounted.resolve(path).is_ok(),published);
            assert_eq!(mounted.usage_blocks(),if published{after_used}else{before_used});reserved(&interrupted,old_extent,phase==0||!published);
            for(lba,bytes)in &old_sectors{assert_eq!(interrupted.bytes.borrow()[lba],*bytes);}
            if cut<event_count-2{assert_eq!(result,Err(ObjectError::TransactionFailed));}else if cut<event_count{assert_eq!(result,Err(ObjectError::CommitUncertain));}else{assert_eq!(result,Ok(()));}
        }
    }
    let full=disk.fork();
    for bank in [BANK_A,BANK_B]{for sector in 0..4{let mut map=full.bytes.borrow_mut();let bytes=map.get_mut(&(STORE_RELATIVE_LBA+bank+1+sector)).unwrap();bytes[16..508].fill(255);bytes[508..].fill(0);let sum=crc32(bytes);bytes[508..].copy_from_slice(&sum.to_le_bytes());}}
    let before=full.bytes.borrow().clone();let mut store=ObjectStore::mount(full.clone(),0).unwrap();
    assert_eq!(store.attach(b"/no-room",id),Err(ObjectError::InsufficientCapacity));assert_eq!(store.generation(),generation);drop(store);
    assert_eq!(*full.bytes.borrow(),before);let mut store=ObjectStore::mount(full,0).unwrap();verify(&mut store,id,key);
}
// ------------------------=
// FUNC: eight_homes
// DESC: Cold-remounts all supported seeded user homes and private keys with extra native-extension allocation headroom.
// ------------------=
fn eight_homes(){
    let disk=Disk{sectors:STORE_RELATIVE_LBA+32768,bytes:Rc::new(RefCell::new(BTreeMap::new())),events:Rc::new(Cell::new(0)),fail:Rc::new(Cell::new(None))};
    let mut store=ObjectStore::format(disk.clone(),0,disk.sectors,[0x78;16]).unwrap();
    for index in 0..8u8{let name=format!("Owner{index}");let owner=ObjectId([index+1;16]);
        store.checkpoint_identity_enrolled(b"fixture identity",Some((owner,name.as_bytes())),Some((&[index+1;32],&[index+101;32]))).unwrap();
        // Host fixtures omit the native .so; reserve its real object/name slot.
        let path=format!("/native-extension-{index}");store.create_attached(b"extension",ObjectType::Blob,Space::Personal,&[index;4096],path.as_bytes()).unwrap();
    }
    assert!(store.persistent_namespace_count()>128);drop(store);
    let mut store=ObjectStore::mount(disk,0).unwrap();
    for index in 0..8u8{let path=format!("/home/Owner{index}/.ssh/id_ed25519");let key=store.resolve(path.as_bytes()).unwrap();let mut bytes=[0;64];
        assert_eq!(store.read_user_ssh_key(key,ObjectId([index+1;16]),&mut bytes),Ok(32));assert_eq!(&bytes[..32],&[index+1;32]);
        assert_eq!(store.read_user_ssh_key(key,ObjectId([index+20;16]),&mut bytes),Err(ObjectError::Unauthorized));
        assert!(store.resolve(format!("/home/Owner{index}/documents/hello.py").as_bytes()).is_ok());
    }
}
// ------------------------=
// FUNC: run
// DESC: Verifies fresh format11, exact v10 migration, every write/flush interruption, delayed reservation retirement and allocation exhaustion rollback.
// ------------------=
fn run(){
    let disk=Disk{sectors:STORE_RELATIVE_LBA+32768,bytes:Rc::new(RefCell::new(BTreeMap::new())),events:Rc::new(Cell::new(0)),fail:Rc::new(Cell::new(None))};
    assert!(core::mem::size_of::<ObjectStore<Disk>>()<=384*1024);
    let mut store=ObjectStore::format(disk.clone(),0,disk.sectors,[0x77;16]).unwrap();
    let id=store.create_attached(b"preserved",ObjectType::Text,Space::Personal,b"first version",b"/preserved").unwrap();store.write(id,b"second version").unwrap();
    store.checkpoint_identity_enrolled(b"fixture identity",Some((ObjectId([9;16]),b"Owner")),Some((b"fixture-private-key",b"fixture-public-key"))).unwrap();
    let key=store.resolve(b"/home/Owner/.ssh/id_ed25519").unwrap();drop(store);
    for root in 0..2{assert_eq!(u32::from_le_bytes(disk.bytes.borrow()[&(STORE_RELATIVE_LBA+root)][8..12].try_into().unwrap()),11);}
    for alternate in [false,true]{let fixture=disk.fork();if alternate{let mut store=ObjectStore::mount(fixture.clone(),0).unwrap();store.attach(b"/orientation",id).unwrap();}migration_case(fixture,id,key);}
    eight_homes();
}
// ------------------------=
// FUNC: main
// DESC: Accommodates simultaneous host fixture snapshots; native stack safety is verified separately from the linked kernel and installed execution.
// ------------------=
fn main(){std::thread::Builder::new().stack_size(16*1024*1024).spawn(run).unwrap().join().unwrap();}
