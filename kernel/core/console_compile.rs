//! Editor-owned native compilation. Uses the existing reviewed assistant UI.
use super::*;
use crate::runtime::{native_compile_job as job, execution::SecurityIdentity, capability::CapabilityType};
use crate::ui::app_assistant as assistant;
struct Lease { owner: SecurityIdentity, capability:u64, name:[u8;28] }
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum ReplyTarget { Assistant, Console }
static mut LEASE: Option<Lease> = None;
static mut REPLY_TARGET: ReplyTarget = ReplyTarget::Assistant;
static mut APP: Option<(SecurityIdentity,crate::runtime::native_app_task::Task)> = None;
static mut APP_ACTIVE: bool = false;
static mut APP_BYTES:[u8;256*1024]=[0;256*1024];
static mut APP_OUTPUT:[u8;4096]=[0;4096];
static mut APP_OUTPUT_LENGTH:usize=0;
#[no_mangle]
pub static mut INFINITY_NATIVE_APP_SNAPSHOT:[u64;8]=[0x494e464341505031,1,0,0,0,0,0,0];
#[no_mangle]
pub static mut INFINITY_NATIVE_APP_NAME:[u8;28]=[0;28];

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(super) enum StartError {
    Busy,
    SessionUnavailable,
    AiDisabled,
    ClockUnavailable,
    CapabilityUnavailable,
    CompilerUnavailable,
    SourceInvalid,
}

// ------------------------=
// FUNC: active
// DESC: Requires a live authenticated owner session before starting or continuing native compilation.
// ------------------=
fn active(console:&ConsoleRuntime)->bool {
    crate::runtime::with_runtime(|runtime| (0..crate::runtime::identity::MAX_SESSIONS)
        .filter_map(|i|runtime.identity.session_nth(i)).any(|s|s.id==console.current_session
            && s.user==console.current_user && s.state==crate::runtime::identity::SessionState::Active)).unwrap_or(false)
}
impl crate::storage::object::ObjectCapabilityPolicy for Lease {
    // ------------------------=
    // FUNC: authorize
    // DESC: Restricts the explicit compile approval to publishing a new executable object, never replacing an existing file.
    // ------------------=
    fn authorize(&self,operation:crate::storage::object::ObjectOperation,_:Option<crate::storage::object::ObjectRef>)->bool {
        use crate::storage::object::ObjectOperation as O;
        if !matches!(operation,O::Create|O::NamespaceAttach|O::RelationshipAttach){return false;}
        let Some(now)=crate::runtime::node_client::clock() else{return false;};
        crate::runtime::with_runtime(|r|r.capabilities.validate(self.capability,self.owner,
            CapabilityType::ObjectWrite,0,1,0,now).is_ok()).unwrap_or(false)
    }
}
// ------------------------=
// FUNC: start
// DESC: Starts an approved immutable editor snapshot, reclaiming expired authority before issuing its publication lease.
// ------------------=
pub(super) fn start(console:&ConsoleRuntime)->Result<(),StartError> {unsafe {
    start_source(console,console.editor_document.bytes(),ReplyTarget::Assistant,true)
}}

// ------------------------=
// FUNC: start_source
// DESC: Starts one immutable source snapshot for an explicit UI or console owner without ambient filesystem access.
// ------------------=
unsafe fn start_source(console:&ConsoleRuntime,source:&[u8],reply:ReplyTarget,requires_ai:bool)->Result<(),StartError> {
    if core::ptr::addr_of!(LEASE).read().is_some() {return Err(StartError::Busy);}
    if !active(console) {return Err(StartError::SessionUnavailable);}
    if requires_ai && !console.ai_chat_allowed() {return Err(StartError::AiDisabled);}
    let Some(now)=crate::runtime::node_client::clock() else{return Err(StartError::ClockUnavailable);};
    let owner=SecurityIdentity(console.current_session.0);
    let capability=crate::runtime::with_runtime(|r|{
        r.capabilities.reclaim_expired_leaves(now);
        r.capabilities.grant(CapabilityType::ObjectWrite,0,1,0,
            owner,owner,Some(now.saturating_add(125)),0).ok()
    }).flatten();
    let Some(capability)=capability else{return Err(StartError::CapabilityUnavailable);};
    let ticks=crate::runtime::ai::qwen::workers::clock_ns();
    if let Err(error)=job::start(owner,source,ticks) {
        crate::runtime::with_runtime(|r|{let _=r.capabilities.retire_leaf(capability,owner);});
        return Err(match error {
            job::Error::Busy=>StartError::Busy,
            job::Error::Unavailable=>StartError::CompilerUnavailable,
            job::Error::SourceTooLarge=>StartError::SourceInvalid,
        });
    }
    let mut name=[0;28];name[..8].copy_from_slice(b"program-");
    for i in 0..16 {name[8+i]=b"0123456789abcdef"[((ticks>>((15-i)*4))&15) as usize];}
    name[24..].copy_from_slice(b".elf");
    REPLY_TARGET=reply;LEASE=Some(Lease{owner,capability,name});Ok(())
}

