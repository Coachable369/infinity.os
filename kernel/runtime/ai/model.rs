use super::types::*;

#[path = "../../ai_model_asset.rs"]
pub mod asset;
use super::generation::{
    model_checksum as conversation_checksum, CREATIVE_MODEL_ID, DIALOGUE_MODEL_ID,
};
pub use asset::{local_model_checksum, model_object_bytes, LOCAL_INTENT_MODEL_ID};
use asset::{CLASS_COUNT, INTENT_WEIGHTS};

pub const LOCAL_PROVIDER_ID: ProviderId = 1;
pub const MAX_MODELS: usize = 8;

#[derive(Clone, Copy)]
pub struct ModelRegistry {
    entries: [Option<ModelDescriptor>; MAX_MODELS],
    count: usize,
}

impl ModelRegistry {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty bounded native model registry.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            entries: [None; MAX_MODELS],
            count: 0,
        }
    }

    // ------------------------=
    // FUNC: register
    // DESC: Registers a checksum-verified model descriptor without filesystem lookup.
    // ------------------=
    pub fn register(&mut self, descriptor: ModelDescriptor) -> Result<(), AiError> {
        if descriptor.id == LOCAL_INTENT_MODEL_ID && descriptor.checksum != local_model_checksum() {
            return Err(AiError::ModelInvalid);
        }
        if descriptor.version == 1
            && matches!(descriptor.id, DIALOGUE_MODEL_ID | CREATIVE_MODEL_ID)
            && descriptor.checksum != conversation_checksum(descriptor.id)
        {
            return Err(AiError::ModelInvalid);
        }
        if let Some(existing) = self
            .entries
            .iter_mut()
            .flatten()
            .find(|m| m.id == descriptor.id)
        {
            *existing = descriptor;
            return Ok(());
        }
        let slot = self
            .entries
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(AiError::RegistryFull)?;
        *slot = Some(descriptor);
        self.count += 1;
        Ok(())
    }

    // ------------------------=
    // FUNC: load
    // DESC: Marks a verified model loaded after checking its declared memory budget.
    // ------------------=
    pub fn load(&mut self, id: ModelId, memory_limit: u64) -> Result<(), AiError> {
        let model = self
            .entries
            .iter_mut()
            .flatten()
            .find(|m| m.id == id)
            .ok_or(AiError::ModelUnknown)?;
        if model.checksum == 0 || model.requirements.memory_bytes > memory_limit {
            return Err(AiError::ModelInvalid);
        }
        model.install_state = InstallState::Loaded;
        Ok(())
    }

    // ------------------------=
    // FUNC: install
    // DESC: Atomically validates, registers, and loads a model package without exposing partial state.
    // ------------------=
    pub fn install(
        &mut self,
        descriptor: ModelDescriptor,
        memory_limit: u64,
    ) -> Result<(), AiError> {
        if descriptor.object_ref == [0; 16]
            || descriptor.size == 0
            || descriptor.checksum == 0
            || descriptor.trust == TrustState::Untrusted
        {
            return Err(AiError::ModelInvalid);
        }
        let mut staged = *self;
        staged.register(descriptor)?;
        staged.load(descriptor.id, memory_limit)?;
        *self = staged;
        Ok(())
    }

    // ------------------------=
    // FUNC: upgrade
    // DESC: Atomically replaces an installed model with a newer verified package and preserves rollback state on failure.
    // ------------------=
    pub fn upgrade(
        &mut self,
        descriptor: ModelDescriptor,
        memory_limit: u64,
    ) -> Result<(), AiError> {
        let current = self.inspect(descriptor.id).ok_or(AiError::ModelUnknown)?;
        if descriptor.version <= current.version {
            return Err(AiError::InvalidRequest);
        }
        self.install(descriptor, memory_limit)
    }

    // ------------------------=
    // FUNC: remove
    // DESC: Removes an optional model package while protecting core System Generation models.
    // ------------------=
    pub fn remove(&mut self, id: ModelId) -> Result<(), AiError> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.is_some_and(|model| model.id == id))
            .ok_or(AiError::ModelUnknown)?;
        if self.entries[index].is_some_and(|model| model.install_class == InstallClass::Core) {
            return Err(AiError::AccessDenied);
        }
        self.entries[index] = None;
        self.count = self.count.saturating_sub(1);
        Ok(())
    }

    // ------------------------=
    // FUNC: unload
    // DESC: Releases a loaded model from the local inference backend.
    // ------------------=
    pub fn unload(&mut self, id: ModelId) -> Result<(), AiError> {
        let model = self
            .entries
            .iter_mut()
            .flatten()
            .find(|m| m.id == id)
            .ok_or(AiError::ModelUnknown)?;
        model.install_state = InstallState::Unloaded;
        Ok(())
    }

    // ------------------------=
    // FUNC: inspect
    // DESC: Returns immutable typed metadata for a registered model.
    // ------------------=
    pub fn inspect(&self, id: ModelId) -> Option<&ModelDescriptor> {
        self.entries.iter().flatten().find(|m| m.id == id)
    }

    // ------------------------=
    // FUNC: nth
    // DESC: Returns the nth registered model for bounded enumeration.
    // ------------------=
    pub fn nth(&self, index: usize) -> Option<&ModelDescriptor> {
        self.entries.iter().flatten().nth(index)
    }

    // ------------------------=
    // FUNC: count
    // DESC: Reports the number of registered models.
    // ------------------=
    pub const fn count(&self) -> usize {
        self.count
    }
}

