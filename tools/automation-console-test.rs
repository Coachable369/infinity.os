#![allow(dead_code)]
//! Production Console adapter and IdentitySystem with explicit storage fault injection.
#[path="../kernel/runtime/mod.rs"]mod runtime;
#[path="../kernel/ui/mod.rs"]mod ui;
mod storage {
    include!("storage.rs");
    use std::sync::atomic::{AtomicBool,AtomicUsize,Ordering};
    pub static FAIL_COMMIT:AtomicBool=AtomicBool::new(false);
    pub static COMMITS:AtomicUsize=AtomicUsize::new(0);
    // ------------------------=
    // FUNC: object_inspect_path
    // DESC: Supplies one immutable system Python fixture; real disk behavior is covered by the storage harness.
    // ------------------=
    pub fn object_inspect_path(path:&[u8])->Result<(object::ObjectMetadata,usize),object::ObjectError>{
        use object::*;if path!=b"/System/wc.py"{return Err(ObjectError::NotFound);}
        Ok((ObjectMetadata{object:ObjectRef{id:ObjectId([1;16])},kind:ObjectType::Text,space:Space::System,owner:ObjectId([0;16]),content_type:ContentType::Utf8Text,flags:0,current_version:1,logical_size:8,created:0,modified:0,tags:[0;8],tags_len:0},0))
    }
    // ------------------------=
    // FUNC: automation_state_commit
    // DESC: Validates actual binary checkpoints and injects commit failure before counting a durable publication.
    // ------------------=
    pub fn automation_state_commit(bytes:&[u8])->Result<u32,object::ObjectError>{
        if FAIL_COMMIT.load(Ordering::Relaxed){return Err(object::ObjectError::TransactionFailed);}
        crate::runtime::automation::Catalog::decode(bytes).map_err(|_|object::ObjectError::CorruptContent)?;Ok(COMMITS.fetch_add(1,Ordering::Relaxed)as u32+1)
    }
}
struct Output;
impl Output{
    // ------------------------=
    // FUNC: write_line
    // DESC: Ignores display diagnostics without making human-readable strings a test oracle.
    // ------------------=
    fn write_line(&mut self,_:&[u8]){}
    // ------------------------=
    // FUNC: write_segments
    // DESC: Accepts the adapter's human-readable presentation without inspecting its prose.
    // ------------------=
    fn write_segments(&mut self,_:&[&[u8]]){}
    // ------------------------=
    // FUNC: write_number
    // DESC: Accepts displayed counters while assertions use authoritative typed values.
    // ------------------=
    fn write_number(&mut self,_:&[u8],_:u64){}
}
struct ConsoleRuntime{current_user:runtime::identity::StableId,current_session:runtime::identity::StableId,output:Output,protected_inspections:usize}
#[path="../kernel/core/console_automation.rs"]mod adapter;
impl ConsoleRuntime{
    // ------------------------=
    // FUNC: execute_runtime_command
    // DESC: Records delegation to the existing protected-service inspector without inspecting command prose.
    // ------------------=
    fn execute_runtime_command(&mut self,_:&[u8])->bool{self.protected_inspections+=1;true}
}
// ------------------------=
// FUNC: output_text
// DESC: Supplies an inert runtime diagnostic sink.
// ------------------=
fn output_text(_:&[u8]){}
// ------------------------=
// FUNC: service_id
// DESC: Resolves the protected service used to exercise adapter routing.
// ------------------=
fn service_id(name:&[u8])->Option<u32>{(name==b"runtime").then_some(1)}
// ------------------------=
// FUNC: service_name
// DESC: Supplies a display label without using it as an assertion oracle.
// ------------------=
fn service_name(_:u32)->&'static[u8]{b"runtime"}
// ------------------------=
// FUNC: service_state_text
// DESC: Supplies a display state without using it as an assertion oracle.
// ------------------=
fn service_state_text(_:runtime::service::ServiceState)->&'static[u8]{b"ready"}
// ------------------------=
// FUNC: main
// DESC: Verifies real session authorization, typed receipts, stale read guards, failure rollback and whitespace command dispatch.
// ------------------=
fn main(){
    use runtime::{automation::*,identity::*};use std::sync::atomic::Ordering;
    runtime::initialize(false);
    let(admin,local,member,ordinary,remote)=runtime::with_runtime(|r|{
        r.automation_ready=true;r.identity.begin_onboarding().unwrap();r.identity.create_machine(b"Automation",0xaa64,1,1).unwrap();
        let admin=r.identity.create_user(b"admin",b"Administrator",2).unwrap();r.identity.create_password(admin.id,b"admin password",3).unwrap();r.identity.complete_onboarding().unwrap();
        let local=r.identity.create_session(admin.id,b"admin password",4).unwrap();
        let member=match r.identity.manage_accounts(local.id,AccountRequest::CreateUser{handle:b"member",display_name:b"Member",password:b"member password",now:5}).unwrap(){AccountResult::User(user)=>user,_=>panic!()};
        let ordinary=r.identity.create_session(member.id,b"member password",6).unwrap();let key=ed25519_dalek::SigningKey::from_bytes(&[7;32]).verifying_key().to_bytes();
        r.identity.manage_ssh_key_durable(local.id,admin.id,key,false,7,|_|Ok(())).unwrap();let remote=r.identity.create_ssh_session(admin.id,key,8).unwrap();(admin,local,member,ordinary,remote)
    }).unwrap();
    let mut c=ConsoleRuntime{current_user:admin.id,current_session:local.id,output:Output,protected_inspections:0};assert!(c.automation_access().can_manage);
    assert_eq!(c.task_automation_execute_receipt(b"service create demo /System/wc.py",None),Err(Error::ClockUnavailable));
    runtime::ai::clock::SYSTEM.observe(1_800_000_000,0);
    let receipt=c.task_automation_execute_receipt(b"service create demo /System/wc.py",None).unwrap();assert_eq!(receipt.action,Action::Create);let id=receipt.id.unwrap();assert_ne!(id,0);
    let original=runtime::with_runtime(|r|r.automation.get(id).unwrap().definition()).unwrap();
    assert_eq!(c.task_automation_execute_receipt(b"service create missing /missing.py",None),Err(Error::ScriptUnavailable));
    let before=runtime::with_runtime(|r|r.automation).unwrap();storage::FAIL_COMMIT.store(true,Ordering::Relaxed);
    assert_eq!(c.task_automation_execute_receipt(b"service disable demo",Some(original)),Err(Error::Storage));assert_eq!(runtime::with_runtime(|r|r.automation).unwrap(),before);storage::FAIL_COMMIT.store(false,Ordering::Relaxed);
    c.task_automation_execute_receipt(b"service disable demo",Some(original)).unwrap();
    assert_eq!(c.task_automation_execute_receipt(b"service inspect demo",Some(original)),Err(Error::Conflict));assert_eq!(c.task_automation_execute_receipt(b"service logs demo",Some(original)),Err(Error::Conflict));
    c.current_user=member.id;c.current_session=ordinary.id;assert!(c.automation_access().can_read);assert!(!c.automation_access().can_manage);
    assert_eq!(c.task_automation_execute_receipt(b"service start demo",None),Err(Error::Denied));assert_eq!(c.task_automation_execute_receipt(b"service inspect demo",None),Err(Error::Denied));assert_eq!(c.task_automation_execute_receipt(b"service logs demo",None),Err(Error::Denied));
    c.current_user=admin.id;c.current_session=remote.id;assert!(c.automation_access().can_read);assert!(!c.automation_access().can_manage);assert_eq!(c.task_automation_execute_receipt(b"service start demo",None),Err(Error::Denied));
    c.current_session=StableId::zero();assert_eq!(c.automation_access().reason,Some(Error::SignInRequired));assert_eq!(c.task_automation_execute_receipt(b"service list",None),Err(Error::SignInRequired));
    c.current_session=local.id;assert!(c.automation_command(b" \tservice\tinspect\truntime \n"));assert_eq!(c.protected_inspections,1);assert!(c.automation_command(b"service\tstart\tdemo"));assert_eq!(runtime::with_runtime(|r|r.automation.get(id).unwrap().state),Some(State::Queued));
    assert_eq!(c.task_automation_execute_receipt(b"service start demo",None),Err(Error::Busy));assert_eq!(c.task_automation_execute_receipt(b"service kill runtime",None),Err(Error::Protected));
    runtime::with_runtime(|r|r.automation_ready=false);assert_eq!(c.automation_access().reason,Some(Error::Storage));
    assert_eq!(storage::COMMITS.load(Ordering::Relaxed),3);
}
