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
static mut REOPEN_AT: u64 = 0;
static mut REOPEN_DEADLINE: u64 = 0;
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
static mut SYNC_VISIBLE_AT: usize = 0;
static mut LEVEL: u16 = 0;
static mut RESTART_LISTENING: bool = false;
static mut CHAT_TURN: u64 = 0;
static mut CONTINUOUS: bool = false;
static mut REPLY_COMPLETE: bool = false;
static mut REOPEN_FAILURE: u8 = 0;
static mut CAPTURE_FRAME_SEEN: bool = false;
static mut CAPTURE_EMPTY_SINCE: u64 = 0;
static mut CAPTURE_EMPTY_REPORTED: bool = false;
static mut LAST_VAD_STATE: VadState = VadState::Waiting;

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
        SYNC_VISIBLE_AT = 0;
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
// FUNC: ready
// DESC: Admits conversational capture only when the selected model and both native audio directions are usable.
// ------------------=
pub fn ready() -> bool {
    super::with_ai_runtime(|ai| ai.chat.selected_model_ready())
        && crate::drivers::audio::capture_available()
        && crate::drivers::audio::playback_rate().is_some()
}
// ------------------------=
// FUNC: synchronized_reply_length
// DESC: Limits the active assistant response to the word boundary reached by the real speech DMA cursor.
// ------------------=
pub fn synchronized_reply_length(turn: u64, full_length: usize) -> usize {
    unsafe {
        if turn != CHAT_TURN || !matches!(STATE, State::Thinking | State::Speaking) {
            full_length
        } else {
            SYNC_VISIBLE_AT.min(full_length)
        }
    }
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
        trace(b"listen failed grant");
        return false;
    };
    if !crate::drivers::audio::capture(OWNER, cap) {
        trace(b"listen failed capture open");
        retire(cap);
        return false;
    }
    INPUT_CAP = cap;
    let rate = crate::drivers::audio::capture_status()
        .map(|s| s.sample_rate)
        .unwrap_or(0);
    if !(&mut *(&raw mut RESAMPLER)).configure(rate) {
        trace(b"listen failed sample rate");
        crate::drivers::audio::stop_capture(OWNER);
        INPUT_CAP = 0;
        return false;
    }
    (&mut *(&raw mut UTTERANCE)).clear(300);
    LEVEL = 0;
    RENEW_AT = super::qwen::workers::clock_ns() + 1_000_000_000;
    REOPEN_AT = 0;
    REOPEN_DEADLINE = 0;
    REOPEN_FAILURE = 0;
    CAPTURE_FRAME_SEEN = false;
    CAPTURE_EMPTY_SINCE = super::qwen::workers::clock_ns();
    CAPTURE_EMPTY_REPORTED = false;
    LAST_VAD_STATE = VadState::Waiting;
    STATE = State::Listening;
    true
}

