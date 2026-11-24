//! Login-only microphone ownership and isolated speaker inference. Desktop voice
//! capture, wake detection, recognition, playback and interruption are not used.
use super::{identity::{StableId,UserState},execution::SecurityIdentity,native_app_task::Task,native_process::State};
use infinity_speaker_core::{self as speaker,policy};
use super::ai::voice_pcm::Resampler;
use zeroize::Zeroizing;
#[path="face_schedule.rs"] mod schedule;
#[path="voice_identity_capture.rs"] mod capture_session;
#[cfg(target_arch="aarch64")]
static WORKER:&[u8]=include_bytes!("../../build/speaker/aarch64/speaker-worker.elf");
#[cfg(target_arch="x86_64")]
static WORKER:&[u8]=include_bytes!("../../build/speaker/x86_64/speaker-worker.elf");
static MODEL:&[u8]=include_bytes!("../../build/speaker/models/resnet34.iface");
static HEAD:&[u8]=include_bytes!("../../build/speaker/models/resnet34.head");
const OWNER:SecurityIdentity=SecurityIdentity(*b"VOICEIDENTITY\0\0\0");
pub use crate::ui::voice_setup_layout::{Stage,Failure};
struct Host{
 account:Option<StableId>,stage:Stage,enrollment:policy::Enrollment,pending:Option<super::voice_identity_login::Pending>,
 proof:Option<super::voice_identity_login::Proof>,template:[u8;policy::BYTES],pcm:[i16;speaker::SAMPLES],length:usize,
 cap:u64,capture_session:capture_session::Session,resampler:Resampler,deadline:u64,captured:u64,level:u16,task:Option<Task>,terminal:Option<State>,scrubbed:usize,result:Option<[f32;speaker::DIM]>,login:bool,greeting_pending:bool,waiting_face:bool,face_started:bool,
}
static mut HOST:Host=Host{account:None,stage:Stage::Idle,enrollment:policy::Enrollment::new(),pending:None,proof:None,
 template:[0;policy::BYTES],pcm:[0;speaker::SAMPLES],length:0,cap:0,capture_session:capture_session::Session::new(),resampler:Resampler::empty(),deadline:0,captured:0,level:0,task:None,terminal:None,scrubbed:0,result:None,login:false,greeting_pending:false,waiting_face:false,face_started:false};
