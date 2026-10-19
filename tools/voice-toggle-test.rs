//! Executes the production conversation controller with deterministic audio and identity seams.
#![allow(dead_code)]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
static BUSY: AtomicBool = AtomicBool::new(false);
static CAPTURES: AtomicUsize = AtomicUsize::new(0);
static CAPTURE_STATE: AtomicUsize = AtomicUsize::new(0);
static CAPTURE_FAILURES: AtomicUsize = AtomicUsize::new(0);
static RENEW_FAILURES: AtomicUsize = AtomicUsize::new(0);
static RENEWALS: AtomicUsize = AtomicUsize::new(0);
static DISCARD_BUSY: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(true);
static FLOW: AtomicBool = AtomicBool::new(false);
static MODEL_READY: AtomicBool = AtomicBool::new(false);
static PREPARE_FAILED: AtomicBool = AtomicBool::new(false);
static PREPARE_CALLS: AtomicUsize = AtomicUsize::new(0);
static CAPTURE_AVAILABLE: AtomicBool = AtomicBool::new(true);
static PLAYBACK_AVAILABLE: AtomicBool = AtomicBool::new(true);
static CALIBRATION_PLAYBACK: AtomicUsize = AtomicUsize::new(0);
static CHAT_READY: AtomicBool = AtomicBool::new(false);
static READY: AtomicBool = AtomicBool::new(false);
static RECOGNITION_ERROR: AtomicUsize = AtomicUsize::new(0);
static HOLD_RECOGNITION: AtomicBool = AtomicBool::new(false);
static OUTPUT: AtomicUsize = AtomicUsize::new(0);
static OUTPUT_REQUEST: AtomicUsize = AtomicUsize::new(0);
static ECHO_PLAYBACK: AtomicBool = AtomicBool::new(false);
static OUTPUT_STOPS: AtomicUsize = AtomicUsize::new(0);
static PLAYBACK_FRAMES: AtomicUsize = AtomicUsize::new(0);
static PLAYBACK_SEQUENCE: AtomicUsize = AtomicUsize::new(1);
static FIRST_PLAYBACK_NS: AtomicUsize = AtomicUsize::new(0);
static NOW: AtomicUsize = AtomicUsize::new(1);
static TURNS: AtomicUsize = AtomicUsize::new(0);
static STREAMING: AtomicBool = AtomicBool::new(false);
static GENERATION_FAILURE: AtomicUsize = AtomicUsize::new(0);
static HOLD_OUTPUT_STOP: AtomicBool = AtomicBool::new(false);
static OUTPUT_SUBMIT_FAILURE: AtomicBool = AtomicBool::new(false);
static OUTPUT_SUBMIT_REJECTIONS: AtomicUsize = AtomicUsize::new(0);
static CANCEL_CALLS: AtomicUsize = AtomicUsize::new(0);
static BACKGROUND_GOAL: AtomicBool = AtomicBool::new(false);
static APP_GENERATING: AtomicBool = AtomicBool::new(false);
static VISIBLE: AtomicUsize = AtomicUsize::new(usize::MAX);
static LONG_REPLY: AtomicBool = AtomicBool::new(false);
const SHORT_RESPONSE: &[u8] = b"Hi. This is the next phrase of the response.";
const LONG_RESPONSE: &[u8] = b"This streamed response contains enough words to start a bounded synthesis request before the full answer has arrived while keeping each word in order and preserving the final part of the answer.";
static SPANS: std::sync::Mutex<Vec<(usize,usize,bool)>> = std::sync::Mutex::new(Vec::new());
static INPUT_GRANTS: AtomicUsize = AtomicUsize::new(0);
static INPUT_LENGTH: AtomicUsize = AtomicUsize::new(0);
static INPUT_FAIL_AT: AtomicUsize = AtomicUsize::new(usize::MAX);
static SELECTED_MODEL: AtomicUsize = AtomicUsize::new(0);
static WAKE_SETTING: AtomicUsize = AtomicUsize::new(0);
static TEST_TRANSCRIPT: std::sync::Mutex<Option<Vec<u8>>> = std::sync::Mutex::new(None);
static COMPOSER: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());
static PROMPTS: std::sync::Mutex<Vec<Vec<u8>>> = std::sync::Mutex::new(Vec::new());
static MICROPHONE: std::sync::Mutex<std::collections::VecDeque<i16>> = std::sync::Mutex::new(std::collections::VecDeque::new());
static PHRASES: std::sync::Mutex<Vec<Vec<u8>>> = std::sync::Mutex::new(Vec::new());
static RECOGNIZED: std::sync::Mutex<Vec<i16>> = std::sync::Mutex::new(Vec::new());
static RECOGNITION_REQUESTS: std::sync::Mutex<Vec<Vec<i16>>> = std::sync::Mutex::new(Vec::new());
// ------------------------=
// FUNC: main
// DESC: Directs callers to the behavioral test harness rather than reporting an unexecuted test as success.
// ------------------=
fn main() { panic!("Run cargo test --bin voice-toggle-test through build-kit"); }
#[path = "../kernel/ui/geometry.rs"] pub mod geometry;
mod ui { pub use crate::geometry; }
mod output {
    // ------------------------=
    // FUNC: write
    // DESC: Provides the production trace sink without using diagnostic text as a behavioral oracle.
    // ------------------=
    pub unsafe fn write(bytes: &[u8]) {
        use std::io::Write;
        let _ = std::io::stderr().write_all(bytes);
    }
}
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
        #[derive(Clone,Copy,PartialEq)] pub enum WakeWord {Infinity,Computer}
        impl WakeWord {
            // ------------------------=
            // FUNC: phrase
            // DESC: Supplies the configured phrase to the production parser fixture.
            // ------------------=
            pub const fn phrase(self)->&'static[u8]{match self{Self::Infinity=>b"Infinity",Self::Computer=>b"Computer"}}
        }
        pub struct VoiceProfile {pub wake_word:WakeWord}
    }
    pub mod audio {
        #[derive(PartialEq)] pub enum CaptureState {Idle,Recording,Overrun,Complete,Cancelled,Denied,DeviceLost}
        #[derive(PartialEq)] pub enum PlaybackState {Idle,Playing}
    }
    pub struct Identity;
    impl Identity {
        // ------------------------=
        // FUNC: session_nth
        // DESC: Supplies the active owner or a locked session for revocation testing.
        // ------------------=
        pub fn session_nth(&self,_:usize)->Option<identity::Session>{Some(identity::Session{id:execution::SecurityIdentity([1;16]),user:execution::SecurityIdentity([2;16]),state:if crate::ACTIVE.load(crate::Ordering::SeqCst){identity::SessionState::Active}else{identity::SessionState::Locked}})}
        // ------------------------=
        // FUNC: ai_profile
        // DESC: Supplies the spoken-reply preference for the production conversation controller.
        // ------------------=
        pub fn ai_profile(&self,_:execution::SecurityIdentity)->Option<identity::AiProfile>{Some(identity::AiProfile{speech_output_enabled:true})}
        // ------------------------=
        // FUNC: voice_profile
        // DESC: Supplies the selected durable wake phrase to the production controller.
        // ------------------=
        pub fn voice_profile(&self,_:execution::SecurityIdentity)->Option<identity::VoiceProfile>{Some(identity::VoiceProfile{wake_word:if crate::WAKE_SETTING.load(crate::Ordering::SeqCst)==0 {identity::WakeWord::Infinity}else{identity::WakeWord::Computer}})}
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
        // Real cold capture supplies a half-second ambient baseline before the
        // fixture's first spoken phrase, just as the widget instructs the user.
        if crate::conversation::microphone_threshold().is_none() {
            let mut input=crate::MICROPHONE.lock().unwrap();
            for _ in 0..8000 {input.push_front(0);}
        }
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
    pub fn capture_available()->bool{crate::CAPTURE_AVAILABLE.load(crate::Ordering::SeqCst)}
    // ------------------------=
    // FUNC: playback_rate
    // DESC: Declares the deterministic output route ready for conversation admission.
    // ------------------=
    pub fn playback_rate()->Option<u32>{crate::PLAYBACK_AVAILABLE.load(crate::Ordering::SeqCst).then_some(48000)}
    // ------------------------=
    // FUNC: playback_state
    // DESC: Supplies idle native output for ambient calibration without simulating microphone speech.
    // ------------------=
    pub fn playback_state()->Option<crate::runtime::audio::PlaybackState>{match crate::CALIBRATION_PLAYBACK.load(crate::Ordering::SeqCst){1=>Some(crate::runtime::audio::PlaybackState::Playing),2=>None,_=>Some(crate::runtime::audio::PlaybackState::Idle)}}
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
    // ------------------------=
    // FUNC: discard_capture_pending
    // DESC: Models an authorized atomic capture boundary without reopening DMA or advancing the clock.
    // ------------------=
    pub fn discard_capture_pending(owner:SecurityIdentity)->Option<bool>{
        if crate::DISCARD_BUSY.load(crate::Ordering::SeqCst) { return None; }
        if owner.0 != [1;16] || !crate::ACTIVE.load(crate::Ordering::SeqCst) { return Some(false); }
        crate::MICROPHONE.lock().unwrap().clear();
        Some(true)
    }
}}
mod chat {
    pub const CHAT_INPUT_CAPACITY:usize=4096;
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
    pub struct Chat {pub generation_state:GenerationState,pub input:Vec<u8>}
    impl Chat {
        // ------------------------=
        // FUNC: selected_model
        // DESC: Selects the fixture native model.
        // ------------------=
        pub fn selected_model(&self)->u32{crate::SELECTED_MODEL.load(crate::Ordering::SeqCst) as u32}
        // ------------------------=
        // FUNC: input
        // DESC: Exposes whether recognized text remains queued in the composer.
        // ------------------=
        pub fn input(&self)->&[u8]{&self.input}
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
        pub fn push_input(&mut self,byte:u8)->bool{
            if !(b' '..=b'~').contains(&byte) || self.input.len()>=CHAT_INPUT_CAPACITY
                || self.input.len()>=crate::INPUT_FAIL_AT.load(crate::Ordering::SeqCst){return false;}
            self.input.push(byte);crate::COMPOSER.lock().unwrap().push(byte);
            crate::INPUT_LENGTH.fetch_add(1,crate::Ordering::SeqCst);true
        }
        // ------------------------=
        // FUNC: pop_input
        // DESC: Mirrors an atomic rollback of the fixture's own composer insertion.
        // ------------------=
        pub fn pop_input(&mut self)->bool{
            if self.input.pop().is_none(){return false;}
            crate::COMPOSER.lock().unwrap().pop();crate::INPUT_LENGTH.fetch_sub(1,crate::Ordering::SeqCst);true
        }
        // ------------------------=
        // FUNC: set_input_cursor
        // DESC: Models the fixture's end-only composer caret used when clearing an unchanged owned prompt.
        // ------------------=
        pub fn set_input_cursor(&mut self,index:usize){assert_eq!(index,self.input.len());}
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
    // FUNC: app_turn_owner
    // DESC: Exposes a separate background application's exclusive inference lease to the production voice controller.
    // ------------------=
    fn app_turn_owner(&self)->Option<usize>{APP_GENERATING.load(Ordering::SeqCst).then_some(2)}
    // ------------------------=
    // FUNC: native_ready
    // DESC: Supplies a selectable model-loading state without affecting microphone activation.
    // ------------------=
    fn native_ready(&self,_:usize)->bool{MODEL_READY.load(Ordering::SeqCst)}
    // ------------------------=
    // FUNC: bind_chat_owner
    // DESC: Rejects session IDs at the conversation boundary; the real session and user identities deliberately differ.
    // ------------------=
    fn bind_chat_owner(&mut self,user:[u8;16]){assert_eq!(user,[2;16]);}
    // ------------------------=
    // FUNC: cancel_chat
    // DESC: Models observable cancellation of background work as well as active conversational inference.
    // ------------------=
    fn cancel_chat(&mut self){
        CANCEL_CALLS.fetch_add(1,Ordering::SeqCst);
        BACKGROUND_GOAL.store(false,Ordering::SeqCst);
        APP_GENERATING.store(false,Ordering::SeqCst);
        STREAMING.store(false,Ordering::SeqCst);
    }
    // ------------------------=
    // FUNC: submit_chat
    // DESC: Rejects unused generation in lifecycle-only tests.
    // ------------------=
    fn submit_chat(&mut self)->bool{
        if !MODEL_READY.load(Ordering::SeqCst) || !CHAT_READY.load(Ordering::SeqCst) {return false;}
        PROMPTS.lock().unwrap().push(std::mem::take(&mut *COMPOSER.lock().unwrap()));
        GENERATION_FAILURE.store(0,Ordering::SeqCst);
        TURNS.fetch_add(1,Ordering::SeqCst);INPUT_LENGTH.store(0,Ordering::SeqCst);true
    }
    // ------------------------=
    // FUNC: submit_voice_chat
    // DESC: Mirrors the production voice-origin submission seam while retaining deterministic fixture accounting.
    // ------------------=
    fn submit_voice_chat(&mut self)->bool{
        if APP_GENERATING.load(Ordering::SeqCst) {self.cancel_chat();}
        self.submit_chat()
    }
}
// ------------------------=
// FUNC: with_ai_runtime
// DESC: Exposes an idle local model to the production controller.
// ------------------=
fn with_ai_runtime<T>(f:impl FnOnce(&mut Ai)->T)->T{
    let input=COMPOSER.lock().unwrap().clone();
    let generation_state=match GENERATION_FAILURE.load(Ordering::SeqCst) {
        1=>chat::GenerationState::Failed, 2=>chat::GenerationState::Cancelled,
        3=>chat::GenerationState::ContextFull,
        _=>if STREAMING.load(Ordering::SeqCst){chat::GenerationState::Running}else{chat::GenerationState::Complete},
    };
    f(&mut Ai{chat:chat::Chat{input,generation_state}})
}
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
    pub fn prepare()->bool{
        crate::PREPARE_CALLS.fetch_add(1,crate::Ordering::SeqCst);
        crate::MODEL_READY.load(crate::Ordering::SeqCst)
    }
    // ------------------------=
    // FUNC: prepared
    // DESC: Exposes the fixture's explicit recognizer readiness state.
    // ------------------=
    pub fn prepared()->bool{crate::MODEL_READY.load(crate::Ordering::SeqCst)}
    // ------------------------=
    // FUNC: prepare_failed
    // DESC: Exposes the terminal recognizer warmup failure without retrying or advertising microphone readiness.
    // ------------------=
    pub fn prepare_failed()->bool{crate::PREPARE_FAILED.load(crate::Ordering::SeqCst)}
    // ------------------------=
    // FUNC: invalidate
    // DESC: Leaves recognition lifecycle control with the fixture's test transitions.
    // ------------------=
    pub fn invalidate(){}
    // ------------------------=
    // FUNC: status
    // DESC: Models asynchronous cancellation drain without running a recognizer.
    // ------------------=
    pub fn status()->Status{
        let error=crate::RECOGNITION_ERROR.load(crate::Ordering::SeqCst) as i32;
        Status{state:if error!=0{InputState::Failed}else if crate::READY.load(crate::Ordering::SeqCst){InputState::Ready}else if crate::BUSY.load(crate::Ordering::SeqCst){InputState::Recognizing}else{InputState::Cancelled},error}
    }
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
        crate::RECOGNITION_REQUESTS.lock().unwrap().push(pcm.to_vec());
        let held=crate::HOLD_RECOGNITION.load(crate::Ordering::SeqCst);
        crate::BUSY.store(held,crate::Ordering::SeqCst);
        crate::READY.store(!held,crate::Ordering::SeqCst);Ok(())
    }
    // ------------------------=
    // FUNC: take
    // DESC: Returns the controlled recognizer result to exercise production wake admission and prompt stripping.
    // ------------------=
    pub fn take(_:SecurityIdentity,out:&mut[u8])->Result<usize,()>{
        crate::READY.store(false,crate::Ordering::SeqCst);
        crate::BUSY.store(false,crate::Ordering::SeqCst);
        let selected = crate::TEST_TRANSCRIPT.lock().unwrap();
        let transcript: &[u8] = selected.as_deref().unwrap_or(b"Infinity test");
        out[..transcript.len()].copy_from_slice(transcript);
        Ok(transcript.len())
    }
}
mod voice_output {
    // ------------------------=
    // FUNC: echo_reference
    // DESC: Supplies either headphone isolation or a deterministic real PCM echo reference for background narration tests.
    // ------------------=
    pub fn echo_reference(_:crate::runtime::execution::SecurityIdentity,out:&mut[i16])->bool{
        let enabled=crate::ECHO_PLAYBACK.load(crate::Ordering::SeqCst);
        out.fill(if enabled {2100} else {0});enabled
    }
    // ------------------------=
    // FUNC: can_prefetch
    // DESC: Admits one lookahead phrase while a nonfinal phrase plays.
    // ------------------=
    pub fn can_prefetch()->bool{crate::OUTPUT.load(crate::Ordering::SeqCst)==4
        || (crate::OUTPUT.load(crate::Ordering::SeqCst)==1
            && crate::SPANS.lock().unwrap().last().is_some_and(|span| !span.2))}
    pub const OUTPUT_LEASE_SECONDS: u64 = 130;
    use crate::runtime::execution::SecurityIdentity;
    #[derive(PartialEq)]
    pub enum OutputState {Idle,Queued,Synthesizing,Ready,Speaking,Complete,Failed,Cancelled,Buffered}
    pub struct Status {pub state:OutputState,pub first_playback_ns:u64}
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
    pub fn status()->Status{Status{state:match crate::OUTPUT.load(crate::Ordering::SeqCst){1=>OutputState::Speaking,2=>OutputState::Complete,3=>OutputState::Synthesizing,4=>OutputState::Buffered,5=>OutputState::Failed,_=>OutputState::Cancelled},first_playback_ns:crate::FIRST_PLAYBACK_NS.load(crate::Ordering::SeqCst) as u64}}
    // ------------------------=
    // FUNC: stop
    // DESC: Supplies the audio-output cancellation seam.
    // ------------------=
    pub fn stop(_:SecurityIdentity){
        crate::OUTPUT_STOPS.fetch_add(1,crate::Ordering::SeqCst);
        if !crate::HOLD_OUTPUT_STOP.load(crate::Ordering::SeqCst){crate::OUTPUT.store(0,crate::Ordering::SeqCst);}
    }
    // ------------------------=
    // FUNC: request_id
    // DESC: Exposes the accepted fixture job independently of its shared session identity.
    // ------------------=
    pub fn request_id(owner:SecurityIdentity)->Option<u64>{
        let request=crate::OUTPUT_REQUEST.load(crate::Ordering::SeqCst) as u64;
        (owner.0==[1;16] && request!=0).then_some(request)
    }
    // ------------------------=
    // FUNC: stop_request
    // DESC: Counts a stop side effect only for an exact current request receipt.
    // ------------------=
    pub fn stop_request(owner:SecurityIdentity,request:u64)->bool{
        if request!=0 && request_id(owner)==Some(request) {stop(owner);true} else {false}
    }
    // ------------------------=
    // FUNC: submit
    // DESC: Rejects unused synthesis jobs during lifecycle testing.
    // ------------------=
    pub fn submit(_:SecurityIdentity,_:u64,text:&[u8])->Result<(),()>{
        if !crate::FLOW.load(crate::Ordering::SeqCst){return Err(());}
        if crate::OUTPUT_SUBMIT_FAILURE.load(crate::Ordering::SeqCst) {
            crate::OUTPUT_SUBMIT_REJECTIONS.fetch_add(1,crate::Ordering::SeqCst);
            return Err(());
        }
        if crate::OUTPUT.load(crate::Ordering::SeqCst)==3
            || (crate::OUTPUT.load(crate::Ordering::SeqCst)==1 && !can_prefetch()){return Err(());}
        assert!(text.len()<=160);
        crate::PHRASES.lock().unwrap().push(text.to_vec());
        crate::OUTPUT_REQUEST.fetch_add(1,crate::Ordering::SeqCst);
        crate::OUTPUT.store(3,crate::Ordering::SeqCst);Ok(())
    }
    // ------------------------=
    // FUNC: submit_progressive_span
    // DESC: Records a generation-safe content span through the conversation fixture.
    // ------------------=
    pub fn submit_progressive_span(owner:SecurityIdentity,cap:u64,text:&[u8],start:usize,end:usize,final_span:bool)->Result<(),()>{
        assert_eq!(text.len(),end-start);
        if start==0 {crate::FIRST_PLAYBACK_NS.store(0,crate::Ordering::SeqCst);}
        submit(owner,cap,text)?;
        crate::SPANS.lock().unwrap().push((start,end,final_span));
        Ok(())
    }
    // ------------------------=
    // FUNC: seal_buffered
    // DESC: Starts deterministic playback after every response span is prepared.
    // ------------------=
    pub fn seal_buffered(_:SecurityIdentity)->bool{crate::FIRST_PLAYBACK_NS.store(crate::NOW.load(crate::Ordering::SeqCst),crate::Ordering::SeqCst);crate::OUTPUT.store(1,crate::Ordering::SeqCst);true}
}
#[path = "../kernel/runtime/ai/voice_pcm.rs"] mod voice_pcm;
#[path = "../kernel/runtime/ai/voice_vad.rs"] mod voice_vad;
#[path = "../kernel/ui/thinking.rs"] mod thinking;
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
    assert!(conversation::toggle(owner));assert_eq!(conversation::state().0,State::Starting);
    assert!(conversation::preparing());assert_eq!(CAPTURES.load(Ordering::SeqCst),0);
    MODEL_READY.store(true,Ordering::SeqCst);
    CHAT_READY.store(true,Ordering::SeqCst);
    conversation::poll();assert_eq!(conversation::state().0,State::Listening);
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
    MICROPHONE.lock().unwrap().extend(vec![0;voice_vad::END_SILENCE+4096]);
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
    assert!(!conversation::wake_armed());
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
    MICROPHONE.lock().unwrap().extend(vec![0;voice_vad::END_SILENCE+4096]);
    for _ in 0..6 {conversation::poll();if conversation::state().0==State::Recognizing {break;}}
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
    MICROPHONE.lock().unwrap().extend(vec![0;voice_vad::END_SILENCE+4096]);
    let input_grants=INPUT_GRANTS.load(Ordering::SeqCst);
    for _ in 0..6 {conversation::poll();if conversation::state().0==State::Recognizing {break;}}
    assert_eq!(conversation::state().0,State::Recognizing);
    assert_eq!(INPUT_GRANTS.load(Ordering::SeqCst),input_grants+1);
    assert_eq!(PHRASES.lock().unwrap().len(),phrase_baseline);
    assert!(RECOGNIZED.lock().unwrap().iter().any(|&v|v==1700));
    conversation::poll();conversation::poll();
    assert_eq!(PHRASES.lock().unwrap()[phrase_baseline],b"New response.");
    // A second reply advances without waiting for microphone input or a gap timer.
    conversation::stop(owner);conversation::poll();TURNS.store(0,Ordering::SeqCst);
    assert!(conversation::start(owner));
    MICROPHONE.lock().unwrap().extend([1000;1600]);MICROPHONE.lock().unwrap().extend(vec![0;voice_vad::END_SILENCE+4096]);
    for _ in 0..10 {conversation::poll();}
    OUTPUT.store(2,Ordering::SeqCst);conversation::poll();
    let phrases=PHRASES.lock().unwrap().len();
    NOW.store(3_000_000_002,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Listening);
    assert_eq!(PHRASES.lock().unwrap().len(),phrases);
    conversation::stop(owner);
    conversation::poll();assert_eq!(conversation::state().0,State::Off);
    TURNS.store(0,Ordering::SeqCst);assert!(conversation::start(owner));
    MICROPHONE.lock().unwrap().extend([1000;1600]);MICROPHONE.lock().unwrap().extend(vec![0;voice_vad::END_SILENCE+4096]);
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
    // A slow, unpunctuated stream does not manufacture a spoken pause at the
    // old 24/80-byte cutoffs. Its hard provider bound still starts playback
    // before the complete answer, without waiting indefinitely for punctuation.
    for available in [24,32,80,159] {
        VISIBLE.store(available,Ordering::SeqCst);
        conversation::poll();assert_eq!(conversation::state().0,State::Thinking);
        assert_eq!(PHRASES.lock().unwrap().len(),before);
    }
    VISIBLE.store(160,Ordering::SeqCst);
    conversation::poll();assert_eq!(conversation::state().0,State::Speaking);
    assert_eq!(PHRASES.lock().unwrap().len(),before+1);
    let first_length=PHRASES.lock().unwrap()[before].len();
    assert_eq!(first_length,LONG_RESPONSE[..160].iter().rposition(u8::is_ascii_whitespace).unwrap()+1);
    assert_eq!(SPANS.lock().unwrap()[spans],(0,first_length,false));
    OUTPUT.store(1,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Thinking);
    assert_eq!(PHRASES.lock().unwrap().len(),before+1);
    STREAMING.store(false,Ordering::SeqCst);VISIBLE.store(usize::MAX,Ordering::SeqCst);
    conversation::poll();assert_eq!(conversation::state().0,State::Speaking);
    assert_eq!(PHRASES.lock().unwrap().len(),before+2);
    assert_eq!(PHRASES.lock().unwrap()[before..].concat(),LONG_RESPONSE);
    assert_eq!(SPANS.lock().unwrap()[spans+1],(first_length,LONG_RESPONSE.len(),true));
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
    MICROPHONE.lock().unwrap().extend([1800;1600]);MICROPHONE.lock().unwrap().extend(vec![0;voice_vad::END_SILENCE+4096]);
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
    MICROPHONE.lock().unwrap().extend([2400;1600]);MICROPHONE.lock().unwrap().extend(vec![0;voice_vad::END_SILENCE+4096]);
    for _ in 0..6 {conversation::poll();if conversation::state().0==State::Recognizing {break;}}
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

    clipped_wake_gate_tests(owner);
    duplicate_transcript_tests(owner);
    wake_gate_tests(owner);
    repeated_reply_wake_tests(owner);
    next_turn_capture_boundary_tests(owner);
    wake_handoff_tests(owner);
    transcript_handoff_tests(owner);
    turn_timing_tests(owner);
    workflow_narration_and_failure_tests(owner);
    audio_failure_workflow_boundary_tests(owner);
    terminal_workflow_speech_isolation_tests(owner);
    background_generation_listening_tests(owner);
    calibrated_instruction_tail_tests(owner);
    recognition_failure_recovery_tests(owner);
}

