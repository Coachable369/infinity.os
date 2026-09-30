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
#[path = "speech_chunk.rs"]
mod speech_chunk;
#[path = "voice_echo.rs"]
mod voice_echo;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Off,
    Listening,
    Recognizing,
    Submitting,
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
static mut ECHO: [i16; 9600] = [0; 9600];
static mut ECHO_VALID_UNTIL: u64 = 0;
static mut TRANSCRIPT: [u8; 512] = [0; 512];
static mut REPLY: [u8; 16384] = [0; 16384];
static mut REPLY_LENGTH: usize = 0;
static mut REPLY_AT: usize = 0;
static mut LEVEL: u16 = 0;
static mut RESTART_LISTENING: bool = false;
static mut CHAT_TURN: u64 = 0;
static mut CONTINUOUS: bool = false;
static mut REPLY_COMPLETE: bool = false;

#[cfg(not(test))]
// ------------------------=
// FUNC: trace
// DESC: Emits privacy-safe conversation state checkpoints to the VM serial trace.
// ------------------=
fn trace(event: &[u8]) {
    unsafe {
        let mut record = [0u8; 96];
        let prefix = b"[VOICE] ";
        let event_length = event.len().min(record.len() - prefix.len() - 1);
        record[..prefix.len()].copy_from_slice(prefix);
        record[prefix.len()..prefix.len() + event_length]
            .copy_from_slice(&event[..event_length]);
        record[prefix.len() + event_length] = b'\n';
        crate::output::write(&record[..prefix.len() + event_length + 1]);
    }
}