// ------------------------=
// FUNC: now
// DESC: Reads the existing monotonic clock without changing audio scheduling.
// ------------------=
fn now()->u64{super::ai::qwen::workers::clock_ns()}
// ------------------------=
// FUNC: snapshot
// DESC: Exposes only presentation state and sample count, never raw audio or embeddings.
// ------------------=
pub fn snapshot()->(Stage,usize,u16){unsafe{let h=&*core::ptr::addr_of!(HOST);(h.stage,h.enrollment.count(),h.level)}}
// ------------------------=
// FUNC: caption
// DESC: Supplies actionable login status while retaining password fallback.
// ------------------=
pub fn caption()->Option<&'static[u8]>{unsafe{let h=&*core::ptr::addr_of!(HOST);if !h.login{return None;}if h.waiting_face{return Some(super::face_setup_host::login_caption());}if h.face_started&&matches!(h.stage,Stage::Failed(_)){return Some(b"Face not verified - retry sign-in");}Some(match h.stage{
 Stage::Capturing=>b"Say Hello Infinity or Infinity Hello",
 Stage::Comparing if h.greeting_pending=>b"Checking your sign-in phrase...",
 Stage::Comparing=>b"Verifying your voice...",
 Stage::Verified=>b"Face and voice verified",
 Stage::Failed(Failure::NoEnrollment)=>b"Voice not enrolled - use your password",
 Stage::Failed(Failure::Mismatch)=>b"Voice not matched - retry sign-in",
 Stage::Failed(Failure::Quiet|Failure::Timeout)=>b"No clear voice captured - retry sign-in",
 Stage::Failed(_)=>b"Voice check unavailable - use your password",
 _=>return None,
})}}
struct Microphone<'a>(&'a mut u64);
impl capture_session::Backend for Microphone<'_>{
 // ------------------------=
 // FUNC: open
 // DESC: Grants one bounded login-workflow input lease without changing the shared audio adapter.
 // ------------------=
 fn open(&mut self,until:u64)->bool{
  let cap=super::with_runtime(|r|r.capabilities.grant(super::capability::CapabilityType::AudioInput,0,1,0,OWNER,OWNER,Some(until/1_000_000_000),0).ok()).flatten();
  let Some(cap)=cap else{return false;};*self.0=cap;
  if crate::drivers::audio::capture(OWNER,cap){true}else{self.close();false}
 }
 // ------------------------=
 // FUNC: renew
 // DESC: Extends the same live device stream within its original capability lifetime.
 // ------------------=
 fn renew(&mut self)->bool{crate::drivers::audio::renew_capture(OWNER,*self.0)}
 // ------------------------=
 // FUNC: discard
 // DESC: Excludes inter-sample microphone data without cycling the hardware stream.
 // ------------------=
 fn discard(&mut self)->bool{crate::drivers::audio::discard_capture_pending(OWNER)==Some(true)}
 // ------------------------=
 // FUNC: close
 // DESC: Stops only this workflow's input and retires its capability.
 // ------------------=
 fn close(&mut self){if *self.0!=0{crate::drivers::audio::stop_capture(OWNER);super::with_runtime(|r|{let _=r.capabilities.retire_leaf(*self.0,OWNER);});*self.0=0;}}
}
// ------------------------=
// FUNC: stop_capture
// DESC: Revokes only this workflow's microphone lease and clears its private resampler history.
// ------------------=
fn stop_capture(h:&mut Host){
 if h.greeting_pending{super::ai::voice_input::stop(OWNER);h.greeting_pending=false;}
 h.capture_session.close(&mut Microphone(&mut h.cap));
 h.resampler.clear();h.level=0;
}
// ------------------------=
// FUNC: fail
// DESC: Fails closed, erasing captured samples and pending face authority without creating a session.
// ------------------=
fn fail(h:&mut Host,error:Failure){
 let stamp=super::ai::clock::SYSTEM.utc_nanoseconds(now()).map(|n|n/1_000_000_000);
 let _=crate::storage::app_errors::record(b"VoiceIdentity",stamp,"enrollment-or-login",format_args!("failure={} samples={} captured_samples={} device={:?}",error as u8,h.enrollment.count(),h.length,crate::drivers::audio::capture_status()));
 if h.login||!matches!(error,Failure::Mismatch|Failure::Quiet|Failure::Clipped|Failure::Timeout|Failure::AnalysisTimeout){stop_capture(h);}
 h.pcm.fill(0);h.length=0;h.pending=None;h.waiting_face=false;h.result=None;h.template.fill(0);h.stage=Stage::Failed(error);}
