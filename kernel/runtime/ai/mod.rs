pub mod application;
pub mod application_host;
pub mod application_agent;
pub mod application_decision;
pub mod agent;
pub mod broker;
pub mod chat;
pub mod control;
pub mod generation;
pub mod intelligence;
pub mod intent;
pub mod kit;
pub mod mapping;
pub mod memory;
pub mod model;
pub mod provider;
pub mod qwen;
pub mod types;
pub mod voice;
pub mod voice_pcm;
#[cfg(target_os = "none")]
pub mod voice_output;
#[cfg(target_os = "none")]
pub mod voice_input;
#[cfg(target_os = "none")]
pub mod voice_conversation;
pub mod wake_word;
pub mod voice_vad;

use agent::AgentManager;
use broker::{ContextBroker, CONTEXT_SYSTEM_STATE};
use chat::ChatRuntime;
use generation::{CREATIVE_MODEL_ID, DIALOGUE_MODEL_ID};
use intent::{ConsequencePolicy, IntentPlan};
use kit::{ExecutionBroker, ExecutionRequest, InferenceMode, IntentRouter, PolicyContext, RequestApi, RequestOrigin, RequestRoute};
use model::{
    conversation_model_descriptor, local_model_descriptor, LocalCpuBackend, ModelRegistry,
    LOCAL_INTENT_MODEL_ID,
};
use provider::local_provider;
use types::*;
use voice::VoiceService;

struct ProgressMessage<'a> { bytes: &'a mut [u8], length: usize }
impl core::fmt::Write for ProgressMessage<'_> {
    // ------------------------=
    // FUNC: write_str
    // DESC: Formats a bounded content-free progress label without allocation.
    // ------------------=
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        if text.len() > self.bytes.len()-self.length {return Err(core::fmt::Error);}
        self.bytes[self.length..self.length+text.len()].copy_from_slice(text.as_bytes());
        self.length+=text.len();Ok(())
    }
}

struct NativePlanExecutor<'a> {
    broker: &'a mut ExecutionBroker,
    active: &'a mut Option<(u64, control::Command)>,
    result: &'a mut Option<(u64, bool)>,
    software: &'a mut Option<intelligence::software::Execution>,
    origin: RequestOrigin,
}

impl intelligence::Executor for NativePlanExecutor<'_> {
    // ------------------------=
    // FUNC: authorize
    // DESC: Performs sidecar admission while preserving the broker's final session authorization.
    // ------------------=
    fn authorize(&self,owner:[u8;16],capability:intelligence::CapabilityDescriptor,_now:u64)->bool {
        owner != [0;16] && capability.availability == intelligence::Availability::Ready
    }

    // ------------------------=
    // FUNC: start
    // DESC: Converts one installed action into the existing exact-once native execution broker.
    // ------------------=
    fn start(&mut self,goal:&intelligence::Goal,task:&intelligence::Task,_:intelligence::CapabilityDescriptor,_now:u64)->Result<u64,intelligence::StartError>{
        if self.active.is_some() || self.broker.busy(){return Err(intelligence::StartError::Busy);}
        let token=goal.id.rotate_left(17)^task.id as u64;
        if matches!(task.capability, intelligence::CapabilityId::UiServices | intelligence::CapabilityId::ApplicationInteraction) {
            let execution=self.software.as_mut().filter(|execution|execution.goal==goal.id && execution.owner==goal.owner)
                .ok_or(intelligence::StartError::Unavailable)?;
            return if execution.begin(token,task.capability) {Ok(token)} else {Err(intelligence::StartError::Busy)};
        }
        let command=match task.capability {
            intelligence::CapabilityId::AiAction(index)=>mapping::CAPABILITY_TARGETS.get(index as usize).and_then(|target|control::Command::decode(target)),
            intelligence::CapabilityId::DevKit=>control::ContextRequest::parse(goal.objective.as_bytes()).map(control::Command::Context),
            _=>None,
        }.ok_or(intelligence::StartError::Unavailable)?;
        self.broker.stage(ExecutionRequest{command,owner:goal.owner,origin:self.origin,correlation_id:token}).map_err(|_|intelligence::StartError::Busy)?;
        if task.capability==intelligence::CapabilityId::DevKit {
            let mut execution=intelligence::software::Execution::new(goal.id,goal.owner);
            execution.begin(token,task.capability);
            *self.software=Some(execution);
        }
        *self.active=Some((token,command));Ok(token)
    }

    // ------------------------=
    // FUNC: observe
    // DESC: Requires workflow receipts for software tasks; dispatcher admission alone never verifies generated software.
    // ------------------=
    fn observe(&mut self,token:u64,_now:u64)->intelligence::Observation{
        if let Some(execution)=self.software.as_mut().filter(|execution|execution.owns(token)) {
            if self.result.is_some_and(|result|result.0==token) {
                let success=self.result.take().unwrap().1;
                *self.active=None;
                if !success {execution.record(execution.goal,execution.owner,intelligence::software::Event::Failed{code:1});}
            }
            let observation=execution.observe(token);
            if !matches!(observation,intelligence::Observation::Running) {*self.active=None;}
            return observation;
        }
        if self.result.is_some_and(|result|result.0==token){let success=self.result.take().unwrap().1;*self.active=None;if success{let mut evidence=[0;16];evidence[..8].copy_from_slice(&token.to_le_bytes());return intelligence::Observation::Succeeded(evidence);}return intelligence::Observation::Failed(1);}intelligence::Observation::Running
    }

    // ------------------------=
    // FUNC: cancel
    // DESC: Cancels the exact broker request owned by an interrupted plan task.
    // ------------------=
    fn cancel(&mut self,token:u64){
        if let Some(execution)=self.software.as_mut(){execution.cancel(token);}
        if self.active.is_some_and(|active|active.0==token){self.broker.cancel();*self.active=None;*self.result=None;}
    }
}

pub struct AiRuntime {
    pub models: ModelRegistry,
    pub providers: kit::ProviderRegistry,
    pub voice: VoiceService,
    pub agents: AgentManager,
    pub chat: ChatRuntime,
    cpu: LocalCpuBackend,
    qwen: Option<qwen::service::Service>,
    other_native: Option<qwen::service::Service>,
    active_native: ModelId,
    app_owner: Option<usize>,
    app_workflow_goal: Option<u64>,
    app_saved: Option<qwen::service::Conversation>,
    application_agent: Option<application_agent::Agent>,
    application_saved: Option<qwen::service::Conversation>,
    app_started_ns: Option<u64>,
    app_progress_ns: Option<u64>,
    app_progress_tokens: usize,
    chat_owner: [u8; 16],
    requests: RequestApi,
    execution: ExecutionBroker,
    intelligence: intelligence::Service,
    intelligence_goal: Option<u64>,
    intelligence_execution: Option<(u64, control::Command)>,
    intelligence_result: Option<(u64, bool)>,
    software_execution: Option<intelligence::software::Execution>,
    intelligence_summary: Option<intelligence::PlanSummary>,
    intelligence_checkpoint_pending: bool,
    instruction_mapping: mapping::ConfiguredMap,
    pub qwen_tokens: u64,
    pub qwen_decode_ns: u64,
    pub qwen_metrics: qwen::metrics::Metrics,
    qwen_first_token_ns: Option<u64>,
    initialized: bool,
    inference_count: u64,
    inference_failures: u64,
    last_result: Option<ModelExecutionResult>,
    last_plan: Option<IntentPlan>,
}

