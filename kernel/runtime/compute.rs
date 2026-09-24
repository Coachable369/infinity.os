//! Versioned distributed-compute contracts and bounded authoritative lifecycle.
//! Placement consumes only live Resource Fabric advertisements; execution consumes
//! real ExecutionManager contexts and never shells into another node.

use super::capability::{CapabilityId, CapabilityManager, CapabilityType};
use super::execution::{
    ContextHandle, ContextState, ExecutionManager, MemoryRegion, PriorityClass, ResourceBudget,
};
use super::fabric::resources::{Directory, Resource, ResourceId, ResourceKind};
use super::node::types::NodeId;

pub const COMPUTE_SCHEMA_VERSION: u16 = 1;
pub const COMPUTE_REQUEST_V1_BYTES: usize = 192;
pub const COMPUTE_RESULT_V1_BYTES: usize = 160;
// The state is embedded after the 16-byte runtime header in the existing
// 4 KiB /system/runtime object. Keeping the aggregate at one ObjectStore
// allocation unit preserves the installed-store layout and recovery math.
pub const COMPUTE_STATE_BYTES: usize = 4080;
pub const COMPUTE_DISPATCH_V1_BYTES: usize = 144;
pub const MAX_COMPUTE_TASKS: usize = 24;
pub const MAX_COMPUTE_NODES: usize = 16;
pub const MAX_COMPUTE_NOTICES: usize = 64;
pub const MAX_COMPUTE_MEMORY: u64 = 64 * 1024 * 1024;
pub const MAX_SLICE_TICKS: u32 = 64;
pub const COMPUTE_RIGHT_EXECUTE: u32 = 1;
pub const COMPUTE_RESULT_CONTRACT_V1: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum WorkloadKind {
    CpuChecksum = 1,
    CpuBoundedCounter = 2,
    AcceleratorInferenceFixture = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ComputeLocality {
    RequireLocal = 1,
    PreferLocal = 2,
    PreferRemote = 3,
    RequireRemote = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ComputeDurability {
    Pinned = 1,
    Restartable = 2,
    CheckpointableScaffold = 3,
    MigratableScaffold = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ComputeState {
    Queued = 1,
    Placed = 2,
    Running = 3,
    Completed = 4,
    Failed = 5,
    Cancelled = 6,
    NodeLost = 7,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ComputeEventKind {
    Queued = 1,
    Placed = 2,
    Started = 3,
    Completed = 4,
    Failed = 5,
    Cancelled = 6,
    Restarted = 7,
    NodeLost = 8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ComputeError {
    AccessDenied = 1,
    InvalidRequest = 2,
    DeadlineExceeded = 3,
    NoPlacement = 4,
    Full = 5,
    UnknownTask = 6,
    InvalidState = 7,
    Cancelled = 8,
    NodeLost = 9,
    UnsupportedDurability = 10,
    StaleResult = 11,
    ResourceUnavailable = 12,
    Context = 13,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComputeRequestV1 {
    pub schema_version: u16,
    pub workload_kind: WorkloadKind,
    pub locality: ComputeLocality,
    pub durability: ComputeDurability,
    pub priority: u8,
    pub privacy_local_only: bool,
    pub workload_id: [u8; 16],
    pub input_refs: [[u8; 16]; 2],
    pub allowed_nodes: [NodeId; 2],
    pub allowed_node_count: u8,
    pub allowed_domains: [u64; 2],
    pub allowed_domain_count: u8,
    pub memory_bytes: u64,
    pub deadline: u64,
    pub correlation_id: u64,
    pub capability_ref: CapabilityId,
    pub affinity: u64,
    pub anti_affinity: u64,
    pub work_units: u32,
    pub cpu_units: u16,
    pub restart_eligible: bool,
    pub result_contract: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComputeDispatchV1 {
    pub schema_version: u16,
    pub workload_kind: WorkloadKind,
    pub state: u8,
    pub operation: u32,
    pub task_id: u64,
    pub epoch: u32,
    pub work_units: u32,
    pub memory_bytes: u64,
    pub deadline: u64,
    pub scope: u64,
    pub workload_id: [u8; 16],
    pub input_refs: [[u8; 16]; 2],
    pub target_node: NodeId,
    pub cpu_units: u16,
    pub priority: u8,
    pub result_contract: u8,
    pub error: u8,
    pub flags: u8,
    pub used_ticks: u64,
}

impl ComputeDispatchV1 {
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes a remote execution slice that fits exactly inside the authenticated native IOP transport envelope.
    // ------------------=
    pub fn encode(self) -> [u8; COMPUTE_DISPATCH_V1_BYTES] {
        let mut out = [0; COMPUTE_DISPATCH_V1_BYTES]; out[0..2].copy_from_slice(&self.schema_version.to_le_bytes()); out[2] = self.workload_kind as u8; out[3] = self.state; out[4..8].copy_from_slice(&self.operation.to_le_bytes()); out[8..16].copy_from_slice(&self.task_id.to_le_bytes()); out[16..20].copy_from_slice(&self.epoch.to_le_bytes()); out[20..24].copy_from_slice(&self.work_units.to_le_bytes()); out[24..32].copy_from_slice(&self.memory_bytes.to_le_bytes()); out[32..40].copy_from_slice(&self.deadline.to_le_bytes()); out[40..48].copy_from_slice(&self.scope.to_le_bytes()); out[48..64].copy_from_slice(&self.workload_id); out[64..80].copy_from_slice(&self.input_refs[0]); out[80..96].copy_from_slice(&self.input_refs[1]); out[96..128].copy_from_slice(&self.target_node.0); out[128..130].copy_from_slice(&self.cpu_units.to_le_bytes()); out[130] = self.priority; out[131] = self.result_contract; out[132] = self.error; out[133] = self.flags; out[136..144].copy_from_slice(&self.used_ticks.to_le_bytes()); out
    }

    // ------------------------=
    // FUNC: decode
    // DESC: Validates the bounded remote compute slice before authenticated admission.
    // ------------------=
    pub fn decode(input: &[u8]) -> Result<Self, ComputeError> {
        if input.len() != COMPUTE_DISPATCH_V1_BYTES || read_u16(input, 0) != COMPUTE_SCHEMA_VERSION || input[131] != COMPUTE_RESULT_CONTRACT_V1 || input[134..136] != [0; 2] || !matches!(read_u32(input, 4), value if value == super::iop::OperationId::ComputeRequest as u32 || value == super::iop::OperationId::ComputeCancel as u32 || value == super::iop::OperationId::ComputeResult as u32) { return Err(ComputeError::InvalidRequest); }
        let workload_kind = match input[2] { 1 => WorkloadKind::CpuChecksum, 2 => WorkloadKind::CpuBoundedCounter, 3 => WorkloadKind::AcceleratorInferenceFixture, _ => return Err(ComputeError::InvalidRequest) };
        let mut workload_id = [0; 16]; workload_id.copy_from_slice(&input[48..64]); let mut input_refs = [[0; 16]; 2]; input_refs[0].copy_from_slice(&input[64..80]); input_refs[1].copy_from_slice(&input[80..96]); let mut target_node = [0; 32]; target_node.copy_from_slice(&input[96..128]);
        let decoded = Self { schema_version: COMPUTE_SCHEMA_VERSION, workload_kind, state: input[3], operation: read_u32(input, 4), task_id: read_u64(input, 8), epoch: read_u32(input, 16), work_units: read_u32(input, 20), memory_bytes: read_u64(input, 24), deadline: read_u64(input, 32), scope: read_u64(input, 40), workload_id, input_refs, target_node: NodeId(target_node), cpu_units: read_u16(input, 128), priority: input[130], result_contract: input[131], error: input[132], flags: input[133], used_ticks: read_u64(input, 136) };
        if decoded.task_id == 0 || decoded.epoch == 0 || decoded.target_node.0 == [0; 32] || decoded.cpu_units == 0 || decoded.memory_bytes == 0 || decoded.memory_bytes > MAX_COMPUTE_MEMORY || (decoded.work_units == 0 && decoded.operation != super::iop::OperationId::ComputeResult as u32) { return Err(ComputeError::InvalidRequest); }
        Ok(decoded)
    }
}

// ------------------------=
// FUNC: execute_remote_dispatch
// DESC: Runs one authenticated remote slice in a real bounded execution context and returns a fenced typed response without shell execution.
// ------------------=
pub fn execute_remote_dispatch(mut dispatch: ComputeDispatchV1, execution: &mut ExecutionManager, local: NodeId, now: u64) -> Result<ComputeDispatchV1, ComputeError> {
    ComputeDispatchV1::decode(&dispatch.encode())?;
    if dispatch.target_node != local { return Err(ComputeError::AccessDenied); }
    if dispatch.deadline != 0 && now >= dispatch.deadline { return Err(ComputeError::DeadlineExceeded); }
    if dispatch.operation == super::iop::OperationId::ComputeCancel as u32 { dispatch.state = ComputeState::Cancelled as u8; dispatch.error = ComputeError::Cancelled as u8; dispatch.operation = super::iop::OperationId::ComputeResult as u32; return Ok(dispatch); }
    if dispatch.operation != super::iop::OperationId::ComputeRequest as u32 { return Err(ComputeError::InvalidRequest); }
    let handle = execution.create(0x1100, workload_image(dispatch.workload_kind), MemoryRegion { base: context_base(dispatch.task_id), length: dispatch.memory_bytes }, 0x6d00u16.saturating_add((dispatch.task_id % 64) as u16), priority(dispatch.priority), ResourceBudget { memory_limit: dispatch.memory_bytes, cpu_weight: dispatch.cpu_units, message_queue_limit: 8, io_priority: 2 }).map_err(|_| ComputeError::Context)?;
    execution.account_memory(handle, dispatch.memory_bytes).map_err(|_| ComputeError::Context)?; execution.set_state(handle, ContextState::Running).map_err(|_| ComputeError::Context)?;
    let ticks = dispatch.work_units.min(MAX_SLICE_TICKS); for _ in 0..ticks { execution.account_cpu_tick(handle).map_err(|_| ComputeError::Context)?; }
    let _ = execution.destroy(handle); dispatch.used_ticks = dispatch.used_ticks.saturating_add(ticks as u64); dispatch.work_units = dispatch.work_units.saturating_sub(ticks); dispatch.state = if dispatch.work_units == 0 { ComputeState::Completed as u8 } else { ComputeState::Running as u8 }; dispatch.operation = super::iop::OperationId::ComputeResult as u32; dispatch.input_refs[0] = output_digest_parts(dispatch.workload_id, dispatch.input_refs, dispatch.epoch, dispatch.used_ticks as u32); Ok(dispatch)
}

impl ComputeRequestV1 {
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes the exact little-endian version-one compute request without relying on Rust layout.
    // ------------------=
    pub fn encode(self) -> [u8; COMPUTE_REQUEST_V1_BYTES] {
        let mut out = [0u8; COMPUTE_REQUEST_V1_BYTES];
        out[0..2].copy_from_slice(&self.schema_version.to_le_bytes());
        out[2] = self.workload_kind as u8;
        out[3] = self.locality as u8;
        out[4] = self.durability as u8;
        out[5] = self.priority;
        out[6] = (self.privacy_local_only as u8) | (self.allowed_domain_count.min(2) << 1);
        out[7] = self.allowed_node_count;
        out[8..24].copy_from_slice(&self.workload_id);
        out[24..40].copy_from_slice(&self.input_refs[0]);
        out[40..56].copy_from_slice(&self.input_refs[1]);
        out[56..88].copy_from_slice(&self.allowed_nodes[0].0);
        out[88..120].copy_from_slice(&self.allowed_nodes[1].0);
        out[120..128].copy_from_slice(&self.allowed_domains[0].to_le_bytes());
        out[128..136].copy_from_slice(&self.allowed_domains[1].to_le_bytes());
        out[136..144].copy_from_slice(&self.memory_bytes.to_le_bytes());
        out[144..152].copy_from_slice(&self.deadline.to_le_bytes());
        out[152..160].copy_from_slice(&self.correlation_id.to_le_bytes());
        out[160..168].copy_from_slice(&self.capability_ref.to_le_bytes());
        out[168..176].copy_from_slice(&self.affinity.to_le_bytes());
        out[176..184].copy_from_slice(&self.anti_affinity.to_le_bytes());
        out[184..188].copy_from_slice(&self.work_units.to_le_bytes());
        out[188..190].copy_from_slice(&self.cpu_units.to_le_bytes());
        out[190] = self.restart_eligible as u8;
        out[191] = self.result_contract;
        out
    }

    // ------------------------=
    // FUNC: decode
    // DESC: Decodes and validates enum, count, boolean, and contract fields in a fixed-width compute request.
    // ------------------=
    pub fn decode(input: &[u8]) -> Result<Self, ComputeError> {
        if input.len() != COMPUTE_REQUEST_V1_BYTES { return Err(ComputeError::InvalidRequest); }
        let schema_version = u16::from_le_bytes([input[0], input[1]]);
        let workload_kind = match input[2] { 1 => WorkloadKind::CpuChecksum, 2 => WorkloadKind::CpuBoundedCounter, 3 => WorkloadKind::AcceleratorInferenceFixture, _ => return Err(ComputeError::InvalidRequest) };
        let locality = match input[3] { 1 => ComputeLocality::RequireLocal, 2 => ComputeLocality::PreferLocal, 3 => ComputeLocality::PreferRemote, 4 => ComputeLocality::RequireRemote, _ => return Err(ComputeError::InvalidRequest) };
        let durability = match input[4] { 1 => ComputeDurability::Pinned, 2 => ComputeDurability::Restartable, 3 => ComputeDurability::CheckpointableScaffold, 4 => ComputeDurability::MigratableScaffold, _ => return Err(ComputeError::InvalidRequest) };
        if schema_version != COMPUTE_SCHEMA_VERSION || input[6] & !0x07 != 0 || ((input[6] >> 1) & 0x03) > 2 || input[7] > 2 || input[190] > 1 || input[191] != COMPUTE_RESULT_CONTRACT_V1 { return Err(ComputeError::InvalidRequest); }
        let mut workload_id = [0u8; 16]; workload_id.copy_from_slice(&input[8..24]);
        let mut input_refs = [[0u8; 16]; 2]; input_refs[0].copy_from_slice(&input[24..40]); input_refs[1].copy_from_slice(&input[40..56]);
        let mut allowed_nodes = [NodeId([0; 32]); 2]; allowed_nodes[0].0.copy_from_slice(&input[56..88]); allowed_nodes[1].0.copy_from_slice(&input[88..120]);
        Ok(Self {
            schema_version, workload_kind, locality, durability, priority: input[5], privacy_local_only: input[6] == 1,
            workload_id, input_refs, allowed_nodes, allowed_node_count: input[7],
            allowed_domains: [read_u64(input, 120), read_u64(input, 128)], allowed_domain_count: (input[6] >> 1) & 0x03,
            memory_bytes: read_u64(input, 136), deadline: read_u64(input, 144), correlation_id: read_u64(input, 152),
            capability_ref: read_u64(input, 160), affinity: read_u64(input, 168), anti_affinity: read_u64(input, 176),
            work_units: read_u32(input, 184), cpu_units: u16::from_le_bytes([input[188], input[189]]),
            restart_eligible: input[190] == 1, result_contract: input[191],
        })
    }

    // ------------------------=
    // FUNC: authority_target
    // DESC: Derives the exact stable capability target from the workload identity.
    // ------------------=
    pub fn authority_target(&self) -> u64 {
        u64::from_le_bytes(self.workload_id[..8].try_into().unwrap_or([0; 8]))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeComputeObservation {
    pub node: NodeId,
    pub trust_domain: u64,
    pub latency_us: u32,
    pub latency_known: bool,
    pub load_percent: u8,
    pub generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComputeAccounting {
    pub reserved_cpu_units: u16,
    pub reserved_memory_bytes: u64,
    pub used_cpu_ticks: u64,
    pub started_at: u64,
    pub finished_at: u64,
    pub restart_count: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComputeSnapshot {
    pub task_id: u64,
    pub epoch: u32,
    pub state: ComputeState,
    pub error: Option<ComputeError>,
    pub node: NodeId,
    pub resource: ResourceId,
    pub correlation_id: u64,
    pub workload_kind: WorkloadKind,
    pub locality: ComputeLocality,
    pub durability: ComputeDurability,
    pub privacy_local_only: bool,
    pub deadline: u64,
    pub context: Option<ContextHandle>,
    pub output_digest: [u8; 16],
    pub accounting: ComputeAccounting,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComputeResultV1 {
    pub schema_version: u16,
    pub snapshot: ComputeSnapshot,
}

impl ComputeResultV1 {
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes a fixed-width result and accounting contract suitable for native IOP response payloads.
    // ------------------=
    pub fn encode(self) -> [u8; COMPUTE_RESULT_V1_BYTES] {
        let mut out = [0u8; COMPUTE_RESULT_V1_BYTES];
        out[0..2].copy_from_slice(&self.schema_version.to_le_bytes()); out[2] = self.snapshot.state as u8; out[3] = self.snapshot.error.map(|error| error as u8).unwrap_or(0);
        out[4..8].copy_from_slice(&self.snapshot.epoch.to_le_bytes()); out[8..16].copy_from_slice(&self.snapshot.task_id.to_le_bytes()); out[16..24].copy_from_slice(&self.snapshot.correlation_id.to_le_bytes());
        out[24..56].copy_from_slice(&self.snapshot.node.0); out[56..72].copy_from_slice(&self.snapshot.resource.0); out[72..88].copy_from_slice(&self.snapshot.output_digest);
        out[88..96].copy_from_slice(&self.snapshot.accounting.used_cpu_ticks.to_le_bytes()); out[96..104].copy_from_slice(&self.snapshot.accounting.reserved_memory_bytes.to_le_bytes());
        out[104..112].copy_from_slice(&self.snapshot.accounting.started_at.to_le_bytes()); out[112..120].copy_from_slice(&self.snapshot.accounting.finished_at.to_le_bytes());
        out[120..122].copy_from_slice(&self.snapshot.accounting.restart_count.to_le_bytes()); out[122..124].copy_from_slice(&self.snapshot.accounting.reserved_cpu_units.to_le_bytes());
        if self.snapshot.context.is_some() { out[124] = 1; }
        out[128] = self.snapshot.workload_kind as u8; out[129] = self.snapshot.locality as u8; out[130] = self.snapshot.durability as u8; out[131] = self.snapshot.privacy_local_only as u8; out[132..140].copy_from_slice(&self.snapshot.deadline.to_le_bytes());
        out
    }

    // ------------------------=
    // FUNC: decode
    // DESC: Decodes a versioned compute result while rejecting unknown lifecycle and error codes.
    // ------------------=
    pub fn decode(input: &[u8]) -> Result<Self, ComputeError> {
        if input.len() != COMPUTE_RESULT_V1_BYTES || read_u16(input, 0) != COMPUTE_SCHEMA_VERSION || input[124] > 1 || input[125..128].iter().any(|byte| *byte != 0) || input[131] > 1 || input[140..].iter().any(|byte| *byte != 0) { return Err(ComputeError::InvalidRequest); }
        let state = decode_state(input[2])?; let error = if input[3] == 0 { None } else { Some(decode_error(input[3])?) };
        let workload_kind = match input[128] { 1 => WorkloadKind::CpuChecksum, 2 => WorkloadKind::CpuBoundedCounter, 3 => WorkloadKind::AcceleratorInferenceFixture, _ => return Err(ComputeError::InvalidRequest) };
        let locality = match input[129] { 1 => ComputeLocality::RequireLocal, 2 => ComputeLocality::PreferLocal, 3 => ComputeLocality::PreferRemote, 4 => ComputeLocality::RequireRemote, _ => return Err(ComputeError::InvalidRequest) };
        let durability = match input[130] { 1 => ComputeDurability::Pinned, 2 => ComputeDurability::Restartable, 3 => ComputeDurability::CheckpointableScaffold, 4 => ComputeDurability::MigratableScaffold, _ => return Err(ComputeError::InvalidRequest) };
        let mut node = [0; 32]; node.copy_from_slice(&input[24..56]); let mut resource = [0; 16]; resource.copy_from_slice(&input[56..72]); let mut output_digest = [0; 16]; output_digest.copy_from_slice(&input[72..88]);
        Ok(Self { schema_version: COMPUTE_SCHEMA_VERSION, snapshot: ComputeSnapshot { task_id: read_u64(input, 8), epoch: read_u32(input, 4), state, error, node: NodeId(node), resource: ResourceId(resource), correlation_id: read_u64(input, 16), workload_kind, locality, durability, privacy_local_only: input[131] == 1, deadline: read_u64(input, 132), context: None, output_digest, accounting: ComputeAccounting { reserved_cpu_units: read_u16(input, 122), reserved_memory_bytes: read_u64(input, 96), used_cpu_ticks: read_u64(input, 88), started_at: read_u64(input, 104), finished_at: read_u64(input, 112), restart_count: read_u16(input, 120) } } })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComputeNotice {
    pub sequence: u64,
    pub task_id: u64,
    pub epoch: u32,
    pub kind: ComputeEventKind,
    pub state: ComputeState,
    pub node: NodeId,
    pub correlation_id: u64,
}

impl ComputeNotice {
    // ------------------------=
    // FUNC: event_type
    // DESC: Maps a lifecycle notice onto the stable Infinity Event Fabric compute registry.
    // ------------------=
    pub fn event_type(self) -> u32 {
        match self.kind {
            ComputeEventKind::Queued => super::EVENT_COMPUTE_QUEUED, ComputeEventKind::Placed => super::EVENT_COMPUTE_PLACED,
            ComputeEventKind::Started => super::EVENT_COMPUTE_STARTED, ComputeEventKind::Completed => super::EVENT_COMPUTE_COMPLETED,
            ComputeEventKind::Failed => super::EVENT_COMPUTE_FAILED, ComputeEventKind::Cancelled => super::EVENT_COMPUTE_CANCELLED,
            ComputeEventKind::Restarted => super::EVENT_COMPUTE_RESTARTED, ComputeEventKind::NodeLost => super::EVENT_COMPUTE_NODE_LOST,
        }
    }

    // ------------------------=
    // FUNC: encode_payload
    // DESC: Encodes notification-only lifecycle data while leaving authoritative recovery to Compute.Inspect.
    // ------------------=
    pub fn encode_payload(self) -> [u8; 64] {
        let mut out = [0; 64]; out[0..2].copy_from_slice(&COMPUTE_SCHEMA_VERSION.to_le_bytes()); out[2] = self.kind as u8; out[3] = self.state as u8;
        out[4..8].copy_from_slice(&self.epoch.to_le_bytes()); out[8..16].copy_from_slice(&self.task_id.to_le_bytes()); out[16..24].copy_from_slice(&self.sequence.to_le_bytes()); out[24..32].copy_from_slice(&self.correlation_id.to_le_bytes()); out[32..64].copy_from_slice(&self.node.0); out
    }
}

// ------------------------=
// FUNC: publish_notice
// DESC: Publishes one compute lifecycle notification through IEF under an explicit event capability.
// ------------------=
pub fn publish_notice(notice: ComputeNotice, source: super::execution::SecurityIdentity, event_capability: CapabilityId, events: &mut super::event::EventFabric, capabilities: &CapabilityManager, now: u64) -> Result<u64, super::event::EventError> {
    events.publish(super::event::EventClass::StateChange, super::event::RoutingDomain::Mesh, notice.event_type(), source, notice.task_id, notice.correlation_id, notice.sequence, &notice.encode_payload(), 160, now, capabilities, event_capability)
}

#[derive(Clone, Copy)]
struct Reservation {
    resource: ResourceId,
    generation: u64,
    kind: ResourceKind,
    amount: u64,
}

#[derive(Clone, Copy)]
struct Placement {
    node: NodeId,
    compute: Reservation,
    memory: Reservation,
}

#[derive(Clone, Copy)]
struct ComputeTask {
    request: ComputeRequestV1,
    snapshot: ComputeSnapshot,
    compute_reservation: Reservation,
    memory_reservation: Reservation,
    progress: u32,
}

pub struct ComputeService {
    tasks: [Option<ComputeTask>; MAX_COMPUTE_TASKS],
    observations: [Option<NodeComputeObservation>; MAX_COMPUTE_NODES],
    archive: [Option<ComputeSnapshot>; MAX_COMPUTE_TASKS],
    notices: [Option<ComputeNotice>; MAX_COMPUTE_NOTICES],
    notice_len: usize,
    next_task: u64,
    next_notice: u64,
    dirty: bool,
}

impl ComputeService {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty bounded compute authority with no implicit nodes or capacity.
    // ------------------=
    pub const fn new() -> Self {
        Self { tasks: [None; MAX_COMPUTE_TASKS], observations: [None; MAX_COMPUTE_NODES], archive: [None; MAX_COMPUTE_TASKS], notices: [None; MAX_COMPUTE_NOTICES], notice_len: 0, next_task: 1, next_notice: 1, dirty: false }
    }

    // ------------------------=
    // FUNC: observe_node
    // DESC: Records scheduler metadata only for a node already represented by actual Resource Fabric inventory.
    // ------------------=
    pub fn observe_node(&mut self, directory: &Directory, observation: NodeComputeObservation) -> Result<(), ComputeError> {
        if observation.node.0 == [0; 32] || observation.generation == 0 || observation.load_percent > 100
            || !directory.entries().iter().flatten().any(|resource| resource.owner == observation.node) { return Err(ComputeError::InvalidRequest); }
        let slot = self.observations.iter().position(|entry| entry.is_some_and(|entry| entry.node == observation.node))
            .or_else(|| self.observations.iter().position(Option::is_none)).ok_or(ComputeError::Full)?;
        if self.observations[slot].is_some_and(|old| observation.generation < old.generation) { return Err(ComputeError::InvalidRequest); }
        self.observations[slot] = Some(observation);
        Ok(())
    }

    // ------------------------=
    // FUNC: request
    // DESC: Validates authority and policy, reserves advertised compute plus memory, and creates one authoritative queued lifecycle.
    // ------------------=
    pub fn request(&mut self, request: ComputeRequestV1, caller: super::execution::SecurityIdentity, capabilities: &CapabilityManager, directory: &mut Directory, local: NodeId, now: u64) -> Result<u64, ComputeError> {
        validate_request(&request, now)?;
        capabilities.validate(request.capability_ref, caller, CapabilityType::ComputeUse, request.authority_target(), COMPUTE_RIGHT_EXECUTE, 0, now).map_err(|_| ComputeError::AccessDenied)?;
        let slot = self.tasks.iter().position(Option::is_none).ok_or(ComputeError::Full)?;
        let placement = self.select_and_reserve(&request, directory, local, now, None)?;
        let task_id = self.next_task; self.next_task = self.next_task.wrapping_add(1).max(1);
        let accounting = ComputeAccounting { reserved_cpu_units: request.cpu_units, reserved_memory_bytes: request.memory_bytes, used_cpu_ticks: 0, started_at: 0, finished_at: 0, restart_count: 0 };
        let snapshot = ComputeSnapshot { task_id, epoch: 1, state: ComputeState::Queued, error: None, node: placement.node, resource: placement.compute.resource, correlation_id: request.correlation_id, workload_kind: request.workload_kind, locality: request.locality, durability: request.durability, privacy_local_only: request.privacy_local_only, deadline: request.deadline, context: None, output_digest: [0; 16], accounting };
        self.tasks[slot] = Some(ComputeTask { request, snapshot, compute_reservation: placement.compute, memory_reservation: placement.memory, progress: 0 });
        self.emit(slot, ComputeEventKind::Queued);
        self.tasks[slot].as_mut().unwrap().snapshot.state = ComputeState::Placed;
        self.emit(slot, ComputeEventKind::Placed);
        Ok(task_id)
    }

    // ------------------------=
    // FUNC: start
    // DESC: Revalidates task authority and creates a genuinely bounded execution context on the selected node runtime.
    // ------------------=
    pub fn start(&mut self, task_id: u64, epoch: u32, caller: super::execution::SecurityIdentity, capabilities: &CapabilityManager, execution: &mut ExecutionManager, directory: &mut Directory, now: u64) -> Result<ContextHandle, ComputeError> {
        let index = self.task_index(task_id)?; let task = self.tasks[index].unwrap();
        if task.snapshot.epoch != epoch { return Err(ComputeError::StaleResult); }
        if task.snapshot.state != ComputeState::Placed { return Err(ComputeError::InvalidState); }
        if task.request.deadline != 0 && now >= task.request.deadline { self.release(index, directory); self.terminal(index, ComputeState::Failed, Some(ComputeError::DeadlineExceeded), ComputeEventKind::Failed, None, now); return Err(ComputeError::DeadlineExceeded); }
        if capabilities.validate(task.request.capability_ref, caller, CapabilityType::ComputeUse, task.request.authority_target(), COMPUTE_RIGHT_EXECUTE, 0, now).is_err() { self.release(index, directory); self.terminal(index, ComputeState::Failed, Some(ComputeError::AccessDenied), ComputeEventKind::Failed, None, now); return Err(ComputeError::AccessDenied); }
        let handle = execution.create(0x1100, workload_image(task.request.workload_kind), MemoryRegion { base: context_base(task_id), length: task.request.memory_bytes }, 0x6c00u16.saturating_add(index as u16), priority(task.request.priority), ResourceBudget { memory_limit: task.request.memory_bytes, cpu_weight: task.request.cpu_units, message_queue_limit: 8, io_priority: 2 }).map_err(|_| ComputeError::Context)?;
        execution.account_memory(handle, task.request.memory_bytes).map_err(|_| ComputeError::Context)?;
        execution.set_state(handle, ContextState::Running).map_err(|_| ComputeError::Context)?;
        let current = self.tasks[index].as_mut().unwrap(); current.snapshot.context = Some(handle); current.snapshot.state = ComputeState::Running; current.snapshot.accounting.started_at = now;
        self.emit(index, ComputeEventKind::Started);
        Ok(handle)
    }

    // ------------------------=
    // FUNC: run_slice
    // DESC: Executes a strictly bounded deterministic slice, accounts every tick, and fences results by execution epoch.
    // ------------------=
    pub fn run_slice(&mut self, task_id: u64, epoch: u32, caller: super::execution::SecurityIdentity, capabilities: &CapabilityManager, execution: &mut ExecutionManager, requested_ticks: u32, directory: &mut Directory, now: u64) -> Result<ComputeSnapshot, ComputeError> {
        let index = self.task_index(task_id)?; let task = self.tasks[index].unwrap();
        if task.snapshot.epoch != epoch { return Err(ComputeError::StaleResult); }
        if task.snapshot.state != ComputeState::Running { return Err(ComputeError::InvalidState); }
        if task.request.deadline != 0 && now >= task.request.deadline { self.finish(index, execution, directory, ComputeState::Failed, Some(ComputeError::DeadlineExceeded), ComputeEventKind::Failed, now); return Err(ComputeError::DeadlineExceeded); }
        if capabilities.validate(task.request.capability_ref, caller, CapabilityType::ComputeUse, task.request.authority_target(), COMPUTE_RIGHT_EXECUTE, 0, now).is_err() { self.finish(index, execution, directory, ComputeState::Failed, Some(ComputeError::AccessDenied), ComputeEventKind::Failed, now); return Err(ComputeError::AccessDenied); }
        let ticks = requested_ticks.min(MAX_SLICE_TICKS).min(task.request.work_units.saturating_sub(task.progress));
        let handle = task.snapshot.context.ok_or(ComputeError::Context)?;
        for _ in 0..ticks { execution.account_cpu_tick(handle).map_err(|_| ComputeError::Context)?; }
        let current = self.tasks[index].as_mut().unwrap(); current.progress = current.progress.saturating_add(ticks); current.snapshot.accounting.used_cpu_ticks = current.snapshot.accounting.used_cpu_ticks.saturating_add(ticks as u64);
        if current.progress >= current.request.work_units {
            current.snapshot.output_digest = output_digest(&current.request, current.snapshot.epoch);
            self.finish(index, execution, directory, ComputeState::Completed, None, ComputeEventKind::Completed, now);
        }
        Ok(self.tasks[index].unwrap().snapshot)
    }

    // ------------------------=
    // FUNC: cancel
    // DESC: Cancels one task under independently scoped authority and releases its execution context and reservations.
    // ------------------=
    pub fn cancel(&mut self, task_id: u64, caller: super::execution::SecurityIdentity, cancel_capability: CapabilityId, capabilities: &CapabilityManager, execution: &mut ExecutionManager, directory: &mut Directory, now: u64) -> Result<(), ComputeError> {
        capabilities.validate(cancel_capability, caller, CapabilityType::ComputeCancel, task_id, 1, 0, now).map_err(|_| ComputeError::AccessDenied)?;
        let index = self.task_index(task_id)?;
        if matches!(self.tasks[index].unwrap().snapshot.state, ComputeState::Completed | ComputeState::Failed | ComputeState::Cancelled) { return Err(ComputeError::InvalidState); }
        self.finish(index, execution, directory, ComputeState::Cancelled, Some(ComputeError::Cancelled), ComputeEventKind::Cancelled, now);
        Ok(())
    }

    // ------------------------=
    // FUNC: node_lost
    // DESC: Fences a lost execution and either fails pinned work or places restartable work on distinct live capacity.
    // ------------------=
    pub fn node_lost(&mut self, peer: NodeId, directory: &mut Directory, local: NodeId, now: u64) {
        for index in 0..MAX_COMPUTE_TASKS {
            let Some(task) = self.tasks[index] else { continue; };
            if task.snapshot.node != peer || !matches!(task.snapshot.state, ComputeState::Placed | ComputeState::Running) { continue; }
            self.release(index, directory);
            self.tasks[index].as_mut().unwrap().snapshot.state = ComputeState::NodeLost;
            self.tasks[index].as_mut().unwrap().snapshot.context = None;
            self.emit(index, ComputeEventKind::NodeLost);
            if task.request.durability == ComputeDurability::Restartable && task.request.restart_eligible {
                match self.select_and_reserve(&task.request, directory, local, now, Some(peer)) {
                    Ok(placement) => {
                        let current = self.tasks[index].as_mut().unwrap();
                        current.snapshot.epoch = current.snapshot.epoch.saturating_add(1); current.snapshot.node = placement.node; current.snapshot.resource = placement.compute.resource; current.snapshot.state = ComputeState::Placed; current.snapshot.error = None; current.snapshot.accounting.restart_count = current.snapshot.accounting.restart_count.saturating_add(1); current.compute_reservation = placement.compute; current.memory_reservation = placement.memory; current.progress = 0;
                        self.emit(index, ComputeEventKind::Restarted); self.emit(index, ComputeEventKind::Placed);
                    }
                    Err(error) => self.terminal(index, ComputeState::Failed, Some(error), ComputeEventKind::Failed, None, now),
                }
            } else {
                self.terminal(index, ComputeState::Failed, Some(ComputeError::NodeLost), ComputeEventKind::Failed, None, now);
            }
        }
    }

    // ------------------------=
    // FUNC: inspect
    // DESC: Returns the authoritative lifecycle snapshot used by both UI and console adapters.
    // ------------------=
    pub fn inspect(&self, task_id: u64) -> Result<ComputeSnapshot, ComputeError> {
        if let Some(task) = self.tasks.iter().flatten().find(|task| task.snapshot.task_id == task_id) { return Ok(task.snapshot); }
        self.archive.iter().flatten().find(|task| task.task_id == task_id).copied().ok_or(ComputeError::UnknownTask)
    }

    // ------------------------=
    // FUNC: task_count
    // DESC: Reports the bounded authoritative distributed task count.
    // ------------------=
    pub fn task_count(&self) -> usize { self.tasks.iter().flatten().count() + self.archive.iter().flatten().count() }

    // ------------------------=
    // FUNC: task_nth
    // DESC: Lists one authoritative distributed task snapshot by stable service order.
    // ------------------=
    pub fn task_nth(&self, index: usize) -> Option<ComputeSnapshot> {
        let active = self.tasks.iter().flatten().count();
        if index < active { self.tasks.iter().flatten().nth(index).map(|task| task.snapshot) } else { self.archive.iter().flatten().nth(index - active).copied() }
    }

    // ------------------------=
    // FUNC: encode_state
    // DESC: Encodes bounded task and audit state for transactional System Space persistence across installed boots.
    // ------------------=
    pub fn encode_state(&self) -> [u8; COMPUTE_STATE_BYTES] {
        let mut out = [0; COMPUTE_STATE_BYTES]; out[..8].copy_from_slice(b"INFCMP01"); out[8..10].copy_from_slice(&COMPUTE_SCHEMA_VERSION.to_le_bytes()); out[16..24].copy_from_slice(&self.next_task.to_le_bytes());
        let mut count = 0usize;
        for snapshot in self.tasks.iter().flatten().map(|task| task.snapshot).chain(self.archive.iter().flatten().copied()).take(MAX_COMPUTE_TASKS) {
            let start = 32 + count * COMPUTE_RESULT_V1_BYTES; out[start..start + COMPUTE_RESULT_V1_BYTES].copy_from_slice(&ComputeResultV1 { schema_version: COMPUTE_SCHEMA_VERSION, snapshot }.encode()); count += 1;
        }
        out[10..12].copy_from_slice(&(count as u16).to_le_bytes()); let checksum = state_checksum(&out[..COMPUTE_STATE_BYTES - 4]); out[COMPUTE_STATE_BYTES - 4..].copy_from_slice(&checksum.to_le_bytes()); out
    }

    // ------------------------=
    // FUNC: restore_state
    // DESC: Restores persisted audit snapshots and converts interrupted work into explicit terminal node-loss state.
    // ------------------=
    pub fn restore_state(&mut self, input: &[u8]) -> Result<(), ComputeError> {
        if input.len() != COMPUTE_STATE_BYTES || &input[..8] != b"INFCMP01" || read_u16(input, 8) != COMPUTE_SCHEMA_VERSION || read_u32(input, COMPUTE_STATE_BYTES - 4) != state_checksum(&input[..COMPUTE_STATE_BYTES - 4]) { return Err(ComputeError::InvalidRequest); }
        let count = read_u16(input, 10) as usize; if count > MAX_COMPUTE_TASKS || input[12..16].iter().any(|byte| *byte != 0) || input[24..32].iter().any(|byte| *byte != 0) { return Err(ComputeError::InvalidRequest); }
        let mut restored: [Option<ComputeSnapshot>; MAX_COMPUTE_TASKS] = [None; MAX_COMPUTE_TASKS]; let mut greatest = 0u64;
        for index in 0..count {
            let start = 32 + index * COMPUTE_RESULT_V1_BYTES; let mut snapshot = ComputeResultV1::decode(&input[start..start + COMPUTE_RESULT_V1_BYTES])?.snapshot;
            if restored[..index].iter().flatten().any(|prior| prior.task_id == snapshot.task_id) { return Err(ComputeError::InvalidRequest); }
            if matches!(snapshot.state, ComputeState::Queued | ComputeState::Placed | ComputeState::Running | ComputeState::NodeLost) { snapshot.state = ComputeState::Failed; snapshot.error = Some(ComputeError::NodeLost); snapshot.context = None; }
            greatest = greatest.max(snapshot.task_id); restored[index] = Some(snapshot);
        }
        self.tasks = [None; MAX_COMPUTE_TASKS]; self.archive = restored; self.next_task = read_u64(input, 16).max(greatest.saturating_add(1)).max(1); self.dirty = false; Ok(())
    }

    // ------------------------=
    // FUNC: needs_persistence
    // DESC: Reports whether authoritative task state has changed since the last successful transactional checkpoint.
    // ------------------=
    pub fn needs_persistence(&self) -> bool { self.dirty }

    // ------------------------=
    // FUNC: mark_persisted
    // DESC: Clears the dirty marker only after the owning installed storage path commits successfully.
    // ------------------=
    pub fn mark_persisted(&mut self) { self.dirty = false; }

    // ------------------------=
    // FUNC: notice_count
    // DESC: Reports retained monotonic lifecycle notifications without treating them as authoritative state.
    // ------------------=
    pub fn notice_count(&self) -> usize { self.notice_len }

    // ------------------------=
    // FUNC: notice_nth
    // DESC: Reads one retained lifecycle notification for event delivery and gap-recovery tests.
    // ------------------=
    pub fn notice_nth(&self, index: usize) -> Option<ComputeNotice> { self.notices.get(index).copied().flatten() }

    // ------------------------=
    // FUNC: task_index
    // DESC: Resolves a public task identity into bounded authoritative storage.
    // ------------------=
    fn task_index(&self, task_id: u64) -> Result<usize, ComputeError> { self.tasks.iter().position(|task| task.is_some_and(|task| task.snapshot.task_id == task_id)).ok_or(ComputeError::UnknownTask) }

    // ------------------------=
    // FUNC: select_and_reserve
    // DESC: Selects live policy-compatible capacity and transactionally reserves compute plus memory generations.
    // ------------------=
    fn select_and_reserve(&self, request: &ComputeRequestV1, directory: &mut Directory, local: NodeId, now: u64, excluded: Option<NodeId>) -> Result<Placement, ComputeError> {
        let compute_kind = if request.workload_kind == WorkloadKind::AcceleratorInferenceFixture { ResourceKind::Accelerator } else { ResourceKind::Compute };
        let mut best: Option<(u64, Resource, Resource)> = None;
        for (index, entry) in directory.entries().iter().enumerate() {
            let Some(compute) = *entry else { continue; };
            if compute.kind != compute_kind || directory.usable_kind(index, compute_kind, now) < request.cpu_units as u64 || excluded == Some(compute.owner) || !self.node_allowed(request, compute.owner, local) { continue; }
            let Some(observation) = self.observations.iter().flatten().find(|entry| entry.node == compute.owner) else { continue; };
            if request.allowed_domain_count != 0 && !request.allowed_domains[..request.allowed_domain_count.min(2) as usize].contains(&observation.trust_domain) { continue; }
            let Some(memory) = directory.entries().iter().enumerate().find_map(|(memory_index, entry)| entry.filter(|resource| resource.owner == compute.owner && resource.kind == ResourceKind::Memory && directory.usable_kind(memory_index, ResourceKind::Memory, now) >= request.memory_bytes)) else { continue; };
            let local_penalty = match (request.locality, compute.owner == local) { (ComputeLocality::PreferLocal, false) | (ComputeLocality::PreferRemote, true) => 1_000_000, _ => 0 };
            let affinity_bonus = if request.affinity != 0 && self.tasks.iter().flatten().any(|task| task.request.affinity == request.affinity && task.snapshot.node == compute.owner && task.snapshot.state == ComputeState::Running) { 50_000 } else { 0 };
            let anti_penalty = if request.anti_affinity != 0 && self.tasks.iter().flatten().any(|task| task.request.affinity == request.anti_affinity && task.snapshot.node == compute.owner && matches!(task.snapshot.state, ComputeState::Placed | ComputeState::Running)) { 2_000_000 } else { 0 };
            let latency_score = if observation.latency_known { observation.latency_us as u64 } else { 250_000 };
            let score = local_penalty + anti_penalty + observation.load_percent as u64 * 10_000 + latency_score;
            let score = score.saturating_sub(affinity_bonus);
            if best.is_none_or(|candidate| score < candidate.0) { best = Some((score, compute, memory)); }
        }
        let (_, compute, memory) = best.ok_or(ComputeError::NoPlacement)?;
        directory.reserve_kind(compute.id, compute.generation, compute.kind, request.cpu_units as u64, now).map_err(|_| ComputeError::ResourceUnavailable)?;
        if directory.reserve_kind(memory.id, memory.generation, ResourceKind::Memory, request.memory_bytes, now).is_err() {
            let _ = directory.release(compute.id, compute.generation, request.cpu_units as u64);
            return Err(ComputeError::ResourceUnavailable);
        }
        Ok(Placement { node: compute.owner, compute: Reservation { resource: compute.id, generation: compute.generation, kind: compute.kind, amount: request.cpu_units as u64 }, memory: Reservation { resource: memory.id, generation: memory.generation, kind: ResourceKind::Memory, amount: request.memory_bytes } })
    }

    // ------------------------=
    // FUNC: node_allowed
    // DESC: Enforces privacy, locality, and explicit node allowlists before scoring a placement.
    // ------------------=
    fn node_allowed(&self, request: &ComputeRequestV1, node: NodeId, local: NodeId) -> bool {
        if request.privacy_local_only && node != local { return false; }
        if request.locality == ComputeLocality::RequireLocal && node != local { return false; }
        if request.locality == ComputeLocality::RequireRemote && node == local { return false; }
        request.allowed_node_count == 0 || request.allowed_nodes[..request.allowed_node_count.min(2) as usize].contains(&node)
    }

    // ------------------------=
    // FUNC: finish
    // DESC: Captures final accounting, destroys a live context, releases reservations, and emits one terminal notice.
    // ------------------=
    fn finish(&mut self, index: usize, execution: &mut ExecutionManager, directory: &mut Directory, state: ComputeState, error: Option<ComputeError>, event: ComputeEventKind, now: u64) {
        if let Some(handle) = self.tasks[index].unwrap().snapshot.context { let _ = execution.destroy(handle); }
        self.release(index, directory);
        self.terminal(index, state, error, event, None, now);
    }

    // ------------------------=
    // FUNC: release
    // DESC: Releases both exact resource-generation reservations once without changing lifecycle state.
    // ------------------=
    fn release(&mut self, index: usize, directory: &mut Directory) {
        let task = self.tasks[index].unwrap();
        let _ = directory.release(task.compute_reservation.resource, task.compute_reservation.generation, task.compute_reservation.amount);
        let _ = directory.release(task.memory_reservation.resource, task.memory_reservation.generation, task.memory_reservation.amount);
    }

    // ------------------------=
    // FUNC: terminal
    // DESC: Commits a terminal authoritative state and its diagnostic reason before notification.
    // ------------------=
    fn terminal(&mut self, index: usize, state: ComputeState, error: Option<ComputeError>, event: ComputeEventKind, context: Option<ContextHandle>, now: u64) {
        let task = self.tasks[index].as_mut().unwrap(); task.snapshot.state = state; task.snapshot.error = error; task.snapshot.context = context; task.snapshot.accounting.finished_at = now;
        self.emit(index, event);
    }

    // ------------------------=
    // FUNC: emit
    // DESC: Appends a monotonic bounded lifecycle notification while authoritative state remains inspectable separately.
    // ------------------=
    fn emit(&mut self, index: usize, kind: ComputeEventKind) {
        if self.notice_len == MAX_COMPUTE_NOTICES { for i in 1..MAX_COMPUTE_NOTICES { self.notices[i - 1] = self.notices[i]; } self.notice_len -= 1; }
        let snapshot = self.tasks[index].unwrap().snapshot;
        self.notices[self.notice_len] = Some(ComputeNotice { sequence: self.next_notice, task_id: snapshot.task_id, epoch: snapshot.epoch, kind, state: snapshot.state, node: snapshot.node, correlation_id: snapshot.correlation_id });
        self.notice_len += 1; self.next_notice = self.next_notice.wrapping_add(1).max(1);
        self.dirty = true;
    }
}

// ------------------------=
// FUNC: validate_request
// DESC: Rejects unbounded, expired, unsupported, or internally inconsistent compute contracts before placement.
// ------------------=
fn validate_request(request: &ComputeRequestV1, now: u64) -> Result<(), ComputeError> {
    if request.schema_version != COMPUTE_SCHEMA_VERSION || request.workload_id == [0; 16] || request.cpu_units == 0 || request.memory_bytes == 0 || request.memory_bytes > MAX_COMPUTE_MEMORY || request.work_units == 0 || request.allowed_node_count > 2 || request.allowed_domain_count > 2 || request.result_contract != COMPUTE_RESULT_CONTRACT_V1 { return Err(ComputeError::InvalidRequest); }
    if request.deadline != 0 && now >= request.deadline { return Err(ComputeError::DeadlineExceeded); }
    if matches!(request.durability, ComputeDurability::CheckpointableScaffold | ComputeDurability::MigratableScaffold) { return Err(ComputeError::UnsupportedDurability); }
    if request.durability == ComputeDurability::Restartable && !request.restart_eligible { return Err(ComputeError::InvalidRequest); }
    Ok(())
}

// ------------------------=
// FUNC: priority
// DESC: Maps the stable wire priority into the native execution manager priority class.
// ------------------=
fn priority(value: u8) -> PriorityClass { match value { 0..=63 => PriorityClass::Background, 64..=191 => PriorityClass::Normal, 192..=239 => PriorityClass::System, _ => PriorityClass::Critical } }

// ------------------------=
// FUNC: workload_image
// DESC: Maps a typed workload to a stable native execution image identity.
// ------------------=
fn workload_image(kind: WorkloadKind) -> u32 { 0x1100 + kind as u32 }

// ------------------------=
// FUNC: context_base
// DESC: Assigns each bounded task slot a disjoint fixed virtual region for host and kernel execution-manager verification.
// ------------------=
fn context_base(task_id: u64) -> u64 { 0x4000_0000u64.saturating_add((task_id % MAX_COMPUTE_TASKS as u64).saturating_mul(0x0800_0000)) }

// ------------------------=
// FUNC: output_digest
// DESC: Produces a deterministic fixture result bound to inputs, workload, and fenced execution epoch.
// ------------------=
fn output_digest(request: &ComputeRequestV1, epoch: u32) -> [u8; 16] {
    output_digest_parts(request.workload_id, request.input_refs, epoch, request.work_units)
}

// ------------------------=
// FUNC: output_digest_parts
// DESC: Produces the same deterministic result identity for coordinator requests and remote dispatch slices.
// ------------------=
fn output_digest_parts(workload_id: [u8; 16], input_refs: [[u8; 16]; 2], epoch: u32, work_units: u32) -> [u8; 16] {
    let mut out = workload_id;
    for index in 0..16 { out[index] = out[index].wrapping_add(input_refs[0][index]).rotate_left((index % 7) as u32).wrapping_add(input_refs[1][15 - index]); }
    for (index, byte) in work_units.to_le_bytes().iter().chain(epoch.to_le_bytes().iter()).enumerate() { out[index] ^= *byte; out[15 - index] = out[15 - index].wrapping_add(*byte); }
    out
}

// ------------------------=
// FUNC: read_u64
// DESC: Reads one little-endian u64 from an already length-validated compute payload.
// ------------------=
fn read_u64(input: &[u8], offset: usize) -> u64 { u64::from_le_bytes(input[offset..offset + 8].try_into().unwrap_or([0; 8])) }

// ------------------------=
// FUNC: read_u32
// DESC: Reads one little-endian u32 from an already length-validated compute payload.
// ------------------=
fn read_u32(input: &[u8], offset: usize) -> u32 { u32::from_le_bytes(input[offset..offset + 4].try_into().unwrap_or([0; 4])) }

// ------------------------=
// FUNC: read_u16
// DESC: Reads one little-endian u16 from an already length-validated compute payload.
// ------------------=
fn read_u16(input: &[u8], offset: usize) -> u16 { u16::from_le_bytes(input[offset..offset + 2].try_into().unwrap_or([0; 2])) }

// ------------------------=
// FUNC: decode_state
// DESC: Rejects unknown wire lifecycle state values.
// ------------------=
fn decode_state(value: u8) -> Result<ComputeState, ComputeError> { match value { 1 => Ok(ComputeState::Queued), 2 => Ok(ComputeState::Placed), 3 => Ok(ComputeState::Running), 4 => Ok(ComputeState::Completed), 5 => Ok(ComputeState::Failed), 6 => Ok(ComputeState::Cancelled), 7 => Ok(ComputeState::NodeLost), _ => Err(ComputeError::InvalidRequest) } }

// ------------------------=
// FUNC: decode_error
// DESC: Rejects unknown wire compute error values.
// ------------------=
fn decode_error(value: u8) -> Result<ComputeError, ComputeError> { match value { 1 => Ok(ComputeError::AccessDenied), 2 => Ok(ComputeError::InvalidRequest), 3 => Ok(ComputeError::DeadlineExceeded), 4 => Ok(ComputeError::NoPlacement), 5 => Ok(ComputeError::Full), 6 => Ok(ComputeError::UnknownTask), 7 => Ok(ComputeError::InvalidState), 8 => Ok(ComputeError::Cancelled), 9 => Ok(ComputeError::NodeLost), 10 => Ok(ComputeError::UnsupportedDurability), 11 => Ok(ComputeError::StaleResult), 12 => Ok(ComputeError::ResourceUnavailable), 13 => Ok(ComputeError::Context), _ => Err(ComputeError::InvalidRequest) } }

// ------------------------=
// FUNC: state_checksum
// DESC: Computes the bounded persisted compute-state checksum used to reject torn or corrupted audit objects.
// ------------------=
fn state_checksum(input: &[u8]) -> u32 { input.iter().fold(0x811c9dc5u32, |value, byte| value.wrapping_mul(16777619) ^ *byte as u32) }
