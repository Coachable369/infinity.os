#![no_std]
//! Native user-mode worker. All model/JPEG/embedding work remains in private
//! memory; the dispatcher exposes only the bounded setup protocol and camera.
use infinity_face_core::{Binding,Enrollment,Policy,State,protocol as p,recognition::{Recognizer,Scratch},jpeg};
use core::{mem::size_of,panic::PanicInfo};
const HEAP_BASE:usize=0x200000000;
const HEAP_BYTES:usize=192*1024*1024;

// ------------------------=
// FUNC: call
// DESC: Enters the native isolated-process dispatcher using only target-specific instruction adapters.
// ------------------=
fn call(op:u64,args:[u64;6])->i64 {unsafe {
    #[cfg(target_arch="aarch64")]
    {let mut result=args[0];core::arch::asm!("svc #0",in("x8")op,inout("x0")result,
        in("x1")args[1],in("x2")args[2],in("x3")args[3],in("x4")args[4],in("x5")args[5],options(nostack));result as i64}
    #[cfg(target_arch="x86_64")]
    {let mut result=op;core::arch::asm!("int 0x80",inout("rax")result,in("rdi")args[0],in("rsi")args[1],
        in("rdx")args[2],in("rcx")args[3],in("r8")args[4],in("r9")args[5],options(nostack));result as i64}
}}
// ------------------------=
// FUNC: exit
// DESC: Terminates the worker through the native process boundary without returning into an unknown caller.
// ------------------=
fn exit(code:u64)->! {call(0x1000,[code,0,0,0,0,0]);loop {call(0x1001,[0;6]);}}
// ------------------------=
// FUNC: panic
// DESC: Converts unexpected worker failure into process termination, never a kernel panic or successful verification.
// ------------------=
#[panic_handler]
fn panic(_: &PanicInfo<'_>)->! {exit(70)}
// ------------------------=
// FUNC: clock
// DESC: Reads the service monotonic clock and rejects unavailable time.
// ------------------=
fn clock()->Result<u64,u32> {let n=call(p::CLOCK_MS,[0;6]);if n<=0 {Err(100)}else{Ok(n as u64)}}
// ------------------------=
// FUNC: control
// DESC: Retrieves a fully initialized command only through the bound worker dispatcher.
// ------------------=
fn control()->Result<p::Control,u32> {
    let mut c=p::Control::empty();
    let n=call(p::CONTROL_READ,[&mut c as *mut _ as u64,size_of::<p::Control>() as u64,0,0,0,0]);
    if n!=size_of::<p::Control>() as i64 || c.version!=p::VERSION || c.reserved!=0 || c.name_len>64 {Err(101)}else{Ok(c)}
}
// ------------------------=
// FUNC: visit
// DESC: Yields compute and observes cancellation or a superseding command between bounded inference visits.
// ------------------=
fn visit(command:u64)->bool {call(0x1001,[0;6]);control().is_ok_and(|c|c.command_id==command && c.command!=p::CANCEL)}
// ------------------------=
// FUNC: report
// DESC: Reports UI progress only; the wire record has no authentication result or credential field.
// ------------------=
fn report(c:&p::Control,phase:u32,error:u32,state:Option<State>) {
    let phase=if phase==p::FAILED && c.command==p::CAPTURE && matches!(state,Some(State::Photos(_)))
        && matches!(error,204..=214|216..=218) {p::REJECTED}else{phase};
    let (photos,comparisons)=match state {Some(State::Photos(n))=>(n as u32,0),Some(State::Verifying(n))=>(3,n as u32),
        Some(State::ReadyToVerify)=>(3,0),Some(State::Verified)=>(3,3),_=>(0,0)};
    let r=p::Report::new(c.command_id,phase,error,photos,comparisons);
    call(p::REPORT_WRITE,[&r as *const _ as u64,size_of::<p::Report>() as u64,0,0,0,0]);
}
struct Arena {used:usize}
impl Arena {
    // ------------------------=
    // FUNC: bytes
    // DESC: Partitions the fixed user-only heap into disjoint aligned slices; no allocation or pointer is obtained from firmware.
    // ------------------=
    unsafe fn bytes(&mut self,len:usize)->Result<&'static mut [u8],u32> {
        let start=self.used.checked_add(63).ok_or(102u32)?&!63;
        let end=start.checked_add(len).ok_or(102u32)?;
        if end>HEAP_BYTES {return Err(102);}
        self.used=end;Ok(core::slice::from_raw_parts_mut((HEAP_BASE+start) as *mut u8,len))
    }
    // ------------------------=
    // FUNC: model
    // DESC: Loads a pinned model through bounded 64 KiB copies, yielding between chunks, once per worker lifetime.
    // ------------------=
    unsafe fn model(&mut self,id:u64)->Result<&'static [u8],u32> {
        let length=call(p::MODEL_READ,[id,0,0,0,0,0]);
        if length<=64 || length>48*1024*1024 {return Err(103);}
        let data=self.bytes(length as usize)?;
        for offset in (0..data.len()).step_by(64*1024) {
            let n=(data.len()-offset).min(64*1024);
            if call(p::MODEL_READ,[id,offset as u64,data.as_mut_ptr().add(offset) as u64,n as u64,0,0])!=n as i64 {return Err(103);}
            call(0x1001,[0;6]);
        }
        Ok(data)
    }
}
// ------------------------=
// FUNC: binding
// DESC: Binds a workflow to the OS-issued setup identity, worker generation, capture phase and pinned model digest.
// ------------------=
fn binding(c:&p::Control)->Binding {Binding {account:c.account,worker:c.worker,lease:c.lease,model:c.model}}
// ------------------------=
// FUNC: capture
// DESC: Copies a pinned JPEG; preview retains a bounded stream while explicit photos always start a fresh stream.
// ------------------=
fn capture(command:u64,encoded:&mut [u8],stream:&mut Option<u64>,live:bool)->Result<(usize,u64,u64),u32> {
    let begin=clock()?;
    if stream.is_some_and(|opened|!live || begin.saturating_sub(opened)>=8000) {
        call(0x1262,[0;6]);*stream=None;
    }
    let result=(|| {
        while stream.is_none() {
            let status=call(0x1260,[0;6]);
            if status==0 {*stream=Some(clock()?);break;}
            if status!= -11 && status!= -16 {return Err(104);}
            if clock()?.saturating_sub(begin)>5000 || !visit(command) {return Err(105);}
        }
        loop {
            if clock()?.saturating_sub(begin)>10_000 || !visit(command) {return Err(105);}
            let mut meta=[0u64;2];
            let status=call(0x1261,[0,0,0,0,meta.as_mut_ptr() as u64,0]);
            if status== -11 {continue;}
            if status==-110 {return Err(105);}
            if status<0 || meta[0]==0 || meta[1]<4 || meta[1]>encoded.len() as u64 {return Err(104);}
            let length=meta[1] as usize;let mut stale=false;
            for offset in (0..length).step_by(4096) {
                let n=(length-offset).min(4096);
                let status=call(0x1261,[meta[0],offset as u64,unsafe {encoded.as_mut_ptr().add(offset)} as u64,n as u64,0,0]);
                if status== -116 {stale=true;break;}
                if status==-110 {return Err(105);}
                if status!=n as i64 {return Err(104);}
                if !visit(command) {return Err(105);}
            }
            if !stale {return Ok((length,meta[0],clock()?));}
        }
    })();
    if !live || result.is_err() {call(0x1262,[0;6]);*stream=None;}
    result
}
// ------------------------=
// FUNC: preview
// DESC: Sends an actual downsampled camera thumbnail in bounded sequential chunks, never a generated enrollment image.
// ------------------=
fn preview(command:u64,rgb:&[u8])->Result<(),u32> {
    if rgb.len()!=640*480*3 {return Err(109);}
    let mut rows=[0u8;160*3*8];
    for y in 0..120 {
        for x in 0..160 {for channel in 0..3 {
            let mut sum=0u32;
            for dy in 0..4 {for dx in 0..4 {sum+=rgb[((y*4+dy)*640+x*4+dx)*3+channel] as u32;}}
            rows[(y%8)*480+x*3+channel]=(sum/16) as u8;
        }}
        if y%8!=7 {continue;}
        let result=call(p::PREVIEW_WRITE,[command,((y-7)*480) as u64,rows.as_ptr() as u64,rows.len() as u64,0,0]);
        if result!=rows.len() as i64 || !visit(command) {
            for byte in &mut rows {unsafe {core::ptr::write_volatile(byte,0);}}
            return Err(105);
        }
    }
    for byte in &mut rows {unsafe {core::ptr::write_volatile(byte,0);}}
    Ok(())
}
// ------------------------=
// FUNC: export_enrollment
// DESC: Transfers verified views in bounded private chunks and clears each temporary transfer buffer, including cancellation paths.
// ------------------=
fn export_enrollment(command:u64,enrollment:&Enrollment)->Result<(),u32> {
    let mut bytes=[0u8;4096];
    let result=(|| {
        for offset in (0..p::ENROLLMENT_BYTES).step_by(bytes.len()) {
            let count=(p::ENROLLMENT_BYTES-offset).min(bytes.len());
            enrollment.export_verified(offset,&mut bytes[..count]).map_err(|_|113u32)?;
            if call(p::ENROLLMENT_WRITE,[command,offset as u64,bytes.as_ptr() as u64,count as u64,0,0])!=count as i64
                || !visit(command) {return Err(113);}
        }
        Ok(())
    })();
    for byte in &mut bytes {unsafe {core::ptr::write_volatile(byte,0);}}
    result
}
// ------------------------=
// FUNC: work
// DESC: Maintains temporary enrollment across explicit photo commands, runs real inference, and separately verifies a new capture phase.
// ------------------=
unsafe fn work()->Result<(),u32> {
    let mut arena=Arena {used:4096};
    // Start means a real camera check, not minutes of silent model preparation.
    let initial=control()?;
    if initial.command!=p::BEGIN {return Err(108);}
    let encoded=arena.bytes(1_228_800)?;let rgb=arena.bytes(640*480*3)?;let decode=arena.bytes(8*1024*1024)?;
    let mut stream=None;
    let (length,_,_)=capture(initial.command_id,encoded,&mut stream,false)?;
    jpeg::decode(&encoded[..length],rgb,decode).map_err(|_|109u32)?;
    preview(initial.command_id,rgb)?;
    let detector=arena.model(1)?;let recognizer=arena.model(2)?;
    let engine=Recognizer::new(detector,recognizer).map_err(|_|106u32)?;
    let floats=arena.bytes(engine.arena_len().checked_mul(4).ok_or(102u32)?)?;
    let floats=core::slice::from_raw_parts_mut(floats.as_mut_ptr().cast::<f32>(),engine.arena_len());
    let scratch=arena.bytes(size_of::<Scratch>())?;scratch.fill(0);
    let scratch=&mut *scratch.as_mut_ptr().cast::<Scratch>();
    let mut enrollment:Option<Enrollment>=None;let mut seen=0;let mut preview_at=0;
    loop {
        let c=control()?;
        if c.command==p::CANCEL {if let Some(e)=enrollment.as_mut(){e.cancel();}call(0x1262,[0;6]);scratch.clear();report(&c,p::CANCELLED,0,None);return Ok(());}
        if let Some(e)=enrollment.as_mut() {
            if let Err(error)=e.tick(clock()?) {
                report(&c,p::FAILED,200+error as u32,Some(e.state()));
                scratch.clear();return Ok(());
            }
        }
        if c.command_id==seen || c.command_id==0 {
            if enrollment.as_ref().is_some_and(|e|matches!(e.state(),State::Photos(_))) && clock()?>=preview_at {
                let result=capture(c.command_id,encoded,&mut stream,true).and_then(|(length,_,_)| {
                    jpeg::decode(&encoded[..length],rgb,decode).map_err(|_|109u32)?;
                    preview(c.command_id,rgb)
                });
                // Capture/Back may supersede a preview mid-copy. Never fail the
                // new command or publish the old frame into its generation.
                if control()?.command_id!=c.command_id {continue;}
                if let Err(error)=result {report(&c,p::FAILED,error,None);return Ok(());}
                preview_at=clock()?.saturating_add(100);
            }
            call(0x1001,[0;6]);continue;
        }
        seen=c.command_id;
        if c.command==p::BEGIN {
            let name=core::str::from_utf8(&c.name[..c.name_len as usize]).map_err(|_|107u32)?;
            enrollment=Some(Enrollment::new(binding(&c),name,Policy {cosine:0.6,min_laplacian_variance:100},clock()?).map_err(|_|107u32)?);
            report(&c,p::READY,0,enrollment.as_ref().map(Enrollment::state));continue;
        }
        let Some(e)=enrollment.as_mut() else {report(&c,p::FAILED,108,None);continue;};
        if c.command!=p::CAPTURE && c.command!=p::VERIFY {report(&c,p::FAILED,108,Some(e.state()));continue;}
        if (c.command==p::CAPTURE && !matches!(e.state(),State::Photos(_)))
            || (c.command==p::VERIFY && !matches!(e.state(),State::ReadyToVerify|State::Verifying(_))) {
            report(&c,p::FAILED,108,Some(e.state()));continue;
        }
        if c.command==p::VERIFY && e.state()==State::ReadyToVerify {
            if let Err(error)=e.begin_verification(binding(&c),clock()?) {report(&c,p::FAILED,200+error as u32,Some(e.state()));continue;}
        }
        let frames=if let State::Verifying(n)=e.state() {3u8.saturating_sub(n)}else{1};
        for _ in 0..frames {
            report(&c,p::PROCESSING,0,Some(e.state()));
            let (length,sequence,captured)=match capture(c.command_id,encoded,&mut stream,false) {Ok(v)=>v,Err(error)=>{report(&c,p::FAILED,error,Some(e.state()));break;}};
            if jpeg::decode(&encoded[..length],rgb,decode).is_err() {report(&c,p::FAILED,109,Some(e.state()));break;}
            if c.command==p::CAPTURE {
                if let Err(error)=preview(c.command_id,rgb) {report(&c,p::FAILED,error,Some(e.state()));break;}
            }
            let recognition=match engine.recognize(rgb,floats,scratch,||visit(c.command_id)) {
                Ok(v)=>v,Err(error)=>{
                    use infinity_face_core::{recognition::Error as R,vision::Error as V};
                    let code=match error {
                        R::Quality(error)=>200+error as u32,
                        R::Image(V::NoFace)=>206,R::Image(V::MultipleFaces|V::Crowded)=>207,
                        R::Image(V::Pose)=>216,R::Image(V::Landmarks)=>212,
                        R::Cancelled=>105,_=>110,
                    };
                    report(&c,p::FAILED,code,Some(e.state()));break;
                }
            };
            let result=e.observe(&recognition.observation(binding(&c),sequence,captured),clock()?);
            match result {
                Ok(state)=>{
                    if state==State::Verified {
                        if let Err(error)=export_enrollment(c.command_id,e) {report(&c,p::FAILED,error,Some(state));break;}
                    }
                    report(&c,match state {State::ReadyToVerify=>p::REVIEW,State::Verified=>p::PASSED,State::Verifying(_)=>p::VERIFYING,_=>p::READY},0,Some(state));
                },
                Err(error)=>{report(&c,p::FAILED,200+error as u32,Some(e.state()));break;}
            }
        }
        for byte in encoded.iter_mut().chain(rgb.iter_mut()) {core::ptr::write_volatile(byte,0);}
    }
}
// ------------------------=
// FUNC: infinity_app_entry
// DESC: Starts the pinned user-mode verification worker; no host API table or kernel callback address is accepted.
// ------------------=
#[no_mangle]
pub extern "C" fn infinity_app_entry(_:usize)->! {
    let result=unsafe {work()};
    if let Err(error)=result {
        if let Ok(command)=control() {report(&command,p::FAILED,error,None);}
    }
    exit(result.err().unwrap_or(0) as u64)
}
