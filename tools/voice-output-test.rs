//! Runs the production speech-job state machine with deterministic hardware/time seams.
#![allow(dead_code)]
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
static NOW: AtomicU64 = AtomicU64::new(1);
static EXPIRE: AtomicBool = AtomicBool::new(false);
static PLAYED: AtomicUsize = AtomicUsize::new(0);
static mut TASK: Option<unsafe fn()> = None;
mod runtime {
    pub mod execution {
        #[derive(Clone, Copy, PartialEq, Eq)]
        pub struct SecurityIdentity(pub [u8; 16]);
    }
    pub mod capability { pub enum CapabilityType { AudioOutput } }
    pub mod audio {
        pub enum PlaybackState { Playing, Complete, Cancelled, Denied }
    }
    pub struct Caps;
    impl Caps {
        // ------------------------=
        // FUNC: validate
        // DESC: Accepts only the fixture's owner and capability.
        // ------------------=
        pub fn validate(&self, cap: u64, owner: execution::SecurityIdentity, _: capability::CapabilityType,
                        _: u64, _: u64, _: u64, _: u64) -> Result<(), ()> {
            if cap == 1 && owner.0 == [1; 16] { Ok(()) } else { Err(()) }
        }
        // ------------------------=
        // FUNC: retire_leaf
        // DESC: Supplies the retirement seam for the real job implementation.
        // ------------------=
        pub fn retire_leaf(&self, _: u64, _: execution::SecurityIdentity) -> Result<(), ()> { Ok(()) }
    }
    pub struct Runtime { pub capabilities: Caps }
    // ------------------------=
    // FUNC: with_runtime
    // DESC: Exposes deterministic authority to the production job.
    // ------------------=
    pub fn with_runtime<T>(f: impl FnOnce(&mut Runtime) -> T) -> Option<T> {
        Some(f(&mut Runtime { capabilities: Caps }))
    }
}
mod drivers { pub mod audio {
    use crate::{runtime::{execution::SecurityIdentity, audio::PlaybackState}, PLAYED, Ordering};
    // ------------------------=
    // FUNC: playback_rate
    // DESC: Supplies a real supported PCM conversion rate.
    // ------------------=
    pub fn playback_rate() -> Option<u32> { Some(48000) }
    // ------------------------=
    // FUNC: play_resident_speech
    // DESC: Records hardware submission only after validating converted PCM dimensions.
    // ------------------=
    pub fn play_resident_speech(_: SecurityIdentity, _: u64, pcm: &[i16], count: usize, rate: u32) -> bool {
        assert_eq!(rate, 48000); assert_eq!(count, 480);
        assert!(pcm[..count].iter().any(|v| *v != 0));
        PLAYED.fetch_add(1, Ordering::SeqCst); true
    }
    // ------------------------=
    // FUNC: stop_playback
    // DESC: Supplies the hardware cancellation seam.
    // ------------------=
    pub fn stop_playback(_: SecurityIdentity) {}
    // ------------------------=
    // FUNC: playback_state
    // DESC: Completes deterministic finite playback.
    // ------------------=
    pub fn playback_state() -> Option<PlaybackState> { Some(PlaybackState::Complete) }
} }
mod types { #[derive(Clone, Copy, Debug)] pub enum AiError { InvalidRequest, QueueFull, AccessDenied, ProviderUnavailable } }
mod voice {
    pub trait SpeechSynthesisProvider {
        // ------------------------=
        // FUNC: synthesize
        // DESC: Mirrors the production provider boundary for the deterministic job test.
        // ------------------=
        fn synthesize(&mut self, text: &[u8], output: &mut [i16]) -> Result<usize, super::types::AiError>;
    }
}
mod qwen { pub mod workers {
    // ------------------------=
    // FUNC: clock_ns
    // DESC: Advances independently of the speech job to exercise elapsed deadlines.
    // ------------------=
    pub fn clock_ns() -> u64 { crate::NOW.load(crate::Ordering::SeqCst) }
    // ------------------------=
    // FUNC: background
    // DESC: Queues without executing on the submitting thread.
    // ------------------=
    pub fn background(task: unsafe fn()) -> bool { unsafe { crate::TASK = Some(task); } true }
} }
#[path = "../kernel/runtime/ai/voice_output.rs"] mod voice_output;
#[no_mangle] static infinity_flite_license: u8 = 1;
// ------------------------=
// FUNC: infinity_flite_synthesize
// DESC: Deterministic provider can finish after deadline without returning a provider error.
// ------------------=
#[no_mangle]
unsafe extern "C" fn infinity_flite_synthesize(_: *const u8, _: usize, pcm: *mut i16, _: usize,
    frames: *mut usize, peak: *mut usize, _: extern "C" fn() -> i32, _: usize) -> i32 {
    for i in 0..80 { *pcm.add(i) = 100; }
    *frames = 80; *peak = 160;
    if EXPIRE.load(Ordering::SeqCst) { NOW.fetch_add(3_000_000_000, Ordering::SeqCst); }
    0
}
// ------------------------=
// FUNC: deadline_and_cancellation_do_not_publish_stale_pcm
// DESC: Exercises production submit/worker/poll transitions, timeout, owner checks and repeat playback.
// ------------------=
#[test]
fn deadline_and_cancellation_do_not_publish_stale_pcm() {
    use voice_output::{OutputState as S, *};
    let owner = runtime::execution::SecurityIdentity([1; 16]);
    assert!(submit(owner, 2, b"test").is_err());
    assert!(submit(owner, 1, b"").is_err());
    EXPIRE.store(true, Ordering::SeqCst);
    submit(owner, 1, b"test").unwrap();
    assert!(submit(owner, 1, b"overlap").is_err());
    assert_eq!(status().state, S::Queued);
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    assert_eq!(status().state, S::Cancelled);
    poll(); assert_eq!(PLAYED.load(Ordering::SeqCst), 0);
    EXPIRE.store(false, Ordering::SeqCst);
    submit(owner, 1, b"test").unwrap();
    assert!(!stop(runtime::execution::SecurityIdentity([2; 16])));
    assert!(stop(owner));
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    poll(); assert_eq!(status().state, S::Cancelled);
    assert_eq!(PLAYED.load(Ordering::SeqCst), 0);
    submit(owner, 1, b"test").unwrap();
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    assert_eq!(status().state, S::Ready);
    poll(); assert_eq!(status().state, S::Speaking);
    poll(); assert_eq!(status().state, S::Complete);
    assert_eq!(PLAYED.load(Ordering::SeqCst), 1);
    submit(owner, 1, b"test").unwrap();
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    assert_eq!(status().state, S::Ready);
    assert!(stop(owner));
    poll(); assert_eq!(status().state, S::Cancelled);
    assert_eq!(PLAYED.load(Ordering::SeqCst), 1);
}
