//! Command Window advisory inference and shared typed-widget decision entrypoint.
use super::*;
use crate::runtime::ai::{self, text_request::{self, AdviceError, AdviceMode, Command, Facts, PendingReply, ReplyState}};

#[used]
#[no_mangle]
pub static mut INFINITY_LLM_ADVICE_SNAPSHOT: [u64;24] = [0;24];
struct AdviceTrace { owner: [u8;16], session: [u8;16], origin: u64, mode: AdviceMode, turn: u64, accepted: u64 }
static mut ADVICE_TRACE: AdviceTrace = AdviceTrace { owner:[0;16], session:[0;16], origin:0,
    mode:AdviceMode::Answer, turn:0, accepted:0 };

// ------------------------=
// FUNC: advice_error
// DESC: Presents typed admission failures without manufacturing a model response.
// ------------------=
fn advice_error(error: AdviceError) -> &'static [u8] {
    text_request::error_message(error)
}

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: record_advice_submission
    // DESC: Retains content-free typed-request provenance on the serialized UI thread for installed behavioral inspection.
    // ------------------=
    fn record_advice_submission(&self, origin: u64, mode: AdviceMode, turn: u64) {
        unsafe {
            let trace=&mut *(&raw mut ADVICE_TRACE);
            let accepted=if trace.owner==self.current_user.0 && trace.session==self.current_session.0 {trace.accepted.saturating_add(1)}else{1};
            *trace=AdviceTrace {owner:self.current_user.0,session:self.current_session.0,origin,mode,turn,accepted};
        }
    }

    // ------------------------=
    // FUNC: publish_llm_snapshot
    // DESC: Publishes seqlocked generation, ownership, native-token, terminal-delivery and structured-action-audit counters without prompts or response content.
    // ------------------=
    fn publish_llm_snapshot(&self) {
        let mut values=[0u64;24];values[0]=0x494e464c4c4d3031;values[1]=1;
        let allowed=self.workplace_authorized(false)&&self.ai_chat_allowed();
        values[3]=u64::from(allowed);
        if allowed {
            unsafe {
                let trace=&*(&raw const ADVICE_TRACE);
                if trace.owner==self.current_user.0 && trace.session==self.current_session.0 {
                    values[4]=trace.origin;values[5]=if trace.mode==AdviceMode::Decide {2}else{1};
                    values[6]=trace.turn;values[7]=trace.accepted;
                }
            }
            ai::with_ai_runtime(|runtime| {
                values[8]=runtime.chat.turn_id();values[9]=runtime.chat.generation_state as u64;
                values[10]=text_request::response(&runtime.chat).map_or(0,|bytes|bytes.len() as u64);
                values[11]=runtime.qwen_tokens;values[12]=runtime.chat.selected_model() as u64;
                values[13]=u64::from(runtime.native_ready(runtime.chat.selected_model()));
                values[14]=u64::from(runtime.control_pending());values[15]=runtime.control_audit_count() as u64;
                let mut fingerprint=0xcbf29ce484222325u64;
                for index in 0..runtime.control_audit_count() {
                    if let Some(record)=runtime.control_audit(index) {
                        fingerprint=(fingerprint^record.request.correlation_id).wrapping_mul(0x100000001b3);
                        use ai::kit::{AuditOutcome,PolicyDenial};
                        let outcome=match record.outcome {
                            AuditOutcome::Staged=>1,AuditOutcome::Authorized=>2,
                            AuditOutcome::Denied(PolicyDenial::OwnerChanged)=>0x301,
                            AuditOutcome::Denied(PolicyDenial::PermissionDenied)=>0x302,
                            AuditOutcome::Denied(PolicyDenial::AiDisabled)=>0x303,
                            AuditOutcome::Completed=>4,AuditOutcome::Failed=>5,
                            AuditOutcome::Deferred=>6,AuditOutcome::Cancelled=>7,
                        };
                        fingerprint=(fingerprint^outcome).wrapping_mul(0x100000001b3);
                    }
                }
                values[16]=fingerprint;values[22]=u64::from(runtime.chat.selected_model_ready());
            });
            if let Some(reply)=self.llm_reply.filter(|reply|reply.owner==self.current_user.0 && reply.session==self.current_session.0) {
                values[17]=reply.turn;values[18]=self.llm_delivered_turn.unwrap_or(0);
                values[19]=u64::from(reply.terminal);values[21]=1;
            }
            values[20]=crate::ui::command_terminal::inspect(|state|state.count as u64);
        }
        unsafe {
            let target=core::ptr::addr_of_mut!(INFINITY_LLM_ADVICE_SNAPSHOT).cast::<u64>();
            let sequence=target.add(2).read_volatile().wrapping_add(2)&!1;
            target.add(2).write_volatile(sequence|1);target.add(23).write_volatile(sequence|1);
            for (index,value) in values.iter().enumerate() {if index!=2&&index!=23 {target.add(index).write_volatile(*value);}}
            core::sync::atomic::fence(core::sync::atomic::Ordering::Release);
            target.add(23).write_volatile(sequence);target.add(2).write_volatile(sequence);
        }
    }

    // ------------------------=
    // FUNC: advice_facts
    // DESC: Collects measured native telemetry without reading files or claiming unavailable external knowledge.
    // ------------------=
    fn advice_facts(&self) -> Option<Facts> {
        crate::runtime::with_runtime(|runtime| {
            let sample = runtime.task_manager.system;
            Facts { sampled_at_ns: sample.timestamp_ns, ram_total: sample.ram_total,
                ram_used: sample.ram_used, cores: sample.cores, cpu_percent: sample.cpu_percent,
                process_count: runtime.execution.count().min(u16::MAX as usize) as u16,
                service_count: runtime.services.count().min(u16::MAX as usize) as u16 }
        })
    }

    // ------------------------=
    // FUNC: llm_command
    // DESC: Submits explicit local advisory prompts and binds terminal delivery to the authenticated requesting session.
    // ------------------=
    pub(super) fn llm_command(&mut self, input: &[u8]) -> bool {
        let Some(command) = text_request::parse_command(input) else { return false; };
        if command==Command::Help {
            self.output.write_line(b"ai ask PROMPT | llm PROMPT     Ask the selected local language model.");
            self.output.write_line(b"ai decide QUESTION            Compare supplied facts and priorities.");
            self.output.write_line(b"Example: ai decide weights=80,20 scores=9,4/4,9");
            self.output.write_line(b"Use 1-8 integer weights totaling 100; 2-8 options; scores 0-1000, higher is better.");
            self.output.write_line(b"Optional eligible=1,0 excludes option 2. Ties stay tied; no eligible option is reported.");
            self.output.write_line(b"ai response                   Show this session's current or partial response.");
            self.output.write_line(b"ai cancel                     Cancel this session's active typed request.");
            self.output.write_line(b"ai new                        Clear this session's idle typed conversation.");
            self.output.write_line(b"Use llm ask/decide/response/cancel/new/help as aliases. Replies appear here when complete.");
            return true;
        }
        if command==Command::Invalid {self.output.write_line(advice_error(AdviceError::InvalidCommand));return true;}
        if !self.workplace_authorized(false) || !self.ai_chat_allowed() {
            self.output.write_line(advice_error(AdviceError::Denied));
            return true;
        }
        match command {
            Command::Help|Command::Invalid => {},
            Command::Prompt(mode, prompt) => {
                let facts = self.advice_facts();
                let result = ai::with_ai_runtime(|runtime|
                    runtime.submit_text_advice(self.current_user.0, prompt, mode, facts, true));
                match result {
                    Ok(turn) => {
                        self.record_advice_submission(1,if text_request::decision_question(prompt) {AdviceMode::Decide}else{mode},turn);
                        self.llm_delivered_turn = None;
                        self.llm_reply = Some(PendingReply { owner: self.current_user.0, session: self.current_session.0,
                            turn, terminal: self.command_terminal_visible() });
                        self.output.write_number(b"Local AI request: ", turn);
                        self.output.write_line(b"Generating a response. You can keep using Command Window.");
                    }
                    Err(error) => self.output.write_line(advice_error(error)),
                }
            }
            Command::Response => self.print_llm_response(),
            Command::New => {
                let turn=unsafe {
                    let trace=&*(&raw const ADVICE_TRACE);
                    (trace.owner==self.current_user.0 && trace.session==self.current_session.0
                        && trace.accepted!=0).then_some(trace.turn)
                };
                let result=turn.ok_or(AdviceError::NotOwned).and_then(|turn|ai::with_ai_runtime(|runtime|
                    runtime.reset_text_advice(self.current_user.0,turn)));
                match result {
                    Ok(())=>{
                        self.llm_reply=None;self.llm_delivered_turn=None;
                        self.output.write_line(b"Started a fresh typed conversation. Your unsent widget draft and saved memory are preserved.");
                    }
                    Err(error)=>self.output.write_line(advice_error(error)),
                }
            }
            Command::Cancel => {
                let owned = self.llm_reply.is_some_and(|reply| ai::with_ai_runtime(|runtime|
                    reply.state(self.current_user.0, self.current_session.0, true, &runtime.chat) == ReplyState::Waiting));
                if owned { ai::with_ai_runtime(|runtime| { runtime.cancel_chat(); }); }
                self.output.write_line(if owned { b"Cancellation requested." } else { b"This command session has no active AI request." });
            }
        }
        true
    }

    // ------------------------=
    // FUNC: print_llm_response
    // DESC: Prints only a reply owned by this command session, preserving all model bytes across bounded terminal rows.
    // ------------------=
    fn print_llm_response(&mut self) {
        let Some(reply) = self.llm_reply else {
            self.output.write_line(b"No Command Window AI request has been submitted in this session."); return;
        };
        ai::with_ai_runtime(|runtime| {
            match reply.state(self.current_user.0, self.current_session.0, self.ai_chat_allowed(), &runtime.chat) {
                ReplyState::Revoked => {self.output.write_line(advice_error(AdviceError::NotOwned));return;},
                ReplyState::Superseded => { self.output.write_line(b"A newer AI turn has replaced this request. Its earlier reply remains in the AI widget history."); return; }
                _ => {}
            }
            if let Some(bytes) = text_request::response(&runtime.chat) {
                for line in bytes.split(|byte| *byte == b'\n') {
                    if line.is_empty() { self.output.write_line(b""); }
                    else { for chunk in line.chunks(LINE_CAPACITY) { self.output.write_line(chunk); } }
                }
            }
            use ai::chat::GenerationState;
            match runtime.chat.generation_state {
                GenerationState::Running => self.output.write_line(b"Response is still generating."),
                GenerationState::Cancelled => self.output.write_line(b"AI request cancelled; any response above is partial."),
                GenerationState::Failed => self.output.write_line(b"Local AI generation failed; any response above is partial."),
                GenerationState::ContextFull => self.output.write_line(b"Local model context is full; any response above is partial. Use ai new before retrying."),
                _ => {}
            }
        });
    }

    // ------------------------=
    // FUNC: poll_llm_response
    // DESC: Delivers a terminal request once after completion, with authorization and turn checks independent of foreground focus.
    // ------------------=
    pub(super) fn poll_llm_response(&mut self) -> bool {
        self.publish_llm_snapshot();
        let Some(reply) = self.llm_reply else { return false; };
        if self.llm_delivered_turn == Some(reply.turn) { return false; }
        let allowed = self.workplace_authorized(false) && self.ai_chat_allowed();
        let state = ai::with_ai_runtime(|runtime|
            reply.state(self.current_user.0, self.current_session.0, allowed, &runtime.chat));
        match state {
            ReplyState::Waiting => false,
            ReplyState::Revoked => { self.llm_reply = None; self.llm_delivered_turn = None; false }
            ReplyState::Superseded => { self.llm_delivered_turn = Some(reply.turn); false }
            ReplyState::Ready(_) => {
                let _capture = crate::ui::command_terminal::Capture::new(reply.terminal);
                self.output.write_number(b"AI response for request ", reply.turn);
                self.print_llm_response();
                self.llm_delivered_turn = Some(reply.turn);
                true
            }
        }
    }

    // ------------------------=
    // FUNC: submit_decision_widget_input
    // DESC: Routes typed decision questions through the same evidence-aware advisory endpoint as Command Window, without changing ordinary or spoken action dispatch.
    // ------------------=
    pub(super) fn submit_decision_widget_input(&mut self) -> bool {
        let mut input = [0u8; ai::chat::CHAT_INPUT_CAPACITY];
        let length = ai::with_ai_runtime(|runtime| {
            if !text_request::decision_question(runtime.chat.input()) { return 0; }
            let length = runtime.chat.input().len();
            input[..length].copy_from_slice(runtime.chat.input()); length
        });
        if length == 0 { return false; }
        let facts = self.advice_facts();
        let result = ai::with_ai_runtime(|runtime|
            runtime.submit_text_advice(self.current_user.0, &input[..length], AdviceMode::Decide, facts, false));
        if let Ok(turn)=result {self.record_advice_submission(2,AdviceMode::Decide,turn);}
        if let Err(error) = result {
            ai::with_ai_runtime(|runtime|runtime.chat.set_advice_notice(error));
        }
        true
    }
}
