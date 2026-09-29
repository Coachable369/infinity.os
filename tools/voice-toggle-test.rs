//! Executes the production conversation controller with deterministic audio and identity seams.
#![allow(dead_code)]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
static BUSY: AtomicBool = AtomicBool::new(false);
static CAPTURES: AtomicUsize = AtomicUsize::new(0);
static CAPTURE_STATE: AtomicUsize = AtomicUsize::new(0);
static ACTIVE: AtomicBool = AtomicBool::new(true);
static FLOW: AtomicBool = AtomicBool::new(false);
static MODEL_READY: AtomicBool = AtomicBool::new(false);
static READY: AtomicBool = AtomicBool::new(false);
static OUTPUT: AtomicUsize = AtomicUsize::new(0);
static NOW: AtomicUsize = AtomicUsize::new(1);
static TURNS: AtomicUsize = AtomicUsize::new(0);
static STREAMING: AtomicBool = AtomicBool::new(false);
static VISIBLE: AtomicUsize = AtomicUsize::new(usize::MAX);
static INPUT_GRANTS: AtomicUsize = AtomicUsize::new(0);
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
    }
    pub mod audio {#[derive(PartialEq)] pub enum CaptureState {Recording,Overrun,Complete,Denied,DeviceLost}}
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
    pub fn capture(_:SecurityIdentity,_:u64)->bool{crate::CAPTURES.fetch_add(1,crate::Ordering::SeqCst);crate::CAPTURE_STATE.store(0,crate::Ordering::SeqCst);true}
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
    // FUNC: renew_capture
    // DESC: Renews only the deterministic fixture capture.
    // ------------------=
    pub fn renew_capture(_:SecurityIdentity,_:u64)->bool{true}
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
        // DESC: Supplies no fabricated assistant speech.
        // ------------------=
        pub fn text(&self)->&[u8]{
            let bytes: &[u8] = if crate::TURNS.load(crate::Ordering::SeqCst)>1 {b"New response."} else {b"Hi. This is the next phrase of the response."};
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
        // DESC: Leaves the composer empty for explicit voice activation.
        // ------------------=
        pub fn input(&self)->&[u8]{b""}
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
        // DESC: Provides the unused recognition submission seam.
        // ------------------=
        pub fn push_input(&mut self,_:u8){}
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
    fn submit_chat(&mut self)->bool{TURNS.fetch_add(1,Ordering::SeqCst); FLOW.load(Ordering::SeqCst)}
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
        crate::READY.store(false,crate::Ordering::SeqCst);out[..4].copy_from_slice(b"test");Ok(4)
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
    pub enum OutputState {Queued,Synthesizing,Ready,Speaking,Complete,Failed,Cancelled}
    pub struct Status {pub state:OutputState}
    // ------------------------=
    // FUNC: status
    // DESC: Models an acknowledged playback stop.
    // ------------------=
    pub fn status()->Status{Status{state:match crate::OUTPUT.load(crate::Ordering::SeqCst){1=>OutputState::Speaking,2=>OutputState::Complete,_=>OutputState::Cancelled}}}
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
        crate::OUTPUT.store(1,crate::Ordering::SeqCst);Ok(())
    }
}
#[path = "../kernel/runtime/ai/voice_pcm.rs"] mod voice_pcm;
#[path = "../kernel/runtime/ai/voice_vad.rs"] mod voice_vad;
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
    for _ in 0..10 {conversation::poll();}
    assert_eq!(conversation::state().0,State::Speaking);
    assert_eq!(*PHRASES.lock().unwrap(),vec![b"Hi.".to_vec()]);
    // Duplex capture stays open while playback remains in progress.
    let captures=CAPTURES.load(Ordering::SeqCst);
    conversation::poll();assert_eq!(CAPTURES.load(Ordering::SeqCst),captures);
    OUTPUT.store(2,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Speaking);
    assert_eq!(CAPTURES.load(Ordering::SeqCst),captures);
    assert_eq!(PHRASES.lock().unwrap().len(),2);
    OUTPUT.store(2,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Listening);
    for terminal in [1,2] {
        let opened=CAPTURES.load(Ordering::SeqCst);
        CAPTURE_STATE.store(terminal,Ordering::SeqCst);conversation::poll();
        assert_eq!(conversation::state().0,State::Listening);
        assert_eq!(CAPTURES.load(Ordering::SeqCst),opened+1);
    }
    MICROPHONE.lock().unwrap().extend([0;3200]);
    MICROPHONE.lock().unwrap().extend([1700;1600]);
    MICROPHONE.lock().unwrap().extend([0;12000]);
    let input_grants=INPUT_GRANTS.load(Ordering::SeqCst);
    for _ in 0..4 {conversation::poll();if conversation::state().0==State::Recognizing {break;}}
    assert_eq!(conversation::state().0,State::Recognizing);
    assert_eq!(INPUT_GRANTS.load(Ordering::SeqCst),input_grants);
    assert_eq!(PHRASES.lock().unwrap().len(),2);
    assert!(RECOGNIZED.lock().unwrap().iter().any(|&v|v==1700));
    conversation::poll();conversation::poll();
    assert_eq!(PHRASES.lock().unwrap()[2],b"New response.");
    // A second reply advances without waiting for microphone input or a gap timer.
    conversation::stop(owner);conversation::poll();TURNS.store(0,Ordering::SeqCst);
    assert!(conversation::start(owner));
    MICROPHONE.lock().unwrap().extend([1000;1600]);MICROPHONE.lock().unwrap().extend([0;12000]);
    for _ in 0..10 {conversation::poll();}
    OUTPUT.store(2,Ordering::SeqCst);conversation::poll();
    let phrases=PHRASES.lock().unwrap().len();
    NOW.store(3_000_000_002,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Speaking);
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
    conversation::poll(); assert_eq!(conversation::state().0,State::Speaking);
    assert_eq!(PHRASES.lock().unwrap()[before],b"Hi.");
    OUTPUT.store(2,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Thinking);
    STREAMING.store(false,Ordering::SeqCst);VISIBLE.store(usize::MAX,Ordering::SeqCst);
    conversation::poll(); assert_eq!(PHRASES.lock().unwrap().len(),before+2);
    OUTPUT.store(2,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Off);
    STREAMING.store(false,Ordering::SeqCst);TURNS.store(0,Ordering::SeqCst);
    assert!(conversation::start(owner));
    conversation::speak_visible_reply(owner,1);conversation::poll();
    assert_eq!(conversation::state().0,State::Speaking);
    let phrases=PHRASES.lock().unwrap().len();
    MICROPHONE.lock().unwrap().extend([2300;1600]);
    conversation::poll();
    assert_eq!(OUTPUT.load(Ordering::SeqCst),0);
    assert_eq!(conversation::state().0,State::Listening);
    MICROPHONE.lock().unwrap().extend([0;12000]);
    for _ in 0..4 {conversation::poll();if conversation::state().0==State::Recognizing {break;}}
    assert_eq!(conversation::state().0,State::Recognizing);
    assert!(RECOGNIZED.lock().unwrap().iter().any(|&v|v==2300));
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
    assert_eq!(CAPTURES.load(Ordering::SeqCst),opened+1);
    STREAMING.store(false,Ordering::SeqCst);VISIBLE.store(usize::MAX,Ordering::SeqCst);
    conversation::poll();assert_eq!(conversation::state().0,State::Speaking);
    OUTPUT.store(2,Ordering::SeqCst);conversation::poll();
    OUTPUT.store(2,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Listening);
    MICROPHONE.lock().unwrap().extend([2400;1600]);MICROPHONE.lock().unwrap().extend([0;12000]);
    for _ in 0..4 {conversation::poll();if conversation::state().0==State::Recognizing {break;}}
    assert_eq!(conversation::state().0,State::Recognizing);
    assert!(RECOGNIZED.lock().unwrap().iter().any(|&v|v==2400));
    conversation::stop(owner);conversation::poll();
}
