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
static mut CHAT_TURN: u64 = 0;
static mut BETWEEN_PHRASES: bool = false;
static mut CONTINUOUS: bool = false;
static mut GAP_SAMPLES: usize = 0;
static mut GAP_DEADLINE: u64 = 0;
static mut ECHO_REMAINING: usize = 0;
// Capture only after playback has stopped. Discard a short acoustic tail, then
// inspect 600 ms of real microphone samples (not a blocking desktop sleep).
const ECHO_TAIL_SAMPLES: usize = 3200;
const INTERRUPT_SAMPLES: usize = 9600;

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
    BETWEEN_PHRASES = false;
    ECHO_REMAINING = 0;
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
        CONTINUOUS = true;
        RESTART_LISTENING = false;
        BETWEEN_PHRASES = false;
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
// FUNC: toggle
// DESC: Treats a click during asynchronous stopping as a restart request, or cancels that request on the next click.
// ------------------=
pub fn toggle(owner: SecurityIdentity) -> bool {
    unsafe {
        if matches!(STATE, State::Off | State::Failed)
            || (STATE == State::Stopping && !RESTART_LISTENING)
        {
            start(owner)
        } else {
            stop(owner)
        }
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
        BETWEEN_PHRASES = false;
        crate::drivers::audio::stop_capture(owner);
        ECHO_REMAINING = 0;
        INPUT_CAP = 0;
        voice_input::stop(owner);
        voice_output::stop(owner);
        if STATE == State::Thinking {
            super::with_ai_runtime(|ai| {
                if ai.chat.turn_id() == CHAT_TURN {
                    ai.cancel_chat();
                }
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
// DESC: Plays one short phrase immediately instead of waiting for an entire 160-character reply chunk.
// ------------------=
unsafe fn speak_next() -> bool {
    let enabled = crate::runtime::with_runtime(|r| {
        (0..crate::runtime::identity::MAX_SESSIONS).filter_map(|i| r.identity.session_nth(i))
            .find(|s| s.id.0 == OWNER.0 && s.state == SessionState::Active)
            .and_then(|s| r.identity.ai_profile(s.user)).map(|p| p.speech_output_enabled).unwrap_or(false)
    }).unwrap_or(false);
    if !enabled { REPLY_AT = REPLY_LENGTH; }
    while REPLY_AT < REPLY_LENGTH && REPLY[REPLY_AT] == b' ' { REPLY_AT += 1; }
    if REPLY_AT >= REPLY_LENGTH {
        if CONTINUOUS { return listen(); }
        STATE = State::Off;
        (&mut *(&raw mut REPLY)).fill(0);
        return true;
    }
    let remaining = &(&*(&raw const REPLY))[REPLY_AT..REPLY_LENGTH];
    let mut count = remaining.len().min(44);
    if let Some(end) = remaining[..count].iter().enumerate().find_map(|(i, b)| {
        (matches!(*b, b'.' | b'!' | b'?') && (i + 1 == remaining.len() || remaining[i + 1] == b' ')).then_some(i + 1)
    }) {
        count = end;
    }
    if count < remaining.len() && !matches!(remaining[count - 1], b'.' | b'!' | b'?') {
        if let Some(split) = remaining[..count]
            .iter()
            .rposition(|b| *b == b' ')
        {
            if split > 0 {
                count = split + 1;
            }
        }
    }
    let Some(cap) = grant(OWNER, CapabilityType::AudioOutput, voice_output::OUTPUT_LEASE_SECONDS) else {
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
// FUNC: check_between_phrases
// DESC: Reopens a fresh microphone stream only after the preceding playback completes.
// ------------------=
unsafe fn check_between_phrases() -> bool {
    if !CONTINUOUS { return speak_next(); }
    let pending = REPLY_AT < REPLY_LENGTH;
    if !listen() { return false; }
    BETWEEN_PHRASES = pending;
    ECHO_REMAINING = ECHO_TAIL_SAMPLES;
    GAP_SAMPLES = 0;
    GAP_DEADLINE = super::qwen::workers::clock_ns().saturating_add(3_000_000_000);
    true
}
// ------------------------=
// FUNC: speak_completed_reply
// DESC: Queues a typed reply on the existing background voice path without opening the microphone.
// ------------------=
pub fn speak_completed_reply(owner: SecurityIdentity, turn: u64) {
    unsafe {
        if !matches!(STATE, State::Off | State::Failed) || !active_owner(owner) { return; }
        OWNER = owner;
        CONTINUOUS = false;
        RESTART_LISTENING = false;
        CHAT_TURN = turn;
        STATE = State::Thinking;
    }
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
        if !CONTINUOUS && matches!(STATE, State::Thinking | State::Speaking)
            && super::with_ai_runtime(|ai| ai.chat.turn_id() != CHAT_TURN) {
            stop(OWNER);
        }
        match STATE {
            State::Listening => {
                let now = super::qwen::workers::clock_ns();
                if BETWEEN_PHRASES && now >= GAP_DEADLINE {
                    // A stalled microphone must not be mistaken for checked silence.
                    stop(OWNER);
                    return true;
                }
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
                    let discard = ECHO_REMAINING.min(n);
                    ECHO_REMAINING -= discard;
                    GAP_SAMPLES = GAP_SAMPLES.saturating_add(n);
                    (&mut *(&raw mut UTTERANCE)).push(&(&*(&raw const MONO))[discard..n]);
                    if BETWEEN_PHRASES {
                        if matches!((&*(&raw const UTTERANCE)).state(), VadState::Speech | VadState::Complete) {
                            // Keep the captured onset/preroll; discard only the obsolete assistant reply.
                            BETWEEN_PHRASES = false;
                            (&mut *(&raw mut REPLY)).fill(0);
                            REPLY_AT = 0; REPLY_LENGTH = 0;
                        } else if GAP_SAMPLES >= ECHO_TAIL_SAMPLES + INTERRUPT_SAMPLES && LEVEL < 300 {
                            crate::drivers::audio::stop_capture(OWNER);
                            INPUT_CAP = 0;
                            BETWEEN_PHRASES = false;
                            (&mut *(&raw mut UTTERANCE)).clear(300);
                            (&mut *(&raw mut RESAMPLER)).clear();
                            (&mut *(&raw mut RAW)).fill(0);
                            (&mut *(&raw mut MONO)).fill(0);
                            LEVEL = 0;
                            if !speak_next() { stop(OWNER); }
                            return true;
                        }
                    }
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
                                let accepted = ai.submit_chat();
                                if accepted {
                                    CHAT_TURN = ai.chat.turn_id();
                                }
                                accepted
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
                if super::with_ai_runtime(|ai| ai.chat.turn_id() != CHAT_TURN) {
                    stop(OWNER);
                    return true;
                }
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
                    if !check_between_phrases() {
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
                        if !start(OWNER) {
                            STATE = State::Failed;
                        }
                    }
                }
            }
            _ => {}
        }
        STATE != previous
    }
}
