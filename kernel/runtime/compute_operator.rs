//! Bounded coordinator joining authoritative compute tasks to authenticated IOP.
//! Transport grants remain explicit and node-scoped; the coordinator never
//! treats a remote processor as local execution capacity.

use super::{
    capability::{CapabilityManager, CapabilityType},
    compute::{ComputeDispatchV1, ComputeError, ComputeService, ComputeState, MAX_SLICE_TICKS},
    execution::{ExecutionManager, SecurityIdentity},
    fabric::resources::Directory,
    iop::{remote::{RemoteError, RemoteResult}, IopRouter, OperationId},
    node::{types::NodeId, NodeRuntime},
};

pub const MAX_COORDINATED_COMPUTE: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemoteComputeAuthority {
    pub node: NodeId,
    pub request_grant: u64,
    pub cancel_grant: u64,
}

#[derive(Clone, Copy)]
struct Flight {
    request: u64,
    service_capability: u64,
    epoch: u32,
    operation: OperationId,
}

#[derive(Clone, Copy)]
struct CoordinatedTask {
    task: u64,
    caller: SecurityIdentity,
    authorities: [RemoteComputeAuthority; 2],
    authority_count: u8,
    flight: Option<Flight>,
    cancel_sent: bool,
}

pub struct ComputeCoordinator {
    tasks: [Option<CoordinatedTask>; MAX_COORDINATED_COMPUTE],
}

impl ComputeCoordinator {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty fixed-capacity outbound compute coordinator.
    // ------------------=
    pub const fn new() -> Self { Self { tasks: [None; MAX_COORDINATED_COMPUTE] } }

    // ------------------------=
    // FUNC: available
    // DESC: Reports whether another authoritative task can be tracked without an unbounded queue.
    // ------------------=
    pub fn available(&self) -> bool { self.tasks.iter().any(Option::is_none) }

    // ------------------------=
    // FUNC: track
    // DESC: Binds one task to explicit per-node request and cancellation grants before any remote dispatch.
    // ------------------=
    pub fn track(&mut self, task: u64, caller: SecurityIdentity, authorities: [RemoteComputeAuthority; 2], authority_count: u8) -> Result<(), ComputeError> {
        if task == 0 || authority_count > 2 || self.tasks.iter().flatten().any(|entry| entry.task == task) { return Err(ComputeError::InvalidRequest); }
        for (index, authority) in authorities.iter().enumerate() {
            if index < authority_count as usize {
                if authority.node.0 == [0; 32] || authority.request_grant == 0 || authority.cancel_grant == 0 || authorities[..index].iter().any(|prior| prior.node == authority.node) { return Err(ComputeError::InvalidRequest); }
            } else if authority.node.0 != [0; 32] || authority.request_grant != 0 || authority.cancel_grant != 0 { return Err(ComputeError::InvalidRequest); }
        }
        let slot = self.tasks.iter().position(Option::is_none).ok_or(ComputeError::Full)?;
        self.tasks[slot] = Some(CoordinatedTask { task, caller, authorities, authority_count, flight: None, cancel_sent: false });
        Ok(())
    }

    // ------------------------=
    // FUNC: task_count
    // DESC: Exposes bounded coordinator occupancy for diagnostics and leak assertions.
    // ------------------=
    pub fn task_count(&self) -> usize { self.tasks.iter().flatten().count() }

    // ------------------------=
    // FUNC: poll
    // DESC: Advances at most one bounded local slice or authenticated remote transition per tracked task.
    // ------------------=
    pub fn poll(&mut self, compute: &mut ComputeService, iop: &mut IopRouter, capabilities: &mut CapabilityManager, nodes: &NodeRuntime, execution: &mut ExecutionManager, directory: &mut Directory, local: NodeId, service: SecurityIdentity, now: u64) {
        for index in 0..MAX_COORDINATED_COMPUTE {
            let Some(mut coordinated) = self.tasks[index] else { continue; };
            let Ok(before) = compute.inspect(coordinated.task) else { self.tasks[index] = None; continue; };
            if let Some(flight) = coordinated.flight {
                if flight.epoch != before.epoch || (flight.operation == OperationId::ComputeRequest && matches!(before.state, ComputeState::Completed | ComputeState::Failed | ComputeState::Cancelled)) {
                    iop.remote.discard(service, flight.request);
                    let _ = capabilities.retire_leaf(flight.service_capability, service);
                    coordinated.flight = None;
                } else if let Some(result) = iop.remote.take_compute_result(service, flight.request) {
                    let _ = capabilities.retire_leaf(flight.service_capability, service);
                    coordinated.flight = None;
                    self.apply_completion(compute, directory, local, now, coordinated.task, flight, result);
                } else {
                    self.tasks[index] = Some(coordinated);
                    continue;
                }
            }
            let Ok(snapshot) = compute.inspect(coordinated.task) else { self.tasks[index] = None; continue; };
            if matches!(snapshot.state, ComputeState::Completed | ComputeState::Failed) { self.tasks[index] = None; continue; }
            if snapshot.state == ComputeState::Cancelled {
                if snapshot.node == local || coordinated.cancel_sent { self.tasks[index] = None; continue; }
                if self.submit_remote(compute, iop, capabilities, nodes, directory, local, service, now, &mut coordinated, true).is_err() { self.tasks[index] = None; } else { coordinated.cancel_sent = true; self.tasks[index] = Some(coordinated); }
                continue;
            }
            if snapshot.node == local {
                let result = if snapshot.state == ComputeState::Placed {
                    compute.start(snapshot.task_id, snapshot.epoch, coordinated.caller, capabilities, local, execution, directory, now).map(|_| snapshot)
                } else {
                    compute.run_slice(snapshot.task_id, snapshot.epoch, coordinated.caller, capabilities, local, execution, MAX_SLICE_TICKS, directory, now)
                };
                if result.is_err() && compute.inspect(snapshot.task_id).is_err() { self.tasks[index] = None; } else { self.tasks[index] = Some(coordinated); }
                continue;
            }
            match self.submit_remote(compute, iop, capabilities, nodes, directory, local, service, now, &mut coordinated, false) {
                Ok(()) => self.tasks[index] = Some(coordinated),
                Err(error) => {
                    let _ = compute.remote_transport_failed(snapshot.task_id, snapshot.epoch, error, directory, local, now);
                    self.tasks[index] = if compute.inspect(snapshot.task_id).is_ok_and(|task| matches!(task.state, ComputeState::Placed | ComputeState::Running)) { Some(coordinated) } else { None };
                }
            }
        }
    }