// ------------------------=
// FUNC: conversation_model_descriptor
// DESC: Describes one bundled local conversational model and its native Object identity.
// ------------------=
pub fn conversation_model_descriptor(id: ModelId, object_ref: [u8; 16]) -> ModelDescriptor {
    ModelDescriptor {
        id,
        version: 1,
        provider: LOCAL_PROVIDER_ID,
        adapter: RuntimeAdapter::InfinityNative,
        capabilities: CAP_REASONING,
        size: super::generation::CONVERSATION_MODEL_OBJECT_BYTES as u32,
        requirements: ModelRequirements {
            memory_bytes: 4 * 1024 * 1024,
            backend: BackendClass::Cpu,
            minimum_backend_version: 1,
        },
        trust: TrustState::SystemVerified,
        object_ref,
        install_state: InstallState::Available,
        install_class: InstallClass::Core,
        checksum: conversation_checksum(id),
        private_data_eligible: true,
    }
}

pub struct LocalCpuBackend {
    queue_depth: u8,
    queue_limit: u8,
    cancelled: [u64; 4],
}

impl LocalCpuBackend {
    // ------------------------=
    // FUNC: new
    // DESC: Creates the bounded CPU inference backend.
    // ------------------=
    pub const fn new(queue_limit: u8) -> Self {
        Self {
            queue_depth: 0,
            queue_limit,
            cancelled: [0; 4],
        }
    }

    // ------------------------=
    // FUNC: cancel
    // DESC: Records a correlation identifier for cancellation before execution.
    // ------------------=
    pub fn cancel(&mut self, correlation_id: u64) {
        let at = correlation_id as usize % self.cancelled.len();
        self.cancelled[at] = correlation_id;
    }

    // ------------------------=
    // FUNC: infer
    // DESC: Executes genuine quantized linear-model inference on the CPU.
    // ------------------=
    pub fn infer(
        &mut self,
        request: &ModelExecutionRequest<'_>,
        now: u64,
    ) -> Result<ModelExecutionResult, AiError> {
        if now >= request.deadline {
            return Err(AiError::DeadlineExceeded);
        }
        if self.cancelled.contains(&request.correlation_id) {
            return Err(AiError::Cancelled);
        }
        let limit = request
            .resource_policy
            .queue_limit
            .min(self.queue_limit)
            .max(1);
        if self.queue_depth >= limit {
            return Err(AiError::QueueFull);
        }
        if request.capability_class & CAP_INTENT_RESOLUTION == 0 {
            return Err(AiError::CapabilityUnsupported);
        }
        self.queue_depth += 1;
        let (intent, confidence_milli) = classify_intent(request.input);
        self.queue_depth -= 1;
        if intent == IntentClass::Unknown {
            return Err(AiError::LowConfidence);
        }
        Ok(ModelExecutionResult {
            model: LOCAL_INTENT_MODEL_ID,
            provider: LOCAL_PROVIDER_ID,
            intent,
            confidence_milli,
            locality: DataLocality::Local,
            context_classes: 0,
            correlation_id: request.correlation_id,
            elapsed_ticks: 1,
        })
    }

