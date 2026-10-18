//! Cooperative Hermes lease for a user-authorized application interaction session.
use super::{application::{Error, Outcome}, application_host::{self, Context}, AiRuntime, qwen};
use super::application_decision::{parse_proposal,parse_for_state,decision_prompt};

pub struct Agent {
    context: Context,
    objective: [u8;512],
    length: usize,
    continuous: bool,
    revision: u64,
    token: Option<u64>,
    approval: Option<u64>,
    inference: bool,
    deadline: u64,
    remaining: u16,
}
impl AiRuntime {
    // ------------------------=
    // FUNC: start_application_agent
    // DESC: Grants one explicitly requested live app session; generated text cannot call this trusted entry point.
    // ------------------=
    pub fn start_application_agent(&mut self,owner:[u8;16],session:[u8;16],objective:&[u8],continuous:bool)->Result<(),Error> {
        if !self.chat.enabled() || objective.is_empty() || objective.len()>512 {return Err(Error::Denied);}
        let c=application_host::context().ok_or(Error::Missing)?;
        if c.owner!=owner || c.session!=session {return Err(Error::Denied);}
        self.stop_application_agent();
        application_host::with_registry(|r|r.grant(c.instance,owner,session,true))?;
        let mut text=[0;512];text[..objective.len()].copy_from_slice(objective);
        self.application_agent=Some(Agent{context:c,objective:text,length:objective.len(),continuous,revision:0,token:None,approval:None,inference:false,deadline:qwen::workers::clock_ns().saturating_add(10_000_000_000),remaining:256});
        status(b"AI interaction enabled for this app. Waiting for its published state.");
        Ok(())
    }
    // ------------------------=
    // FUNC: stop_application_agent
    // DESC: Revokes action authority and releases only this session's inference lease, preserving desktop conversation.
    // ------------------=
    pub fn stop_application_agent(&mut self) {
        if let Some(agent)=self.application_agent.take() {
            let c=agent.context;
            if self.software_execution(c.owner).is_some_and(|execution|execution.context==c.instance && execution.interaction_revision==0) {
                self.record_application_receipt(c,super::intelligence::software::Event::Failed{code:30});
            }
            self.record_application_receipt(c,super::intelligence::software::Event::InteractionStopped{context:c.instance});
            let _=application_host::with_registry(|r|r.grant(c.instance,c.owner,c.session,false));
            if let Some(token)=agent.approval {
                let mut panel=crate::ui::app_assistant::read(1);
                if panel.pending==crate::ui::app_assistant::Action::NativeApproval && panel.argument_len==8 && panel.argument[..8]==token.to_le_bytes() {
                    panel.pending=crate::ui::app_assistant::Action::None;
                    crate::ui::app_assistant::write(1,panel);
                }
            }
            if agent.inference {self.release_application_inference();}
        }
    }
    // ------------------------=
    // FUNC: record_application_receipt
    // DESC: Connects real granted application state and action receipts to the exact generating goal without creating execution authority.
    // ------------------=
    fn record_application_receipt(&mut self,context:Context,event:super::intelligence::software::Event) {
        if let Some(goal)=self.software_execution(context.owner)
            .filter(|execution|execution.context==context.instance).map(|execution|execution.goal) {
            self.record_software_event(goal,context.owner,event);
        }
    }
    // ------------------------=
    // FUNC: approve_application_action
    // DESC: Accepts an explicit trusted UI approval for the exact live user/session/token, never model text.
    // ------------------=
    pub fn approve_application_action(&mut self,owner:[u8;16],session:[u8;16],token:u64)->Result<(),Error> {
        let agent=self.application_agent.as_mut().ok_or(Error::Missing)?;
        let c=agent.context;
        if c.owner!=owner || c.session!=session || agent.approval!=Some(token) || !self.chat.enabled() {return Err(Error::Denied);}
        let request=application_host::with_registry(|r|r.approve_review(c.instance,owner,session,token,qwen::workers::clock_ns()))?;
        agent.approval=None;agent.token=Some(request.token);agent.remaining-=1;
        let mut panel=crate::ui::app_assistant::read(1);
        if panel.pending==crate::ui::app_assistant::Action::NativeApproval && panel.argument_len==8 && panel.argument[..8]==token.to_le_bytes() {
            panel.pending=crate::ui::app_assistant::Action::None;
            crate::ui::app_assistant::write(1,panel);
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: release_application_inference
    // DESC: Cancels bounded app decision work and restores the shared model conversation before another consumer enters.
    // ------------------=
    fn release_application_inference(&mut self) {
        if let Some(service)=self.qwen.as_mut() {
            service.cancel();
            if let Some(saved)=self.application_saved.take(){service.restore_conversation(saved);}else{service.clear_conversation();}
        }
    }
    // ------------------------=
    // FUNC: poll_application_agent
    // DESC: Advances observe-decide-act without blocking audio, retries, or treating model prose as execution evidence.
    // ------------------=
    pub(super) fn poll_application_agent(&mut self)->bool {
        let Some(mut agent)=self.application_agent.take() else{return false;};
        let c=agent.context;
        let now=qwen::workers::clock_ns();
        if !self.chat.enabled() || application_host::context().is_none_or(|live|live.instance!=c.instance || live.session!=c.session) {
            self.application_agent=Some(agent);self.stop_application_agent();return false;
        }
        let observation=application_host::with_registry(|r|r.observe(c.instance,c.owner,c.session));
        let Ok(state)=observation else{self.application_agent=Some(agent);self.stop_application_agent();return false;};
        if state.revision>0 && state.choices.iter().any(Option::is_some) {
            self.record_application_receipt(c,super::intelligence::software::Event::InteractionReady{context:c.instance,revision:state.revision});
        }
        if let Some(token)=agent.approval {
            let review=application_host::with_registry(|r|r.review(c.instance,c.owner,c.session,now));
            if !matches!(review,Ok(Some(request)) if request.token==token) {
                status(b"The review expired or app state changed. Nothing was executed; submit a new request.");
                self.application_agent=Some(agent);self.stop_application_agent();return true;
            }
            self.application_agent=Some(agent);return false;
        }
        if state.revision==0 && now>=agent.deadline {
            status(b"This app did not publish an AI interaction contract. AI control stopped; the app remains usable.");
            self.application_agent=Some(agent);self.stop_application_agent();return true;
        }
        if let Some(token)=agent.token {
            let receipt=application_host::with_registry(|r|r.result(c.instance,c.owner,c.session,now));
            if let Ok(Some(result))=receipt {
                if result.token==token {
                    agent.token=None;
                    if result.outcome==Outcome::Applied {
                        self.record_application_receipt(c,super::intelligence::software::Event::InteractionApplied{
                            context:c.instance,revision:result.revision,token:result.token});
                    }
                    if result.outcome!=Outcome::Applied || !agent.continuous || agent.remaining==0 {
                        status(if result.outcome==Outcome::Applied {b"App confirmed the action and published updated state."}else{b"App action did not complete reliably. AI control paused; no automatic retry."});
                        self.application_agent=Some(agent);self.stop_application_agent();return true;
                    }
                }
            }
            self.application_agent=Some(agent);return false;
        }
        if state.terminal {
            status(b"The app reports this interaction has ended.");
            self.application_agent=Some(agent);self.stop_application_agent();return true;
        }
        if agent.inference {
            let Some(service)=self.qwen.as_mut() else{self.application_agent=Some(agent);self.stop_application_agent();return false;};
            let mut budget=qwen::pump::PumpBudget::new(crate::ui::performance::monotonic_ns());
            let mut failed=false;
            let mut framed=false;
            while budget.next(crate::ui::performance::monotonic_ns()) {
                if service.poll().is_err(){failed=true;break;}
                if parse_proposal(service.output()).is_some_and(|p|p.arguments.is_some()) {framed=true;break;}
                if !service.busy() || service.output().len()>768 {break;}
            }
            if failed || now>=agent.deadline || service.output().len()>768 || state.revision!=agent.revision {
                status(b"AI decision interrupted, stale, or timed out. Control paused; you can continue using the app.");
                self.application_agent=Some(agent);self.stop_application_agent();return true;
            }
            if service.busy() && !framed {self.application_agent=Some(agent);return true;}
            let mut response=[0u8;768];
            let length=service.output().len();response[..length].copy_from_slice(service.output());
            let selected=if service.completed() || framed {parse_for_state(&response[..length],&state)}else{None};
            self.release_application_inference();agent.inference=false;
            if let Some(proposal)=selected.as_ref().filter(|p|state.choices.iter().flatten().any(|c|c.id==p.id && c.risk==super::application::Risk::Sensitive)) {
                let review=application_host::with_registry(|r|r.prepare_review(c.instance,c.owner,c.session,agent.revision,proposal.id,proposal.arguments,proposal.text,now));
                if let Ok(request)=review {
                    let mut message=[0u8;4096];
                    if let Some(length)=super::application_decision::review_text(&request,&mut message) {
                        let mut panel=crate::ui::app_assistant::read(1);
                        panel.reply(&message[..length]);panel.expanded=true;
                        panel.pending=crate::ui::app_assistant::Action::NativeApproval;
                        panel.argument[..8].copy_from_slice(&request.token.to_le_bytes());panel.argument_len=8;
                        crate::ui::app_assistant::write(1,panel);
                        agent.approval=Some(request.token);self.application_agent=Some(agent);return true;
                    }
                }
                status(b"This action could not be prepared for review. Nothing executed.");
                self.application_agent=Some(agent);self.stop_application_agent();return true;
            }
            let proposed=selected.ok_or(Error::Invalid).and_then(|proposal|application_host::with_registry(|r|
                r.propose_payload(c.instance,c.owner,c.session,agent.revision,proposal.id,proposal.arguments,proposal.text,false,now,now.saturating_add(10_000_000_000))));
            match proposed {
                Ok(request)=>{agent.token=Some(request.token);agent.remaining-=1;status(b"AI action sent. Waiting for the app's updated state and receipt.");self.application_agent=Some(agent);},
                Err(_)=>{
                    let mut message=[0u8;1024];
                    let prefix=b"Invalid app proposal. No action executed; AI control paused. Unaccepted model output:\n";
                    message[..prefix.len()].copy_from_slice(prefix);
                    message[prefix.len()..prefix.len()+length].copy_from_slice(&response[..length]);
                    status(&message[..prefix.len()+length]);
                    self.application_agent=Some(agent);self.stop_application_agent();
                },
            }
            return true;
        }
        if !state.agent_turn || state.revision==0 || state.revision==agent.revision || self.app_owner.is_some() || self.execution.busy() || self.intelligence_goal.is_some() || self.qwen.as_ref().is_none_or(|s|s.busy()) {
            self.application_agent=Some(agent);return false;
        }
        let mut prompt=[0u8;24576];
        let Some(length)=decision_prompt(&agent.objective[..agent.length],&state,&mut prompt) else {
            status(b"App interaction exceeds the bounded model context. AI control paused.");self.application_agent=Some(agent);self.stop_application_agent();return true;
        };
        let service=self.qwen.as_mut().unwrap();
        self.application_saved=service.suspend_conversation();
        if service.submit(&prompt[..length]).is_err() {
            self.release_application_inference();status(b"Local model could not accept this app decision. AI control paused.");
            self.application_agent=Some(agent);self.stop_application_agent();return true;
        }
        agent.inference=true;agent.revision=state.revision;agent.deadline=now.saturating_add(90_000_000_000);
        self.application_agent=Some(agent);status(b"AI is choosing an action from the app's current legal choices.");true
    }
}
// ------------------------=
// FUNC: status
// DESC: Uses the existing native app assistant panel to expose truthful interaction progress.
// ------------------=
fn status(message:&[u8]) {
    let mut panel=crate::ui::app_assistant::read(1);panel.reply(message);crate::ui::app_assistant::write(1,panel);
}
