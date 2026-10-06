use super::*;
use crate::clipboard::{SessionClipboard,ClipboardKind as K,ClipboardError as E};
use std::{vec,vec::Vec,rc::Rc,cell::RefCell};

// ------------------------=
// FUNC: clipboard_cross_app_and_atomic_failures
// DESC: Exercises shared UTF-8, complete-buffer admission, typed payload isolation and failed-copy preservation.
// ------------------=
#[test]
fn clipboard_cross_app_and_atomic_failures() {
    let mut c=SessionClipboard::new();let owner=[1;16];let mut out=[0;16384];
    c.session(owner,true);c.write(owner,K::Utf8Text,"caf\u{e9}".as_bytes(),10).unwrap();
    let n=c.read(owner,K::Utf8Text,&mut out,11).unwrap();assert_eq!(&out[..n],"caf\u{e9}".as_bytes());
    assert_eq!(c.write(owner,K::Utf8Text,&[0xff],12),Err(E::InvalidType));
    assert_eq!(c.write(owner,K::Utf8Text,&vec![42;16385],12),Err(E::TooLarge));
    assert_eq!(c.read(owner,K::Utf8Text,&mut out[..2],12),Err(E::TooLarge));
    assert_eq!(c.read(owner,K::ObjectRefs,&mut out,12),Err(E::InvalidType));
    assert_eq!(c.read(owner,K::Utf8Text,&mut out,12).unwrap(),n);
    assert_eq!(c.read([2;16],K::Utf8Text,&mut out,12),Err(E::AccessDenied));
    c.write(owner,K::Utf8Text,b"",13).unwrap();
    assert_eq!(c.read(owner,K::Utf8Text,&mut out,14),Err(E::Empty));
}
// ------------------------=
// FUNC: lock_logout_and_stale_browser_commands
// DESC: Proves lock, logout and new users cannot inherit clipboard contents or queued browser authority.
// ------------------=
#[test]
fn lock_logout_and_stale_browser_commands() {
    let mut c=SessionClipboard::new();let a=[1;16];let mut out=[0;32];c.session(a,true);
    c.write(a,K::Utf8Text,b"secret",1).unwrap();let epoch=c.epoch(a);c.grant(epoch,false,1);
    c.session(a,false);assert_eq!(c.browser_read(&mut out,2),Err(E::AccessDenied));
    c.session(a,true);c.grant(epoch,false,3);assert_eq!(c.browser_read(&mut out,4),Err(E::AccessDenied));
    assert_eq!(c.read(a,K::Utf8Text,&mut out,4),Err(E::Empty));
    c.write(a,K::Utf8Text,b"new",5).unwrap();c.session([0;16],false);c.session([2;16],true);
    assert_eq!(c.read([2;16],K::Utf8Text,&mut out,6),Err(E::Empty));
}
// ------------------------=
// FUNC: expiry_and_browser_policy_are_enforced
// DESC: Exercises expiry boundary, independent permissions, one-shot gestures and expired grants.
// ------------------=
#[test]
fn expiry_and_browser_policy_are_enforced() {
    let mut c=SessionClipboard::new();let a=[1;16];let mut out=[0;16];c.session(a,true);
    assert!(c.configure(a,1,true,true,0));c.write(a,K::Utf8Text,b"text",100).unwrap();
    assert_eq!(c.read(a,K::Utf8Text,&mut out,1099).unwrap(),4);
    assert_eq!(c.read(a,K::Utf8Text,&mut out,1100),Err(E::Empty));
    c.grant(c.epoch(a),true,1100);c.expire(1101);
    c.browser_write(b"after expiry",1101).unwrap();
    assert!(!c.configure(a,86401,true,true,1101));assert_eq!(c.policy(),(1,true,true));
    c.configure(a,0,true,true,1101);c.write(a,K::Utf8Text,b"text",1101).unwrap();let epoch=c.epoch(a);
    assert_eq!(c.browser_read(&mut out,1102),Err(E::AccessDenied));
    c.grant(epoch,false,1102);assert_eq!(c.browser_read(&mut out,1103).unwrap(),4);
    assert_eq!(c.browser_read(&mut out,1104),Err(E::AccessDenied));
    c.grant(epoch,true,1102);c.browser_write(b"browser",1103).unwrap();
    assert_eq!(c.read(a,K::Utf8Text,&mut out,1104).unwrap(),7);
    c.configure(a,0,false,false,1104);c.grant(epoch,false,1105);c.grant(epoch,true,1105);
    assert_eq!(c.browser_read(&mut out,1106),Err(E::AccessDenied));assert_eq!(c.browser_write(b"attack",1106),Err(E::AccessDenied));
    assert_eq!(c.read(a,K::Utf8Text,&mut out,1106).unwrap(),7);
    c.configure(a,0,true,true,1106);c.grant(epoch,false,1107);
    assert_eq!(c.browser_read(&mut out,1108),Err(E::AccessDenied));
    let epoch=c.epoch(a);c.grant(epoch,false,1107);assert_eq!(c.browser_read(&mut out,3107),Err(E::AccessDenied));
    c.grant(epoch,false,3108);c.grant(epoch,true,3108);c.revoke_browser();
    c.grant(epoch,false,3109);c.grant(epoch,true,3109);
    assert_eq!(c.browser_read(&mut out,3109),Err(E::AccessDenied));
    assert_eq!(c.browser_write(b"stale tab",3109),Err(E::AccessDenied));
}
// ------------------------=
// FUNC: paste_never_executes_or_partially_mutates
// DESC: Rejects terminal control bytes, multiline content and overflow while preserving the complete original draft.
// ------------------=
#[test]
fn paste_never_executes_or_partially_mutates() {
    let mut b=[0;16];b[..4].copy_from_slice(b"abcd");let(mut n,mut at)=(4,2);
    for bad in [b"one\ntwo".as_slice(),b"x\r",b"\x1b[2J",b"\0",b"\t",&[b'x';17]] {
        let before=b;assert!(paste_line(&mut b,&mut n,&mut at,bad).is_err());assert_eq!(b,before);assert_eq!((n,at),(4,2));
    }
    paste_line(&mut b,&mut n,&mut at,b"XY").unwrap();assert_eq!(&b[..n],b"abXYcd");assert_eq!(at,4);
}
// ------------------------=
// FUNC: history_recall_search_and_private_mode
// DESC: Verifies bounded history, draft restoration, newest-first search, secret exclusions and complete privacy erasure.
// ------------------=
#[test]
fn history_recall_search_and_private_mode() {
    let mut h=History::new();h.record(b"system status");h.record(b"system info");h.record(b"system info");
    h.record(b"credential create password=never-retain");assert_eq!(h.count,2);
    h.record(b"work history find system");assert_eq!(h.count,2);
    assert_eq!(h.recall(true,b"draft").unwrap().text(),b"system info");
    assert_eq!(h.recall(true,b"").unwrap().text(),b"system status");
    h.recall(false,b"");assert_eq!(h.recall(false,b"").unwrap().text(),b"draft");
    assert_eq!(h.search(b"status",HISTORY).unwrap().1.text(),b"system status");
    assert_eq!(h.reverse_search(b"system").unwrap().text(),b"system info");
    assert_eq!(h.reverse_search(b"system info").unwrap().text(),b"system status");
    assert!(h.reverse_search(b"system status").is_none());
    for i in 0..50 {h.record(&[b'A'+i]);}assert_eq!(h.count,HISTORY);
    h.set_private(true);h.record(b"private command");assert!(h.search(b"",HISTORY).is_none());
    assert!(h.entries.iter().all(|e|e.bytes==[0;INPUT]));assert_eq!(h.draft.bytes,[0;INPUT]);
    h.set_private(false);h.record(b"visible");assert_eq!(h.count,1);h.clear();assert_eq!(h.count,0);
}
// ------------------------=
// FUNC: completion_is_non_destructive
// DESC: Exercises ambiguous prefixes, exact matches and absent candidates without executing any candidate.
// ------------------=
#[test]
fn completion_is_non_destructive() {
    let mut c=Completion::new(b"work c");for name in COMMANDS {c.offer(name);}
    assert_eq!(c.result().unwrap().text(),b"work c");assert!(c.matches>1);
    let mut c=Completion::new(b"work check");for name in COMMANDS {c.offer(name);}
    assert_eq!(c.result().unwrap().text(),b"work checksum ");assert_eq!(c.matches,1);
    let mut c=Completion::new(b"absent");c.offer(b"work");assert!(c.result().is_none());
}
// ------------------------=
// FUNC: quoted_paths_and_invalid_commands
// DESC: Tests quoted paths and escape handling as parsed values, including malformed and excess arguments.
// ------------------=
#[test]
fn quoted_paths_and_invalid_commands() {
    let w=Words::parse(b"work compare '/home/a b' \"/home/c d\"").unwrap();assert_eq!(w.len(),4);
    assert_eq!(w.get(2),Some(b"/home/a b".as_slice()));assert_eq!(w.get(3),Some(b"/home/c d".as_slice()));
    assert!(Words::parse(b"work 'unfinished").is_err());assert!(Words::parse(b"work x\\").is_err());
    assert!(Words::parse(b"1 2 3 4 5 6 7 8 9").is_err());
    assert_eq!(Words::parse(b"a\\ b").unwrap().get(0),Some(b"a b".as_slice()));
}
// ------------------------=
// FUNC: checksum_and_comparison
// DESC: Checks a standard SHA-256 vector and exact equality, unequal-length and first-byte difference outcomes.
// ------------------=
#[test]
fn checksum_and_comparison() {
    assert_eq!(checksum(b"abc"),parse_digest(b"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad").unwrap());
    assert!(parse_digest(b"00").is_err());assert!(parse_digest(&[b'z';64]).is_err());
    assert!(compare(b"same",b"same").same);assert_eq!(compare(b"a",b"ab").first_difference,Some(1));
    assert_eq!(compare(b"ab",b"xb").first_difference,Some(0));
}

