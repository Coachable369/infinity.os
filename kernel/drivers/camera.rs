//! Serialized native-camera service. Only authenticated application dispatch may
//! copy frames; the firmware entrypoint remains kernel-private.
use crate::runtime::{camera::{MAX_FRAME_BYTES,Error,transport::Transport},camera_uvc::{Uvc,Exchange},
    execution::SecurityIdentity,capability::CapabilityType,iop::{IopMessage,OperationId},native_memory::PageTable};
static mut EXCHANGE:Option<Exchange>=None;
static mut FRAME:[u8;MAX_FRAME_BYTES]=[0;MAX_FRAME_BYTES];
static mut SLOT:[u8;MAX_FRAME_BYTES]=[0;MAX_FRAME_BYTES];
static mut SCRATCH:[u8;MAX_FRAME_BYTES]=[0;MAX_FRAME_BYTES];
static mut PACKET:[u8;65536]=[0;65536];
static mut SERVICE:Option<Transport<'static,Uvc<'static>>>=None;
static mut LEASE:Option<(SecurityIdentity,u64,u64)>=None;

// ------------------------=
// FUNC: active
// DESC: Checks the authenticated session on every privileged operation, including capture when the application is not running.
// ------------------=
fn active(owner:SecurityIdentity)->bool {
    crate::runtime::with_runtime(|runtime|(0..crate::runtime::identity::MAX_SESSIONS)
        .filter_map(|index|runtime.identity.session_nth(index))
        .any(|session|session.id.0==owner.0 && session.state==crate::runtime::identity::SessionState::Active)).unwrap_or(false)
}
// ------------------------=
// FUNC: initialize
// DESC: Installs a boot-provided privileged adapter only on targets retaining firmware services.
// ------------------=
pub unsafe fn initialize(address:usize) {
    #[cfg(target_arch="aarch64")]
    if address!=0 && (*core::ptr::addr_of!(EXCHANGE)).is_none() {
        EXCHANGE=Some(core::mem::transmute::<usize,Exchange>(address));
    }
    #[cfg(not(target_arch="aarch64"))]
    let _=address;
}
// ------------------------=
// FUNC: grant
// DESC: Opens a short lease after an explicit authenticated console action; never grants camera access implicitly to applications.
// ------------------=
pub fn grant(owner:SecurityIdentity)->bool {unsafe {
    if !active(owner) || (*core::ptr::addr_of!(EXCHANGE)).is_none() || (*core::ptr::addr_of!(LEASE)).is_some() {return false;}
    if (&*core::ptr::addr_of!(SERVICE)).as_ref().is_some_and(|service|service.state()!=crate::runtime::camera::transport::State::Idle) {return false;}
    let now=crate::runtime::ai::qwen::workers::clock_ns()/1_000_000_000;
    let deadline=now.saturating_add(30);
    let cap=crate::runtime::with_runtime(|runtime|runtime.capabilities.grant(CapabilityType::CameraCapture,0,1,0,owner,owner,Some(deadline),0).ok()).flatten();
    if let Some(cap)=cap {LEASE=Some((owner,cap,deadline));true}else{false}
}}
// ------------------------=
// FUNC: cancel
// DESC: Revokes delivery immediately and leaves the transport alive until its bounded shutdown completes.
// ------------------=
pub fn cancel(owner:SecurityIdentity) {unsafe {
    if let Some((held,cap,_))=*core::ptr::addr_of!(LEASE) {
        if held!=owner {return;}
        crate::runtime::with_runtime(|runtime|{let _=runtime.capabilities.revoke(cap);let _=runtime.capabilities.retire_leaf(cap,owner);});
        LEASE=None;
        if let Some(service)=(&mut *core::ptr::addr_of_mut!(SERVICE)).as_mut() {service.cancel();}
    }
}}
// ------------------------=
// FUNC: poll
// DESC: Advances one bounded device operation and expires authority independently of application polling.
// ------------------=
pub fn poll() {unsafe {
    let now=crate::runtime::ai::qwen::workers::clock_ns();
    if let Some((owner,_,deadline))=*core::ptr::addr_of!(LEASE) {if !active(owner) || now/1_000_000_000>=deadline {cancel(owner);}}
    if let Some(service)=(&mut *core::ptr::addr_of_mut!(SERVICE)).as_mut() {
        crate::runtime::with_runtime(|runtime|service.tick(&runtime.capabilities,now/1_000_000_000,now/1_000_000,&mut *core::ptr::addr_of_mut!(SCRATCH)));
    }
}}
// ------------------------=
// FUNC: dispatch
// DESC: Derives identity and lease server-side and routes isolated-process camera calls through authenticated IOP.
// ------------------=
pub fn dispatch(owner:SecurityIdentity,tables:&mut impl PageTable,operation:u64,args:[u64;6])->i64 {unsafe {
    let Some((held,cap,deadline))=*core::ptr::addr_of!(LEASE) else{return -13;};
    if held!=owner {return -13;}
    if !active(owner) {cancel(owner);return -13;}
    let now=crate::runtime::ai::qwen::workers::clock_ns()/1_000_000_000;
    if now>=deadline {cancel(owner);return -110;}
    if (*core::ptr::addr_of!(SERVICE)).is_none() {
        let Some(exchange)=*core::ptr::addr_of!(EXCHANGE) else{return -19;};
        let driver=Uvc::new(exchange,&mut *core::ptr::addr_of_mut!(PACKET),&mut *core::ptr::addr_of_mut!(FRAME));
        SERVICE=Transport::new(&mut *core::ptr::addr_of_mut!(SLOT),driver).ok();
    }
    let op=match operation {0x1260=>OperationId::CameraCaptureStart,0x1261=>OperationId::CameraFrameRead,0x1262=>OperationId::CameraCaptureStop,_=>return -38};
    let request_deadline=if operation==0x1260 {deadline.min(now.saturating_add(5))}else{deadline};
    let Ok(request)=IopMessage::request(op,1,owner,cap,request_deadline,0,&[]) else{return -22;};
    let Some(service)=(&mut *core::ptr::addr_of_mut!(SERVICE)).as_mut() else{return -19;};
    let result=crate::runtime::with_runtime(|runtime|service.dispatch(&request,owner,&runtime.capabilities,now,args,tables));
    if operation==0x1260 && matches!(result,Some(Ok(_))) {LEASE=Some((owner,cap,request_deadline));}
    match result {Some(Ok(count))=>count as i64,Some(Err(Error::Unavailable))=>-11,Some(Err(Error::Memory))=>-14,
        Some(Err(Error::Busy))=>-16,Some(Err(Error::Stale))=>-116,Some(Err(Error::Expired))=>-110,
        Some(Err(Error::Denied))=>-13,_=>-22}
}}
