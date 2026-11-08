//! Independent native Python workers for durable administrator-approved automation.
use super::{automation::{self,Catalog,Entry,Error,Kind,State},execution::SecurityIdentity,
    native_compiler_task::Task,native_job_files::File,native_memory::memory_mut,native_process::State as ProcessState};
const SLOTS:usize=4;
const INPUTS:usize=8;
const SOURCE_BYTES:usize=256*1024;
const OUTPUT_BYTES:usize=16*1024;
struct Source{path:automation::Text<96>,bytes:[u8;SOURCE_BYTES],length:usize}
impl Source{const EMPTY:Self=Self{path:automation::Text::empty(),bytes:[0;SOURCE_BYTES],length:0};}
struct Worker{id:u32,owner:[u8;16],task:Option<Task>,sources:[Source;INPUTS],source_count:usize,
    output:[u8;OUTPUT_BYTES],errors:[u8;OUTPUT_BYTES],output_len:usize,error_len:usize,pending_exit:Option<i64>,retry_at:u64,executed:bool}
impl Worker{const EMPTY:Self=Self{id:0,owner:[0;16],task:None,sources:[const{Source::EMPTY};INPUTS],source_count:0,output:[0;OUTPUT_BYTES],errors:[0;OUTPUT_BYTES],output_len:0,error_len:0,pending_exit:None,retry_at:0,executed:false};}
static mut WORKERS:[Worker;SLOTS]=[const{Worker::EMPTY};SLOTS];
static mut NEXT_PROCESS:u32=0x44410001;
static mut NEXT_ADMISSION:u64=0;
static mut LAST_ADMITTED:u32=0;
static mut CURSOR:usize=0;
static mut LAST_PUBLICATION:u64=u64::MAX;
#[no_mangle]
pub static mut INFINITY_AUTOMATION_SNAPSHOT:[u64;200]=[0;200];

