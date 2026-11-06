//! Shared authenticated signout and one-shot login speech.
use super::*;
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: sign_out_session
    // DESC: Ends only this authenticated session, preserving saved layout and refusing to discard an unsaved editor document.
    // ------------------=
    pub(super) fn sign_out_session(&mut self)->bool {
        if self.current_session.is_zero()||self.settings_editing||!self.editor_document.is_saved()||crate::ui::terminal_edit::inspect(|s|s.active&&!s.document.is_saved()){return false;}
        if !self.persist_desktop_layout(){return false;}
        let owner=crate::runtime::execution::SecurityIdentity(self.current_session.0);
        let ended=crate::runtime::with_runtime(|r|r.identity.end_session(self.current_session,self.current_user).is_ok()).unwrap_or(false);
        if !ended{return false;}
        #[cfg(target_os="none")]
        {crate::runtime::ai::voice_conversation::stop(owner);crate::runtime::ai::voice_output::stop(owner);}
        #[cfg(not(target_os="none"))] let _=owner;
        self.accounts_cancel();self.cancel_account_password();self.cancel_node_pairing_input();
        self.current_session=crate::runtime::identity::StableId::zero();
        self.login_welcome=0;self.ai_chat_scroll_grab=None;
        self.reset_app_assistant_session();self.mode=ConsoleMode::Authentication;
        self.system_step=crate::runtime::with_runtime(|r|crate::runtime::face_login::account_selection(self.current_user,r.identity.user_count(),
            |index|r.identity.user_nth(index).map(|user|user.id))).unwrap_or(0);
        self.system_focus=0;self.reset_input();self.publish_command_terminal();true
    }
    // ------------------------=
    // FUNC: schedule_login_welcome
    // DESC: Arms the one-shot greeting only on authenticated entry, never when returning from a desktop app.
    // ------------------=
    pub(super) fn schedule_login_welcome(&mut self){
        self.login_welcome=1;self.login_welcome_retry_ns=0;self.login_welcome_request=0;
        self.login_welcome_deadline=crate::runtime::ai::qwen::workers::clock_ns().saturating_add(12_000_000_000);
    }
    // ------------------------=
    // FUNC: poll_login_welcome
    // DESC: Queues one personalized native speech job after login, respecting mute and delaying mic autostart only until playback completes.
    // ------------------=
    #[cfg(target_os="none")]
    pub(super) fn poll_login_welcome(&mut self,now:u64)->bool {
        use crate::runtime::{ai::voice_output::{self,OutputState},capability::CapabilityType,execution::SecurityIdentity};
        if self.login_welcome==0{return false;}
        let owner=SecurityIdentity(self.current_session.0);
        if self.current_session.is_zero()||matches!(self.mode,ConsoleMode::Locked|ConsoleMode::Authentication|ConsoleMode::Startup){
            self.login_welcome=0;voice_output::stop(owner);return false;
        }
        if now>self.login_welcome_deadline {
            // A greeting is optional output, never an indefinite microphone
            // startup gate. Cancel only its recorded job, not newer narration.
            if self.login_welcome_request!=0 {voice_output::stop_request(owner,self.login_welcome_request);}
            self.login_welcome=0;self.login_welcome_request=0;
            crate::output_text(b"[voice] login greeting deadline; releasing microphone startup\n");
            return false;
        }
        let state=voice_output::status().state;
        let busy=matches!(state,OutputState::Queued|OutputState::Synthesizing|OutputState::Ready|OutputState::Speaking|OutputState::Buffered);
        if self.login_welcome==2 {
            let waiting=busy && voice_output::request_id(owner)==Some(self.login_welcome_request);
            if !waiting {self.login_welcome=0;self.login_welcome_request=0;}
            return waiting;
        }
        if busy||!matches!(crate::runtime::ai::voice_conversation::state().0,crate::runtime::ai::voice_conversation::State::Off|crate::runtime::ai::voice_conversation::State::Failed){return true;}
        if now<self.login_welcome_retry_ns{return true;}
        let mut phrase=[0;160];
        let prepared=crate::runtime::with_runtime(|r|{
            if !r.identity.ai_profile(self.current_user).is_some_and(|p|p.speech_output_enabled){return None;}
            let active=(0..crate::runtime::identity::MAX_SESSIONS).filter_map(|i|r.identity.session_nth(i))
                .any(|s|s.id==self.current_session&&s.user==self.current_user&&s.state==crate::runtime::identity::SessionState::Active);
            if !active{return None;}
            let n=crate::runtime::ai::session_request::welcome(r.identity.user(self.current_user)?.display_name.as_bytes(),&mut phrase);
            let cap=r.capabilities.grant(CapabilityType::AudioOutput,0,1,0,owner,owner,
                Some(now/1_000_000_000+voice_output::OUTPUT_LEASE_SECONDS),0).ok()?;
            Some((cap,n))
        }).flatten();
        let Some((cap,n))=prepared else{self.login_welcome=0;return false;};
        match voice_output::submit(owner,cap,&phrase[..n]) {
            Ok(())=>{self.login_welcome=2;self.login_welcome_request=voice_output::request_id(owner).unwrap_or(0);true}
            Err(error)=>{
                crate::runtime::with_runtime(|r|{let _=r.capabilities.retire_leaf(cap,owner);});
                if matches!(error,crate::runtime::ai::types::AiError::ProviderUnavailable|crate::runtime::ai::types::AiError::QueueFull){
                    self.login_welcome_retry_ns=now.saturating_add(250_000_000);true
                }else{self.login_welcome=0;false}
            }
        }
    }
}
