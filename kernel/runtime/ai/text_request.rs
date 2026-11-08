//! Typed advisory requests and response ownership. Voice ingress is unchanged.
use super::{chat::{ChatRole, ChatRuntime, GenerationState}, AiRuntime};
use core::fmt::Write;
#[path="decision_facts.rs"] pub mod weighted;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdviceMode { Answer, Decide }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdviceError { Denied, Busy, Unavailable, InvalidInput, InvalidDecision, ContextFull, Failed }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command<'a> { Prompt(AdviceMode, &'a [u8]), Response, Cancel, Help }

// ------------------------=
// FUNC: parse_command
// DESC: Parses only complete advisory command words; the prompt remains uninterpreted user data.
// ------------------=
pub fn parse_command(input: &[u8]) -> Option<Command<'_>> {
    let input = input.trim_ascii();
    if matches!(input, b"ai response" | b"llm response") { return Some(Command::Response); }
    if matches!(input, b"ai cancel" | b"llm cancel") { return Some(Command::Cancel); }
    if matches!(input, b"ai ask" | b"ai decide" | b"llm" | b"llm help" | b"ai help") { return Some(Command::Help); }
    for (prefix, mode) in [(b"ai ask ".as_slice(), AdviceMode::Answer),
        (b"ai decide ".as_slice(), AdviceMode::Decide), (b"llm decide ".as_slice(), AdviceMode::Decide),
        (b"llm ".as_slice(), AdviceMode::Answer)] {
        if let Some(prompt) = input.strip_prefix(prefix) {
            let prompt = prompt.trim_ascii();
            return Some(if prompt.is_empty() { Command::Help } else { Command::Prompt(mode, prompt) });
        }
    }
    None
}

// ------------------------=
// FUNC: decision_question
// DESC: Recognizes explicit decision advice and common comparative questions without reclassifying ordinary OS action requests.
// ------------------=
pub fn decision_question(input: &[u8]) -> bool {
    let text = input.trim_ascii();
    [b"decide ".as_slice(), b"help me decide ", b"should i ", b"which should ",
        b"which is better", b"which option ", b"choose between ", b"choose the option ", b"what is the best ", b"weigh ", b"compare "]
        .iter().any(|prefix| text.get(..prefix.len()).is_some_and(|s| s.eq_ignore_ascii_case(prefix)))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub sampled_at_ns: u64,
    pub ram_total: u64,
    pub ram_used: u64,
    pub cores: u16,
    pub cpu_percent: u8,
    pub process_count: u16,
    pub service_count: u16,
}
struct Prompt<'a> { bytes: &'a mut [u8], length: usize }
impl Write for Prompt<'_> {
    // ------------------------=
    // FUNC: write_str
    // DESC: Rejects oversized evidence instead of silently removing facts or constraints from a decision.
    // ------------------=
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        if text.len() > self.bytes.len() - self.length { return Err(core::fmt::Error); }
        self.bytes[self.length..self.length + text.len()].copy_from_slice(text.as_bytes());
        self.length += text.len();
        Ok(())
    }
}

// ------------------------=
// FUNC: prepare_prompt
// DESC: Labels measured facts, uncertainty, and weighted comparison guidance without granting any OS action authority.
// ------------------=
pub fn prepare_prompt(mode: AdviceMode, input: &[u8], facts: Option<Facts>, out: &mut [u8]) -> Result<usize, AdviceError> {
    if input.trim_ascii().is_empty() || input.len() > super::chat::CHAT_INPUT_CAPACITY
        || input.iter().any(|b| !(32..=126).contains(b) && !matches!(b, b'\n' | b'\t')) {
        return Err(AdviceError::InvalidInput);
    }
    let mut prompt = Prompt { bytes: out, length: 0 };
    prompt.write_str("Answer as an adviser. Do not execute actions or emit OS tool records. Distinguish supplied facts, remembered facts, assumptions, and unknowns. Do not claim access to files, web results, or current information that was not supplied. ").map_err(|_| AdviceError::ContextFull)?;
    if mode == AdviceMode::Decide {
        prompt.write_str("Compare realistic options against the user's constraints and priorities. If the user supplies weights and scores, calculate each weighted sum and check the arithmetic. Otherwise state qualitative priorities; do not invent numeric precision. Recommend one option with a concise evidence-based rationale, principal tradeoff, uncertainty, and what missing fact could change the choice. If a necessary fact is missing, ask a focused question. This is advice, not authorization to act. ").map_err(|_| AdviceError::ContextFull)?;
    }
    if let Some(facts) = facts {
        write!(prompt, "\nOS telemetry sampled at monotonic {} ns: {} registered execution contexts (including retained stopped records), {} kernel service records. ",
            facts.sampled_at_ns, facts.process_count, facts.service_count).map_err(|_| AdviceError::ContextFull)?;
        if facts.sampled_at_ns != 0 && facts.ram_total != 0 {
            write!(prompt, "RAM used/total bytes: {}/{}; CPU utilization: {} percent across {} cores. ",
                facts.ram_used, facts.ram_total, facts.cpu_percent, facts.cores).map_err(|_| AdviceError::ContextFull)?;
        } else {
            prompt.write_str("RAM and CPU utilization measurements are unavailable. ").map_err(|_| AdviceError::ContextFull)?;
        }
    }
    if mode == AdviceMode::Decide {
        if let Some(ranking) = weighted::rank(input).map_err(|_| AdviceError::InvalidDecision)? {
            prompt.write_str("\nVerified weighted arithmetic (each weighted sum is divided by 100; larger is better): ").map_err(|_| AdviceError::ContextFull)?;
            for index in 0..ranking.count {
                write!(prompt, "option {} = {}/100, eligible={}; ", index+1, ranking.scores[index],
                    u8::from(ranking.eligible&(1<<index)!=0)).map_err(|_| AdviceError::ContextFull)?;
            }
            if let Some(winner)=ranking.winner() {
                write!(prompt, "The unique highest-scoring eligible option is {}. ",winner).map_err(|_| AdviceError::ContextFull)?;
            } else {
                prompt.write_str("The best eligible options are tied; state the tie and ask for a tie-breaking priority. ").map_err(|_| AdviceError::ContextFull)?;
            }
        }
    }
    prompt.write_str("\nUser question and supplied information:\n").map_err(|_| AdviceError::ContextFull)?;
    prompt.write_str(core::str::from_utf8(input).map_err(|_| AdviceError::InvalidInput)?).map_err(|_| AdviceError::ContextFull)?;
    Ok(prompt.length)
}