#[derive(Clone)]
struct Disk(Rc<RefCell<Vec<[u8;512]>>>);
impl storage::BlockDevice for Disk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Returns the test disk's actual sector capacity.
    // ------------------=
    fn block_count(&self)->u64 {self.0.borrow().len() as u64}
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads exact sectors for the production native object store.
    // ------------------=
    fn read_sector(&mut self,lba:u64,out:&mut [u8;512])->bool {if let Some(s)=self.0.borrow().get(lba as usize) {*out=*s;true}else{false}}
    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes exact sectors for production object-store transaction tests.
    // ------------------=
    fn write_sector(&mut self,lba:u64,input:&[u8;512])->bool {if let Some(s)=self.0.borrow_mut().get_mut(lba as usize) {*s=*input;true}else{false}}
    // ------------------------=
    // FUNC: flush
    // DESC: Commits the in-memory sector snapshot for remount verification.
    // ------------------=
    fn flush(&mut self)->bool {true}
}
struct Native(storage::object::ObjectStore<Disk>);
impl Files for Native {
    // ------------------------=
    // FUNC: read
    // DESC: Tests real native object reads through the workplace file API.
    // ------------------=
    fn read(&mut self,path:&[u8],out:&mut [u8])->Result<usize,Error> {let id=self.0.resolve(path).map_err(|_|Error::Storage)?;self.0.read(id,None,out).map_err(|_|Error::Storage)}
    // ------------------------=
    // FUNC: copy
    // DESC: Exercises the same transactional copy operation used by the live shell.
    // ------------------=
    fn copy(&mut self,a:&[u8],b:&[u8])->Result<(),Error> {self.0.copy_path_attached(a,b).map(|_|()).map_err(|_|Error::Storage)}
    // ------------------------=
    // FUNC: write
    // DESC: Writes a production content version through the workplace restore API.
    // ------------------=
    fn write(&mut self,path:&[u8],bytes:&[u8])->Result<u32,Error> {let id=self.0.resolve(path).map_err(|_|Error::Storage)?;self.0.write(id,bytes).map_err(|_|Error::Storage)}
}
// ------------------------=
// FUNC: backups_restore_and_remount_real_storage
// DESC: Proves independent backups, integrity-gated restore, retained old versions, collision refusal and reboot persistence.
// ------------------=
#[test]
fn backups_restore_and_remount_real_storage() {
    use storage::object::{ObjectStore,ObjectType,Space,STORE_RELATIVE_LBA};
    let count=STORE_RELATIVE_LBA as usize+32768;let disk=Disk(Rc::new(RefCell::new(vec![[0;512];count])));
    let mut f=Native(ObjectStore::format(disk.clone(),0,count as u64,[7;16]).unwrap());
    let source=b"/home/default/documents/source.txt";let backup_path=b"/home/default/documents/backup.txt";
    let id=f.0.create_attached(b"source.txt",ObjectType::Text,Space::Personal,b"original",source).unwrap();
    let hash=backup(&mut f,source,backup_path).unwrap();assert_ne!(id,f.0.resolve(backup_path).unwrap());
    assert!(backup(&mut f,source,backup_path).is_err());f.write(source,b"changed").unwrap();
    let mut bytes=[0;CONTENT];assert_eq!(f.read(backup_path,&mut bytes).unwrap(),8);assert_eq!(&bytes[..8],b"original");
    assert_eq!(restore(&mut f,backup_path,source,[0;32]),Err(Error::Integrity));
    assert_eq!(f.read(source,&mut bytes).unwrap(),7);assert_eq!(&bytes[..7],b"changed");
    let restored=restore(&mut f,backup_path,source,hash).unwrap();assert!(restored>2);
    let n=f.0.read(id,Some(2),&mut bytes).unwrap();assert_eq!(&bytes[..n],b"changed");
    drop(f);let mut f=Native(ObjectStore::mount(disk,0).unwrap());
    let n=f.read(source,&mut bytes).unwrap();assert_eq!(&bytes[..n],b"original");
    let moved=b"/home/default/documents/moved.txt";f.0.move_entry(backup_path,moved).unwrap();
    assert!(f.0.resolve(backup_path).is_err());assert_ne!(f.0.resolve(moved).unwrap(),id);
    assert!(f.0.move_entry(moved,source).is_err());assert!(f.0.resolve(moved).is_ok());
}
// ------------------------=
// FUNC: support_report_is_typed_and_redacted
// DESC: Parses the exported JSON and checks its complete allowlisted schema and unavailable-value semantics.
// ------------------=
#[test]
fn support_report_is_typed_and_redacted() {
    let r=SupportReport{architecture:1,installed:true,width:1024,height:768,ready_devices:3,storage_used:Some(7),storage_total:Some(10),tasks:None,browser_state:Some(2),browser_error:None};
    let mut bytes=[0;512];let n=r.json(&mut bytes).unwrap();
    let value:serde_json::Value=serde_json::from_slice(&bytes[..n]).unwrap();
    assert_eq!(value,serde_json::json!({"format":1,"architecture":1,"installed":true,"width":1024,"height":768,
        "ready_devices":3,"storage_used_blocks":7,"storage_total_blocks":10,"tasks":null,"browser_state":2,"browser_error":null}));
    assert_eq!(r.json(&mut [0;8]),Err(Error::Capacity));
}

