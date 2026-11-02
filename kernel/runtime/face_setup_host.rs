//! BSP-owned setup bridge. Models, worker and notices are identical in live and
//! installed kernels; no writable model path or user-selected executable exists.
use super::{native_face_job::Job,face_setup::{Setup,Stage,Error},identity::StableId,execution::SecurityIdentity};
#[cfg(all(not(test),target_arch="aarch64"))]
static WORKER:&[u8]=include_bytes!("../../build/face/aarch64/face-worker.elf");
#[cfg(all(not(test),target_arch="x86_64"))]
static WORKER:&[u8]=include_bytes!("../../build/face/x86_64/face-worker.elf");
#[cfg(not(test))]
static DETECTOR:&[u8]=include_bytes!("../../build/face/models/yunet.iface");
#[cfg(not(test))]
static RECOGNIZER:&[u8]=include_bytes!("../../build/face/models/sface.iface");
// Coordinator tests inject a launch boundary and never execute these markers.
#[cfg(test)]
static WORKER:&[u8]=&[0;64];
#[cfg(test)]
static DETECTOR:&[u8]=&[1;64];
#[cfg(test)]
static RECOGNIZER:&[u8]=&[2;64];
#[cfg(not(test))]
#[used]
static YUNET_LICENSE:&[u8]=include_bytes!("../../model-cache/face/licenses/YuNet-LICENSE");
#[cfg(not(test))]
#[used]
static SFACE_LICENSE:&[u8]=include_bytes!("../../model-cache/face/licenses/SFace-LICENSE");
#[cfg(not(test))]
#[used]
static JPEG_NOTICE:&[u8]=include_bytes!("../../build/face/native-deps/stb_image.h");
static mut JOB:Job=Job::new();
static mut GENERATION:u64=0;
static mut PAGE_VISIBLE:bool=false;
static mut LOGIN_JOB:Job=Job::new();
static mut LOGIN_ACCOUNT:Option<StableId>=None;
static mut LOGIN_AFTER:u64=0;
static mut LOGIN_FAILED:bool=false;
static mut LOGIN_START_ERROR:Option<super::face_login::StartError>=None;

// ------------------------=
// FUNC: start_login
// DESC: Starts an explicit pre-session comparison with a global retry interval and immutable packaged worker/models.
// ------------------=
pub fn start_login(account:StableId)->bool {unsafe {
    use super::face_login::StartError;
    if let Some(active)=LOGIN_ACCOUNT {return active==account;}
    LOGIN_FAILED=true;LOGIN_START_ERROR=None;
    let now=super::ai::qwen::workers::clock_ns();
    if now<LOGIN_AFTER {LOGIN_START_ERROR=Some(StartError::Busy);return false;}
    if !crate::drivers::camera::available() {LOGIN_START_ERROR=Some(StartError::CameraUnavailable);return false;}
    let Some((_,controller))=super::native_compile_job::platform() else{LOGIN_START_ERROR=Some(StartError::WorkerUnavailable);return false;};
    let Some(generation)=GENERATION.checked_add(1) else{return false;};GENERATION=generation;
    let mut owner=*b"FACELOG\0\0\0\0\0\0\0\0\0";owner[8..].copy_from_slice(&generation.to_le_bytes());
    let mut digest=[0;32];digest.copy_from_slice(&RECOGNIZER[8..40]);
    if let Err(error)=(&mut *core::ptr::addr_of_mut!(LOGIN_JOB)).start_login(WORKER,[DETECTOR,RECOGNIZER],SecurityIdentity(owner),generation,account,digest,now,
        super::native_memory::memory_mut(),controller) {LOGIN_START_ERROR=Some(error);return false;}
    // Rate-limit real comparisons, not camera/device/startup failures.
    LOGIN_AFTER=now.saturating_add(15_000_000_000);
    LOGIN_ACCOUNT=Some(account);LOGIN_FAILED=false;true
}}
// ------------------------=
// FUNC: poll_login
// DESC: Cancels on account/page changes and returns a successful result once, only after clean worker exit; scrubbing continues after dismissal.
// ------------------=
pub fn poll_login(selected:Option<StableId>)->Option<super::face_login::Proof> {unsafe {
    let job=&mut *core::ptr::addr_of_mut!(LOGIN_JOB);
    let authorized=LOGIN_ACCOUNT.is_some() && LOGIN_ACCOUNT==selected;
    if LOGIN_ACCOUNT.is_some() && !authorized {job.cancel();LOGIN_ACCOUNT=None;}
    if job.poll(authorized,super::ai::qwen::workers::clock_ns(),super::native_memory::memory_mut()).is_err() {job.cancel();LOGIN_ACCOUNT=None;}
    let proof=job.take_login_proof();
    if job.terminal().is_some() {
        if LOGIN_ACCOUNT.is_some() && proof.is_none() {LOGIN_FAILED=true;}
        LOGIN_ACCOUNT=None;
    }
    proof
}}
// ------------------------=
// FUNC: login_busy
// DESC: Exposes pending comparison state without exposing scores or biometric data.
// ------------------=
pub fn login_busy()->bool {unsafe {(*core::ptr::addr_of!(LOGIN_ACCOUNT)).is_some()}}
// ------------------------=
// FUNC: login_caption
// DESC: Gives visible pending/retry feedback without exposing account existence, embeddings or comparison scores.
// ------------------=
pub fn login_caption()->&'static [u8] {
    use super::face_login::StartError;
    if login_busy() {b"Look at the camera - comparing..."}
    else if let Some(error)=unsafe {LOGIN_START_ERROR} {match error {
        StartError::Busy=>b"Please wait - facial sign-in is resetting",
        StartError::CameraUnavailable=>b"Camera unavailable - connect and retry",
        StartError::Disabled=>b"Facial sign-in disabled in Settings",
        StartError::EnrollmentUnavailable=>b"Face not enrolled - sign in with password",
        StartError::WorkerUnavailable=>b"Face service unavailable - retry",
    }}
    else if unsafe {LOGIN_FAILED} {b"Not verified - Retry Facial Sign-in"}
    else {b"Sign In with Facial Recognition"}
}

