//! Explicitly enabled local conversation. All transitions run on the desktop owner;
//! decoding runs on a bounded AP and uses the existing local AI service.
use super::{
    chat::{ChatRole, GenerationState},
    voice_input, voice_output,
    voice_pcm::Resampler,
    voice_vad::{Utterance, VadState},
};
use crate::runtime::{
    capability::CapabilityType,
    execution::SecurityIdentity,
    identity::{SessionState, WakeWord},
};
use super::wake_word;
#[path = "../../ui/voice_indicator.rs"]
pub mod indicator;
#[path = "speech_chunk.rs"]
mod speech_chunk;
#[path = "voice_echo.rs"]
mod voice_echo;
#[path = "voice_timing.rs"]
pub mod timing;
use timing::Milestone;
#[cfg(not(test))]
#[path = "voice_trace.rs"]
mod voice_trace;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Off,
    Starting,
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
static mut CAPABILITY_REPLACE_AT: u64 = 0;
static mut REOPEN_AT: u64 = 0;
static mut REOPEN_DEADLINE: u64 = 0;
static mut RESAMPLER: Resampler = Resampler::empty();
static mut UTTERANCE: Utterance = Utterance::new(300);
static mut RAW: [i16; 4800] = [0; 4800];
static mut MONO: [i16; 4800] = [0; 4800];
static mut ECHO: [i16; 9600] = [0; 9600];
static mut ECHO_VALID_UNTIL: u64 = 0;
static mut TRANSCRIPT: [u8; 512] = [0; 512];
static mut PENDING_PROMPT: [u8; 512] = [0; 512];
static mut PENDING_PROMPT_LENGTH: usize = 0;
static mut PENDING_PROMPT_MODEL: u32 = 0;
static mut PENDING_PROMPT_WORD: WakeWord = WakeWord::Infinity;
static mut PENDING_PROMPT_UNTIL: u64 = 0;
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
static mut TURN_TIMING: timing::Timeline = timing::Timeline::new();
static mut TURN_TIMING_ACTIVE: bool = false;
static mut LAST_TIMING_RECORD: (u64, u64) = (0, 0);
// One follow-up utterance may accumulate while its predecessor is decoded.
// The existing Utterance owns at most ten seconds; there is no transcript queue.
static mut UTTERANCE_STARTED_AT: u64 = 0;
static mut UTTERANCE_ENDED_AT: u64 = 0;
static mut HANDOFF_PCM: [i16; 4800] = [0; 4800];
static mut HANDOFF_LENGTH: usize = 0;
static mut RECOGNITION_MODEL: u32 = 0;
static mut RECOGNITION_WORD: WakeWord = WakeWord::Infinity;
static mut RECOGNITION_CONTEXT_VALID: bool = false;
static mut RECOGNITION_MODEL_WAS_READY: bool = false;
static mut RECOGNITION_WAKE_AUTHORIZED: bool = false;
static mut FOLLOWUP_ONSET_UNTIL: u64 = 0;
static mut WAKE_ARMED: bool = false;
static mut WAKE_ARMED_UNTIL: u64 = 0;
static mut WAKE_ARMED_MODEL: u32 = 0;
static mut WAKE_ARMED_WORD: WakeWord = WakeWord::Infinity;
const WAKE_COMMAND_WINDOW_NS: u64 = 10_000_000_000;
const SUBMIT_WAIT_NS: u64 = 10_000_000_000;
const CAPTURE_CAPABILITY_SECONDS: u64 = 60;
const CAPTURE_CAPABILITY_REPLACE_NS: u64 = 45_000_000_000;

#[cfg(not(test))]
// ------------------------=
// FUNC: trace
// DESC: Emits privacy-safe conversation state checkpoints to the VM serial trace.
// ------------------=
fn trace(event: &[u8]) {
    voice_trace::write(b"[VOICE] ", event, super::qwen::workers::clock_ns());
}

#[cfg(test)]
// ------------------------=
// FUNC: trace
// DESC: Keeps host conversation harnesses independent of the native serial device.
// ------------------=
fn trace(_: &[u8]) {}

// ------------------------=
// FUNC: timing_snapshot
// DESC: Copies content-free measurements on the conversation owner thread for structured diagnostics.
// ------------------=
pub fn timing_snapshot() -> timing::Timeline {
    unsafe { TURN_TIMING }
}

