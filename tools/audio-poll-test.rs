//! Executes the production audio adapter with deterministic hardware and time seams.
#![allow(dead_code)]
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
static NOW: AtomicU64 = AtomicU64::new(0);
static CLOCK_VALID: AtomicBool = AtomicBool::new(true);
static READS: AtomicUsize = AtomicUsize::new(0);
static POSITIONS: AtomicUsize = AtomicUsize::new(0);
static REENTER: AtomicBool = AtomicBool::new(false);
const OWNER: execution::SecurityIdentity = execution::SecurityIdentity([1; 16]);
#[path = "../kernel/runtime/execution.rs"] pub mod execution;
#[path = "../kernel/runtime/capability.rs"] pub mod capability;
#[path = "../kernel/runtime/audio.rs"] pub mod audio_contract;
#[path = "../kernel/drivers/audio.rs"] mod audio;
pub use runtime::iop;

mod boot_info { pub struct BootInfo { pub boot_reserved: u32 } }
mod output {
    // ------------------------=
    // FUNC: write
    // DESC: Discards diagnostics so prose is never a behavioral test oracle.
    // ------------------=
    pub unsafe fn write(_: &[u8]) {}
}
mod ui { pub mod performance {
    // ------------------------=
    // FUNC: monotonic_ns
    // DESC: Supplies explicit test time without sleeping or depending on input events.
    // ------------------=
    pub fn monotonic_ns() -> Option<u64> {
        crate::CLOCK_VALID.load(crate::Ordering::SeqCst)
            .then(|| crate::NOW.load(crate::Ordering::SeqCst))
    }
} }
mod runtime {
    pub use crate::{audio_contract as audio, capability, execution};
    pub struct Runtime { pub capabilities: capability::CapabilityManager }
    static RUNTIME: std::sync::Mutex<Runtime> = std::sync::Mutex::new(Runtime {
        capabilities: capability::CapabilityManager::new(),
    });
    // ------------------------=
    // FUNC: with_runtime
    // DESC: Runs real capability validation under one deterministic runtime owner.
    // ------------------=
    pub fn with_runtime<T>(action: impl FnOnce(&mut Runtime) -> T) -> Option<T> {
        Some(action(&mut RUNTIME.lock().unwrap()))
    }
    pub mod iop {
        use super::execution::SecurityIdentity;
        pub const IOP_VERSION: u16 = 1;
        #[derive(PartialEq)] pub enum MessageType { Request }
        pub enum OperationId { AudioTone, AudioCaptureStart, AudioPlaybackStart }
        pub struct Header {
            pub protocol_version: u16, pub schema_version: u16, pub message_type: MessageType,
            pub caller_identity: SecurityIdentity, pub payload_length: usize,
            pub deadline: u64, pub operation_type_id: u32, pub capability_ref: u64,
        }
        pub struct IopMessage { pub header: Header }
        impl IopMessage {
            // ------------------------=
            // FUNC: request
            // DESC: Supplies only the typed envelope; production AudioStream validates its authority and deadline.
            // ------------------=
            pub fn request(operation: OperationId, _: u64, caller: SecurityIdentity,
                capability: u64, deadline: u64, _: u64, payload: &[u8]) -> Result<Self, ()> {
                Ok(Self { header: Header { protocol_version: IOP_VERSION, schema_version: 1,
                    message_type: MessageType::Request, caller_identity: caller,
                    payload_length: payload.len(), deadline,
                    operation_type_id: operation as u32, capability_ref: capability } })
            }
        }
    }
}
mod hda {
    pub const SAMPLES: usize = 9600;
    pub const CAPTURE_FRAMES: usize = 96000;
    pub const CAPTURE_SAMPLES: usize = CAPTURE_FRAMES * 2;
    pub enum Error { Timeout, Unsupported, Busy, Invalid, Dma }
    pub struct Dma;
    impl Dma {
        // ------------------------=
        // FUNC: new
        // DESC: Provides no hardware allocation in the deterministic adapter harness.
        // ------------------=
        pub const fn new() -> Self { Self }
    }
    pub struct Hda {
        pub sample_rate: u32, pub capturing: bool, pub playing: bool,
        sample: i16, origin_ns: u64, consumed_frames: u64,
    }
    pub struct Progress { pub played_bytes: u32, pub complete: bool }
    impl Hda {
        // ------------------------=
        // FUNC: initialize
        // DESC: Attaches a finite mock device while leaving the real adapter state machine intact.
        // ------------------=
        pub unsafe fn initialize(_: usize, _: *mut Dma) -> Result<Self, Error> {
            Ok(Self { sample_rate: 48000, capturing: false, playing: false,
                sample: 0, origin_ns: 0, consumed_frames: 0 })
        }
        // ------------------------=
        // FUNC: capture_available
        // DESC: Exposes the deterministic microphone route.
        // ------------------=
        pub fn capture_available(&self) -> bool { true }
        // ------------------------=
        // FUNC: start_capture
        // DESC: Starts a fresh mock DMA generation without changing the test clock.
        // ------------------=
        pub unsafe fn start_capture(&mut self) -> Result<(), Error> {
            self.capturing = true; self.sample = 0; self.consumed_frames = 0;
            self.origin_ns = crate::NOW.load(crate::Ordering::SeqCst).saturating_sub(1_000_000);
            Ok(())
        }
        // ------------------------=
        // FUNC: read_capture
        // DESC: Counts actual hardware access and returns ordered stereo frames, optionally attempting reentrant service.
        // ------------------=
        pub unsafe fn read_capture(&mut self, out: &mut [i16]) -> Result<usize, Error> {
            assert!(self.capturing);
            crate::READS.fetch_add(1, crate::Ordering::SeqCst);
            if crate::REENTER.swap(false, crate::Ordering::SeqCst) {
                let now = crate::NOW.fetch_add(1_000_000, crate::Ordering::SeqCst);
                crate::audio::poll();
                crate::NOW.store(now, crate::Ordering::SeqCst);
            }
            let now = crate::NOW.load(crate::Ordering::SeqCst);
            let produced = now.saturating_sub(self.origin_ns) * u64::from(self.sample_rate) / 1_000_000_000;
            let frames = produced.saturating_sub(self.consumed_frames).min(out.len() as u64 / 2) as usize;
            for frame in out[..frames * 2].chunks_exact_mut(2) {
                frame.fill(self.sample); self.sample = (self.sample + 1) % 20000;
            }
            self.consumed_frames += frames as u64;
            Ok(frames * 2)
        }
        // ------------------------=
        // FUNC: stop_capture
        // DESC: Records that the production adapter stopped capture after a terminal state.
        // ------------------=
        pub unsafe fn stop_capture(&mut self) { self.capturing = false; }
        // ------------------------=
        // FUNC: start
        // DESC: Starts mock playback for deadline and revocation tests.
        // ------------------=
        pub unsafe fn start(&mut self, _: &[i16]) -> Result<(), Error> { self.playing = true; Ok(()) }
        // ------------------------=
        // FUNC: start_resident
        // DESC: Implements the resident playback hardware seam without altering queue semantics.
        // ------------------=
        pub unsafe fn start_resident(&mut self, samples: &'static [i16]) -> Result<(), Error> { self.start(samples) }
        // ------------------------=
        // FUNC: position
        // DESC: Counts hardware playback reads independently of any rendering or pointer work.
        // ------------------=
        pub unsafe fn position(&self) -> Result<u32, Error> {
            crate::POSITIONS.fetch_add(1, crate::Ordering::SeqCst); Ok(0)
        }
        // ------------------------=
        // FUNC: resident_progress
        // DESC: Leaves resident audio pending until explicit cancellation in these capture-focused tests.
        // ------------------=
        pub unsafe fn resident_progress(&self) -> Result<Progress, Error> {
            self.position()?; Ok(Progress { played_bytes: 0, complete: false })
        }
        // ------------------------=
        // FUNC: stop
        // DESC: Records that the adapter ended hardware playback.
        // ------------------=
        pub unsafe fn stop(&mut self) { self.playing = false; }
    }
    // ------------------------=
    // FUNC: capture_fault_is_recoverable
    // DESC: Preserves the hardware error contract for the production adapter.
    // ------------------=
    pub fn capture_fault_is_recoverable(error: Error) -> bool { matches!(error, Error::Dma) }
    // ------------------------=
    // FUNC: tone_at_rate
    // DESC: Supplies finite deterministic tone samples without testing synthesis here.
    // ------------------=
    pub fn tone_at_rate(samples: &mut [i16], _: u32) -> Result<(), Error> { samples.fill(1); Ok(()) }
}

// ------------------------=
// FUNC: grant
// DESC: Issues real bounded capability authority to the fixture owner.
// ------------------=
fn grant(kind: capability::CapabilityType) -> u64 {
    let now = NOW.load(Ordering::SeqCst) / 1_000_000_000;
    runtime::with_runtime(|r| r.capabilities.grant(kind, 0, 1, 0, OWNER, OWNER, Some(now + 60), 0).unwrap()).unwrap()
}
// ------------------------=
// FUNC: capture
// DESC: Starts actual adapter capture with a fresh authorized native stream.
// ------------------=
fn capture(now: u64) -> u64 {
    CLOCK_VALID.store(true, Ordering::SeqCst); NOW.store(now, Ordering::SeqCst);
    let cap = grant(capability::CapabilityType::AudioInput);
    assert!(audio::capture(OWNER, cap)); cap
}
// ------------------------=
// FUNC: trace
// DESC: Feeds identical elapsed times with different event-loop counts and returns actual delivered PCM and hardware work.
// ------------------=
fn trace(repeats: usize, stride_ms: usize, start: u64) -> (Vec<i16>, usize) {
    capture(start); READS.store(0, Ordering::SeqCst);
    for ms in (0..100).step_by(stride_ms) {
        NOW.store(start + ms as u64 * 1_000_000, Ordering::SeqCst);
        for _ in 0..repeats { audio::poll(); }
    }
    NOW.store(start + 99_000_000, Ordering::SeqCst); audio::poll();
    let status = audio::capture_status().unwrap();
    assert_eq!(status.state, audio_contract::CaptureState::Recording);
    assert_eq!(status.frames, 4800);
    let mut pcm = vec![0; 4800];
    assert_eq!(audio::read_capture(OWNER, &mut pcm), pcm.len());
    let reads = READS.load(Ordering::SeqCst);
    assert!(audio::stop_capture(OWNER)); (pcm, reads)
}
// ------------------------=
// FUNC: actual_adapter_time_cadence_and_authority
// DESC: Exercises real poll dispatch, event-count independence, stream resets, lock contention, clock faults, deadlines, and revocation.
// ------------------=
#[test]
fn actual_adapter_time_cadence_and_authority() {
    use audio_contract::{CaptureState, PlaybackState};
    audio::initialize(&boot_info::BootInfo { boot_reserved: 1 });
    let idle = trace(200, 1, 1_000_000_000);
    let pointer_busy = trace(1, 1, 2_000_000_000);
    assert_eq!(idle, pointer_busy);
    assert_eq!(idle.1, 100);
    let slow_presentation = trace(1, 10, 2_200_000_000);
    assert_eq!(idle.0, slow_presentation.0);
    assert_eq!(slow_presentation.1, 11);

    capture(3_000_000_000); READS.store(0, Ordering::SeqCst);
    audio::poll();
    NOW.fetch_add(999_999, Ordering::SeqCst); audio::poll();
    assert_eq!(READS.load(Ordering::SeqCst), 1);
    NOW.fetch_add(1, Ordering::SeqCst);
    REENTER.store(true, Ordering::SeqCst); audio::poll();
    assert_eq!(READS.load(Ordering::SeqCst), 2);
    NOW.fetch_add(1_000_000, Ordering::SeqCst); audio::poll();
    assert_eq!(READS.load(Ordering::SeqCst), 3);
    NOW.fetch_add(100_000_000, Ordering::SeqCst); audio::poll();
    for _ in 0..100 { audio::poll(); }
    assert_eq!(READS.load(Ordering::SeqCst), 4, "no catch-up burst");
    assert!(audio::stop_capture(OWNER));
    capture(NOW.load(Ordering::SeqCst)); audio::poll();
    assert_eq!(READS.load(Ordering::SeqCst), 5, "new stream polls immediately");
    CLOCK_VALID.store(false, Ordering::SeqCst); audio::poll();
    assert_eq!(audio::capture_status().unwrap().state, CaptureState::Denied);
    assert_eq!(audio::read_capture(OWNER, &mut [0; 48]), 0);

    capture(4_000_000_000); audio::poll();
    NOW.fetch_sub(1, Ordering::SeqCst); audio::poll();
    assert_eq!(audio::capture_status().unwrap().state, CaptureState::Denied);
    let cap = capture(5_000_000_000); audio::poll();
    runtime::with_runtime(|r| r.capabilities.revoke(cap).unwrap());
    assert_eq!(audio::read_capture(OWNER, &mut [0; 48]), 0,
        "cadence never bypasses immediate consumer authority checks");
    NOW.fetch_add(1_000_000, Ordering::SeqCst); audio::poll();
    assert_eq!(audio::capture_status().unwrap().state, CaptureState::Denied);

    capture(6_000_000_000);
    for ms in (0..=3000).step_by(100) { NOW.store(6_000_000_000 + ms * 1_000_000, Ordering::SeqCst); audio::poll(); }
    assert_eq!(audio::capture_status().unwrap().state, CaptureState::Complete);

    NOW.store(10_000_000_000, Ordering::SeqCst);
    let cap = grant(capability::CapabilityType::AudioOutput);
    assert!(audio::tone(OWNER, cap)); POSITIONS.store(0, Ordering::SeqCst);
    audio::poll(); for _ in 0..100 { audio::poll(); }
    assert_eq!(POSITIONS.load(Ordering::SeqCst), 1);
    NOW.store(12_000_000_000, Ordering::SeqCst); audio::poll();
    assert_eq!(audio::playback_state(), Some(PlaybackState::Complete));
    let cap = grant(capability::CapabilityType::AudioOutput);
    assert!(audio::tone(OWNER, cap));
    CLOCK_VALID.store(false, Ordering::SeqCst); audio::poll();
    assert_eq!(audio::playback_state(), Some(PlaybackState::Denied));
}
