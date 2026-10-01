//! Runs the production speech-job state machine with deterministic hardware/time seams.
#![allow(dead_code)]
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
static NOW: AtomicU64 = AtomicU64::new(1);
static EXPIRE: AtomicBool = AtomicBool::new(false);
static INVALID: AtomicBool = AtomicBool::new(false);
static REVOKED: AtomicBool = AtomicBool::new(false);
static PLAYED: AtomicUsize = AtomicUsize::new(0);
static PLAYING: AtomicBool = AtomicBool::new(false);
static HOLD: AtomicBool = AtomicBool::new(false);
static WORKER_BUSY: AtomicBool = AtomicBool::new(false);
static QUEUE_ROOM: AtomicBool = AtomicBool::new(true);
static PLAY_PTR: AtomicUsize = AtomicUsize::new(0);
static APPENDS: AtomicUsize = AtomicUsize::new(0);
static DMA_FRAMES: AtomicUsize = AtomicUsize::new(160);
static mut TASK: Option<unsafe fn()> = None;
mod runtime {
    pub mod execution {
        #[derive(Clone, Copy, PartialEq, Eq)]
        pub struct SecurityIdentity(pub [u8; 16]);
    }
    pub mod capability { pub enum CapabilityType { AudioOutput } }
    pub mod audio {
        #[derive(PartialEq, Eq)]
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
            if cap == 1 && owner.0 == [1; 16] && !crate::REVOKED.load(crate::Ordering::SeqCst) { Ok(()) } else { Err(()) }
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
    pub struct InfinityAudioStatus { pub generation:u64,pub queued_frames:u64,pub played_frames:u64,pub content_position:usize,pub content_boundary:bool }
    // ------------------------=
    // FUNC: playback_rate
    // DESC: Supplies a real supported PCM conversion rate.
    // ------------------=
    pub fn playback_rate() -> Option<u32> { Some(48000) }
    // ------------------------=
    // FUNC: playback_frames
    // DESC: Models a delayed hardware cursor independently from the synthesis deadline clock.
    // ------------------=
    pub fn playback_frames(_: SecurityIdentity) -> Option<usize> { Some(crate::DMA_FRAMES.load(Ordering::SeqCst)) }
    // ------------------------=
    // FUNC: infinity_audio_open
    // DESC: Opens the deterministic reusable queue fixture.
    // ------------------=
    pub fn infinity_audio_open(_:SecurityIdentity,_:u64,_:u64)->bool {true}
    // ------------------------=
    // FUNC: infinity_audio_append
    // DESC: Records converted PCM before one continuous playback session.
    // ------------------=
    pub fn infinity_audio_append(_:SecurityIdentity,_:u64,_:u64,pcm:&[i16],_:usize,_:usize)->bool {
        assert_eq!(pcm.len(),320);assert!(pcm.iter().any(|v|*v!=0));
        crate::PLAY_PTR.store(pcm.as_ptr() as usize,Ordering::SeqCst);
        crate::APPENDS.fetch_add(1,Ordering::SeqCst);true
    }
    // ------------------------=
    // FUNC: infinity_audio_can_append
    // DESC: Allows the fixture to force a bounded queue rollover without accepting a partial span.
    // ------------------=
    pub fn infinity_audio_can_append(_:SecurityIdentity,_:u64,_:usize,_:usize,_:usize)->Option<bool> {
        Some(crate::QUEUE_ROOM.load(Ordering::SeqCst))
    }
    // ------------------------=
    // FUNC: infinity_audio_seal
    // DESC: Starts the deterministic continuous stream after its queue is complete.
    // ------------------=
    pub fn infinity_audio_seal(_:SecurityIdentity,_:u64)->bool {
        crate::PLAYING.store(crate::HOLD.load(Ordering::SeqCst),Ordering::SeqCst);
        PLAYED.fetch_add(1,Ordering::SeqCst);true
    }
    // ------------------------=
    // FUNC: infinity_audio_progress
    // DESC: Maps the deterministic DMA cursor to a content cursor.
    // ------------------=
    pub fn infinity_audio_progress(_:SecurityIdentity,generation:u64)->Option<InfinityAudioStatus>{
        let frames=crate::DMA_FRAMES.load(Ordering::SeqCst).min(160);
        Some(InfinityAudioStatus{generation,queued_frames:160,played_frames:frames as u64,content_position:frames*4/160,content_boundary:frames==160})
    }
    // ------------------------=
    // FUNC: infinity_audio_echo_reference
    // DESC: Supplies already-played deterministic PCM to the duplex fixture.
    // ------------------=
    pub fn infinity_audio_echo_reference(_:SecurityIdentity,_:u64,output:&mut[i16])->bool {output.fill(100);true}
    // ------------------------=
    // FUNC: play_resident_speech
    // DESC: Records hardware submission only after validating converted PCM dimensions.
    // ------------------=
    pub fn play_resident_speech(_: SecurityIdentity, _: u64, pcm: &[i16], count: usize, rate: u32) -> bool {
        assert_eq!(rate, 48000); assert_eq!(count, 320);
        assert!(pcm[..count].iter().any(|v| *v != 0));
        crate::PLAY_PTR.store(pcm.as_ptr() as usize, Ordering::SeqCst);
        crate::PLAYING.store(crate::HOLD.load(Ordering::SeqCst), Ordering::SeqCst);
        PLAYED.fetch_add(1, Ordering::SeqCst); true
    }
    // ------------------------=
    // FUNC: stop_playback
    // DESC: Supplies the hardware cancellation seam.
    // ------------------=
    pub fn stop_playback(_: SecurityIdentity) { crate::PLAYING.store(false, Ordering::SeqCst); }
    // ------------------------=
    // FUNC: playback_state
    // DESC: Completes deterministic finite playback.
    // ------------------=
    pub fn playback_state() -> Option<PlaybackState> { Some(if crate::PLAYING.load(Ordering::SeqCst) { PlaybackState::Playing } else { PlaybackState::Complete }) }
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
mod voice_input {
    // ------------------------=
    // FUNC: invalidate
    // DESC: Records no recognizer state in the isolated output harness.
    // ------------------=
    pub fn invalidate() {}
    // ------------------------=
    // FUNC: prepare
    // DESC: Accepts post-synthesis recognizer warmup in the isolated output harness.
    // ------------------=
    pub fn prepare() -> bool { true }
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
    pub fn background(task: unsafe fn()) -> bool {
        if crate::WORKER_BUSY.load(crate::Ordering::SeqCst) { return false; }
        unsafe { crate::TASK = Some(task); } true
    }
} }
#[path = "../kernel/runtime/ai/voice_output.rs"] mod voice_output;
// ------------------------=
// FUNC: infinity_kokoro_native_synthesize
// DESC: Deterministic provider can finish after deadline without returning a provider error.
// ------------------=
#[no_mangle]
unsafe extern "C" fn infinity_kokoro_native_synthesize(_: *const u8, _: usize, pcm: *mut i16, capacity: usize,
    frames: *mut usize, cancel: extern "C" fn(*mut core::ffi::c_void) -> i32, context: *mut core::ffi::c_void) -> i32 {
    assert_eq!(capacity, 720000);
    if cancel(context) != 0 { *frames = 0; return 2; }
    for i in 0..80 { *pcm.add(i) = 100; }
    *frames = if INVALID.load(Ordering::SeqCst) { capacity + 1 } else { 80 };
    if EXPIRE.load(Ordering::SeqCst) { NOW.fetch_add((voice_output::SYNTHESIS_SECONDS + 1) * 1_000_000_000, Ordering::SeqCst); }
    0
}
// ------------------------=
// FUNC: infinity_kokoro_native_diagnostics
// DESC: Supplies bounded provider accounting through the real native ABI.
// ------------------=
#[no_mangle]
unsafe extern "C" fn infinity_kokoro_native_diagnostics(out: *mut usize) {
    for i in 0..12 { *out.add(i) = 0; }
    *out.add(1) = 564940256;
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
    INVALID.store(true, Ordering::SeqCst);
    submit(owner, 1, b"invalid frames").unwrap();
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    assert_eq!(status().state, S::Failed);
    assert_eq!(status().error, 6);
    poll(); assert_eq!(PLAYED.load(Ordering::SeqCst), 1);
    INVALID.store(false, Ordering::SeqCst);
    submit(owner, 1, b"revoked").unwrap();
    REVOKED.store(true, Ordering::SeqCst);
    poll();
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    poll(); assert_eq!(status().state, S::Cancelled);
    assert_eq!(PLAYED.load(Ordering::SeqCst), 1);
    REVOKED.store(false, Ordering::SeqCst);
    // Buffer every span into one generation and start hardware only after the
    // final span is resident, eliminating phrase-boundary silence.
    HOLD.store(true, Ordering::SeqCst);
    let played=PLAYED.load(Ordering::SeqCst);
    let appended=APPENDS.load(Ordering::SeqCst);
    submit_span(owner,1,b"First sentence.",0,15,false).unwrap();
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    poll(); assert!(can_prefetch());
    assert_eq!(status().state,S::Buffered);
    assert_eq!(PLAYED.load(Ordering::SeqCst),played,"incomplete speech must not start a choppy partial stream");
    submit_span(owner,1,b"Second sentence.",15,31,true).unwrap();
    assert!(!can_prefetch());
    assert!(submit(owner, 1, b"Third sentence.").is_err());
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    poll();assert_eq!(status().state,S::Speaking);
    assert_eq!(PLAYED.load(Ordering::SeqCst),played+1,"one sealed response must start exactly once");
    assert_eq!(APPENDS.load(Ordering::SeqCst),appended+2,"both spans must occupy the same response stream");
    DMA_FRAMES.store(80,Ordering::SeqCst);
    let progress=playback_progress(owner).unwrap();
    assert_eq!((progress.frames,progress.total_frames),(80,160));
    let mut reference=[0;9600];
    NOW.fetch_add(2_000_000_000,Ordering::SeqCst);
    assert!(echo_reference(owner,&mut reference));
    assert!(reference.iter().any(|&sample|sample!=0),"DMA speech must remain in the echo reference despite delayed device playback");
    assert!(!echo_reference(runtime::execution::SecurityIdentity([2;16]),&mut reference));
    PLAYING.store(false,Ordering::SeqCst);poll();
    assert_eq!(status().state,S::Complete);
    assert_eq!(PLAYED.load(Ordering::SeqCst),played+1,"response playback must not restart between spans");
    // Busy inference workers delay, rather than discard, queued synthesis.
    WORKER_BUSY.store(true,Ordering::SeqCst);
    submit(owner,1,b"Waiting for an AP.").unwrap();
    poll(); assert_eq!(status().state,S::Queued);
    assert!(unsafe { (&*(&raw const TASK)).is_none() });
    WORKER_BUSY.store(false,Ordering::SeqCst);poll();
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    poll(); assert_eq!(status().state,S::Speaking);
    stop(owner);poll();
    WORKER_BUSY.store(true,Ordering::SeqCst);
    submit(owner,1,b"Cancel before dispatch.").unwrap();
    stop(owner);poll();assert_eq!(status().state,S::Cancelled);
    assert!(unsafe { (&*(&raw const TASK)).is_none() });

    // A rejected final span must fail closed before any partial response reaches DMA.
    WORKER_BUSY.store(false,Ordering::SeqCst);
    submit_span(owner,1,b"Prepare this phrase.",0,20,false).unwrap();
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    poll();assert_eq!(status().state,S::Buffered);
    let played=PLAYED.load(Ordering::SeqCst);
    INVALID.store(true,Ordering::SeqCst);
    submit_span(owner,1,b"Rejected followup.",20,38,true).unwrap();
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    poll();
    assert!(!PLAYING.load(Ordering::SeqCst));
    assert_eq!(status().state,S::Failed);
    assert!(!can_prefetch());
    assert_eq!(PLAYED.load(Ordering::SeqCst),played);
    stop(owner);poll();
    INVALID.store(false,Ordering::SeqCst);
}
