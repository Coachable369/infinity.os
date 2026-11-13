//! One bounded interruption recognizer, owned by the current spoken turn.
use super::*;
static mut PENDING: bool = false;
static mut TURN: u64 = 0;
static mut MODEL: u32 = 0;
static mut WORD: WakeWord = WakeWord::Infinity;
static mut CAPTURE_STARTED: u64 = 0;
static mut CAPTURE_ENDED: u64 = 0;
static mut INTERRUPTING: bool = false;
static mut PROBE: bool = false;
static mut PROBED_SAMPLES: usize = 0;

// ------------------------=
// FUNC: pending
// DESC: Prevents reply completion from discarding an interruption already being recognized.
// ------------------=
pub(super) unsafe fn pending() -> bool { PENDING }

// ------------------------=
// FUNC: capturing
// DESC: Keeps an interruption's onset and wake prefix across the end of the spoken reply.
// ------------------=
pub(super) unsafe fn capturing() -> bool {
    INTERRUPTING && matches!((&*(&raw const UTTERANCE)).state(), VadState::Speech | VadState::Complete)
}

// ------------------------=
// FUNC: clear
// DESC: Revokes interruption context; the normal owner cancellation path retains worker buffers until acknowledgement.
// ------------------=
pub(super) unsafe fn clear() { PENDING=false; TURN=0; MODEL=0;CAPTURE_STARTED=0;CAPTURE_ENDED=0;INTERRUPTING=false; PROBE=false; PROBED_SAMPLES=0; }

