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
    // FUNC: bind_chat_owner
    // DESC: Prevents conversation context from crossing authenticated user boundaries.
    // ------------------=
    pub fn bind_chat_owner(&mut self, owner: [u8; 16]) {
        if self.chat_owner != owner {
            if let Some(service) = self.qwen.as_mut() {
                service.clear_conversation();
            }
            let ready = self.qwen.is_some();
            self.chat = ChatRuntime::new();
            self.chat.set_qwen_ready(ready);
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
    // FUNC: load_qwen
    // DESC: Installs the verified native backend into the existing local AI service.
    // ------------------=
    pub fn load_qwen(&mut self, bytes: &'static [u8], arena: &'static mut [u8]) -> bool {
        let started = crate::ui::performance::monotonic_ns();
        match qwen::service::Service::load(bytes, arena) {
            Ok(service) => {
                self.qwen_metrics.load_ns = started.zip(crate::ui::performance::monotonic_ns())
                    .map_or(0, |(a,b)| b.saturating_sub(a));
                let descriptor = ModelDescriptor {
                    id: chat::QWEN_FULL_MODEL_ID,
                    version: 1,
                    provider: model::LOCAL_PROVIDER_ID,
                    adapter: RuntimeAdapter::InfinityNative,
                    capabilities: CAP_REASONING,
                    size: 5_027_783_488,
                    requirements: ModelRequirements {
                        memory_bytes: 6_369_960_768,
                        backend: BackendClass::Cpu,
                        minimum_backend_version: 1,
                    },
                    trust: TrustState::SystemVerified,
                    object_ref: [0; 16],
                    install_state: InstallState::Loaded,
                    install_class: InstallClass::SystemOptional,
                    checksum: 0xbdcd98d9,
                    private_data_eligible: true,
                };
                if self.models.register(descriptor).is_err() {
                    return false;
                }
                self.qwen = Some(service);
                self.chat.set_qwen_ready(true);
                true
            }
            Err(_) => {
                self.chat.set_qwen_ready(false);
                false
            }
        }
    }
    // ------------------------=
    // FUNC: submit_chat
    // DESC: Routes Qwen to bounded local inference; legacy models retain their existing behavior.
    // ------------------=
    pub fn submit_chat(&mut self) -> bool {
        if self.chat.selected_model() != chat::QWEN_FULL_MODEL_ID {
            return self.chat.submit_input();
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
        if let Some((a,b)) = started.zip(crate::ui::performance::monotonic_ns()) {
            self.qwen_metrics.max_pump_ns = self.qwen_metrics.max_pump_ns.max(b.saturating_sub(a));
        }
        changed
    }
    // ------------------------=
    // FUNC: poll_qwen_inner
    // DESC: Performs one cooperative inference pump, including response publication.
    // ------------------=
    fn poll_qwen_inner(&mut self) -> bool {
        let Some(service) = self.qwen.as_mut() else {
            return false;
        };
        if self.chat.selected_model() != chat::QWEN_FULL_MODEL_ID || !self.chat.enabled() {
            service.cancel();
            return false;
        }
        #[cfg(target_os = "none")]
        let started = crate::ui::performance::monotonic_ns();
        for _ in 0..256 {
            let slice_start = crate::ui::performance::monotonic_ns();
            let result = service.poll();
            self.qwen_metrics.slice(slice_start, crate::ui::performance::monotonic_ns(), matches!(result, Ok(true)));
            match result {
                Ok(true) => {
                    self.chat.update_native_response(service.output());
                    if !service.busy() {
                        self.chat.generation_state = chat::GenerationState::Complete;
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
                    return true;
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
                    self.chat.generation_state = chat::GenerationState::Complete;
                    return true;
                }
                break;
            }
            #[cfg(target_os = "none")]
            if started
                .zip(crate::ui::performance::monotonic_ns())
                .is_some_and(|(a, b)| b.saturating_sub(a) >= 2_000_000)
            {
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