// ------------------------=
// FUNC: sync_page
// DESC: Starts the authorized preview once on page entry and revokes it immediately on page exit; failures require an explicit retry.
// ------------------=
pub fn sync_page(visible:bool,account:StableId,secret:&[u8],name:&str)->bool {unsafe {
    let entered=visible && !PAGE_VISIBLE;
    if !visible && PAGE_VISIBLE {cancel();}
    PAGE_VISIBLE=visible;
    entered && !start(account,secret,name)
}}

// ------------------------=
// FUNC: available
// DESC: Reflects actual camera and process-platform availability without starting capture.
// ------------------=
pub fn available()->bool {crate::drivers::camera::available() && unsafe {super::native_compile_job::platform().is_some()}}
// ------------------------=
// FUNC: view
// DESC: Borrows setup state synchronously on the BSP for rendering without copying camera pixels into unrelated application state.
// ------------------=
pub fn view<R>(read:impl FnOnce(&Setup)->R)->R {unsafe {read(&(*core::ptr::addr_of!(JOB)).setup)}}
// ------------------------=
// FUNC: start
// DESC: Starts the fixed isolated worker after password authorization; verified enrollment is saved only after the explicit review action.
// ------------------=
pub fn start(account:StableId,secret:&[u8],name:&str)->bool {unsafe {
    if !available() {return false;}
    let Some((_,controller))=super::native_compile_job::platform() else{return false;};
    let memory=super::native_memory::memory_mut();
    let Some(generation)=(*core::ptr::addr_of!(GENERATION)).checked_add(1) else{return false;};
    GENERATION=generation;
    let mut owner=*b"FACESET\0\0\0\0\0\0\0\0\0";
    owner[8..].copy_from_slice(&generation.to_le_bytes());
    let mut digest=[0u8;32];
    if RECOGNIZER.len()<40 {return false;}
    digest.copy_from_slice(&RECOGNIZER[8..40]);
    (&mut *core::ptr::addr_of_mut!(JOB)).start_setup(WORKER,[DETECTOR,RECOGNIZER],SecurityIdentity(owner),
        generation,account,secret,digest,name,super::ai::qwen::workers::clock_ns(),memory,controller).is_ok()
}}
// ------------------------=
// FUNC: capture
// DESC: Dispatches one explicit photograph request from the setup controller.
// ------------------=
pub fn capture()->Result<(),Error> {unsafe {(&mut *core::ptr::addr_of_mut!(JOB)).setup.capture(super::ai::qwen::workers::clock_ns()/1_000_000)}}
// ------------------------=
// FUNC: verify
// DESC: Dispatches a fresh verification phase after review, never a login request.
// ------------------=
pub fn verify()->Result<(),Error> {unsafe {(&mut *core::ptr::addr_of_mut!(JOB)).setup.verify(super::ai::qwen::workers::clock_ns()/1_000_000)}}
// ------------------------=
// FUNC: cancel
// DESC: Immediately revokes camera authority on Back, Skip or setup dismissal.
// ------------------=
pub fn cancel() {unsafe {(&mut *core::ptr::addr_of_mut!(JOB)).cancel();}}
// ------------------------=
// FUNC: poll
// DESC: Advances one bounded worker or scrub visit and requests repaint only when visible progress changes.
// ------------------=
pub fn poll(on_setup_page:bool)->bool {unsafe {
    let job=&mut *core::ptr::addr_of_mut!(JOB);
    let before=(job.setup.stage(),job.setup.pending(),job.setup.rejection(),job.setup.checking_photo(),job.setup.preview_revision(),job.setup.preview().is_some(),job.terminal());
    if !on_setup_page && !matches!(job.setup.stage(),Stage::Idle|Stage::Cancelled|Stage::Passed) {job.cancel();}
    if job.poll(on_setup_page,super::ai::qwen::workers::clock_ns(),super::native_memory::memory_mut()).is_err() {job.cancel();}
    before!=(job.setup.stage(),job.setup.pending(),job.setup.rejection(),job.setup.checking_photo(),job.setup.preview_revision(),job.setup.preview().is_some(),job.terminal())
}}