// ------------------------=
// FUNC: cancel
// DESC: Cancels account/page transitions; worker memory is scrubbed cooperatively by the next poll.
// ------------------=
pub fn cancel(){unsafe{let h=&mut *core::ptr::addr_of_mut!(HOST);stop_capture(h);h.account=None;h.pending=None;h.proof=None;h.result=None;h.pcm.fill(0);h.template.fill(0);h.enrollment.clear();h.length=0;h.stage=Stage::Idle;h.login=false;h.waiting_face=false;h.face_started=false;}}
// ------------------------=
// FUNC: begin_enrollment
// DESC: Authorizes enrollment with the configured account password before granting any microphone access.
// ------------------=
pub fn begin_enrollment(account:StableId,secret:&[u8])->bool{unsafe{
 let h=&mut *core::ptr::addr_of_mut!(HOST);if h.task.is_some(){return false;}
 let authorized=super::with_runtime(|r|r.identity.authenticate(account,secret,now()/1_000_000).is_ok()).unwrap_or(false);
 if !authorized{fail(h,Failure::Unauthorized);return false;}
 cancel();let h=&mut *core::ptr::addr_of_mut!(HOST);h.account=Some(account);h.deadline=now().saturating_add(240_000_000_000);h.stage=Stage::Ready;true
}}
// ------------------------=
// FUNC: record
// DESC: Starts explicit fresh enrollment capture using only an account-authorized short-lived lease.
// ------------------=
pub fn record()->bool{unsafe{let h=&mut *core::ptr::addr_of_mut!(HOST);
 if h.account.is_none()||h.task.is_some()||!matches!(h.stage,Stage::Ready|Stage::Failed(Failure::Mismatch|Failure::Quiet|Failure::Clipped|Failure::Worker|Failure::Timeout|Failure::AnalysisTimeout|Failure::Unavailable)){return false;}
 capture(h)
}}
// ------------------------=
// FUNC: capture
// DESC: Opens a bounded login-only capture lease and starts with empty private sample history.
// ------------------=
fn capture(h:&mut Host)->bool{
 h.pcm.fill(0);h.length=0;h.captured=0;h.level=0;h.resampler.clear();
 let clock=now();
 if !h.capture_session.begin(clock,&mut Microphone(&mut h.cap)){fail(h,Failure::Unavailable);return false;}
 let rate=crate::drivers::audio::capture_status().map(|s|s.sample_rate).unwrap_or(0);
 if !h.resampler.configure(rate){fail(h,Failure::Unavailable);return false;}
 if h.login{h.pending=h.account.map(|account|super::voice_identity_login::Pending::begin(account,clock/1_000_000));}
 h.deadline=clock+20_000_000_000;h.stage=Stage::Capturing;true
}
// ------------------------=
// FUNC: start_login
// DESC: Opens the active selected account's speaker template and starts voice capture before any face comparison.
// ------------------=
pub fn start_login(account:StableId)->bool{unsafe{
 let h=&mut *core::ptr::addr_of_mut!(HOST);if h.task.is_some()||super::face_setup_host::login_busy()||matches!(h.stage,Stage::Capturing|Stage::Comparing){return false;}
 let authorized=super::with_runtime(|r|r.identity.user(account).is_some_and(|u|u.state==UserState::Active)&&r.identity.user_profile(account).is_some_and(|p|p.face_login_enabled)).unwrap_or(false);
 if !authorized{return false;}
 cancel();let h=&mut *core::ptr::addr_of_mut!(HOST);h.login=true;h.account=Some(account);
 h.pending=Some(super::voice_identity_login::Pending::begin(account,now()/1_000_000));
 let Some(key)=super::with_runtime(|r|r.nodes.voice_vault_key(&account.0)).flatten() else{fail(h,Failure::NoEnrollment);return false;};
 let mut record=Zeroizing::new([0;super::voice_identity_vault::BYTES]);
 let model:[u8;32]=MODEL[8..40].try_into().unwrap();
 let plain=if crate::storage::voice_identity_load(account.0,&mut *record)==Ok(record.len()){
 super::voice_identity_vault::open(&key,account,&model,&record)}else{None};
 let Some(plain)=plain else{fail(h,Failure::NoEnrollment);return false;};h.template.copy_from_slice(&*plain);capture(h)
}}
// ------------------------=
// FUNC: waiting_for_face
// DESC: Allows camera polling only after successful speaker verification in this attempt.
// ------------------=
pub fn waiting_for_face()->bool{unsafe{(*core::ptr::addr_of!(HOST)).waiting_face}}
// ------------------------=
// FUNC: accept_face
// DESC: Completes the voice-first attempt with one fresh face proof and never restarts voice capture.
// ------------------=
pub fn accept_face(account:StableId,face:super::face_login::Proof)->bool{unsafe{
 let h=&mut *core::ptr::addr_of_mut!(HOST);
 if !h.waiting_face||h.account!=Some(account){return false;}
 h.proof=h.pending.take().and_then(|p|p.complete(account,face,now()/1_000_000));h.waiting_face=false;
 if h.proof.is_some(){h.stage=Stage::Verified;true}else{fail(h,Failure::Unauthorized);false}
}}
// ------------------------=
// FUNC: save
// DESC: Persists verified speaker samples with authenticated encryption and verifies durable readback before reporting success.
// ------------------=
fn save(h:&mut Host)->bool{
 let Some(account)=h.account else{return false;};
 let mut payload=Zeroizing::new([0;policy::BYTES]);if h.enrollment.encode(&mut payload).is_err(){return false;}
 let Some(key)=super::with_runtime(|r|r.nodes.voice_vault_key(&account.0)).flatten() else{return false;};
 let Some(seed)=(unsafe{super::native_python_entropy::next(&account.0)})else{return false;};let seed=Zeroizing::new(seed);
 let model:[u8;32]=MODEL[8..40].try_into().unwrap();
 let Some(record)=super::voice_identity_vault::seal(&key,account,&model,seed[..12].try_into().unwrap(),&payload)else{return false;};
 if crate::storage::voice_identity_commit(account.0,&*record).is_err(){return false;}
 let mut readback=Zeroizing::new([0;super::voice_identity_vault::BYTES]);
 crate::storage::voice_identity_load(account.0,&mut *readback)==Ok(readback.len())&&super::voice_identity_vault::open(&key,account,&model,&readback).is_some_and(|p|*p==*payload)
}
// ------------------------=
// FUNC: recognize_greeting
// DESC: Checks login words from the exact private speaker sample; keeps its capability valid until transcript delivery.
// ------------------=
fn recognize_greeting(h:&mut Host){
 if super::ai::voice_input::submit(OWNER,h.cap,&h.pcm).is_ok(){
  h.greeting_pending=true;h.stage=Stage::Comparing;h.deadline=now()+30_000_000_000;
 }else{fail(h,Failure::Unavailable);}
}
// ------------------------=
// FUNC: poll_greeting
// DESC: Requires a recognized login greeting before speaker inference; unrelated speech never starts face authentication.
// ------------------=
fn poll_greeting(h:&mut Host,clock:u64){
 use super::ai::voice_input::{self,InputState};
 voice_input::poll();
 if clock>=h.deadline{fail(h,Failure::Timeout);return;}
 match voice_input::status().state{
  InputState::Ready=>{
   let mut text=[0u8;512];
   let result=voice_input::take(OWNER,&mut text);
   h.greeting_pending=false;
   let accepted=result.ok().is_some_and(|n|h.pending.as_mut().is_some_and(|p|p.accept_greeting(h.account.unwrap(),h.captured,clock/1_000_000,&text[..n])));
   text.fill(0);
   if accepted{launch(h);}else{capture(h);}
  },
  InputState::Failed|InputState::Cancelled=>fail(h,Failure::Unavailable),
  _=>{},
 }
}
// ------------------------=
// FUNC: launch
// DESC: Starts only the packaged isolated Rust speaker worker with bounded private model memory.
// ------------------=
fn launch(h:&mut Host)->bool{unsafe{
 if h.login{stop_capture(h);}
 let Some((_,controller))=super::native_compile_job::platform()else{fail(h,Failure::Worker);return false;};
 h.deadline=now()+60_000_000_000;h.result=None;h.scrubbed=0;h.terminal=None;
 match Task::start_camera_worker(WORKER,OWNER,9,super::native_memory::memory_mut(),controller,h.deadline,128*1024*1024/4096){
 Ok(task)=>{h.task=Some(task);h.stage=Stage::Comparing;true},Err(_)=>{fail(h,Failure::Worker);false}}
}}
// ------------------------=
// FUNC: poll
// DESC: Advances authorized capture, isolated inference, matching and private-memory reclamation without blocking desktop services.
// ------------------=
pub fn poll(enrollment:Option<StableId>,selected:Option<StableId>)->bool{unsafe{
 let h=&mut *core::ptr::addr_of_mut!(HOST);let before=(h.stage,h.enrollment.count(),h.greeting_pending);let clock=now();
 let valid=h.account.is_some()&&h.account==if h.login{selected}else{enrollment}
  &&super::with_runtime(|r|h.account.is_some_and(|a|r.identity.user(a).is_some_and(|u|u.state==UserState::Active))).unwrap_or(false);
 if !valid&&h.account.is_some(){cancel();}
 let h=&mut *core::ptr::addr_of_mut!(HOST);
 if h.waiting_face && (!h.pending.as_ref().is_some_and(|p|p.valid(h.account.unwrap(),clock/1_000_000))||!super::face_setup_host::login_busy()){fail(h,Failure::Mismatch);}
 if !h.capture_session.service(clock,&mut Microphone(&mut h.cap)){fail(h,Failure::Unavailable);}
 if h.cap!=0&&h.stage!=Stage::Capturing{let mut discard=[0i16;4096];crate::drivers::audio::read_capture(OWNER,&mut discard);discard.fill(0);}
 if h.stage==Stage::Capturing{
  if clock>=h.deadline{fail(h,Failure::Timeout);}
  else{
   let mut raw=[0i16;4096];let mut mono=[0i16;4096];
   let n=crate::drivers::audio::read_capture(OWNER,&mut raw);let(used,nout)=h.resampler.process(&raw[..n],&mut mono);
   if used!=n{fail(h,Failure::Unavailable);}else{
    if nout>0{h.level=mono[..nout].iter().map(|s|s.unsigned_abs()).max().unwrap_or(0);}
    if h.captured==0&&nout>0&&mono[..nout].iter().map(|&s|(s as i64)*(s as i64)).sum::<i64>()/nout as i64>90_000{h.captured=clock/1_000_000;}
    if h.captured!=0{let take=nout.min(speaker::SAMPLES-h.length);h.pcm[h.length..h.length+take].copy_from_slice(&mono[..take]);h.length+=take;}
    if h.length==speaker::SAMPLES{if h.login{recognize_greeting(h);}else{launch(h);}}
   }
   raw.fill(0);mono.fill(0);
  }
 }
 if h.greeting_pending{poll_greeting(h,clock);}
 if let Some(mut task)=h.task.take(){
  if let Some(state)=h.terminal{
   if schedule::batch(now,|_|task.scrub_private(&mut h.scrubbed).then_some(())).is_some(){
    if task.release(super::native_memory::memory_mut()).is_ok(){finish(h,state,valid&&h.stage==Stage::Comparing,now()/1_000_000);}
    else{fail(h,Failure::Worker);}
    h.terminal=None;
   }else{h.task=Some(task);}
  }
  else{
   let alive=valid&&h.stage==Stage::Comparing&&clock<h.deadline;
   let state=schedule::batch(now,|tick| {
   let state=task.poll(tick,alive&&tick<h.deadline,|caller,tables,op,args|{
    use super::native_memory::{copy_to_user,copy_from_user};
    if !alive||caller!=OWNER{return -13;}
    match op{
     0x1280=>{
      let data=match args[0]{1=>MODEL,2=>HEAD,3=>core::slice::from_raw_parts(h.pcm.as_ptr()as *const u8,speaker::SAMPLES*2),_=>return -2};
      if args[4]!=0||args[5]!=0{return -22;}
      if args[3]==0{return if args[1]==0&&args[2]==0{data.len()as i64}else{-22};}
      if args[3]>65536{return -22;}
      let Some(end)=args[1].checked_add(args[3])else{return -22;};
      let Some(bytes)=data.get(args[1]as usize..end as usize)else{return -22;};
      if copy_to_user(tables,args[2],bytes).is_ok(){bytes.len()as i64}else{-14}
     },
     0x1281=>1,
     0x1282=>{
      if args[1]!=1024||h.result.is_some()||args[2..].iter().any(|&v|v!=0){return -22;}
      let mut b=[0u8;1024];if copy_from_user(tables,args[0],&mut b).is_err(){return -14;}
      let mut vector=[0f32;256];for (v,b)in vector.iter_mut().zip(b.chunks_exact(4)){*v=f32::from_le_bytes(b.try_into().unwrap());}
      b.fill(0);if speaker::normalize(&mut vector).is_err(){return -22;}h.result=Some(vector);1024
     },_=>-38,
    }
   });
   (state!=State::Runnable).then_some(state)
   }).unwrap_or(State::Runnable);
   if state!=State::Runnable{
    h.terminal=Some(state);h.pcm.fill(0);h.length=0;

   }
   h.task=Some(task);
  }
 }
 before!=(h.stage,h.enrollment.count(),h.greeting_pending)
}}
// ------------------------=
// FUNC: retryable_login
// DESC: Allows automatic retry only after worker reclamation and a recoverable login failure; missing enrollment and storage errors remain explicit.
// ------------------=
pub fn retryable_login()->bool{unsafe{
 let h=&*core::ptr::addr_of!(HOST);
 h.login&&h.task.is_none()&&h.terminal.is_none()&&matches!(h.stage,Stage::Failed(Failure::Mismatch|Failure::Quiet|Failure::Clipped|Failure::Timeout|Failure::AnalysisTimeout))
}}
// ------------------------=
// FUNC: take_proof
// DESC: Delivers a completed combined proof only once to the kernel login controller.
// ------------------=
pub fn take_proof()->Option<super::voice_identity_login::Proof>{unsafe{(&mut *core::ptr::addr_of_mut!(HOST)).proof.take()}}

