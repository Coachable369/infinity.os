//! Exercises production worker admission/cleanup against a deterministic native-task adapter.
//! This is worker control verification, not guest CPython execution evidence.
use std::cell::{Cell,RefCell};
#[path="../kernel/runtime/automation.rs"]pub mod catalog;
thread_local!{
    static DURABLE:RefCell<Vec<u8>>=const{RefCell::new(Vec::new())};
    static FAIL_COMMIT:Cell<bool>=const{Cell::new(false)};
    static ADMIN:Cell<bool>=const{Cell::new(true)};
    static POLLS:Cell<usize>=const{Cell::new(0)};
    static CANCELS:Cell<usize>=const{Cell::new(0)};
    static RELEASES:Cell<usize>=const{Cell::new(0)};
    static EXIT:Cell<Option<i64>>=const{Cell::new(None)};
}
mod storage{
    pub mod object{#[derive(Clone,Copy,PartialEq,Eq)]pub struct ObjectId(pub[u8;16]);#[derive(PartialEq)]pub enum Space{Personal}pub struct Object{pub id:ObjectId}pub struct Metadata{pub object:Object,pub owner:ObjectId,pub space:Space,pub current_version:u32}}
    pub struct Child{pub path:[u8;96],pub path_len:u8}
    // ------------------------=
    // FUNC: automation_state_commit
    // DESC: Injects checkpoint failures while retaining the exact last durable artifact.
    // ------------------=
    pub fn automation_state_commit(bytes:&[u8])->Result<u32,()>{if crate::FAIL_COMMIT.get(){return Err(());}crate::catalog::Catalog::decode(bytes).unwrap();crate::DURABLE.with_borrow_mut(|d|*d=bytes.to_vec());Ok(1)}
    // ------------------------=
    // FUNC: object_inspect_path
    // DESC: Supplies an owned source identity through the same typed storage shape used by the native worker.
    // ------------------=
    pub fn object_inspect_path(_: &[u8])->Result<(object::Metadata,()),()>{Ok((object::Metadata{object:object::Object{id:object::ObjectId([1;16])},owner:object::ObjectId([7;16]),space:object::Space::Personal,current_version:1},()))}
    // ------------------------=
    // FUNC: object_read_path
    // DESC: Provides an immutable binary source snapshot for admission testing.
    // ------------------=
    pub fn object_read_path(_:&[u8],version:Option<u32>,out:&mut[u8])->Result<(object::ObjectId,usize),()>{assert_eq!(version,Some(1));out[..4].copy_from_slice(&[1,2,3,4]);Ok((object::ObjectId([1;16]),4))}
    // ------------------------=
    // FUNC: namespace_child_nth
    // DESC: Returns an empty adjacent-module namespace for the bounded fixture.
    // ------------------=
    pub fn namespace_child_nth(_:&[u8],_:usize)->Result<Option<Child>,()>{Ok(None)}
}
mod runtime{
    pub use crate::catalog as automation;
    pub struct Runtime{pub automation:automation::Catalog,pub automation_ready:bool,pub identity:Identity}
    pub struct Identity;
    pub mod identity{pub struct StableId(pub[u8;16]);#[derive(PartialEq)]pub enum UserState{Active}pub struct User{pub state:UserState}}
    impl Identity{
        // ------------------------=
        // FUNC: user
        // DESC: Resolves only the fixture's approving user.
        // ------------------=
        pub fn user(&self,id:identity::StableId)->Option<identity::User>{(id.0==[7;16]).then_some(identity::User{state:identity::UserState::Active})}
        // ------------------------=
        // FUNC: is_administrator
        // DESC: Allows tests to revoke the persistent executing account between native slices.
        // ------------------=
        pub fn is_administrator(&self,_:identity::StableId)->bool{crate::ADMIN.get()}
    }
    thread_local!{static RUNTIME:std::cell::RefCell<Runtime>=const{std::cell::RefCell::new(Runtime{automation:automation::Catalog::new(),automation_ready:true,identity:Identity})};}
    // ------------------------=
    // FUNC: with_runtime
    // DESC: Runs the production worker against one authoritative catalog without holding it across callbacks.
    // ------------------=
    pub fn with_runtime<T>(f:impl FnOnce(&mut Runtime)->T)->Option<T>{Some(RUNTIME.with_borrow_mut(f))}
    pub mod execution{pub struct SecurityIdentity(pub[u8;16]);}
    pub mod native_memory{
        // ------------------------=
        // FUNC: memory_mut
        // DESC: Supplies a harmless allocator token to the control-only native-task adapter.
        // ------------------=
        pub fn memory_mut()->usize{0}
    }
    pub mod native_process{#[derive(Clone,Copy,PartialEq,Eq)]pub enum State{Runnable,Exited(i64),Cancelled,Deadline,PlatformFailure}}
    pub mod native_job_files{pub struct File<'a>{pub path:&'a[u8],pub bytes:&'a[u8]}}
    pub mod native_python_package{pub const PYTHON:&[u8]=&[42];}
    pub mod native_compile_job{
        // ------------------------=
        // FUNC: platform
        // DESC: Supplies a supported adapter token for worker admission tests.
        // ------------------=
        pub fn platform()->Option<((),())>{Some(((),()))}
    }
    pub mod native_python_entropy{
        // ------------------------=
        // FUNC: next
        // DESC: Produces deterministic private launch entropy for the control-only fixture.
        // ------------------=
        pub fn next(_:&[u8;16])->Option<[u8;32]>{Some([9;32])}
    }
    pub mod native_python_scope{
        // ------------------------=
        // FUNC: readable
        // DESC: Enforces fixture owner equality using the same worker call boundary as production.
        // ------------------=
        pub fn readable(owner:[u8;16],user:[u8;16],_:bool,_:&[u8])->bool{owner==user}
    }
    pub mod native_compiler_task{
        use super::{native_process::State,native_job_files::File};pub struct Task{cancelled:bool,id:u32}
        impl Task{
            // ------------------------=
            // FUNC: start
            // DESC: Allocates an inert task while asserting the worker supplied the approved executable and owner.
            // ------------------=
            pub fn start(image:&'static[u8],args:&[&[u8]],owner:super::execution::SecurityIdentity,id:u32,_:u32,_:usize,_:(),_:(),_:u64)->Result<Self,()>{assert_eq!(image,&[42]);assert_eq!(args.len(),2);assert_eq!(owner.0,[7;16]);Ok(Self{cancelled:false,id})}
            // ------------------------=
            // FUNC: set_launch_secret
            // DESC: Confirms admission supplied private launch entropy before the first execution visit.
            // ------------------=
            pub fn set_launch_secret(&mut self,secret:[u8;32]){assert_eq!(secret,[9;32]);}
            // ------------------------=
            // FUNC: cancel
            // DESC: Records cooperative cancellation separately from immediate task reclamation.
            // ------------------=
            pub fn cancel(&mut self){self.cancelled=true;crate::CANCELS.set(crate::CANCELS.get()+1);}
            // ------------------------=
            // FUNC: poll
            // DESC: Asserts durable process admission precedes every simulated execution visit.
            // ------------------=
            pub fn poll(&mut self,_:u64,authorized:bool,_:usize,_:&[u8],files:&[File<'_>],out:&mut[u8],_:&mut[u8])->State{
                if !authorized||self.cancelled{return State::Cancelled;}
                crate::DURABLE.with_borrow(|bytes|{let c=crate::catalog::Catalog::decode(bytes).unwrap();assert!(c.count(crate::catalog::Kind::Daemon)+c.count(crate::catalog::Kind::Schedule)>0);});
                let exists=super::with_runtime(|r|[crate::catalog::Kind::Daemon,crate::catalog::Kind::Schedule].iter().any(|kind|(0..r.automation.count(*kind)).any(|index|r.automation.nth(*kind,index).is_some_and(|e|e.process_id==self.id&&e.state==crate::catalog::State::Running)))).unwrap();assert!(exists);
                assert_eq!(files[0].bytes,&[1,2,3,4]);out[..4].copy_from_slice(&[11,22,33,44]);crate::POLLS.set(crate::POLLS.get()+1);crate::EXIT.get().map_or(State::Runnable,State::Exited)
            }
            // ------------------------=
            // FUNC: result_lengths
            // DESC: Returns the exact typed output length produced by the deterministic native adapter.
            // ------------------=
            pub fn result_lengths(&self,_:&[u8],_:&mut[u8],_:&mut[u8])->Option<(usize,usize,bool)>{Some((4,0,false))}
            // ------------------------=
            // FUNC: execution_id
            // DESC: Exposes simulated native entry only after the adapter has received an execution visit.
            // ------------------=
            pub fn execution_id(&self)->Option<u32>{(crate::POLLS.get()>0).then_some(self.id)}
            // ------------------------=
            // FUNC: release
            // DESC: Counts each native allocation reclamation and consumes its unique task handle.
            // ------------------=
            pub fn release(self,_:usize)->Result<(),()>{crate::RELEASES.set(crate::RELEASES.get()+1);Ok(())}
        }
    }
    pub use crate::automation_worker;
}
pub use runtime::*;
#[path="../kernel/runtime/automation_worker.rs"]pub mod automation_worker;
// ------------------------=
// FUNC: command
// DESC: Stages and durably commits real parsed management commands before publishing them to the worker.
// ------------------=
fn command(input:&[u8],now:u64)->u32{let mut c=runtime::with_runtime(|r|r.automation).unwrap();let id=c.apply(&catalog::parse(input).unwrap(),[7;16],true,now).unwrap().unwrap();let mut bytes=[0;catalog::STATE_BYTES];let n=c.encode(&mut bytes).unwrap();storage::automation_state_commit(&bytes[..n]).unwrap();runtime::with_runtime(|r|r.automation=c);id}
// ------------------------=
// FUNC: pump
// DESC: Advances a full round of bounded worker visits with controlled UTC and monotonic clocks.
// ------------------=
fn pump(now:u64){for step in 0..4{runtime::automation_worker::poll(now*1_000_000_000+step,now,now);}}
// ------------------------=
// FUNC: state
// DESC: Inspects authoritative lifecycle state without interpreting logs or diagnostics.
// ------------------=
fn state(id:u32)->catalog::Entry{runtime::with_runtime(|r|r.automation.get(id).unwrap()).unwrap()}
// ------------------------=
// FUNC: main
// DESC: Verifies claim-before-run, failed commit isolation, cleanup, output, completion retry, revocation and stop/kill distinction.
// ------------------=
fn main(){
    let id=command(b"service create job /job.py start=boot",1);FAIL_COMMIT.set(true);pump(1);assert_eq!(POLLS.get(),0);assert_eq!(state(id).state,catalog::State::Stopped);assert_eq!(RELEASES.get(),1);
    FAIL_COMMIT.set(false);pump(6);assert_eq!(state(id).state,catalog::State::Running);pump(7);assert!(POLLS.get()>0);
    command(b"service stop job",8);let cancels=CANCELS.get();pump(8);assert_eq!(CANCELS.get(),cancels+1);assert_eq!(state(id).state,catalog::State::Stopped);
    command(b"service start job",9);pump(9);let cancels=CANCELS.get();command(b"service kill job",10);pump(10);assert_eq!(CANCELS.get(),cancels);assert_eq!(state(id).last_exit,Some(-9));
    command(b"service start job",11);pump(11);ADMIN.set(false);pump(12);assert_eq!(state(id).state,catalog::State::Failed);assert_eq!(state(id).last_exit,Some(-130));ADMIN.set(true);
    let once=command(b"schedule create one /one.py at=20",13);pump(20);assert_eq!(state(once).state,catalog::State::Running);EXIT.set(Some(0));FAIL_COMMIT.set(true);pump(21);assert_eq!(state(once).state,catalog::State::Running);let count=POLLS.get();pump(22);assert_eq!(POLLS.get(),count);FAIL_COMMIT.set(false);pump(23);assert_eq!(state(once).state,catalog::State::Completed);let mut out=[0;32];assert_eq!(runtime::automation_worker::copy_logs(once,&mut out),4);assert_eq!(&out[..4],&[11,22,33,44]);pump(999);assert_eq!(POLLS.get(),count);
    DURABLE.with_borrow(|bytes|assert_eq!(catalog::Catalog::decode(bytes).unwrap().get(once).unwrap().runs,1));
}