// ------------------------=
// FUNC: command
// DESC: Implements native terminal cc and run commands against explicit ObjectStore paths.
// ------------------=
pub(super) fn command(console:&mut ConsoleRuntime,input:&[u8])->bool {unsafe {
    use crate::ui::native_development::Command;
    let Some(command)=crate::ui::native_development::parse(input) else{return false;};
    let path=match command{
        Command::Compile(path)=>path,
        Command::Run(path)=>{launch(console,path);return true;},
        Command::Status=>{
            let owner=SecurityIdentity(console.current_session.0);
            console.output.write_line(match job::status(owner){
                job::Status::Idle=>b"Native C compiler: idle.",job::Status::Compile=>b"Native C compiler: compiling.",
                job::Status::Link=>b"Native C compiler: linking.",job::Status::Complete=>b"Native C compiler: complete.",
                job::Status::Failed=>b"Native C compiler: failed.",job::Status::Cancelled=>b"Native C compiler: cancelled.",
            });return true;
        }
        Command::Cancel=>{
            let owner=SecurityIdentity(console.current_session.0);
            console.output.write_line(if job::cancel(owner){b"Native C build cancellation accepted."}else{b"cc-cancel: no active build owned by this session."});
            return true;
        }
    };
    let mut source=[0u8;job::SOURCE_LIMIT];
    let Ok((_,length))=crate::storage::object_read_path(path,None,&mut source) else{
        console.output.write_line(b"cc: source object was not found or exceeds the native source limit.");return true;
    };
    match start_source(console,&source[..length],ReplyTarget::Console,false) {
        Ok(())=>console.output.write_line(b"Native C build queued."),
        Err(StartError::Busy)=>console.output.write_line(b"cc: another native build is active."),
        Err(StartError::SessionUnavailable)=>console.output.write_line(b"cc: an authenticated session is required."),
        Err(StartError::ClockUnavailable)=>console.output.write_line(b"cc: the system clock is not ready."),
        Err(StartError::CapabilityUnavailable)=>console.output.write_line(b"cc: output capability unavailable."),
        Err(StartError::CompilerUnavailable)=>console.output.write_line(b"cc: native compiler unavailable on this boot."),
        Err(StartError::SourceInvalid)=>console.output.write_line(b"cc: empty or oversized source."),
        Err(StartError::AiDisabled)=>console.output.write_line(b"cc: unexpected policy failure."),
    }
    true
}}
// ------------------------=
// FUNC: poll
// DESC: Keeps compiler work off input dispatch, cancels lost ownership, and reports actual compiler output through the existing panel.
// ------------------=
pub(super) fn poll(console:&mut ConsoleRuntime) {unsafe {
    poll_app(console);
    let Some(lease)=(&*(&raw const LEASE)).as_ref() else{return;};
    let owner=SecurityIdentity(console.current_session.0);
    let now=crate::runtime::ai::qwen::workers::clock_ns();
    let target=REPLY_TARGET;
    let allowed=active(console)&&(target==ReplyTarget::Console||console.ai_chat_allowed()&&console.assistant_owner_alive(2))
        && <Lease as crate::storage::object::ObjectCapabilityPolicy>::authorize(lease,crate::storage::object::ObjectOperation::Create,None);
    if !job::poll(owner,allowed,now){return;}
    if lease.owner==owner {
        let mut panel=assistant::read(2);
        job::result(owner,|status,binary,diagnostics,_truncated| {
            if status==job::Status::Complete {
                let prefix=b"/home/default/";
                let mut path=[0;95];path[..prefix.len()].copy_from_slice(prefix);
                path[prefix.len()..prefix.len()+lease.name.len()].copy_from_slice(&lease.name);
                let length=prefix.len()+lease.name.len();
                let saved=crate::storage::object_save_download(crate::storage::object::DownloadCreateRequest {
                    name:&lease.name,content:binary,path:&path[..length],media_type:b"application/x-elf",
                    owner:crate::storage::object::ObjectId(console.current_user.0),
                },lease);
                if let Ok(reference)=saved {
                    INFINITY_NATIVE_APP_NAME=lease.name;
                    let state=&mut *(&raw mut INFINITY_NATIVE_APP_SNAPSHOT);
                    state[2]=state[2].wrapping_add(1);state[3]=1;
                    state[4]=u64::from_le_bytes(reference.id.0[..8].try_into().unwrap());
                    state[5]=u64::from_le_bytes(reference.id.0[8..].try_into().unwrap());
                    let mut message=[0;160];let lead=b"Compiled successfully. Executable: ";
                    message[..lead.len()].copy_from_slice(lead);message[lead.len()..lead.len()+length].copy_from_slice(&path[..length]);
                    if target==ReplyTarget::Assistant{panel.reply(&message[..lead.len()+length]);}
                    else{console.output.write_line(&message[..lead.len()+length]);}
                }else if target==ReplyTarget::Assistant{panel.reply(b"Compilation succeeded, but the executable could not be saved. No existing file was overwritten.");}
                else{console.output.write_line(b"cc: executable publication failed without overwriting existing data.");}
            }else if !diagnostics.is_empty(){
                if target==ReplyTarget::Assistant{panel.reply(diagnostics);}else{
                    for line in diagnostics.split(|byte|*byte==b'\n'){if !line.is_empty(){console.output.write_line(line);}}
                }
            }else if status==job::Status::Cancelled{
                if target==ReplyTarget::Assistant{panel.reply(b"Compilation cancelled. No executable was published.");}
                else{console.output.write_line(b"cc: build cancelled; no executable was published.");}
            }else if target==ReplyTarget::Assistant{panel.reply(b"Native compilation failed. The source remains in the editor; no executable was published.");}
            else{console.output.write_line(b"cc: native compilation failed; no executable was published.");}
        });
        if target==ReplyTarget::Assistant{assistant::write(2,panel);}console.redraw();
    }
    let lease=core::ptr::addr_of_mut!(LEASE).replace(None).unwrap();
    crate::runtime::with_runtime(|r|{let _=r.capabilities.retire_leaf(lease.capability,lease.owner);});
}}