// ------------------------=
// FUNC: poll
// DESC: Captures while generating or speaking, validates an addressed interruption, and cancels only its original conversational turn.
// ------------------=
pub(super) unsafe fn poll(playing: bool) -> bool {
    if !matches!((&*(&raw const UTTERANCE)).state(), VadState::Speech | VadState::Complete) {INTERRUPTING=false;PROBED_SAMPLES=0;}
    if !capture_frame(playing) { stop(OWNER); return true; }
    if playing && matches!((&*(&raw const UTTERANCE)).state(), VadState::Speech | VadState::Complete) {INTERRUPTING=true;}
    if !PENDING {
        let samples=(&*(&raw const UTTERANCE)).active_speech_samples();
        let probe_at = if PROBED_SAMPLES == 0 { super::super::voice_vad::INTERRUPT_PROBE_SAMPLES }
            else { PROBED_SAMPLES.saturating_add(super::super::voice_vad::RATE) };
        if samples >= probe_at {
            if let Some(pcm)=(&*(&raw const UTTERANCE)).interruption_probe() {
                let available=super::super::with_ai_runtime(|ai| ai.chat.turn_id()==CHAT_TURN && ai.app_turn_owner().is_none());
                if available {
                    RECOGNIZE_CAP=grant(OWNER,CapabilityType::AudioInput,CAPTURE_CAPABILITY_SECONDS).unwrap_or(0);
                    PENDING=voice_input::submit(OWNER,RECOGNIZE_CAP,pcm).is_ok();
                    if PENDING {
                        PROBE=true;PROBED_SAMPLES=samples;TURN=CHAT_TURN;
                        MODEL=super::super::with_ai_runtime(|ai|ai.chat.selected_model());WORD=configured_wake_word(OWNER);
                        trace(b"duplex stop probe queued before endpoint");
                        return false;
                    }
                    retire(RECOGNIZE_CAP);RECOGNIZE_CAP=0;
                }
            }
        }
        match (&*(&raw const UTTERANCE)).state() {
            VadState::Complete => {
                // Never replace a typed edit or an independently owned app job.
                let available=super::super::with_ai_runtime(|ai| ai.chat.turn_id()==CHAT_TURN && ai.app_turn_owner().is_none());
                if available {
                    RECOGNIZE_CAP=grant(OWNER,CapabilityType::AudioInput,CAPTURE_CAPABILITY_SECONDS).unwrap_or(0);
                    PENDING=(&*(&raw const UTTERANCE)).speech()
                        .is_some_and(|pcm|voice_input::submit(OWNER,RECOGNIZE_CAP,pcm).is_ok());
                    if PENDING {
                        PROBE=false;
                        CAPTURE_STARTED=UTTERANCE_STARTED_AT;
                        CAPTURE_ENDED=UTTERANCE_ENDED_AT;
                        TURN=CHAT_TURN;
                        MODEL=super::super::with_ai_runtime(|ai|ai.chat.selected_model());
                        WORD=configured_wake_word(OWNER);
                        trace(b"duplex interruption recognition queued");
                    } else {retire(RECOGNIZE_CAP);RECOGNIZE_CAP=0;}
                }
                clear_captured_turn();
            }
            VadState::NoSpeech | VadState::Limit => clear_captured_turn(),
            _=>{}
        }
        return false;
    }
    match voice_input::status().state {
        voice_input::InputState::Ready => {
            let result=voice_input::take(OWNER,&mut *(&raw mut TRANSCRIPT));
            retire(RECOGNIZE_CAP);RECOGNIZE_CAP=0;PENDING=false;
            let valid=active_owner(OWNER) && WORD==configured_wake_word(OWNER)
                && super::super::with_ai_runtime(|ai|ai.chat.turn_id()==TURN
                    && ai.chat.selected_model()==MODEL
                    && ai.app_turn_owner().is_none());
            if let Ok(n) = result {
                let transcript=&(&*(&raw const TRANSCRIPT))[..n];
                let reply = &(&*(&raw const REPLY))[..REPLY_LENGTH];
                if valid && (addressed_stop_with_echo(transcript, reply, WORD)
                    || (!PROBE && stop_speech_command(transcript) && !reply_fragment(transcript, reply))) {
                    stop_spoken_reply(); (&mut *(&raw mut TRANSCRIPT)).fill(0); return true;
                }
                if valid && playback_transcript(transcript) {
                    (&mut *(&raw mut TRANSCRIPT)).fill(0); PROBE=false;
                    trace(b"duplex playback transcript discarded"); return false;
                }
            }
            if PROBE {
                PROBE=false;
                if valid && result.ok().is_some_and(|n| {
                    let input=&(&*(&raw const TRANSCRIPT))[..n];
                    if let wake_word::Match::Command{start}=wake_word::classify(input,WORD) {
                        stop_speech_command(&input[start..])
                    } else { false }
                }) {
                    stop_spoken_reply(); (&mut *(&raw mut TRANSCRIPT)).fill(0); return true;
                }
                (&mut *(&raw mut TRANSCRIPT)).fill(0);
                return false;
            }
            let addressed=result.ok().filter(|n|valid && *n!=0).map(|n|
                (n,wake_word::classify(&(&*(&raw const TRANSCRIPT))[..n],WORD)));
            let action=match addressed {
                Some((n,wake_word::Match::Command{start}))=>{
                    let end=wake_word::deduplicated_command_end(&(&*(&raw const TRANSCRIPT))[..n],WORD,start);
                    let mut checked=[0u8;512];
                    let valid=normalize_command(&(&*(&raw const TRANSCRIPT))[start..end],&mut checked)
                        .is_some_and(|length|length<=super::super::chat::CHAT_INPUT_CAPACITY);
                    checked.fill(0);valid.then_some((start,end))
                },
                Some((_,wake_word::Match::WakeOnly))=>Some((0,0)),
                _=>None,
            };
            if let Some((start,end))=action {
                // A typed draft is independent of a spoken stop. Preserve it
                // when a different spoken follow-up would need the composer.
                if !super::super::with_ai_runtime(|ai|ai.chat.input().is_empty())
                    && !stop_speech_command(&(&*(&raw const TRANSCRIPT))[start..end]) {
                    (&mut *(&raw mut TRANSCRIPT)).fill(0);
                    return false;
                }
                if start != end && stop_speech_command(&(&*(&raw const TRANSCRIPT))[start..end]) {
                    stop_spoken_reply();
                    (&mut *(&raw mut TRANSCRIPT)).fill(0);
                    return true;
                }
                // A recognized wake is the cancellation boundary, never raw VAD
                // or arbitrary room speech. Native workers drain normally.
                INTERRUPTED_REPLY = Some((OWNER, TURN));
                voice_output::stop_request(OWNER,REPLY_OUTPUT_REQUEST);
                super::super::with_ai_runtime(|ai|{if ai.chat.turn_id()==TURN {ai.cancel_chat();}});
                trace(b"duplex authorized interruption");
                report_timing();TURN_TIMING_ACTIVE=false;
                clear_wake_command();
                if start==end {
                    resume_captured_turn();arm_followup();
                } else {
                    clear_captured_turn();
                    if queue_command(&(&*(&raw const TRANSCRIPT))[start..end]) {
                        let now=super::super::qwen::workers::clock_ns();
                        (&mut *(&raw mut TURN_TIMING)).begin(CAPTURE_ENDED);
                        (&mut *(&raw mut TURN_TIMING)).observe(Milestone::CaptureStarted,CAPTURE_STARTED);
                        (&mut *(&raw mut TURN_TIMING)).observe(Milestone::RecognitionReady,now);
                        (&mut *(&raw mut TURN_TIMING)).observe(Milestone::Transcript,now);
                        TURN_TIMING_ACTIVE=true;
                        STATE=if submit_pending_transcript(){State::Thinking}else{State::Submitting};
                    } else {resume_captured_turn();}
                }
                (&mut *(&raw mut TRANSCRIPT)).fill(0);
                return true;
            }
            (&mut *(&raw mut TRANSCRIPT)).fill(0);
            // The submitted clip was cleared on dispatch; preserve any later microphone input.
            trace(b"duplex unaddressed audio discarded");
        }
        voice_input::InputState::Failed | voice_input::InputState::Cancelled => {
            PENDING=false;retire(RECOGNIZE_CAP);RECOGNIZE_CAP=0;PROBE=false;
            trace(b"duplex recognition failed; original reply retained");
        }
        _=>{}
    }
    false
}
