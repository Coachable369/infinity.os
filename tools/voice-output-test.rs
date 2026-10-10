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
static SYNTHESIS_FRAMES: AtomicUsize = AtomicUsize::new(80);
static SYNTHESIS_VALUE: AtomicUsize = AtomicUsize::new(100);
static SYNTHESIS_DELAY_NS: AtomicU64 = AtomicU64::new(0);
static SYNTHESIS_RESULT: AtomicUsize = AtomicUsize::new(0);
static RECOGNITION_RESULT: AtomicUsize = AtomicUsize::new(0);
static NATIVE_BUSY: AtomicBool = AtomicBool::new(false);
static PREPARE_ENTERED: AtomicBool = AtomicBool::new(false);
static PREPARE_RELEASE: AtomicBool = AtomicBool::new(false);
static PREPARE_HOLD: AtomicBool = AtomicBool::new(true);
static PREPARE_RESULT: AtomicUsize = AtomicUsize::new(0);
static PREPARE_CALLS: AtomicUsize = AtomicUsize::new(0);
static PREPARE_RETURN_FAULT: AtomicBool = AtomicBool::new(false);
static SEAL_FAIL: AtomicBool = AtomicBool::new(false);
static RECOGNIZED_SAMPLES: AtomicUsize = AtomicUsize::new(0);
static BUFFERED_PCM: std::sync::Mutex<Vec<i16>> = std::sync::Mutex::new(Vec::new());
static PLAYED_PCM: std::sync::Mutex<Vec<Vec<i16>>> = std::sync::Mutex::new(Vec::new());
static BUFFERED_CONTENT: std::sync::Mutex<Vec<(usize, usize)>> = std::sync::Mutex::new(Vec::new());
static PLAYED_CONTENT: std::sync::Mutex<Vec<Vec<(usize, usize)>>> = std::sync::Mutex::new(Vec::new());
const RESIDENT_SAMPLES: usize = 48_000 * 2 * 32;
static PLAY_PTR: AtomicUsize = AtomicUsize::new(0);
static APPENDS: AtomicUsize = AtomicUsize::new(0);
static DMA_FRAMES: AtomicUsize = AtomicUsize::new(160);
static mut TASK: Option<unsafe fn()> = None;
mod runtime {
    pub mod execution {
        #[derive(Clone, Copy, PartialEq, Eq)]
        pub struct SecurityIdentity(pub [u8; 16]);
    }
    pub mod capability { pub enum CapabilityType { AudioOutput, AudioInput } }
    pub mod iop {
        pub enum OperationId { SpeechRecognize }
        pub struct IopMessage { pub capability:u64 }
        impl IopMessage {
            // ------------------------=
            // FUNC: request
            // DESC: Retains the supplied capability through the recognition authorization seam.
            // ------------------=
            pub fn request(_:OperationId,_:u64,_:super::execution::SecurityIdentity,capability:u64,_:u64,_:u64,_:&[u8])->Result<Self,()>{Ok(Self{capability})}
        }
    }
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
    pub fn infinity_audio_open(_:SecurityIdentity,_:u64,_:u64)->bool {
        assert!(!crate::PLAYING.load(Ordering::SeqCst));
        crate::BUFFERED_PCM.lock().unwrap().clear();
        crate::BUFFERED_CONTENT.lock().unwrap().clear();
        true
    }
    // ------------------------=
    // FUNC: infinity_audio_append
    // DESC: Records converted PCM before one continuous playback session.
    // ------------------=
    pub fn infinity_audio_append(_:SecurityIdentity,_:u64,_:u64,pcm:&[i16],start:usize,end:usize)->bool {
        assert_eq!(pcm.len(),crate::SYNTHESIS_FRAMES.load(Ordering::SeqCst)*4);
        assert!(pcm.iter().any(|v|*v!=0));
        assert!(!crate::PLAYING.load(Ordering::SeqCst));
        let mut buffered=crate::BUFFERED_PCM.lock().unwrap();
        if pcm.len()>crate::RESIDENT_SAMPLES-buffered.len() {return false;}
        buffered.extend_from_slice(pcm);
        crate::BUFFERED_CONTENT.lock().unwrap().push((start,end));
        crate::PLAY_PTR.store(pcm.as_ptr() as usize,Ordering::SeqCst);
        crate::APPENDS.fetch_add(1,Ordering::SeqCst);true
    }
    // ------------------------=
    // FUNC: infinity_audio_can_append
    // DESC: Allows the fixture to force a bounded queue rollover without accepting a partial span.
    // ------------------=
    pub fn infinity_audio_can_append(_:SecurityIdentity,_:u64,count:usize,_:usize,_:usize)->Option<bool> {
        Some(crate::QUEUE_ROOM.load(Ordering::SeqCst)
            && count<=crate::RESIDENT_SAMPLES-crate::BUFFERED_PCM.lock().unwrap().len())
    }
    // ------------------------=
    // FUNC: infinity_audio_seal
    // DESC: Starts the deterministic continuous stream after its queue is complete.
    // ------------------=
    pub fn infinity_audio_seal(_:SecurityIdentity,_:u64)->bool {
        assert!(!crate::PLAYING.load(Ordering::SeqCst));
        if crate::SEAL_FAIL.load(Ordering::SeqCst) { return false; }
        crate::PLAYED_PCM.lock().unwrap().push(crate::BUFFERED_PCM.lock().unwrap().clone());
        crate::PLAYED_CONTENT.lock().unwrap().push(crate::BUFFERED_CONTENT.lock().unwrap().clone());
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
    pub const RECOGNITION_DEADLINE_SECONDS:u64=45;
    pub trait SpeechRecognitionProvider {
        // ------------------------=
        // FUNC: recognize_pcm
        // DESC: Mirrors the production recognition provider interface.
        // ------------------=
        fn recognize_pcm(&mut self,pcm:&[i16],rate:u32,out:&mut[u8])->Result<usize,super::types::AiError>;
    }
    // ------------------------=
    // FUNC: authorize_recognition
    // DESC: Exercises production job authorization with the fixture's revocable capabilities.
    // ------------------=
    pub fn authorize_recognition(request:&super::runtime::iop::IopMessage,owner:super::runtime::execution::SecurityIdentity,
        _:usize,caps:&super::runtime::Caps,now:u64)->Result<(),super::types::AiError>{
        caps.validate(request.capability,owner,super::runtime::capability::CapabilityType::AudioInput,0,1,0,now)
            .map_err(|_|super::types::AiError::AccessDenied)
    }
    pub trait SpeechSynthesisProvider {
        // ------------------------=
        // FUNC: synthesize
        // DESC: Mirrors the production provider boundary for the deterministic job test.
        // ------------------=
        fn synthesize(&mut self, text: &[u8], output: &mut [i16]) -> Result<usize, super::types::AiError>;
    }
}
#[path = "../kernel/runtime/ai/voice_input.rs"] mod voice_input;
// Isolated production state machines allow terminal warmup scenarios without
// adding a reset-only API or changing the successful engine's retained state.
#[path = "../kernel/runtime/ai/voice_input.rs"] mod failed_voice_input;
#[path = "../kernel/runtime/ai/voice_input.rs"] mod expired_voice_input;
#[path = "../kernel/runtime/ai/voice_input.rs"] mod recognition_fault_voice_input;
#[path = "../kernel/runtime/ai/voice_input.rs"] mod late_prepare_voice_input;
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
    pub unsafe fn background(task: unsafe fn()) -> bool {
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
    if NATIVE_BUSY.load(Ordering::Acquire) { return 8; }
    if cancel(context) != 0 { *frames = 0; return 2; }
    let result=SYNTHESIS_RESULT.load(Ordering::SeqCst) as i32;
    if result != 0 { return result; }
    let generated=SYNTHESIS_FRAMES.load(Ordering::SeqCst);
    for i in 0..generated { *pcm.add(i) = SYNTHESIS_VALUE.load(Ordering::SeqCst) as i16; }
    *frames = if INVALID.load(Ordering::SeqCst) { capacity + 1 } else { generated };
    NOW.fetch_add(SYNTHESIS_DELAY_NS.load(Ordering::SeqCst),Ordering::SeqCst);
    if EXPIRE.load(Ordering::SeqCst) { NOW.fetch_add((voice_output::SYNTHESIS_SECONDS + 1) * 1_000_000_000, Ordering::SeqCst); }
    0
}
// ------------------------=
// FUNC: infinity_kokoro_native_prepare_recognition
// DESC: Holds the real preparation worker at the native ownership boundary while a typed reply attempts synthesis.
// ------------------=
#[no_mangle]
unsafe extern "C" fn infinity_kokoro_native_prepare_recognition(memory:*mut usize,
    cancel:extern "C" fn(*mut core::ffi::c_void)->i32,context:*mut core::ffi::c_void)->i32 {
    PREPARE_CALLS.fetch_add(1,Ordering::SeqCst);
    if NATIVE_BUSY.swap(true,Ordering::AcqRel) { return 8; }
    PREPARE_ENTERED.store(true,Ordering::Release);
    while PREPARE_HOLD.load(Ordering::Acquire) && !PREPARE_RELEASE.load(Ordering::Acquire) { std::thread::yield_now(); }
    *memory=128;
    let result=if cancel(context)==0 {PREPARE_RESULT.load(Ordering::SeqCst) as i32} else {2};
    NATIVE_BUSY.store(false,Ordering::Release);
    if PREPARE_RETURN_FAULT.load(Ordering::SeqCst) { late_prepare_voice_input::native_engine_fault(); }
    result
}
// ------------------------=
// FUNC: infinity_kokoro_native_recognize
// DESC: Checks that queued recognition preserves its original PCM across native-engine contention.
// ------------------=
#[no_mangle]
unsafe extern "C" fn infinity_kokoro_native_recognize(pcm:*const i16,samples:usize,text:*mut u8,_:usize,
    length:*mut usize,memory:*mut usize,cancel:extern "C" fn(*mut core::ffi::c_void)->i32,context:*mut core::ffi::c_void)->i32{
    if NATIVE_BUSY.load(Ordering::Acquire) {return 8;}
    if cancel(context)!=0 {return 2;}
    let result=RECOGNITION_RESULT.load(Ordering::SeqCst) as i32;
    if result != 0 {return result;}
    assert_eq!(std::slice::from_raw_parts(pcm,samples),[17,-19,23,-29]);
    RECOGNIZED_SAMPLES.fetch_add(samples,Ordering::SeqCst);
    std::ptr::copy_nonoverlapping(b"accepted".as_ptr(),text,8);*length=8;*memory=128;0
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
    // Real warmup and output state machines overlap on separate workers. A
    // busy reply stays queued with its buffers unavailable until warmup exits.
    NATIVE_BUSY.store(true,Ordering::Release);
    assert!(!voice_input::prepare());
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    assert!(!voice_input::prepared());
    assert!(!PREPARE_ENTERED.load(Ordering::Acquire));
    NATIVE_BUSY.store(false,Ordering::Release);
    voice_input::poll();
    let prepare=unsafe { (&mut *(&raw mut TASK)).take().unwrap() };
    let thread=std::thread::spawn(move||unsafe {prepare();});
    while !PREPARE_ENTERED.load(Ordering::Acquire) {std::thread::yield_now();}
    submit(owner,1,b"Typed reply during preparation.").unwrap();
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    assert_eq!(status().state,S::Queued);
    assert_eq!(status().error,0);
    assert!(submit(owner,1,b"Must not reuse queued buffers.").is_err());
    assert_eq!(PLAYED.load(Ordering::SeqCst),0);
    PREPARE_RELEASE.store(true,Ordering::Release);thread.join().unwrap();
    assert!(voice_input::prepared());
    poll();unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    assert_eq!(status().state,S::Ready);
    poll();poll();assert_eq!(status().state,S::Complete);
    assert_eq!(PLAYED.swap(0,Ordering::SeqCst),1);

    // Recognition keeps PCM, does not expose a transcript on busy, and accepts
    // no replacement buffer until the pending request is completed or cancelled.
    NATIVE_BUSY.store(true,Ordering::Release);
    voice_input::submit(owner,1,&[17,-19,23,-29]).unwrap();
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    assert_eq!(voice_input::status().state,voice_input::InputState::Queued);
    assert!(voice_input::submit(owner,1,&[1]).is_err());
    assert_eq!(RECOGNIZED_SAMPLES.load(Ordering::SeqCst),0);
    NATIVE_BUSY.store(false,Ordering::Release);voice_input::poll();
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    let mut transcript=[0;32];
    assert_eq!(voice_input::take(owner,&mut transcript).unwrap(),8);
    assert_eq!(&transcript[..8],b"accepted");
    assert_eq!(RECOGNIZED_SAMPLES.load(Ordering::SeqCst),4);
    for reason in 0..3 {
        NATIVE_BUSY.store(true,Ordering::Release);
        voice_input::submit(owner,1,&[17,-19,23,-29]).unwrap();
        unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
        match reason {
            0=>assert!(voice_input::stop(owner)),
            1=>REVOKED.store(true,Ordering::SeqCst),
            _=>{NOW.fetch_add(46_000_000_000,Ordering::SeqCst);}
        }
        voice_input::poll();
        assert_eq!(voice_input::status().state,voice_input::InputState::Cancelled);
        assert!(voice_input::take(owner,&mut transcript).is_err());
        assert!(unsafe { (&*(&raw const TASK)).is_none() });
        REVOKED.store(false,Ordering::SeqCst);NATIVE_BUSY.store(false,Ordering::Release);
    }
    // Synthesis cancellation while waiting for shared native ownership never
    // resubmits or publishes the cancelled response.
    for reason in 0..3 {
        NATIVE_BUSY.store(true,Ordering::Release);
        submit(owner,1,b"Cancel while engine busy.").unwrap();
        unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
        match reason {
            0=>{assert!(stop(owner));},
            1=>REVOKED.store(true,Ordering::SeqCst),
            _=>{NOW.fetch_add((SYNTHESIS_SECONDS+1)*1_000_000_000,Ordering::SeqCst);}
        }
        poll();assert_eq!(status().state,S::Cancelled);
        assert!(unsafe { (&*(&raw const TASK)).is_none() });
        NATIVE_BUSY.store(false,Ordering::Release);REVOKED.store(false,Ordering::SeqCst);
    }
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
    assert!(!can_prefetch());
    assert!(submit_span(owner,1,b"No overlapping synthesis.",31,56,true).is_err());
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

    // Two individually valid 16.7-second phrases exceed the real resident
    // queue. The second must remain intact while the first owns DMA, then
    // continue in order without dropping the response or mutating playing PCM.
    SYNTHESIS_FRAMES.store(400_000,Ordering::SeqCst);
    SYNTHESIS_VALUE.store(100,Ordering::SeqCst);
    let played=PLAYED.load(Ordering::SeqCst);
    let batches=PLAYED_PCM.lock().unwrap().len();
    submit_span(owner,1,b"First long phrase.",0,18,false).unwrap();
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    poll();assert_eq!(status().state,S::Buffered);
    assert!(submit_span(owner,1,b"Invalid rewind.",0,15,true).is_err());
    SYNTHESIS_VALUE.store(200,Ordering::SeqCst);
    submit_span(owner,1,b"Second long phrase.",18,37,true).unwrap();
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    poll();assert_eq!(status().state,S::Ready);
    assert_eq!(PLAYED.load(Ordering::SeqCst),played+1);
    let first_playback=status().first_playback_ns;
    assert_ne!(first_playback,0);
    assert!(!can_prefetch());
    for _ in 0..8 {poll();}
    assert_eq!(PLAYED.load(Ordering::SeqCst),played+1);
    assert!(submit(owner,1,b"Must wait.").is_err());
    assert!(!stop(runtime::execution::SecurityIdentity([2;16])));
    assert!(PLAYING.load(Ordering::SeqCst));
    NOW.fetch_add(17_000_000_000,Ordering::SeqCst);
    PLAYING.store(false,Ordering::SeqCst);poll();
    assert_eq!(status().state,S::Speaking);
    assert_eq!(PLAYED.load(Ordering::SeqCst),played+2);
    assert_eq!(status().first_playback_ns,first_playback);
    {
        let pcm=PLAYED_PCM.lock().unwrap();
        assert_eq!(pcm[batches].len(),1_600_000);
        assert_eq!(pcm[batches+1].len(),1_600_000);
        assert!(pcm[batches][64..1_599_936].iter().all(|&sample|sample==100));
        assert!(pcm[batches+1][64..1_599_936].iter().all(|&sample|sample==200));
        assert!(pcm[batches].chunks_exact(2).all(|pair|pair[0]==pair[1]));
        assert!(pcm[batches+1].chunks_exact(2).all(|pair|pair[0]==pair[1]));
        let content=PLAYED_CONTENT.lock().unwrap();
        assert_eq!(content[batches],[(0,18)]);
        assert_eq!(content[batches+1],[(18,37)]);
    }
    PLAYING.store(false,Ordering::SeqCst);poll();
    assert_eq!(status().state,S::Complete);

    // Cancellation and revocation during rollover must retire the retained
    // phrase instead of playing it after the original generation drains.
    for revoked in [false,true] {
        submit_span(owner,1,b"First long phrase.",0,18,false).unwrap();
        unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
        poll();
        submit_span(owner,1,b"Second long phrase.",18,37,true).unwrap();
        unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
        poll();assert_eq!(status().state,S::Ready);
        let played=PLAYED.load(Ordering::SeqCst);
        if revoked {REVOKED.store(true,Ordering::SeqCst);} else {assert!(stop(owner));}
        poll();poll();
        assert_eq!(status().state,S::Cancelled);
        assert!(!PLAYING.load(Ordering::SeqCst));
        assert_eq!(PLAYED.load(Ordering::SeqCst),played);
        REVOKED.store(false,Ordering::SeqCst);
    }
    SYNTHESIS_FRAMES.store(80,Ordering::SeqCst);
    SYNTHESIS_VALUE.store(100,Ordering::SeqCst);
    submit(owner,1,b"Next turn.").unwrap();
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    poll();assert_eq!(status().state,S::Speaking);
    PLAYING.store(false,Ordering::SeqCst);poll();
    assert_eq!(status().state,S::Complete);

    // Structured timing distinguishes scheduler wait, actual synthesis and
    // post-synthesis publication, then resets for the next response.
    let accepted=NOW.load(Ordering::SeqCst)+1_000_000_000;
    NOW.store(accepted,Ordering::SeqCst);
    WORKER_BUSY.store(true,Ordering::SeqCst);
    SYNTHESIS_DELAY_NS.store(2_500_000_000,Ordering::SeqCst);
    submit(owner,1,b"Measured response.").unwrap();
    assert_eq!(status().queued_ns,accepted);
    assert_eq!(status().queue_wait_ns,0);
    assert_eq!(status().first_playback_ns,0);
    NOW.fetch_add(3_000_000,Ordering::SeqCst);poll();
    assert_eq!(status().state,S::Queued);
    assert_eq!(status().queue_wait_ns,0);
    WORKER_BUSY.store(false,Ordering::SeqCst);poll();
    NOW.fetch_add(2_000_000,Ordering::SeqCst);
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    assert_eq!(status().state,S::Ready);
    assert_eq!(status().queue_wait_ns,5_000_000);
    assert_eq!(status().synthesis_ns,2_500_000_000);
    assert_eq!(status().first_playback_ns,0);
    NOW.fetch_add(11_000_000,Ordering::SeqCst);poll();
    assert_eq!(status().state,S::Speaking);
    assert_eq!(status().first_playback_ns,accepted+2_516_000_000);
    PLAYING.store(false,Ordering::SeqCst);poll();
    assert_eq!(status().state,S::Complete);
    assert_eq!(status().queued_ns,accepted);
    assert_eq!(status().first_playback_ns,accepted+2_516_000_000);

    // A cancelled queued request never reports a worker or audible start.
    WORKER_BUSY.store(true,Ordering::SeqCst);
    NOW.fetch_add(1_000_000,Ordering::SeqCst);
    let cancelled_at=NOW.load(Ordering::SeqCst);
    submit(owner,1,b"Cancel queued timing.").unwrap();
    assert_eq!(status().queued_ns,cancelled_at);
    assert_eq!(status().queue_wait_ns,0);
    assert_eq!(status().first_playback_ns,0);
    NOW.fetch_add(9_000_000,Ordering::SeqCst);
    assert!(stop(owner));poll();
    assert_eq!(status().state,S::Cancelled);
    assert_eq!(status().queue_wait_ns,0);
    assert_eq!(status().first_playback_ns,0);
    assert!(unsafe { (&*(&raw const TASK)).is_none() });

    // Later spans belong to the same response clock even though each has its
    // own scheduling delay; sealing records the first hardware start once.
    WORKER_BUSY.store(false,Ordering::SeqCst);
    SYNTHESIS_DELAY_NS.store(1_000_000,Ordering::SeqCst);
    let accepted=NOW.load(Ordering::SeqCst);
    submit_span(owner,1,b"First.",0,6,false).unwrap();
    NOW.fetch_add(2_000_000,Ordering::SeqCst);
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    poll();assert_eq!(status().state,S::Buffered);
    assert_eq!(status().queued_ns,accepted);
    assert_eq!(status().queue_wait_ns,2_000_000);
    assert_eq!(status().first_playback_ns,0);
    submit_span(owner,1,b"Second.",6,13,true).unwrap();
    assert_eq!(status().queued_ns,accepted);
    assert_eq!(status().queue_wait_ns,0);
    NOW.fetch_add(4_000_000,Ordering::SeqCst);
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    poll();assert_eq!(status().state,S::Speaking);
    assert_eq!(status().queued_ns,accepted);
    assert_eq!(status().queue_wait_ns,4_000_000);
    assert_eq!(status().first_playback_ns,accepted+8_000_000);
    PLAYING.store(false,Ordering::SeqCst);poll();
    assert_eq!(status().state,S::Complete);
    SYNTHESIS_DELAY_NS.store(0,Ordering::SeqCst);

    // A native fatal fault is terminal even when the UI repeatedly asks to
    // prepare again; no warmup job or replacement deadline may be scheduled.
    PREPARE_HOLD.store(false,Ordering::SeqCst);
    PREPARE_RESULT.store(7,Ordering::SeqCst);
    let calls=PREPARE_CALLS.load(Ordering::SeqCst);
    assert!(!failed_voice_input::prepare());
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    assert!(failed_voice_input::prepare_failed());
    assert!(!failed_voice_input::prepared());
    for _ in 0..128 {
        NOW.fetch_add(1_000_000_000,Ordering::SeqCst);
        assert!(!failed_voice_input::prepare());
        failed_voice_input::poll();
        assert!(unsafe { (&*(&raw const TASK)).is_none() });
    }
    assert_eq!(PREPARE_CALLS.load(Ordering::SeqCst),calls+1);

    // Transient admission contention remains retryable, but exhausting the
    // original deadline is terminal, not an unbounded series of new leases.
    PREPARE_RESULT.store(0,Ordering::SeqCst);
    NATIVE_BUSY.store(true,Ordering::SeqCst);
    let calls=PREPARE_CALLS.load(Ordering::SeqCst);
    assert!(!expired_voice_input::prepare());
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    assert!(!expired_voice_input::prepare_failed());
    NOW.fetch_add(90_000_000_000,Ordering::SeqCst);
    expired_voice_input::poll();
    assert!(expired_voice_input::prepare_failed());
    NATIVE_BUSY.store(false,Ordering::SeqCst);
    for _ in 0..128 {
        NOW.fetch_add(1_000_000_000,Ordering::SeqCst);
        assert!(!expired_voice_input::prepare());
        expired_voice_input::poll();
        assert!(unsafe { (&*(&raw const TASK)).is_none() });
    }
    assert_eq!(PREPARE_CALLS.load(Ordering::SeqCst),calls+1);
    assert!(voice_input::prepared());

    // A terminal notification after native admission releases but before the
    // warmup wrapper publishes success must not be overwritten by readiness.
    PREPARE_RETURN_FAULT.store(true,Ordering::SeqCst);
    assert!(!late_prepare_voice_input::prepare());
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    PREPARE_RETURN_FAULT.store(false,Ordering::SeqCst);
    assert!(late_prepare_voice_input::prepare_failed());
    assert!(!late_prepare_voice_input::prepared());

    // Driver publication has its own error7 and must not poison native speech.
    SEAL_FAIL.store(true,Ordering::SeqCst);
    submit(owner,1,b"Device playback failure.").unwrap();
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    poll();
    assert_eq!(status().state,S::Failed);
    assert_eq!(status().error,7);
    assert!(voice_input::prepared());
    assert!(!voice_input::prepare_failed());
    SEAL_FAIL.store(false,Ordering::SeqCst);

    // A native synthesis fault poisons the shared engine after successful
    // warmup. Unlike a driver failure, it also removes recognizer readiness.
    SYNTHESIS_RESULT.store(7,Ordering::SeqCst);
    submit(owner,1,b"Native synthesis fault.").unwrap();
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    poll();
    assert_eq!(status().state,S::Failed);
    assert_eq!(status().error,7);
    assert!(voice_input::prepare_failed());
    assert!(!voice_input::prepared());
    let calls=PREPARE_CALLS.load(Ordering::SeqCst);
    for _ in 0..128 {
        assert!(!voice_input::prepare());
        voice_input::poll();
        assert!(unsafe { (&*(&raw const TASK)).is_none() });
    }
    assert_eq!(PREPARE_CALLS.load(Ordering::SeqCst),calls);
    SYNTHESIS_RESULT.store(0,Ordering::SeqCst);

    // Recognition independently reports the same terminal fault, and repeated
    // prepare requests cannot restart its previously ready native context.
    assert!(!recognition_fault_voice_input::prepare());
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    assert!(recognition_fault_voice_input::prepared());
    RECOGNITION_RESULT.store(7,Ordering::SeqCst);
    recognition_fault_voice_input::submit(owner,1,&[17,-19,23,-29]).unwrap();
    unsafe { (&mut *(&raw mut TASK)).take().unwrap()(); }
    assert_eq!(recognition_fault_voice_input::status().state,recognition_fault_voice_input::InputState::Failed);
    assert_eq!(recognition_fault_voice_input::status().error,7);
    assert!(recognition_fault_voice_input::prepare_failed());
    assert!(!recognition_fault_voice_input::prepared());
    let calls=PREPARE_CALLS.load(Ordering::SeqCst);
    for _ in 0..128 {
        assert!(!recognition_fault_voice_input::prepare());
        recognition_fault_voice_input::poll();
        assert!(unsafe { (&*(&raw const TASK)).is_none() });
    }
    assert_eq!(PREPARE_CALLS.load(Ordering::SeqCst),calls);
}
