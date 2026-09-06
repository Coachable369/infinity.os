pub mod agent;
pub mod broker;
pub mod chat;
pub mod intent;
pub mod model;
pub mod provider;
pub mod types;
pub mod voice;

use agent::AgentManager;
use broker::{ContextBroker, CONTEXT_SYSTEM_STATE};
use chat::ChatRuntime;
use intent::{ConsequencePolicy, IntentPlan};
use model::{local_model_descriptor, LocalCpuBackend, ModelRegistry, LOCAL_INTENT_MODEL_ID};
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
    initialized: bool,
    inference_count: u64,
    inference_failures: u64,
    last_result: Option<ModelExecutionResult>,
    last_plan: Option<IntentPlan>,
}

impl AiRuntime {
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
        self.providers.register(local_provider())?;
        let mut descriptor = local_model_descriptor();
        descriptor.object_ref = object_ref;
        self.models.register(descriptor)?;
        self.models.load(LOCAL_INTENT_MODEL_ID, 128 * 1024)?;
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
    let object_ref = crate::storage::local_ai_model_object_ref().unwrap_or([0; 16]);
    #[cfg(not(target_os = "none"))]
    let object_ref = [0; 16];
    with_ai_runtime(|runtime| runtime.initialize_with_object_ref(object_ref).is_ok())
}
