//! One bounded native recognizer job. The UI never runs the decoder itself.
use super::{types::AiError, voice::SpeechRecognitionProvider};
use crate::runtime::{capability::CapabilityType, execution::SecurityIdentity};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static STATE: AtomicUsize = AtomicUsize::new(0);
static CANCEL: AtomicBool = AtomicBool::new(false);
static mut OWNER: SecurityIdentity = SecurityIdentity([0; 16]);
static mut CAPABILITY: u64 = 0;
static mut DEADLINE: u64 = 0;
static mut PCM: [i16; 160000] = [0; 160000];
static mut SAMPLES: usize = 0;
static mut TEXT: [u8; 512] = [0; 512];
static mut LENGTH: usize = 0;
static mut MEMORY: usize = 0;
static mut ELAPSED: u64 = 0;
static mut ERROR: i32 = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputState {
    Idle,
    Queued,
    Recognizing,
    Ready,
    Failed,
    Cancelled,
}
#[derive(Clone, Copy)]
pub struct InputStatus {
    pub state: InputState,
    pub error: i32,
    pub elapsed_ns: u64,
    pub heap_bytes: usize,
}
unsafe extern "C" {
    fn infinity_kokoro_native_recognize(
        pcm: *const i16,
        samples: usize,
        text: *mut u8,
        capacity: usize,
        length: *mut usize,
        memory: *mut usize,
        cancel: extern "C" fn(*mut core::ffi::c_void) -> i32,
        context: *mut core::ffi::c_void,
    ) -> i32;
}
struct Whisper;
impl SpeechRecognitionProvider for Whisper {
    // ------------------------=
    // FUNC: recognize_pcm
    // DESC: Executes real native English recognition through the existing provider-neutral interface.
    // ------------------=
    fn recognize_pcm(&mut self, pcm: &[i16], rate: u32, out: &mut [u8]) -> Result<usize, AiError> {
        if rate != 16000 {
            return Err(AiError::InvalidRequest);
        }
        unsafe {
            let mut length = 0;
            let mut memory = 0;
            ERROR = infinity_kokoro_native_recognize(
                pcm.as_ptr(),
                pcm.len(),
                out.as_mut_ptr(),
                out.len(),
                &mut length,
                &mut memory,
                cancelled,
                core::ptr::null_mut(),
            );
            MEMORY = memory;
            if ERROR == 0 {
                Ok(length)
            } else {
                Err(AiError::ProviderUnavailable)
            }
        }
    }
}
// ------------------------=
// FUNC: cancelled
// DESC: Checks cancellation and a monotonic deadline without acquiring UI or runtime locks on an AP.
// ------------------=
extern "C" fn cancelled(_: *mut core::ffi::c_void) -> i32 {
    (CANCEL.load(Ordering::Acquire) || super::qwen::workers::clock_ns() >= unsafe { DEADLINE })
        as i32
}
// ------------------------=
// FUNC: worker
// DESC: Owns private utterance storage until real recognition completes, then erases PCM before publication.
// ------------------=
unsafe fn worker() {
    STATE.store(2, Ordering::Release);
    let start = super::qwen::workers::clock_ns();
    LENGTH = Whisper
        .recognize_pcm(
            &(&*(&raw const PCM))[..SAMPLES],
            16000,
            &mut *(&raw mut TEXT),
        )
        .unwrap_or(0);
    (&mut *(&raw mut PCM)).fill(0);
    ELAPSED = super::qwen::workers::clock_ns().saturating_sub(start);
    let state = if cancelled(core::ptr::null_mut()) != 0 || ERROR == 2 {
        5
    } else if ERROR == 0 {
        3
    } else {
        4
    };
    if state != 3 {
        (&mut *(&raw mut TEXT)).fill(0);
        LENGTH = 0;
    }
    STATE.store(state, Ordering::Release);
}
// ------------------------=
// FUNC: submit
// DESC: Validates microphone ownership and schedules bounded native recognition without UI-thread fallback.
// ------------------=
pub fn submit(owner: SecurityIdentity, capability: u64, pcm: &[i16]) -> Result<(), AiError> {
    if pcm.is_empty() || pcm.len() > 160000 {
        return Err(AiError::InvalidRequest);
    }
    if matches!(STATE.load(Ordering::Acquire), 1 | 2 | 3) {
        return Err(AiError::QueueFull);
    }
    let now = super::qwen::workers::clock_ns();
    let request = crate::runtime::iop::IopMessage::request(
        crate::runtime::iop::OperationId::SpeechRecognize,
        now,
        owner,
        capability,
        now / 1_000_000_000 + 45,
        now,
        &[],
    )
    .map_err(|_| AiError::InvalidRequest)?;
    let valid = crate::runtime::with_runtime(|r| {
        super::voice::authorize_recognition(
            &request,
            owner,
            pcm.len(),
            &r.capabilities,
            now / 1_000_000_000,
        )
        .is_ok()
    })
    .unwrap_or(false);
    if !valid {
        return Err(AiError::AccessDenied);
    }
    unsafe {
        OWNER = owner;
        CAPABILITY = capability;
        DEADLINE = now.saturating_add(45_000_000_000);
        (&mut *(&raw mut PCM))[..pcm.len()].copy_from_slice(pcm);
        SAMPLES = pcm.len();
        (&mut *(&raw mut TEXT)).fill(0);
        LENGTH = 0;
        ERROR = 0;
        MEMORY = 0;
        ELAPSED = 0;
        CANCEL.store(false, Ordering::Release);
        STATE.store(1, Ordering::Release);
        if !super::qwen::workers::background(worker) {
            (&mut *(&raw mut PCM)).fill(0);
            STATE.store(4, Ordering::Release);
            return Err(AiError::ProviderUnavailable);
        }
    }
    Ok(())
}
// ------------------------=
// FUNC: stop
// DESC: Cancels only the owning session without racing a worker's private storage.
// ------------------=
pub fn stop(owner: SecurityIdentity) -> bool {
    if STATE.load(Ordering::Acquire) == 0 || unsafe { OWNER != owner } {
        return false;
    }
    CANCEL.store(true, Ordering::Release);
    if STATE.load(Ordering::Acquire) >= 3 {
        unsafe {
            (&mut *(&raw mut TEXT)).fill(0);
            LENGTH = 0;
        }
        STATE.store(5, Ordering::Release);
    }
    true
}
// ------------------------=
// FUNC: status
// DESC: Exposes only non-private metrics after the worker release-publishes completion.
// ------------------=
pub fn status() -> InputStatus {
    let state = STATE.load(Ordering::Acquire);
    InputStatus {
        state: match state {
            1 => InputState::Queued,
            2 => InputState::Recognizing,
            3 => InputState::Ready,
            4 => InputState::Failed,
            5 => InputState::Cancelled,
            _ => InputState::Idle,
        },
        error: if state >= 3 { unsafe { ERROR } } else { 0 },
        elapsed_ns: if state >= 3 { unsafe { ELAPSED } } else { 0 },
        heap_bytes: if state >= 3 { unsafe { MEMORY } } else { 0 },
    }
}
// ------------------------=
// FUNC: take
// DESC: Delivers a transcript once to its authorized owner and erases the provider's copy.
// ------------------=
pub fn take(owner: SecurityIdentity, out: &mut [u8]) -> Result<usize, AiError> {
    if STATE.load(Ordering::Acquire) != 3 {
        return Err(AiError::QueueFull);
    }
    unsafe {
        if OWNER != owner {
            return Err(AiError::AccessDenied);
        }
        let valid = crate::runtime::with_runtime(|r| {
            r.capabilities
                .validate(
                    CAPABILITY,
                    owner,
                    CapabilityType::AudioInput,
                    0,
                    1,
                    0,
                    super::qwen::workers::clock_ns() / 1_000_000_000,
                )
                .is_ok()
        })
        .unwrap_or(false);
        if !valid || CANCEL.load(Ordering::Acquire) {
            stop(owner);
            return Err(AiError::AccessDenied);
        }
        if out.len() < LENGTH {
            return Err(AiError::InvalidRequest);
        }
        let length = LENGTH;
        out[..length].copy_from_slice(&(&*(&raw const TEXT))[..length]);
        (&mut *(&raw mut TEXT)).fill(0);
        LENGTH = 0;
        STATE.store(0, Ordering::Release);
        Ok(length)
    }
}
// ------------------------=
// FUNC: poll
// DESC: Revocation cancels ongoing decoding and prevents unauthorized transcript publication.
// ------------------=
pub fn poll() {
    if !matches!(STATE.load(Ordering::Acquire), 1 | 2 | 3) {
        return;
    }
    unsafe {
        let valid = crate::runtime::with_runtime(|r| {
            r.capabilities
                .validate(
                    CAPABILITY,
                    OWNER,
                    CapabilityType::AudioInput,
                    0,
                    1,
                    0,
                    super::qwen::workers::clock_ns() / 1_000_000_000,
                )
                .is_ok()
        })
        .unwrap_or(false);
        if !valid || cancelled(core::ptr::null_mut()) != 0 {
            stop(OWNER);
        }
    }
}