#[cfg(test)]
// ------------------------=
// FUNC: trace
// DESC: Keeps host conversation harnesses independent of the native serial device.
// ------------------=
fn trace(_: &[u8]) {}

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
// FUNC: submit_pending_transcript
// DESC: Submits a recognized composer only after its selected local model becomes ready.
// ------------------=
unsafe fn submit_pending_transcript() -> bool {
    super::with_ai_runtime(|ai| {
        if ai.chat.input().is_empty()
            || ai.chat.generation_state == GenerationState::Running
        {
            return false;
        }
        if !ai.submit_chat() { return false; }
        CHAT_TURN = ai.chat.turn_id();
        REPLY_AT = 0;
        REPLY_LENGTH = 0;
        REPLY_COMPLETE = false;
        trace(b"model turn accepted");
        true
    })
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
    if INPUT_CAP != 0 {
        STATE = State::Listening;
        return true;
    }
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
        // Microphone availability is independent of model loading.  Fresh
        // installs must begin listening while the selected model is still
        // being prepared, rather than presenting a misleading VOICE OFF state.
        let ready = super::with_ai_runtime(|ai| {
            ai.chat.input().is_empty()
                && ai.chat.generation_state != GenerationState::Running
        });
        if !ready || !crate::drivers::audio::capture_available() {
            return false;
        }
        OWNER = owner;
        CONTINUOUS = true;
        RESTART_LISTENING = false;
        if !listen() {
            STATE = State::Failed;
            trace(b"initial capture failed");
            return false;
        }
        trace(b"conversation listening");
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
        crate::drivers::audio::stop_capture(owner);
        INPUT_CAP = 0;
        voice_input::stop(owner);
        voice_output::stop(owner);
        if matches!(STATE, State::Thinking | State::Speaking) {
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
        (&mut *(&raw mut ECHO)).fill(0);ECHO_VALID_UNTIL=0;
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
// DESC: Queues a complete sentence or bounded word-aligned segment from the visible response.
// ------------------=
unsafe fn speak_next() -> bool {
    let enabled = crate::runtime::with_runtime(|r| {
        (0..crate::runtime::identity::MAX_SESSIONS).filter_map(|i| r.identity.session_nth(i))
            .find(|s| s.id.0 == OWNER.0 && s.state == SessionState::Active)
            .and_then(|s| r.identity.ai_profile(s.user)).map(|p| p.speech_output_enabled).unwrap_or(false)
    }).unwrap_or(false);
    if !enabled {
        trace(b"speech output disabled by profile");
        REPLY_AT = REPLY_LENGTH;
    }
    while REPLY_AT < REPLY_LENGTH && REPLY[REPLY_AT] == b' ' { REPLY_AT += 1; }
    if REPLY_AT >= REPLY_LENGTH {
        if !REPLY_COMPLETE {
            STATE = State::Thinking;
            return true;
        }
        trace(b"reply drained");
        if CONTINUOUS { return listen(); }
        STATE = State::Off;
        (&mut *(&raw mut REPLY)).fill(0);
        return true;
    }
    let remaining = &(&*(&raw const REPLY))[REPLY_AT..REPLY_LENGTH];
    let count = speech_chunk::next(remaining, REPLY_COMPLETE, REPLY_AT == 0);
    if count == 0 { STATE = State::Thinking; return true; }
    let Some(cap) = grant(OWNER, CapabilityType::AudioOutput, voice_output::OUTPUT_LEASE_SECONDS) else {
        trace(b"speech output lease denied");
        return false;
    };
    if voice_output::submit(OWNER, cap, &remaining[..count]).is_err() {
        trace(b"speech submission rejected");
        retire(cap);
        return false;
    }
    trace(b"speech queued");
    REPLY_AT += count;
    STATE = State::Speaking;
    true
}
// ------------------------=
// FUNC: refresh_reply
// DESC: Mirrors cumulative visible model output while preserving the already queued speech cursor.
// ------------------=
unsafe fn refresh_reply() {
    super::with_ai_runtime(|ai| {
        REPLY_COMPLETE = ai.chat.generation_state == GenerationState::Complete;
        if let Some(message) = ai.chat.message(ai.chat.message_count().saturating_sub(1)).filter(|m| m.role == ChatRole::Assistant) {
            REPLY_LENGTH = 0;
            for &byte in message.text() {
                if REPLY_LENGTH == 16384 { break; }
                if byte.is_ascii() {
                    REPLY[REPLY_LENGTH] = if (32..=126).contains(&byte) { byte } else { b' ' };
                    REPLY_LENGTH += 1;
                }
            }
        }
    });
}
// ------------------------=
// FUNC: speak_visible_reply
// DESC: Starts sentence buffering as visible output arrives, preserving an enabled conversation's listening preference.
// ------------------=
pub fn speak_visible_reply(owner: SecurityIdentity, turn: u64) {
    unsafe {
        if !matches!(STATE, State::Off | State::Failed | State::Listening) || !active_owner(owner) { return; }
        if STATE == State::Listening && OWNER != owner { return; }
        let continuous = STATE == State::Listening;
        OWNER = owner;
        CONTINUOUS = continuous;
        REPLY_AT = 0; REPLY_LENGTH = 0; REPLY_COMPLETE = false;
        RESTART_LISTENING = false;
        CHAT_TURN = turn;
        STATE = State::Thinking;
    }
}
// ------------------------=
// FUNC: capture_frame
// DESC: Renews explicit microphone authority and feeds bounded echo-reduced input during listening and spoken replies.
// ------------------=
unsafe fn capture_frame(duplex: bool) -> bool {
    use crate::runtime::audio::CaptureState;
    let Some(status)=crate::drivers::audio::capture_status() else {return true;};
    if status.state != CaptureState::Recording {
        // A bounded DMA overrun or expired capture window is not a user mute.
        // Discard the incomplete utterance and acquire fresh authority, without
        // dropping the conversational turn. Permission/device failures stay fatal.
        if CONTINUOUS && matches!(status.state,CaptureState::Overrun|CaptureState::Complete) {
            let previous=STATE;
            retire(INPUT_CAP);INPUT_CAP=0;
            let reopened=listen();
            if reopened {STATE=previous;}
            return reopened;
        }
        return false;
    }
    let now=super::qwen::workers::clock_ns();
    if now>=RENEW_AT {
        let cap=grant(OWNER,CapabilityType::AudioInput,60).unwrap_or(0);
        if cap==0 || !crate::drivers::audio::renew_capture(OWNER,cap) {retire(cap);return false;}
        INPUT_CAP=cap;RENEW_AT=now+1_000_000_000;
    }
    let count=crate::drivers::audio::read_capture(OWNER,&mut *(&raw mut RAW));
    if count==0 {return true;}
    let (used,n)=(&mut *(&raw mut RESAMPLER)).process(&(&*(&raw const RAW))[..count],&mut *(&raw mut MONO));
    if used!=count {return false;}
    if duplex && voice_output::echo_reference(OWNER,&mut *(&raw mut ECHO)) {
        ECHO_VALID_UNTIL=now+200_000_000;
    }
    if now<ECHO_VALID_UNTIL {
        voice_echo::subtract(&mut (&mut *(&raw mut MONO))[..n],&*(&raw const ECHO));
    } else {
        (&mut *(&raw mut ECHO)).fill(0);
    }
    LEVEL=(&*(&raw const MONO))[..n].iter().map(|x|x.unsigned_abs()).max().unwrap_or(0);
    (&mut *(&raw mut UTTERANCE)).push(&(&*(&raw const MONO))[..n]);
    (&mut *(&raw mut RAW)).fill(0);(&mut *(&raw mut MONO)).fill(0);
    if duplex && matches!((&*(&raw const UTTERANCE)).state(),VadState::NoSpeech|VadState::Limit) {
        (&mut *(&raw mut UTTERANCE)).clear(300);
    }
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
        if matches!(STATE, State::Thinking | State::Speaking)
            && super::with_ai_runtime(|ai| ai.chat.turn_id() != CHAT_TURN || matches!(ai.chat.generation_state,
                GenerationState::Failed | GenerationState::Cancelled | GenerationState::ContextFull)) {
            stop(OWNER);
        }
        // Do not reopen the microphone while the model is still producing its
        // first audible response. A follow-up utterance at this point used to
        // cancel the accepted turn before any speech job reached synthesis.
        // Conversation state becomes Speaking when synthesis is queued, before
        // any PCM exists. Reopen duplex capture only after the output service
        // confirms hardware playback, otherwise microphone activity can cancel
        // Kokoro while it is still synthesizing the first phrase.
        if CONTINUOUS && STATE == State::Speaking
            && voice_output::status().state == voice_output::OutputState::Speaking {
            if INPUT_CAP == 0 {
                let previous=STATE;
                if !listen() {stop(OWNER);return true;}
                STATE=previous;
            }
            if !capture_frame(true) {stop(OWNER);return true;}
            let utterance=&*(&raw const UTTERANCE);
            if utterance.state()==VadState::Complete
                || (utterance.state()==VadState::Speech
                    && utterance.active_speech_samples()>=super::voice_vad::RATE/4) {
                // Keep the onset and preroll already captured. Do not use stop(),
                // which deliberately erases microphone storage on user disable.
                voice_output::stop(OWNER);
                super::with_ai_runtime(|ai| {if ai.chat.turn_id()==CHAT_TURN {ai.cancel_chat();}});
                (&mut *(&raw mut REPLY)).fill(0);REPLY_AT=0;REPLY_LENGTH=0;
                REPLY_COMPLETE=false;STATE=State::Listening;
            }
        }
        match STATE {
            State::Listening => {
                if !capture_frame(false) {stop(OWNER);return true;}
                match (&*(&raw const UTTERANCE)).state() {
                    VadState::Complete => {
                        // Capture shutdown retires its lease. Recognition is
                        // asynchronous, so it needs independent authority that
                        // remains valid until the transcript is taken.
                        RECOGNIZE_CAP = grant(OWNER,CapabilityType::AudioInput,15)
                            .unwrap_or(0);
                        let submitted = (&*(&raw const UTTERANCE))
                            .speech()
                            .map(|pcm| voice_input::submit(OWNER, RECOGNIZE_CAP, pcm).is_ok())
                            .unwrap_or(false);
                        crate::drivers::audio::stop_capture(OWNER);
                        INPUT_CAP = 0;
                        (&mut *(&raw mut UTTERANCE)).clear(300);
                        LEVEL = 0;
                        if submitted {
                            trace(b"recognition queued");
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
                    let queued = result
                        .ok()
                        .map(|n| {
                            super::with_ai_runtime(|ai| {
                                if !ai.chat.input().is_empty()
                                    || ai.chat.generation_state == GenerationState::Running
                                {
                                    return false;
                                }
                                let mut inserted = n != 0;
                                for &byte in &(&*(&raw const TRANSCRIPT))[..n] {
                                    inserted &= ai.chat.push_input(byte);
                                }
                                inserted
                            })
                        })
                        .unwrap_or(false);
                    (&mut *(&raw mut TRANSCRIPT)).fill(0);
                    if queued {
                        trace(b"transcript ready");
                        STATE = if submit_pending_transcript() {
                            trace(b"transcript submitted");
                            State::Thinking
                        } else {
                            State::Submitting
                        };
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
            State::Submitting => {
                if submit_pending_transcript() { STATE = State::Thinking; }
            }
            State::Thinking => {
                if super::with_ai_runtime(|ai| ai.chat.turn_id() != CHAT_TURN) {
                    stop(OWNER);
                    return true;
                }
                let generation = super::with_ai_runtime(|ai| ai.chat.generation_state);
                if matches!(generation, GenerationState::Running | GenerationState::Complete) {
                    refresh_reply();
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
                    refresh_reply();
                    if !speak_next() {
                        stop(OWNER);
                    }
                }
                voice_output::OutputState::Failed | voice_output::OutputState::Cancelled => {
                    trace(b"speech output failed or cancelled");
                    stop(OWNER);
                }
                _ => {
                    if voice_output::can_prefetch() {
                        refresh_reply();
                        if REPLY_AT < REPLY_LENGTH && speech_chunk::next(&(&*(&raw const REPLY))[REPLY_AT..REPLY_LENGTH], REPLY_COMPLETE, REPLY_AT == 0) > 0 {
                            if !speak_next() { stop(OWNER); }
                        }
                    }
                }
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
