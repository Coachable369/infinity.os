//! Single-owner native speech output jobs. BSP owns authority and playback;
//! a bounded AP task owns synthesis buffers until release-publishing completion.
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use crate::runtime::{audio::PlaybackState, capability::CapabilityType, execution::SecurityIdentity};
use super::{types::AiError, voice::SpeechSynthesisProvider};
const SOURCE_RATE: u32 = 24000;
const CAPACITY: usize = SOURCE_RATE as usize * 30;
// Kokoro takes ~3.3 seconds for 1.25 seconds of speech on the native probe.
// This is a bounded asynchronous execution budget, not a performance claim.
pub const SYNTHESIS_SECONDS: u64 = 90;
pub const OUTPUT_LEASE_SECONDS: u64 = SYNTHESIS_SECONDS + 40;
static STATE: AtomicUsize = AtomicUsize::new(0);
static CANCEL: AtomicBool = AtomicBool::new(false);
static mut OWNER: SecurityIdentity = SecurityIdentity([0; 16]);
static mut CAPABILITY: u64 = 0;
static mut DEADLINE: u64 = 0;
static mut TEXT: [u8; 160] = [0; 160];
static mut LENGTH: usize = 0;
static mut PCM: [i16; CAPACITY] = [0; CAPACITY];
static mut FRAMES: usize = 0;
static mut PEAK: usize = 0;
static mut ELAPSED: u64 = 0;
static mut ERROR: i32 = 0;
#[repr(C, align(128))]
struct Resident([i16; 48_000 * 2 * 32]);
static mut RESIDENT: [Resident; 2] = [Resident([0; 48_000 * 2 * 32]), Resident([0; 48_000 * 2 * 32])];
static mut WRITE_SLOT: usize = 0;
static mut PLAY_SLOT: usize = 0;
static mut PLAYING: bool = false;
static mut RATE: u32 = 0;
static mut OUTPUT_SAMPLES: usize = 0;
static mut DISPATCHED: bool = false;
#[path = "../../drivers/speech_pcm.rs"]
mod speech_pcm;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputState { Idle, Queued, Synthesizing, Ready, Failed, Speaking, Cancelled, Complete }
#[derive(Clone, Copy)]
pub struct OutputStatus { pub state: OutputState, pub frames: usize, pub peak_bytes: usize, pub synthesis_ns: u64, pub error: i32 }
struct NativeSpeech;
unsafe extern "C" {
    fn infinity_kokoro_native_synthesize(text: *const u8, length: usize, pcm: *mut i16,
        capacity: usize, frames: *mut usize,
        cancel: extern "C" fn(*mut core::ffi::c_void) -> i32, context: *mut core::ffi::c_void) -> i32;
    fn infinity_kokoro_native_diagnostics(out: *mut usize);
}
impl SpeechSynthesisProvider for NativeSpeech {
    // ------------------------=
    // FUNC: synthesize
    // DESC: Runs embedded Kokoro on the AP and validates its bounded 24 kHz PCM result before publication.
    // ------------------=
    fn synthesize(&mut self, text: &[u8], output: &mut [i16]) -> Result<usize, AiError> {
        unsafe {
            let mut frames = 0;
            let error = infinity_kokoro_native_synthesize(text.as_ptr(), text.len(), output.as_mut_ptr(),
                output.len(), &mut frames, kokoro_cancelled, core::ptr::null_mut());
            let mut diagnostics = [0usize; 12];
            infinity_kokoro_native_diagnostics(diagnostics.as_mut_ptr());
            PEAK = diagnostics[1]; ERROR = error;
            if error == 0 && frames > 0 && frames <= output.len() { Ok(frames) }
            else { if error == 0 { ERROR = 6; } Err(AiError::ProviderUnavailable) }
        }
    }
}
// ------------------------=
// FUNC: kokoro_cancelled
// DESC: Adapts the private native provider callback without granting access to runtime locks.
// ------------------=
extern "C" fn kokoro_cancelled(_: *mut core::ffi::c_void) -> i32 { cancelled() }
// ------------------------=
// FUNC: cancelled
// DESC: Checks only atomics and the native timer on an AP, never runtime locks or desktop state.
// ------------------=
extern "C" fn cancelled() -> i32 {
    (CANCEL.load(Ordering::Acquire) || super::qwen::workers::clock_ns() >= unsafe { DEADLINE }) as i32
}
// ------------------------=
// FUNC: worker
// DESC: Synthesizes one bounded phrase off the UI thread and release-publishes its result.
// ------------------=
unsafe fn worker() {
    STATE.store(2, Ordering::Release);
    let start = super::qwen::workers::clock_ns();
    let result = NativeSpeech.synthesize(&(&*(&raw const TEXT))[..LENGTH], &mut *(&raw mut PCM));
    FRAMES = result.unwrap_or(0);
    if result.is_ok() && cancelled() == 0 {
        OUTPUT_SAMPLES = ((FRAMES * RATE as usize + SOURCE_RATE as usize - 1) / SOURCE_RATE as usize) * 2;
        (&mut *(&raw mut RESIDENT))[WRITE_SLOT].0.fill(0);
        speech_pcm::fill_rate(&(&*(&raw const PCM))[..FRAMES], &mut (&mut *(&raw mut RESIDENT))[WRITE_SLOT].0[..OUTPUT_SAMPLES], 0, SOURCE_RATE, RATE);
    }
    ELAPSED = super::qwen::workers::clock_ns().saturating_sub(start);
    STATE.store(if ERROR == 2 || cancelled() != 0 { 6 } else if result.is_ok() { 3 } else { 4 }, Ordering::Release);
}
// ------------------------=
// FUNC: submit
// DESC: Queues one bounded native phrase, optionally ahead of playback; busy APs never force UI-thread synthesis.
// ------------------=
pub fn submit(owner: SecurityIdentity, capability: u64, text: &[u8]) -> Result<(), AiError> {
    if text.is_empty() || text.len() > 160 || text.iter().any(|v| !(32..=126).contains(v)) { return Err(AiError::InvalidRequest); }
    if !matches!(STATE.load(Ordering::Acquire), 0 | 4 | 5 | 6 | 7) { return Err(AiError::QueueFull); }
    if unsafe { PLAYING && OWNER != owner } { return Err(AiError::QueueFull); }
    let now = super::qwen::workers::clock_ns();
    let valid = crate::runtime::with_runtime(|r| r.capabilities.validate(capability, owner,
        CapabilityType::AudioOutput, 0, 1, 0, now / 1_000_000_000).is_ok()).unwrap_or(false);
    if !valid { return Err(AiError::AccessDenied); }
    let rate = crate::drivers::audio::playback_rate().ok_or(AiError::ProviderUnavailable)?;
    if !matches!(rate, 44100 | 48000) { return Err(AiError::ProviderUnavailable); }
    unsafe {
        retire(); WRITE_SLOT = if PLAYING { 1 - PLAY_SLOT } else { 0 };
        RATE = rate; OWNER = owner; CAPABILITY = capability; DEADLINE = now.saturating_add(SYNTHESIS_SECONDS * 1_000_000_000);
        (&mut *(&raw mut TEXT)).fill(0); (&mut *(&raw mut TEXT))[..text.len()].copy_from_slice(text); LENGTH = text.len();
        FRAMES = 0; PEAK = 0; ERROR = 0; ELAPSED = 0; CANCEL.store(false, Ordering::Release);
        STATE.store(1, Ordering::Release);
        // Inference can temporarily occupy every AP. Keep one bounded job
        // queued and retry on poll rather than dropping a streaming reply.
        DISPATCHED = super::qwen::workers::background(worker);
    }
    Ok(())
}
// ------------------------=
// FUNC: can_prefetch
// DESC: Allows exactly one synthesis job ahead of the immutable playing DMA buffer.
// ------------------=
pub fn can_prefetch() -> bool { STATE.load(Ordering::Acquire) == 5 && !CANCEL.load(Ordering::Acquire) }
// ------------------------=
// FUNC: stop
// DESC: Cancels only the owning session's job; worker buffers are not reused until it acknowledges cancellation.
// ------------------=
pub fn stop(owner: SecurityIdentity) -> bool {
    if STATE.load(Ordering::Acquire) == 0 || unsafe { OWNER != owner } { return false; }
    CANCEL.store(true, Ordering::Release); true
}
// ------------------------=
// FUNC: status
// DESC: Provides non-private diagnostics, reading worker metrics only after completion publication.
// ------------------=
pub fn status() -> OutputStatus {
    let state = STATE.load(Ordering::Acquire);
    OutputStatus { state: match state {1=>OutputState::Queued,2=>OutputState::Synthesizing,3=>OutputState::Ready,4=>OutputState::Failed,5=>OutputState::Speaking,6=>OutputState::Cancelled,7=>OutputState::Complete,_=>OutputState::Idle},
        frames: if state >= 3 { unsafe { FRAMES } } else { 0 }, peak_bytes: if state >= 3 { unsafe { PEAK } } else { 0 },
        synthesis_ns: if state >= 3 { unsafe { ELAPSED } } else { 0 }, error: if state >= 3 { unsafe { ERROR } } else { 0 } }
}
// ------------------------=
// FUNC: retire
// DESC: Releases completed capability and private phrase storage on the BSP, never while an AP owns it.
// ------------------=
unsafe fn retire() {
    if CAPABILITY != 0 {
        let cap = CAPABILITY; let owner = OWNER;
        crate::runtime::with_runtime(|r| { let _ = r.capabilities.retire_leaf(cap, owner); }); CAPABILITY = 0;
    }
    (&mut *(&raw mut TEXT)).fill(0); (&mut *(&raw mut PCM)).fill(0);
    for slot in 0..2 {
        if !PLAYING || slot != PLAY_SLOT { (&mut *(&raw mut RESIDENT))[slot].0.fill(0); }
    }
}
// ------------------------=
// FUNC: poll
// DESC: Revalidates authority, hands completed PCM to native DMA, and tracks cancellation without blocking the UI.
// ------------------=
pub fn poll() {
    let mut state = STATE.load(Ordering::Acquire);
    if state == 0 { return; }
    unsafe {
        if PLAYING {
            if CANCEL.load(Ordering::Acquire) || matches!(state, 4 | 6) { crate::drivers::audio::stop_playback(OWNER); }
            if let Some(playback) = crate::drivers::audio::playback_state() {
                if playback != PlaybackState::Playing {
                    PLAYING = false;
                    if playback != PlaybackState::Complete { CANCEL.store(true, Ordering::Release); }
                    if state == 5 {
                        state = if playback == PlaybackState::Complete { 7 } else { 6 };
                        STATE.store(state, Ordering::Release); retire();
                    }
                }
            }
        }
        if state <= 3 {
            let now = super::qwen::workers::clock_ns() / 1_000_000_000;
            let valid = crate::runtime::with_runtime(|r| r.capabilities.validate(CAPABILITY, OWNER,
                CapabilityType::AudioOutput, 0, 1, 0, now).is_ok()).unwrap_or(false);
            if !valid { CANCEL.store(true, Ordering::Release); }
        }
        if state == 1 && !DISPATCHED {
            if cancelled() != 0 { STATE.store(6, Ordering::Release); retire(); }
            else { DISPATCHED = super::qwen::workers::background(worker); }
            return;
        }
        if state == 3 {
            if CANCEL.load(Ordering::Acquire) { STATE.store(6, Ordering::Release); retire(); }
            else if PLAYING { return; }
            else if crate::drivers::audio::play_resident_speech(OWNER, CAPABILITY,
                &(&*(&raw const RESIDENT))[WRITE_SLOT].0[..RATE as usize * 2 * 32], OUTPUT_SAMPLES, RATE) {
                // Hardware owns the immutable resident buffer until its stream stops.
                PLAY_SLOT = WRITE_SLOT; PLAYING = true;
                CAPABILITY = 0; (&mut *(&raw mut PCM)).fill(0); (&mut *(&raw mut TEXT)).fill(0);
                STATE.store(5, Ordering::Release);
            } else { ERROR = 7; STATE.store(4, Ordering::Release); retire(); }
        } else if matches!(state, 4 | 6 | 7) && CAPABILITY != 0 { retire(); }
    }
}