// ------------------------=
// FUNC: calibrated_instruction_tail_tests
// DESC: Passes a mid-instruction pause and quieter suffix through the production calibrated capture path as exactly one complete recognizer request.
// ------------------=
fn calibrated_instruction_tail_tests(owner:runtime::execution::SecurityIdentity) {
    use conversation::State;
    assert!(conversation::start(owner));
    let before=RECOGNITION_REQUESTS.lock().unwrap().len();
    HOLD_RECOGNITION.store(true,Ordering::SeqCst);
    MICROPHONE.lock().unwrap().extend([1000;1600]);
    MICROPHONE.lock().unwrap().extend([0;8000]);
    MICROPHONE.lock().unwrap().extend([220;24000]);
    MICROPHONE.lock().unwrap().extend(vec![0;voice_vad::END_SILENCE]);
    for _ in 0..20 {
        conversation::poll();
        if conversation::state().0==State::Recognizing {break;}
    }
    assert_eq!(conversation::state().0,State::Recognizing);
    assert_eq!(RECOGNITION_REQUESTS.lock().unwrap().len(),before+1);
    let pcm=RECOGNIZED.lock().unwrap();
    assert_eq!(pcm.iter().filter(|&&sample|sample==1000).count(),1600);
    assert_eq!(pcm.iter().filter(|&&sample|sample==220).count(),24000);
    drop(pcm);
    conversation::stop(owner);
    HOLD_RECOGNITION.store(false,Ordering::SeqCst);
    BUSY.store(false,Ordering::SeqCst);
    conversation::poll();
    assert_eq!(conversation::state().0,State::Off);
}

