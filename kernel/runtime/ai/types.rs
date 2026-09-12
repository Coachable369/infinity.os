use crate::runtime::{capability::CapabilityId, execution::SecurityIdentity, iop::OperationId};

pub type ModelId = u32;
pub type ProviderId = u16;
pub type AgentId = u32;

pub const CAP_INTENT_RESOLUTION: u32 = 1 << 0;
pub const CAP_REASONING: u32 = 1 << 1;
pub const CAP_SPEECH_RECOGNITION: u32 = 1 << 2;
pub const CAP_SPEECH_SYNTHESIS: u32 = 1 << 3;
pub const CAP_EMBEDDING: u32 = 1 << 4;
pub const CAP_VISION: u32 = 1 << 5;
pub const CAP_CLASSIFICATION: u32 = 1 << 6;
pub const CAP_CODING: u32 = 1 << 7;
pub const CAP_PLANNING: u32 = 1 << 8;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RuntimeAdapter {
    InfinityNative,
    Gguf,
    Onnx,
    HardwareSpecific,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BackendClass {
    Cpu,
    Gpu,
    Npu,
    AiAccelerator,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SupportState {
    Tested,
    ImplementedUntested,
    Scaffolded,
    Unsupported,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrustState {
    SystemVerified,
    UserApproved,
    Untrusted,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InstallState {
    Available,
    Loaded,
    Unloaded,
    Invalid,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InstallClass {
    Core,
    SystemOptional,
    PostInstall,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProviderPolicy {
    LocalOnly,
    PreferLocal,
    RemoteAllowed,
    AskBeforeRemote,
    PrivateDataLocalOnly,
    LowestLatency,
    LowestPower,
    HighestQuality,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PrivacyPolicy {
    Public,
    SystemMetadata,
    Personal,
    Secret,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WorkloadClass {
    Interactive,
    Background,
    Indexing,
    Maintenance,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DataLocality {
    Local,
    Remote,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ModelRequirements {
    pub memory_bytes: u64,
    pub backend: BackendClass,
    pub minimum_backend_version: u16,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ModelDescriptor {
    pub id: ModelId,
    pub version: u16,
    pub provider: ProviderId,
    pub adapter: RuntimeAdapter,
    pub capabilities: u32,
    pub size: u64,
    pub requirements: ModelRequirements,
    pub trust: TrustState,
    pub object_ref: [u8; 16],
    pub install_state: InstallState,
    pub install_class: InstallClass,
    pub checksum: u32,
    pub private_data_eligible: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ResourcePolicy {
    pub workload: WorkloadClass,
    pub memory_limit: u64,
    pub cpu_weight: u16,
    pub queue_limit: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ModelExecutionRequest<'a> {
    pub model: Option<ModelId>,
    pub capability_class: u32,
    pub input: &'a [u8],
    pub input_refs: [[u8; 16]; 4],
    pub input_ref_count: u8,
    pub options: InferenceOptions,
    pub resource_policy: ResourcePolicy,
    pub privacy_policy: PrivacyPolicy,
    pub provider_policy: ProviderPolicy,
    pub deadline: u64,
    pub correlation_id: u64,
    pub caller: SecurityIdentity,
    pub caller_capability: CapabilityId,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InferenceOptions {
    pub maximum_output_units: u16,
    pub deterministic: bool,
    pub priority: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IntentClass {
    SystemStatus,
    DeviceList,
    SystemInfo,
    SystemBootStatus,
    MemoryStatus,
    Unknown,
}

impl IntentClass {
    // ------------------------=
    // FUNC: operation
    // DESC: Maps a model classification to a closed typed IOP operation.
    // ------------------=
    pub const fn operation(self) -> Option<OperationId> {
        match self {
            Self::SystemStatus => Some(OperationId::SystemStatus),
            Self::DeviceList => Some(OperationId::DeviceList),
            Self::SystemInfo => Some(OperationId::SystemInfo),
            Self::SystemBootStatus => Some(OperationId::SystemBootStatus),
            Self::MemoryStatus => Some(OperationId::MemoryStatus),
            Self::Unknown => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ModelExecutionResult {
    pub model: ModelId,
    pub provider: ProviderId,
    pub intent: IntentClass,
    pub confidence_milli: u16,
    pub locality: DataLocality,
    pub context_classes: u32,
    pub correlation_id: u64,
    pub elapsed_ticks: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AiError {
    ModelUnknown,
    ModelInvalid,
    ModelNotLoaded,
    CapabilityUnsupported,
    ProviderUnavailable,
    RemoteApprovalRequired,
    PrivacyDenied,
    AccessDenied,
    DeadlineExceeded,
    Cancelled,
    QueueFull,
    InvalidRequest,
    LowConfidence,
    RegistryFull,
}
