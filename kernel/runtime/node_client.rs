//! Kernel-owned GUI/Console broker. Full active-session authority is checked before issuing a single-operation capability.
use super::*;
use identity::{StableId, SessionState, MAX_SESSIONS, SESSION_IDENTITY_MANAGE};
use iop::{IopError, IopMessage, NodeOperationV1, OperationId};

#[used]
#[no_mangle]
static mut INFINITY_NODE_LOCAL_DIAGNOSTIC_SNAPSHOT: [u64; 32] = [0; 32];

// ------------------------=
// FUNC: completion_words
// DESC: Encodes the exact local owned request outcome without altering remote broker observations.
// ------------------=
fn completion_words(id: u64, result: Result<NodeOperationV1, IopError>) -> [u64; 32] {
    let mut words=[0u64;32];words[0]=0x494e464c4f434c31;words[1]=1;words[3]=id;
    match result {
        Ok(response)=>{words[4]=1;for (index,chunk) in response.encode().chunks_exact(8).enumerate(){words[5+index]=u64::from_le_bytes(chunk.try_into().unwrap());}}
        Err(error)=>words[4]=error as u64+2,
    }
    words
}

// ------------------------=
// FUNC: publish_local_completion
// DESC: Publishes only a locally admitted IOP result after completion, using a bounded debugger-only generation guard.
// ------------------=
fn publish_local_completion(id:u64,result:Result<NodeOperationV1,IopError>){
    let mut words=completion_words(id,result);
    unsafe {
        let pointer=(&raw mut INFINITY_NODE_LOCAL_DIAGNOSTIC_SNAPSHOT).cast::<u64>();
        let generation=core::ptr::read_volatile(pointer.add(2)).wrapping_add(2)&!1;
        core::ptr::write_volatile(pointer.add(2),generation|1);words[2]=generation;words[31]=generation;
        for index in 0..32 {if index!=2 {core::ptr::write_volatile(pointer.add(index),words[index]);}}
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
        core::ptr::write_volatile(pointer.add(2),generation);
    }
}

#[cfg(test)]
mod local_completion_tests {
    use super::*;
    // ------------------------=
    // FUNC: local_response_and_failure_are_exact
    // DESC: Preserves typed durable handles and clears response bytes on the next failed local request.
    // ------------------=
    #[test]
    fn local_response_and_failure_are_exact(){
        let mut response=crate::runtime::node::reconciliation::request([17;32],OperationId::NodeCapabilityGrant);
        response.handle=(1u64<<63)|7;response.flags=3;
        let success=completion_words(91,Ok(response));
        assert_eq!(success[3],91);assert_eq!(success[4],1);
        assert_eq!(success[9],response.handle);
        let failed=completion_words(92,Err(IopError::AccessDenied));
        assert_eq!(failed[3],92);assert_ne!(failed[4],1);
        assert!(failed[5..15].iter().all(|word|*word==0));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodePresentation {
    pub selected: Option<node::types::NodeId>,
    pub verification: Option<node::wire_trust::Verification>,
    pub remaining: u64,
    pub checkpoint: u64,
    pub projection: u64,
    pub stale: bool,
    pub sessions: usize,
    pub policy_offset: usize,
}

// ------------------------=
// FUNC: presentation
// DESC: Captures bounded visible node state for retained Settings invalidation without treating the view as authority.
// ------------------=
pub fn presentation(runtime: &InfinityRuntime) -> NodePresentation {
    let verification = runtime.node_selection.and_then(|peer| runtime.nodes.local_id().and_then(|local| runtime.node_transport.trust.verification(local, peer)));
    NodePresentation {
        selected: runtime.node_selection,
        verification,
        remaining: verification.map(|value| value.expires.saturating_sub(runtime.node_clock.unwrap_or(value.expires))).unwrap_or(0),
        checkpoint: runtime.nodes.control_version(),
        projection: runtime.node_projection.version,
        stale: runtime.node_projection.stale,
        sessions: runtime.nodes.sessions().iter().flatten().filter(|session| session.state == node::types::SessionState::Established).count(),
        policy_offset: runtime.node_policy_offset,
    }
}

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
        if operation == OperationId::NodePairConfirm {
            let context = runtime.services.inspect(SERVICE_CONSOLE).and_then(|service| service.context).ok_or(IopError::AccessDenied)?;
            if !runtime.ui.trusted.input_is_for(context.0 as u32, crate::ui::trusted::TrustedSurface::NodePairing, now) { return Err(IopError::AccessDenied); }
        }
        if operation == OperationId::NodeCapabilityGrant {
            let context = runtime.services.inspect(SERVICE_CONSOLE).and_then(|service| service.context).ok_or(IopError::AccessDenied)?;
            if !runtime.ui.trusted.input_is_for(context.0 as u32, crate::ui::trusted::TrustedSurface::CapabilityConsent, now) { return Err(IopError::AccessDenied); }
        }
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
                if matches!(operation, OperationId::NodePairBegin | OperationId::NodePairConfirm | OperationId::NodePairCancel | OperationId::NodeSessionOpen) {
                    nodes.commit_wire_control(wire, link, operation, request, now, correlation, causation, persist_control_state)
                } else if matches!(operation, OperationId::NodeJoin | OperationId::NodeLeave) {
                    nodes.commit_domain_intent(operation, request, now, correlation, causation, persist_control_state)
                } else if matches!(operation, OperationId::NodeLinkConfigure | OperationId::NodeLinkRemove) {
                    nodes.commit_link_configuration(operation, request, now, correlation, causation, persist_control_state)
                } else { nodes.commit_control(operation, request, now, correlation, causation, persist_control_state) }
            })?;
            let reply = runtime.iop.receive(0xd002, now)?;
            if reply.header.request_id != id || reply.header.causation_id != id { return Err(IopError::InvalidHeader); }
            Ok((response, notice, now))
        })();
        let _ = runtime.capabilities.retire_leaf(capability, service);
        publish_local_completion(id,result.as_ref().map(|value|value.0).map_err(|error|*error));
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
    let operation = [OperationId::NodeLinkList, OperationId::NodeList, OperationId::NodeInspect, OperationId::NodeDiscoverStatus, OperationId::NodeDomainList, OperationId::NodeDomainInspect, OperationId::NodeSessionList, OperationId::NodeSessionInspect, OperationId::NodePolicyRead, OperationId::NodeTrustRead, OperationId::NodeHealth, OperationId::NodeDiagnostics, OperationId::NodeCapabilityList, OperationId::NodeAuditList, OperationId::NodeAuditInspect, OperationId::MeshStatus, OperationId::MeshMemberList, OperationId::MeshPolicyRead].into_iter().find(|op| op.machine_id() == request.operation).ok_or(IopError::AccessDenied)?;
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

