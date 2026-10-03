//! Single-owner native speech output jobs. BSP owns authority and playback;
//! a bounded AP task owns synthesis buffers until release-publishing completion.
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use crate::runtime::{audio::PlaybackState, capability::CapabilityType, execution::SecurityIdentity};
use super::{types::AiError, voice::SpeechSynthesisProvider};
const SOURCE_RATE: u32 = 24000;
const CAPACITY: usize = SOURCE_RATE as usize * 30;
const NATIVE_ENGINE_BUSY: i32 = 8;
const WAITING_ENGINE: usize = 9;
// Kokoro takes ~3.3 seconds for 1.25 seconds of speech on the native probe.
// This is a bounded asynchronous execution budget, not a performance claim.
pub const SYNTHESIS_SECONDS: u64 = 90;
pub const OUTPUT_LEASE_SECONDS: u64 = SYNTHESIS_SECONDS + 40;
static STATE: AtomicUsize = AtomicUsize::new(0);
static CANCEL: AtomicBool = AtomicBool::new(false);
static RESPONSE_QUEUED_NS: AtomicU64 = AtomicU64::new(0);
static SPAN_QUEUED_NS: AtomicU64 = AtomicU64::new(0);
static QUEUE_WAIT_NS: AtomicU64 = AtomicU64::new(0);
static FIRST_PLAYBACK_NS: AtomicU64 = AtomicU64::new(0);
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
const RESIDENT_CAPACITY: usize = 48_000 * 2 * 32;
static mut RESIDENT: Resident = Resident([0; 48_000 * 2 * 32]);
static mut PLAYING: bool = false;
static mut RATE: u32 = 0;
static mut OUTPUT_SAMPLES: usize = 0;
static mut DISPATCHED: bool = false;
static STREAM_GENERATION: AtomicUsize = AtomicUsize::new(0);
static mut STREAM_OPEN: bool = false;
static mut PENDING_AFTER_DRAIN: bool = false;
static mut PLAYBACK_DRAINED: bool = false;
static mut CONTENT_START: usize = 0;
static mut CONTENT_END: usize = 0;
static mut FINAL_CHUNK: bool = false;
#[path = "../../drivers/speech_pcm.rs"]
mod speech_pcm;
#[cfg(not(test))]
#[path = "voice_trace.rs"]
mod voice_trace;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputState { Idle, Queued, Synthesizing, Ready, Failed, Speaking, Cancelled, Complete, Buffered }
#[derive(Clone, Copy)]
pub struct OutputStatus {
    pub state: OutputState,
    pub frames: usize,
    pub peak_bytes: usize,
    pub synthesis_ns: u64,
    /// Monotonic acceptance time of this response's first speech span.
    pub queued_ns: u64,
    /// Current span's acceptance-to-worker duration; zero until the worker starts.
    pub queue_wait_ns: u64,
    /// First successful hardware start for this response; zero until playback starts.
    pub first_playback_ns: u64,
    pub error: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaybackProgress { pub sequence: usize, pub frames: usize, pub total_frames: usize, pub content_position: usize, pub content_boundary: bool }
struct NativeSpeech;

#[cfg(not(test))]
// ------------------------=
// FUNC: trace
// DESC: Emits monotonic speech-worker lifecycle checkpoints without exposing prompt content or PCM.
// ------------------=
fn trace(event: &[u8]) {
    voice_trace::write(b"[VOICE OUT] ", event, super::qwen::workers::clock_ns());
}

#[cfg(test)]
// ------------------------=
// FUNC: trace
// DESC: Keeps host behavior harnesses independent of the native serial device.
// ------------------=
fn trace(_: &[u8]) {}

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
            ERROR = error;
            if error == NATIVE_ENGINE_BUSY { return Err(AiError::QueueFull); }
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
    let start = super::qwen::workers::clock_ns();
    QUEUE_WAIT_NS.store(start.saturating_sub(SPAN_QUEUED_NS.load(Ordering::Acquire)), Ordering::Release);
    STATE.store(2, Ordering::Release);
    if ERROR != NATIVE_ENGINE_BUSY { trace(b"synthesis started"); }
    let result = NativeSpeech.synthesize(&(&*(&raw const TEXT))[..LENGTH], &mut *(&raw mut PCM));
    if ERROR == NATIVE_ENGINE_BUSY && cancelled() == 0 {
        STATE.store(WAITING_ENGINE, Ordering::Release);
        return;
    }
    FRAMES = result.unwrap_or(0);
    if result.is_ok() && cancelled() == 0 {
        OUTPUT_SAMPLES = ((FRAMES * RATE as usize + SOURCE_RATE as usize - 1) / SOURCE_RATE as usize) * 2;
        speech_pcm::fill_rate(&(&*(&raw const PCM))[..FRAMES], &mut (&mut *(&raw mut RESIDENT)).0[..OUTPUT_SAMPLES], 0, SOURCE_RATE, RATE);
    }
    ELAPSED = super::qwen::workers::clock_ns().saturating_sub(start);
    trace(if result.is_ok() { b"synthesis completed" } else { match ERROR {
        1 => b"synthesis failed: invalid request",
        2 => b"synthesis cancelled",
        3 => b"synthesis failed: model",
        4 => b"synthesis failed: phonemizer",
        5 => b"synthesis failed: inference",
        6 => b"synthesis failed: pcm",
        7 => b"synthesis failed: native fault",
        _ => b"synthesis failed: unknown",
    }});
    STATE.store(if ERROR == 2 || cancelled() != 0 { 6 } else if result.is_ok() { 3 } else { 4 }, Ordering::Release);
}
// ------------------------=
// FUNC: submit_span
// DESC: Synthesizes one bounded content span into the reusable InfinityAudio queue under generation-safe authority.
// ------------------=
pub fn submit_span(owner: SecurityIdentity, capability: u64, text: &[u8], content_start: usize,
    content_end: usize, final_chunk: bool) -> Result<(), AiError> {
    if text.is_empty() || text.len() > 160 || text.iter().any(|v| !(32..=126).contains(v)) { return Err(AiError::InvalidRequest); }
    let state = STATE.load(Ordering::Acquire);
    if !matches!(state, 0 | 4 | 6 | 7 | 8) || content_end < content_start { return Err(AiError::QueueFull); }
    if unsafe { (PLAYING || STREAM_OPEN) && OWNER != owner } { return Err(AiError::QueueFull); }
    if state == 8 && content_start < unsafe { CONTENT_END } { return Err(AiError::InvalidRequest); }
    let now = super::qwen::workers::clock_ns();
    let valid = crate::runtime::with_runtime(|r| r.capabilities.validate(capability, owner,
        CapabilityType::AudioOutput, 0, 1, 0, now / 1_000_000_000).is_ok()).unwrap_or(false);
    if !valid { return Err(AiError::AccessDenied); }
    let rate = crate::drivers::audio::playback_rate().ok_or(AiError::ProviderUnavailable)?;
    if !matches!(rate, 44100 | 48000) { return Err(AiError::ProviderUnavailable); }
    unsafe {
        retire();
        if state != 8 {
            STREAM_OPEN = false; PLAYING = false; PENDING_AFTER_DRAIN = false; PLAYBACK_DRAINED = false;
            RESPONSE_QUEUED_NS.store(now, Ordering::Release);
            FIRST_PLAYBACK_NS.store(0, Ordering::Release);
            let next = STREAM_GENERATION.load(Ordering::Acquire).wrapping_add(1).max(1);
            STREAM_GENERATION.store(next, Ordering::Release);
        }
        RATE = rate; OWNER = owner; CAPABILITY = capability; DEADLINE = now.saturating_add(SYNTHESIS_SECONDS * 1_000_000_000);
        CONTENT_START = content_start; CONTENT_END = content_end; FINAL_CHUNK = final_chunk;
        (&mut *(&raw mut TEXT)).fill(0); (&mut *(&raw mut TEXT))[..text.len()].copy_from_slice(text); LENGTH = text.len();
        FRAMES = 0; PEAK = 0; ERROR = 0; ELAPSED = 0; CANCEL.store(false, Ordering::Release);
        SPAN_QUEUED_NS.store(now, Ordering::Release);
        QUEUE_WAIT_NS.store(0, Ordering::Release);
        STATE.store(1, Ordering::Release);
        // Inference can temporarily occupy every AP. Keep one bounded job
        // queued and retry on poll rather than dropping a streaming reply.
        DISPATCHED = super::qwen::workers::background(worker);
        trace(if DISPATCHED { b"job dispatched" } else { b"job waiting for worker" });
    }
    Ok(())
}
// ------------------------=
// FUNC: submit
// DESC: Preserves the one-shot speech API by queuing and sealing one complete content span.
// ------------------=
pub fn submit(owner: SecurityIdentity, capability: u64, text: &[u8]) -> Result<(), AiError> {
    submit_span(owner, capability, text, 0, text.len(), true)
}
// ------------------------=
// FUNC: can_prefetch
// DESC: Reports that DMA owns the current phrase and the worker can prepare exactly one following phrase.
// ------------------=
pub fn can_prefetch() -> bool {
    let state = STATE.load(Ordering::Acquire);
    !CANCEL.load(Ordering::Acquire) && state == 8
}
// ------------------------=
// FUNC: record_playback_start
// DESC: Records the first successful hardware start once per response, preserving it across resident batch rollover.
// ------------------=
fn record_playback_start() {
    let _ = FIRST_PLAYBACK_NS.compare_exchange(0, super::qwen::workers::clock_ns(), Ordering::AcqRel, Ordering::Acquire);
}
// ------------------------=
// FUNC: seal_buffered
// DESC: Starts one continuous playback session after every prepared response span is resident.
// ------------------=
pub fn seal_buffered(owner: SecurityIdentity) -> bool {
    unsafe {
        if owner != OWNER || !STREAM_OPEN || STATE.load(Ordering::Acquire) != 8 { return false; }
        if crate::drivers::audio::infinity_audio_seal(owner, STREAM_GENERATION.load(Ordering::Acquire) as u64) {
            record_playback_start();
            PLAYING = true; STATE.store(5, Ordering::Release); trace(b"continuous playback started");
            let _ = super::voice_input::prepare();
            true
        } else {
            crate::drivers::audio::stop_playback(owner); STREAM_OPEN = false; ERROR = 7; STATE.store(4, Ordering::Release); false
        }
    }
}
// ------------------------=
// FUNC: playback_progress
// DESC: Returns the actual DMA cursor and resident phrase duration used to synchronize visible response text with speech.
// ------------------=
pub fn playback_progress(owner: SecurityIdentity) -> Option<PlaybackProgress> {
    unsafe {
        if !PLAYING || OWNER != owner { return None; }
        let generation = STREAM_GENERATION.load(Ordering::Acquire);
        crate::drivers::audio::infinity_audio_progress(owner, generation as u64).map(|progress| PlaybackProgress {
            sequence: progress.generation as usize,
            frames: progress.played_frames as usize,
            total_frames: progress.queued_frames as usize,
            content_position: progress.content_position,
            content_boundary: progress.content_boundary,
        })
    }
}
// ------------------------=
// FUNC: echo_reference
// DESC: Copies recent own-session DMA audio at the microphone rate; only the desktop owner reads immutable playing storage.
// ------------------=
pub fn echo_reference(owner: SecurityIdentity, output: &mut [i16]) -> bool {
    unsafe { PLAYING && owner == OWNER && crate::drivers::audio::infinity_audio_echo_reference(
        owner, STREAM_GENERATION.load(Ordering::Acquire) as u64, output) }
}
// ------------------------=
// FUNC: stop
// DESC: Cancels only the owning session's job; worker buffers are not reused until it acknowledges cancellation.
// ------------------=
pub fn stop(owner: SecurityIdentity) -> bool {
    if STATE.load(Ordering::Acquire) == 0 || unsafe { OWNER != owner } { return false; }
    CANCEL.store(true, Ordering::Release);
    unsafe {
        if PLAYING || STREAM_OPEN { crate::drivers::audio::stop_playback(owner); }
        if STATE.load(Ordering::Acquire) == 8 { STATE.store(6, Ordering::Release); retire(); }
    }
    true
}
// ------------------------=
// FUNC: status
// DESC: Provides non-private diagnostics, reading worker metrics only after completion publication.
// ------------------=
pub fn status() -> OutputStatus {
    let state = STATE.load(Ordering::Acquire);
    OutputStatus { state: match state {1|WAITING_ENGINE=>OutputState::Queued,2=>OutputState::Synthesizing,3=>OutputState::Ready,4=>OutputState::Failed,5=>OutputState::Speaking,6=>OutputState::Cancelled,7=>OutputState::Complete,8=>OutputState::Buffered,_=>OutputState::Idle},
        frames: if (3..=8).contains(&state) { unsafe { FRAMES } } else { 0 }, peak_bytes: if (3..=8).contains(&state) { unsafe { PEAK } } else { 0 },
        synthesis_ns: if (3..=8).contains(&state) { unsafe { ELAPSED } } else { 0 },
        queued_ns: RESPONSE_QUEUED_NS.load(Ordering::Acquire),
        queue_wait_ns: QUEUE_WAIT_NS.load(Ordering::Acquire),
        first_playback_ns: FIRST_PLAYBACK_NS.load(Ordering::Acquire),
        error: if (3..=8).contains(&state) { unsafe { ERROR } } else { 0 } }
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
    (&mut *(&raw mut TEXT)).fill(0);
    let pcm_used = if ERROR == 0 { FRAMES.min(CAPACITY) } else { CAPACITY };
    (&mut *(&raw mut PCM))[..pcm_used].fill(0);
    (&mut *(&raw mut RESIDENT)).0[..OUTPUT_SAMPLES.min(RESIDENT_CAPACITY)].fill(0);
}