impl AiRuntime {
    // ------------------------=
    // FUNC: route_request
    // DESC: Sends every user request through the same intent engine and stages native actions with caller provenance.
    // ------------------=
    fn route_request(
        &mut self,
        input: &[u8],
        origin: RequestOrigin,
    ) -> Result<RequestRoute, &'static [u8]> {
        if !self.chat.enabled() {
            return Err(b"AI is disabled in Settings.");
        }
        if self.app_owner.is_some()
            || self.execution.busy()
            || self.qwen.as_ref().is_some_and(|s| s.busy())
        {
            return Err(b"The local model is busy. Wait for the current reply, then retry.");
        }
        let request = self.requests.accept(input, self.chat_owner, origin);
        let mut route = IntentRouter::route(&mut self.instruction_mapping, &request);
        let software_goal=crate::ui::app_assistant::editor_compile_intent::software_workflow(input);
        if let (RequestRoute::Control(command), false) = (route, software_goal) {
            self.execution
                .stage(ExecutionRequest {
                    command,
                    owner: request.owner,
                    origin: request.origin,
                    correlation_id: request.correlation_id,
                })
                .map_err(|_| b"An OS action is already pending.".as_slice())?;
        } else if intelligence::Service::classify(input, false)
            == intelligence::RequestClass::ComplexGoal
        {
            let goal_origin = match origin {
                RequestOrigin::Voice => intelligence::RequestOrigin::Voice,
                RequestOrigin::DesktopChat => intelligence::RequestOrigin::Text,
                RequestOrigin::Application(id) => intelligence::RequestOrigin::Application(id),
                RequestOrigin::Model => intelligence::RequestOrigin::NativeService,
            };
            let limits = intelligence::ExecutionLimits {
                maximum_tasks: 8,
                maximum_parallel: 2,
                maximum_replans: 1,
                deadline: crate::ui::performance::monotonic_ns()
                    .unwrap_or(0)
                    .saturating_add(if software_goal {1_200_000_000_000} else {300_000_000_000}),
            };
            let id = self
                .intelligence
                .propose(
                    request.owner,
                    goal_origin,
                    input,
                    &intelligence::InstalledCapabilities,
                    crate::ui::performance::monotonic_ns().unwrap_or(0),
                    limits,
                )
                .map_err(|_| {
                    b"The objective could not be converted into a safe executable plan.".as_slice()
                })?;
            self.intelligence
                .begin(id, request.owner, &intelligence::InstalledCapabilities)
                .map_err(|_| b"The plan could not be started safely.".as_slice())?;
            self.intelligence_goal = Some(id);
            self.intelligence_summary = None;
            self.intelligence_checkpoint_pending = true;
            route = RequestRoute::Goal(id);
        }
        Ok(route)
    }

    // ------------------------=
    // FUNC: route_application_request
    // DESC: Routes raw app input before document context is constructed; context cannot become command authority.
    // ------------------=
    pub fn route_application_request(
        &mut self,
        id: usize,
        input: &[u8],
    ) -> Result<RequestRoute, &'static [u8]> {
        if id >= 13 {
            return Err(b"Unknown application context.");
        }
        if input.is_empty() { return Err(b"An application request is required."); }
        self.stop_application_agent();
        self.cancel_app_turn();
        self.interrupt_active_intelligence();
        self.route_request(input, RequestOrigin::Application(id))
    }

    // ------------------------=
    // FUNC: pending_control_origin
    // DESC: Allows native dispatch to reject actions whose original application is no longer focused.
    // ------------------=
    pub fn pending_control_origin(&self) -> Option<RequestOrigin> { self.execution.pending_origin() }

    // ------------------------=
    // FUNC: acceptance_native_output
    // DESC: Exposes decoded bytes only in host acceptance harnesses for failed protocol diagnostics, never in the installed OS.
    // ------------------=
    #[cfg(not(target_os = "none"))]
    pub fn acceptance_native_output(&self) -> &[u8] {
        self.qwen.as_ref().map(|service| service.output()).unwrap_or(b"")
    }

    // ------------------------=
    // FUNC: acceptance_native_progress
    // DESC: Exposes completed token work to host behavioral tests without model text or guest memory access.
    // ------------------=
    #[cfg(not(target_os = "none"))]
    pub fn acceptance_native_progress(&self) -> usize {
        self.qwen.as_ref().map_or(0, |service| service.progress_tokens())
    }

    // ------------------------=
    // FUNC: submit_app_turn
    // DESC: Borrows the loaded model exclusively for isolated app inference without changing desktop conversation messages.
    // ------------------=
    pub fn submit_app_turn(&mut self, id: usize, prompt: &[u8]) -> Result<(), &'static [u8]> {
        self.submit_app_request(id,prompt,qwen::service::SubmissionKind::Conversation)
    }

    // ------------------------=
    // FUNC: submit_app_source
    // DESC: Reserves a useful bounded source-generation budget while reusing the same authorized application inference lease.
    // ------------------=
    pub fn submit_app_source(&mut self,id:usize,prompt:&[u8])->Result<(),&'static [u8]> {
        self.submit_app_request(id,prompt,qwen::service::SubmissionKind::ApplicationSource)
    }

    // ------------------------=
    // FUNC: submit_app_request
    // DESC: Shares app-model admission, context restoration and cancellation across conversational and typed source requests.
    // ------------------=
    fn submit_app_request(&mut self,id:usize,prompt:&[u8],kind:qwen::service::SubmissionKind)->Result<(),&'static [u8]> {
        if !self.chat.enabled() {
            return Err(b"AI is disabled in Settings.");
        }
        let workflow_goal=crate::ui::app_assistant::read(id).workflow_goal;
        if !self.generation_workflow_current(workflow_goal) {
            return Err(b"This application request belongs to an interrupted goal.");
        }
        if self.app_owner.is_some()
            || self.execution.busy()
            || self.qwen.as_ref().is_some_and(|s| s.busy())
        {
            return Err(b"The local model is busy. Wait for the current reply, then retry.");
        }
        let selected = self.chat.selected_model();
        if matches!(selected, chat::HERMES_MODEL_ID | chat::MINISTRAL_MODEL_ID) && selected != self.active_native {
            if !self.native_ready(selected) { return Err(b"The selected model is still loading or is not installed."); }
            core::mem::swap(&mut self.qwen, &mut self.other_native);
            self.active_native = selected;
        }
        let Some(service) = self.qwen.as_mut() else {
            return Err(b"The local language model is still loading or is not installed.");
        };
        self.app_saved = service.suspend_conversation();
        if service.submit_with_kind(prompt,kind).is_err() {
            if let Some(saved) = self.app_saved.take() {
                service.restore_conversation(saved);
            }
            return Err(if kind==qwen::service::SubmissionKind::ApplicationSource {
                b"This source request leaves insufficient local context for a complete rewrite. Reduce the application scope or source size and retry."
            } else {b"This request exceeds the local model context. Select a smaller passage and retry."});
        }
        self.app_owner = Some(id);
        self.app_workflow_goal = workflow_goal;
        let now = crate::ui::performance::monotonic_ns();
        self.app_started_ns = now;
        self.app_progress_ns = now;
        self.app_progress_tokens = service.progress_tokens();
        Ok(())
    }

    // ------------------------=
    // FUNC: app_turn_owner
    // DESC: Exposes only the window lease so the console can cancel on focus or permission changes.
    // ------------------=
    pub fn app_turn_owner(&self) -> Option<usize> {
        self.app_owner
    }
    // ------------------------=
    // FUNC: app_generation_progress
    // DESC: Exposes content-free progress for the current isolated application-generation lease.
    // ------------------=
    pub fn app_generation_progress(&self) -> Option<qwen::service::InferenceProgress> {
        self.app_owner.and_then(|_| self.qwen.as_ref().map(|service| service.progress()))
    }

    // ------------------------=
    // FUNC: cancel_app_turn
    // DESC: Releases app inference and clears private document tokens before another consumer uses the model.
    // ------------------=
    pub fn cancel_app_turn(&mut self) {
        if let Some(id) = self.app_owner.take() {
            if let Some(service) = self.qwen.as_mut() {
                if let Some(saved) = self.app_saved.take() {
                    service.restore_conversation(saved);
                } else {
                    service.clear_conversation();
                }
            }
            let mut panel = crate::ui::app_assistant::read(id);
            if panel.workflow_goal == self.app_workflow_goal {
                panel.cancel_delivery();
                panel.reply(b"App request cancelled. Any streamed source remains in the editor; no further code will be applied.");
                crate::ui::app_assistant::write(id, panel);
            }
        }
        self.clear_app_turn_timing();
    }

    // ------------------------=
    // FUNC: cancel_editor_request
    // DESC: Revokes only the closing editor's inference and matching goal without interrupting unrelated desktop conversation.
    // ------------------=
    pub fn cancel_editor_request(&mut self) {
        let goal = crate::ui::app_assistant::read(2).workflow_goal;
        if self.app_owner == Some(2) { self.cancel_app_turn(); }
        if goal.is_some() && goal == self.intelligence_goal { self.interrupt_active_intelligence(); }
        let mut panel = crate::ui::app_assistant::read(2);
        panel.cancel_delivery();
        crate::ui::app_assistant::write(2, panel);
    }

    // ------------------------=
    // FUNC: clear_app_turn_timing
    // DESC: Releases the bounded progress watchdog whenever the app-generation model lease ends.
    // ------------------=
    fn clear_app_turn_timing(&mut self) {
        self.app_workflow_goal = None;
        self.app_started_ns = None;
        self.app_progress_ns = None;
        self.app_progress_tokens = 0;
    }

    // ------------------------=
    // FUNC: poll_app_turn
    // DESC: Cooperatively streams app output and stages complete edits without publishing private text to desktop chat or speech.
    // ------------------=
    fn poll_app_turn(&mut self) -> bool {
        let Some(id) = self.app_owner else {
            return false;
        };
        let panel = crate::ui::app_assistant::read(id);
        if panel.generation != crate::ui::app_assistant::GenerationStatus::Running {
            self.cancel_app_turn();
            crate::ui::app_assistant::write(id, panel);
            return true;
        }
        if crate::ui::app_assistant::read(id).workflow_goal != self.app_workflow_goal
            || !self.generation_workflow_current(self.app_workflow_goal) {
            self.cancel_app_turn();
            return true;
        }
        let Some(service) = self.qwen.as_mut() else {
            let mut panel = crate::ui::app_assistant::read(id);
            panel.generation = crate::ui::app_assistant::GenerationStatus::Failed;
            panel.reply(b"The local model became unavailable. Any streamed source remains in the editor. You can retry or cancel.");
            crate::ui::app_assistant::write(id, panel);
            self.app_owner = None;
            self.clear_app_turn_timing();
            return false;
        };
        let mut budget = qwen::pump::PumpBudget::new(crate::ui::performance::monotonic_ns());
        while budget.next(crate::ui::performance::monotonic_ns()) {
            let result = service.poll();
            let now = crate::ui::performance::monotonic_ns();
            // Prefill advances real inference work without producing source text.
            // Repeated waiting polls do not renew the watchdog; the overall
            // deadline still bounds a model that emits unusable output.
            if result.is_ok() && service.progress_tokens() > self.app_progress_tokens {
                let previous=self.app_progress_tokens;
                self.app_progress_tokens = service.progress_tokens();
                self.app_progress_ns = now;
                let progress=service.progress();
                if progress.generated_tokens==0 && previous/32!=self.app_progress_tokens/32 {
                    let mut panel=crate::ui::app_assistant::read(id);
                    let mut message=[0u8;128];
                    let mut writer=ProgressMessage{bytes:&mut message,length:0};
                    use core::fmt::Write;
                    let _=write!(&mut writer,"Preparing SDK context: {} / {} tokens. Code will stream into the editor.",progress.processed_tokens,progress.prompt_tokens);
                    let length=writer.length;
                    panel.reply(&message[..length]);
                    crate::ui::app_assistant::write(id,panel);
                    return true;
                }
            }
            if crate::ui::app_assistant::generation_timed_out(
                self.app_started_ns,
                self.app_progress_ns,
                now,
                crate::ui::app_assistant::generation_budget(id, &panel.request[..panel.request_len]),
            ) {
                service.cancel();
                let mut panel = crate::ui::app_assistant::read(id);
                panel.generation = crate::ui::app_assistant::GenerationStatus::Failed;
                panel.reply(b"Generation reached its progress or time limit. Partial source remains in the editor. The model is available for another request.");
                crate::ui::app_assistant::write(id, panel);
                if let Some(saved) = self.app_saved.take() {
                    service.restore_conversation(saved);
                } else {
                    service.clear_conversation();
                }
                self.app_owner = None;
                self.clear_app_turn_timing();
                return true;
            }
            // Service output is append-only during this lease, and Ok(true)
            // is its visible-output change notification. Waiting for a worker
            // or advancing prefill cannot complete an unchanged source frame;
            // do not rescan up to 16 KiB on every cooperative waiting poll.
            // The terminal path below still validates the entire final output.
            let framed = if result == Ok(true) {
                crate::ui::app_assistant::read(id).completed_generation_output_length(service.output())
            } else {
                None
            };
            if let Some(end) = framed {
                let mut output = [0u8; 16384];
                output[..end].copy_from_slice(&service.output()[..end]);
                service.cancel();
                let mut panel = crate::ui::app_assistant::read(id);
                panel.generation = if panel.accept_for_owner(id, &output[..end], true) {
                    crate::ui::app_assistant::GenerationStatus::Complete
                } else {
                    crate::ui::app_assistant::GenerationStatus::Failed
                };
                crate::ui::app_assistant::write(id, panel);
                if let Some(saved) = self.app_saved.take() {
                    service.restore_conversation(saved);
                } else {
                    service.clear_conversation();
                }
                self.app_owner = None;
                self.clear_app_turn_timing();
                return true;
            }
            if result.is_err() || !service.busy() {
                let mut panel = crate::ui::app_assistant::read(id);
                if result.is_err() {
                    panel.generation = crate::ui::app_assistant::GenerationStatus::Failed;
                    panel.reply(b"Local generation failed. Partial source remains in the editor; you can retry or cancel.");
                } else {
                    let output = service.output();
                    let end = output
                        .iter()
                        .rposition(|b| !b.is_ascii_whitespace())
                        .map_or(0, |i| i + 1);
                    panel.generation =
                        if panel.accept_for_owner(id, &output[..end], service.completed()) {
                            crate::ui::app_assistant::GenerationStatus::Complete
                        } else {
                            crate::ui::app_assistant::GenerationStatus::Failed
                        };
                }
                crate::ui::app_assistant::write(id, panel);
                if let Some(saved) = self.app_saved.take() {
                    service.restore_conversation(saved);
                } else {
                    service.clear_conversation();
                }
                self.app_owner = None;
                self.clear_app_turn_timing();
                return true;
            }
            if result == Ok(true) {
                let mut panel = crate::ui::app_assistant::read(id);
                let update=panel.stream_model_output(id,service.output());
                if update.progressed {
                    self.app_progress_ns = now;
                }
                crate::ui::app_assistant::write(id, panel);
                // The editor consumer requests presentation only when it has
                // committed a paced buffer. Hidden source bytes alone do not
                // invalidate the desktop; final/error paths above still do.
                return update.redraw;
            }
        }
        false
    }

    // ------------------------=
    // FUNC: refresh_native_readiness
    // DESC: Derives chat availability from the owned native services so boot and owner changes cannot leave stale model flags.
    // ------------------=
    fn refresh_native_readiness(&mut self) {
        let ministral_ready = self.native_ready(chat::MINISTRAL_MODEL_ID);
        let hermes_ready = self.native_ready(chat::HERMES_MODEL_ID);
        self.chat.set_ministral_ready(ministral_ready);
        self.chat.set_hermes_ready(hermes_ready);
    }

    // ------------------------=
    // FUNC: native_profile
    // DESC: Exposes active native measurements without exposing model memory or bypassing inference authority.
    // ------------------=
    pub fn native_profile(&self) -> Option<(qwen::metrics::Profile, [u64; 3], usize)> {
        self.qwen.as_ref().map(|service| {
            let (profile, workers) = service.profile();
            (profile, workers, service.allocated_bytes)
        })
    }
    // ------------------------=
    // FUNC: reset_native_benchmark
    // DESC: Clears idle native conversation caches for an explicitly requested repeatable benchmark.
    // ------------------=
    pub fn reset_native_benchmark(&mut self) -> bool {
        if self.qwen.as_ref().is_some_and(|service| service.busy()) || !self.chat.input().is_empty() {
            return false;
        }
        for slot in [&mut self.qwen, &mut self.other_native] {
            if let Some(service) = slot.as_mut() { service.clear_conversation(); }
        }
        true
    }
    // ------------------------=
    // FUNC: native_ready
    // DESC: Resolves model availability across isolated active and parked native services.
    // ------------------=
    pub fn native_ready(&self, id: ModelId) -> bool {
        if id == self.active_native { return self.qwen.is_some(); }
        match id {
            chat::HERMES_MODEL_ID | chat::MINISTRAL_MODEL_ID => self.other_native.is_some(),
            _ => false,
        }
    }
    // ------------------------=
    // FUNC: load_hermes
    // DESC: Registers optional pinned Hermes with independent KV memory behind the existing local boundary.
    // ------------------=
    pub fn load_hermes(&mut self, bytes: &'static [u8], arena: &'static mut [u8]) -> bool {
        if self.qwen.is_some() || self.active_native != chat::HERMES_MODEL_ID { return false; }
        let started = crate::ui::performance::monotonic_ns();
        let memory_bytes = bytes.len() as u64 + arena.len() as u64;
        let Ok(service) = qwen::service::Service::load_hermes(bytes, arena) else { return false; };
        let descriptor = ModelDescriptor {
            id: chat::HERMES_MODEL_ID, version: 1, provider: model::LOCAL_PROVIDER_ID,
            adapter: RuntimeAdapter::InfinityNative,
            capabilities: CAP_REASONING | CAP_INTENT_RESOLUTION | CAP_CLASSIFICATION,
            size: bytes.len() as u64,
            requirements: ModelRequirements { memory_bytes,
                backend: BackendClass::Cpu, minimum_backend_version: 1 },
            trust: TrustState::SystemVerified, object_ref: [0; 16],
            install_state: InstallState::Loaded, install_class: InstallClass::SystemOptional,
            checksum: 0xe06f7791, private_data_eligible: true,
        };
        if self.models.register(descriptor).is_err() { return false; }
        self.qwen_metrics.load_ns = started.zip(crate::ui::performance::monotonic_ns())
            .map_or(0, |(a,b)| b.saturating_sub(a));
        self.qwen = Some(service);
        self.chat.set_hermes_ready(true);
        true
    }
    // ------------------------=
    // FUNC: bind_chat_owner
    // DESC: Prevents conversation context from crossing authenticated user boundaries.
    // ------------------=
    pub fn bind_chat_owner(&mut self, owner: [u8; 16]) {
        if self.chat_owner != owner {
            self.stop_application_agent();
            self.cancel_app_turn();
            self.execution.cancel();
            self.intelligence = intelligence::Service::new();
            self.intelligence_goal = None;
            self.intelligence_execution = None;
            self.intelligence_result = None;
            self.software_execution = None;
            self.intelligence_summary = None;
            self.intelligence_checkpoint_pending = false;
            if let Some(service) = self.qwen.as_mut() {
                service.clear_conversation();
            }
            if let Some(service) = self.other_native.as_mut() {
                service.clear_conversation();
            }
            let ministral_ready = self.native_ready(chat::MINISTRAL_MODEL_ID);
            let hermes_ready = self.native_ready(chat::HERMES_MODEL_ID);
            self.chat = ChatRuntime::new();
            self.chat.set_ministral_ready(ministral_ready);
            self.chat.set_hermes_ready(hermes_ready);
            self.chat_owner = owner;
        }
    }
    // ------------------------=
    // FUNC: cancel_chat
    // DESC: Cancels active native generation while preserving its partial response.
    // ------------------=
    pub fn cancel_chat(&mut self) -> bool {
        let mut editor = crate::ui::app_assistant::read(2);
        let editor_active = matches!(editor.generation, crate::ui::app_assistant::GenerationStatus::Queued | crate::ui::app_assistant::GenerationStatus::Running)
            || editor.generation == crate::ui::app_assistant::GenerationStatus::Complete
                && (editor.stream.delivered < editor.argument_len || editor.apply_generated_compile_requested());
        let active_work=self.application_agent.is_some() || self.app_owner.is_some() || self.intelligence_goal.is_some() || editor_active;
        if editor_active {
            editor.cancel_delivery();
            crate::ui::app_assistant::write(2, editor);
        }
        self.stop_application_agent();
        if self.app_owner.is_some() {
            self.cancel_app_turn();
        }
        self.interrupt_active_intelligence();
        let broker_cancelled=self.execution.cancel();
        if active_work || broker_cancelled {
            self.chat.generation_state = chat::GenerationState::Cancelled;
            return true;
        }
        let Some(service) = self.qwen.as_mut() else {
            return false;
        };
        let active = service.busy();
        service.cancel();
        if active {
            self.chat.generation_state = chat::GenerationState::Cancelled;
        }
        active
    }
    // ------------------------=
    // FUNC: load_ministral
    // DESC: Registers verified secondary Ministral weights without sharing mutable caches.
    // ------------------=
    pub fn load_ministral(&mut self, bytes: &'static [u8], arena: &'static mut [u8]) -> bool {
        if self.other_native.is_some() || self.active_native != chat::HERMES_MODEL_ID { return false; }
        let Ok(service) = qwen::service::Service::load(bytes, arena) else { return false; };
        if bytes.len() != 2_147_023_008 { return false; }
        let descriptor = ModelDescriptor {
            id: chat::MINISTRAL_MODEL_ID, version: 1, provider: model::LOCAL_PROVIDER_ID,
            adapter: RuntimeAdapter::InfinityNative, capabilities: CAP_REASONING,
            size: bytes.len() as u64,
            requirements: ModelRequirements { memory_bytes: 3_489_200_288, backend: BackendClass::Cpu, minimum_backend_version: 1 },
            trust: TrustState::SystemVerified, object_ref: [0;16], install_state: InstallState::Loaded,
            install_class: InstallClass::SystemOptional, checksum: 0xd450d19e, private_data_eligible: true,
        };
        if self.models.register(descriptor).is_err() { return false; }
        self.other_native = Some(service);
        self.chat.set_ministral_ready(true);
        true
    }
    // ------------------------=
    // FUNC: submit_chat
    // DESC: Routes selected native models to bounded local inference; legacy helpers retain their behavior.
    // ------------------=
    pub fn submit_chat(&mut self) -> bool {
        self.submit_chat_from(RequestOrigin::DesktopChat)
    }

    // ------------------------=
    // FUNC: submit_voice_chat
    // DESC: Routes an authenticated voice transcript through the same AIKit ingress while preserving its origin.
    // ------------------=
    pub fn submit_voice_chat(&mut self) -> bool {
        self.submit_chat_from(RequestOrigin::Voice)
    }

    // ------------------------=
    // FUNC: submit_chat_from
    // DESC: Applies shared routing and execution boundaries while letting explicit new user input interrupt a stalled app-generation lease.
    // ------------------=
    fn submit_chat_from(&mut self, origin: RequestOrigin) -> bool {
        self.stop_application_agent();
        if self.app_owner.is_some() {
            self.cancel_app_turn();
        }
        self.interrupt_active_intelligence();
        if self.execution.busy()
            || !self.chat.enabled()
            || self.qwen.as_ref().is_some_and(|s| s.busy())
        {
            return false;
        }
        let mut input = [0u8; 4096];
        let length = self.chat.input().len();
        if length > input.len() {
            return false;
        }
        input[..length].copy_from_slice(self.chat.input());
        let Ok(route) = self.route_request(&input[..length], origin) else {
            return false;
        };
        if matches!(route, RequestRoute::Control(_)) {
            self.chat.begin_native_turn();
            self.chat.generation_state = chat::GenerationState::Running;
            return true;
        }
        if let RequestRoute::Goal(id) = route {
            self.chat.begin_native_turn();
            let plan = self.intelligence.inspect(id, self.chat_owner);
            let message = if plan
                .is_some_and(|plan| plan.state == intelligence::PlanState::AwaitingApproval)
            {
                b"I created a bounded plan. One or more sensitive steps require your approval before execution.".as_slice()
            } else {
                b"I created a bounded plan using installed InfinityOS capabilities. Execution will continue asynchronously and report verified progress.".as_slice()
            };
            self.chat.update_native_response(message);
            self.chat.generation_state = chat::GenerationState::Complete;
            return true;
        }
        if !matches!(
            self.chat.selected_model(),
            chat::MINISTRAL_MODEL_ID | chat::HERMES_MODEL_ID
        ) {
            return self.chat.submit_input();
        }
        if self.chat.selected_model() != self.active_native {
            if !self.native_ready(self.chat.selected_model()) {
                return false;
            }
            self.cancel_chat();
            core::mem::swap(&mut self.qwen, &mut self.other_native);
            self.active_native = self.chat.selected_model();
        }
        let Some(service) = self.qwen.as_mut() else {
            return false;
        };
        if !self.chat.enabled() || service.busy() {
            return false;
        }
        let submitted_ns = crate::ui::performance::monotonic_ns();
        let input = self.chat.input();
        let submitted = if route == RequestRoute::Inference(InferenceMode::Conversation) { service.submit(input) }
            else { service.submit_desktop(input, control::INSTRUCTIONS) };
        if let Err(error) = submitted {
            self.chat.generation_state = if error == qwen::gguf::Error::Overflow {
                chat::GenerationState::ContextFull
            } else {
                chat::GenerationState::Failed
            };
            return false;
        }
        self.chat.generation_state = chat::GenerationState::Running;
        self.qwen_metrics
            .begin(submitted_ns, service.reused_tokens, service.prefill_tokens);
        self.qwen_tokens = 0;
        self.qwen_decode_ns = 0;
        self.qwen_first_token_ns = None;
        self.chat.begin_native_turn();
        true
    }

    // ------------------------=
    // FUNC: interrupt_active_intelligence
    // DESC: Checkpoints and releases an active plan when the user explicitly submits a newer text or voice objective.
    // ------------------=
    fn interrupt_active_intelligence(&mut self) {
        let Some(id) = self.intelligence_goal.take() else {
            return;
        };
        let owner = self.chat_owner;
        for panel_id in 0..crate::ui::app_assistant::PANEL_SLOTS {
            let mut panel=crate::ui::app_assistant::read(panel_id);
            if panel.cancel_workflow(id) {
                panel.reply(b"The previous goal was interrupted. Its partial source is preserved and will not be saved or compiled automatically.");
                crate::ui::app_assistant::write(panel_id,panel);
            }
        }
        let mut executor = NativePlanExecutor {
            broker: &mut self.execution,
            active: &mut self.intelligence_execution,
            result: &mut self.intelligence_result,
            software: &mut self.software_execution,
            origin: RequestOrigin::DesktopChat,
        };
        let _ = self.intelligence.interrupt(id, owner, &mut executor);
        self.intelligence_summary = self.intelligence.inspect(id,owner).map(|plan|intelligence::PlanSummary{
            goal_id:id,state:plan.state,total:plan.task_count,
            completed:plan.tasks.iter().flatten().filter(|task|task.state==intelligence::TaskState::Succeeded).count() as u8,
            failed_task:plan.tasks.iter().flatten().find(|task|task.state==intelligence::TaskState::Failed).map(|task|task.id),
        });
        self.intelligence_checkpoint_pending = true;
    }
    // ------------------------=
    // FUNC: take_control
    // DESC: Consumes a command exactly once after rechecking the currently authenticated UI owner and permission.
    // ------------------=
    pub fn take_control(&mut self, owner: [u8; 16], allowed: bool) -> Option<control::Command> {
        let origin = self.execution.pending_origin();
        let context = PolicyContext {
            requester: owner,
            owner: self.chat_owner,
            chat_enabled: self.chat.enabled(),
            desktop_allowed: allowed,
        };
        match self.execution.take(context) {
            Ok(Some(request)) => Some(request.command),
            Ok(None) => None,
            Err(_) => {
                if let Some((token,_))=self.intelligence_execution {
                    self.intelligence_result=Some((token,false));
                }
                if let Some(RequestOrigin::Application(id)) = origin {
                    let mut panel = crate::ui::app_assistant::read(id);
                    panel.pending = crate::ui::app_assistant::Action::None;
                    panel.reply(b"OS action cancelled: its application, session or permission changed.");
                    crate::ui::app_assistant::write(id, panel);
                    return None;
                }
                self.chat.generation_state = chat::GenerationState::Cancelled;
                self.chat.update_native_response(b"OS action cancelled: the session or AI permission changed.");
                None
            }
        }
    }

    // ------------------------=
    // FUNC: control_pending
    // DESC: Allows presentation to observe dispatch or denial even when no model tokens are being generated.
    // ------------------=
    pub fn control_pending(&self) -> bool { self.execution.pending() }

    // ------------------------=
    // FUNC: finish_control
    // DESC: Publishes the dispatcher's actual outcome instead of treating model text as execution evidence.
    // ------------------=
    pub fn finish_control(&mut self, command: control::Command, success: bool) {
        let origin = self.execution.active_origin();
        if self.execution.finish(command, success).is_err() {
            return;
        }
        if let Some((token, expected)) = self.intelligence_execution {
            if expected == command {
                self.intelligence_result = Some((token, success));
                if self.software_execution.as_ref().is_some_and(|execution|execution.owns(token)) {
                    return;
                }
            }
        }
        if matches!(origin, Some(RequestOrigin::Application(_))) {
            return;
        }
        self.chat.update_native_response(command.result(success));
        self.chat.generation_state = if success {
            chat::GenerationState::Complete
        } else {
            chat::GenerationState::Failed
        };
    }

    // ------------------------=
    // FUNC: defer_control
    // DESC: Audits a native save or confirmation handoff without falsely reporting the command as completed.
    // ------------------=
    pub fn defer_control(&mut self, command: control::Command) {
        let _ = self.execution.defer(command);
    }

    // ------------------------=
    // FUNC: control_audit_count
    // DESC: Exposes the retained structured execution audit length for diagnostics and behavioral verification.
    // ------------------=
    pub const fn control_audit_count(&self) -> usize {
        self.execution.audit_count()
    }

    // ------------------------=
    // FUNC: control_audit
    // DESC: Returns one chronological structured execution audit record without granting mutation access.
    // ------------------=
    pub fn control_audit(&self, index: usize) -> Option<kit::AuditRecord> {
        self.execution.audit(index)
    }
    // ------------------------=
    // FUNC: poll_qwen
    // DESC: Advances local inference outside rendering and publishes model-produced text only.
    // ------------------=
    pub fn poll_qwen(&mut self) -> bool {
        if self.application_agent.is_some() && self.poll_application_agent() {return true;}
        if self.intelligence_goal.is_some() && self.poll_intelligence_runtime() {
            return true;
        }
        if self.app_owner.is_some() {
            return self.poll_app_turn();
        }
        if !self.qwen.as_ref().is_some_and(|service| service.busy()) {
            return false;
        }
        let started = crate::ui::performance::monotonic_ns();
        let changed = self.poll_qwen_inner();
        if self.qwen.as_ref().is_some_and(|service| !service.busy()) {
            self.qwen_metrics
                .finish(crate::ui::performance::monotonic_ns());
        }
        if let Some((a, b)) = started.zip(crate::ui::performance::monotonic_ns()) {
            self.qwen_metrics.max_pump_ns = self.qwen_metrics.max_pump_ns.max(b.saturating_sub(a));
        }
        changed
    }
    // ------------------------=
    // FUNC: record_first_visible_response
    // DESC: Closes the user-observed chat latency interval only after presentation completes.
    // ------------------=
    pub fn record_first_visible_response(&mut self) {
        self.qwen_metrics.visible(crate::ui::performance::monotonic_ns());
    }
    // ------------------------=
    // FUNC: poll_qwen_inner
    // DESC: Performs one cooperative inference pump, including response publication.
    // ------------------=
    fn poll_qwen_inner(&mut self) -> bool {
        let Some(service) = self.qwen.as_mut() else {
            return false;
        };
        if self.chat.selected_model() != self.active_native || !self.chat.enabled() {
            service.cancel();
            self.chat.generation_state = chat::GenerationState::Cancelled;
            return true;
        }
        let mut budget = qwen::pump::PumpBudget::new(crate::ui::performance::monotonic_ns());
        while budget.next(crate::ui::performance::monotonic_ns()) {
            let slice_start = crate::ui::performance::monotonic_ns();
            let result = service.poll();
            self.qwen_metrics.slice(slice_start, crate::ui::performance::monotonic_ns(), matches!(result, Ok(true)));
            match result {
                Ok(true) => {
                    let mut command = None;
                    let published = control::publish(&mut self.chat, &mut command,
                        service.output(), !service.busy(), service.completed());
                    if let Some(command) = command {
                        let envelope = self.requests.accept(&[], self.chat_owner, RequestOrigin::Model);
                        let request = ExecutionRequest { command, owner: envelope.owner, origin: envelope.origin,
                            correlation_id: envelope.correlation_id };
                        let _ = self.execution.stage(request);
                    }
                    self.qwen_tokens += 1;
                    #[cfg(target_os = "none")]
                    if let Some(now) = crate::ui::performance::monotonic_ns() {
                        if let Some(first) = self.qwen_first_token_ns {
                            self.qwen_decode_ns = now.saturating_sub(first);
                        } else {
                            self.qwen_first_token_ns = Some(now);
                        }
                    }
                    return published;
                }
                Ok(false) => (),
                Err(_) => {
                    service.cancel();
                    self.inference_failures += 1;
                    self.chat.generation_state = chat::GenerationState::Failed;
                    return true;
                }
            }
            if !service.busy() {
                if self.chat.generation_state == chat::GenerationState::Running {
                    let mut command = None;
                    control::publish(&mut self.chat, &mut command, service.output(), true, service.completed());
                    if let Some(command) = command {
                        let envelope = self.requests.accept(&[], self.chat_owner, RequestOrigin::Model);
                        let request = ExecutionRequest { command, owner: envelope.owner, origin: envelope.origin,
                            correlation_id: envelope.correlation_id };
                        let _ = self.execution.stage(request);
                    }
                    return true;
                }
                break;
            }
        }
        false
    }
    // ------------------------=
    // FUNC: new
    // DESC: Creates the modular AI runtime with bounded queues and no ambient providers.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            models: ModelRegistry::new(),
            providers: kit::ProviderRegistry::new(),
            voice: VoiceService::new(),
            agents: AgentManager::new(),
            chat: ChatRuntime::new(),
            cpu: LocalCpuBackend::new(4),
            qwen: None,
            other_native: None,
            active_native: chat::HERMES_MODEL_ID,
            app_owner: None,
            app_workflow_goal: None,
            app_saved: None,
            application_agent: None,
            application_saved: None,
            app_started_ns: None,
            app_progress_ns: None,
            app_progress_tokens: 0,
            chat_owner: [0; 16],
            requests: RequestApi::new(),
            execution: ExecutionBroker::new(),
            intelligence: intelligence::Service::new(),
            intelligence_goal: None,
            intelligence_execution: None,
            intelligence_result: None,
            software_execution: None,
            intelligence_summary: None,
            intelligence_checkpoint_pending: false,
            instruction_mapping: mapping::ConfiguredMap::new(),
            qwen_tokens: 0,
            qwen_decode_ns: 0,
            qwen_metrics: qwen::metrics::Metrics::new(),
            qwen_first_token_ns: None,
            initialized: false,
            inference_count: 0,
            inference_failures: 0,
            last_result: None,
            last_plan: None,
        }
    }

    // ------------------------=
    // FUNC: initialize
    // DESC: Registers and loads the checksum-verified offline CPU model and local provider.
    // ------------------=
    pub fn initialize(&mut self) -> Result<(), AiError> {
        self.initialize_with_object_ref([0; 16])
    }

    // ------------------------=
    // FUNC: initialize_with_object_ref
    // DESC: Initializes the local provider and binds its descriptor to a verified native model Object ID.
    // ------------------=
    pub fn initialize_with_object_ref(&mut self, object_ref: [u8; 16]) -> Result<(), AiError> {
        self.initialize_with_model_refs(object_ref, [0; 16], [0; 16])
    }

    // ------------------------=
    // FUNC: initialize_with_model_refs
    // DESC: Registers and loads the complete verified default local model set from native Object identities.
    // ------------------=
    pub fn initialize_with_model_refs(
        &mut self,
        intent_ref: [u8; 16],
        dialogue_ref: [u8; 16],
        creative_ref: [u8; 16],
    ) -> Result<(), AiError> {
        self.providers.register(local_provider())?;
        let mut descriptor = local_model_descriptor();
        descriptor.object_ref = intent_ref;
        self.models.register(descriptor)?;
        self.models.load(LOCAL_INTENT_MODEL_ID, 128 * 1024)?;
        self.models.register(conversation_model_descriptor(
            DIALOGUE_MODEL_ID,
            dialogue_ref,
        ))?;
        self.models.register(conversation_model_descriptor(
            CREATIVE_MODEL_ID,
            creative_ref,
        ))?;
        self.models.load(DIALOGUE_MODEL_ID, 8 * 1024 * 1024)?;
        self.models.load(CREATIVE_MODEL_ID, 8 * 1024 * 1024)?;
        self.initialized = true;
        Ok(())
    }

    // ------------------------=
    // FUNC: infer
    // DESC: Routes a typed request and executes it through an eligible provider backend.
    // ------------------=
    pub fn infer(
        &mut self,
        request: &ModelExecutionRequest<'_>,
        now: u64,
    ) -> Result<ModelExecutionResult, AiError> {
        if !self.initialized {
            return Err(AiError::ProviderUnavailable);
        }
        let provider = self.providers.select(request)?;
        let model_id = request.model.unwrap_or(LOCAL_INTENT_MODEL_ID);
        let model = self.models.inspect(model_id).ok_or(AiError::ModelUnknown)?;
        if model.install_state != InstallState::Loaded {
            return Err(AiError::ModelNotLoaded);
        }
        if model.capabilities & request.capability_class != request.capability_class {
            return Err(AiError::CapabilityUnsupported);
        }
        if model.requirements.backend != BackendClass::Cpu
            || request.resource_policy.memory_limit < model.requirements.memory_bytes
            || request.resource_policy.cpu_weight == 0
            || request.resource_policy.queue_limit == 0
        {
            return Err(AiError::InvalidRequest);
        }
        if provider.local && provider.id == model.provider {
            match self.cpu.infer(request, now) {
                Ok(result) => {
                    self.inference_count = self.inference_count.saturating_add(1);
                    self.last_result = Some(result);
                    Ok(result)
                }
                Err(error) => {
                    self.inference_failures = self.inference_failures.saturating_add(1);
                    Err(error)
                }
            }
        } else {
            Err(AiError::ProviderUnavailable)
        }
    }

    // ------------------------=
    // FUNC: request_distributed_inference
    // DESC: Adapts a provider-neutral AI request into authorized Resource Fabric accelerator work while preserving locality and privacy policy.
    // ------------------=
    pub fn request_distributed_inference(
        &self,
        request: &ModelExecutionRequest<'_>,
        workload_id: [u8; 16],
        compute_capability: crate::runtime::capability::CapabilityId,
        compute: &mut crate::runtime::compute::ComputeService,
        capabilities: &crate::runtime::capability::CapabilityManager,
        directory: &mut crate::runtime::fabric::resources::Directory,
        local: crate::runtime::node::types::NodeId,
        now: u64,
    ) -> Result<u64, AiError> {
        let distributed = self.distributed_inference_request(request, workload_id, compute_capability, now)?;
        compute.request(distributed, request.caller, capabilities, directory, local, now).map_err(map_distributed_compute_error)
    }

    // ------------------------=
    // FUNC: request_coordinated_distributed_inference
    // DESC: Submits provider-neutral remote inference through the production coordinator with explicit per-node IOP grants.
    // ------------------=
    pub fn request_coordinated_distributed_inference(
        &self,
        request: &ModelExecutionRequest<'_>,
        workload_id: [u8; 16],
        compute_capability: crate::runtime::capability::CapabilityId,
        authorities: [crate::runtime::compute_operator::RemoteComputeAuthority; 2],
        authority_count: u8,
        now: u64,
    ) -> Result<u64, AiError> {
        let distributed = self.distributed_inference_request(request, workload_id, compute_capability, now)?;
        crate::runtime::submit_compute_request(distributed, request.caller, authorities, authority_count, now).map_err(map_distributed_compute_error)
    }

    // ------------------------=
    // FUNC: distributed_inference_request
    // DESC: Validates AI privacy and provider policy before producing the shared accelerator scheduling contract.
    // ------------------=
    fn distributed_inference_request(
        &self,
        request: &ModelExecutionRequest<'_>,
        workload_id: [u8; 16],
        compute_capability: crate::runtime::capability::CapabilityId,
        now: u64,
    ) -> Result<crate::runtime::compute::ComputeRequestV1, AiError> {
        if !self.initialized || request.input_ref_count > 4 || request.deadline <= now {
            return Err(if request.deadline <= now { AiError::DeadlineExceeded } else { AiError::InvalidRequest });
        }
        if matches!(request.provider_policy, ProviderPolicy::LocalOnly | ProviderPolicy::PrivateDataLocalOnly | ProviderPolicy::AskBeforeRemote)
            || matches!(request.privacy_policy, PrivacyPolicy::Personal | PrivacyPolicy::Secret) { return Err(AiError::PrivacyDenied); }
        let provider = self.providers.select(request)?;
        if provider.local { return Err(AiError::ProviderUnavailable); }
        let mut refs = [[0; 16]; 2];
        for (index, value) in request.input_refs.iter().take(request.input_ref_count.min(2) as usize).enumerate() { refs[index] = *value; }
        Ok(crate::runtime::compute::ComputeRequestV1 {
            schema_version: crate::runtime::compute::COMPUTE_SCHEMA_VERSION,
            workload_kind: crate::runtime::compute::WorkloadKind::AcceleratorInferenceFixture,
            locality: crate::runtime::compute::ComputeLocality::RequireRemote,
            durability: crate::runtime::compute::ComputeDurability::Restartable,
            priority: request.options.priority,
            privacy_local_only: false,
            workload_id,
            input_refs: refs,
            allowed_nodes: [crate::runtime::node::types::NodeId([0; 32]); 2],
            allowed_node_count: 0,
            allowed_domains: [0; 2],
            allowed_domain_count: 0,
            memory_bytes: request.resource_policy.memory_limit,
            deadline: request.deadline,
            correlation_id: request.correlation_id,
            capability_ref: compute_capability,
            affinity: request.model.unwrap_or(0) as u64,
            anti_affinity: 0,
            work_units: (request.options.maximum_output_units as u32).max(1),
            cpu_units: request.resource_policy.cpu_weight.max(1),
            restart_eligible: true,
            result_contract: crate::runtime::compute::COMPUTE_RESULT_CONTRACT_V1,
        })
    }

    // ------------------------=
    // FUNC: resolve_intent
    // DESC: Produces and validates a typed IntentPlan while leaving execution to InfinityOS policy and IOP.
    // ------------------=
    pub fn resolve_intent(
        &mut self,
        input: &[u8],
        caller: crate::runtime::execution::SecurityIdentity,
        capability: crate::runtime::capability::CapabilityId,
        now: u64,
        correlation_id: u64,
    ) -> Result<IntentPlan, AiError> {
        let context =
            ContextBroker::request(CONTEXT_SYSTEM_STATE, CONTEXT_SYSTEM_STATE, correlation_id)?;
        let request = ModelExecutionRequest {
            model: None,
            capability_class: CAP_INTENT_RESOLUTION,
            input,
            input_refs: context.object_refs,
            input_ref_count: context.object_count,
            options: InferenceOptions {
                maximum_output_units: 1,
                deterministic: true,
                priority: 200,
            },
            resource_policy: ResourcePolicy {
                workload: WorkloadClass::Interactive,
                memory_limit: 128 * 1024,
                cpu_weight: 200,
                queue_limit: 4,
            },
            privacy_policy: PrivacyPolicy::SystemMetadata,
            provider_policy: ProviderPolicy::LocalOnly,
            deadline: now.saturating_add(2000),
            correlation_id,
            caller,
            caller_capability: capability,
        };
        let mut result = self.infer(&request, now)?;
        result.context_classes = context.classes;
        self.last_result = Some(result);
        let plan = IntentPlan::from_model_result(result, context)?;
        ConsequencePolicy::validate(&plan)?;
        self.last_plan = Some(plan);
        Ok(plan)
    }

    // ------------------------=
    // FUNC: initialized
    // DESC: Reports whether the verified local AI runtime is ready.
    // ------------------=
    pub const fn initialized(&self) -> bool {
        self.initialized
    }

    // ------------------------=
    // FUNC: inference_count
    // DESC: Reports completed local inference operations.
    // ------------------=
    pub const fn inference_count(&self) -> u64 {
        self.inference_count
    }

    // ------------------------=
    // FUNC: inference_failures
    // DESC: Reports rejected or failed inference operations.
    // ------------------=
    pub const fn inference_failures(&self) -> u64 {
        self.inference_failures
    }

    // ------------------------=
    // FUNC: queue_depth
    // DESC: Reports current bounded CPU inference queue usage.
    // ------------------=
    pub const fn queue_depth(&self) -> u8 {
        self.cpu.queue_depth()
    }

    // ------------------------=
    // FUNC: last_result
    // DESC: Returns locality-safe diagnostics for the most recent successful inference.
    // ------------------=
    pub const fn last_result(&self) -> Option<ModelExecutionResult> {
        self.last_result
    }

    // ------------------------=
    // FUNC: last_plan
    // DESC: Returns the most recent validated typed plan for privacy-safe diagnostics.
    // ------------------=
    pub const fn last_plan(&self) -> Option<IntentPlan> {
        self.last_plan
    }

    // ------------------------=
    // FUNC: intelligence_plan
    // DESC: Exposes one immutable owner-scoped IntelligenceKit plan to authorized native presentation surfaces.
    // ------------------=
    pub fn intelligence_plan(&self, id: u64, owner: [u8; 16]) -> Option<&intelligence::Plan> {
        self.intelligence.inspect(id, owner)
    }

    // ------------------------=
    // FUNC: approve_intelligence_task
    // DESC: Applies explicit owner approval to one sensitive task without granting native authority.
    // ------------------=
    pub fn approve_intelligence_task(&mut self,id:u64,task:u16,owner:[u8;16])->Result<(),intelligence::ServiceError>{
        self.intelligence.approve(id,task,owner)
    }

    // ------------------------=
    // FUNC: poll_intelligence_runtime
    // DESC: Cooperatively advances the active plan through the existing broker without blocking inference or audio.
    // ------------------=
    fn poll_intelligence_runtime(&mut self)->bool{
        let Some(id)=self.intelligence_goal else{return false;};let owner=self.chat_owner;
        let origin=self.intelligence.inspect(id,owner).map(|plan|match plan.goal.origin{intelligence::RequestOrigin::Voice=>RequestOrigin::Voice,intelligence::RequestOrigin::Application(id)=>RequestOrigin::Application(id),intelligence::RequestOrigin::Text=>RequestOrigin::DesktopChat,intelligence::RequestOrigin::NativeService=>RequestOrigin::Model}).unwrap_or(RequestOrigin::DesktopChat);
        let mut executor=NativePlanExecutor{broker:&mut self.execution,active:&mut self.intelligence_execution,result:&mut self.intelligence_result,software:&mut self.software_execution,origin};
        let Ok(summary)=self.intelligence.poll(id,owner,&intelligence::InstalledCapabilities,&mut executor,crate::ui::performance::monotonic_ns().unwrap_or(0))else{self.intelligence_goal=None;self.cancel_app_turn();self.stop_application_agent();self.chat.update_native_response(b"The goal stopped because its deadline or execution authority changed.");self.chat.generation_state=chat::GenerationState::Failed;return true;};
        if self.intelligence_summary==Some(summary){return false;}self.intelligence_summary=Some(summary);self.intelligence_checkpoint_pending=true;
        match summary.state{
            intelligence::PlanState::Completed=>{
                let message=match self.software_execution.as_ref().filter(|execution|execution.goal==id) {
                    Some(execution) if execution.interaction_active=>b"The application was saved, compiled and launched. Its AI interaction contract is ready; the interactive session continues separately.".as_slice(),
                    Some(execution) if execution.context==0=>b"The requested source was saved and validated by its native toolchain. No application was launched.".as_slice(),
                    Some(_)=>b"The application source was saved and validated, and the native process launched successfully.".as_slice(),
                    None=>b"The goal completed. Every plan step returned native verification evidence.".as_slice(),
                };
                self.chat.update_native_response(message);self.chat.generation_state=chat::GenerationState::Complete;self.intelligence_goal=None;
            },
            intelligence::PlanState::Failed=>{self.cancel_app_turn();self.stop_application_agent();self.chat.update_native_response(b"The goal stopped at a failed or unavailable step. Completed verified steps were preserved.");self.chat.generation_state=chat::GenerationState::Failed;self.intelligence_goal=None;},
            intelligence::PlanState::AwaitingApproval=>self.chat.update_native_response(b"The plan is ready, but a sensitive step requires your explicit approval."),
            intelligence::PlanState::Executing=>self.chat.update_native_response(b"The plan is running through authorized native InfinityOS services."),
            _=>{},
        }true
    }

    // ------------------------=
    // FUNC: poll_intelligence
    // DESC: Advances an active goal through caller-supplied native registry and execution adapters.
    // ------------------=
    pub fn poll_intelligence<R: intelligence::CapabilityRegistry, E: intelligence::Executor>(
        &mut self,
        id: u64,
        owner: [u8; 16],
        registry: &R,
        executor: &mut E,
        now: u64,
    ) -> Result<intelligence::PlanSummary, intelligence::ServiceError> {
        self.intelligence.poll(id, owner, registry, executor, now)
    }

    // ------------------------=
    // FUNC: checkpoint_intelligence
    // DESC: Produces a versioned owner-bound checkpoint suitable for native ObjectStore persistence.
    // ------------------=
    pub fn checkpoint_intelligence(
        &self,
        id: u64,
        owner: [u8; 16],
        out: &mut [u8],
    ) -> Result<usize, intelligence::ServiceError> {
        self.intelligence.encode(id, owner, out)
    }

    // ------------------------=
    // FUNC: take_intelligence_checkpoint_request
    // DESC: Coalesces native ObjectStore writes to factual plan state transitions.
    // ------------------=
    pub fn take_intelligence_checkpoint_request(&mut self)->Option<u64>{
        if !self.intelligence_checkpoint_pending{return None;}self.intelligence_checkpoint_pending=false;self.intelligence_summary.map(|summary|summary.goal_id).or(self.intelligence_goal)
    }

    // ------------------------=
    // FUNC: software_execution_id
    // DESC: Supplies the exact active goal identity for a trusted generated-source compilation lease.
    // ------------------=
    pub fn software_execution_id(&self)->Option<u64>{
        self.software_execution.as_ref().filter(|execution|Some(execution.goal)==self.intelligence_goal)
            .map(|execution|execution.goal)
    }

    // ------------------------=
    // FUNC: software_goal_for_request
    // DESC: Binds dispatch only to the exact originating goal command, rejecting an older dequeued command after replacement.
    // ------------------=
    pub fn software_goal_for_request(&self,input:&[u8])->Option<u64> {
        let goal=self.software_execution_id()?;
        let plan=self.intelligence.inspect(goal,self.chat_owner)?;
        let request=control::ContextRequest::parse(plan.goal.objective.as_bytes())?;
        (input==&request.bytes[..request.length]).then_some(goal)
    }

    // ------------------------=
    // FUNC: generation_workflow_current
    // DESC: Validates an immutable generation lease against the authenticated active software goal, never a newly captured ambient goal.
    // ------------------=
    pub fn generation_workflow_current(&self,goal:Option<u64>)->bool {
        goal.is_none() || goal==self.software_execution_id()
            && self.software_execution.as_ref().is_some_and(|execution|execution.owner==self.chat_owner)
    }

    // ------------------------=
    // FUNC: record_software_event
    // DESC: Records actual owner-bound storage, compiler, process, and interaction receipts without accepting generated claims.
    // ------------------=
    pub fn record_software_event(&mut self,goal:u64,owner:[u8;16],event:intelligence::software::Event)->bool{
        let accepted=self.software_execution.as_mut().is_some_and(|execution|execution.record(goal,owner,event));
        if accepted {
            if matches!(event,intelligence::software::Event::RuntimeFailed{..}) {
                let _=self.intelligence.mark_runtime_failure(goal,owner);
                self.stop_application_agent();
                // The failed app is already retired by its owning lifecycle.
                // Do not cancel a newer inference request or reopen the plan.
                if self.intelligence_goal==Some(goal) {
                    self.intelligence_goal=None;
                    self.intelligence_execution=None;
                    self.intelligence_result=None;
                    if self.chat_owner==owner && self.app_owner.is_none()
                        && !self.qwen.as_ref().is_some_and(|service|service.busy()) {
                        // Retiring the active plan must also retire its waiting
                        // chat state; voice ingress blocks while it is Running.
                        // A separate model turn retains its own response/state.
                        self.chat.update_native_response(b"The generated application failed at runtime. Its saved source and diagnostics are preserved. Any admitted repair only validates the source; the application will not restart automatically.");
                        self.chat.generation_state=chat::GenerationState::Failed;
                    }
                }
            }
            self.intelligence_checkpoint_pending=true;
        }
        accepted
    }

    // ------------------------=
    // FUNC: software_execution
    // DESC: Exposes verified workflow and ongoing interaction evidence to owner-scoped native observers.
    // ------------------=
    pub fn software_execution(&self,owner:[u8;16])->Option<&intelligence::software::Execution>{
        self.software_execution.as_ref().filter(|execution|execution.owner==owner)
    }

    // ------------------------=
    // FUNC: restore_intelligence
    // DESC: Restores interrupted owner-bound work without repeating verified completed steps.
    // ------------------=
    pub fn restore_intelligence<R: intelligence::CapabilityRegistry>(
        &mut self,
        input: &[u8],
        owner: [u8; 16],
        registry: &R,
    ) -> Result<u64, intelligence::ServiceError> {
        let id=self.intelligence.restore(input, owner, registry)?;
        self.intelligence.begin(id,owner,registry)?;
        self.intelligence_goal=Some(id);self.intelligence_summary=None;self.intelligence_checkpoint_pending=true;Ok(id)
    }
}