// ------------------------=
// FUNC: background_generation_listening_tests
// DESC: Exercises microphone handoff behind background narration, retained output ownership, wake-gated interruption and subsequent retry.
// ------------------=
fn background_generation_listening_tests(owner:runtime::execution::SecurityIdentity) {
    use conversation::State;
    for output_state in [1,3,4] {
        assert!(conversation::start(owner));
        recognize_voice(b"Infinity create an application");
        assert_eq!(conversation::state().0,State::Thinking);
        APP_GENERATING.store(true,Ordering::SeqCst);
        BACKGROUND_GOAL.store(true,Ordering::SeqCst);
        OUTPUT_REQUEST.fetch_add(1,Ordering::SeqCst);
        OUTPUT.store(output_state,Ordering::SeqCst);
        let request=voice_output::request_id(owner);
        let stops=OUTPUT_STOPS.load(Ordering::SeqCst);
        let cancels=CANCEL_CALLS.load(Ordering::SeqCst);
        let turns=TURNS.load(Ordering::SeqCst);
        let captures=CAPTURES.load(Ordering::SeqCst);
        conversation::poll();
        assert_eq!(conversation::state().0,State::Listening);
        assert_eq!(voice_output::request_id(owner),request);
        assert_eq!(OUTPUT.load(Ordering::SeqCst),output_state);
        assert_eq!(OUTPUT_STOPS.load(Ordering::SeqCst),stops);
        assert_eq!(CANCEL_CALLS.load(Ordering::SeqCst),cancels);
        assert_eq!(CAPTURES.load(Ordering::SeqCst),captures);
        assert!(APP_GENERATING.load(Ordering::SeqCst));
        assert!(BACKGROUND_GOAL.load(Ordering::SeqCst));
        if output_state==1 {
            let recognized=RECOGNITION_REQUESTS.lock().unwrap().len();
            ECHO_PLAYBACK.store(true,Ordering::SeqCst);
            MICROPHONE.lock().unwrap().extend([2100;4800]);
            conversation::poll();
            assert_eq!(conversation::state().0,State::Listening);
            assert_eq!(RECOGNITION_REQUESTS.lock().unwrap().len(),recognized);
            assert_eq!(TURNS.load(Ordering::SeqCst),turns);
            assert_eq!(OUTPUT_STOPS.load(Ordering::SeqCst),stops);
            ECHO_PLAYBACK.store(false,Ordering::SeqCst);
            NOW.fetch_add(250_000_000,Ordering::SeqCst);
        }
        recognize_voice(b"unaddressed room conversation");
        assert_eq!(TURNS.load(Ordering::SeqCst),turns);
        assert_eq!(OUTPUT_STOPS.load(Ordering::SeqCst),stops);
        recognize_voice(b"Infinity stop the application generation");
        assert_eq!(TURNS.load(Ordering::SeqCst),turns+1);
        assert_eq!(OUTPUT_STOPS.load(Ordering::SeqCst),stops+1);
        assert_eq!(PROMPTS.lock().unwrap().last().unwrap(),b"stop the application generation");
        // The real runtime's shared voice ingress cancels the previous app
        // lease before admitting a new request; mirror that observable seam.
        assert!(!APP_GENERATING.load(Ordering::SeqCst));
        assert!(!BACKGROUND_GOAL.load(Ordering::SeqCst));
        GENERATION_FAILURE.store(1,Ordering::SeqCst);
        conversation::poll();conversation::poll();
        assert_eq!(conversation::state().0,State::Listening);
        recognize_voice(b"Infinity try another request");
        assert_eq!(TURNS.load(Ordering::SeqCst),turns+2);
        complete_voice_reply();
        conversation::stop(owner);conversation::poll();
    }
    // An ordinary in-progress conversational response must not yield merely
    // because another producer has a speech receipt: it still owns inference.
    assert!(conversation::start(owner));
    recognize_voice(b"Infinity explain the current task");
    STREAMING.store(true,Ordering::SeqCst);
    OUTPUT_REQUEST.fetch_add(1,Ordering::SeqCst);OUTPUT.store(3,Ordering::SeqCst);
    conversation::poll();
    assert_eq!(conversation::state().0,State::Thinking);
    conversation::stop(owner);conversation::poll();
}

