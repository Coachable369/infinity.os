//! BSP-owned setup bridge. Models, worker and notices are identical in live and
//! installed kernels; no writable model path or user-selected executable exists.
use super::{native_face_job::Job,face_setup::{Setup,Stage,Error},identity::StableId,execution::SecurityIdentity};
#[cfg(target_arch="aarch64")]
static WORKER:&[u8]=include_bytes!("../../build/face/aarch64/face-worker.elf");
#[cfg(target_arch="x86_64")]
static WORKER:&[u8]=include_bytes!("../../build/face/x86_64/face-worker.elf");
static DETECTOR:&[u8]=include_bytes!("../../build/face/models/yunet.iface");
static RECOGNIZER:&[u8]=include_bytes!("../../build/face/models/sface.iface");
#[used]
static YUNET_LICENSE:&[u8]=include_bytes!("../../model-cache/face/licenses/YuNet-LICENSE");
#[used]
static SFACE_LICENSE:&[u8]=include_bytes!("../../model-cache/face/licenses/SFace-LICENSE");
#[used]
static JPEG_NOTICE:&[u8]=include_bytes!("../../build/face/native-deps/stb_image.h");
static mut JOB:Job=Job::new();
static mut GENERATION:u64=0;
static mut PAGE_VISIBLE:bool=false;

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
// DESC: Starts the fixed isolated worker after account-password authorization; no persistent facial credential is created.
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
    let before=(job.setup.stage(),job.setup.pending(),job.setup.preview_revision(),job.setup.preview().is_some(),job.terminal());
    if !on_setup_page && !matches!(job.setup.stage(),Stage::Idle|Stage::Cancelled|Stage::Passed) {job.cancel();}
    if job.poll(on_setup_page,super::ai::qwen::workers::clock_ns(),super::native_memory::memory_mut()).is_err() {job.cancel();}
    before!=(job.setup.stage(),job.setup.pending(),job.setup.preview_revision(),job.setup.preview().is_some(),job.terminal())
}}
