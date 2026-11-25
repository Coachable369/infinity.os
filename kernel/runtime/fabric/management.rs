//! Shared authenticated Fabric management. Capacity and provider policy use
//! the existing authorities; clients cannot maintain competing state.
use super::resources::{FabricCapacity, ResourceError, RESOURCE_KINDS};
use crate::runtime::{InfinityRuntime, identity::{StableId, SessionState, MAX_SESSIONS, SESSION_IDENTITY_MANAGE}};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapacitySnapshot {
    pub observed_at: u64,
    pub categories: [FabricCapacity; 11],
}

// ------------------------=
// FUNC: inspect_capacity
// DESC: Authenticates the active operator before projecting authoritative local and fabric capacity for GUI, CLI and AI clients.
// ------------------=
pub fn inspect_capacity(user: StableId, session: StableId) -> Result<CapacitySnapshot, ResourceError> {
    crate::runtime::with_runtime(|runtime| {
        let now = runtime.node_clock.ok_or(ResourceError::Stale)?;
        inspect_capacity_from(runtime, user, session, now)
    }).ok_or(ResourceError::NotFound)?
}

// ------------------------=
// FUNC: inspect_capacity_from
// DESC: Shares session authorization and capacity projection without granting resource access or changing allocations.
// ------------------=
pub(crate) fn inspect_capacity_from(runtime: &InfinityRuntime, user: StableId, session: StableId, now: u64) -> Result<CapacitySnapshot, ResourceError> {
    let authorized = runtime.identity.console_session_authorized(session, user)
        && (0..MAX_SESSIONS).filter_map(|i| runtime.identity.session_nth(i)).any(|candidate| {
            candidate.id == session && candidate.user == user && candidate.state == SessionState::Active
                && candidate.capabilities & SESSION_IDENTITY_MANAGE != 0
        });
    if !authorized { return Err(ResourceError::AccessDenied); }
    let local = runtime.nodes.local_id().ok_or(ResourceError::NotFound)?;
    let mut snapshot = CapacitySnapshot { observed_at: now, categories: [FabricCapacity::default(); 11] };
    for (index, kind) in RESOURCE_KINDS.iter().enumerate() {
        snapshot.categories[index] = runtime.fabric_resources.capacity(*kind, local, now)?;
    }
    Ok(snapshot)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProviderSnapshot {
    pub limits: crate::runtime::compute::ProviderComputeLimits,
    pub revision: u64,
    pub allocated_memory_bytes: u64,
    pub allocated_cpu_units: u64,
    pub active_tasks: usize,
}

// ------------------------=
// FUNC: provider_authorized
// DESC: Requires an active administrator-owned session before exposing or changing machine contribution policy.
// ------------------=
fn provider_authorized(runtime: &InfinityRuntime, user: StableId, session: StableId) -> bool {
    runtime.identity.is_administrator(user)
        && runtime.identity.console_session_authorized(session, user)
        && (0..MAX_SESSIONS).filter_map(|i| runtime.identity.session_nth(i)).any(|candidate| {
            candidate.id == session && candidate.user == user && candidate.state == SessionState::Active
                && candidate.capabilities & SESSION_IDENTITY_MANAGE != 0
        })
}

// ------------------------=
// FUNC: provider_snapshot
// DESC: Reads limits and allocation from the single authoritative provider rather than a client-side copy.
// ------------------=
fn provider_snapshot(runtime: &InfinityRuntime) -> ProviderSnapshot {
    let (limits, revision) = runtime.remote_compute.limits();
    let (allocated_memory_bytes, allocated_cpu_units, active_tasks) = runtime.remote_compute.allocation();
    ProviderSnapshot { limits, revision, allocated_memory_bytes, allocated_cpu_units, active_tasks }
}

// ------------------------=
// FUNC: inspect_provider
// DESC: Provides one authenticated policy and usage API for Settings, console and AI clients.
// ------------------=
pub fn inspect_provider(user: StableId, session: StableId) -> Result<ProviderSnapshot, crate::runtime::compute::ComputeError> {
    use crate::runtime::compute::ComputeError;
    crate::runtime::with_runtime(|runtime| {
        if !provider_authorized(runtime, user, session) { return Err(ComputeError::AccessDenied); }
        Ok(provider_snapshot(runtime))
    }).ok_or(ComputeError::InvalidState)?
}

// ------------------------=
// FUNC: update_provider
// DESC: Persists an administrator's revision-checked contribution change through the shared provider authority.
// ------------------=
pub fn update_provider(user: StableId, session: StableId,
    limits: crate::runtime::compute::ProviderComputeLimits, expected_revision: u64,
) -> Result<ProviderSnapshot, crate::runtime::compute::ComputeError> {
    use crate::runtime::compute::ComputeError;
    crate::runtime::with_runtime(|runtime| {
        let now = runtime.node_clock.ok_or(ComputeError::InvalidState)?;
        update_provider_from(runtime, user, session, limits, expected_revision, now,
            &mut crate::runtime::persist_provider_state)
    }).ok_or(ComputeError::InvalidState)?
}

// ------------------------=
// FUNC: update_provider_from
// DESC: Issues and retires a nondelegable one-operation capability only after authenticating the administrator session.
// ------------------=
pub(crate) fn update_provider_from(runtime: &mut InfinityRuntime, user: StableId, session: StableId,
    limits: crate::runtime::compute::ProviderComputeLimits, expected_revision: u64, now: u64,
    commit: &mut impl FnMut(&[u8]) -> bool,
) -> Result<ProviderSnapshot, crate::runtime::compute::ComputeError> {
    use crate::runtime::{compute::{ComputeError, PROVIDER_COMPUTE_POLICY_TARGET}, capability::CapabilityType};
    if !provider_authorized(runtime, user, session) { return Err(ComputeError::AccessDenied); }
    let caller = runtime.service_identity(crate::runtime::SERVICE_CONSOLE).ok_or(ComputeError::AccessDenied)?;
    let expiry = now.checked_add(1).ok_or(ComputeError::InvalidState)?;
    let capability = runtime.capabilities.grant(CapabilityType::ResourcePolicyModify,
        PROVIDER_COMPUTE_POLICY_TARGET, 1, 0, caller, caller, Some(expiry), 0)
        .map_err(|_| ComputeError::AccessDenied)?;
    let result = runtime.remote_compute.set_limits_durable(limits, expected_revision, caller,
        capability, &runtime.capabilities, &mut runtime.execution, now, commit);
    let retired = runtime.capabilities.retire_leaf(capability, caller);
    if retired.is_err() { return Err(ComputeError::InvalidState); }
    result?;
    Ok(provider_snapshot(runtime))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProviderUpdate {
    pub limits: crate::runtime::compute::ProviderComputeLimits,
    pub expected_revision: u64,
}

// ------------------------=
// FUNC: decode_provider_update
// DESC: Converts the registered console operation into a strict typed request; never infers consent or truncates numeric values.
// ------------------=
pub fn decode_provider_update(node: &crate::runtime::console_language::OperationNode<'_>)
    -> Result<ProviderUpdate, crate::runtime::compute::ComputeError> {
    use crate::runtime::compute::{ComputeError, ProviderComputeLimits};
    if node.schema.domain != b"pool" || node.schema.operation != crate::runtime::iop::OperationId::ProviderConfigure {
        return Err(ComputeError::InvalidRequest);
    }
    let argument = |name: &[u8]| node.arguments.iter().flatten()
        .find(|argument| argument.name == name).map(|argument| argument.value);
    if argument(b"confirm") != Some(b"true".as_slice()) { return Err(ComputeError::AccessDenied); }
    let enabled = match argument(b"enabled") {
        Some(b"true") => true, Some(b"false") => false, _ => return Err(ComputeError::InvalidRequest),
    };
    let number = |name: &[u8]| -> Result<u64, ComputeError> {
        let bytes = argument(name).ok_or(ComputeError::InvalidRequest)?;
        if bytes.is_empty() { return Err(ComputeError::InvalidRequest); }
        bytes.iter().try_fold(0u64, |value, byte| {
            if !byte.is_ascii_digit() { return Err(ComputeError::InvalidRequest); }
            value.checked_mul(10).and_then(|value| value.checked_add((byte - b'0') as u64))
                .ok_or(ComputeError::InvalidRequest)
        })
    };
    Ok(ProviderUpdate {
        limits: ProviderComputeLimits { enabled, memory_bytes: number(b"memory")?,
            cpu_units: u16::try_from(number(b"cpu")?).map_err(|_| ComputeError::InvalidRequest)?,
            concurrent_tasks: u16::try_from(number(b"tasks")?).map_err(|_| ComputeError::InvalidRequest)? },
        expected_revision: number(b"revision")?,
    })
}

// ------------------------=
// FUNC: advertised_provider_resource
// DESC: Limits advertised availability to the durable provider allowance while preserving measured physical capacity and resource identity.
// ------------------=
pub(crate) fn advertised_provider_resource(
    mut resource: super::resources::Resource,
    limits: crate::runtime::compute::ProviderComputeLimits,
    allocation: (u64, u64, usize),
) -> super::resources::Resource {
    use super::resources::ResourceKind;
    let allowance = match resource.kind {
        ResourceKind::Compute => Some((limits.concurrent_tasks as u64)
            .saturating_sub(allocation.2 as u64)
            .min((limits.cpu_units as u64).saturating_sub(allocation.1))),
        ResourceKind::Memory => Some(limits.memory_bytes.saturating_sub(allocation.0)),
        _ => None,
    };
    if let Some(allowance) = allowance {
        resource.available = if limits.enabled { resource.available.min(allowance) } else { 0 };
        if !limits.enabled { resource.capabilities &= !1; }
    }
    resource
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeSnapshot {
    pub id: crate::runtime::node::types::NodeId,
    pub local: bool,
    pub reachability: crate::runtime::node::types::Reachability,
    pub trust: crate::runtime::node::types::TrustState,
    pub last_seen: u64,
    pub categories: [super::resources::CapacityTotals; 11],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClusterSnapshot {
    pub capacity: CapacitySnapshot,
    pub nodes: [Option<NodeSnapshot>; crate::runtime::node::types::MAX_DISCOVERED_NODES + 1],
}

// ------------------------=
// FUNC: inspect_cluster
// DESC: Reads cluster and per-node projections atomically through the same authenticated management authority.
// ------------------=
pub fn inspect_cluster(user: StableId, session: StableId) -> Result<ClusterSnapshot, ResourceError> {
    crate::runtime::with_runtime(|runtime| {
        let now = runtime.node_clock.ok_or(ResourceError::Stale)?;
        inspect_cluster_from(runtime, user, session, now)
    }).ok_or(ResourceError::NotFound)?
}

// ------------------------=
// FUNC: inspect_cluster_from
// DESC: Joins discovered node identities with resource observations without granting remote access or mutating state.
// ------------------=
pub(crate) fn inspect_cluster_from(runtime: &InfinityRuntime, user: StableId, session: StableId, now: u64)
    -> Result<ClusterSnapshot, ResourceError> {
    use crate::runtime::node::types::{Reachability, TrustState};
    let capacity = inspect_capacity_from(runtime, user, session, now)?;
    let local = runtime.nodes.local_id().ok_or(ResourceError::NotFound)?;
    let mut result = ClusterSnapshot { capacity, nodes: [None; crate::runtime::node::types::MAX_DISCOVERED_NODES + 1] };
    result.nodes[0] = Some(NodeSnapshot { id: local, local: true,
        reachability: Reachability::Online, trust: TrustState::Trusted, last_seen: now,
        categories: [super::resources::CapacityTotals::default(); 11] });
    let mut next = 1;
    for node in runtime.nodes.discovered_nodes().iter().flatten() {
        if node.id == local { continue; }
        result.nodes[next] = Some(NodeSnapshot { id: node.id, local: false,
            reachability: node.reachability, trust: node.trust, last_seen: node.last_seen,
            categories: [super::resources::CapacityTotals::default(); 11] });
        next += 1;
    }
    for node in result.nodes.iter_mut().flatten() {
        for (index, kind) in RESOURCE_KINDS.iter().enumerate() {
            node.categories[index] = runtime.fabric_resources.node_capacity(*kind, node.id, now)?;
        }
    }
    Ok(result)
}

// ------------------------=
// FUNC: inspect_jobs
// DESC: Exposes the bounded authoritative job inventory only to an active administrator; no client-side job records are invented.
// ------------------=
pub fn inspect_jobs(user: StableId, session: StableId)
    -> Result<[Option<crate::runtime::compute::ComputeSnapshot>; crate::runtime::compute::MAX_COMPUTE_TASKS * 2], ResourceError> {
    crate::runtime::with_runtime(|runtime| inspect_jobs_from(runtime, user, session))
        .ok_or(ResourceError::NotFound)?
}

// ------------------------=
// FUNC: inspect_jobs_from
// DESC: Checks operator authority before projecting active and retained job states from the compute service.
// ------------------=
pub(crate) fn inspect_jobs_from(runtime: &InfinityRuntime, user: StableId, session: StableId)
    -> Result<[Option<crate::runtime::compute::ComputeSnapshot>; crate::runtime::compute::MAX_COMPUTE_TASKS * 2], ResourceError> {
    if !provider_authorized(runtime, user, session) { return Err(ResourceError::AccessDenied); }
    Ok(core::array::from_fn(|index| runtime.compute.task_nth(index)))
}

// ------------------------=
// FUNC: cancel_job
// DESC: Cancels the selected execution epoch through the shared authority used by management clients.
// ------------------=
pub fn cancel_job(user: StableId, session: StableId, task: u64, epoch: u32)
    -> Result<(), crate::runtime::compute::ComputeError> {
    use crate::runtime::compute::ComputeError;
    crate::runtime::with_runtime(|runtime| {
        let now = runtime.node_clock.ok_or(ComputeError::InvalidState)?;
        cancel_job_from(runtime, user, session, task, epoch, now)
    }).ok_or(ComputeError::InvalidState)?
}

// ------------------------=
// FUNC: cancel_job_from
// DESC: Rejects stale selections and unauthorized sessions, then retires the single-use cancellation capability on every outcome.
// ------------------=
pub(crate) fn cancel_job_from(runtime: &mut InfinityRuntime, user: StableId, session: StableId,
    task: u64, epoch: u32, now: u64) -> Result<(), crate::runtime::compute::ComputeError> {
    use crate::runtime::{compute::ComputeError, capability::CapabilityType};
    if !provider_authorized(runtime, user, session) { return Err(ComputeError::AccessDenied); }
    if runtime.compute.inspect(task)?.epoch != epoch { return Err(ComputeError::StaleResult); }
    let caller = runtime.service_identity(crate::runtime::SERVICE_CONSOLE).ok_or(ComputeError::AccessDenied)?;
    let expiry = now.checked_add(1).ok_or(ComputeError::InvalidState)?;
    let capability = runtime.capabilities.grant(CapabilityType::ComputeCancel, task, 1, 0,
        caller, caller, Some(expiry), 0).map_err(|_| ComputeError::AccessDenied)?;
    let result = runtime.compute.cancel(task, caller, capability, &runtime.capabilities,
        &mut runtime.execution, &mut runtime.fabric_resources, now);
    runtime.capabilities.retire_leaf(capability, caller).map_err(|_| ComputeError::InvalidState)?;
    result
}