    // ------------------------=
    // FUNC: queue_depth
    // DESC: Reports the bounded inference queue depth.
    // ------------------=
    pub const fn queue_depth(&self) -> u8 {
        self.queue_depth
    }

    #[cfg(not(target_os = "none"))]
    // ------------------------=
    // FUNC: set_queue_depth_for_test
    // DESC: Sets synthetic queue occupancy for deterministic host backpressure verification.
    // ------------------=
    pub fn set_queue_depth_for_test(&mut self, depth: u8) {
        self.queue_depth = depth.min(self.queue_limit);
    }
}

// ------------------------=
// FUNC: classify_intent
// DESC: Tokenizes input and evaluates the bundled quantized model parameters.
// ------------------=
fn classify_intent(input: &[u8]) -> (IntentClass, u16) {
    let mut scores = [0i32; CLASS_COUNT];
    let mut token = [0u8; 24];
    let mut length = 0usize;
    for byte in input.iter().copied().chain(core::iter::once(b' ')) {
        if byte.is_ascii_alphanumeric() {
            if length < token.len() {
                token[length] = byte.to_ascii_lowercase();
                length += 1;
            }
        } else if length != 0 {
            for feature in &INTENT_WEIGHTS {
                if feature.token == &token[..length] {
                    for class in 0..CLASS_COUNT {
                        scores[class] += feature.weights[class] as i32;
                    }
                }
            }
            length = 0;
        }
    }
    let mut best = 0usize;
    let mut second = 0i32;
    for index in 1..CLASS_COUNT {
        if scores[index] > scores[best] {
            second = scores[best].max(second);
            best = index;
        } else {
            second = second.max(scores[index]);
        }
    }
    let score = scores[best];
    if score < 55 || score - second < 20 {
        return (IntentClass::Unknown, 0);
    }
    let confidence = (600 + (score - second).min(400)) as u16;
    let intent = match best {
        0 => IntentClass::SystemStatus,
        1 => IntentClass::DeviceList,
        2 => IntentClass::SystemInfo,
        3 => IntentClass::SystemBootStatus,
        _ => IntentClass::MemoryStatus,
    };
    (intent, confidence)
}

// ------------------------=
// FUNC: local_model_descriptor
// DESC: Describes the bundled offline intent model and its native Object reference.
// ------------------=
pub fn local_model_descriptor() -> ModelDescriptor {
    ModelDescriptor {
        id: LOCAL_INTENT_MODEL_ID,
        version: 1,
        provider: LOCAL_PROVIDER_ID,
        adapter: RuntimeAdapter::InfinityNative,
        capabilities: CAP_INTENT_RESOLUTION | CAP_CLASSIFICATION,
        size: model_object_bytes().len() as u32,
        requirements: ModelRequirements {
            memory_bytes: 64 * 1024,
            backend: BackendClass::Cpu,
            minimum_backend_version: 1,
        },
        trust: TrustState::SystemVerified,
        // The runtime resolves the installed ObjectRef through the native model
        // registry. Zero denotes the immutable live-media payload before a
        // persistent System object is mounted.
        object_ref: [0; 16],
        install_state: InstallState::Available,
        install_class: InstallClass::SystemOptional,
        checksum: local_model_checksum(),
        private_data_eligible: true,
    }
}