// ------------------------=
// FUNC: report_timing
// DESC: Emits each new measured stage once, never mistaking a missing playback event for success.
// ------------------=
unsafe fn report_timing() {
    let record = timing_snapshot().record();
    let key = (record[1], record[2]);
    if key != LAST_TIMING_RECORD && record[2] != 0 {
        LAST_TIMING_RECORD = key;
        #[cfg(not(test))]
        voice_trace::timing(record);
    }
}

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
// FUNC: configured_wake_word
// DESC: Resolves the active owner's durable wake phrase without granting any new authority.
// ------------------=
fn configured_wake_word(owner: SecurityIdentity) -> WakeWord {
    crate::runtime::with_runtime(|runtime| {
        (0..crate::runtime::identity::MAX_SESSIONS)
            .filter_map(|index| runtime.identity.session_nth(index))
            .find(|session| session.id.0 == owner.0 && session.state == SessionState::Active)
            .and_then(|session| runtime.identity.voice_profile(session.user))
            .map(|profile| profile.wake_word)
    })
    .flatten()
    .unwrap_or(WakeWord::Infinity)
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
// FUNC: normalize_command
// DESC: Converts supported speech punctuation and whitespace before any ASCII composer mutation; rejects unsupported UTF-8 atomically.
// ------------------=
fn normalize_command(input: &[u8], output: &mut [u8]) -> Option<usize> {
    let text = core::str::from_utf8(input).ok()?;
    let mut length = 0;
    for ch in text.chars() {
        let bytes: &[u8] = match ch {
            '\u{2018}' | '\u{2019}' | '\u{201a}' | '\u{201b}' => b"'",
            '\u{201c}' | '\u{201d}' | '\u{201e}' | '\u{201f}' => b"\"",
            '\u{2010}'..='\u{2015}' | '\u{2212}' => b"-",
            '\u{2026}' => b"...",
            _ if ch.is_whitespace() => b" ",
            _ if ch.is_ascii() && !ch.is_ascii_control() => {
                if length == output.len() { return None; }
                output[length] = ch as u8; length += 1;
                continue;
            }
            _ => return None,
        };
        if bytes == b" " && (length == 0 || output[length - 1] == b' ') { continue; }
        if bytes.len() > output.len() - length { return None; }
        output[length..length + bytes.len()].copy_from_slice(bytes); length += bytes.len();
    }
    while length != 0 && output[length - 1] == b' ' { length -= 1; }
    (length != 0).then_some(length)
}
// ------------------------=
// FUNC: clear_pending_prompt
// DESC: Erases controller-owned prompt state and optionally removes only an unchanged owned composer, never a user's edits.
// ------------------=
unsafe fn clear_pending_prompt(remove_composer: bool) {
    if remove_composer && PENDING_PROMPT_LENGTH != 0 {
        super::with_ai_runtime(|ai| {
            if ai.chat.input() == &(&*(&raw const PENDING_PROMPT))[..PENDING_PROMPT_LENGTH] {
                ai.chat.set_input_cursor(PENDING_PROMPT_LENGTH);
                for _ in 0..PENDING_PROMPT_LENGTH { let _ = ai.chat.pop_input(); }
            }
        });
    }
    (&mut *(&raw mut PENDING_PROMPT)).fill(0);
    PENDING_PROMPT_LENGTH = 0;
    PENDING_PROMPT_UNTIL = 0;
}
// ------------------------=
// FUNC: pending_prompt_valid
// DESC: Binds a queued voice prompt to its original owner, model, wake setting, deadline, and exact unchanged composer.
// ------------------=
unsafe fn pending_prompt_valid() -> bool {
    PENDING_PROMPT_LENGTH != 0 && active_owner(OWNER)
        && super::qwen::workers::clock_ns() < PENDING_PROMPT_UNTIL
        && configured_wake_word(OWNER) == PENDING_PROMPT_WORD
        && super::with_ai_runtime(|ai| ai.chat.selected_model() == PENDING_PROMPT_MODEL
            && ai.chat.input() == &(&*(&raw const PENDING_PROMPT))[..PENDING_PROMPT_LENGTH])
}
// ------------------------=
// FUNC: queue_command
// DESC: Inserts a fully validated bounded transcript atomically and records ownership before waiting for local-model readiness.
// ------------------=
unsafe fn queue_command(input: &[u8]) -> bool {
    clear_pending_prompt(false);
    let Some(length) = normalize_command(input, &mut *(&raw mut PENDING_PROMPT)) else {
        clear_pending_prompt(false);
        return false;
    };
    if length > super::chat::CHAT_INPUT_CAPACITY { clear_pending_prompt(false); return false; }
    let inserted = super::with_ai_runtime(|ai| {
        if !ai.chat.input().is_empty() || ai.chat.generation_state == GenerationState::Running { return false; }
        for index in 0..length {
            if !ai.chat.push_input(PENDING_PROMPT[index]) {
                for _ in 0..index { let _ = ai.chat.pop_input(); }
                return false;
            }
        }
        true
    });
    if !inserted { clear_pending_prompt(false); return false; }
    PENDING_PROMPT_LENGTH = length;
    PENDING_PROMPT_MODEL = super::with_ai_runtime(|ai| ai.chat.selected_model());
    PENDING_PROMPT_WORD = configured_wake_word(OWNER);
    PENDING_PROMPT_UNTIL = super::qwen::workers::clock_ns().saturating_add(SUBMIT_WAIT_NS);
    true
}
// ------------------------=
// FUNC: submit_pending_transcript
// DESC: Submits only the unchanged authorized voice composer after its original selected local model becomes ready.
// ------------------=
unsafe fn submit_pending_transcript() -> bool {
    if !pending_prompt_valid() { return false; }
    let submitted = super::with_ai_runtime(|ai| {
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
    });
    if submitted {
        (&mut *(&raw mut TURN_TIMING)).observe(Milestone::Submitted, super::qwen::workers::clock_ns());
        clear_pending_prompt(false);
    }
    submitted
}
// ------------------------=
// FUNC: state
// DESC: Provides visible session state and audio level without exposing raw captured speech.
// ------------------=
pub fn state() -> (State, u16) {
    unsafe { (STATE, LEVEL) }
}

// ------------------------=
// FUNC: wake_armed
// DESC: Reports whether a wake-only utterance has opened the next bounded command turn.
// ------------------=
pub fn wake_armed() -> bool {
    unsafe { WAKE_ARMED }
}
// ------------------------=
// FUNC: clear_wake_command
// DESC: Consumes or revokes the one command authorized by an explicit wake-only utterance.
// ------------------=
unsafe fn clear_wake_command() {
    WAKE_ARMED = false;
    WAKE_ARMED_UNTIL = 0;
    WAKE_ARMED_MODEL = 0;
}
// ------------------------=
// FUNC: clear_captured_turn
// DESC: Erases the single pending utterance and endpoint carry without touching microphone authority.
// ------------------=
unsafe fn clear_captured_turn() {
    (&mut *(&raw mut UTTERANCE)).clear(300);
    (&mut *(&raw mut HANDOFF_PCM)).fill(0);
    HANDOFF_LENGTH = 0;
    UTTERANCE_STARTED_AT = 0;
    UTTERANCE_ENDED_AT = 0;
    LAST_VAD_STATE = VadState::Waiting;
}
// ------------------------=
// FUNC: resume_captured_turn
// DESC: Promotes bounded audio captured during recognition without reopening DMA or resetting its resampler.
// ------------------=
unsafe fn resume_captured_turn() {
    STATE = State::Listening;
    LEVEL = 0;
    RECOGNITION_CONTEXT_VALID = false;
    RECOGNITION_MODEL_WAS_READY = false;
    RECOGNITION_WAKE_AUTHORIZED = false;
    FOLLOWUP_ONSET_UNTIL = 0;
}
// ------------------------=
// FUNC: ready
// DESC: Admits conversational capture only when the selected model and both native audio directions are usable.
// ------------------=
pub fn ready() -> bool {
    super::with_ai_runtime(|ai| ai.chat.selected_model_ready())
        && crate::drivers::audio::capture_available()
        && crate::drivers::audio::playback_rate().is_some()
        && voice_input::prepared()
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
// FUNC: reset_listening_state
// DESC: Clears private turn buffers while preserving the already-authorized microphone DMA stream.
// ------------------=
unsafe fn reset_listening_state() {
    clear_captured_turn();
    (&mut *(&raw mut RESAMPLER)).clear();
    (&mut *(&raw mut RAW)).fill(0);
    (&mut *(&raw mut MONO)).fill(0);
    (&mut *(&raw mut ECHO)).fill(0);
    ECHO_VALID_UNTIL = 0;
    LEVEL = 0;
    REOPEN_AT = 0;
    REOPEN_DEADLINE = 0;
    REOPEN_FAILURE = 0;
    CAPTURE_FRAME_SEEN = false;
    CAPTURE_EMPTY_SINCE = super::qwen::workers::clock_ns();
    CAPTURE_EMPTY_REPORTED = false;
    LAST_VAD_STATE = VadState::Waiting;
    RECOGNITION_CONTEXT_VALID = false;
    RECOGNITION_WAKE_AUTHORIZED = false;
    FOLLOWUP_ONSET_UNTIL = 0;
    STATE = State::Listening;
}

// ------------------------=
// FUNC: listen
// DESC: Opens one bounded capture stream or reuses its live DMA path for the next conversational turn.
// ------------------=
unsafe fn listen() -> bool {
    if INPUT_CAP != 0 {
        reset_listening_state();
        return true;
    }
    let Some(cap) = grant(OWNER, CapabilityType::AudioInput, CAPTURE_CAPABILITY_SECONDS) else {
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
        retire(INPUT_CAP);
        INPUT_CAP = 0;
        return false;
    }
    let now = super::qwen::workers::clock_ns();
    RENEW_AT = now + 1_000_000_000;
    CAPABILITY_REPLACE_AT = now + CAPTURE_CAPABILITY_REPLACE_NS;
    reset_listening_state();
    true
}

// ------------------------=
// FUNC: reopen_capture
// DESC: Recovers a completed or overrun DMA stream without discarding speech already accepted by VAD.
// ------------------=
unsafe fn reopen_capture() -> bool {
    let Some(cap) = grant(OWNER, CapabilityType::AudioInput, CAPTURE_CAPABILITY_SECONDS) else {
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
    let now = super::qwen::workers::clock_ns();
    RENEW_AT = now + 1_000_000_000;
    CAPABILITY_REPLACE_AT = now + CAPTURE_CAPABILITY_REPLACE_NS;
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
        let route_ready = super::with_ai_runtime(|ai| ai.chat.selected_model_ready())
            && crate::drivers::audio::capture_available()
            && crate::drivers::audio::playback_rate().is_some();
        if !idle || !route_ready {
            return false;
        }
        OWNER = owner;
        CONTINUOUS = true;
        clear_wake_command();
        RESTART_LISTENING = false;
        super::with_ai_runtime(|ai| {
            ai.bind_chat_owner(owner.0);
            ai.chat.set_enabled(true);
            ai.chat.set_minimized(false);
        });
        if !voice_input::prepare() {
            if voice_input::prepare_failed() {
                STATE = State::Failed;
                trace(b"recognizer warmup failed");
                return false;
            }
            STATE = State::Starting;
            trace(b"conversation waiting for recognizer warmup");
            return true;
        }
        if !listen() {
            STATE = State::Failed;
            trace(b"initial capture failed");
            return false;
        }
        trace(b"conversation listening");
        true
    }
}

// ------------------------=
// FUNC: autostart
// DESC: Consumes one scheduled desktop start; fatal errors and manual stops never reopen capture automatically.
// ------------------=
pub fn autostart(owner: SecurityIdentity, deadline: &mut u64, now: u64) -> Option<bool> {
    if *deadline == u64::MAX || now < *deadline { return None; }
    *deadline = u64::MAX;
    if state().0 != State::Off { return None; }
    Some(start(owner))
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
        report_timing();
        TURN_TIMING_ACTIVE = false;
        RESTART_LISTENING = false;
        crate::drivers::audio::stop_capture(owner);
        INPUT_CAP = 0;
        CAPABILITY_REPLACE_AT = 0;
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
        (&mut *(&raw mut HANDOFF_PCM)).fill(0);
        HANDOFF_LENGTH = 0;
        UTTERANCE_STARTED_AT = 0;
        UTTERANCE_ENDED_AT = 0;
        RECOGNITION_CONTEXT_VALID = false;
        RECOGNITION_WAKE_AUTHORIZED = false;
        FOLLOWUP_ONSET_UNTIL = 0;
        (&mut *(&raw mut RESAMPLER)).clear();
        (&mut *(&raw mut RAW)).fill(0);
        (&mut *(&raw mut MONO)).fill(0);
        (&mut *(&raw mut ECHO)).fill(0);ECHO_VALID_UNTIL=0;
        (&mut *(&raw mut TRANSCRIPT)).fill(0);
        clear_pending_prompt(true);
        (&mut *(&raw mut REPLY)).fill(0);
        REPLY_LENGTH = 0;
        REPLY_AT = 0;
        SYNC_VISIBLE_AT = 0;
        LEVEL = 0;
        clear_wake_command();
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
        if TURN_TIMING_ACTIVE && timing_snapshot().at(Milestone::FirstAudio).is_some() {
            (&mut *(&raw mut TURN_TIMING)).observe(Milestone::Drained, super::qwen::workers::clock_ns());
        }
        report_timing();
        TURN_TIMING_ACTIVE = false;
        SYNC_VISIBLE_AT = REPLY_LENGTH;
        if CONTINUOUS {
            // A completed reply never authorizes another spoken request.
            // Only a newly recognized wake phrase may open the next command.
            clear_wake_command();
            if voice_input::prepared() {
                return listen();
            }
            let _ = voice_input::prepare();
            STATE = State::Starting;
            trace(b"conversation waiting for recognizer rewarm");
            return true;
        }
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
    if TURN_TIMING_ACTIVE {
        (&mut *(&raw mut TURN_TIMING)).observe(Milestone::SpeechQueued, super::qwen::workers::clock_ns());
    }
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
    if TURN_TIMING_ACTIVE && REPLY_LENGTH != 0 {
        (&mut *(&raw mut TURN_TIMING)).observe(Milestone::FirstText, super::qwen::workers::clock_ns());
    }
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
        report_timing();
        TURN_TIMING_ACTIVE = false;
        if continuous && INPUT_CAP != 0 {
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
        clear_wake_command();
        REPLY_AT = 0; REPLY_LENGTH = 0; REPLY_COMPLETE = false;
        SYNC_VISIBLE_AT = 0;
        RESTART_LISTENING = false;
        CHAT_TURN = turn;
        STATE = State::Thinking;
    }
}

// ------------------------=
// FUNC: renew_capture_authority
// DESC: Extends live DMA and rotates its expiring capability without closing the host microphone stream.
// ------------------=
unsafe fn renew_capture_authority(now: u64) -> bool {
    if INPUT_CAP == 0 {
        return false;
    }
    if now >= CAPABILITY_REPLACE_AT {
        if let Some(next) = grant(OWNER, CapabilityType::AudioInput, CAPTURE_CAPABILITY_SECONDS) {
            if crate::drivers::audio::renew_capture(OWNER, next) {
                INPUT_CAP = next;
                CAPABILITY_REPLACE_AT = now.saturating_add(CAPTURE_CAPABILITY_REPLACE_NS);
                RENEW_AT = now.saturating_add(1_000_000_000);
                trace(b"capture authority rotated");
                return true;
            }
            retire(next);
        }
        trace(b"capture authority rotation retry");
        RENEW_AT = now.saturating_add(10_000_000);
        return true;
    }
    if crate::drivers::audio::renew_capture(OWNER, INPUT_CAP) {
        RENEW_AT = now.saturating_add(1_000_000_000);
        true
    } else {
        trace(b"capture renewal retry");
        RENEW_AT = now.saturating_add(10_000_000);
        true
    }
}

// ------------------------=
// FUNC: drain_muted_capture
// DESC: Keeps continuous microphone DMA healthy while recognition, inference, and uninterruptible speech own the turn.
// ------------------=
unsafe fn drain_muted_capture() -> bool {
    use crate::runtime::audio::CaptureState;
    if INPUT_CAP == 0 {
        return true;
    }
    let now = super::qwen::workers::clock_ns();
    let Some(status) = crate::drivers::audio::capture_status() else {
        return true;
    };
    if status.state != CaptureState::Recording {
        if matches!(status.state, CaptureState::Overrun | CaptureState::Complete) {
            retire(INPUT_CAP);
            INPUT_CAP = 0;
            CAPABILITY_REPLACE_AT = 0;
            trace(b"muted capture ended; defer reopen");
            return true;
        }
        trace(b"muted capture terminal failure");
        return false;
    }
    if now >= RENEW_AT && !renew_capture_authority(now) {
        return false;
    }
    let _ = crate::drivers::audio::read_capture(OWNER, &mut *(&raw mut RAW));
    (&mut *(&raw mut RAW)).fill(0);
    (&mut *(&raw mut MONO)).fill(0);
    LEVEL = 0;
    true
}
// ------------------------=
// FUNC: synchronize_visible_reply
// DESC: Advances presentation at completed word boundaries from the resident playback cursor and promotes prefetched phrases only when hardware begins them.
// ------------------=
unsafe fn synchronize_visible_reply() -> bool {
    let before = SYNC_VISIBLE_AT;
    if TURN_TIMING_ACTIVE && matches!(STATE, State::Thinking | State::Speaking) {
        let first = voice_output::status().first_playback_ns;
        if first != 0 && timing_snapshot().at(Milestone::SpeechQueued).is_some_and(|queued| first >= queued) {
            (&mut *(&raw mut TURN_TIMING)).observe(Milestone::FirstAudio, first);
        }
    }
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
        if !renew_capture_authority(now) { return false; }
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
    // Keep exactly the first following utterance during ASR. Audio after that
    // bounded segment is deliberately drained, never queued as another turn.
    let accept = STATE != State::Recognizing || (RECOGNITION_CONTEXT_VALID
        && (UTTERANCE_STARTED_AT != 0 || now < FOLLOWUP_ONSET_UNTIL));
    if STATE == State::Recognizing && UTTERANCE_STARTED_AT == 0
        && FOLLOWUP_ONSET_UNTIL != 0 && now >= FOLLOWUP_ONSET_UNTIL {
        clear_captured_turn();
        FOLLOWUP_ONSET_UNTIL = 0;
    }
    let consumed = if accept {
        (&mut *(&raw mut UTTERANCE)).push(&(&*(&raw const MONO))[..n])
    } else { n };
    let vad=(&*(&raw const UTTERANCE)).state();
    if UTTERANCE_STARTED_AT == 0 && matches!(vad, VadState::Speech | VadState::Complete) {
        UTTERANCE_STARTED_AT = now.max(1);
    }
    if UTTERANCE_ENDED_AT == 0 && vad == VadState::Complete {
        UTTERANCE_ENDED_AT = now.max(1);
    }
    // VAD can finish part way through a driver chunk. Preserve that suffix
    // before clearing MONO so the next command's onset is not silently lost.
    if STATE == State::Listening && vad == VadState::Complete && consumed < n {
        HANDOFF_LENGTH = n - consumed;
        (&mut *(&raw mut HANDOFF_PCM))[..HANDOFF_LENGTH]
            .copy_from_slice(&(&*(&raw const MONO))[consumed..n]);
    }
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
        if STATE == State::Recognizing && RECOGNITION_CONTEXT_VALID
            && (configured_wake_word(OWNER) != RECOGNITION_WORD
                || super::with_ai_runtime(|ai| ai.chat.selected_model() != RECOGNITION_MODEL
                    || (RECOGNITION_MODEL_WAS_READY && !ai.chat.selected_model_ready())
                    || ai.chat.generation_state == GenerationState::Running)) {
            RECOGNITION_CONTEXT_VALID = false;
            RECOGNITION_WAKE_AUTHORIZED = false;
            clear_wake_command();
            clear_captured_turn();
            trace(b"recognition context changed; pending audio erased");
        }
        if WAKE_ARMED && (configured_wake_word(OWNER) != WAKE_ARMED_WORD
            || super::with_ai_runtime(|ai| ai.chat.selected_model() != WAKE_ARMED_MODEL
                || !ai.chat.selected_model_ready()
                || ai.chat.generation_state == GenerationState::Running)) {
            clear_wake_command();
            trace(b"wake command revoked by context change");
        }
        if matches!(STATE, State::Thinking | State::Speaking)
            && super::with_ai_runtime(|ai| ai.chat.turn_id() != CHAT_TURN || matches!(ai.chat.generation_state,
                GenerationState::Failed | GenerationState::Cancelled | GenerationState::ContextFull)) {
            trace(b"conversation stopped model state");
            stop(OWNER);
        }
        // Recognition retains one following utterance so wake-only decoding
        // cannot erase an immediately spoken command. Inference and playback
        // still own the accepted turn; their input is drained, never queued.
        if STATE == State::Recognizing && !capture_frame(false) {
            trace(b"conversation stopped recognition capture");
            stop(OWNER);
            return true;
        }
        if matches!(STATE, State::Submitting | State::Thinking | State::Speaking)
            && !drain_muted_capture()
        {
            trace(b"conversation stopped muted capture");
            stop(OWNER);
            return true;
        }
        match STATE {
            State::Starting => {
                if voice_input::prepare_failed() {
                    stop(OWNER);
                    STATE = State::Failed;
                    trace(b"recognizer warmup failed");
                } else if voice_input::prepared() {
                    if listen() { trace(b"conversation listening after warmup"); }
                    else { STATE = State::Failed; trace(b"capture failed after warmup"); }
                } else {
                    let _ = voice_input::prepare();
                }
            }
            State::Listening => {
                if WAKE_ARMED
                    && super::qwen::workers::clock_ns() >= WAKE_ARMED_UNTIL
                    && (UTTERANCE_STARTED_AT == 0 || UTTERANCE_STARTED_AT >= WAKE_ARMED_UNTIL)
                {
                    clear_wake_command();
                    trace(b"wake command window expired");
                }
                if (&*(&raw const UTTERANCE)).state() != VadState::Complete && !capture_frame(false) {
                    trace(b"conversation stopped capture frame");
                    stop(OWNER);
                    return true;
                }
                match (&*(&raw const UTTERANCE)).state() {
                    VadState::Complete => {
                        // Recognition receives separate bounded authority; the
                        // same capture stream retains one following utterance.
                        RECOGNIZE_CAP = grant(
                            OWNER,
                            CapabilityType::AudioInput,
                            CAPTURE_CAPABILITY_SECONDS,
                        )
                            .unwrap_or(0);
                        let submitted = (&*(&raw const UTTERANCE))
                            .speech()
                            .map(|pcm| voice_input::submit(OWNER, RECOGNIZE_CAP, pcm).is_ok())
                            .unwrap_or(false);
                        if submitted {
                            report_timing();
                            (&mut *(&raw mut TURN_TIMING)).begin(UTTERANCE_ENDED_AT);
                            TURN_TIMING_ACTIVE = true;
                        }
                        RECOGNITION_MODEL = super::with_ai_runtime(|ai| ai.chat.selected_model());
                        RECOGNITION_WORD = configured_wake_word(OWNER);
                        RECOGNITION_CONTEXT_VALID = submitted;
                        RECOGNITION_MODEL_WAS_READY = super::with_ai_runtime(|ai| ai.chat.selected_model_ready());
                        RECOGNITION_WAKE_AUTHORIZED = WAKE_ARMED && UTTERANCE_STARTED_AT != 0
                            && UTTERANCE_STARTED_AT < WAKE_ARMED_UNTIL;
                        FOLLOWUP_ONSET_UNTIL = super::qwen::workers::clock_ns()
                            .saturating_add(WAKE_COMMAND_WINDOW_NS);
                        (&mut *(&raw mut UTTERANCE)).clear(300);
                        UTTERANCE_STARTED_AT = 0;
                        UTTERANCE_ENDED_AT = 0;
                        LAST_VAD_STATE = VadState::Waiting;
                        if submitted && HANDOFF_LENGTH != 0 {
                            (&mut *(&raw mut UTTERANCE)).push(&(&*(&raw const HANDOFF_PCM))[..HANDOFF_LENGTH]);
                            if (&*(&raw const UTTERANCE)).state() == VadState::Speech {
                                UTTERANCE_STARTED_AT = super::qwen::workers::clock_ns().max(1);
                            }
                        }
                        (&mut *(&raw mut HANDOFF_PCM)).fill(0);
                        HANDOFF_LENGTH = 0;
                        LEVEL = 0;
                        if submitted {
                            trace(b"recognition queued");
                            STATE = State::Recognizing;
                        } else {
                            trace(b"conversation stopped recognition submit");
                            stop(OWNER);
                        }
                    }
                    VadState::NoSpeech => {
                        clear_wake_command();
                        clear_captured_turn();
                    }
                    VadState::Limit => { clear_wake_command(); }
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
                    if result_failed {
                        (&mut *(&raw mut TRANSCRIPT)).fill(0);
                        stop(OWNER);
                        STATE = State::Failed;
                        trace(b"recognition result authority failed");
                        return true;
                    }
                    if !RECOGNITION_CONTEXT_VALID {
                        (&mut *(&raw mut TRANSCRIPT)).fill(0);
                        clear_captured_turn();
                        resume_captured_turn();
                        return true;
                    }
                    let mut command = None;
                    let mut wake_only = false;
                    let mut unaddressed = false;
                    if let Ok(n) = result {
                        if n != 0 {
                            match wake_word::classify(
                                &(&*(&raw const TRANSCRIPT))[..n],
                                configured_wake_word(OWNER),
                            ) {
                                wake_word::Match::Absent if RECOGNITION_WAKE_AUTHORIZED => command = Some((0, n)),
                                wake_word::Match::Absent => unaddressed = true,
                                wake_word::Match::WakeOnly => wake_only = true,
                                wake_word::Match::Command { start } => {
                                    command = Some((start, n));
                                }
                            }
                        }
                    }
                    let queued = command
                        .map(|(start, end)| {
                            clear_wake_command();
                            queue_command(&(&*(&raw const TRANSCRIPT))[start..end])
                        })
                        .unwrap_or(false);
                    (&mut *(&raw mut TRANSCRIPT)).fill(0);
                    if queued {
                        (&mut *(&raw mut TURN_TIMING)).observe(Milestone::Transcript, super::qwen::workers::clock_ns());
                        clear_captured_turn();
                        RECOGNITION_CONTEXT_VALID = false;
                        RECOGNITION_WAKE_AUTHORIZED = false;
                        trace(b"transcript ready");
                        STATE = if submit_pending_transcript() {
                            trace(b"transcript submitted");
                            State::Thinking
                        } else {
                            State::Submitting
                        };
                    } else if wake_only {
                        if matches!((&*(&raw const UTTERANCE)).state(), VadState::Limit | VadState::NoSpeech) {
                            clear_wake_command();
                        } else {
                            WAKE_ARMED = true;
                            WAKE_ARMED_UNTIL = super::qwen::workers::clock_ns()
                                .saturating_add(WAKE_COMMAND_WINDOW_NS);
                            WAKE_ARMED_MODEL = super::with_ai_runtime(|ai| ai.chat.selected_model());
                            WAKE_ARMED_WORD = configured_wake_word(OWNER);
                        }
                        trace(b"wake phrase armed command capture");
                        resume_captured_turn();
                    } else if unaddressed {
                        trace(b"unaddressed speech ignored");
                        resume_captured_turn();
                    } else {
                        trace(match (result_was_empty,result_failed) {
                            (true,_) => b"recognition returned empty transcript",
                            (_,true) => b"recognition result take failed",
                            _ => b"transcript rejected by chat state",
                        });
                        clear_wake_command();
                        resume_captured_turn();
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
                    // Empty/oversized hypotheses are failed utterances, not failed
                    // microphone devices. Discard them without cycling host DMA.
                    // Cancellation, revoked authority, and decoder faults must
                    // not silently obtain a fresh microphone lease.
                    if status.state == voice_input::InputState::Failed && matches!(status.error, 5 | 6) {
                        if status.error == 6 {
                            (&mut *(&raw mut TRANSCRIPT)).fill(0);
                            clear_wake_command();
                            clear_captured_turn();
                        }
                        if !RECOGNITION_CONTEXT_VALID { clear_captured_turn(); }
                        resume_captured_turn();
                    } else {
                        stop(OWNER);
                        STATE = State::Failed;
                    }
                }
                _ => {}
            },
            State::Submitting => {
                if !pending_prompt_valid() {
                    clear_pending_prompt(true);
                    clear_wake_command();
                    clear_captured_turn();
                    resume_captured_turn();
                    trace(b"queued transcript expired or context changed");
                } else if submit_pending_transcript() { STATE = State::Thinking; }
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
        report_timing();
        STATE != previous || presentation_changed
    }
}