struct FaultFiles {source:Vec<u8>,target:Vec<u8>,corrupt:bool,fail_write:bool,writes:usize}
impl Files for FaultFiles {
    // ------------------------=
    // FUNC: read
    // DESC: Returns independently held source or target bytes for injected storage-failure tests.
    // ------------------=
    fn read(&mut self,path:&[u8],out:&mut [u8])->Result<usize,Error> {
        let bytes=if path==b"source" {&self.source}else{&self.target};
        if bytes.len()>out.len() {return Err(Error::Capacity);}
        out[..bytes.len()].copy_from_slice(bytes);Ok(bytes.len())
    }
    // ------------------------=
    // FUNC: copy
    // DESC: Injects a successful storage call whose readback is corrupt, to test independent verification.
    // ------------------=
    fn copy(&mut self,_:&[u8],_:&[u8])->Result<(),Error> {
        self.target=self.source.clone();if self.corrupt {self.target[0]^=1;}Ok(())
    }
    // ------------------------=
    // FUNC: write
    // DESC: Injects commit failure or post-write corruption without falsely reporting verification success.
    // ------------------=
    fn write(&mut self,_:&[u8],bytes:&[u8])->Result<u32,Error> {
        self.writes+=1;if self.fail_write {return Err(Error::Storage);}
        self.target=bytes.to_vec();if self.corrupt {self.target[0]^=1;}Ok(2)
    }
}
// ------------------------=
// FUNC: backup_and_restore_reject_corrupt_storage
// DESC: Proves untrusted backup bytes never reach write, commit failures preserve target, and bad readback is not success.
// ------------------=
#[test]
fn backup_and_restore_reject_corrupt_storage() {
    let mut files=FaultFiles{source:b"original".to_vec(),target:b"previous".to_vec(),corrupt:true,fail_write:false,writes:0};
    assert_eq!(backup(&mut files,b"source",b"target"),Err(Error::Integrity));
    let expected=checksum(b"original");files.target=b"previous".to_vec();
    assert_eq!(restore(&mut files,b"source",b"target",[0;32]),Err(Error::Integrity));
    assert_eq!(files.writes,0);assert_eq!(files.target,b"previous");
    files.fail_write=true;
    assert_eq!(restore(&mut files,b"source",b"target",expected),Err(Error::Storage));
    assert_eq!(files.target,b"previous");files.fail_write=false;
    assert_eq!(restore(&mut files,b"source",b"target",expected),Err(Error::Integrity));
}
