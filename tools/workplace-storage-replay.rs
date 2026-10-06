//! Replays a native copy on a read-only installed artifact; all writes stay in RAM.
#![allow(dead_code)]
mod storage;
use storage::{BlockDevice,object::ObjectStore};
use std::{fs::File,io::{Read,Seek,SeekFrom},collections::BTreeMap,rc::Rc,cell::RefCell};
struct Overlay {file:File,sectors:u64,writes:Rc<RefCell<BTreeMap<u64,[u8;512]>>>}
impl BlockDevice for Overlay {
    // ------------------------=
    // FUNC: block_count
    // DESC: Preserves the installed artifact's actual disk geometry.
    // ------------------=
    fn block_count(&self)->u64 {self.sectors}
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads replayed writes before the immutable installed bytes.
    // ------------------=
    fn read_sector(&mut self,lba:u64,out:&mut [u8;512])->bool {
        if let Some(bytes)=self.writes.borrow().get(&lba) {*out=*bytes;return true;}
        self.file.seek(SeekFrom::Start(lba*512)).is_ok() && self.file.read_exact(out).is_ok()
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Confines every transaction write to volatile memory.
    // ------------------=
    fn write_sector(&mut self,lba:u64,bytes:&[u8;512])->bool {self.writes.borrow_mut().insert(lba,*bytes);true}
    // ------------------------=
    // FUNC: flush
    // DESC: Finishes only the volatile replay transaction.
    // ------------------=
    fn flush(&mut self)->bool {true}
}
// ------------------------=
// FUNC: replay
// DESC: Resolves the real parent/source and exposes native copy errors while verifying successful copies byte-for-byte.
// ------------------=
fn replay(path:String,source:String,destination:String) {
    let file=File::open(path).unwrap();let sectors=file.metadata().unwrap().len()/512;
    let mut disk=Overlay{file,sectors,writes:Rc::new(RefCell::new(BTreeMap::new()))};let mut gpt=[0;512];
    let persisted=Overlay{file:disk.file.try_clone().unwrap(),sectors,writes:disk.writes.clone()};
    assert!(disk.read_sector(2,&mut gpt));let start=u64::from_le_bytes(gpt[160..168].try_into().unwrap());
    let mut store=ObjectStore::mount(disk,start).unwrap();
    let parent=&destination.as_bytes()[..destination.rfind('/').unwrap()];
    let parent=store.resolve(parent).and_then(|id|store.metadata(id)).map(|m|(m.kind as u8,m.space as u8,m.current_version));
    eprintln!("parent={parent:?}; namespace_entries={}",(0..128).filter(|i|store.namespace_entry(*i).is_some()).count());
    let id=store.resolve(source.as_bytes()).unwrap();let mut before=[0;16384];
    let length=store.read(id,None,&mut before).unwrap();
    let copied=store.copy_path_attached(source.as_bytes(),destination.as_bytes());
    eprintln!("copy={copied:?}");let copied=copied.unwrap();assert_ne!(id,copied);
    let mut after=[0;16384];assert_eq!(store.read(copied,None,&mut after).unwrap(),length);
    assert_eq!(before[..length],after[..length]);
    drop(store);let mut store=ObjectStore::mount(persisted,start).unwrap();
    assert_eq!(store.resolve(destination.as_bytes()).unwrap(),copied);
    assert_eq!(store.read(copied,None,&mut after).unwrap(),length);assert_eq!(before[..length],after[..length]);
}
// ------------------------=
// FUNC: main
// DESC: Runs the read-only artifact replay with sufficient stack for native transaction snapshots.
// ------------------=
fn main() {
    let args:Vec<String>=std::env::args().collect();assert_eq!(args.len(),4);
    let (path,source,destination)=(args[1].clone(),args[2].clone(),args[3].clone());
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(move||replay(path,source,destination)).unwrap().join().unwrap();
}
