//! Editor-owned native compilation. Uses the existing reviewed assistant UI.
use super::*;
use crate::runtime::{native_compile_job as job, execution::SecurityIdentity, capability::CapabilityType};
use crate::ui::app_assistant as assistant;
#[path = "../runtime/native_console_input.rs"]
mod native_input;
#[path = "../ui/workflow_progress.rs"]
mod workflow_progress;
use workflow_progress::{Progress as VoiceProgress, Step as VoiceStep};
static mut APP_INPUT:native_input::Input=native_input::Input::new();
static mut APP_CANCEL:bool=false;
static mut APP_LINE_OPEN:bool=false;
struct Lease {
    owner: SecurityIdentity,
    capability:u64,
    name:[u8;28],
    source_length:usize,
    launch:bool,
    compile_output_reported:bool,
    interaction_objective:[u8;512],
    interaction_length:usize,
    generated:bool,
    document_revision:u64,
    request:[u8;1024],
    request_length:usize,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum ReplyTarget { Assistant, Console }
static mut LEASE: Option<Lease> = None;
static mut REPLY_TARGET: ReplyTarget = ReplyTarget::Assistant;
static mut APP: Option<(SecurityIdentity,crate::runtime::native_app_task::Task)> = None;
static mut APP_ACTIVE: bool = false;
static mut APP_BYTES:[u8;256*1024]=[0;256*1024];
static mut APP_OUTPUT:[u8;4096]=[0;4096];
static mut APP_OUTPUT_LENGTH:usize=0;
static mut APP_UI_REVISION:u64=0;
#[no_mangle]
pub static mut INFINITY_NATIVE_APP_SNAPSHOT:[u64;8]=[0x494e464341505031,1,0,0,0,0,0,0];
#[no_mangle]
pub static mut INFINITY_NATIVE_APP_NAME:[u8;28]=[0;28];
#[no_mangle]
pub static mut INFINITY_NATIVE_WORKFLOW_SNAPSHOT:[u64;9]=[0x494e4643574f524b,1,0,0,0,0,0,0,0];
static mut WORKFLOW_FLAGS:u64=0;
static mut VOICE_PROGRESS:VoiceProgress=VoiceProgress::new();
static mut VOICE_OWNER:SecurityIdentity=SecurityIdentity([0;16]);
#[no_mangle]
pub static mut INFINITY_NATIVE_WORKFLOW_VOICE_SNAPSHOT:[u64;8]=[0x494e46574f494345,1,0,0,0,0,0,0];

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(super) enum StartError {
    Busy,
    SessionUnavailable,
    AiDisabled,
    ClockUnavailable,
    CapabilityUnavailable,
    CompilerUnavailable,
    SourceInvalid,
    SaveFailed,
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
pub(super) fn start(console:&mut ConsoleRuntime)->Result<(),StartError> {unsafe {
    let mut path=[0u8;95];
    let source_path=if console.editor_document_path_length>0 {
        &console.editor_document_path[..console.editor_document_path_length]
    }else{b"<editor-buffer>"};
    let path_length=source_path.len().min(path.len());path[..path_length].copy_from_slice(&source_path[..path_length]);
    {
        let snapshot:&ConsoleRuntime=console;
        start_source(snapshot,snapshot.editor_document.bytes(),&path[..path_length],ReplyTarget::Assistant,true)?;
    }
    open_build_transcript(console,&path[..path_length],28);
    Ok(())
}}

// ------------------------=
// FUNC: save_generated_and_build
// DESC: Persists generated C under a collision-safe Documents name before admitting a build and optional explicitly requested launch.
// ------------------=
pub(super) fn save_generated_and_build(console:&mut ConsoleRuntime,launch_result:bool)->Result<(),StartError> {unsafe {
    if APP_ACTIVE || (&*(&raw const LEASE)).is_some() {return Err(StartError::Busy);}
    if !active(console) {return Err(StartError::SessionUnavailable);}
    if !console.ai_chat_allowed() {return Err(StartError::AiDisabled);}
    let ticks=crate::runtime::ai::qwen::workers::clock_ns();
    let mut name=[0u8;24];name[..6].copy_from_slice(b"app-c-");
    for i in 0..16 {name[6+i]=b"0123456789abcdef"[((ticks>>((15-i)*4))&15) as usize];}
    name[22..].copy_from_slice(b".c");
    let base=crate::storage::object::DOCUMENTS_PATH;
    let mut path=[0u8;95];path[..base.len()].copy_from_slice(base);path[base.len()]=b'/';
    let length=base.len()+1+name.len();path[base.len()+1..length].copy_from_slice(&name);
    crate::storage::object_create_owned_note_at(&name,console.editor_document.bytes(),&path[..length],
        crate::storage::object::ObjectId(console.current_user.0))
        .map_err(|_|StartError::SaveFailed)?;
    console.editor_document_path[..length].copy_from_slice(&path[..length]);console.editor_document_path_length=length;
    console.editor_document_name[..name.len()].copy_from_slice(&name);console.editor_document_name_length=name.len();
    console.editor_document.save();console.detect_editor_language();
    {
        let snapshot:&ConsoleRuntime=console;
        start_source(snapshot,snapshot.editor_document.bytes(),&path[..length],ReplyTarget::Assistant,true)?;
    }
    open_build_transcript(console,&path[..length],28);
    if let Some(lease)=(&mut *(&raw mut LEASE)).as_mut() {
        lease.launch=launch_result;
        let panel=assistant::read(2);
        let request=&panel.request[..panel.request_len];
        lease.generated=true;
        lease.request[..request.len()].copy_from_slice(request);lease.request_length=request.len();
        if launch_result && assistant::editor_compile_intent::create_application(request)
            && assistant::editor_compile_intent::requests_opponent(request) && request.len()<=512 {
            lease.interaction_objective[..request.len()].copy_from_slice(request);lease.interaction_length=request.len();
        }
    }
    Ok(())
}}

// ------------------------=
// FUNC: start_source
// DESC: Starts one immutable source snapshot for an explicit UI or console owner without ambient filesystem access.
// ------------------=
unsafe fn start_source(console:&ConsoleRuntime,source:&[u8],source_path:&[u8],reply:ReplyTarget,requires_ai:bool)->Result<(),StartError> {
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
    let source_length=source_path.len().min(95);
    REPLY_TARGET=reply;LEASE=Some(Lease { owner,capability,name,source_length,
        launch:false,compile_output_reported:false,interaction_objective:[0;512],interaction_length:0,
        generated:false,document_revision:console.assistant_revision(),request:[0;1024],request_length:0 });
    Ok(())
}

// ------------------------=
// FUNC: publish_workflow
// DESC: Publishes structured compile and launch transitions for installed-runtime behavioral verification.
// ------------------=
unsafe fn publish_workflow(stage:u64,command_length:usize,diagnostic_length:usize,output_length:usize) {
    let state=&mut *(&raw mut INFINITY_NATIVE_WORKFLOW_SNAPSHOT);
    state[2]=state[2].wrapping_add(1)|1;
    if stage==1 {state[4]=0;state[5]=0;state[6]=0;state[7]=0;}
    WORKFLOW_FLAGS|=1u64<<stage.min(63);
    state[3]=stage;
    if command_length>0 {state[4]=command_length as u64;}
    if diagnostic_length>0 {state[5]=diagnostic_length as u64;}
    if output_length>0 {state[6]=output_length as u64;}
    state[7]=WORKFLOW_FLAGS;
    state[2]=state[2].wrapping_add(1);state[8]=state[2];
}

// ------------------------=
// FUNC: publish_voice_progress
// DESC: Publishes content-free narration queue state for installed behavioral verification.
// ------------------=
unsafe fn publish_voice_progress() {
    let (scheduled,announced,pending,in_flight)=(&*(&raw const VOICE_PROGRESS)).snapshot();
    let state=&mut *(&raw mut INFINITY_NATIVE_WORKFLOW_VOICE_SNAPSHOT);
    state[2]=state[2].wrapping_add(1)|1;
    state[3]=scheduled as u64;state[4]=announced as u64;state[5]=pending as u64;
    state[6]=in_flight.map(|step|step as u64+1).unwrap_or(0);
    state[2]=state[2].wrapping_add(1);state[7]=state[2];
}

// ------------------------=
// FUNC: begin_voice_progress
// DESC: Starts ordered one-time narration for the authenticated owner of a new native build.
// ------------------=
unsafe fn begin_voice_progress(owner:SecurityIdentity) {
    VOICE_OWNER=owner;
    (&mut *(&raw mut VOICE_PROGRESS)).begin();
    publish_voice_progress();
}

// ------------------------=
// FUNC: schedule_voice_progress
// DESC: Queues a typed workflow milestone without duplicating an already scheduled announcement.
// ------------------=
unsafe fn schedule_voice_progress(step:VoiceStep) {
    if (&mut *(&raw mut VOICE_PROGRESS)).schedule(step) {publish_voice_progress();}
}

// ------------------------=
// FUNC: voice_progress_enabled
// DESC: Requires the workflow owner to remain active with AI and spoken output enabled.
// ------------------=
fn voice_progress_enabled(console:&ConsoleRuntime,owner:SecurityIdentity)->bool {
    active(console)&&console.ai_chat_allowed()&&SecurityIdentity(console.current_session.0)==owner
        && crate::runtime::with_runtime(|runtime|runtime.identity.ai_profile(console.current_user)
            .map(|profile|profile.speech_output_enabled).unwrap_or(false)).unwrap_or(false)
}

// ------------------------=
// FUNC: poll_voice_progress
// DESC: Speaks queued structural milestones in order without interrupting an AI reply or overlapping native audio.
// ------------------=
unsafe fn poll_voice_progress(console:&ConsoleRuntime) {
    let owner=VOICE_OWNER;
    let progress=&mut *(&raw mut VOICE_PROGRESS);
    let (_,_,pending,in_flight)=progress.snapshot();
    if pending==0&&in_flight.is_none(){return;}
    if !voice_progress_enabled(console,owner) {progress.discard();publish_voice_progress();return;}
    use crate::runtime::ai::{voice_conversation::State as ConversationState,voice_output::OutputState};
    let output=crate::runtime::ai::voice_output::status().state;
    if in_flight.is_some() {
        if matches!(output,OutputState::Complete|OutputState::Failed|OutputState::Cancelled) {
            let _=progress.completed(output==OutputState::Complete);publish_voice_progress();
        }
        return;
    }
    if !matches!(crate::runtime::ai::voice_conversation::state().0,
        ConversationState::Off|ConversationState::Listening)
        || !matches!(output,OutputState::Idle|OutputState::Complete|OutputState::Failed|OutputState::Cancelled) {return;}
    let Some(step)=progress.next() else{return;};
    let now=crate::runtime::ai::qwen::workers::clock_ns()/1_000_000_000;
    let capability=crate::runtime::with_runtime(|runtime|runtime.capabilities.grant(
        CapabilityType::AudioOutput,0,1,0,owner,owner,
        Some(now+crate::runtime::ai::voice_output::OUTPUT_LEASE_SECONDS),0).ok()).flatten();
    let Some(capability)=capability else{return;};
    if crate::runtime::ai::voice_output::submit(owner,capability,step.phrase()).is_ok() {
        let _=progress.submitted(step);publish_voice_progress();
    }else{
        crate::runtime::with_runtime(|runtime|{let _=runtime.capabilities.retire_leaf(capability,owner);});
    }
}

// ------------------------=
// FUNC: open_build_transcript
// DESC: Opens or focuses the command window and records the user-facing native compilation command without discarding an existing terminal transcript.
// ------------------=
unsafe fn open_build_transcript(console:&mut ConsoleRuntime,source_path:&[u8],output_length:usize) {
    WORKFLOW_FLAGS=0;
    begin_voice_progress(SecurityIdentity(console.current_session.0));
    console.command_window_suspended=console.command_window.visible;
    console.open_command_window();
    console.output.write_segments(&[b"$ cc ",source_path]);
    publish_workflow(1,5+source_path.len(),0,output_length);
    console.redraw();
}

// ------------------------=
// FUNC: report_stage_output
// DESC: Writes bounded compiler or linker diagnostics to the command window, including an explicit successful no-diagnostics result.
// ------------------=
fn report_stage_output(console:&mut ConsoleRuntime,label:&[u8],diagnostics:&[u8],truncated:bool) {
    if diagnostics.is_empty() {
        console.output.write_segments(&[label,b": no diagnostics."]);
    }else{
        for line in diagnostics.split(|byte|*byte==b'\n') {if !line.is_empty(){console.output.write_line(line);}}
        if truncated {console.output.write_line(b"Compiler output was truncated to its safe display limit.");}
    }
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
    match start_source(console,&source[..length],path,ReplyTarget::Console,false) {
        Ok(())=>open_build_transcript(console,path,28),
        Err(StartError::Busy)=>console.output.write_line(b"cc: another native build is active."),
        Err(StartError::SessionUnavailable)=>console.output.write_line(b"cc: an authenticated session is required."),
        Err(StartError::ClockUnavailable)=>console.output.write_line(b"cc: the system clock is not ready."),
        Err(StartError::CapabilityUnavailable)=>console.output.write_line(b"cc: output capability unavailable."),
        Err(StartError::CompilerUnavailable)=>console.output.write_line(b"cc: native compiler unavailable on this boot."),
        Err(StartError::SourceInvalid)=>console.output.write_line(b"cc: empty or oversized source."),
        Err(StartError::SaveFailed)=>console.output.write_line(b"cc: source could not be saved."),
        Err(StartError::AiDisabled)=>console.output.write_line(b"cc: unexpected policy failure."),
    }
    true
}}
// ------------------------=
// FUNC: poll
// DESC: Keeps compiler work off input dispatch, cancels lost ownership, and reports actual compiler output through the existing panel.
// ------------------=
pub(super) fn poll(console:&mut ConsoleRuntime) {unsafe {
    poll_voice_progress(console);
    poll_app(console);
    let Some(lease)=(&mut *(&raw mut LEASE)).as_mut() else{return;};
    let owner=SecurityIdentity(console.current_session.0);
    let now=crate::runtime::ai::qwen::workers::clock_ns();
    let target=REPLY_TARGET;
    let mut launch_path=[0u8;95];let mut launch_length=0;
    let allowed=active(console)&&(target==ReplyTarget::Console||console.ai_chat_allowed()&&console.assistant_owner_alive(2))
        && <Lease as crate::storage::object::ObjectCapabilityPolicy>::authorize(lease,crate::storage::object::ObjectOperation::Create,None);
    let finished=job::poll(owner,allowed,now);
    if lease.owner==owner && job::status(owner)==job::Status::Link && !lease.compile_output_reported {
        job::stage_diagnostics(owner,job::Status::Link,|diagnostics,truncated| {
            report_stage_output(console,b"Compiler output",diagnostics,truncated);
            publish_workflow(2,5+lease.source_length,diagnostics.len(),lease.name.len());
            schedule_voice_progress(VoiceStep::Linking);
        });
        lease.compile_output_reported=true;
        console.redraw();
    }
    if !finished{return;}
    if lease.owner==owner {
        let mut panel=assistant::read(2);
        job::result(owner,|status,binary,diagnostics,_truncated| {
            if status==job::Status::Complete {
                report_stage_output(console,b"Linker output",diagnostics,_truncated);
                let prefix=b"/home/default/";
                let mut path=[0;95];path[..prefix.len()].copy_from_slice(prefix);
                path[prefix.len()..prefix.len()+lease.name.len()].copy_from_slice(&lease.name);
                let length=prefix.len()+lease.name.len();
                let saved=crate::storage::object_save_download(crate::storage::object::DownloadCreateRequest {
                    name:&lease.name,content:binary,path:&path[..length],media_type:b"application/x-elf",
                    owner:crate::storage::object::ObjectId(console.current_user.0),
                },lease);
                if let Ok(reference)=saved {
                    if lease.launch {launch_path[..length].copy_from_slice(&path[..length]);launch_length=length;}
                    INFINITY_NATIVE_APP_NAME=lease.name;
                    let state=&mut *(&raw mut INFINITY_NATIVE_APP_SNAPSHOT);
                    state[2]=state[2].wrapping_add(1);state[3]=1;
                    state[4]=u64::from_le_bytes(reference.id.0[..8].try_into().unwrap());
                    state[5]=u64::from_le_bytes(reference.id.0[8..].try_into().unwrap());
                    let mut message=[0;160];let lead=b"Compiled successfully. Executable: ";
                    message[..lead.len()].copy_from_slice(lead);message[lead.len()..lead.len()+length].copy_from_slice(&path[..length]);
                    console.output.write_line(&message[..lead.len()+length]);
                    publish_workflow(3,5+lease.source_length,diagnostics.len(),length);
                    schedule_voice_progress(VoiceStep::ExecutableReady);
                    if target==ReplyTarget::Assistant{panel.reply(&message[..lead.len()+length]);}
                }else{
                    console.output.write_line(b"Executable publication failed; no existing file was overwritten.");
                    schedule_voice_progress(VoiceStep::Failed);
                    if target==ReplyTarget::Assistant{panel.reply(b"Compilation succeeded, but the executable could not be saved. No existing file was overwritten.");}
                }
            }else if !diagnostics.is_empty(){
                report_stage_output(console,b"Compiler output",diagnostics,_truncated);
                publish_workflow(6,5+lease.source_length,diagnostics.len(),lease.name.len());
                schedule_voice_progress(VoiceStep::Failed);
                if target==ReplyTarget::Assistant {
                    panel.reply(diagnostics);
                    if status==job::Status::Failed && allowed && lease.generated
                        && panel.queue_compiler_repair(&lease.request[..lease.request_length],lease.document_revision,console.assistant_revision(),diagnostics) {
                        panel.reply(b"The compiler found errors. Repairing this unchanged source using its diagnostics (maximum two attempts). Your source is saved; no app has launched.");
                    }
                }
            }else if status==job::Status::Cancelled{
                console.output.write_line(b"Compilation cancelled. No executable was published.");
                publish_workflow(6,5+lease.source_length,0,lease.name.len());
                schedule_voice_progress(VoiceStep::Cancelled);
                if target==ReplyTarget::Assistant{panel.reply(b"Compilation cancelled. No executable was published.");}
            }else{
                report_stage_output(console,b"Compiler output",diagnostics,_truncated);
                console.output.write_line(b"Native compilation failed; no executable was published.");
                publish_workflow(6,5+lease.source_length,0,lease.name.len());
                schedule_voice_progress(VoiceStep::Failed);
                if target==ReplyTarget::Assistant{panel.reply(b"Native compilation failed. The source remains in the editor; no executable was published.");}
            }
        });
        if target==ReplyTarget::Assistant{assistant::write(2,panel);}console.redraw();
    }
    let lease=core::ptr::addr_of_mut!(LEASE).replace(None).unwrap();
    crate::runtime::with_runtime(|r|{let _=r.capabilities.retire_leaf(lease.capability,lease.owner);});
    if launch_length>0 {
        launch(console,&launch_path[..launch_length]);
        if APP_ACTIVE && lease.interaction_length>0 {
            let _=crate::runtime::ai::with_ai_runtime(|ai|ai.start_application_agent(console.current_user.0,console.current_session.0,&lease.interaction_objective[..lease.interaction_length],true));
        }
    }
}}

// ------------------------=
// FUNC: launch
// DESC: Opens a bounded native ELF selected in File Navigator in a private user-mode process, never by calling its entry in the kernel.
// ------------------=
pub(super) fn launch(console:&mut ConsoleRuntime,path:&[u8])->bool {unsafe {
    if !path.ends_with(b".elf") {return false;}
    if APP_ACTIVE {
        console.command_window_suspended=true;console.open_command_window();
        console.output.write_line(b"A native program is already running. Exit it or press Ctrl+C before launching another.");
        APP_LINE_OPEN=false;console.redraw();return true;
    }
    console.command_window_suspended=console.command_window.visible;
    console.open_command_window();
    console.output.write_segments(&[b"Launching application: ",path]);
    publish_workflow(4,0,0,path.len());
    console.redraw();
    console.ai_chat_focus=0;
    crate::ui::native_uikit::close();APP_UI_REVISION=0;
    let mut panel=assistant::read(1);panel.focused=false;assistant::write(1,panel);
    if !active(console) {
        console.output.write_line(b"Cannot launch: the authenticated session is unavailable.");return true;
    }
    let Some((kernel,controller))=job::platform() else {
        console.output.write_line(b"Native execution is unavailable on this boot.");return true;
    };
    let bytes=&mut *(&raw mut APP_BYTES);
    let Ok((_,length))=crate::storage::object_read_path(path,None,bytes) else {
        console.output.write_line(b"Cannot read this executable.");return true;
    };
    let owner=SecurityIdentity(console.current_session.0);
    if VOICE_OWNER!=owner || (&*(&raw const VOICE_PROGRESS)).snapshot().0==0 {
        VOICE_OWNER=owner;
        (&mut *(&raw mut VOICE_PROGRESS)).discard();
    }
    schedule_voice_progress(VoiceStep::Launching);
    match crate::runtime::native_app_task::Task::start(&bytes[..length],owner,2,
        crate::runtime::native_memory::memory_mut(),kernel,controller,u64::MAX) {
        Ok(task)=>{
            let _=crate::runtime::ai::application_host::attach(console.current_user.0,console.current_session.0);
            APP=Some((owner,task));APP_ACTIVE=true;APP_OUTPUT_LENGTH=0;
            APP_INPUT=native_input::Input::new();APP_CANCEL=false;APP_LINE_OPEN=false;
            console.reset_input();
            INFINITY_NATIVE_APP_SNAPSHOT[3]=2;
            INFINITY_NATIVE_APP_SNAPSHOT[6]=0;INFINITY_NATIVE_APP_SNAPSHOT[7]=0;
            console.output.write_line(b"Application launched in an isolated native process.");
            publish_workflow(5,0,0,path.len());
            schedule_voice_progress(VoiceStep::Running);
        }
        Err(_)=>{
            console.output.write_line(b"Executable rejected: invalid format, architecture, permissions, or insufficient memory.");
            schedule_voice_progress(VoiceStep::LaunchFailed);
        },
    }
    console.redraw();true
}}
// ------------------------=
// FUNC: input
// DESC: Routes focused terminal input to the running program before shell history or command execution can observe it.
// ------------------=
pub(super) fn input(console:&mut ConsoleRuntime,key:ConsoleKey)->bool {unsafe {
    if !APP_ACTIVE{return false;}
    if crate::ui::native_uikit::presentation().active {
        match key {
            ConsoleKey::Shortcut(b'c')=>APP_CANCEL=true,
            ConsoleKey::Escape=>{let _=crate::ui::native_uikit::request_close();},
            _=>{},
        }
        return true;
    }
    match key {
        ConsoleKey::Shortcut(b'c')=>{APP_CANCEL=true;console.reset_input();},
        ConsoleKey::Shortcut(b'd')=>{(&mut *(&raw mut APP_INPUT)).close();},
        ConsoleKey::Enter=>{
            if (&mut *(&raw mut APP_INPUT)).submit(&console.command[..console.command_length]) {
                INFINITY_NATIVE_APP_SNAPSHOT[6]=0;
                console.output.write_line(&console.command[..console.command_length]);
                APP_LINE_OPEN=false;console.reset_input();
            }
        },
        _=>{console.edit_input(key);},
    }
    true
}}
// ------------------------=
// FUNC: poll_app
// DESC: Runs one bounded application slice with checked terminal I/O, parked reads and authenticated cancellation.
// ------------------=
fn poll_app(console:&mut ConsoleRuntime) {unsafe {
    use crate::runtime::native_process::State;
    let Some((owner,task))=(&mut *(&raw mut APP)).as_mut() else{return;};
    let authorized=*owner==SecurityIdentity(console.current_session.0)&&active(console)&&console.assistant_owner_alive(1)&&!APP_CANCEL;
    if authorized && (&*(&raw const APP_INPUT)).waiting {return;}
    let state=task.poll(crate::runtime::ai::qwen::workers::clock_ns(),authorized,|_,tables,op,args| {
        if op==0x1003 {
            return (&mut *(&raw mut APP_INPUT)).read(args[1].min(1024) as usize,|bytes|
                crate::runtime::native_memory::copy_to_user(tables,args[0],bytes).is_ok());
        }
        if (0x1140..=0x1142).contains(&op) {
            return native_uikit_syscall(tables,op,args);
        }
        if (0x1150..=0x1156).contains(&op) {
            return native_interaction_syscall(tables,op,args,console.current_session.0);
        }
        if op!=0x1002{return -88;}
        let length=args[1].min(4096) as usize;
        if length>4096-APP_OUTPUT_LENGTH{return -28;}
        let buffer=&mut *(&raw mut APP_OUTPUT);
        if crate::runtime::native_memory::copy_from_user(tables,args[0],&mut buffer[APP_OUTPUT_LENGTH..APP_OUTPUT_LENGTH+length]).is_err(){return -14;}
        APP_OUTPUT_LENGTH+=length;length as i64
    });
    let ui_revision=crate::ui::native_uikit::presentation().revision;
    if ui_revision!=APP_UI_REVISION {APP_UI_REVISION=ui_revision;console.redraw();}
    if APP_OUTPUT_LENGTH>0 {
        for &byte in &(&*(&raw const APP_OUTPUT))[..APP_OUTPUT_LENGTH] {
            if byte==b'\r'{continue;}
            if !APP_LINE_OPEN {console.output.write_line(b"");APP_LINE_OPEN=true;}
            if byte==b'\n'{APP_LINE_OPEN=false;continue;}
            let row=console.output.count-1;
            if console.output.lengths[row]==LINE_CAPACITY {console.output.write_line(b"");}
            let row=console.output.count-1;let column=console.output.lengths[row];
            console.output.lines[row][column]=if byte.is_ascii_graphic()||byte==b' '{byte}else{b' '};
            console.output.lengths[row]+=1;
        }
        INFINITY_NATIVE_APP_SNAPSHOT[7]=INFINITY_NATIVE_APP_SNAPSHOT[7].saturating_add(APP_OUTPUT_LENGTH as u64);
        APP_OUTPUT_LENGTH=0;console.redraw();
    }
    if state==State::Runnable {INFINITY_NATIVE_APP_SNAPSHOT[6]=u64::from((&*(&raw const APP_INPUT)).waiting);}
    if state==State::Runnable{return;}
    let ui_was_active=crate::ui::native_uikit::presentation().active;
    let (owner,task)=core::ptr::addr_of_mut!(APP).replace(None).unwrap();APP_ACTIVE=false;
    crate::runtime::ai::application_host::detach();
    let released=task.release(crate::runtime::native_memory::memory_mut()).is_ok();
    crate::ui::native_uikit::close();APP_UI_REVISION=0;
    if ui_was_active {
        console.command_window.visible=false;
        if console.desktop_app==DesktopAppKind::CommandWindow {console.desktop_app=DesktopAppKind::None;}
    }
    INFINITY_NATIVE_APP_SNAPSHOT[3]=if matches!(state,State::Exited(_))&&released{3}else{4};
    INFINITY_NATIVE_APP_SNAPSHOT[6]=if let State::Exited(code)=state{code as u64}else{u64::MAX};
    if owner==SecurityIdentity(console.current_session.0) {
        match state {
            State::Exited(code) if released=>console.output.write_number(b"Program exit status: ",code as u64),
            State::Cancelled=>console.output.write_line(b"Program cancelled."),
            _=>console.output.write_line(b"Program stopped safely (fault, deadline, or resource limit)."),
        }
        console.redraw();
    }
}}

// ------------------------=
// FUNC: native_interaction_syscall
// DESC: Copies bounded interaction packets across the process boundary without trusting application pointers or identity.
// ------------------=
fn native_interaction_syscall<T:crate::runtime::native_memory::PageTable>(tables:&mut T,op:u64,args:[u64;6],session:[u8;16])->i64 {
    use crate::runtime::native_memory::{copy_from_user,copy_to_user};
    let length=match op {0x1150=>2072,0x1151=>192,0x1152=>8,0x1153=>72,0x1154=>16,0x1155=>416,0x1156=>592,_=>return -3};
    if args[1]!=length as u64 {return -3;}
    let mut input=[0u8;2072];
    if copy_from_user(tables,args[0],&mut input[..length]).is_err(){return -14;}
    // Validate the complete destination before consuming a deliver-once request.
    if matches!(op,0x1153|0x1156) && copy_to_user(tables,args[0],&input[..length]).is_err(){return -14;}
    let mut output=[0u8;592];
    match crate::runtime::ai::application_host::syscall(op,session,&input[..length],&mut output,crate::runtime::ai::qwen::workers::clock_ns()) {
        Ok(written)=>{
            if written>0 && copy_to_user(tables,args[0],&output[..written]).is_err(){return -14;}
            written as i64
        },
        Err(crate::runtime::ai::application::Error::Denied)=>-13,
        Err(crate::runtime::ai::application::Error::Busy)=>-16,
        Err(crate::runtime::ai::application::Error::Stale)=>-116,
        Err(_)=>-3,
    }
}

// ------------------------=
// FUNC: native_uikit_syscall
// DESC: Decodes bounded declarative UI Kit packets from the active isolated application.
// ------------------=
fn native_uikit_syscall<T:crate::runtime::native_memory::PageTable>(tables:&mut T,op:u64,args:[u64;6])->i64 {
    use crate::runtime::native_memory::{copy_from_user,copy_to_user};
    if op==0x1140 {
        if args[1] < 64 {return -3;}
        let mut bytes=[0u8;64];
        if copy_from_user(tables,args[0],&mut bytes).is_err(){return -14;}
        if read_u32(&bytes,0)!=1 {return -3;}
        let width=read_u32(&bytes,4);let height=read_u32(&bytes,8);
        let length=read_u32(&bytes,12) as usize;
        if length==0||length>48{return -3;}
        return if crate::ui::native_uikit::open(width,height,&bytes[16..16+length]){0}else{-3};
    }
    if op==0x1141 {
        if args[1] < 64 {return -3;}
        let mut bytes=[0u8;64];
        if copy_from_user(tables,args[0],&mut bytes).is_err(){return -14;}
        if read_u32(&bytes,0)!=1 {return -3;}
        let kind=match read_u32(&bytes,8){
            1=>crate::ui::native_uikit::ElementKind::Label,
            2=>crate::ui::native_uikit::ElementKind::Display,
            3=>crate::ui::native_uikit::ElementKind::Button,
            _=>return -3,
        };
        let length=read_u32(&bytes,28) as usize;
        if length>32{return -3;}
        let mut label=[0u8;32];label[..length].copy_from_slice(&bytes[32..32+length]);
        let element=crate::ui::native_uikit::Element{id:read_u32(&bytes,4),kind,
            x:read_u32(&bytes,12),y:read_u32(&bytes,16),width:read_u32(&bytes,20),
            height:read_u32(&bytes,24),label,label_length:length};
        return if crate::ui::native_uikit::upsert(element){0}else{-3};
    }
    if op==0x1142 {
        if args[1] < 16 {return -3;}
        let Some(event)=crate::ui::native_uikit::poll() else{return 0;};
        let mut bytes=[0u8;16];bytes[..4].copy_from_slice(&1u32.to_le_bytes());
        bytes[4..8].copy_from_slice(&event.kind.to_le_bytes());
        bytes[8..12].copy_from_slice(&event.id.to_le_bytes());
        bytes[12..].copy_from_slice(&event.value.to_le_bytes());
        return if copy_to_user(tables,args[0],&bytes).is_ok(){1}else{-14};
    }
    -88
}

// ------------------------=
// FUNC: read_u32
// DESC: Reads one aligned-independent little-endian field from a validated UI Kit packet.
// ------------------=
fn read_u32(bytes:&[u8],offset:usize)->u32 {
    u32::from_le_bytes(bytes[offset..offset+4].try_into().unwrap())
}

// ------------------------=
// FUNC: activate_uikit
// DESC: Sends one content-local pointer activation to the running native UI application.
// ------------------=
pub(super) fn activate_uikit(x:u32,y:u32,width:u32,height:u32)->bool {
    let presentation=crate::ui::native_uikit::presentation();
    if !presentation.active||width==0||height==0{return false;}
    crate::ui::native_uikit::activate(x.saturating_mul(presentation.width)/width,
        y.saturating_mul(presentation.height)/height)
}

// ------------------------=
// FUNC: close_uikit
// DESC: Requests graceful native UI closure before the scheduler cancellation fallback.
// ------------------=
pub(super) fn close_uikit()->bool {
    crate::ui::native_uikit::request_close()
}
