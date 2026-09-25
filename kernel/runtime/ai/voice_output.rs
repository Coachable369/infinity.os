//! Single-owner native speech output jobs. BSP owns authority and playback;
//! a bounded AP task owns synthesis buffers until release-publishing completion.
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use crate::runtime::{audio::PlaybackState, capability::CapabilityType, execution::SecurityIdentity};
use super::{types::AiError, voice::SpeechSynthesisProvider};
const CAPACITY: usize = 240000;
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
static mut RESIDENT: Resident = Resident([0; 48_000 * 2 * 32]);
static mut RATE: u32 = 0;
static mut OUTPUT_SAMPLES: usize = 0;
#[path = "../../drivers/speech_pcm.rs"]
mod speech_pcm;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputState { Idle, Queued, Synthesizing, Ready, Failed, Speaking, Cancelled, Complete }
#[derive(Clone, Copy)]
pub struct OutputStatus { pub state: OutputState, pub frames: usize, pub peak_bytes: usize, pub synthesis_ns: u64, pub error: i32 }
unsafe extern "C" {
    static infinity_flite_license: u8;
    fn infinity_flite_synthesize(text: *const u8, length: usize, pcm: *mut i16, capacity: usize,
        frames: *mut usize, peak: *mut usize, cancel: extern "C" fn() -> i32, limit: usize) -> i32;
}
struct Flite;
impl SpeechSynthesisProvider for Flite {
    // ------------------------=
    // FUNC: synthesize
    // DESC: Runs the pinned native voice behind the existing replaceable speech-provider contract.
    // ------------------=
    fn synthesize(&mut self, text: &[u8], output: &mut [i16]) -> Result<usize, AiError> {
        unsafe {
            let mut frames = 0; let mut peak = 0;
            let error = infinity_flite_synthesize(text.as_ptr(), text.len(), output.as_mut_ptr(), output.len(),
                &mut frames, &mut peak, cancelled, 8 * 1024 * 1024);
            PEAK = peak; ERROR = error;
            if error == 0 { Ok(frames) } else { Err(AiError::ProviderUnavailable) }
        }
    }
}
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
    let result = Flite.synthesize(&(&*(&raw const TEXT))[..LENGTH], &mut *(&raw mut PCM));
    FRAMES = result.unwrap_or(0);
    if result.is_ok() && cancelled() == 0 {
        OUTPUT_SAMPLES = ((FRAMES * RATE as usize + 7999) / 8000) * 2;
        (&mut *(&raw mut RESIDENT.0)).fill(0);
        speech_pcm::fill(&(&*(&raw const PCM))[..FRAMES], &mut (&mut *(&raw mut RESIDENT.0))[..OUTPUT_SAMPLES], 0, RATE);
    }
    ELAPSED = super::qwen::workers::clock_ns().saturating_sub(start);
    STATE.store(if ERROR == 2 || cancelled() != 0 { 6 } else if result.is_ok() { 3 } else { 4 }, Ordering::Release);
}
// ------------------------=
// FUNC: submit
// DESC: Validates output authority and queues one native phrase; unavailable APs never force UI-thread synthesis.
// ------------------=
pub fn submit(owner: SecurityIdentity, capability: u64, text: &[u8]) -> Result<(), AiError> {
    // Retain the complete upstream notices in every linked installed runtime.
    unsafe { core::ptr::read_volatile(&raw const infinity_flite_license); }
    if text.is_empty() || text.len() > 160 || text.iter().any(|v| !(32..=126).contains(v)) { return Err(AiError::InvalidRequest); }
    if !matches!(STATE.load(Ordering::Acquire), 0 | 4 | 6 | 7) { return Err(AiError::QueueFull); }
    let now = super::qwen::workers::clock_ns();
    let valid = crate::runtime::with_runtime(|r| r.capabilities.validate(capability, owner,
        CapabilityType::AudioOutput, 0, 1, 0, now / 1_000_000_000).is_ok()).unwrap_or(false);
    if !valid { return Err(AiError::AccessDenied); }
    let rate = crate::drivers::audio::playback_rate().ok_or(AiError::ProviderUnavailable)?;
    unsafe {
        retire(); RATE = rate; OWNER = owner; CAPABILITY = capability; DEADLINE = now.saturating_add(2_000_000_000);
        (&mut *(&raw mut TEXT)).fill(0); (&mut *(&raw mut TEXT))[..text.len()].copy_from_slice(text); LENGTH = text.len();
        FRAMES = 0; PEAK = 0; ERROR = 0; ELAPSED = 0; CANCEL.store(false, Ordering::Release);
        STATE.store(1, Ordering::Release);
        if !super::qwen::workers::background(worker) {
            STATE.store(4, Ordering::Release); retire(); return Err(AiError::ProviderUnavailable);
        }
    }
    Ok(())
}
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
    (&mut *(&raw mut RESIDENT.0)).fill(0);
}
// ------------------------=
// FUNC: poll
// DESC: Revalidates authority, hands completed PCM to native DMA, and tracks cancellation without blocking the UI.
// ------------------=
pub fn poll() {
    let state = STATE.load(Ordering::Acquire);
    if state == 0 { return; }
    unsafe {
        if state <= 3 {
            let now = super::qwen::workers::clock_ns() / 1_000_000_000;
            let valid = crate::runtime::with_runtime(|r| r.capabilities.validate(CAPABILITY, OWNER,
                CapabilityType::AudioOutput, 0, 1, 0, now).is_ok()).unwrap_or(false);
            if !valid { CANCEL.store(true, Ordering::Release); }
        }
        if state == 3 {
            if CANCEL.load(Ordering::Acquire) { STATE.store(6, Ordering::Release); retire(); }
            else if crate::drivers::audio::play_resident_speech(OWNER, CAPABILITY,
                &(&*(&raw const RESIDENT.0))[..RATE as usize * 2 * 32], OUTPUT_SAMPLES, RATE) {
                // Hardware owns the immutable resident buffer until its stream stops.
                CAPABILITY = 0; (&mut *(&raw mut PCM)).fill(0); (&mut *(&raw mut TEXT)).fill(0);
                STATE.store(5, Ordering::Release);
            } else { ERROR = 7; STATE.store(4, Ordering::Release); retire(); }
        } else if state == 5 {
            if CANCEL.load(Ordering::Acquire) { crate::drivers::audio::stop_playback(OWNER); }
            if let Some(playback) = crate::drivers::audio::playback_state() {
                let next = match playback { PlaybackState::Playing=>5,PlaybackState::Complete=>7,PlaybackState::Cancelled=>6,_=>4 };
                if next != 5 { if next == 4 { ERROR = 8; } STATE.store(next, Ordering::Release); retire(); }
            }
        } else if matches!(state, 4 | 6 | 7) && CAPABILITY != 0 { retire(); }
    }
}
