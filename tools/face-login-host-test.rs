//! Production login coordinator with deterministic hardware/worker boundaries.
//! This proves dispatch, retry and cancellation, not physical-camera recognition.
use std::sync::atomic::{AtomicBool,AtomicU64,AtomicUsize,Ordering};
static CAMERA:AtomicBool=AtomicBool::new(false);
static NOW:AtomicU64=AtomicU64::new(1_000_000);
static STARTS:AtomicUsize=AtomicUsize::new(0);
static FAIL:AtomicBool=AtomicBool::new(true);
mod identity {#[derive(Clone,Copy,PartialEq,Eq)] pub struct StableId(pub [u8;16]);}
mod execution {pub struct SecurityIdentity(pub [u8;16]);}
#[path="../kernel/runtime/face_login.rs"] mod face_login;
#[path="../kernel/runtime/face_setup.rs"] mod face_setup;
#[path="../kernel/runtime/face_setup_host.rs"] mod face_setup_host;
mod drivers {pub mod camera {
    // ------------------------=
    // FUNC: available
    // DESC: Supplies controllable camera availability at the hardware boundary.
    // ------------------=
    pub fn available()->bool {crate::CAMERA.load(crate::Ordering::Relaxed)}
}}
mod ai {pub mod qwen {pub mod workers {
    // ------------------------=
    // FUNC: clock_ns
    // DESC: Supplies deterministic monotonic time to the production retry policy.
    // ------------------=
    pub fn clock_ns()->u64 {crate::NOW.load(crate::Ordering::Relaxed)}
}}}
mod native_compile_job {
    // ------------------------=
    // FUNC: platform
    // DESC: Supplies an available worker platform without booting a VM.
    // ------------------=
    pub fn platform()->Option<((),())> {Some(((),()))}
}
mod native_memory {
    // ------------------------=
    // FUNC: memory_mut
    // DESC: Represents private worker memory at the injected launch boundary.
    // ------------------=
    pub fn memory_mut() {}
}
mod native_face_job {
    use super::*;
    pub struct Job {pub setup:face_setup::Setup, cancelled:bool}
    impl Job {
        // ------------------------=
        // FUNC: new
        // DESC: Creates an idle fake hardware job with the real protocol state.
        // ------------------=
        pub const fn new()->Self {Self{setup:face_setup::Setup::new(),cancelled:false}}
        // ------------------------=
        // FUNC: start_login
        // DESC: Records actual launch requests and injects an enrollment failure or pending login.
        // ------------------=
        pub fn start_login(&mut self,_:&[u8],_:[&'static[u8];2],_:execution::SecurityIdentity,
            worker:u64,_:identity::StableId,digest:[u8;32],now:u64,_:(),_:())->Result<(),face_login::StartError> {
            STARTS.fetch_add(1,Ordering::Relaxed);
            if FAIL.load(Ordering::Relaxed) {return Err(face_login::StartError::EnrollmentUnavailable);}
            self.cancelled=false;
            self.setup.begin_login(worker,digest,&[1;face_setup::wire::ENROLLMENT_BYTES],now/1_000_000).unwrap();Ok(())
        }
        // ------------------------=
        // FUNC: start_setup
        // DESC: Rejects unrelated enrollment requests in the login-only harness.
        // ------------------=
        pub fn start_setup(&mut self,_:&[u8],_:[&'static[u8];2],_:execution::SecurityIdentity,
            _:u64,_:identity::StableId,_:&[u8],_:[u8;32],_:&str,_:u64,_:(),_:())->Result<(),()> {Err(())}
        // ------------------------=
        // FUNC: poll
        // DESC: Advances the real authorization-cancellation boundary without producing identity proof.
        // ------------------=
        pub fn poll(&mut self,authorized:bool,_:u64,_:())->Result<(),()> {if !authorized {self.cancel();}Ok(())}
        // ------------------------=
        // FUNC: cancel
        // DESC: Marks a dismissed login terminal and clears protocol state.
        // ------------------=
        pub fn cancel(&mut self) {self.cancelled=true;self.setup.cancel();}
        // ------------------------=
        // FUNC: terminal
        // DESC: Exposes the hardware stub terminal transition.
        // ------------------=
        pub fn terminal(&self)->Option<u8> {self.cancelled.then_some(1)}
        // ------------------------=
        // FUNC: take_login_proof
        // DESC: Never manufactures biometric success in a hardware-free test.
        // ------------------=
        pub fn take_login_proof(&mut self)->Option<face_login::Proof> {None}
    }
}
// ------------------------=
// FUNC: click_dispatch_retry_and_account_cancellation
// DESC: Exercises real startup dispatch, immediate retry after failed preflight, duplicate-click suppression, cooldown and account cancellation.
// ------------------=
#[test]
fn click_dispatch_retry_and_account_cancellation() {
    let account=identity::StableId([7;16]);
    assert!(!face_setup_host::start_login(account));
    assert_eq!(STARTS.load(Ordering::Relaxed),0);
    CAMERA.store(true,Ordering::Relaxed);
    assert!(!face_setup_host::start_login(account));
    assert_eq!(STARTS.load(Ordering::Relaxed),1);
    FAIL.store(false,Ordering::Relaxed);
    assert!(face_setup_host::start_login(account));
    assert!(face_setup_host::login_busy());
    assert_eq!(STARTS.load(Ordering::Relaxed),2);
    assert!(face_setup_host::start_login(account));
    assert!(!face_setup_host::start_login(identity::StableId([8;16])));
    assert_eq!(STARTS.load(Ordering::Relaxed),2);
    assert!(face_setup_host::poll_login(Some(account)).is_none());
    assert!(face_setup_host::login_busy());
    assert!(face_setup_host::poll_login(None).is_none());
    assert!(!face_setup_host::login_busy());
    assert!(!face_setup_host::start_login(account));
    NOW.store(16_000_000_000,Ordering::Relaxed);
    assert!(face_setup_host::start_login(account));
    assert_eq!(STARTS.load(Ordering::Relaxed),3);
}