// ------------------------=
// FUNC: reopen_capture
// DESC: Recovers a completed or overrun DMA stream without discarding speech already accepted by VAD.
// ------------------=
unsafe fn reopen_capture() -> bool {
    let Some(cap) = grant(OWNER, CapabilityType::AudioInput, 60) else {
        REOPEN_FAILURE = 1;
        return false;
    };
    if !crate::drivers::audio::capture(OWNER, cap) {
        REOPEN_FAILURE = 2;
        retire(cap);
        return false;
    }
    INPUT_CAP = cap;
    let rate = crate::drivers::audio::capture_status().map(|s| s.sample_rate).unwrap_or(0);
    if !(&mut *(&raw mut RESAMPLER)).configure(rate) {
        REOPEN_FAILURE = 3;
        crate::drivers::audio::stop_capture(OWNER);
        INPUT_CAP = 0;
        return false;
    }
    RENEW_AT = super::qwen::workers::clock_ns() + 1_000_000_000;
    REOPEN_AT = 0;
    REOPEN_DEADLINE = 0;
    REOPEN_FAILURE = 0;
    CAPTURE_FRAME_SEEN = false;
    CAPTURE_EMPTY_SINCE = super::qwen::workers::clock_ns();
    CAPTURE_EMPTY_REPORTED = false;
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
        // Do not collect speech before the complete local path is able to
        // accept it.  The widget exposes this interval as STARTING instead of
        // letting users unknowingly stack transcripts behind model loading.
        let idle = super::with_ai_runtime(|ai| {
            ai.chat.input().is_empty()
                && ai.chat.generation_state != GenerationState::Running
        });
        if !idle || !ready() {
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
        SYNC_VISIBLE_AT = 0;
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
        SYNC_VISIBLE_AT = REPLY_LENGTH;
    }
    while REPLY_AT < REPLY_LENGTH && REPLY[REPLY_AT] == b' ' { REPLY_AT += 1; }
    if REPLY_AT >= REPLY_LENGTH {
        if !REPLY_COMPLETE {
            STATE = State::Thinking;
            return true;
        }
        if voice_output::status().state == voice_output::OutputState::Buffered {
            return voice_output::seal_buffered(OWNER);
        }
        if matches!(voice_output::status().state, voice_output::OutputState::Queued
            | voice_output::OutputState::Synthesizing | voice_output::OutputState::Ready
            | voice_output::OutputState::Speaking) {
            STATE = State::Speaking;
            return true;
        }
        trace(b"reply drained");
        SYNC_VISIBLE_AT = REPLY_LENGTH;
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
    let start = REPLY_AT;
    let end = REPLY_AT + count;
    let final_chunk = REPLY_COMPLETE && end >= REPLY_LENGTH;
    if voice_output::submit_span(OWNER, cap, &remaining[..count], start, end, final_chunk).is_err() {
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
        if continuous && INPUT_CAP != 0 {
            crate::drivers::audio::stop_capture(owner);
            retire(INPUT_CAP);
            INPUT_CAP = 0;
            (&mut *(&raw mut UTTERANCE)).clear(300);
            (&mut *(&raw mut RESAMPLER)).clear();
            (&mut *(&raw mut RAW)).fill(0);
            (&mut *(&raw mut MONO)).fill(0);
            (&mut *(&raw mut ECHO)).fill(0);
            ECHO_VALID_UNTIL = 0;
            LEVEL = 0;
        }
        OWNER = owner;
        CONTINUOUS = continuous;
        REPLY_AT = 0; REPLY_LENGTH = 0; REPLY_COMPLETE = false;
        SYNC_VISIBLE_AT = 0;
        RESTART_LISTENING = false;
        CHAT_TURN = turn;
        STATE = State::Thinking;
    }
}
// ------------------------=
// FUNC: synchronize_visible_reply
// DESC: Advances presentation at completed word boundaries from the resident playback cursor and promotes prefetched phrases only when hardware begins them.
// ------------------=
unsafe fn synchronize_visible_reply() -> bool {
    let before = SYNC_VISIBLE_AT;
    if let Some(progress) = voice_output::playback_progress(OWNER) {
        let mut target = progress.content_position.min(REPLY_LENGTH);
        if target < REPLY_LENGTH && !progress.content_boundary {
            while target > SYNC_VISIBLE_AT && !REPLY[target - 1].is_ascii_whitespace() { target -= 1; }
        }
        SYNC_VISIBLE_AT = SYNC_VISIBLE_AT.max(target);
    } else if voice_output::status().state == voice_output::OutputState::Complete {
        SYNC_VISIBLE_AT = REPLY_LENGTH;
    }
    before != SYNC_VISIBLE_AT
}
// ------------------------=
// FUNC: capture_frame
// DESC: Renews explicit microphone authority and feeds bounded echo-reduced input during listening and spoken replies.
// ------------------=
unsafe fn capture_frame(duplex: bool) -> bool {
    use crate::runtime::audio::CaptureState;
    let now=super::qwen::workers::clock_ns();
    if INPUT_CAP==0 && CONTINUOUS {
        if REOPEN_DEADLINE!=0 && now>=REOPEN_DEADLINE {
            trace(match REOPEN_FAILURE {
                1 => b"capture recovery expired grant",
                2 => b"capture recovery expired open",
                3 => b"capture recovery expired sample rate",
                _ => b"capture recovery expired unknown",
            });
            return false;
        }
        if now<REOPEN_AT {return true;}
        if reopen_capture() {
            trace(b"capture recovered");
        } else {
            if REOPEN_DEADLINE==0 {REOPEN_DEADLINE=now.saturating_add(2_000_000_000);}
            REOPEN_AT=now.saturating_add(100_000_000);
        }
        return true;
    }
    let Some(status)=crate::drivers::audio::capture_status() else {return true;};
    if status.state != CaptureState::Recording {
        // A bounded DMA overrun or expired capture window is not a user mute.
        // Preserve the accepted utterance and retry hardware re-open across the
        // codec reset boundary. Permission/device failures stay fatal.
        if CONTINUOUS && matches!(status.state,CaptureState::Overrun|CaptureState::Complete) {
            retire(INPUT_CAP);INPUT_CAP=0;
            REOPEN_AT=now;
            REOPEN_DEADLINE=now.saturating_add(2_000_000_000);
            trace(if status.state==CaptureState::Overrun {b"capture overrun recovery"} else {b"capture window recovery"});
            return true;
        }
        trace(match status.state {
            CaptureState::Idle => b"capture terminal idle",
            CaptureState::Cancelled => b"capture terminal cancelled",
            CaptureState::Denied => b"capture terminal denied",
            CaptureState::DeviceLost => b"capture terminal device-lost",
            _ => b"capture terminal unknown",
        });
        return false;
    }
    if now>=RENEW_AT {
        // Extend the short IOP stream deadline with the existing 60-second
        // microphone authority. Allocating a second capability before retiring
        // the first fails when the bounded global table is otherwise full and
        // made continuous listening drop after its first renewal.
        if INPUT_CAP==0 {return false;}
        if !crate::drivers::audio::renew_capture(OWNER,INPUT_CAP) {
            // The adapter lock is shared with the high-frequency DMA pump. A
            // single collision must not erase an utterance or cycle desktop
            // autostart; the existing five-second stream remains authorized
            // while a bounded retry is scheduled.
            trace(b"capture renewal retry");
            RENEW_AT=now.saturating_add(10_000_000);
            return true;
        }
        RENEW_AT=now+1_000_000_000;
    }
    let count=crate::drivers::audio::read_capture(OWNER,&mut *(&raw mut RAW));
    if count==0 {
        if !CAPTURE_EMPTY_REPORTED && now.saturating_sub(CAPTURE_EMPTY_SINCE)>=1_000_000_000 {
            trace(b"capture recording but no samples 1s");
            CAPTURE_EMPTY_REPORTED=true;
        }
        return true;
    }
    if !CAPTURE_FRAME_SEEN {
        trace(b"capture samples received");
        CAPTURE_FRAME_SEEN=true;
    } else if CAPTURE_EMPTY_REPORTED {
        trace(b"capture samples resumed");
    }
    CAPTURE_EMPTY_SINCE=now;
    CAPTURE_EMPTY_REPORTED=false;
    let (used,n)=(&mut *(&raw mut RESAMPLER)).process(&(&*(&raw const RAW))[..count],&mut *(&raw mut MONO));
    if used!=count {
        trace(b"capture resampler backpressure");
        return false;
    }
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
    let vad=(&*(&raw const UTTERANCE)).state();
    if vad!=LAST_VAD_STATE {
        trace(match vad {
            VadState::Waiting => b"vad waiting",
            VadState::Speech => b"vad speech onset",
            VadState::Complete => b"vad utterance complete",
            VadState::NoSpeech => b"vad no speech limit",
            VadState::Limit => b"vad sample limit",
            VadState::Cancelled => b"vad cancelled",
        });
        LAST_VAD_STATE=vad;
    }
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
        let mut presentation_changed = synchronize_visible_reply();
        if matches!(STATE, State::Off | State::Failed) {
            return false;
        }
        if STATE != State::Stopping && !active_owner(OWNER) {
            trace(b"conversation stopped inactive owner");
            stop(OWNER);
        }
        if matches!(STATE, State::Thinking | State::Speaking)
            && super::with_ai_runtime(|ai| ai.chat.turn_id() != CHAT_TURN || matches!(ai.chat.generation_state,
                GenerationState::Failed | GenerationState::Cancelled | GenerationState::ContextFull)) {
            trace(b"conversation stopped model state");
            stop(OWNER);
        }
        // Playback owns the conversational audio interval. The microphone is
        // reopened only after the complete response drains, so acoustic input
        // can never cancel or truncate an accepted assistant response.
        match STATE {
            State::Listening => {
                if !capture_frame(false) {
                    trace(b"conversation stopped capture frame");
                    stop(OWNER);
                    return true;
                }
                match (&*(&raw const UTTERANCE)).state() {
                    VadState::Complete => {
                        // Capture shutdown retires its lease. Recognition is
                        // asynchronous, so it needs independent authority that
                        // remains valid until the transcript is taken.
                        RECOGNIZE_CAP = grant(OWNER,CapabilityType::AudioInput,60)
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
                            trace(b"conversation stopped recognition submit");
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
                    let result_was_empty = matches!(&result, Ok(0));
                    let result_failed = result.is_err();
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
                        trace(match (result_was_empty,result_failed) {
                            (true,_) => b"recognition returned empty transcript",
                            (_,true) => b"recognition result take failed",
                            _ => b"transcript rejected by chat state",
                        });
                        stop(OWNER);
                    }
                }
                voice_input::InputState::Failed | voice_input::InputState::Cancelled => {
                    let status = voice_input::status();
                    trace(if status.state==voice_input::InputState::Cancelled {
                        b"recognition cancelled"
                    } else { match status.error {
                        1 => b"recognition invalid request",
                        2 => b"recognition deadline or cancellation",
                        3 => b"recognition model initialization failed",
                        4 => b"recognition decode failed",
                        5 => b"recognition no hypothesis",
                        6 => b"recognition transcript overflow",
                        7 => b"recognition engine fatal state",
                        _ => b"recognition provider failure",
                    }});
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
                    presentation_changed |= synchronize_visible_reply();
                    refresh_reply();
                    if !speak_next() {
                        stop(OWNER);
                    }
                }
                voice_output::OutputState::Buffered => {
                    refresh_reply();
                    if !speak_next() { stop(OWNER); }
                }
                voice_output::OutputState::Speaking if voice_output::can_prefetch() => {
                    refresh_reply();
                    if !speak_next() { stop(OWNER); }
                }
                voice_output::OutputState::Failed | voice_output::OutputState::Cancelled => {
                    trace(b"speech output failed or cancelled");
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
                        | voice_output::OutputState::Buffered
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
        presentation_changed |= synchronize_visible_reply();
        STATE != previous || presentation_changed
    }
}