// ------------------------=
// FUNC: terminal_workflow_speech_isolation_tests
// DESC: Replays terminal chat admission and stale conversational recovery after a newer workflow job is accepted, preserving audio and microphone reuse.
// ------------------=
fn terminal_workflow_speech_isolation_tests(owner:runtime::execution::SecurityIdentity) {
    use conversation::State;
    for failure in 1..=3 {
        assert!(conversation::start(owner));
        GENERATION_FAILURE.store(failure,Ordering::SeqCst);
        conversation::speak_visible_reply(owner,1);
        assert_eq!(conversation::state().0,State::Listening);
        voice_output::submit(owner,1,b"Workflow ended.").unwrap();
        let stops=OUTPUT_STOPS.load(Ordering::SeqCst);
        conversation::poll();
        assert_eq!(conversation::state().0,State::Listening);
        assert_eq!(OUTPUT.load(Ordering::SeqCst),3);
        assert_eq!(OUTPUT_STOPS.load(Ordering::SeqCst),stops);
        // Explicit user stop remains session-wide even for foreign-producer speech.
        conversation::stop(owner);conversation::poll();
        assert_eq!(OUTPUT.load(Ordering::SeqCst),0);
        assert_eq!(OUTPUT_STOPS.load(Ordering::SeqCst),stops+1);
        assert_eq!(conversation::state().0,State::Off);
        GENERATION_FAILURE.store(0,Ordering::SeqCst);
    }
    for had_reply in [false,true] {
        assert!(conversation::start(owner));
        recognize_voice(b"Infinity begin an application");
        if had_reply {
            conversation::poll();assert_eq!(conversation::state().0,State::Speaking);
            OUTPUT.store(2,Ordering::SeqCst);
        }
        voice_output::submit(owner,1,b"Generation cancelled.").unwrap();
        let request=voice_output::request_id(owner);
        let stops=OUTPUT_STOPS.load(Ordering::SeqCst);
        let cancelled=CANCEL_CALLS.load(Ordering::SeqCst);
        GENERATION_FAILURE.store(1,Ordering::SeqCst);
        conversation::poll();conversation::poll();
        assert_eq!(conversation::state().0,State::Listening);
        assert_eq!(OUTPUT.load(Ordering::SeqCst),3);
        assert_eq!(voice_output::request_id(owner),request);
        assert_eq!(OUTPUT_STOPS.load(Ordering::SeqCst),stops);
        assert_eq!(CANCEL_CALLS.load(Ordering::SeqCst),cancelled);
        OUTPUT.store(2,Ordering::SeqCst);conversation::poll();
        assert_eq!(conversation::state().0,State::Listening);
        conversation::stop(owner);conversation::poll();
        GENERATION_FAILURE.store(0,Ordering::SeqCst);
    }
}

// ------------------------=
// FUNC: audio_failure_workflow_boundary_tests
// DESC: Keeps completed acknowledgment audio failures from cancelling background work while explicit stop and running-reply recovery still release their owned work.
// ------------------=
fn audio_failure_workflow_boundary_tests(owner:runtime::execution::SecurityIdentity) {
    use conversation::State;
    for failure in [0,5,6] {
        assert!(conversation::start(owner));
        recognize_voice(b"Infinity create an application");
        BACKGROUND_GOAL.store(true,Ordering::SeqCst);
        APP_GENERATING.store(true,Ordering::SeqCst);
        let cancelled=CANCEL_CALLS.load(Ordering::SeqCst);
        let rejected=OUTPUT_SUBMIT_REJECTIONS.load(Ordering::SeqCst);
        if failure==6 {
            OUTPUT_SUBMIT_FAILURE.store(true,Ordering::SeqCst);
        } else {
            conversation::poll();assert_eq!(conversation::state().0,State::Speaking);
            OUTPUT.store(failure,Ordering::SeqCst);
        }
        conversation::poll();conversation::poll();
        OUTPUT_SUBMIT_FAILURE.store(false,Ordering::SeqCst);
        assert_eq!(conversation::state().0,State::Listening);
        assert_eq!(CANCEL_CALLS.load(Ordering::SeqCst),cancelled);
        assert_eq!(OUTPUT_SUBMIT_REJECTIONS.load(Ordering::SeqCst),rejected+usize::from(failure==6));
        assert!(BACKGROUND_GOAL.load(Ordering::SeqCst));
        assert!(APP_GENERATING.load(Ordering::SeqCst));
        assert!(!conversation::wake_armed());
        let turns=TURNS.load(Ordering::SeqCst);
        recognize_voice(b"unaddressed follow-up");
        assert_eq!(TURNS.load(Ordering::SeqCst),turns);
        conversation::stop(owner);conversation::poll();
        BACKGROUND_GOAL.store(false,Ordering::SeqCst);APP_GENERATING.store(false,Ordering::SeqCst);
    }
    for speaking in [false,true] {
        assert!(conversation::start(owner));
        recognize_voice(b"Infinity begin a workflow");
        if speaking {conversation::poll();assert_eq!(conversation::state().0,State::Speaking);}
        BACKGROUND_GOAL.store(true,Ordering::SeqCst);APP_GENERATING.store(true,Ordering::SeqCst);
        let cancelled=CANCEL_CALLS.load(Ordering::SeqCst);
        conversation::stop(owner);conversation::poll();
        assert_eq!(conversation::state().0,State::Off);
        assert_eq!(CANCEL_CALLS.load(Ordering::SeqCst),cancelled+1);
        assert!(!BACKGROUND_GOAL.load(Ordering::SeqCst));assert!(!APP_GENERATING.load(Ordering::SeqCst));
    }
    assert!(conversation::start(owner));
    recognize_voice(b"Infinity describe the current task");
    STREAMING.store(true,Ordering::SeqCst);LONG_REPLY.store(true,Ordering::SeqCst);
    OUTPUT_SUBMIT_FAILURE.store(true,Ordering::SeqCst);
    let cancelled=CANCEL_CALLS.load(Ordering::SeqCst);
    let rejected=OUTPUT_SUBMIT_REJECTIONS.load(Ordering::SeqCst);
    conversation::poll();conversation::poll();
    assert_eq!(conversation::state().0,State::Listening);
    assert_eq!(CANCEL_CALLS.load(Ordering::SeqCst),cancelled+1);
    assert_eq!(OUTPUT_SUBMIT_REJECTIONS.load(Ordering::SeqCst),rejected+1);
    assert!(!STREAMING.load(Ordering::SeqCst));
    OUTPUT_SUBMIT_FAILURE.store(false,Ordering::SeqCst);LONG_REPLY.store(false,Ordering::SeqCst);
    conversation::stop(owner);conversation::poll();
}

// ------------------------=
// FUNC: workflow_narration_and_failure_tests
// DESC: Exercises speech admission contention, addressed narration interruption, and safe microphone recovery after unsuccessful model turns.
// ------------------=
fn workflow_narration_and_failure_tests(owner: runtime::execution::SecurityIdentity) {
    use conversation::State;
    HOLD_RECOGNITION.store(false,Ordering::SeqCst);
    assert!(conversation::start(owner));
    let turns=TURNS.load(Ordering::SeqCst);
    OUTPUT.store(3,Ordering::SeqCst);
    HOLD_OUTPUT_STOP.store(true,Ordering::SeqCst);
    recognize_voice(b"unaddressed background words");
    assert_eq!(TURNS.load(Ordering::SeqCst),turns);
    assert_eq!(conversation::state().0,State::Listening);
    recognize_voice(b"Infinity cancel that generation");
    assert_eq!(TURNS.load(Ordering::SeqCst),turns+1);
    let captures=CAPTURES.load(Ordering::SeqCst);
    let phrases=PHRASES.lock().unwrap().len();
    for _ in 0..8 {
        conversation::poll();
        assert_eq!(conversation::state().0,State::Thinking);
        assert_eq!(PHRASES.lock().unwrap().len(),phrases);
        assert_eq!(CAPTURES.load(Ordering::SeqCst),captures);
    }
    HOLD_OUTPUT_STOP.store(false,Ordering::SeqCst);
    OUTPUT.store(0,Ordering::SeqCst);
    complete_voice_reply();
    conversation::stop(owner);conversation::poll();

    for failure in 1..=3 {
        assert!(conversation::start(owner));
        recognize_voice(b"Infinity create an application");
        GENERATION_FAILURE.store(failure,Ordering::SeqCst);
        conversation::poll();conversation::poll();
        assert_eq!(conversation::state().0,State::Listening);
        assert!(!conversation::wake_armed());
        let turns=TURNS.load(Ordering::SeqCst);
        recognize_voice(b"unaddressed follow-up");
        assert_eq!(TURNS.load(Ordering::SeqCst),turns);
        recognize_voice(b"Infinity open settings");
        assert_eq!(TURNS.load(Ordering::SeqCst),turns+1);
        complete_voice_reply();
        conversation::stop(owner);conversation::poll();
    }
    for terminal_output in [0,5] {
        assert!(conversation::start(owner));
        recognize_voice(b"Infinity report progress");
        conversation::poll();
        assert_eq!(conversation::state().0,State::Speaking);
        OUTPUT.store(terminal_output,Ordering::SeqCst);
        conversation::poll();conversation::poll();
        assert_eq!(conversation::state().0,State::Listening);
        conversation::stop(owner);conversation::poll();
    }
    // Explicit mute and lock remain terminal even when a turn has failed.
    for revoke in [false,true] {
        assert!(conversation::start(owner));
        recognize_voice(b"Infinity begin a workflow");
        GENERATION_FAILURE.store(1,Ordering::SeqCst);
        if revoke {ACTIVE.store(false,Ordering::SeqCst);} else {conversation::stop(owner);}
        conversation::poll();conversation::poll();
        assert_eq!(conversation::state().0,State::Off);
        ACTIVE.store(true,Ordering::SeqCst);
        GENERATION_FAILURE.store(0,Ordering::SeqCst);
    }
}

