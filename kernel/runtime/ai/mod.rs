pub mod agent;
pub mod broker;
pub mod chat;
pub mod generation;
pub mod intent;
pub mod memory;
pub mod model;
pub mod provider;
pub mod qwen;
pub mod types;
pub mod voice;
pub mod voice_pcm;
#[cfg(target_os = "none")]
pub mod voice_output;
pub mod voice_vad;

use agent::AgentManager;
use broker::{ContextBroker, CONTEXT_SYSTEM_STATE};
use chat::ChatRuntime;
use generation::{CREATIVE_MODEL_ID, DIALOGUE_MODEL_ID};
use intent::{ConsequencePolicy, IntentPlan};
use model::{
    conversation_model_descriptor, local_model_descriptor, LocalCpuBackend, ModelRegistry,
    LOCAL_INTENT_MODEL_ID,
};
use provider::{local_provider, ProviderRouter};
use types::*;
use voice::VoiceService;

pub struct AiRuntime {
    pub models: ModelRegistry,
    pub providers: ProviderRouter,
    pub voice: VoiceService,
    pub agents: AgentManager,
    pub chat: ChatRuntime,
    cpu: LocalCpuBackend,
    qwen: Option<qwen::service::Service>,
    other_native: Option<qwen::service::Service>,
    active_native: ModelId,
    chat_owner: [u8; 16],
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
            if let Some(service) = self.qwen.as_mut() {
                service.clear_conversation();
            }
            if let Some(service) = self.other_native.as_mut() { service.clear_conversation(); }
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
        if !matches!(self.chat.selected_model(), chat::MINISTRAL_MODEL_ID | chat::HERMES_MODEL_ID) {
            return self.chat.submit_input();
        }
        if self.chat.selected_model() != self.active_native {
            if !self.native_ready(self.chat.selected_model()) { return false; }
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
        if let Err(error) = service.submit(self.chat.input()) {
            self.chat.generation_state = if error == qwen::gguf::Error::Overflow {
                chat::GenerationState::ContextFull
            } else {
                chat::GenerationState::Failed
            };
            return false;
        }
        self.chat.generation_state = chat::GenerationState::Running;
        self.qwen_metrics.begin(submitted_ns, service.reused_tokens, service.prefill_tokens);
        self.qwen_tokens = 0;
        self.qwen_decode_ns = 0;
        self.qwen_first_token_ns = None;
        self.chat.begin_native_turn();
        true
    }
    // ------------------------=
    // FUNC: poll_qwen
    // DESC: Advances local inference outside rendering and publishes model-produced text only.
    // ------------------=
    pub fn poll_qwen(&mut self) -> bool {
        if !self.qwen.as_ref().is_some_and(|service| service.busy()) {
            return false;
        }
        let started = crate::ui::performance::monotonic_ns();
        let changed = self.poll_qwen_inner();
        if self.qwen.as_ref().is_some_and(|service| !service.busy()) {
            self.qwen_metrics.finish(crate::ui::performance::monotonic_ns());
        }
        if let Some((a,b)) = started.zip(crate::ui::performance::monotonic_ns()) {
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
            return false;
        }
        let mut budget = qwen::pump::PumpBudget::new(crate::ui::performance::monotonic_ns());
        while budget.next(crate::ui::performance::monotonic_ns()) {
            let slice_start = crate::ui::performance::monotonic_ns();
            let result = service.poll();
            self.qwen_metrics.slice(slice_start, crate::ui::performance::monotonic_ns(), matches!(result, Ok(true)));
            match result {
                Ok(true) => {
                    let published = self.chat.publish_native_completion(service.output(), !service.busy());
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
                    self.chat.publish_native_completion(service.output(), true);
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
            providers: ProviderRouter::new(),
            voice: VoiceService::new(),
            agents: AgentManager::new(),
            chat: ChatRuntime::new(),
            cpu: LocalCpuBackend::new(4),
            qwen: None,
            other_native: None,
            active_native: chat::HERMES_MODEL_ID,
            chat_owner: [0; 16],
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