impl AiRuntime {
    // ------------------------=
    // FUNC: submit_text_advice
    // DESC: Admits authenticated typed advice to the real local model with no execution capability, preserving the existing audio ingress and polling path.
    // ------------------=
    pub fn submit_text_advice(&mut self, owner: [u8; 16], input: &[u8], mode: AdviceMode,
        facts: Option<Facts>, preserve_composer: bool) -> Result<u64, AdviceError> {
        if owner == [0;16] || owner != self.chat_owner || !self.chat.enabled() { return Err(AdviceError::Denied); }
        if self.execution.busy() || self.app_owner.is_some() || self.application_agent.is_some()
            || self.intelligence_goal.is_some() || self.chat.generation_state == GenerationState::Running
            || self.qwen.as_ref().is_some_and(|service| service.busy()) { return Err(AdviceError::Busy); }
        if !self.native_ready(self.chat.selected_model()) { return Err(AdviceError::Unavailable); }
        let mode = if decision_question(input) { AdviceMode::Decide } else { mode };
        let mut prompt = [0u8; 6144];
        let length = prepare_prompt(mode, input, facts, &mut prompt)?;
        let mut context = [0u8;1536];
        let context_length = self.hermes_memory.as_ref().map_or(Ok(0), |profile|
            profile.selected_context(input, &mut context)).map_err(|_| AdviceError::ContextFull)?;
        if self.chat.selected_model() != self.active_native {
            core::mem::swap(&mut self.qwen, &mut self.other_native);
            self.active_native = self.chat.selected_model();
        }
        let submitted_ns = crate::ui::performance::monotonic_ns();
        let service = self.qwen.as_mut().ok_or(AdviceError::Unavailable)?;
        service.submit_personal(&prompt[..length], &context[..context_length], None).map_err(|error|
            if error == super::qwen::gguf::Error::Overflow { AdviceError::ContextFull } else { AdviceError::Failed })?;
        let request = self.requests.accept(input, owner, super::kit::RequestOrigin::DesktopChat);
        self.inference_request = super::kit::InferenceRequest::admit(&request,
            super::kit::RequestRoute::Inference(super::kit::InferenceMode::Conversation));
        self.deferred_tool_catalog = false;
        self.memory_inquiries.cancel();
        self.qwen_metrics.begin(submitted_ns, service.reused_tokens, service.prefill_tokens);
        self.qwen_tokens = 0;
        self.qwen_decode_ns = 0;
        self.qwen_first_token_ns = None;
        self.chat_deadline_ns = submitted_ns.map(|now| now.saturating_add(120_000_000_000));
        let mut draft = [0u8; super::chat::CHAT_INPUT_CAPACITY];
        let draft_length = self.chat.input().len();
        let draft_cursor = self.chat.input_cursor();
        if preserve_composer { draft[..draft_length].copy_from_slice(self.chat.input()); }
        self.chat.clear_input();
        for &byte in input { self.chat.push_input(if matches!(byte, b'\n' | b'\t') { b' ' } else { byte }); }
        self.chat.begin_native_turn();
        self.chat.generation_state = GenerationState::Running;
        if preserve_composer {
            for &byte in &draft[..draft_length] { self.chat.push_input(byte); }
            self.chat.set_input_cursor(draft_cursor);
        }
        Ok(self.chat.turn_id())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingReply { pub owner: [u8;16], pub session: [u8;16], pub turn: u64, pub terminal: bool }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplyState { Waiting, Ready(GenerationState), Revoked, Superseded }
impl PendingReply {
    // ------------------------=
    // FUNC: state
    // DESC: Binds delayed output to its exact initiating session and turn, preventing voice or later widget replies from being copied into the terminal.
    // ------------------=
    pub fn state(self, owner: [u8;16], session: [u8;16], allowed: bool, chat: &ChatRuntime) -> ReplyState {
        if !allowed || self.owner != owner || self.session != session || session == [0;16] { return ReplyState::Revoked; }
        if self.turn != chat.turn_id() { return ReplyState::Superseded; }
        match chat.generation_state {
            GenerationState::Running | GenerationState::Ready => ReplyState::Waiting,
            state => ReplyState::Ready(state),
        }
    }
}

// ------------------------=
// FUNC: response
// DESC: Exposes only an assistant message from the current turn, never an earlier answer while the model is still prefilling.
// ------------------=
pub fn response(chat: &ChatRuntime) -> Option<&[u8]> {
    chat.message_count().checked_sub(1).and_then(|index| chat.message(index))
        .filter(|message| message.role == ChatRole::Assistant).map(|message| message.text())
}