// ------------------------=
// FUNC: duplicate_transcript_tests
// DESC: Exercises repeated wake prefixes and repeated recognizer command copies through the production voice admission path.
// ------------------=
fn duplicate_transcript_tests(owner: runtime::execution::SecurityIdentity) {
    PROMPTS.lock().unwrap().clear();
    assert!(conversation::start(owner));
    for transcript in [
        b"Infinity Infinity open settings".as_slice(),
        b"Infinity open settings Infinity open settings".as_slice(),
        b"Infinity open settings Infinity open settings Infinity open settings".as_slice(),
        b"Infinity open settings open settings open settings".as_slice(),
    ] {
        recognize_voice(transcript);
        assert_eq!(PROMPTS.lock().unwrap().last().unwrap(), b"open settings");
        complete_voice_reply();
    }
    recognize_voice(b"Infinity explain the Infinity operating system");
    assert_eq!(
        PROMPTS.lock().unwrap().last().unwrap(),
        b"explain the Infinity operating system"
    );
    complete_voice_reply();
    conversation::stop(owner);
    conversation::poll();
}

// ------------------------=
// FUNC: next_turn_capture_boundary_tests
// DESC: Rejects buffered speaker-era audio while admitting immediate distinct wake commands after three replies without waits or capture restarts.
// ------------------=
fn next_turn_capture_boundary_tests(owner: runtime::execution::SecurityIdentity) {
    use conversation::State;
    assert!(conversation::start(owner));
    recognize_voice(b"Infinity begin");
    let captures = CAPTURES.load(Ordering::SeqCst);
    for (pending, command) in [(4801, b"Infinity open browser".as_slice()),
        (32000, b"Infinity what time is it"), (192000, b"Infinity open settings")] {
        assert_eq!(conversation::state().0, State::Thinking);
        conversation::poll();
        assert_eq!(conversation::state().0, State::Speaking);
        MICROPHONE.lock().unwrap().extend(std::iter::repeat_n(1700, pending));
        let now = NOW.load(Ordering::SeqCst);
        OUTPUT.store(2, Ordering::SeqCst);
        conversation::poll();
        assert_eq!(conversation::state().0, State::Listening);
        assert!(MICROPHONE.lock().unwrap().is_empty());
        assert_eq!(NOW.load(Ordering::SeqCst), now);
        assert_eq!(CAPTURES.load(Ordering::SeqCst), captures);
        assert!(!conversation::wake_armed());
        let requests = RECOGNITION_REQUESTS.lock().unwrap().len();
        let turns = TURNS.load(Ordering::SeqCst);
        recognize_voice(command);
        assert_eq!(TURNS.load(Ordering::SeqCst), turns + 1);
        let input = RECOGNITION_REQUESTS.lock().unwrap();
        assert_eq!(input.len(), requests + 1);
        assert!(!input.last().unwrap().contains(&1700));
    }
    // A transient adapter lock must neither reopen capture nor advertise
    // Listening until the stale boundary is actually discarded.
    conversation::poll();
    assert_eq!(conversation::state().0, State::Speaking);
    MICROPHONE.lock().unwrap().extend([1700; 9600]);
    let requests = RECOGNITION_REQUESTS.lock().unwrap().len();
    OUTPUT.store(2, Ordering::SeqCst);
    DISCARD_BUSY.store(true, Ordering::SeqCst);
    conversation::poll();
    assert_eq!(conversation::state().0, State::Speaking);
    assert_eq!(RECOGNITION_REQUESTS.lock().unwrap().len(), requests);
    DISCARD_BUSY.store(false, Ordering::SeqCst);
    let now = NOW.load(Ordering::SeqCst);
    conversation::poll();
    assert_eq!(conversation::state().0, State::Listening);
    assert!(MICROPHONE.lock().unwrap().is_empty());
    assert_eq!(NOW.load(Ordering::SeqCst), now);
    assert_eq!(CAPTURES.load(Ordering::SeqCst), captures);
    conversation::stop(owner);
    conversation::poll();
}

// ------------------------=
// FUNC: repeated_reply_wake_tests
// DESC: Repeats short and progressive multi-span replies, checking fresh wake admission and a stable capture lease after each drain.
// ------------------=
fn repeated_reply_wake_tests(owner: runtime::execution::SecurityIdentity) {
    use conversation::State;
    assert!(conversation::start(owner));
    let captures = CAPTURES.load(Ordering::SeqCst);
    for word in [0, 1] {
        WAKE_SETTING.store(word, Ordering::SeqCst);
        for (index, command) in [b"hello".as_slice(), b"focus text editor", b"close browser", b"open app tray"].iter().enumerate() {
            let turns = TURNS.load(Ordering::SeqCst);
            recognize_voice(b"ambient conversation");
            assert_eq!(TURNS.load(Ordering::SeqCst), turns);
            recognize_voice(if word == 0 { b"Infinity" } else { b"Computer" });
            assert!(conversation::wake_armed());
            recognize_voice(command);
            assert_eq!(TURNS.load(Ordering::SeqCst), turns + 1);
            assert_eq!(PROMPTS.lock().unwrap().last().unwrap(), command);
            LONG_REPLY.store(index % 2 == 1, Ordering::SeqCst);
            conversation::poll();
            for _ in 0..16 {
                assert_eq!(conversation::state().0, State::Speaking);
                if OUTPUT.load(Ordering::SeqCst) == 3 {
                    OUTPUT.store(1, Ordering::SeqCst);
                }
                conversation::poll();
                if OUTPUT.load(Ordering::SeqCst) == 1 {
                    assert!(!conversation::wake_armed());
                    assert_eq!(CAPTURES.load(Ordering::SeqCst), captures);
                    OUTPUT.store(2, Ordering::SeqCst);
                    conversation::poll();
                    if conversation::state().0 == State::Listening { break; }
                }
            }
            assert_eq!(conversation::state().0, State::Listening);
            assert!(!conversation::wake_armed());
            assert_eq!(CAPTURES.load(Ordering::SeqCst), captures);
            recognize_voice(b"unaddressed follow-up");
            assert_eq!(TURNS.load(Ordering::SeqCst), turns + 1);
        }
    }
    LONG_REPLY.store(false, Ordering::SeqCst);
    WAKE_SETTING.store(0, Ordering::SeqCst);
    conversation::stop(owner);
    conversation::poll();
}