// ------------------------=
// FUNC: map_distributed_compute_error
// DESC: Preserves stable provider-neutral AI errors across distributed compute admission and coordination failures.
// ------------------=
fn map_distributed_compute_error(error: crate::runtime::compute::ComputeError) -> AiError {
    match error {
        crate::runtime::compute::ComputeError::AccessDenied => AiError::AccessDenied,
        crate::runtime::compute::ComputeError::DeadlineExceeded => AiError::DeadlineExceeded,
        crate::runtime::compute::ComputeError::Full => AiError::QueueFull,
        _ => AiError::ProviderUnavailable,
    }
}

static mut AI_RUNTIME: AiRuntime = AiRuntime::new();

// ------------------------=
// FUNC: with_ai_runtime
// DESC: Provides serialized access to the colocated AI runtime until service isolation is available.
// ------------------=
pub fn with_ai_runtime<T>(f: impl FnOnce(&mut AiRuntime) -> T) -> T {
    let runtime = unsafe { &mut *(&raw mut AI_RUNTIME) };
    runtime.refresh_native_readiness();
    f(runtime)
}

// ------------------------=
// FUNC: initialize_global
// DESC: Initializes the global AI service state after Object Service readiness.
// ------------------=
pub fn initialize_global() -> bool {
    #[cfg(target_os = "none")]
    let refs = crate::storage::local_ai_model_object_refs().unwrap_or([[0; 16]; 3]);
    #[cfg(not(target_os = "none"))]
    let refs = [[0; 16]; 3];
    with_ai_runtime(|runtime| {
        runtime
            .initialize_with_model_refs(refs[0], refs[1], refs[2])
            .is_ok()
    })
}