// ------------------------=
// FUNC: finish
// DESC: Publishes sample or authentication results only after the isolated worker's private memory has been scrubbed and released.
// ------------------=
fn finish(h:&mut Host,state:State,alive:bool,clock:u64){
    if alive&&state==State::Exited(0){
     if let Some(mut vector)=h.result.take(){
      if h.login{
       let scores=policy::comparison(&h.template,&vector);
       if policy::matches_combined(&h.template,&vector)==Ok(true){
        let account=h.account.unwrap();
        if h.pending.as_mut().is_some_and(|p|p.accept_voice(account,h.captured,clock)){
         h.face_started=true;
         if super::face_setup_host::start_login(account){h.waiting_face=true;h.stage=Stage::Verified;}
         else{fail(h,Failure::Unavailable);}
        }else{fail(h,Failure::Unauthorized);}}
       else{
        let stamp=super::ai::clock::SYSTEM.utc_nanoseconds(now()).map(|n|n/1_000_000_000);
        let _=crate::storage::app_errors::record(b"VoiceIdentity",stamp,"speaker-comparison",format_args!("agreement_and_centroid={scores:?} combined_threshold=0.50 face_required=true"));
        fail(h,Failure::Mismatch);
       }
       h.template.fill(0);
      }else if h.enrollment.add(vector).is_err(){fail(h,Failure::Mismatch);}
      else if h.enrollment.count()==policy::COUNT{if save(h){stop_capture(h);h.stage=Stage::Saved;h.enrollment.clear();}else{fail(h,Failure::Storage);}}
      else{h.stage=Stage::Ready;}
      for x in &mut vector{unsafe{core::ptr::write_volatile(x,0.0);}}
     }else{fail(h,Failure::Worker);}
    }else if h.stage!=Stage::Idle{
        let stamp=super::ai::clock::SYSTEM.utc_nanoseconds(now()).map(|n|n/1_000_000_000);
        let _=crate::storage::app_errors::record(b"VoiceIdentity",stamp,"worker",format_args!("state={state:?} deadline_expired={}",now()>=h.deadline));
        let error=if state==State::Exited(74){Failure::Quiet}else if state==State::Exited(78){Failure::Clipped}else if now()>=h.deadline{Failure::AnalysisTimeout}else{Failure::Worker};
        fail(h,error);
    }
}