// ------------------------=
// FUNC: launch
// DESC: Opens a bounded native ELF selected in File Navigator in a private user-mode process, never by calling its entry in the kernel.
// ------------------=
pub(super) fn launch(console:&mut ConsoleRuntime,path:&[u8])->bool {unsafe {
    if !path.ends_with(b".elf") {return false;}
    console.open_command_window();
    if APP_ACTIVE || !active(console) {
        console.output.write_line(b"Cannot launch: another native program is active or the session is unavailable.");return true;
    }
    let Some((kernel,controller))=job::platform() else {
        console.output.write_line(b"Native execution is unavailable on this boot.");return true;
    };
    let bytes=&mut *(&raw mut APP_BYTES);
    let Ok((_,length))=crate::storage::object_read_path(path,None,bytes) else {
        console.output.write_line(b"Cannot read this executable.");return true;
    };
    let owner=SecurityIdentity(console.current_session.0);
    let now=crate::runtime::ai::qwen::workers::clock_ns();
    match crate::runtime::native_app_task::Task::start(&bytes[..length],owner,2,
        crate::runtime::native_memory::memory_mut(),kernel,controller,now.saturating_add(60_000_000_000)) {
        Ok(task)=>{
            APP=Some((owner,task));APP_ACTIVE=true;APP_OUTPUT_LENGTH=0;
            INFINITY_NATIVE_APP_SNAPSHOT[3]=2;
            console.output.write_line(b"Running isolated native C program...");
        }
        Err(_)=>console.output.write_line(b"Executable rejected: invalid format, architecture, permissions, or insufficient memory."),
    }
    console.redraw();true
}}
// ------------------------=
// FUNC: poll_app
// DESC: Runs one bounded application slice and exposes only checked console output; unsupported authority fails explicitly.
// ------------------=
fn poll_app(console:&mut ConsoleRuntime) {unsafe {
    use crate::runtime::native_process::State;
    let Some((owner,task))=(&mut *(&raw mut APP)).as_mut() else{return;};
    let authorized=*owner==SecurityIdentity(console.current_session.0)&&active(console)&&console.assistant_owner_alive(1);
    let state=task.poll(crate::runtime::ai::qwen::workers::clock_ns(),authorized,|_,tables,op,args| {
        if op!=0x1002{return -88;}
        let length=args[1].min(4096) as usize;
        if length>4096-APP_OUTPUT_LENGTH{return -28;}
        let buffer=&mut *(&raw mut APP_OUTPUT);
        if crate::runtime::native_memory::copy_from_user(tables,args[0],&mut buffer[APP_OUTPUT_LENGTH..APP_OUTPUT_LENGTH+length]).is_err(){return -14;}
        APP_OUTPUT_LENGTH+=length;length as i64
    });
    if state==State::Runnable{return;}
    let (owner,task)=core::ptr::addr_of_mut!(APP).replace(None).unwrap();APP_ACTIVE=false;
    let released=task.release(crate::runtime::native_memory::memory_mut()).is_ok();
    INFINITY_NATIVE_APP_SNAPSHOT[3]=if matches!(state,State::Exited(_))&&released{3}else{4};
    INFINITY_NATIVE_APP_SNAPSHOT[6]=if let State::Exited(code)=state{code as u64}else{u64::MAX};
    INFINITY_NATIVE_APP_SNAPSHOT[7]=APP_OUTPUT_LENGTH as u64;
    if owner==SecurityIdentity(console.current_session.0) {
        for line in (&*(&raw const APP_OUTPUT))[..APP_OUTPUT_LENGTH].split(|b|*b==b'\n') {
            if !line.is_empty(){console.output.write_line(line);}
        }
        match state {
            State::Exited(code) if released=>console.output.write_number(b"Program exit status: ",code as u64),
            State::Cancelled=>console.output.write_line(b"Program cancelled."),
            _=>console.output.write_line(b"Program stopped safely (fault, deadline, or resource limit)."),
        }
        console.redraw();
    }
}}
