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
// Separate setup authorization is not a desktop session or an unlock token.
static mut SETUP_AUTH:Option<(SecurityIdentity,crate::runtime::identity::StableId,u64)>=None;

// ------------------------=
// FUNC: available
// DESC: Reports whether the native capture adapter exists without opening the camera or creating authority.
// ------------------=
pub fn available()->bool {unsafe {(*core::ptr::addr_of!(EXCHANGE)).is_some()}}

// ------------------------=
// FUNC: active
// DESC: Checks the authenticated session on every privileged operation, including capture when the application is not running.
// ------------------=
fn active(owner:SecurityIdentity)->bool {
    let setup=unsafe {*core::ptr::addr_of!(SETUP_AUTH)};
    if let Some((held,account,deadline))=setup {
        if held==owner {
            let now=crate::runtime::ai::qwen::workers::clock_ns()/1_000_000;
            return now<deadline && crate::runtime::with_runtime(|runtime|
                runtime.identity.onboarding_state()==crate::runtime::identity::OnboardingState::InProgress
                && runtime.identity.user(account).is_some_and(|user|user.state==crate::runtime::identity::UserState::Active)
            ).unwrap_or(false);
        }
    }
    crate::runtime::with_runtime(|runtime|(0..crate::runtime::identity::MAX_SESSIONS)
        .filter_map(|index|runtime.identity.session_nth(index))
        .any(|session|session.id.0==owner.0 && session.state==crate::runtime::identity::SessionState::Active)).unwrap_or(false)
}
// ------------------------=
// FUNC: setup_authorized
// DESC: Rechecks the exact password-authorized account and worker binding before private enrollment transfer or persistence.
// ------------------=
pub(crate) fn setup_authorized(owner:SecurityIdentity,account:crate::runtime::identity::StableId)->bool {
    (unsafe {(*core::ptr::addr_of!(SETUP_AUTH)).is_some_and(|(held,user,_)|held==owner && user==account)})
        && active(owner)
}
// ------------------------=
// FUNC: authorize_setup
// DESC: Verifies an existing password without creating a session and scopes camera authority to one short-lived setup worker.
// ------------------=
pub(crate) fn authorize_setup(owner:SecurityIdentity,account:crate::runtime::identity::StableId,secret:&[u8])->bool {unsafe {
    if owner.0==[0;16] || (*core::ptr::addr_of!(SETUP_AUTH)).is_some()
        || (*core::ptr::addr_of!(LEASE)).is_some() || (*core::ptr::addr_of!(EXCHANGE)).is_none() {return false;}
    let now=crate::runtime::ai::qwen::workers::clock_ns()/1_000_000;
    let allowed=crate::runtime::with_runtime(|runtime|
        runtime.identity.onboarding_state()==crate::runtime::identity::OnboardingState::InProgress
        && runtime.identity.user(account).is_some_and(|user|user.state==crate::runtime::identity::UserState::Active)
        && runtime.identity.authenticate(account,secret,now).is_ok()).unwrap_or(false);
    if allowed {SETUP_AUTH=Some((owner,account,now.saturating_add(240_000)));}
    allowed
}}
// ------------------------=
// FUNC: revoke_setup
// DESC: Removes setup authority and revokes capture on cancellation, completion, page exit, or worker failure.
// ------------------=
pub(crate) fn revoke_setup(owner:SecurityIdentity) {unsafe {
    if (*core::ptr::addr_of!(SETUP_AUTH)).is_some_and(|(held,_,_)|held==owner) {
        SETUP_AUTH=None;cancel(owner);
    }
}}
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
    if let Some((owner,_,_))=*core::ptr::addr_of!(SETUP_AUTH) {if !active(owner) {revoke_setup(owner);}}
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
    let request_deadline=if operation==0x1260 {deadline.min(now.saturating_add(crate::runtime::camera::MAX_CAPTURE_SECONDS))}else{deadline};
    let Ok(request)=IopMessage::request(op,1,owner,cap,request_deadline,0,&[]) else{return -22;};
    let Some(service)=(&mut *core::ptr::addr_of_mut!(SERVICE)).as_mut() else{return -19;};
    let result=crate::runtime::with_runtime(|runtime|service.dispatch(&request,owner,&runtime.capabilities,now,args,tables));
    if let Some(Err(error))=result {
        let diagnostic:&[u8]=match error {
            Error::Memory=>b"[camera] delivery failed: private destination mapping\n",
            Error::Expired=>b"[camera] delivery failed: capture deadline\n",
            Error::Denied=>b"[camera] delivery failed: authority revoked\n",
            Error::Invalid=>b"[camera] delivery failed: invalid request\n",
            _=>b"",
        };
        if !diagnostic.is_empty(){crate::output_text(diagnostic);}
    }
    if operation==0x1260 && matches!(result,Some(Ok(_))) {LEASE=Some((owner,cap,request_deadline));}
    match result {Some(Ok(count))=>count as i64,Some(Err(Error::Unavailable))=>-11,Some(Err(Error::Memory))=>-14,
        Some(Err(Error::Busy))=>-16,Some(Err(Error::Stale))=>-116,Some(Err(Error::Expired))=>-110,
        Some(Err(Error::Denied))=>-13,_=>-22}
}}