// ------------------------=
// FUNC: authorized
// DESC: Rechecks the persistent approving account on every native slice, independent of login sessions.
// ------------------=
fn authorized(owner:[u8;16])->bool{super::with_runtime(|r|r.identity.user(super::identity::StableId(owner)).is_some_and(|u|u.state==super::identity::UserState::Active)&&r.identity.is_administrator(super::identity::StableId(owner))).unwrap_or(false)}
// ------------------------=
// FUNC: commit
// DESC: Persists a staged lifecycle checkpoint before publishing it to command or graphical clients.
// ------------------=
fn commit(candidate:Catalog)->Result<(),Error>{let mut bytes=[0;automation::STATE_BYTES];let n=candidate.encode(&mut bytes)?;crate::storage::automation_state_commit(&bytes[..n]).map_err(|_|Error::Storage)?;super::with_runtime(|r|r.automation=candidate).ok_or(Error::Unavailable)?;Ok(())}
// ------------------------=
// FUNC: snapshot_source
// DESC: Captures exact owned source versions without granting an unattended process ambient object access.
// ------------------=
fn snapshot_source(target:&mut Source,path:&[u8],owner:[u8;16])->Result<(),Error>{
    automation::validate_path(path)?;
    let(metadata,_)=crate::storage::object_inspect_path(path).map_err(|_|Error::Unavailable)?;
    if !super::native_python_scope::readable(metadata.owner.0,owner,metadata.space==crate::storage::object::Space::Personal,path){return Err(Error::Denied);}
    let(object,length)=crate::storage::object_read_path(path,Some(metadata.current_version),&mut target.bytes).map_err(|_|Error::Unavailable)?;
    if object!=metadata.object.id{return Err(Error::Unavailable);}
    target.path=automation::Text::new(path)?;target.length=length;Ok(())
}
// ------------------------=
// FUNC: prepare
// DESC: Allocates a confined native CPython job and snapshots bounded adjacent Python modules before execution.
// ------------------=
unsafe fn prepare(worker:&mut Worker,entry:Entry,now:u64)->Result<u32,Error>{
    worker.id=entry.id;worker.owner=entry.owner;worker.output.fill(0);worker.errors.fill(0);worker.output_len=0;worker.error_len=0;worker.source_count=0;worker.executed=false;
    if !authorized(entry.owner){return Err(Error::Denied);}
    snapshot_source(&mut worker.sources[0],entry.path.bytes(),entry.owner)?;worker.source_count=1;
    let path=entry.path.bytes();let slash=path.iter().rposition(|b|*b==b'/').ok_or(Error::Invalid)?;let directory=if slash==0{b"/".as_slice()}else{&path[..slash]};
    for index in 0..256{let child=crate::storage::namespace_child_nth(directory,index).map_err(|_|Error::Unavailable)?;let Some(child)=child else{break;};let path=&child.path[..child.path_len as usize];if path==entry.path.bytes()||!path.ends_with(b".py"){continue;}
        let(metadata,_)=crate::storage::object_inspect_path(path).map_err(|_|Error::Unavailable)?;
        if !super::native_python_scope::readable(metadata.owner.0,entry.owner,metadata.space==crate::storage::object::Space::Personal,path){continue;}
        if worker.source_count==INPUTS{return Err(Error::Full);}snapshot_source(&mut worker.sources[worker.source_count],path,entry.owner)?;worker.source_count+=1;
    }
    let(kernel,controller)=super::native_compile_job::platform().ok_or(Error::Unavailable)?;
    let secret=super::native_python_entropy::next(&entry.owner).ok_or(Error::Unavailable)?;
    let pid=NEXT_PROCESS;NEXT_PROCESS=pid.checked_add(1).ok_or(Error::Full)?;
    let mut task=Task::start(super::native_python_package::PYTHON,&[b"/system/python",entry.path.bytes()],SecurityIdentity(entry.owner),pid,1,memory_mut(),kernel,controller,
        if entry.kind==Kind::Daemon{u64::MAX}else{now.saturating_add(3_600_000_000_000)}).map_err(|_|Error::Unavailable)?;
    task.set_launch_secret(secret);worker.task=Some(task);worker.id=entry.id;worker.owner=entry.owner;worker.output.fill(0);worker.errors.fill(0);worker.output_len=0;worker.error_len=0;worker.pending_exit=None;worker.retry_at=0;worker.executed=false;Ok(pid)
}
// ------------------------=
// FUNC: finish
// DESC: Reclaims native mappings and retains the exit receipt until its durable completion checkpoint succeeds.
// ------------------=
unsafe fn finish(worker:&mut Worker,exit:i64,utc:u64)->bool{
    if let Some(task)=worker.task.take(){worker.pending_exit=Some(if task.release(memory_mut()).is_ok(){exit}else{-5});}
    else if worker.pending_exit.is_none(){worker.pending_exit=Some(exit);}
    let Some(mut candidate)=super::with_runtime(|r|r.automation)else{return false;};
    if candidate.finished(worker.id,worker.pending_exit.unwrap(),utc).is_err(){return false;}
    if commit(candidate).is_err(){return false;}worker.pending_exit=None;true
}
// ------------------------=
// FUNC: poll
// DESC: Advances one fair native slice and admits a due job only after recording its durable at-most-once claim.
// ------------------=
pub fn poll(now:u64,utc:u64,boot_seconds:u64)->bool{unsafe{
    if !super::with_runtime(|r|r.automation_ready).unwrap_or(false){return false;}
    let mut changed=false;
    let workers=&mut *(&raw mut WORKERS);
    let index=CURSOR%SLOTS;CURSOR=CURSOR.wrapping_add(1);let worker=&mut workers[index];
    if worker.pending_exit.is_some(){if now>=worker.retry_at{changed|=finish(worker,worker.pending_exit.unwrap(),utc);worker.retry_at=now.saturating_add(1_000_000_000);}}
    else if worker.task.is_some(){
        let entry=super::with_runtime(|r|r.automation.get(worker.id)).flatten();
        let allowed=entry.is_some()&&authorized(worker.owner);
        if entry.is_none_or(|e|e.state==State::Killing){changed|=finish(worker,-9,utc);}
        else{
            let task=worker.task.as_mut().unwrap();if entry.is_some_and(|e|e.state==State::Stopping){task.cancel();}
            let files: [File<'_>;INPUTS]=core::array::from_fn(|i|File{path:worker.sources[i].path.bytes(),bytes:&worker.sources[i].bytes[..worker.sources[i].length]});
            let state=task.poll(now,allowed,memory_mut(),b"",&files[..worker.source_count],&mut worker.output,&mut worker.errors);
            worker.executed|=task.execution_id().is_some();
            if let Some((out,err,_))=task.result_lengths(b"",&mut worker.output,&mut worker.errors){worker.output_len=out;worker.error_len=err;}
            if state!=ProcessState::Runnable{let exit=match state{ProcessState::Exited(code)=>code,ProcessState::Cancelled=>-130,ProcessState::Deadline=>-124,_=>-1};changed|=finish(worker,exit,utc);}
        }
    }
    if now>=NEXT_ADMISSION{if let Some(slot_index)=workers.iter().position(|w|w.task.is_none()&&w.pending_exit.is_none()){
        if let Some(entry)=super::with_runtime(|r|r.automation.due_after(utc,boot_seconds,LAST_ADMITTED)).flatten(){
            for(index,previous)in workers.iter_mut().enumerate(){if index!=slot_index&&previous.id==entry.id&&previous.task.is_none()&&previous.pending_exit.is_none(){previous.id=0;previous.executed=false;previous.output_len=0;previous.error_len=0;}}
            LAST_ADMITTED=entry.id;
            let slot=&mut workers[slot_index];
            match prepare(slot,entry,now){
                Ok(pid)=>{let mut candidate=super::with_runtime(|r|r.automation).unwrap();
                    if candidate.started(entry.id,pid,utc).is_ok()&&commit(candidate).is_ok(){changed=true;}
                    else{if let Some(task)=slot.task.take(){let _=task.release(memory_mut());}NEXT_ADMISSION=now.saturating_add(5_000_000_000);}
                },
                Err(error)=>{let mut candidate=super::with_runtime(|r|r.automation).unwrap();let exit=match error{Error::Denied=>-13,Error::Full=>-28,_=>-2};let _=candidate.finished(entry.id,exit,utc);changed|=commit(candidate).is_ok();NEXT_ADMISSION=now.saturating_add(50_000_000);},
            }
        }
    }}
    publish();changed
}}
// ------------------------=
// FUNC: logs
// DESC: Copies the latest retained worker output without executing code or exposing another catalog's private data.
// ------------------=
pub fn logs(id:u32,output:&mut[u8],errors:&mut[u8])->Option<(usize,usize)>{unsafe{let worker=(&*(&raw const WORKERS)).iter().find(|w|w.id==id)?;let a=output.len().min(worker.output_len);let b=errors.len().min(worker.error_len);output[..a].copy_from_slice(&worker.output[..a]);errors[..b].copy_from_slice(&worker.errors[..b]);Some((a,b))}}
// ------------------------=
// FUNC: publish
// DESC: Publishes seqlocked numeric lifecycle evidence without paths, source bytes, account identities or authority.
// ------------------=
fn publish(){let Some(catalog)=super::with_runtime(|r|r.automation)else{return;};unsafe{
    let target=core::ptr::addr_of_mut!(INFINITY_AUTOMATION_SNAPSHOT).cast::<u64>();
    let workers=&*(&raw const WORKERS);let mut signature=catalog.revision;for worker in workers{signature=signature.wrapping_mul(131).wrapping_add(worker.id as u64).wrapping_add(u64::from(worker.executed)).wrapping_add(worker.output_len as u64).wrapping_add(worker.error_len as u64);}
    if LAST_PUBLICATION==signature{return;}LAST_PUBLICATION=signature;
    let sequence=core::ptr::read_volatile(target.add(2)).wrapping_add(2)&!1;core::ptr::write_volatile(target.add(2),sequence|1);
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
    core::ptr::write_volatile(target,0x494e464155544f31);core::ptr::write_volatile(target.add(1),1);core::ptr::write_volatile(target.add(3),catalog.revision);
    let mut count=0;for kind in[Kind::Daemon,Kind::Schedule]{for index in 0..catalog.count(kind){let e=catalog.nth(kind,index).unwrap();let at=8+count*12;let values=[e.id as u64,if kind==Kind::Daemon{0}else{1},match e.state{State::Stopped=>0,State::Queued=>1,State::Running=>2,State::Stopping=>3,State::Killing=>4,State::Completed=>5,State::Failed=>6},e.runs,e.last_exit.unwrap_or(0)as u64,u64::from(e.last_exit.is_some()),e.next_run,u64::from(e.enabled),u64::from(e.process_id),workers.iter().find(|w|w.id==e.id).map_or(0,|w|u64::from(w.executed)),workers.iter().find(|w|w.id==e.id).map_or(0,|w|w.output_len as u64),workers.iter().find(|w|w.id==e.id).map_or(0,|w|w.error_len as u64)];for(i,value)in values.iter().enumerate(){core::ptr::write_volatile(target.add(at+i),*value);}count+=1;}}
    core::ptr::write_volatile(target.add(4),count as u64);core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);core::ptr::write_volatile(target.add(7),sequence);core::ptr::write_volatile(target.add(2),sequence);
}}

// ------------------------=
// FUNC: copy_logs
// DESC: Combines retained stdout and stderr in a bounded presentation buffer without granting execution authority.
// ------------------=
pub fn copy_logs(id:u32,out:&mut[u8])->usize{unsafe{let Some(worker)=(&*(&raw const WORKERS)).iter().find(|w|w.id==id)else{return 0;};let a=out.len().min(worker.output_len);out[..a].copy_from_slice(&worker.output[..a]);let b=(out.len()-a).min(worker.error_len);out[a..a+b].copy_from_slice(&worker.errors[..b]);a+b}}
