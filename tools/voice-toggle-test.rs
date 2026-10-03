//! Executes the production conversation controller with deterministic audio and identity seams.
#![allow(dead_code)]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
static BUSY: AtomicBool = AtomicBool::new(false);
static CAPTURES: AtomicUsize = AtomicUsize::new(0);
static CAPTURE_STATE: AtomicUsize = AtomicUsize::new(0);
static CAPTURE_FAILURES: AtomicUsize = AtomicUsize::new(0);
static RENEW_FAILURES: AtomicUsize = AtomicUsize::new(0);
static RENEWALS: AtomicUsize = AtomicUsize::new(0);
static ACTIVE: AtomicBool = AtomicBool::new(true);
static FLOW: AtomicBool = AtomicBool::new(false);
static MODEL_READY: AtomicBool = AtomicBool::new(false);
static CHAT_READY: AtomicBool = AtomicBool::new(false);
static READY: AtomicBool = AtomicBool::new(false);
static OUTPUT: AtomicUsize = AtomicUsize::new(0);
static PLAYBACK_FRAMES: AtomicUsize = AtomicUsize::new(0);
static PLAYBACK_SEQUENCE: AtomicUsize = AtomicUsize::new(1);
static NOW: AtomicUsize = AtomicUsize::new(1);
static TURNS: AtomicUsize = AtomicUsize::new(0);
static STREAMING: AtomicBool = AtomicBool::new(false);
static VISIBLE: AtomicUsize = AtomicUsize::new(usize::MAX);
static LONG_REPLY: AtomicBool = AtomicBool::new(false);
const SHORT_RESPONSE: &[u8] = b"Hi. This is the next phrase of the response.";
const LONG_RESPONSE: &[u8] = b"This streamed response contains enough words to start a bounded synthesis request before the full answer has arrived while keeping each word in order and preserving the final part of the answer.";
static SPANS: std::sync::Mutex<Vec<(usize,usize,bool)>> = std::sync::Mutex::new(Vec::new());
static INPUT_GRANTS: AtomicUsize = AtomicUsize::new(0);
static INPUT_LENGTH: AtomicUsize = AtomicUsize::new(0);
static DIRECT_TRANSCRIPT: AtomicBool = AtomicBool::new(false);
static MICROPHONE: std::sync::Mutex<std::collections::VecDeque<i16>> = std::sync::Mutex::new(std::collections::VecDeque::new());
static PHRASES: std::sync::Mutex<Vec<Vec<u8>>> = std::sync::Mutex::new(Vec::new());
static RECOGNIZED: std::sync::Mutex<Vec<i16>> = std::sync::Mutex::new(Vec::new());
// ------------------------=
// FUNC: main
// DESC: Directs callers to the behavioral test harness rather than reporting an unexecuted test as success.
// ------------------=
fn main() { panic!("Run cargo test --bin voice-toggle-test through build-kit"); }
#[path = "../kernel/ui/geometry.rs"] pub mod geometry;
mod ui { pub use crate::geometry; }
mod runtime {
    pub mod execution {
        #[derive(Clone,Copy,PartialEq,Eq)] pub struct SecurityIdentity(pub [u8;16]);
    }
    pub mod capability {pub enum CapabilityType {AudioInput,AudioOutput}}
    pub mod identity {
        pub const MAX_SESSIONS:usize=1;
        #[derive(PartialEq)] pub enum SessionState {Active,Locked}
        pub struct Session {pub id:super::execution::SecurityIdentity,pub user:super::execution::SecurityIdentity,pub state:SessionState}
        pub struct AiProfile {pub speech_output_enabled:bool}
        #[derive(Clone,Copy)] pub enum WakeWord {Infinity,Computer}
        impl WakeWord {
            // ------------------------=
            // FUNC: phrase
            // DESC: Supplies the configured phrase to the production parser fixture.
            // ------------------=
            pub const fn phrase(self)->&'static[u8]{match self{Self::Infinity=>b"Infinity",Self::Computer=>b"Computer"}}
        }
        pub struct VoiceProfile {pub wake_word:WakeWord}
    }
    pub mod audio {#[derive(PartialEq)] pub enum CaptureState {Idle,Recording,Overrun,Complete,Cancelled,Denied,DeviceLost}}
    pub struct Identity;
    impl Identity {
        // ------------------------=
        // FUNC: session_nth
        // DESC: Supplies the active owner or a locked session for revocation testing.
        // ------------------=
        pub fn session_nth(&self,_:usize)->Option<identity::Session>{Some(identity::Session{id:execution::SecurityIdentity([1;16]),user:execution::SecurityIdentity([1;16]),state:if crate::ACTIVE.load(crate::Ordering::SeqCst){identity::SessionState::Active}else{identity::SessionState::Locked}})}
        // ------------------------=
        // FUNC: ai_profile
        // DESC: Supplies the spoken-reply preference for the production conversation controller.
        // ------------------=
        pub fn ai_profile(&self,_:execution::SecurityIdentity)->Option<identity::AiProfile>{Some(identity::AiProfile{speech_output_enabled:true})}
        // ------------------------=
        // FUNC: voice_profile
        // DESC: Supplies the default durable wake phrase to the production controller.
        // ------------------=
        pub fn voice_profile(&self,_:execution::SecurityIdentity)->Option<identity::VoiceProfile>{Some(identity::VoiceProfile{wake_word:identity::WakeWord::Infinity})}
    }
    pub struct Caps;
    impl Caps {
        // ------------------------=
        // FUNC: grant
        // DESC: Supplies a deterministic lease to the real controller.
        // ------------------=
        pub fn grant(&mut self,kind:capability::CapabilityType,_:u64,_:u64,_:u64,_:execution::SecurityIdentity,_:execution::SecurityIdentity,_:Option<u64>,_:u64)->Result<u64,()>{
            if matches!(kind,capability::CapabilityType::AudioInput){crate::INPUT_GRANTS.fetch_add(1,crate::Ordering::SeqCst);}
            Ok(1)
        }
        // ------------------------=
        // FUNC: retire_leaf
        // DESC: Accepts retirement of the fixture lease.
        // ------------------=
        pub fn retire_leaf(&mut self,_:u64,_:execution::SecurityIdentity)->Result<(),()>{Ok(())}
    }
    pub struct Runtime {pub identity:Identity,pub capabilities:Caps}
    // ------------------------=
    // FUNC: with_runtime
    // DESC: Gives the production controller deterministic native authority.
    // ------------------=
    pub fn with_runtime<T>(f:impl FnOnce(&mut Runtime)->T)->Option<T>{Some(f(&mut Runtime{identity:Identity,capabilities:Caps}))}
}
mod drivers {pub mod audio {
    use crate::runtime::{execution::SecurityIdentity,audio::CaptureState};
    pub struct Status {pub sample_rate:u32,pub state:CaptureState}
    // ------------------------=
    // FUNC: capture
    // DESC: Counts actual controller requests to reopen the microphone.
    // ------------------=
    pub fn capture(_:SecurityIdentity,_:u64)->bool{
        crate::CAPTURES.fetch_add(1,crate::Ordering::SeqCst);
        if crate::CAPTURE_FAILURES.fetch_update(crate::Ordering::SeqCst,crate::Ordering::SeqCst,|n|if n==0{None}else{Some(n-1)}).is_ok(){return false;}
        crate::CAPTURE_STATE.store(0,crate::Ordering::SeqCst);true
    }
    // ------------------------=
    // FUNC: capture_status
    // DESC: Supplies a supported native capture format.
    // ------------------=
    pub fn capture_status()->Option<Status>{Some(Status{sample_rate:16000,state:match crate::CAPTURE_STATE.load(crate::Ordering::SeqCst){1=>CaptureState::Overrun,2=>CaptureState::Complete,3=>CaptureState::Denied,4=>CaptureState::DeviceLost,_=>CaptureState::Recording}})}
    // ------------------------=
    // FUNC: stop_capture
    // DESC: Closes the deterministic capture seam.
    // ------------------=
    pub fn stop_capture(_:SecurityIdentity){crate::MICROPHONE.lock().unwrap().clear();}
    // ------------------------=
    // FUNC: capture_available
    // DESC: Declares the fixture capture device ready.
    // ------------------=
    pub fn capture_available()->bool{true}
    // ------------------------=
    // FUNC: playback_rate
    // DESC: Declares the deterministic output route ready for conversation admission.
    // ------------------=
    pub fn playback_rate()->Option<u32>{Some(48000)}
    // ------------------------=
    // FUNC: renew_capture
    // DESC: Renews only the deterministic fixture capture.
    // ------------------=
    pub fn renew_capture(_:SecurityIdentity,_:u64)->bool{
        crate::RENEWALS.fetch_add(1,crate::Ordering::SeqCst);
        crate::RENEW_FAILURES.fetch_update(crate::Ordering::SeqCst,crate::Ordering::SeqCst,|n|if n==0{None}else{Some(n-1)}).is_err()
    }
    // ------------------------=
    // FUNC: read_capture
    // DESC: Supplies silence without advancing recognition in toggle tests.
    // ------------------=
    pub fn read_capture(_:SecurityIdentity,out:&mut[i16])->usize{
        let mut input=crate::MICROPHONE.lock().unwrap();
        let count=out.len().min(input.len());
        for sample in &mut out[..count] {*sample=input.pop_front().unwrap();}
        count
    }
}}
mod chat {
    #[derive(PartialEq)] pub enum ChatRole {Assistant}
    #[derive(Clone,Copy,PartialEq)] pub enum GenerationState {Running,Complete,Failed,Cancelled,ContextFull}
    pub struct Message {pub role:ChatRole}
    impl Message {
        // ------------------------=
        // FUNC: text
        // DESC: Supplies bounded short and multi-span response bytes with controllable streaming visibility.
        // ------------------=
        pub fn text(&self)->&[u8]{
            let bytes: &[u8] = if crate::LONG_REPLY.load(crate::Ordering::SeqCst) {crate::LONG_RESPONSE}
                else if crate::TURNS.load(crate::Ordering::SeqCst)>1 {b"New response."} else {crate::SHORT_RESPONSE};
            &bytes[..bytes.len().min(crate::VISIBLE.load(crate::Ordering::SeqCst))]
        }
    }
    pub struct Chat {pub generation_state:GenerationState}
    impl Chat {
        // ------------------------=
        // FUNC: selected_model
        // DESC: Selects the fixture native model.
        // ------------------=
        pub fn selected_model(&self)->usize{0}
        // ------------------------=
        // FUNC: input
        // DESC: Exposes whether recognized text remains queued in the composer.
        // ------------------=
        pub fn input(&self)->&[u8]{if crate::INPUT_LENGTH.load(crate::Ordering::SeqCst)==0 {b""} else {b"test"}}
        // ------------------------=
        // FUNC: selected_model_ready
        // DESC: Exposes the deterministic local-model load boundary.
        // ------------------=
        pub fn selected_model_ready(&self)->bool{crate::CHAT_READY.load(crate::Ordering::SeqCst)}
        // ------------------------=
        // FUNC: turn_id
        // DESC: Supplies a stable owned chat turn.
        // ------------------=
        pub fn turn_id(&self)->u64{1}
        // ------------------------=
        // FUNC: set_enabled
        // DESC: Accepts the controller's chat activation.
        // ------------------=
        pub fn set_enabled(&mut self,_:bool){}
        // ------------------------=
        // FUNC: set_minimized
        // DESC: Accepts the controller's presentation request.
        // ------------------=
        pub fn set_minimized(&mut self,_:bool){}
        // ------------------------=
        // FUNC: push_input
        // DESC: Records recognized characters in the bounded composer fixture.
        // ------------------=
        pub fn push_input(&mut self,_:u8)->bool{crate::INPUT_LENGTH.fetch_add(1,crate::Ordering::SeqCst);true}
        // ------------------------=
        // FUNC: message_count
        // DESC: Reports no completed messages during lifecycle tests.
        // ------------------=
        pub fn message_count(&self)->usize{1}
        // ------------------------=
        // FUNC: message
        // DESC: Returns no fabricated assistant message.
        // ------------------=
        pub fn message(&self,_:usize)->Option<Message>{Some(Message{role:ChatRole::Assistant})}
    }
}
struct Ai {chat:chat::Chat}
impl Ai {
    // ------------------------=
    // FUNC: native_ready
    // DESC: Supplies a selectable model-loading state without affecting microphone activation.
    // ------------------=
    fn native_ready(&self,_:usize)->bool{MODEL_READY.load(Ordering::SeqCst)}
    // ------------------------=
    // FUNC: bind_chat_owner
    // DESC: Accepts the fixture owner binding.
    // ------------------=
    fn bind_chat_owner(&mut self,_:[u8;16]){}
    // ------------------------=
    // FUNC: cancel_chat
    // DESC: Supplies the cancellation seam.
    // ------------------=
    fn cancel_chat(&mut self){}
    // ------------------------=
    // FUNC: submit_chat
    // DESC: Rejects unused generation in lifecycle-only tests.
    // ------------------=
    fn submit_chat(&mut self)->bool{
        if !MODEL_READY.load(Ordering::SeqCst) {return false;}
        TURNS.fetch_add(1,Ordering::SeqCst);INPUT_LENGTH.store(0,Ordering::SeqCst);true
    }
}
// ------------------------=
// FUNC: with_ai_runtime
// DESC: Exposes an idle local model to the production controller.
// ------------------=
fn with_ai_runtime<T>(f:impl FnOnce(&mut Ai)->T)->T{f(&mut Ai{chat:chat::Chat{generation_state:if STREAMING.load(Ordering::SeqCst){chat::GenerationState::Running}else{chat::GenerationState::Complete}}})}
mod qwen {pub mod workers {
    // ------------------------=
    // FUNC: clock_ns
    // DESC: Uses fixed time to isolate toggle transitions.
    // ------------------=
    pub fn clock_ns()->u64{crate::NOW.load(crate::Ordering::SeqCst) as u64}
}}
mod voice_input {
    use crate::runtime::execution::SecurityIdentity;
    #[derive(PartialEq)] pub enum InputState {Queued,Recognizing,Ready,Failed,Cancelled}
    pub struct Status {pub state:InputState,pub error:i32}
    // ------------------------=
    // FUNC: prepare
    // DESC: Mirrors deterministic completion of the native recognizer warmup gate.
    // ------------------=
    pub fn prepare()->bool{crate::MODEL_READY.load(crate::Ordering::SeqCst)}
    // ------------------------=
    // FUNC: prepared
    // DESC: Exposes the fixture's explicit recognizer readiness state.
    // ------------------=
    pub fn prepared()->bool{crate::MODEL_READY.load(crate::Ordering::SeqCst)}
    // ------------------------=
    // FUNC: invalidate
    // DESC: Leaves recognition lifecycle control with the fixture's test transitions.
    // ------------------=
    pub fn invalidate(){}
    // ------------------------=
    // FUNC: status
    // DESC: Models asynchronous cancellation drain without running a recognizer.
    // ------------------=
    pub fn status()->Status{Status{state:if crate::READY.load(crate::Ordering::SeqCst){InputState::Ready}else if crate::BUSY.load(crate::Ordering::SeqCst){InputState::Recognizing}else{InputState::Cancelled},error:0}}
    // ------------------------=
    // FUNC: stop
    // DESC: Leaves the simulated worker busy until the test acknowledges cancellation.
    // ------------------=
    pub fn stop(_:SecurityIdentity){crate::READY.store(false,crate::Ordering::SeqCst);}
    // ------------------------=
    // FUNC: submit
    // DESC: Rejects unused recognition jobs during lifecycle tests.
    // ------------------=
    pub fn submit(_:SecurityIdentity,_:u64,pcm:&[i16])->Result<(),()>{
        if !crate::FLOW.load(crate::Ordering::SeqCst){return Err(());}
        *crate::RECOGNIZED.lock().unwrap()=pcm.to_vec();
        crate::READY.store(true,crate::Ordering::SeqCst);Ok(())
    }
    // ------------------------=
    // FUNC: take
    // DESC: Provides no fabricated transcript.
    // ------------------=
    pub fn take(_:SecurityIdentity,out:&mut[u8])->Result<usize,()>{
        crate::READY.store(false,crate::Ordering::SeqCst);
        let transcript: &[u8] = if crate::DIRECT_TRANSCRIPT.load(crate::Ordering::SeqCst) {
            b"hello"
        } else {
            b"Infinity test"
        };
        out[..transcript.len()].copy_from_slice(transcript);
        Ok(transcript.len())
    }
}
mod voice_output {
    // ------------------------=
    // FUNC: echo_reference
    // DESC: Supplies headphone playback with no acoustic return for the controller interruption test.
    // ------------------=
    pub fn echo_reference(_:crate::runtime::execution::SecurityIdentity,out:&mut[i16])->bool{out.fill(0);false}
    // ------------------------=
    // FUNC: can_prefetch
    // DESC: Leaves buffer concurrency to the separate production output harness.
    // ------------------=
    pub fn can_prefetch()->bool{false}
    pub const OUTPUT_LEASE_SECONDS: u64 = 130;
    use crate::runtime::execution::SecurityIdentity;
    #[derive(PartialEq)]
    pub enum OutputState {Queued,Synthesizing,Ready,Speaking,Complete,Failed,Cancelled,Buffered}
    pub struct Status {pub state:OutputState}
    pub struct PlaybackProgress {pub sequence:usize,pub frames:usize,pub total_frames:usize,pub content_position:usize,pub content_boundary:bool}
    // ------------------------=
    // FUNC: playback_progress
    // DESC: Exposes deterministic DMA progress for text and speech synchronization assertions.
    // ------------------=
    pub fn playback_progress(_:SecurityIdentity)->Option<PlaybackProgress>{
        if crate::OUTPUT.load(crate::Ordering::SeqCst)==1 {
            let frames=crate::PLAYBACK_FRAMES.load(crate::Ordering::SeqCst);
            Some(PlaybackProgress{sequence:crate::PLAYBACK_SEQUENCE.load(crate::Ordering::SeqCst),frames,total_frames:100,content_position:frames*3/100,content_boundary:frames==100})
        } else {None}
    }
    // ------------------------=
    // FUNC: status
    // DESC: Models an acknowledged playback stop.
    // ------------------=
    pub fn status()->Status{Status{state:match crate::OUTPUT.load(crate::Ordering::SeqCst){1=>OutputState::Speaking,2=>OutputState::Complete,3=>OutputState::Synthesizing,4=>OutputState::Buffered,_=>OutputState::Cancelled}}}
    // ------------------------=
    // FUNC: stop
    // DESC: Supplies the audio-output cancellation seam.
    // ------------------=
    pub fn stop(_:SecurityIdentity){crate::OUTPUT.store(0,crate::Ordering::SeqCst);}
    // ------------------------=
    // FUNC: submit
    // DESC: Rejects unused synthesis jobs during lifecycle testing.
    // ------------------=
    pub fn submit(_:SecurityIdentity,_:u64,text:&[u8])->Result<(),()>{
        if !crate::FLOW.load(crate::Ordering::SeqCst){return Err(());}
        assert!(text.len()<=160);
        crate::PHRASES.lock().unwrap().push(text.to_vec());
        crate::OUTPUT.store(3,crate::Ordering::SeqCst);Ok(())
    }
    // ------------------------=
    // FUNC: submit_span
    // DESC: Records a generation-safe content span through the conversation fixture.
    // ------------------=
    pub fn submit_span(owner:SecurityIdentity,cap:u64,text:&[u8],start:usize,end:usize,final_span:bool)->Result<(),()>{
        assert_eq!(text.len(),end-start);
        submit(owner,cap,text)?;
        crate::SPANS.lock().unwrap().push((start,end,final_span));
        Ok(())
    }
    // ------------------------=
    // FUNC: seal_buffered
    // DESC: Starts deterministic playback after every response span is prepared.
    // ------------------=
    pub fn seal_buffered(_:SecurityIdentity)->bool{crate::OUTPUT.store(1,crate::Ordering::SeqCst);true}
}
#[path = "../kernel/runtime/ai/voice_pcm.rs"] mod voice_pcm;
#[path = "../kernel/runtime/ai/voice_vad.rs"] mod voice_vad;
#[path = "../kernel/runtime/ai/wake_word.rs"] mod wake_word;
#[path = "../kernel/runtime/ai/voice_conversation.rs"] mod conversation;
// ------------------------=
// FUNC: toggles_restart_after_drain_without_reopening_after_revocation
// DESC: Executes rapid off/on/off/on, cancellation acknowledgement, repeated reuse and owner revocation through the production controller.
// ------------------=
#[test]
fn toggles_restart_after_drain_without_reopening_after_revocation(){
    use conversation::State;
    let owner=runtime::execution::SecurityIdentity([1;16]);
    MODEL_READY.store(false,Ordering::SeqCst);
    CHAT_READY.store(false,Ordering::SeqCst);
    assert!(!conversation::toggle(owner));assert_eq!(conversation::state().0,State::Off);
    MODEL_READY.store(true,Ordering::SeqCst);
    CHAT_READY.store(true,Ordering::SeqCst);
    assert!(conversation::toggle(owner));assert_eq!(conversation::state().0,State::Listening);
    BUSY.store(true,Ordering::SeqCst);
    assert!(conversation::toggle(owner));assert_eq!(conversation::state().0,State::Stopping);
    assert!(conversation::toggle(owner));conversation::poll();assert_eq!(CAPTURES.load(Ordering::SeqCst),1);
    assert!(conversation::toggle(owner));BUSY.store(false,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Off);assert_eq!(CAPTURES.load(Ordering::SeqCst),1);
    for count in 2..5 {
        assert!(conversation::toggle(owner));assert_eq!(conversation::state().0,State::Listening);
        assert!(conversation::toggle(owner));assert!(conversation::toggle(owner));conversation::poll();
        assert_eq!(conversation::state().0,State::Listening);assert_eq!(CAPTURES.load(Ordering::SeqCst),count*2-1);
        conversation::stop(owner);conversation::poll();
    }
    assert!(conversation::toggle(owner));assert!(conversation::toggle(owner));assert!(conversation::toggle(owner));
    ACTIVE.store(false,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Failed);
    ACTIVE.store(true,Ordering::SeqCst);FLOW.store(true,Ordering::SeqCst);
    assert!(conversation::start(owner));
    MICROPHONE.lock().unwrap().extend([1000;1600]);
    MICROPHONE.lock().unwrap().extend([0;12000]);
    for _ in 0..10 {conversation::poll();if conversation::state().0==State::Thinking {break;}}
    assert_eq!(conversation::state().0,State::Thinking);
    assert_eq!(INPUT_LENGTH.load(Ordering::SeqCst),0);
    assert_eq!(TURNS.load(Ordering::SeqCst),1);
    let captures_before_reply=CAPTURES.load(Ordering::SeqCst);
    conversation::poll();
    assert_eq!(conversation::state().0,State::Speaking);
    assert_eq!(CAPTURES.load(Ordering::SeqCst),captures_before_reply);
    assert_eq!(*PHRASES.lock().unwrap(),vec![SHORT_RESPONSE.to_vec()]);
    // Queued synthesis and active playback retain exclusive conversation audio
    // ownership until every response phrase has completed.
    conversation::poll();
    assert_eq!(CAPTURES.load(Ordering::SeqCst),captures_before_reply);
    OUTPUT.store(1,Ordering::SeqCst);PLAYBACK_FRAMES.store(50,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::synchronized_reply_length(1,SHORT_RESPONSE.len()),0);
    PLAYBACK_FRAMES.store(100,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::synchronized_reply_length(1,SHORT_RESPONSE.len()),3);
    let captures=CAPTURES.load(Ordering::SeqCst);
    assert_eq!(captures,captures_before_reply);
    MODEL_READY.store(false,Ordering::SeqCst);
    OUTPUT.store(2,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Starting);
    assert_eq!(PHRASES.lock().unwrap().len(),1);
    assert_eq!(CAPTURES.load(Ordering::SeqCst),captures);
    MODEL_READY.store(true,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Listening);
    assert!(conversation::wake_armed());
    assert_eq!(CAPTURES.load(Ordering::SeqCst),captures);
    for terminal in [1,2] {
        let opened=CAPTURES.load(Ordering::SeqCst);
        CAPTURE_STATE.store(terminal,Ordering::SeqCst);conversation::poll();conversation::poll();
        assert_eq!(conversation::state().0,State::Listening);
        assert_eq!(CAPTURES.load(Ordering::SeqCst),opened+1);
    }
    // A codec may need one service interval after a finite DMA window stops.
    // Recovery must retain already accepted speech instead of cycling voice
    // autostart and clearing the utterance before Whisper can receive it.
    MICROPHONE.lock().unwrap().extend([1900;1600]);
    conversation::poll();
    let opened=CAPTURES.load(Ordering::SeqCst);
    CAPTURE_FAILURES.store(1,Ordering::SeqCst);
    CAPTURE_STATE.store(1,Ordering::SeqCst);
    conversation::poll();
    assert_eq!(conversation::state().0,State::Listening);
    conversation::poll();
    assert_eq!(CAPTURES.load(Ordering::SeqCst),opened+1);
    assert_eq!(conversation::state().0,State::Listening);
    NOW.store(100_000_002,Ordering::SeqCst);
    conversation::poll();
    assert_eq!(CAPTURES.load(Ordering::SeqCst),opened+2);
    MICROPHONE.lock().unwrap().extend([0;12000]);
    for _ in 0..4 {conversation::poll();if conversation::state().0==State::Recognizing {break;}}
    assert_eq!(conversation::state().0,State::Recognizing);
    assert!(RECOGNIZED.lock().unwrap().iter().any(|&v|v==1900));
    conversation::poll();conversation::poll();
    conversation::stop(owner);conversation::poll();
    assert!(conversation::start(owner));
    // A concurrent DMA pump may hold the adapter lock during a renewal poll.
    // One missed renewal must retain the active utterance and retry without
    // returning voice control to desktop autostart.
    let renewals=RENEWALS.load(Ordering::SeqCst);
    RENEW_FAILURES.store(1,Ordering::SeqCst);
    NOW.store(1_100_000_003,Ordering::SeqCst);
    conversation::poll();
    assert_eq!(conversation::state().0,State::Listening);
    assert_eq!(RENEWALS.load(Ordering::SeqCst),renewals+1);
    NOW.store(1_110_000_003,Ordering::SeqCst);
    conversation::poll();
    assert_eq!(conversation::state().0,State::Listening);
    assert_eq!(RENEWALS.load(Ordering::SeqCst),renewals+2);
    let phrase_baseline=PHRASES.lock().unwrap().len();
    MICROPHONE.lock().unwrap().extend([0;3200]);
    MICROPHONE.lock().unwrap().extend([1700;1600]);
    MICROPHONE.lock().unwrap().extend([0;12000]);
    let input_grants=INPUT_GRANTS.load(Ordering::SeqCst);
    for _ in 0..4 {conversation::poll();if conversation::state().0==State::Recognizing {break;}}
    assert_eq!(conversation::state().0,State::Recognizing);
    assert_eq!(INPUT_GRANTS.load(Ordering::SeqCst),input_grants+1);
    assert_eq!(PHRASES.lock().unwrap().len(),phrase_baseline);
    assert!(RECOGNIZED.lock().unwrap().iter().any(|&v|v==1700));
    conversation::poll();conversation::poll();
    assert_eq!(PHRASES.lock().unwrap()[phrase_baseline],b"New response.");
    // A second reply advances without waiting for microphone input or a gap timer.
    conversation::stop(owner);conversation::poll();TURNS.store(0,Ordering::SeqCst);
    assert!(conversation::start(owner));
    MICROPHONE.lock().unwrap().extend([1000;1600]);MICROPHONE.lock().unwrap().extend([0;12000]);
    for _ in 0..10 {conversation::poll();}
    OUTPUT.store(2,Ordering::SeqCst);conversation::poll();
    let phrases=PHRASES.lock().unwrap().len();
    NOW.store(3_000_000_002,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Listening);
    assert_eq!(PHRASES.lock().unwrap().len(),phrases);
    conversation::stop(owner);
    conversation::poll();assert_eq!(conversation::state().0,State::Off);
    TURNS.store(0,Ordering::SeqCst);assert!(conversation::start(owner));
    MICROPHONE.lock().unwrap().extend([1000;1600]);MICROPHONE.lock().unwrap().extend([0;12000]);
    for _ in 0..10 {conversation::poll();}
    OUTPUT.store(2,Ordering::SeqCst);conversation::poll();
    let phrases=PHRASES.lock().unwrap().len();
    ACTIVE.store(false,Ordering::SeqCst);conversation::poll();conversation::poll();
    assert_eq!(conversation::state().0,State::Off);
    assert_eq!(PHRASES.lock().unwrap().len(),phrases);
    ACTIVE.store(true,Ordering::SeqCst);
    STREAMING.store(true,Ordering::SeqCst); TURNS.store(0,Ordering::SeqCst);
    VISIBLE.store(2,Ordering::SeqCst);
    conversation::speak_visible_reply(owner,1);
    conversation::poll(); assert_eq!(conversation::state().0,State::Thinking);
    let before=PHRASES.lock().unwrap().len();
    VISIBLE.store(4,Ordering::SeqCst);
    conversation::poll(); assert_eq!(conversation::state().0,State::Thinking);
    assert_eq!(PHRASES.lock().unwrap().len(),before);
    STREAMING.store(false,Ordering::SeqCst);VISIBLE.store(usize::MAX,Ordering::SeqCst);
    conversation::poll(); assert_eq!(PHRASES.lock().unwrap().len(),before+1);
    assert_eq!(PHRASES.lock().unwrap()[before],SHORT_RESPONSE);
    OUTPUT.store(2,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Off);

    // Long streamed answers still publish before generation completes. Every
    // accepted byte and content range reaches exactly one synthesis request.
    LONG_REPLY.store(true,Ordering::SeqCst);
    STREAMING.store(true,Ordering::SeqCst);VISIBLE.store(4,Ordering::SeqCst);
    let before=PHRASES.lock().unwrap().len();
    let spans=SPANS.lock().unwrap().len();
    conversation::speak_visible_reply(owner,1);
    conversation::poll();assert_eq!(conversation::state().0,State::Thinking);
    assert_eq!(PHRASES.lock().unwrap().len(),before);
    VISIBLE.store(78,Ordering::SeqCst);
    conversation::poll();assert_eq!(conversation::state().0,State::Speaking);
    assert_eq!(PHRASES.lock().unwrap().len(),before+1);
    let first_length=PHRASES.lock().unwrap()[before].len();
    assert!(first_length<=80 && first_length<LONG_RESPONSE.len());
    assert_eq!(SPANS.lock().unwrap()[spans],(0,first_length,false));
    OUTPUT.store(4,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Thinking);
    assert_eq!(PHRASES.lock().unwrap().len(),before+1);
    STREAMING.store(false,Ordering::SeqCst);VISIBLE.store(usize::MAX,Ordering::SeqCst);
    conversation::poll();assert_eq!(conversation::state().0,State::Speaking);
    assert_eq!(PHRASES.lock().unwrap().len(),before+2);
    assert_eq!(PHRASES.lock().unwrap()[before..].concat(),LONG_RESPONSE);
    assert_eq!(SPANS.lock().unwrap()[spans+1],(first_length,LONG_RESPONSE.len(),true));
    OUTPUT.store(1,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Speaking);
    OUTPUT.store(2,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Off);
    LONG_REPLY.store(false,Ordering::SeqCst);
    STREAMING.store(false,Ordering::SeqCst);TURNS.store(0,Ordering::SeqCst);
    assert!(conversation::start(owner));
    conversation::speak_visible_reply(owner,1);conversation::poll();
    assert_eq!(conversation::state().0,State::Speaking);
    let phrases=PHRASES.lock().unwrap().len();
    let opened=CAPTURES.load(Ordering::SeqCst);
    OUTPUT.store(1,Ordering::SeqCst);conversation::poll();
    // Microphone energy during output must never cancel or truncate playback.
    // Listening resumes only after the complete response drains.
    MICROPHONE.lock().unwrap().extend([2300;1600]);
    conversation::poll();
    assert_eq!(OUTPUT.load(Ordering::SeqCst),1);
    assert_eq!(conversation::state().0,State::Speaking);
    MICROPHONE.lock().unwrap().extend([2300;3200]);
    conversation::poll();
    assert_eq!(OUTPUT.load(Ordering::SeqCst),1);
    assert_eq!(conversation::state().0,State::Speaking);
    assert_eq!(CAPTURES.load(Ordering::SeqCst),opened);
    assert_eq!(PHRASES.lock().unwrap().len(),phrases);
    conversation::stop(owner);conversation::poll();
    for terminal in [3,4] {
        assert!(conversation::start(owner));
        let opened=CAPTURES.load(Ordering::SeqCst);
        CAPTURE_STATE.store(terminal,Ordering::SeqCst);
        conversation::poll();conversation::poll();
        assert_eq!(conversation::state().0,State::Off);
        assert_eq!(CAPTURES.load(Ordering::SeqCst),opened);
    }
    // A slow first inference can outlast an input DMA ring. Recovery must
    // retain continuous conversation instead of turning the reply into a
    // one-way typed response whose completion leaves the microphone off.
    TURNS.store(0,Ordering::SeqCst);assert!(conversation::start(owner));
    MICROPHONE.lock().unwrap().extend([1800;1600]);MICROPHONE.lock().unwrap().extend([0;12000]);
    for _ in 0..10 {conversation::poll();if conversation::state().0==State::Thinking {break;}}
    assert_eq!(conversation::state().0,State::Thinking);
    STREAMING.store(true,Ordering::SeqCst);VISIBLE.store(0,Ordering::SeqCst);
    conversation::poll();assert_eq!(conversation::state().0,State::Thinking);
    let opened=CAPTURES.load(Ordering::SeqCst);
    CAPTURE_STATE.store(1,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Thinking);
    assert_eq!(CAPTURES.load(Ordering::SeqCst),opened);
    STREAMING.store(false,Ordering::SeqCst);VISIBLE.store(usize::MAX,Ordering::SeqCst);
    conversation::poll();assert_eq!(conversation::state().0,State::Speaking);
    OUTPUT.store(1,Ordering::SeqCst);conversation::poll();
    assert_eq!(CAPTURES.load(Ordering::SeqCst),opened);
    OUTPUT.store(2,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Listening);
    MICROPHONE.lock().unwrap().extend([2400;1600]);MICROPHONE.lock().unwrap().extend([0;12000]);
    for _ in 0..4 {conversation::poll();if conversation::state().0==State::Recognizing {break;}}
    assert_eq!(conversation::state().0,State::Recognizing);
    assert!(RECOGNIZED.lock().unwrap().iter().any(|&v|v==2400));
    conversation::stop(owner);conversation::poll();

    // A long-lived conversation rotates expiring native authority without
    // reopening the VirtualBox/CoreAudio device or losing listening state.
    assert!(conversation::start(owner));
    let opened=CAPTURES.load(Ordering::SeqCst);
    let grants=INPUT_GRANTS.load(Ordering::SeqCst);
    NOW.store(49_000_000_003,Ordering::SeqCst);
    conversation::poll();
    assert_eq!(conversation::state().0,State::Listening);
    assert_eq!(CAPTURES.load(Ordering::SeqCst),opened);
    assert_eq!(INPUT_GRANTS.load(Ordering::SeqCst),grants+1);
    conversation::stop(owner);conversation::poll();

    // An explicitly enabled visible voice session must submit a valid Whisper
    // transcript even when recognition omits the optional wake phrase.
    DIRECT_TRANSCRIPT.store(true,Ordering::SeqCst);
    TURNS.store(0,Ordering::SeqCst);
    assert!(conversation::start(owner));
    MICROPHONE.lock().unwrap().extend([2100;1600]);
    MICROPHONE.lock().unwrap().extend([0;12000]);
    for _ in 0..10 {conversation::poll();if conversation::state().0==State::Thinking {break;}}
    assert_eq!(conversation::state().0,State::Thinking);
    assert_eq!(TURNS.load(Ordering::SeqCst),1);
    conversation::stop(owner);conversation::poll();
    DIRECT_TRANSCRIPT.store(false,Ordering::SeqCst);
}
