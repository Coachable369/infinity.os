//! Session-private SSH advice leases. Desktop messages, memory and audio delivery are never remote reply sources.
use super::{AiRuntime,chat::GenerationState,qwen,text_request::{self,AdviceError,AdviceMode,Facts},types::ModelId};

pub const OWNER:usize=crate::ui::app_assistant::PANEL_SLOTS;
pub const CAPACITY:usize=8;
pub const OUTPUT_BYTES:usize=16384;
struct Session {
    owner:[u8;16],session:[u8;16],turn:u64,model:ModelId,state:GenerationState,
    output:[u8;OUTPUT_BYTES],length:usize,history:Option<qwen::service::Conversation>,
}
impl Session {
    // ------------------------=
    // FUNC: new
    // DESC: Initializes an empty private session slot with no inherited transcript or model context.
    // ------------------=
    const fn new()->Self {Self{owner:[0;16],session:[0;16],turn:0,model:0,state:GenerationState::Ready,output:[0;OUTPUT_BYTES],length:0,history:None}}
}
pub(super) struct State {
    sessions:[Session;CAPACITY],active:Option<usize>,saved:Option<qwen::service::Conversation>,
    pub(super) local_epoch:u64,saved_epoch:u64,next_turn:u64,started:Option<u64>,progress:Option<u64>,tokens:usize,
}
impl State {
    // ------------------------=
    // FUNC: new
    // DESC: Creates bounded session storage without allocating a second model or changing desktop ownership.
    // ------------------=
    pub(super) const fn new()->Self {Self{sessions:[const{Session::new()};CAPACITY],active:None,saved:None,local_epoch:0,saved_epoch:0,next_turn:1,started:None,progress:None,tokens:0}}
    // ------------------------=
    // FUNC: find
    // DESC: Resolves only an exact authenticated user and SSH session pair.
    // ------------------=
    fn find(&self,owner:[u8;16],session:[u8;16])->Option<usize>{self.sessions.iter().position(|s|s.turn!=0&&s.owner==owner&&s.session==session)}
}
// ------------------------=
// FUNC: authorized
// DESC: Rechecks active SSH identity, AI capability and the requesting user's own profile independently of the desktop session.
// ------------------=
fn authorized(owner:[u8;16],session:[u8;16])->bool {
    use crate::runtime::identity::{StableId,SessionOrigin,SessionState,UserState,SESSION_AI_USE,MAX_SESSIONS};
    if owner==[0;16]||session==[0;16]{return false;}
    crate::runtime::with_runtime(|runtime|{
        runtime.identity.console_session_authorized(StableId(session),StableId(owner))
            && runtime.identity.user(StableId(owner)).is_some_and(|u|u.state==UserState::Active)
            && runtime.identity.ai_profile(StableId(owner)).is_some_and(|p|p.chat_enabled)
            && (0..MAX_SESSIONS).filter_map(|i|runtime.identity.session_nth(i)).any(|s|s.id.0==session&&s.user.0==owner&&s.origin==SessionOrigin::Ssh&&s.state==SessionState::Active&&s.capabilities&SESSION_AI_USE!=0)
    }).unwrap_or(false)
}
impl AiRuntime {
    // ------------------------=
    // FUNC: remote_advice_active
    // DESC: Reports the remote lease without exposing its prompt, owner or output to a desktop widget.
    // ------------------=
    pub fn remote_advice_active(&self)->bool {self.remote_advice.active.is_some()}
    // ------------------------=
    // FUNC: remote_advice_authorized
    // DESC: Allows the outer model poller to preserve a remote lease only while its own SSH session remains authorized.
    // ------------------=
    pub fn remote_advice_authorized(&self)->bool {self.remote_advice.active.is_some_and(|i|{let s=&self.remote_advice.sessions[i];authorized(s.owner,s.session)})}
    // ------------------------=
    // FUNC: submit_remote_advice
    // DESC: Borrows the native model for one authorized SSH conversation while suspending and fencing local context restoration.
    // ------------------=
    pub fn submit_remote_advice(&mut self,owner:[u8;16],session:[u8;16],model:ModelId,input:&[u8],mode:AdviceMode,facts:Option<Facts>)->Result<u64,AdviceError> {
        if !authorized(owner,session){return Err(AdviceError::Denied);}
        if self.app_owner.is_some()||self.execution.busy()||self.application_agent.is_some()||self.intelligence_goal.is_some()
            ||self.chat.generation_state==GenerationState::Running||self.qwen.as_ref().is_some_and(|s|s.busy())||self.other_native.as_ref().is_some_and(|s|s.busy()){return Err(AdviceError::Busy);}
        if !matches!(model,super::chat::HERMES_MODEL_ID|super::chat::MINISTRAL_MODEL_ID)||!self.native_ready(model){return Err(AdviceError::Unavailable);}
        let mut prompt=[0u8;6144];let mode=if text_request::decision_question(input){AdviceMode::Decide}else{mode};
        let length=text_request::prepare_prompt(mode,input,facts,&mut prompt)?;
        let existing=self.remote_advice.find(owner,session);
        let index=existing.or_else(||self.remote_advice.sessions.iter().position(|s|s.turn==0||!authorized(s.owner,s.session))).ok_or(AdviceError::Busy)?;
        if self.remote_advice.next_turn==u64::MAX{return Err(AdviceError::ContextFull);}
        if model!=self.active_native{core::mem::swap(&mut self.qwen,&mut self.other_native);self.active_native=model;}
        let service=self.qwen.as_mut().ok_or(AdviceError::Unavailable)?;
        let saved=service.suspend_conversation().ok_or(AdviceError::Busy)?;
        let reuse=existing==Some(index)&&self.remote_advice.sessions[index].model==model;
        if reuse {if let Some(history)=self.remote_advice.sessions[index].history.take(){service.restore_conversation(history);}}
        let submitted=service.submit_with_kind(&prompt[..length],qwen::service::SubmissionKind::Conversation);prompt.fill(0);
        if let Err(error)=submitted {
            if reuse {self.remote_advice.sessions[index].history=service.suspend_conversation();}
            service.restore_conversation(saved);
            return Err(if error==qwen::gguf::Error::Overflow{AdviceError::ContextFull}else{AdviceError::Failed});
        }
        if !reuse {self.remote_advice.sessions[index]=Session::new();}
        let turn=self.remote_advice.next_turn;self.remote_advice.next_turn+=1;
        let slot=&mut self.remote_advice.sessions[index];slot.owner=owner;slot.session=session;slot.model=model;slot.turn=turn;slot.state=GenerationState::Running;slot.output.fill(0);slot.length=0;
        self.remote_advice.active=Some(index);self.remote_advice.saved=Some(saved);self.remote_advice.saved_epoch=self.remote_advice.local_epoch;
        let now=crate::ui::performance::monotonic_ns();self.remote_advice.started=now;self.remote_advice.progress=now;self.remote_advice.tokens=service.progress_tokens();
        self.app_owner=Some(OWNER);self.app_workflow_goal=None;
        Ok(turn)
    }
    // ------------------------=
    // FUNC: remote_advice_state
    // DESC: Returns content-free delivery state only for the requesting live SSH session.
    // ------------------=
    pub fn remote_advice_state(&self,owner:[u8;16],session:[u8;16])->Option<(u64,GenerationState)> {
        if !authorized(owner,session){return None;}let index=self.remote_advice.find(owner,session)?;let slot=&self.remote_advice.sessions[index];Some((slot.turn,slot.state))
    }
    // ------------------------=
    // FUNC: remote_advice_latest
    // DESC: Resolves a session's latest turn without using desktop trace state or another connection's pending reply.
    // ------------------=
    pub fn remote_advice_latest(&self,owner:[u8;16],session:[u8;16])->Result<u64,AdviceError> {
        if !authorized(owner,session){return Err(AdviceError::Denied);}self.remote_advice_state(owner,session).map(|v|v.0).ok_or(AdviceError::NotOwned)
    }
    // ------------------------=
    // FUNC: remote_advice_reply
    // DESC: Copies a complete bounded private reply only when user, session and turn all match; small destinations never receive truncated output.
    // ------------------=
    pub fn remote_advice_reply(&self,owner:[u8;16],session:[u8;16],turn:u64,out:&mut[u8])->Result<(GenerationState,usize),AdviceError> {
        if !authorized(owner,session){return Err(AdviceError::Denied);}
        let index=self.remote_advice.find(owner,session).ok_or(AdviceError::NotOwned)?;let slot=&self.remote_advice.sessions[index];
        if slot.turn!=turn{return Err(AdviceError::NotOwned);}if out.len()<slot.length{return Err(AdviceError::ContextFull);}
        out[..slot.length].copy_from_slice(&slot.output[..slot.length]);Ok((slot.state,slot.length))
    }
    // ------------------------=
    // FUNC: cancel_remote_advice
    // DESC: Cancels only the exact request owned by this active SSH session, preserving every unrelated conversation.
    // ------------------=
    pub fn cancel_remote_advice(&mut self,owner:[u8;16],session:[u8;16],turn:u64)->Result<(),AdviceError> {
        if !authorized(owner,session){return Err(AdviceError::Denied);}
        let index=self.remote_advice.find(owner,session).ok_or(AdviceError::NotOwned)?;
        if self.remote_advice.sessions[index].turn!=turn||self.remote_advice.active!=Some(index){return Err(AdviceError::NotOwned);}
        self.finish_remote_advice(GenerationState::Cancelled,false);Ok(())
    }
    // ------------------------=
    // FUNC: reset_remote_advice
    // DESC: Erases only an idle requesting session's reply and model history; active work requires explicit cancellation first.
    // ------------------=
    pub fn reset_remote_advice(&mut self,owner:[u8;16],session:[u8;16])->Result<(),AdviceError> {
        if !authorized(owner,session){return Err(AdviceError::Denied);}
        if let Some(index)=self.remote_advice.find(owner,session){if self.remote_advice.active==Some(index){return Err(AdviceError::Busy);}self.remote_advice.sessions[index]=Session::new();}Ok(())
    }
    // ------------------------=
    // FUNC: finish_remote_advice
    // DESC: Releases remote tokens and restores the suspended local conversation only when its ownership and memory epoch remain current.
    // ------------------=
    pub(super) fn finish_remote_advice(&mut self,state:GenerationState,revoke:bool) {
        let Some(index)=self.remote_advice.active.take()else{return;};
        if let Some(service)=self.qwen.as_mut(){
            if service.busy(){service.cancel();}
            self.remote_advice.sessions[index].history=service.suspend_conversation();
            if let Some(saved)=self.remote_advice.saved.take(){if self.remote_advice.saved_epoch==self.remote_advice.local_epoch{service.restore_conversation(saved);}else{service.clear_conversation();}}
            else{service.clear_conversation();}
        }else{self.remote_advice.saved=None;self.remote_advice.sessions[index].history=None;}
        if revoke {self.remote_advice.sessions[index]=Session::new();}else{self.remote_advice.sessions[index].state=state;}
        self.remote_advice.started=None;self.remote_advice.progress=None;self.remote_advice.tokens=0;
        self.app_owner=None;self.app_workflow_goal=None;
    }
    // ------------------------=
    // FUNC: poll_remote_advice
    // DESC: Advances only the isolated SSH lease, with session revocation and bounded progress; no desktop transcript or speech queue is updated.
    // ------------------=
    pub(super) fn poll_remote_advice(&mut self)->bool {
        let Some(index)=self.remote_advice.active else{return false;};
        if !self.remote_advice_authorized(){self.finish_remote_advice(GenerationState::Cancelled,true);return false;}
        if self.qwen.is_none(){self.finish_remote_advice(GenerationState::Failed,false);return false;}
        let mut budget=qwen::pump::PumpBudget::new(crate::ui::performance::monotonic_ns());
        while budget.next(crate::ui::performance::monotonic_ns()) {
            let now=crate::ui::performance::monotonic_ns();
            if self.remote_advice.started.zip(now).is_some_and(|(a,b)|b.saturating_sub(a)>120_000_000_000)
                || self.remote_advice.progress.zip(now).is_some_and(|(a,b)|b.saturating_sub(a)>30_000_000_000){self.finish_remote_advice(GenerationState::Failed,false);break;}
            let service=self.qwen.as_mut().unwrap();let result=service.poll();
            if service.progress_tokens()>self.remote_advice.tokens{self.remote_advice.tokens=service.progress_tokens();self.remote_advice.progress=now;}
            if result==Ok(true)||result.is_err()||!service.busy(){let output=service.output();let slot=&mut self.remote_advice.sessions[index];slot.length=output.len().min(OUTPUT_BYTES);slot.output[..slot.length].copy_from_slice(&output[..slot.length]);}
            if result.is_err()||!service.busy(){let state=if result.is_err(){GenerationState::Failed}else if service.completed(){GenerationState::Complete}else{GenerationState::ContextFull};self.finish_remote_advice(state,false);break;}
        }
        false
    }
}