// ------------------------=
// FUNC: clock
// DESC: Reads the native service-pump time, falling back only to an observed hardware monotonic clock.
// ------------------=
pub fn clock() -> Option<u64> { with_runtime(|runtime| runtime.node_clock).flatten().or_else(|| crate::ui::performance::monotonic_ns().map(|ns| ns / 1_000_000_000)) }

// ------------------------=
// FUNC: begin_pairing_input
// DESC: Reserves the native trusted pairing input surface for the complete active operator identity before accepting a verification decision.
// ------------------=
pub fn begin_pairing_input(user: StableId, session: StableId) -> Result<crate::ui::trusted::SecureInputLease, IopError> {
    begin_operator_input(user, session, crate::ui::trusted::TrustedSurface::NodePairing)
}

// ------------------------=
// FUNC: begin_capability_input
// DESC: Reserves exclusive native capability-consent input for a scoped human grant decision.
// ------------------=
pub fn begin_capability_input(user: StableId, session: StableId) -> Result<crate::ui::trusted::SecureInputLease, IopError> {
    begin_operator_input(user, session, crate::ui::trusted::TrustedSurface::CapabilityConsent)
}

// ------------------------=
// FUNC: begin_operator_input
// DESC: Authorizes the complete active operator before acquiring one kernel-selected protected input surface.
// ------------------=
fn begin_operator_input(user: StableId, session: StableId, surface: crate::ui::trusted::TrustedSurface) -> Result<crate::ui::trusted::SecureInputLease, IopError> {
    let now = clock().ok_or(IopError::DeadlineExceeded)?;
    with_runtime(|runtime| {
        let active = (0..MAX_SESSIONS).filter_map(|index| runtime.identity.session_nth(index)).any(|candidate| candidate.id == session && candidate.user == user && candidate.state == SessionState::Active && candidate.capabilities & SESSION_IDENTITY_MANAGE != 0);
        if !active { return Err(IopError::AccessDenied); }
        let context = runtime.services.inspect(SERVICE_CONSOLE).and_then(|service| service.context).ok_or(IopError::AccessDenied)?;
        runtime.ui.trusted.expire(now);
        runtime.ui.trusted.acquire_secure_input(true, context.0 as u32, surface, now.saturating_add(60)).map_err(|_| IopError::AccessDenied)
    }).ok_or(IopError::UnknownEndpoint)?
}