// ------------------------=
// FUNC: recognition_failure_recovery_tests
// DESC: Keeps failed utterances on the same capture lease and prevents automatic reopening after fatal errors or manual stop.
// ------------------=
fn recognition_failure_recovery_tests(owner: runtime::execution::SecurityIdentity) {
    use conversation::State;
    HOLD_RECOGNITION.store(true,Ordering::SeqCst);
    // Cold speech starts warming before the language model is available.
    let captures_before=CAPTURES.load(Ordering::SeqCst);
    let prepares=PREPARE_CALLS.load(Ordering::SeqCst);
    MODEL_READY.store(false,Ordering::SeqCst);
    CHAT_READY.store(false,Ordering::SeqCst);
    let mut deadline=10;
    assert_eq!(conversation::autostart(owner,&mut deadline,9),None);
    assert_eq!(conversation::autostart(owner,&mut deadline,10),Some(true));
    assert_eq!(deadline,u64::MAX);
    assert_eq!(PREPARE_CALLS.load(Ordering::SeqCst),prepares+1);
    assert!(conversation::preparing());
    assert_eq!(CAPTURES.load(Ordering::SeqCst),captures_before);
    assert_eq!(conversation::autostart(owner,&mut deadline,250_000_009),None);
    CHAT_READY.store(true,Ordering::SeqCst);
    conversation::poll();
    assert_eq!(conversation::state().0,State::Starting);
    MODEL_READY.store(true,Ordering::SeqCst);
    // Warm bytes alone must not remove the veil before capture is admitted.
    assert!(conversation::ready());
    assert!(conversation::preparing());
    CHAT_READY.store(false,Ordering::SeqCst);
    conversation::poll();
    assert_eq!(conversation::state().0,State::Starting);
    assert_eq!(CAPTURES.load(Ordering::SeqCst),captures_before);
    CHAT_READY.store(true,Ordering::SeqCst);
    CAPTURE_AVAILABLE.store(false,Ordering::SeqCst);
    conversation::poll();
    assert_eq!(conversation::state().0,State::Starting);
    assert!(conversation::preparing());
    assert_eq!(CAPTURES.load(Ordering::SeqCst),captures_before);
    CAPTURE_AVAILABLE.store(true,Ordering::SeqCst);
    PLAYBACK_AVAILABLE.store(false,Ordering::SeqCst);
    conversation::poll();
    assert_eq!(conversation::state().0,State::Starting);
    assert_eq!(CAPTURES.load(Ordering::SeqCst),captures_before);
    PLAYBACK_AVAILABLE.store(true,Ordering::SeqCst);
    conversation::poll();
    assert_eq!(conversation::state().0,State::Listening);
    assert!(conversation::preparing());
    // Login audio and a contended output adapter cannot be calibrated as ambient.
    for output in [1,2] {
        CALIBRATION_PLAYBACK.store(output,Ordering::SeqCst);
        MICROPHONE.lock().unwrap().extend([4000;8000]);
        drain_microphone();
        assert!(conversation::preparing());
        assert_eq!(conversation::microphone_threshold(),None);
    }
    CALIBRATION_PLAYBACK.store(0,Ordering::SeqCst);
    // A cold ADC/filter can deliver initial quiet frames before ambient PCM.
    // Those frames must not pin the gate below the sustained room noise.
    MICROPHONE.lock().unwrap().extend([0;640]);
    MICROPHONE.lock().unwrap().extend([500;7360]);
    drain_microphone();
    assert!(!conversation::preparing());
    assert_eq!(conversation::microphone_threshold(),Some(1000));
    let requests=RECOGNITION_REQUESTS.lock().unwrap().len();
    MICROPHONE.lock().unwrap().extend(std::iter::repeat_n(500,160000));
    drain_microphone();
    assert!(!conversation::waiting_for_pause());
    assert_eq!(RECOGNITION_REQUESTS.lock().unwrap().len(),requests);
    HOLD_RECOGNITION.store(true,Ordering::SeqCst);
    MICROPHONE.lock().unwrap().extend([1800;3200]);
    MICROPHONE.lock().unwrap().extend(vec![500;voice_vad::END_SILENCE]);
    drain_microphone();
    assert_eq!(conversation::state().0,State::Recognizing);
    assert_eq!(RECOGNITION_REQUESTS.lock().unwrap().len(),requests+1);
    assert!(RECOGNIZED.lock().unwrap().contains(&1800));
    finish_recognition(b"Infinity test");
    complete_voice_reply();
    assert_eq!(CAPTURES.load(Ordering::SeqCst),captures_before+1);
    let captures=CAPTURES.load(Ordering::SeqCst);
    let turns=TURNS.load(Ordering::SeqCst);
    for error in [6,5,6] {
        begin_delayed_recognition(1800);
        let grants=INPUT_GRANTS.load(Ordering::SeqCst);
        BUSY.store(false,Ordering::SeqCst);
        RECOGNITION_ERROR.store(error,Ordering::SeqCst);
        conversation::poll();
        RECOGNITION_ERROR.store(0,Ordering::SeqCst);
        assert_eq!(conversation::state().0,State::Listening);
        assert_eq!(CAPTURES.load(Ordering::SeqCst),captures);
        assert_eq!(INPUT_GRANTS.load(Ordering::SeqCst),grants);
        assert_eq!(TURNS.load(Ordering::SeqCst),turns);
    }
    begin_delayed_recognition(1800);
    finish_recognition(b"Infinity what is your name");
    complete_voice_reply();
    assert_eq!(CAPTURES.load(Ordering::SeqCst),captures);
    begin_delayed_recognition(1800);
    BUSY.store(false,Ordering::SeqCst);
    RECOGNITION_ERROR.store(3,Ordering::SeqCst);
    conversation::poll();
    RECOGNITION_ERROR.store(0,Ordering::SeqCst);
    assert_eq!(conversation::state().0,State::Failed);
    assert!(!conversation::preparing());
    assert_eq!(conversation::autostart(owner,&mut deadline,100_000_000_000),None);
    assert_eq!(CAPTURES.load(Ordering::SeqCst),captures);
    assert!(conversation::start(owner));
    conversation::stop(owner);conversation::poll();
    assert_eq!(conversation::state().0,State::Off);
    let captures=CAPTURES.load(Ordering::SeqCst);
    assert_eq!(conversation::autostart(owner,&mut deadline,200_000_000_000),None);
    assert_eq!(CAPTURES.load(Ordering::SeqCst),captures);
}

// ------------------------=
// FUNC: turn_timing_tests
// DESC: Measures actual controller milestones, rejects stale prior playback, and requires output drain before a spoken turn is complete.
// ------------------=
fn turn_timing_tests(owner: runtime::execution::SecurityIdentity) {
    use conversation::{State, timing::Milestone};
    assert!(conversation::start(owner));
    NOW.fetch_add(1_000_000_000,Ordering::SeqCst);
    FIRST_PLAYBACK_NS.store(1,Ordering::SeqCst);
    recognize_voice(b"Infinity, measure this response");
    let accepted=conversation::timing_snapshot();
    assert!(accepted.at(Milestone::Endpoint).is_some());
    assert!(accepted.at(Milestone::Transcript).is_some());
    assert!(accepted.at(Milestone::Submitted).is_some());
    assert_eq!(accepted.at(Milestone::FirstAudio),None);
    NOW.fetch_add(100_000_000,Ordering::SeqCst);
    conversation::poll();
    assert_eq!(conversation::state().0,State::Speaking);
    assert!(conversation::timing_snapshot().at(Milestone::FirstText).is_some());
    assert!(conversation::timing_snapshot().at(Milestone::SpeechQueued).is_some());
    assert!(!conversation::timing_snapshot().complete());
    NOW.fetch_add(200_000_000,Ordering::SeqCst);
    let first=NOW.load(Ordering::SeqCst) as u64;
    FIRST_PLAYBACK_NS.store(first as usize,Ordering::SeqCst);
    OUTPUT.store(1,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::timing_snapshot().at(Milestone::FirstAudio),Some(first));
    assert!(!conversation::timing_snapshot().complete());
    NOW.fetch_add(300_000_000,Ordering::SeqCst);
    OUTPUT.store(2,Ordering::SeqCst);conversation::poll();
    let drained=conversation::timing_snapshot();
    assert!(drained.complete());
    assert_eq!(drained.elapsed(Milestone::SpeechQueued,Milestone::FirstAudio),Some(200_000_000));
    assert_eq!(drained.elapsed(Milestone::FirstAudio,Milestone::Drained),Some(300_000_000));
    assert_eq!(conversation::state().0,State::Listening);
    recognize_voice(b"Infinity, another measured response");
    assert!(conversation::timing_snapshot().sequence>drained.sequence);
    assert_eq!(conversation::timing_snapshot().at(Milestone::FirstAudio),None);
    conversation::stop(owner);conversation::poll();
}

// ------------------------=
// FUNC: recognize_voice
// DESC: Drives real VAD and conversation transitions with a controlled recognized transcript.
// ------------------=
fn recognize_voice(transcript: &[u8]) {
    assert_eq!(conversation::state().0,conversation::State::Listening);
    *TEST_TRANSCRIPT.lock().unwrap()=Some(transcript.to_vec());
    MICROPHONE.lock().unwrap().extend([2100;1600]);
    MICROPHONE.lock().unwrap().extend(vec![0;voice_vad::END_SILENCE+4096]);
    for _ in 0..10 {
        conversation::poll();
        if conversation::state().0==conversation::State::Recognizing {break;}
    }
    assert_eq!(conversation::state().0,conversation::State::Recognizing);
    conversation::poll();
    *TEST_TRANSCRIPT.lock().unwrap()=None;
}

// ------------------------=
// FUNC: complete_voice_reply
// DESC: Drains the accepted spoken response and verifies that listening resumes without authorizing another command.
// ------------------=
fn complete_voice_reply() {
    assert_eq!(conversation::state().0,conversation::State::Thinking);
    conversation::poll();
    assert_eq!(conversation::state().0,conversation::State::Speaking);
    OUTPUT.store(2,Ordering::SeqCst);
    conversation::poll();
    assert_eq!(conversation::state().0,conversation::State::Listening);
    assert!(!conversation::wake_armed());
}

// ------------------------=
// FUNC: clipped_wake_gate_tests
// DESC: Exercises clipped wake-only and combined utterances through real VAD, prompt submission and one-turn authorization with no pointer events.
// ------------------=
fn clipped_wake_gate_tests(owner: runtime::execution::SecurityIdentity) {
    use conversation::State;
    TURNS.store(0,Ordering::SeqCst);
    PROMPTS.lock().unwrap().clear();
    assert!(conversation::start(owner));
    for (index, wake) in [b"Infin" as &[u8],b"finity",b"Infinity"].iter().enumerate() {
        recognize_voice(wake);
        assert!(conversation::wake_armed());
        assert_eq!(TURNS.load(Ordering::SeqCst),index*2);
        recognize_voice(b"keep every command word");
        assert_eq!(PROMPTS.lock().unwrap().last().unwrap(),b"keep every command word");
        complete_voice_reply();

        let mut addressed=b"HEY, ".to_vec();
        addressed.extend_from_slice(wake);
        addressed.extend_from_slice(b"! Preserve Infinity inside the command.");
        recognize_voice(&addressed);
        assert_eq!(PROMPTS.lock().unwrap().last().unwrap(),b"Preserve Infinity inside the command.");
        complete_voice_reply();
        recognize_voice(b"ambient speech after the answer");
        assert_eq!(conversation::state().0,State::Listening);
        assert!(!conversation::wake_armed());
        assert_eq!(TURNS.load(Ordering::SeqCst),(index+1)*2);
    }
    WAKE_SETTING.store(1,Ordering::SeqCst);conversation::poll();
    for transcript in [b"Infin, help" as &[u8],b"finity, help",b"Infinity, help"] {
        recognize_voice(transcript);
        assert_eq!(conversation::state().0,State::Listening);
        assert!(!conversation::wake_armed());
        assert_eq!(TURNS.load(Ordering::SeqCst),6);
    }
    recognize_voice(b"Computer, selected wake still works");
    assert_eq!(PROMPTS.lock().unwrap().last().unwrap(),b"selected wake still works");
    complete_voice_reply();
    conversation::stop(owner);conversation::poll();
    WAKE_SETTING.store(0,Ordering::SeqCst);
}

