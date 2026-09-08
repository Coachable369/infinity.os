//! Kernel-owned GUI/Console broker. Full active-session authority is checked before issuing a single-operation capability.
use super::*;
use identity::{StableId, SessionState, MAX_SESSIONS, SESSION_IDENTITY_MANAGE};
use iop::{IopError, IopMessage, NodeOperationV1, OperationId};

// ------------------------=
// FUNC: submit
// DESC: Routes an explicit authenticated human action through ordinary IOP and publishes only a durable committed notice.
// ------------------=
pub fn submit(user: StableId, session: StableId, operation: OperationId, request: NodeOperationV1) -> Result<NodeOperationV1, IopError> {
    let result = with_runtime(|runtime| {
        let now = runtime.node_clock.or_else(|| crate::ui::performance::monotonic_ns().map(|ns| ns / 1_000_000_000)).ok_or(IopError::DeadlineExceeded)?;
        let active = (0..MAX_SESSIONS).filter_map(|index| runtime.identity.session_nth(index)).any(|candidate| {
            candidate.id == session && candidate.user == user && candidate.state == SessionState::Active && candidate.capabilities & SESSION_IDENTITY_MANAGE != 0
        });
        if !active { return Err(IopError::AccessDenied); }
        let caller = runtime.service_identity(SERVICE_CONSOLE).ok_or(IopError::AccessDenied)?;
        let service = runtime.service_identity(SERVICE_NODE_TRUST).ok_or(IopError::AccessDenied)?;
        runtime.iop.ensure_owned_endpoint(0xd001, service)?;
        runtime.iop.ensure_owned_endpoint(0xd002, caller)?;
        let id = runtime.iop.next_node_request()?;
        let deadline = now.checked_add(30).ok_or(IopError::DeadlineExceeded)?;
        let capability = runtime.capabilities.grant(CapabilityType::ServiceCall, operation.machine_id() as u64, 1, 0, service, caller, Some(deadline), 0)?;
        let result = (|| {
            let message = IopMessage::request(operation, id, caller, capability, deadline, id, &request.encode())?;
            runtime.iop.send(0xd001, message, &runtime.capabilities, now)?;
            // Execution is synchronous while the runtime is exclusively borrowed;
            // session state cannot change between the active-session check and dequeue.
            let link = runtime.node_transport.peer_link(node::types::NodeId(request.node_id));
            let wire = &mut runtime.node_transport.trust;
            let (response, notice) = runtime.iop.dispatch_node_transaction(&runtime.capabilities, &mut runtime.nodes, operation, 0xd001, 0xd002, service, now, |nodes, request, correlation, causation| {
                if matches!(operation, OperationId::NodePairBegin | OperationId::NodePairConfirm | OperationId::NodePairCancel) {
                    nodes.commit_wire_control(wire, link, operation, request, now, correlation, causation, persist_control_state)
                } else if matches!(operation, OperationId::NodeJoin | OperationId::NodeLeave) {
                    nodes.commit_domain_intent(operation, request, now, correlation, causation, persist_control_state)
                } else { nodes.commit_control(operation, request, now, correlation, causation, persist_control_state) }
            })?;
            let reply = runtime.iop.receive(0xd002, now)?;
            if reply.header.request_id != id || reply.header.causation_id != id { return Err(IopError::InvalidHeader); }
            Ok((response, notice, now))
        })();
        let _ = runtime.capabilities.retire_leaf(capability, service);
        result
    }).ok_or(IopError::UnknownEndpoint)??;
    let _ = publish_committed_node_control(result.1, result.2);
    Ok(result.0)
}

// ------------------------=
// FUNC: read
// DESC: Gives the kernel-owned inspector bounded read-only IOP access with a single exact-operation capability, retired after the reply.
// ------------------=
pub(super) fn read(runtime: &mut InfinityRuntime, request: NodeOperationV1, now: u64) -> Result<NodeOperationV1, IopError> {
    let operation = [OperationId::NodeList, OperationId::NodeInspect, OperationId::NodeDiscoverStatus, OperationId::NodeDomainList, OperationId::NodeDomainInspect, OperationId::NodeSessionList, OperationId::NodeSessionInspect, OperationId::NodePolicyRead, OperationId::NodeTrustRead, OperationId::NodeHealth, OperationId::NodeDiagnostics, OperationId::NodeCapabilityList, OperationId::NodeAuditList, OperationId::NodeAuditInspect, OperationId::MeshStatus, OperationId::MeshMemberList, OperationId::MeshPolicyRead].into_iter().find(|op| op.machine_id() == request.operation).ok_or(IopError::AccessDenied)?;
    let caller = runtime.service_identity(SERVICE_SETTINGS).ok_or(IopError::AccessDenied)?;
    let service = runtime.service_identity(SERVICE_NODE_TRUST).ok_or(IopError::AccessDenied)?;
    runtime.iop.ensure_owned_endpoint(0xd101, service)?;
    runtime.iop.ensure_owned_endpoint(0xd102, caller)?;
    let id = runtime.iop.next_node_request()?;
    let deadline = now.checked_add(30).ok_or(IopError::DeadlineExceeded)?;
    let cap = runtime.capabilities.grant(CapabilityType::ServiceCall, operation.machine_id() as u64, 1, 0, service, caller, Some(deadline), 0)?;
    let result = (|| {
        let message = IopMessage::request(operation, id, caller, cap, deadline, id, &request.encode())?;
        runtime.iop.send(0xd101, message, &runtime.capabilities, now)?;
        let response = iop::dispatch_node_operation(&mut runtime.iop, &runtime.capabilities, &mut runtime.nodes, operation, 0xd101, 0xd102, service, now)?;
        let reply = runtime.iop.receive(0xd102, now)?;
        if reply.header.request_id != id || reply.header.causation_id != id { return Err(IopError::InvalidHeader); }
        Ok(response)
    })();
    let _ = runtime.capabilities.retire_leaf(cap, service);
    result
}

// ------------------------=
// FUNC: query
// DESC: Authorizes the complete active user/session identity before exposing node metadata through the same read-only IOP broker used by Settings.
// ------------------=
pub fn query(user: StableId, session: StableId, request: NodeOperationV1) -> Result<NodeOperationV1, IopError> {
    with_runtime(|runtime| {
        let active = (0..MAX_SESSIONS).filter_map(|index| runtime.identity.session_nth(index)).any(|candidate| candidate.id == session && candidate.user == user && candidate.state == SessionState::Active && candidate.capabilities & SESSION_IDENTITY_MANAGE != 0);
        if !active { return Err(IopError::AccessDenied); }
        let now = runtime.node_clock.or_else(|| crate::ui::performance::monotonic_ns().map(|ns| ns / 1_000_000_000)).ok_or(IopError::DeadlineExceeded)?;
        read(runtime, request, now)
    }).ok_or(IopError::UnknownEndpoint)?
}
