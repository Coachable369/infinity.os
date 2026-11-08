#![allow(dead_code)]
//! Production workplace adapter exercised against real native identities and transactional object storage.
#[path="../kernel/runtime/mod.rs"]mod runtime;
#[path="../kernel/ui/mod.rs"]mod ui;
#[path="../kernel/core/console_enterprise_access.rs"]mod access;
use infinity_enterprise_core::{self as work,Files,Error};
use std::{cell::RefCell,rc::Rc};
mod storage {
    include!("storage.rs");
    use super::{Disk,RefCell};
    thread_local!{pub static STORE:RefCell<Option<object::ObjectStore<Disk>>>=const{RefCell::new(None)};}
    // ------------------------=
    // FUNC: with_store
    // DESC: Serializes the same native ObjectStore operations used by the production workplace adapter.
    // ------------------=
    pub fn with_store<T>(f:impl FnOnce(&mut object::ObjectStore<Disk>)->Result<T,object::ObjectError>)->Result<T,object::ObjectError>{STORE.with(|s|f(s.borrow_mut().as_mut().unwrap()))}
    // ------------------------=
    // FUNC: object_inspect_path
    // DESC: Resolves authoritative owner and content metadata without any test-side authorization shortcut.
    // ------------------=
    pub fn object_inspect_path(path:&[u8])->Result<(object::ObjectMetadata,usize),object::ObjectError>{with_store(|s|{let id=s.resolve(path)?;Ok((s.metadata(id)?,0))})}
    // ------------------------=
    // FUNC: namespace_resolve
    // DESC: Uses the native namespace to detect absent names and collisions.
    // ------------------=
    pub fn namespace_resolve(path:&[u8])->Result<object::ObjectId,object::ObjectError>{with_store(|s|s.resolve(path))}
    // ------------------------=
    // FUNC: object_read_path
    // DESC: Reads real native content versions after production adapter authorization.
    // ------------------=
    pub fn object_read_path(path:&[u8],version:Option<u32>,out:&mut[u8])->Result<(object::ObjectId,usize),object::ObjectError>{with_store(|s|{let id=s.resolve(path)?;Ok((id,s.read(id,version,out)?))})}
    // ------------------------=
    // FUNC: object_write_path
    // DESC: Commits native versions so denied operations can be checked against unchanged generation and bytes.
    // ------------------=
    pub fn object_write_path(path:&[u8],bytes:&[u8])->Result<u32,object::ObjectError>{with_store(|s|{let id=s.resolve(path)?;s.write(id,bytes)})}
    // ------------------------=
    // FUNC: object_copy_path
    // DESC: Creates independent backups through the actual native transaction implementation.
    // ------------------=
    pub fn object_copy_path(a:&[u8],b:&[u8])->Result<object::ObjectId,object::ObjectError>{with_store(|s|s.copy_path_attached(a,b))}
    // ------------------------=
    // FUNC: object_create_owned_note_at
    // DESC: Creates real owner-tagged Personal content using the same production storage operation.
    // ------------------=
    pub fn object_create_owned_note_at(name:&[u8],bytes:&[u8],path:&[u8],owner:object::ObjectId)->Result<object::ObjectId,object::ObjectError>{with_store(|s|s.create_owned_attached(name,object::ObjectType::Text,object::Space::Personal,bytes,path,owner))}
}
#[derive(Clone)]struct Disk(Rc<RefCell<Vec<[u8;512]>>>);
impl storage::BlockDevice for Disk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Exposes the actual sector count of the native test disk.
    // ------------------=
    fn block_count(&self)->u64{self.0.borrow().len()as u64}
    // ------------------------=
    // FUNC: read_sector
    // DESC: Returns exactly one existing sector across native mounts.
    // ------------------=
    fn read_sector(&mut self,lba:u64,out:&mut[u8;512])->bool{if let Some(bytes)=self.0.borrow().get(lba as usize){*out=*bytes;true}else{false}}
    // ------------------------=
    // FUNC: write_sector
    // DESC: Persists one native transaction sector without allowing writes outside the disk.
    // ------------------=
    fn write_sector(&mut self,lba:u64,bytes:&[u8;512])->bool{if let Some(out)=self.0.borrow_mut().get_mut(lba as usize){*out=*bytes;true}else{false}}
    // ------------------------=
    // FUNC: flush
    // DESC: Makes completed in-memory writes available to the independent remount.
    // ------------------=
    fn flush(&mut self)->bool{true}
}
// ------------------------=
// FUNC: output_text
// DESC: Supplies an inert diagnostic sink while tests assert typed state and object bytes.
// ------------------=
fn output_text(_:&[u8]){}
// ------------------------=
// FUNC: main
// DESC: Verifies local and SSH owner parity, foreign access refusal, retained clipboard on denial, credential revocation and durable authorized writes.
// ------------------=
fn main(){
    use runtime::identity::*;use storage::object::*;use access::NativeFiles;
    runtime::initialize(false);
    let (admin,local,member,ordinary,remote,key)=runtime::with_runtime(|r|{
        r.identity.begin_onboarding().unwrap();r.identity.create_machine(b"Workplace",0xaa64,1,1).unwrap();
        let admin=r.identity.create_user(b"admin",b"Administrator",2).unwrap();r.identity.create_password(admin.id,b"admin password",3).unwrap();r.identity.complete_onboarding().unwrap();
        let local=r.identity.create_session(admin.id,b"admin password",4).unwrap();
        let member=match r.identity.manage_accounts(local.id,AccountRequest::CreateUser{handle:b"member",display_name:b"Member",password:b"member password",now:5}).unwrap(){AccountResult::User(user)=>user,_=>panic!()};
        let ordinary=r.identity.create_session(member.id,b"member password",6).unwrap();let key=ed25519_dalek::SigningKey::from_bytes(&[7;32]).verifying_key().to_bytes();
        r.identity.manage_ssh_key_durable(local.id,member.id,key,false,7,|_|Ok(())).unwrap();let remote=r.identity.create_ssh_session(member.id,key,8).unwrap();(admin,local,member,ordinary,remote,key)
    }).unwrap();
    let sectors=STORE_RELATIVE_LBA as usize+32768;let disk=Disk(Rc::new(RefCell::new(vec![[0;512];sectors])));
    let mut store=ObjectStore::format(disk.clone(),0,sectors as u64,[0x74;16]).unwrap();
    for (path,owner,kind,content) in [(b"/worka".as_slice(),admin.id.0,ObjectType::NamespaceNode,b"".as_slice()),(b"/workb",member.id.0,ObjectType::NamespaceNode,b""),(b"/shared-work",[0;16],ObjectType::NamespaceNode,b""),(b"/worka/private.txt",admin.id.0,ObjectType::Text,b"admin secret"),(b"/workb/own.txt",member.id.0,ObjectType::Text,b"member data"),(b"/shared-work/shared.txt",[0;16],ObjectType::Text,b"shared")]{store.create_owned_attached(b"fixture",kind,Space::Personal,content,path,ObjectId(owner)).unwrap();}
    storage::STORE.with(|s|*s.borrow_mut()=Some(store));
    let mut bytes=[0;128];
    for session in [ordinary.id,remote.id]{
        let mut files=NativeFiles{owner:member.id,session};assert!(files.authorized(true));
        let generation=storage::with_store(|s|Ok(s.generation())).unwrap();
        assert_eq!(files.read(b"/worka/private.txt",&mut bytes),Err(Error::Denied));
        assert_eq!(files.write(b"/worka/private.txt",b"attack"),Err(Error::Denied));
        assert_eq!(files.copy(b"/worka/private.txt",b"/workb/stolen.txt"),Err(Error::Denied));
        assert_eq!(files.copy(b"/workb/own.txt",b"/worka/injected.txt"),Err(Error::Denied));
        assert_eq!(files.create(b"/worka/report.json",b"{}"),Err(Error::Denied));
        assert_eq!(work::restore(&mut files,b"/workb/own.txt",b"/worka/private.txt",work::checksum(b"member data")),Err(Error::Denied));
        assert_eq!(storage::with_store(|s|Ok(s.generation())).unwrap(),generation);
        assert_eq!(files.read(b"/workb/own.txt",&mut bytes),Ok(11));assert_eq!(&bytes[..11],b"member data");
        assert_eq!(files.read(b"/shared-work/shared.txt",&mut bytes),Ok(6));
    }
    let mut files=NativeFiles{owner:member.id,session:remote.id};
    let digest=work::backup(&mut files,b"/workb/own.txt",b"/workb/backup.txt").unwrap();
    assert_eq!(storage::object_inspect_path(b"/workb/backup.txt").unwrap().0.owner,ObjectId(member.id.0));
    files.write(b"/workb/own.txt",b"changed").unwrap();work::restore(&mut files,b"/workb/backup.txt",b"/workb/own.txt",digest).unwrap();
    files.create(b"/workb/report.json",b"{}").unwrap();assert_eq!(storage::object_inspect_path(b"/workb/report.json").unwrap().0.owner,ObjectId(member.id.0));
    assert_eq!(files.create(b"/workb/report.json",b"replace"),Err(Error::Conflict));
    let mut admin_files=NativeFiles{owner:admin.id,session:local.id};assert_eq!(admin_files.read(b"/workb/own.txt",&mut bytes),Err(Error::Denied));
    let mut clipboard=ui::clipboard::SessionClipboard::new();clipboard.session(local.id.0,true);clipboard.write_public_text(local.id.0,b"retained local text",10).unwrap();
    let history=clipboard.history_previews(local.id.0,b"",11);clipboard.grant(clipboard.epoch(local.id.0),false,11);
    assert_eq!(access::clear_clipboard(&mut clipboard,remote.id.0),Err(Error::Denied));
    assert_eq!(clipboard.history_previews(local.id.0,b"",12),history);assert_eq!(clipboard.browser_read(&mut bytes,12),Ok(19));
    assert_eq!(access::clear_clipboard(&mut clipboard,local.id.0),Ok(()));assert_eq!(clipboard.read(local.id.0,ui::clipboard::ClipboardKind::Utf8Text,&mut bytes,13),Err(ui::clipboard::ClipboardError::Empty));
    assert_eq!(clipboard.history_previews(local.id.0,b"",13),[ui::clipboard::HistoryPreview::EMPTY;ui::clipboard::HISTORY_CAPACITY]);
    runtime::with_runtime(|r|r.identity.manage_ssh_key_durable(local.id,member.id,key,true,9,|_|Ok(())).unwrap());
    assert_eq!(files.read(b"/workb/own.txt",&mut bytes),Err(Error::Denied));assert_eq!(files.write(b"/workb/own.txt",b"revoked"),Err(Error::Denied));
    let mut wrong=NativeFiles{owner:member.id,session:local.id};assert_eq!(wrong.read(b"/workb/own.txt",&mut bytes),Err(Error::Denied));
    storage::STORE.with(|s|{s.borrow_mut().take();});let mut mounted=ObjectStore::mount(disk,0).unwrap();
    let id=mounted.resolve(b"/worka/private.txt").unwrap();let n=mounted.read(id,None,&mut bytes).unwrap();assert_eq!(&bytes[..n],b"admin secret");
    let id=mounted.resolve(b"/workb/own.txt").unwrap();let n=mounted.read(id,None,&mut bytes).unwrap();assert_eq!(&bytes[..n],b"member data");
}