// ------------------------=
// FUNC: wake_gate_tests
// DESC: Verifies every voice request needs a fresh configured wake, while one-shot wake capture, model changes, toggles, queued requests and typed drafts remain safe.
// ------------------=
fn wake_gate_tests(owner: runtime::execution::SecurityIdentity) {
    use conversation::State;
    TURNS.store(0,Ordering::SeqCst);
    PROMPTS.lock().unwrap().clear();
    assert!(conversation::start(owner));
    let captures=CAPTURES.load(Ordering::SeqCst);
    for transcript in [b"hello" as &[u8],b"discuss infinity today",b"Infinityx help"] {
        recognize_voice(transcript);
        assert_eq!(conversation::state().0,State::Listening);
        assert!(!conversation::wake_armed());
        assert_eq!(TURNS.load(Ordering::SeqCst),0);
        assert_eq!(INPUT_LENGTH.load(Ordering::SeqCst),0);
        assert_eq!(CAPTURES.load(Ordering::SeqCst),captures);
    }
    recognize_voice(b"Infinity, hello");
    assert_eq!(*PROMPTS.lock().unwrap(),vec![b"hello".to_vec()]);
    complete_voice_reply();
    recognize_voice(b"this is ambient speech after a reply");
    assert_eq!(TURNS.load(Ordering::SeqCst),1);
    assert_eq!(conversation::state().0,State::Listening);
    assert_eq!(CAPTURES.load(Ordering::SeqCst),captures);

    // An explicit wake-only utterance permits exactly one following command.
    recognize_voice(b"Infinity");
    assert!(conversation::wake_armed());
    assert_eq!(TURNS.load(Ordering::SeqCst),1);
    recognize_voice(b"Infinity");
    assert!(conversation::wake_armed());
    assert_eq!(TURNS.load(Ordering::SeqCst),1);
    recognize_voice(b"help me");
    assert!(!conversation::wake_armed());
    assert_eq!(PROMPTS.lock().unwrap().last().unwrap(),b"help me");
    complete_voice_reply();
    recognize_voice(b"another unaddressed request");
    assert_eq!(TURNS.load(Ordering::SeqCst),2);
    assert_eq!(conversation::state().0,State::Listening);

    recognize_voice(b"Infinity");
    NOW.fetch_add(10_000_000_000,Ordering::SeqCst);
    conversation::poll();
    assert!(!conversation::wake_armed());
    recognize_voice(b"expired command");
    assert_eq!(TURNS.load(Ordering::SeqCst),2);
    recognize_voice(b"Infinity");
    assert!(conversation::toggle(owner));conversation::poll();
    assert_eq!(conversation::state().0,State::Off);
    assert!(conversation::toggle(owner));
    assert!(!conversation::wake_armed());
    recognize_voice(b"command after toggle");
    assert_eq!(TURNS.load(Ordering::SeqCst),2);

    recognize_voice(b"Infinity");
    SELECTED_MODEL.store(1,Ordering::SeqCst);conversation::poll();
    assert!(!conversation::wake_armed());
    recognize_voice(b"command after model switch");
    assert_eq!(TURNS.load(Ordering::SeqCst),2);
    recognize_voice(b"Infinity");
    CHAT_READY.store(false,Ordering::SeqCst);MODEL_READY.store(false,Ordering::SeqCst);
    conversation::poll();assert!(!conversation::wake_armed());
    recognize_voice(b"command during model reload");
    assert_eq!(conversation::state().0,State::Listening);
    CHAT_READY.store(true,Ordering::SeqCst);MODEL_READY.store(true,Ordering::SeqCst);
    recognize_voice(b"command after model reload");
    assert_eq!(TURNS.load(Ordering::SeqCst),2);

    // Switching the configured wake revokes an already armed old phrase.
    recognize_voice(b"Infinity");
    WAKE_SETTING.store(1,Ordering::SeqCst);conversation::poll();
    assert!(!conversation::wake_armed());
    for transcript in [b"Infinity help" as &[u8],b"my computer is fast",b"Computerized help"] {
        recognize_voice(transcript);
        assert_eq!(conversation::state().0,State::Listening);
        assert_eq!(TURNS.load(Ordering::SeqCst),2);
    }
    recognize_voice(b"Computer, help");
    assert_eq!(PROMPTS.lock().unwrap().last().unwrap(),b"help");
    complete_voice_reply();
    assert_eq!(TURNS.load(Ordering::SeqCst),3);

    // Ignored speech must not overwrite or auto-submit a typed composer.
    with_ai_runtime(|ai| for &byte in b"typed question" {assert!(ai.chat.push_input(byte));});
    recognize_voice(b"ambient words");
    assert_eq!(*COMPOSER.lock().unwrap(),b"typed question");
    assert_eq!(TURNS.load(Ordering::SeqCst),3);
    with_ai_runtime(|ai| assert!(ai.submit_chat()));
    assert_eq!(PROMPTS.lock().unwrap().last().unwrap(),b"typed question");
    conversation::speak_visible_reply(owner,1);
    complete_voice_reply();

    // A wake-addressed request queued during reload retains authorization for
    // that one request, never opening a wake-free input window afterwards.
    CHAT_READY.store(false,Ordering::SeqCst);MODEL_READY.store(false,Ordering::SeqCst);
    recognize_voice(b"Computer, queued command");
    assert_eq!(conversation::state().0,State::Submitting);
    assert_eq!(TURNS.load(Ordering::SeqCst),4);
    assert_eq!(*COMPOSER.lock().unwrap(),b"queued command");
    CHAT_READY.store(true,Ordering::SeqCst);MODEL_READY.store(true,Ordering::SeqCst);
    conversation::poll();
    assert_eq!(TURNS.load(Ordering::SeqCst),5);
    complete_voice_reply();
    recognize_voice(b"still ambient");
    assert_eq!(TURNS.load(Ordering::SeqCst),5);
    assert_eq!(conversation::state().0,State::Listening);
    conversation::stop(owner);conversation::poll();
    WAKE_SETTING.store(0,Ordering::SeqCst);SELECTED_MODEL.store(0,Ordering::SeqCst);
}

