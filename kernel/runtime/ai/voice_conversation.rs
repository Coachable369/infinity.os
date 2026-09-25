//! Explicitly enabled local conversation. All transitions run on the desktop owner;
//! decoding runs on a bounded AP and uses the existing local AI service.
use super::{
    chat::{ChatRole, GenerationState},
    voice_input, voice_output,
    voice_pcm::Resampler,
    voice_vad::{Utterance, VadState},
};
use crate::runtime::{
    capability::CapabilityType, execution::SecurityIdentity, identity::SessionState,
};
#[path = "../../ui/voice_indicator.rs"]
pub mod indicator;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Off,
    Listening,
    Recognizing,
    Thinking,
    Speaking,
    Stopping,
    Failed,
}
static mut STATE: State = State::Off;
static mut OWNER: SecurityIdentity = SecurityIdentity([0; 16]);
static mut INPUT_CAP: u64 = 0;
static mut RECOGNIZE_CAP: u64 = 0;
static mut RENEW_AT: u64 = 0;
static mut RESAMPLER: Resampler = Resampler::empty();
static mut UTTERANCE: Utterance = Utterance::new(300);
static mut RAW: [i16; 4800] = [0; 4800];
static mut MONO: [i16; 4800] = [0; 4800];
static mut TRANSCRIPT: [u8; 512] = [0; 512];
static mut REPLY: [u8; 16384] = [0; 16384];
static mut REPLY_LENGTH: usize = 0;
static mut REPLY_AT: usize = 0;
static mut LEVEL: u16 = 0;
static mut RESTART_LISTENING: bool = false;