    // ------------------------=
    // FUNC: apply_completion
    // DESC: Applies only caller-owned correlated results and maps transport loss into the authoritative recovery path.
    // ------------------=
    fn apply_completion(&mut self, compute: &mut ComputeService, directory: &mut Directory, local: NodeId, now: u64, task: u64, flight: Flight, result: RemoteResult<ComputeDispatchV1>) {
        if flight.operation == OperationId::ComputeCancel { return; }
        match result.result {
            Ok(payload) => { let _ = compute.accept_remote_result(task, payload, directory, now); }
            Err(error) => {
                // The IOP request lease is intentionally shorter than the
                // workload deadline.  Expiring that per-hop lease while the
                // workload is still live means the selected node stopped
                // responding; restartable work must enter node-loss recovery
                // rather than being terminally misreported as deadline expiry.
                let snapshot = compute.inspect(task).ok();
                if error == RemoteError::DeadlineExceeded && snapshot.is_some_and(|snapshot|
                    (snapshot.deadline == 0 || now < snapshot.deadline) && directory.entries().iter().flatten().any(|resource|
                        resource.id == snapshot.resource && resource.owner == snapshot.node && resource.online && now < resource.expires)) {
                    return;
                }
                let error = if error == RemoteError::DeadlineExceeded
                    && snapshot.is_some_and(|snapshot| snapshot.deadline == 0 || now < snapshot.deadline)
                { ComputeError::NodeLost } else { map_remote_error(error) };
                let _ = compute.remote_transport_failed(task, flight.epoch, error, directory, local, now);
            }
        }
    }

    // ------------------------=
    // FUNC: submit_remote
    // DESC: Mints a short-lived internal ServiceCall capability and submits one request through the authenticated shared IOP router.
    // ------------------=
    fn submit_remote(&self, compute: &mut ComputeService, iop: &mut IopRouter, capabilities: &mut CapabilityManager, nodes: &NodeRuntime, directory: &mut Directory, local: NodeId, service: SecurityIdentity, now: u64, coordinated: &mut CoordinatedTask, cancel: bool) -> Result<(), ComputeError> {
        let snapshot = compute.inspect(coordinated.task)?;
        let authority = coordinated.authorities[..coordinated.authority_count as usize].iter().find(|authority| authority.node == snapshot.node).copied().ok_or(ComputeError::AccessDenied)?;
        let payload = if cancel { compute.remote_cancel_dispatch(snapshot.task_id, snapshot.epoch)? } else { compute.remote_dispatch(snapshot.task_id, coordinated.caller, capabilities, local, directory, now)? };
        let operation = if cancel { OperationId::ComputeCancel } else { OperationId::ComputeRequest };
        let transport_deadline = if payload.deadline == 0 { now.saturating_add(30) } else { payload.deadline.min(now.saturating_add(30)) };
        if transport_deadline <= now { return Err(ComputeError::DeadlineExceeded); }
        capabilities.reclaim_expired_leaves(now);
        let service_capability = capabilities.grant(CapabilityType::ServiceCall, operation as u64, 1, 0, service, service, Some(transport_deadline), 0).map_err(|_| ComputeError::Full)?;
        let grant = if cancel { authority.cancel_grant } else { authority.request_grant };
        match iop.next_node_request().map_err(|_| ComputeError::Full).and_then(|correlation| iop.request_remote_compute(capabilities, nodes, service, service_capability, snapshot.node, grant, payload, snapshot.correlation_id, correlation, now, transport_deadline).map_err(map_submit_error)) {
            Ok(request) => { coordinated.flight = Some(Flight { request, service_capability, epoch: snapshot.epoch, operation }); Ok(()) }
            Err(error) => { let _ = capabilities.retire_leaf(service_capability, service); Err(error) }
        }
    }
}

// ------------------------=
// FUNC: map_submit_error
// DESC: Maps synchronous authenticated-router admission failures into typed compute lifecycle errors.
// ------------------=
fn map_submit_error(error: RemoteError) -> ComputeError { map_remote_error(error) }

// ------------------------=
// FUNC: map_remote_error
// DESC: Separates node/session loss from authority, deadline, capacity, and malformed-result failures.
// ------------------=
fn map_remote_error(error: RemoteError) -> ComputeError {
    match error {
        RemoteError::AccessDenied | RemoteError::CapabilityRequired | RemoteError::CapabilityExpired | RemoteError::CapabilityRevoked | RemoteError::CapabilityScopeDenied | RemoteError::PolicyDenied => ComputeError::AccessDenied,
        RemoteError::DeadlineExceeded => ComputeError::DeadlineExceeded,
        RemoteError::QueueFull | RemoteError::ServiceUnavailable => ComputeError::ResourceUnavailable,
        RemoteError::SessionNotFound | RemoteError::TransportClosed | RemoteError::TrustRequired | RemoteError::TrustRevoked | RemoteError::NodeBlocked => ComputeError::NodeLost,
        _ => ComputeError::InvalidRequest,
    }
}