// ------------------------=
// FUNC: microphone_phrase
// DESC: Creates deterministic speech plus exactly the production endpoint silence in mono samples.
// ------------------=
fn microphone_phrase(amplitude: i16) -> Vec<i16> {
    let mut pcm=vec![amplitude;1600];
    pcm.extend_from_slice(&[0;voice_vad::END_SILENCE]);
    pcm
}
// ------------------------=
// FUNC: drain_microphone
// DESC: Services finite capture chunks while the asynchronous recognizer intentionally remains pending.
// ------------------=
fn drain_microphone() {
    for _ in 0..100 {
        if MICROPHONE.lock().unwrap().is_empty() {return;}
        conversation::poll();
    }
    panic!("bounded microphone fixture did not drain");
}
// ------------------------=
// FUNC: finish_recognition
// DESC: Completes one delayed native recognition result without manufacturing another controller turn.
// ------------------=
fn finish_recognition(transcript: &[u8]) {
    assert_eq!(conversation::state().0,conversation::State::Recognizing);
    *TEST_TRANSCRIPT.lock().unwrap()=Some(transcript.to_vec());
    BUSY.store(false,Ordering::SeqCst);READY.store(true,Ordering::SeqCst);
    conversation::poll();
    *TEST_TRANSCRIPT.lock().unwrap()=None;
}
// ------------------------=
// FUNC: begin_delayed_recognition
// DESC: Sends a real segmented utterance to the production controller and holds its asynchronous result.
// ------------------=
fn begin_delayed_recognition(amplitude: i16) {
    assert_eq!(conversation::state().0,conversation::State::Listening);
    MICROPHONE.lock().unwrap().extend(microphone_phrase(amplitude));
    drain_microphone();
    assert_eq!(conversation::state().0,conversation::State::Recognizing);
}
// ------------------------=
// FUNC: wake_handoff_tests
// DESC: Verifies lossless bounded wake-to-command capture, strict authorization, expiry, repeated turns, and privacy resets through real VAD and controller state.
// ------------------=
fn wake_handoff_tests(owner: runtime::execution::SecurityIdentity) {
    use conversation::State;
    HOLD_RECOGNITION.store(true,Ordering::SeqCst);
    TURNS.store(0,Ordering::SeqCst);PROMPTS.lock().unwrap().clear();
    RECOGNITION_REQUESTS.lock().unwrap().clear();
    assert!(conversation::start(owner));
    let captures=CAPTURES.load(Ordering::SeqCst);

    // The command starts within the same driver block that ends the wake,
    // then finishes while Whisper is still working. Every soft-onset sample
    // must survive instead of being discarded by the muted-drain path.
    for turn in 0..3 {
        let mut command=vec![120;3200];
        command.extend(microphone_phrase(2300+turn));
        let mut expected=voice_vad::Utterance::new(300);
        expected.push(&command);
        MICROPHONE.lock().unwrap().extend(microphone_phrase(1100));
        MICROPHONE.lock().unwrap().extend(command);
        drain_microphone();
        assert_eq!(conversation::state().0,State::Recognizing);
        assert_eq!(TURNS.load(Ordering::SeqCst),turn as usize);
        NOW.fetch_add(8_000_000_000,Ordering::SeqCst);
        finish_recognition(b"Infinity");
        assert_eq!(conversation::state().0,State::Listening);
        assert!(conversation::wake_armed());
        conversation::poll();
        assert_eq!(conversation::state().0,State::Recognizing);
        assert_eq!(&*RECOGNIZED.lock().unwrap(),expected.speech().unwrap());
        assert_eq!(RECOGNIZED.lock().unwrap().iter().filter(|&&v|v==120).count(),3200);
        finish_recognition(b"help me");
        assert_eq!(conversation::state().0,State::Thinking);
        assert_eq!(PROMPTS.lock().unwrap().last().unwrap(),b"help me");
        complete_voice_reply();
        assert_eq!(CAPTURES.load(Ordering::SeqCst),captures);
    }

    // A combined wake and command owns exactly one turn: an extra utterance
    // captured during ASR may not run after its response finishes.
    begin_delayed_recognition(1200);
    MICROPHONE.lock().unwrap().extend(microphone_phrase(3100));drain_microphone();
    let submitted=RECOGNITION_REQUESTS.lock().unwrap().len();
    finish_recognition(b"Infinity, combined request");
    complete_voice_reply();conversation::poll();
    assert_eq!(RECOGNITION_REQUESTS.lock().unwrap().len(),submitted);
    assert_eq!(TURNS.load(Ordering::SeqCst),4);

    // Ambient speech cannot authorize its retained successor. A real wake in
    // that successor is nevertheless retained and can authorize one command.
    begin_delayed_recognition(1400);
    MICROPHONE.lock().unwrap().extend(microphone_phrase(1500));drain_microphone();
    finish_recognition(b"ambient conversation");
    conversation::poll();assert_eq!(conversation::state().0,State::Recognizing);
    finish_recognition(b"more ambient conversation");
    assert_eq!(conversation::state().0,State::Listening);
    assert_eq!(TURNS.load(Ordering::SeqCst),4);
    begin_delayed_recognition(1400);
    MICROPHONE.lock().unwrap().extend(microphone_phrase(1600));drain_microphone();
    finish_recognition(b"ambient conversation");
    conversation::poll();finish_recognition(b"Infinity, retained wake");
    assert_eq!(PROMPTS.lock().unwrap().last().unwrap(),b"retained wake");
    complete_voice_reply();

    // Do not retain speech that only starts after the provisional ten-second
    // handoff window while the wake's decode result is still pending.
    begin_delayed_recognition(1700);
    NOW.fetch_add(10_000_000_000,Ordering::SeqCst);
    MICROPHONE.lock().unwrap().extend(microphone_phrase(3200));drain_microphone();
    let submitted=RECOGNITION_REQUESTS.lock().unwrap().len();
    finish_recognition(b"Infinity");conversation::poll();
    assert_eq!(conversation::state().0,State::Listening);
    assert_eq!(RECOGNITION_REQUESTS.lock().unwrap().len(),submitted);
    // Starting before the armed deadline remains valid even if completing
    // that same utterance and decoding it occur after the deadline.
    MICROPHONE.lock().unwrap().extend([2400;1600]);drain_microphone();
    NOW.fetch_add(11_000_000_000,Ordering::SeqCst);
    MICROPHONE.lock().unwrap().extend([0;voice_vad::END_SILENCE]);drain_microphone();
    assert_eq!(conversation::state().0,State::Recognizing);
    finish_recognition(b"started before expiry");
    assert_eq!(PROMPTS.lock().unwrap().last().unwrap(),b"started before expiry");
    complete_voice_reply();

    // The bounded capture cannot silently turn a ten-second prefix into a
    // command. Drop the entire overlong turn and require a fresh wake phrase.
    let submitted=RECOGNITION_REQUESTS.lock().unwrap().len();
    let turns=TURNS.load(Ordering::SeqCst);
    MICROPHONE.lock().unwrap().extend(vec![2400;voice_vad::MAX_SAMPLES]);
    drain_microphone();
    assert_eq!(conversation::state().0,State::Listening);
    assert!(conversation::waiting_for_pause());
    assert_eq!(RECOGNITION_REQUESTS.lock().unwrap().len(),submitted);
    MICROPHONE.lock().unwrap().extend(microphone_phrase(2600));drain_microphone();
    assert!(!conversation::waiting_for_pause());
    assert_eq!(RECOGNITION_REQUESTS.lock().unwrap().len(),submitted);
    begin_delayed_recognition(1700);finish_recognition(b"unaddressed tail");
    assert_eq!(TURNS.load(Ordering::SeqCst),turns);
    begin_delayed_recognition(1700);
    MICROPHONE.lock().unwrap().extend(vec![2400;voice_vad::MAX_SAMPLES]);drain_microphone();
    finish_recognition(b"Infinity");
    assert_eq!(conversation::state().0,State::Listening);
    assert!(!conversation::wake_armed());
    MICROPHONE.lock().unwrap().extend(microphone_phrase(2600));drain_microphone();
    begin_delayed_recognition(1700);finish_recognition(b"Infinity, after overflow");
    assert_eq!(PROMPTS.lock().unwrap().last().unwrap(),b"after overflow");
    complete_voice_reply();

    // A pending wake cannot authorize private audio after model/wake changes,
    // explicit cancellation, session lock, or capture permission revocation.
    for reason in 0..6 {
        begin_delayed_recognition(1800);
        MICROPHONE.lock().unwrap().extend(microphone_phrase(3300));drain_microphone();
        let turns=TURNS.load(Ordering::SeqCst);
        match reason {
            0 => {SELECTED_MODEL.store(9,Ordering::SeqCst);conversation::poll();},
            1 => {WAKE_SETTING.store(1,Ordering::SeqCst);conversation::poll();},
            2 => {CHAT_READY.store(false,Ordering::SeqCst);conversation::poll();},
            3 => {assert!(conversation::stop(owner));},
            4 => {ACTIVE.store(false,Ordering::SeqCst);conversation::poll();},
            _ => {CAPTURE_STATE.store(3,Ordering::SeqCst);conversation::poll();},
        }
        if reason<3 {
            finish_recognition(b"Infinity");
            assert_eq!(conversation::state().0,State::Listening);
            assert!(!conversation::wake_armed());
            conversation::poll();assert_eq!(TURNS.load(Ordering::SeqCst),turns);
            conversation::stop(owner);
        }
        BUSY.store(false,Ordering::SeqCst);READY.store(false,Ordering::SeqCst);
        ACTIVE.store(true,Ordering::SeqCst);CHAT_READY.store(true,Ordering::SeqCst);
        SELECTED_MODEL.store(0,Ordering::SeqCst);WAKE_SETTING.store(0,Ordering::SeqCst);
        conversation::poll();
        assert_eq!(conversation::state().0,State::Off);
        assert!(conversation::start(owner));conversation::poll();
        assert_eq!(conversation::state().0,State::Listening);
        assert!(!conversation::wake_armed());
        assert_eq!(TURNS.load(Ordering::SeqCst),turns);
    }
    conversation::stop(owner);BUSY.store(false,Ordering::SeqCst);conversation::poll();
    HOLD_RECOGNITION.store(false,Ordering::SeqCst);
}

// ------------------------=
// FUNC: transcript_handoff_tests
// DESC: Exercises atomic UTF-8-to-composer transfer, rejection recovery, owned queue expiry/context changes, and terminal warmup failures.
// ------------------=
fn transcript_handoff_tests(owner: runtime::execution::SecurityIdentity) {
    use conversation::State;
    assert!(conversation::start(owner));
    recognize_voice("Infinity, what’s \"new\"—today?\nNext\u{a0}line…".as_bytes());
    assert_eq!(PROMPTS.lock().unwrap().last().unwrap(),b"what's \"new\"-today? Next line...");
    complete_voice_reply();
    let turns=TURNS.load(Ordering::SeqCst);
    for transcript in ["Infinity, unsupported 🚀", "Infinity, \n\t\u{a0}", ""] {
        recognize_voice(transcript.as_bytes());
        assert_eq!(conversation::state().0,State::Listening);
        assert!(COMPOSER.lock().unwrap().is_empty());
        assert_eq!(TURNS.load(Ordering::SeqCst),turns);
    }
    INPUT_FAIL_AT.store(4,Ordering::SeqCst);
    recognize_voice(b"Infinity, partially inserted text must roll back");
    INPUT_FAIL_AT.store(usize::MAX,Ordering::SeqCst);
    assert_eq!(conversation::state().0,State::Listening);
    assert!(COMPOSER.lock().unwrap().is_empty());
    assert_eq!(INPUT_LENGTH.load(Ordering::SeqCst),0);

    // A recognized voice command never replaces or submits an existing draft.
    with_ai_runtime(|ai|for &byte in b"my typed draft"{assert!(ai.chat.push_input(byte));});
    recognize_voice(b"Infinity, spoken request");
    assert_eq!(*COMPOSER.lock().unwrap(),b"my typed draft");
    assert_eq!(TURNS.load(Ordering::SeqCst),turns);
    COMPOSER.lock().unwrap().clear();INPUT_LENGTH.store(0,Ordering::SeqCst);

    for reason in 0..6 {
        CHAT_READY.store(false,Ordering::SeqCst);MODEL_READY.store(false,Ordering::SeqCst);
        recognize_voice(b"Infinity, queued voice request");
        assert_eq!(conversation::state().0,State::Submitting);
        assert_eq!(*COMPOSER.lock().unwrap(),b"queued voice request");
        match reason {
            0=>{NOW.fetch_add(10_000_000_000,Ordering::SeqCst);},
            1=>{SELECTED_MODEL.store(8,Ordering::SeqCst);},
            2=>{WAKE_SETTING.store(1,Ordering::SeqCst);},
            3=>{with_ai_runtime(|ai|assert!(ai.chat.push_input(b'!')));},
            4=>{ACTIVE.store(false,Ordering::SeqCst);},
            _=>{conversation::stop(owner);},
        }
        CHAT_READY.store(true,Ordering::SeqCst);MODEL_READY.store(true,Ordering::SeqCst);
        conversation::poll();
        assert_eq!(TURNS.load(Ordering::SeqCst),turns);
        if reason==3 {assert_eq!(*COMPOSER.lock().unwrap(),b"queued voice request!");}
        else {assert!(COMPOSER.lock().unwrap().is_empty());}
        COMPOSER.lock().unwrap().clear();INPUT_LENGTH.store(0,Ordering::SeqCst);
        SELECTED_MODEL.store(0,Ordering::SeqCst);WAKE_SETTING.store(0,Ordering::SeqCst);
        ACTIVE.store(true,Ordering::SeqCst);
        if matches!(conversation::state().0,State::Stopping|State::Off) {
            conversation::poll();assert!(conversation::start(owner));
        }
        assert_eq!(conversation::state().0,State::Listening);
    }
    recognize_voice(b"Infinity, recovered normally");
    assert_eq!(PROMPTS.lock().unwrap().last().unwrap(),b"recovered normally");
    complete_voice_reply();conversation::stop(owner);conversation::poll();

    // Warmup failure is visible and cannot reopen capture by retrying a
    // terminal engine on every poll or by repeatedly toggling the widget.
    MODEL_READY.store(false,Ordering::SeqCst);
    let captures=CAPTURES.load(Ordering::SeqCst);
    assert!(conversation::start(owner));assert_eq!(conversation::state().0,State::Starting);
    PREPARE_FAILED.store(true,Ordering::SeqCst);conversation::poll();
    assert_eq!(conversation::state().0,State::Failed);
    for _ in 0..5 {conversation::poll();}
    assert_eq!(CAPTURES.load(Ordering::SeqCst),captures);
    assert!(!conversation::start(owner));assert_eq!(conversation::state().0,State::Failed);
    PREPARE_FAILED.store(false,Ordering::SeqCst);MODEL_READY.store(true,Ordering::SeqCst);
    assert!(conversation::start(owner));conversation::stop(owner);conversation::poll();
}