// ------------------------=
// FUNC: active_owner
// DESC: Stops voice authority at logout or lock rather than relying only on capability expiration.
// ------------------=
fn active_owner(owner: SecurityIdentity) -> bool {
    crate::runtime::with_runtime(|r| {
        (0..crate::runtime::identity::MAX_SESSIONS)
            .filter_map(|i| r.identity.session_nth(i))
            .any(|s| s.id.0 == owner.0 && s.state == SessionState::Active)
    })
    .unwrap_or(false)
}
// ------------------------=
// FUNC: grant
// DESC: Grants a short direction-specific voice lease only for the active logged-in session.
// ------------------=
fn grant(owner: SecurityIdentity, kind: CapabilityType, seconds: u64) -> Option<u64> {
    if !active_owner(owner) {
        return None;
    }
    let now = super::qwen::workers::clock_ns() / 1_000_000_000;
    crate::runtime::with_runtime(|r| {
        r.capabilities
            .grant(kind, 0, 1, 0, owner, owner, Some(now + seconds), 0)
            .ok()
    })
    .flatten()
}
// ------------------------=
// FUNC: retire
// DESC: Retires a controller-owned lease without affecting other applications' audio authority.
// ------------------=
fn retire(cap: u64) {
    if cap != 0 {
        unsafe {
            crate::runtime::with_runtime(|r| {
                let _ = r.capabilities.retire_leaf(cap, OWNER);
            });
        }
    }
}
// ------------------------=
// FUNC: state
// DESC: Provides visible session state and audio level without exposing raw captured speech.
// ------------------=
pub fn state() -> (State, u16) {
    unsafe { (STATE, LEVEL) }
}
// ------------------------=
// FUNC: listen
// DESC: Opens a bounded capture lease and resets private utterance storage between conversational turns.
// ------------------=
unsafe fn listen() -> bool {
    let Some(cap) = grant(OWNER, CapabilityType::AudioInput, 60) else {
        return false;
    };
    if !crate::drivers::audio::capture(OWNER, cap) {
        retire(cap);
        return false;
    }
    INPUT_CAP = cap;
    let rate = crate::drivers::audio::capture_status()
        .map(|s| s.sample_rate)
        .unwrap_or(0);
    if !(&mut *(&raw mut RESAMPLER)).configure(rate) {
        crate::drivers::audio::stop_capture(OWNER);
        INPUT_CAP = 0;
        return false;
    }
    (&mut *(&raw mut UTTERANCE)).clear(300);
    LEVEL = 0;
    RENEW_AT = super::qwen::workers::clock_ns() + 1_000_000_000;
    STATE = State::Listening;
    true
}
// ------------------------=
// FUNC: start
// DESC: Enables the selected local model or cancels the current turn before explicitly restarting listening.
// ------------------=
pub fn start(owner: SecurityIdentity) -> bool {
    unsafe {
        if !active_owner(owner) {
            return false;
        }
        if !matches!(STATE, State::Off | State::Failed) {
            if OWNER != owner {
                return false;
            }
            if STATE == State::Listening {
                return true;
            }
            stop(owner);
            RESTART_LISTENING = true;
            return true;
        }
        let ready = super::with_ai_runtime(|ai| {
            ai.native_ready(ai.chat.selected_model())
                && ai.chat.input().is_empty()
                && ai.chat.generation_state != GenerationState::Running
        });
        if !ready || !crate::drivers::audio::capture_available() {
            return false;
        }
        OWNER = owner;
        RESTART_LISTENING = false;
        if !listen() {
            STATE = State::Failed;
            return false;
        }
        super::with_ai_runtime(|ai| {
            ai.bind_chat_owner(owner.0);
            ai.chat.set_enabled(true);
            ai.chat.set_minimized(false);
        });
        true
    }
}
// ------------------------=
// FUNC: stop
// DESC: Immediately closes the microphone and requests bounded worker, inference, and playback cancellation.
// ------------------=
pub fn stop(owner: SecurityIdentity) -> bool {
    unsafe {
        if OWNER != owner || STATE == State::Off {
            return false;
        }
        RESTART_LISTENING = false;
        crate::drivers::audio::stop_capture(owner);
        INPUT_CAP = 0;
        voice_input::stop(owner);
        voice_output::stop(owner);
        if STATE == State::Thinking {
            super::with_ai_runtime(|ai| {
                ai.cancel_chat();
            });
        }
        retire(RECOGNIZE_CAP);
        RECOGNIZE_CAP = 0;
        (&mut *(&raw mut UTTERANCE)).cancel();
        (&mut *(&raw mut RESAMPLER)).clear();
        (&mut *(&raw mut RAW)).fill(0);
        (&mut *(&raw mut MONO)).fill(0);
        (&mut *(&raw mut TRANSCRIPT)).fill(0);
        (&mut *(&raw mut REPLY)).fill(0);
        REPLY_LENGTH = 0;
        REPLY_AT = 0;
        LEVEL = 0;
        STATE = State::Stopping;
        true
    }
}
// ------------------------=
// FUNC: speak_next
// DESC: Sends bounded sentence chunks through the same cancellable native playback provider as Console speech.
// ------------------=
unsafe fn speak_next() -> bool {
    if REPLY_AT >= REPLY_LENGTH {
        return listen();
    }
    let remaining = &(&*(&raw const REPLY))[REPLY_AT..REPLY_LENGTH];
    let mut count = remaining.len().min(160);
    if count < remaining.len() {
        if let Some(split) = remaining[..count]
            .iter()
            .rposition(|b| *b == b' ' || *b == b'.')
        {
            if split > 0 {
                count = split + 1;
            }
        }
    }
    let Some(cap) = grant(OWNER, CapabilityType::AudioOutput, 40) else {
        return false;
    };
    if voice_output::submit(OWNER, cap, &remaining[..count]).is_err() {
        retire(cap);
        return false;
    }
    REPLY_AT += count;
    STATE = State::Speaking;
    true
}
// ------------------------=
// FUNC: poll
// DESC: Advances bounded capture, native recognition, Hermes, and speech states without decoding on the UI thread.
// ------------------=
pub fn poll() -> bool {
    unsafe {
        let previous = STATE;
        if matches!(STATE, State::Off | State::Failed) {
            return false;
        }
        if STATE != State::Stopping && !active_owner(OWNER) {
            stop(OWNER);
        }
        match STATE {
            State::Listening => {
                let now = super::qwen::workers::clock_ns();
                if now >= RENEW_AT {
                    let cap = grant(OWNER, CapabilityType::AudioInput, 60).unwrap_or(0);
                    if !crate::drivers::audio::renew_capture(OWNER, cap) {
                        retire(cap);
                        stop(OWNER);
                        return true;
                    }
                    INPUT_CAP = cap;
                    RENEW_AT = now + 1_000_000_000;
                }
                let Some(status) = crate::drivers::audio::capture_status() else {
                    return false;
                };
                if status.state != crate::runtime::audio::CaptureState::Recording {
                    stop(OWNER);
                    return true;
                }
                let count = crate::drivers::audio::read_capture(OWNER, &mut *(&raw mut RAW));
                if count != 0 {
                    let (used, n) = (&mut *(&raw mut RESAMPLER))
                        .process(&(&*(&raw const RAW))[..count], &mut *(&raw mut MONO));
                    if used != count {
                        stop(OWNER);
                        return true;
                    }
                    LEVEL = (&*(&raw const MONO))[..n]
                        .iter()
                        .map(|x| x.unsigned_abs())
                        .max()
                        .unwrap_or(0);
                    (&mut *(&raw mut UTTERANCE)).push(&(&*(&raw const MONO))[..n]);
                    (&mut *(&raw mut RAW)).fill(0);
                    (&mut *(&raw mut MONO)).fill(0);
                }
                match (&*(&raw const UTTERANCE)).state() {
                    VadState::Complete => {
                        RECOGNIZE_CAP = grant(OWNER, CapabilityType::AudioInput, 15).unwrap_or(0);
                        let submitted = (&*(&raw const UTTERANCE))
                            .speech()
                            .map(|pcm| voice_input::submit(OWNER, RECOGNIZE_CAP, pcm).is_ok())
                            .unwrap_or(false);
                        crate::drivers::audio::stop_capture(OWNER);
                        INPUT_CAP = 0;
                        (&mut *(&raw mut UTTERANCE)).clear(300);
                        LEVEL = 0;
                        if submitted {
                            STATE = State::Recognizing;
                        } else {
                            stop(OWNER);
                        }
                    }
                    VadState::NoSpeech | VadState::Limit => {
                        (&mut *(&raw mut UTTERANCE)).clear(300);
                    }
                    _ => {}
                }
            }
            State::Recognizing => match voice_input::status().state {
                voice_input::InputState::Ready => {
                    let result = voice_input::take(OWNER, &mut *(&raw mut TRANSCRIPT));
                    retire(RECOGNIZE_CAP);
                    RECOGNIZE_CAP = 0;
                    let submitted = result
                        .ok()
                        .map(|n| {
                            super::with_ai_runtime(|ai| {
                                if !ai.chat.input().is_empty()
                                    || ai.chat.generation_state == GenerationState::Running
                                {
                                    return false;
                                }
                                for &byte in &(&*(&raw const TRANSCRIPT))[..n] {
                                    ai.chat.push_input(byte);
                                }
                                ai.submit_chat()
                            })
                        })
                        .unwrap_or(false);
                    (&mut *(&raw mut TRANSCRIPT)).fill(0);
                    if submitted {
                        STATE = State::Thinking;
                    } else {
                        stop(OWNER);
                    }
                }
                voice_input::InputState::Failed | voice_input::InputState::Cancelled => {
                    let status = voice_input::status();
                    retire(RECOGNIZE_CAP);
                    RECOGNIZE_CAP = 0;
                    // Only an acoustic no-hypothesis result may resume capture.
                    // Cancellation, revoked authority, and decoder faults must
                    // not silently obtain a fresh microphone lease.
                    if status.state == voice_input::InputState::Failed && status.error == 5 {
                        if !listen() {
                            stop(OWNER);
                        }
                    } else {
                        stop(OWNER);
                        STATE = State::Failed;
                    }
                }
                _ => {}
            },
            State::Thinking => {
                let generation = super::with_ai_runtime(|ai| ai.chat.generation_state);
                if generation == GenerationState::Complete {
                    REPLY_LENGTH = 0;
                    REPLY_AT = 0;
                    (&mut *(&raw mut REPLY)).fill(0);
                    super::with_ai_runtime(|ai| {
                        if let Some(message) = ai
                            .chat
                            .message(ai.chat.message_count().saturating_sub(1))
                            .filter(|m| m.role == ChatRole::Assistant)
                        {
                            for &byte in message.text() {
                                if REPLY_LENGTH == 16384 {
                                    break;
                                }
                                if byte.is_ascii() {
                                    REPLY[REPLY_LENGTH] = if (32..=126).contains(&byte) {
                                        byte
                                    } else {
                                        b' '
                                    };
                                    REPLY_LENGTH += 1;
                                }
                            }
                        }
                    });
                    if !speak_next() {
                        stop(OWNER);
                    }
                } else if matches!(
                    generation,
                    GenerationState::Failed
                        | GenerationState::Cancelled
                        | GenerationState::ContextFull
                ) {
                    stop(OWNER);
                }
            }
            State::Speaking => match voice_output::status().state {
                voice_output::OutputState::Complete => {
                    if !speak_next() {
                        stop(OWNER);
                    }
                }
                voice_output::OutputState::Failed | voice_output::OutputState::Cancelled => {
                    stop(OWNER);
                }
                _ => {}
            },
            State::Stopping => {
                if !matches!(
                    voice_input::status().state,
                    voice_input::InputState::Queued | voice_input::InputState::Recognizing
                ) && !matches!(
                    voice_output::status().state,
                    voice_output::OutputState::Queued
                        | voice_output::OutputState::Synthesizing
                        | voice_output::OutputState::Ready
                        | voice_output::OutputState::Speaking
                ) {
                    STATE = State::Off;
                    if RESTART_LISTENING {
                        RESTART_LISTENING = false;
                        start(OWNER);
                    }
                }
            }
            _ => {}
        }
        STATE != previous
    }
}