// ------------------------=
// FUNC: publish_resident_span
// DESC: Appends complete spans, draining a full resident generation before publishing its retained overflow span into the next generation.
// ------------------=
unsafe fn publish_resident_span() -> bool {
    if !STREAM_OPEN {
        let next = STREAM_GENERATION.load(Ordering::Acquire).wrapping_add(1).max(1);
        STREAM_GENERATION.store(next, Ordering::Release);
    }
    let generation = STREAM_GENERATION.load(Ordering::Acquire) as u64;
    if STREAM_OPEN {
        match crate::drivers::audio::infinity_audio_can_append(
            OWNER, generation, OUTPUT_SAMPLES, CONTENT_START, CONTENT_END,
        ) {
            None => return true,
            Some(false) => {
                // Keep the completed worker span private until DMA releases the
                // old immutable generation. Never discard prepared response
                // audio simply because the fixed resident queue is full.
                if crate::drivers::audio::infinity_audio_seal(OWNER, generation) {
                    record_playback_start();
                    PLAYING = true;
                    PENDING_AFTER_DRAIN = true;
                    PLAYBACK_DRAINED = false;
                    trace(b"resident batch playback started; next span retained");
                    return true;
                }
                crate::drivers::audio::stop_playback(OWNER);
                STREAM_OPEN = false;
                ERROR = 7;
                STATE.store(4, Ordering::Release);
                retire();
                return false;
            }
            Some(true) => {}
        }
    }
    let opened = STREAM_OPEN || crate::drivers::audio::infinity_audio_open(OWNER, CAPABILITY, generation);
    if opened && crate::drivers::audio::infinity_audio_append(
        OWNER, CAPABILITY, generation, &(&*(&raw const RESIDENT)).0[..OUTPUT_SAMPLES],
        CONTENT_START, CONTENT_END,
    ) {
        STREAM_OPEN = true;
        CAPABILITY = 0;
        (&mut *(&raw mut PCM))[..FRAMES.min(CAPACITY)].fill(0);
        (&mut *(&raw mut RESIDENT)).0[..OUTPUT_SAMPLES.min(RESIDENT_CAPACITY)].fill(0);
        (&mut *(&raw mut TEXT)).fill(0);
        if !FINAL_CHUNK {
            STATE.store(8, Ordering::Release);
            trace(b"phrase buffered");
            true
        } else if crate::drivers::audio::infinity_audio_seal(OWNER, generation) {
            record_playback_start();
            PLAYING = true;
            PLAYBACK_DRAINED = false;
            PENDING_AFTER_DRAIN = false;
            STATE.store(5, Ordering::Release);
            trace(b"continuous response playback started");
            let _ = super::voice_input::prepare();
            true
        } else {
            crate::drivers::audio::stop_playback(OWNER);
            STREAM_OPEN = false;
            ERROR = 7;
            STATE.store(4, Ordering::Release);
            retire();
            false
        }
    } else {
        if opened { crate::drivers::audio::stop_playback(OWNER); }
        STREAM_OPEN = false;
        ERROR = 7;
        STATE.store(4, Ordering::Release);
        retire();
        false
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
            if CANCEL.load(Ordering::Acquire) { crate::drivers::audio::stop_playback(OWNER); }
            if let Some(playback) = crate::drivers::audio::playback_state() {
                if playback != PlaybackState::Playing {
                    PLAYING = false; STREAM_OPEN = false;
                    if playback != PlaybackState::Complete {
                        CANCEL.store(true, Ordering::Release);
                        PENDING_AFTER_DRAIN = false;
                    } else if matches!(state, 1 | 2 | 3 | WAITING_ENGINE) {
                        PLAYBACK_DRAINED = true;
                    }
                    if state == 5 && !PENDING_AFTER_DRAIN {
                        state = if playback == PlaybackState::Complete { 7 } else { 6 };
                        STATE.store(state, Ordering::Release); retire();
                    }
                }
            }
        }
        state = STATE.load(Ordering::Acquire);
        if state <= 3 || state == WAITING_ENGINE {
            let now = super::qwen::workers::clock_ns() / 1_000_000_000;
            let valid = crate::runtime::with_runtime(|r| r.capabilities.validate(CAPABILITY, OWNER,
                CapabilityType::AudioOutput, 0, 1, 0, now).is_ok()).unwrap_or(false);
            if !valid { CANCEL.store(true, Ordering::Release); }
        }
        if state == WAITING_ENGINE {
            DISPATCHED = false;
            state = 1;
            STATE.store(state, Ordering::Release);
        }
        if state == 1 && !DISPATCHED {
            if cancelled() != 0 { STATE.store(6, Ordering::Release); retire(); }
            else { DISPATCHED = super::qwen::workers::background(worker); }
            return;
        }
        if state == 3 {
            if CANCEL.load(Ordering::Acquire) { STATE.store(6, Ordering::Release); retire(); }
            else if !PENDING_AFTER_DRAIN || PLAYBACK_DRAINED {
                PENDING_AFTER_DRAIN = false;
                PLAYBACK_DRAINED = false;
                let _ = publish_resident_span();
            }
        } else if matches!(state, 4 | 6 | 7) && CAPABILITY != 0 { retire(); }
    }
}
