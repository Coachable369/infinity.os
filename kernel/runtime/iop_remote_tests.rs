//! HOST router-boundary tests. Session setup is a local fixture, not wire or
//! installed-system evidence. No source text or diagnostic strings are oracles.
use super::*;
use crate::runtime::node::types::PolicyDecision;

struct Fixture {
    router: IopRouter,
    nodes: NodeRuntime,
    caps: CapabilityManager,
    caller: SecurityIdentity,
    peer: NodeId,
    cap: u64,
    session: u64,
    grant: u64,
}

// ------------------------=
// FUNC: expired_transient_capabilities_are_reclaimed_before_new_work
// DESC: Fills the bounded authority table with expired service leaves, reclaims them through the typed API, and proves new work can be authorized.
// ------------------=
#[test]
fn expired_transient_capabilities_are_reclaimed_before_new_work() {
    use crate::runtime::capability::{CapabilityType, MAX_CAPABILITIES};
    let mut capabilities = CapabilityManager::new();
    let service = SecurityIdentity([71; 16]);
    for target in 1..=MAX_CAPABILITIES as u64 {
        capabilities.grant(CapabilityType::ServiceCall, target, 1, 0, service, service, Some(5), 0).unwrap();
    }
    assert!(capabilities.grant(CapabilityType::ServiceCall, 999, 1, 0, service, service, Some(8), 0).is_err());
    assert_eq!(capabilities.reclaim_expired_leaves(6), MAX_CAPABILITIES);
    assert_eq!(capabilities.count(), 0);
    let replacement = capabilities.grant(CapabilityType::ServiceCall, 999, 1, 0, service, service, Some(8), 0).unwrap();
    assert!(capabilities.validate(replacement, service, CapabilityType::ServiceCall, 999, 1, 0, 6).is_ok());
}

// ------------------------=
// FUNC: reused_slot_preserves_authenticated_request_order
// DESC: Reuses a completed low slot through real request APIs and admits production-selected requests through the existing authenticated router replay gate.
// ------------------=
#[test]
fn reused_slot_preserves_authenticated_request_order(){
    let mut f=Fixture::new();
    let first=f.request(5,30).unwrap();let older=f.request(5,30).unwrap();
    let response=f.reply(first);f.admit(response,6).unwrap();
    f.router.remote.take_result(f.caller,first).unwrap();
    let newer=f.request(6,30).unwrap();assert!(first<older&&older<newer);
    for p in f.router.remote.pending.iter_mut().flatten(){p.sent=false;}
    let mut ids=[0;2];
    for id in &mut ids {
        let index=f.router.remote.next_unsent().unwrap();
        let request=f.router.remote.pending[index].as_ref().unwrap().request;
        *id=request.message.id;
        f.admit(data(f.peer,request.reference,request.message),7).unwrap();
        f.router.remote.pending[index].as_mut().unwrap().sent=true;
    }
    assert_eq!(ids,[older,newer]);
    let old=f.router.remote.pending.iter().flatten().find(|p|p.request.message.id==older).unwrap().request;
    assert_eq!(f.admit(data(f.peer,old.reference,old.message),8),Err(RemoteError::ReplayRejected));
}

// ------------------------=
// FUNC: unsent_stream_heads_are_fair_under_backpressure
// DESC: Exercises fixed scheduler state with two peer streams, proving a blocked head is retried without selecting its newer request or starving another peer.
// ------------------=
#[test]
fn unsent_stream_heads_are_fair_under_backpressure(){
    let mut f=Fixture::new();
    for _ in 0..4{f.request(5,30).unwrap();}
    for (i,p) in f.router.remote.pending.iter_mut().enumerate().take(4){
        let p=p.as_mut().unwrap();p.sent=false;
        // Scheduler-only second stream: no fabricated peer enters transport or admission.
        if i>=2{p.request.peer=NodeId([0x91;32]);}
    }
    assert_eq!(f.router.remote.next_unsent(),Some(0));
    assert_eq!(f.router.remote.next_unsent(),Some(2));
    assert_eq!(f.router.remote.next_unsent(),Some(0));
    f.router.remote.pending[2].as_mut().unwrap().sent=true;
    assert_eq!(f.router.remote.next_unsent(),Some(3));
    f.router.remote.pending[3].as_mut().unwrap().sent=true;
    assert_eq!(f.router.remote.next_unsent(),Some(0));
    // Terminal/cancelled older work does not indefinitely fence this stream.
    f.router.remote.pending[0]=None;
    assert_eq!(f.router.remote.next_unsent(),Some(1));
    f.router.remote.pending[1].as_mut().unwrap().sent=true;
    assert_eq!(f.router.remote.next_unsent(),None);
}

// ------------------------=
// FUNC: storage_grants_require_human_approval_and_one_registered_operation
// DESC: Exercises the real node grant executor with storage operation IDs and proves explicit consent, exact operation, scope and expiry remain enforced.
// ------------------=
#[test]
fn storage_grants_require_human_approval_and_one_registered_operation() {
    use crate::runtime::iop::{execute_node_operation, IopError, NODE_OPERATION_HUMAN_APPROVED};
    let mut f = Fixture::new();
    let mut request = NodeOperationV1 { node_id: f.peer.0, handle: 0, scope: 42, lease_deadline: 100,
        operation: OperationId::NodeCapabilityGrant.machine_id(), rights: 1,
        value: OperationId::ReplicaTransferChunk.machine_id(), flags: 0, schema_version: 1 };
    assert_eq!(execute_node_operation(&mut f.nodes, OperationId::NodeCapabilityGrant, request, 5, 7), Err(IopError::AccessDenied));
    request.flags = NODE_OPERATION_HUMAN_APPROVED;
    let granted = execute_node_operation(&mut f.nodes, OperationId::NodeCapabilityGrant, request, 5, 7).unwrap();
    assert!(f.nodes.authorize_remote(granted.handle, f.peer, request.value, 42, 1, 6).is_ok());
    assert!(f.nodes.authorize_remote(granted.handle, f.peer, OperationId::ReplicaTransferBegin.machine_id(), 42, 1, 6).is_err());
    assert!(f.nodes.authorize_remote(granted.handle, f.peer, request.value, 43, 1, 6).is_err());
    assert!(f.nodes.authorize_remote(granted.handle, f.peer, request.value, 42, 1, 100).is_err());
    request.value = u32::MAX;
    assert_eq!(execute_node_operation(&mut f.nodes, OperationId::NodeCapabilityGrant, request, 5, 7), Err(IopError::InvalidPayload));
}

// ------------------------=
// FUNC: compute_grants_accept_only_registered_request_and_cancel_operations
// DESC: Exercises the real node grant executor and proves both compute wire operations receive exact peer, scope, rights and lease-bound authority.
// ------------------=
#[test]
fn compute_grants_accept_only_registered_request_and_cancel_operations() {
    use crate::runtime::iop::{execute_node_operation, NODE_OPERATION_HUMAN_APPROVED};
    let mut f = Fixture::new();
    for operation in [OperationId::ComputeRequest, OperationId::ComputeCancel] {
        let request = NodeOperationV1 {
            node_id: f.peer.0,
            handle: 0,
            scope: 11,
            lease_deadline: 100,
            operation: OperationId::NodeCapabilityGrant.machine_id(),
            rights: 1,
            value: operation.machine_id(),
            flags: NODE_OPERATION_HUMAN_APPROVED,
            schema_version: 1,
        };
        let granted = execute_node_operation(
            &mut f.nodes,
            OperationId::NodeCapabilityGrant,
            request,
            5,
            7,
        )
        .unwrap();
        assert!(f
            .nodes
            .authorize_remote(granted.handle, f.peer, operation.machine_id(), 11, 1, 6)
            .is_ok());
        assert!(f
            .nodes
            .authorize_remote(granted.handle, f.peer, operation.machine_id(), 12, 1, 6)
            .is_err());
    }
}

impl Fixture {
    // ------------------------=
    // FUNC: new
    // DESC: Creates explicit local trust, session and narrow authority for HOST router-boundary tests.
    // ------------------=
    fn new() -> Self {
        let mut nodes = NodeRuntime::new();
        nodes.initialize(&[17; 32], true).unwrap();
        let mut remote = NodeRuntime::new();
        let peer = remote.initialize(&[34; 32], true).unwrap();
        nodes
            .discover(remote.advertise(1, 1, 1).unwrap(), 1)
            .unwrap();
        let pair = nodes.begin_pairing(peer, 2).unwrap();
        nodes
            .confirm_pairing(pair.id, pair.verification_code, true, 3, 1)
            .unwrap();
        let (_, public) = super::super::super::crypto::NodeCrypto::agreement_keypair(&[67; 32]);
        let session = nodes
            .open_session(peer, &[51; 32], &public, b"router-test-one", 4, 1)
            .unwrap();
        let mut policy = nodes.discovered_nodes()[0].unwrap().policy;
        policy.categories[0] = PolicyDecision::Allow;
        policy.categories[1] = PolicyDecision::Allow;
        policy.categories[2] = PolicyDecision::Allow;
        nodes.update_policy(peer, policy, 4, 1).unwrap();
        let grant = nodes
            .grant_remote(
                peer,
                OperationId::NodePolicyUpdate.machine_id(),
                0,
                1,
                200,
                4,
                1,
            )
            .unwrap();
        let caller = SecurityIdentity([81; 16]);
        let mut caps = CapabilityManager::new();
        let cap = caps
            .grant(
                CapabilityType::ServiceCall,
                OperationId::NodeTrustRead.machine_id() as u64,
                1,
                0,
                caller,
                caller,
                Some(100),
                0,
            )
            .unwrap();
        Self {
            router: IopRouter::new(),
            nodes,
            caps,
            caller,
            peer,
            cap,
            session,
            grant,
        }
    }

    // ------------------------=
    // FUNC: request
    // DESC: Submits through the public router and models only the successful transport handoff in this unit fixture.
    // ------------------=
    fn request(&mut self, now: u64, deadline: u64) -> Result<u64, RemoteError> {
        let id = self.router.request_remote_node(
            &self.caps,
            &self.nodes,
            self.caller,
            self.cap,
            self.peer,
            1,
            payload(self.nodes.local_id().unwrap(), OperationId::NodeTrustRead),
            71,
            72,
            now,
            deadline,
        )?;
        self.router
            .remote
            .pending
            .iter_mut()
            .flatten()
            .find(|p| p.request.message.id == id)
            .unwrap()
            .sent = true;
        Ok(id)
    }

    // ------------------------=
    // FUNC: reply
    // DESC: Encodes a peer response at the authenticated-data router boundary without pretending this is a network test.
    // ------------------=
    fn reply(&self, id: u64) -> ReceivedData {
        let pending = self
            .router
            .remote
            .pending
            .iter()
            .flatten()
            .find(|p| p.request.message.id == id)
            .unwrap();
        let mut message = pending.request.message;
        message.kind = 2;
        message.causation = id;
        data(self.peer, pending.request.reference, message)
    }

    // ------------------------=
    // FUNC: admit
    // DESC: Exercises actual public admission and its capability, session and correlation checks.
    // ------------------=
    fn admit(&mut self, message: ReceivedData, now: u64) -> Result<(), RemoteError> {
        self.router
            .receive_remote_node(&self.caps, &self.nodes, message, now)
    }

    // ------------------------=
    // FUNC: poll
    // DESC: Exercises bounded production completion cleanup independently of a transport fixture.
    // ------------------=
    fn poll(&mut self, now: u64) {
        self.router
            .poll_remote_node(&self.caps, &mut self.nodes, &mut WireTrust::new(), now);
    }
}

// ------------------------=
// FUNC: payload
// DESC: Builds canonical typed requests with no unbounded fields.
// ------------------=
fn payload(node: NodeId, op: OperationId) -> NodeOperationV1 {
    NodeOperationV1 {
        node_id: node.0,
        handle: 0,
        scope: 0,
        lease_deadline: 0,
        operation: op.machine_id(),
        rights: 0,
        value: 0,
        flags: 0,
        schema_version: 1,
    }
}

// ------------------------=
// FUNC: data
// DESC: Wraps encoded IOP bytes in the authenticated delivery type for HOST admission tests only.
// ------------------=
fn data(peer: NodeId, reference: [u8; 16], envelope: Envelope) -> ReceivedData {
    let mut bytes = [0; 192];
    let (encoded, length) = encode(envelope).unwrap();
    bytes[..length].copy_from_slice(&encoded[..length]);
    ReceivedData {
        peer,
        reference,
        length,
        bytes,
    }
}

// ------------------------=
// FUNC: storage_payload
// DESC: Constructs a full-identity HOST schema fixture; it is not an installed storage replica.
// ------------------=
fn storage_payload() -> StorageOperationV1 {
    StorageOperationV1 { operation: super::super::storage_protocol::Operation::TransferChunk,
        object: [239; 16], authority_generation: 9, manifest_generation: 17, object_version: 3,
        offset: 65536, scope: 0, value: 4, length: 64, data: [42; 64] }
}

// ------------------------=
// FUNC: compute_payload
// DESC: Constructs one bounded typed remote CPU request for authenticated frame and admission behavior.
// ------------------=
fn compute_payload(target_node: NodeId) -> ComputeDispatchV1 {
    ComputeDispatchV1 { schema_version: super::super::super::compute::COMPUTE_SCHEMA_VERSION,
        workload_kind: super::super::super::compute::WorkloadKind::CpuChecksum,
        state: super::super::super::compute::ComputeState::Placed as u8,
        operation: OperationId::ComputeRequest as u32, task_id: 41, epoch: 2, work_units: 9,
        memory_bytes: 4096, deadline: 20, scope: 0, workload_id: [7; 16], input_refs: [[8; 16], [9; 16]],
        target_node, cpu_units: 4, priority: 128,
        result_contract: super::super::super::compute::COMPUTE_RESULT_CONTRACT_V1,
        error: 0, flags: 0, used_ticks: 0 }
}

// ------------------------=
// FUNC: compute_frames_roundtrip_without_storage_aliasing
// DESC: Proves the full-size compute schema survives the authenticated IOP frame and cannot be decoded as a storage or truncated message.
// ------------------=
#[test]
fn compute_frames_roundtrip_without_storage_aliasing() {
    let target = NodeId([55; 32]); let payload = compute_payload(target);
    let envelope = Envelope { kind: 1, error: 0, id: 11, correlation: 12, causation: 13, grant: 14, lease: 10, payload: Payload::Compute(payload) };
    let (bytes, length) = encode(envelope).unwrap(); assert_eq!(length, COMPUTE_FRAME_BYTES);
    assert_eq!(decode(&bytes[..length]).unwrap(), envelope);
    assert_eq!(decode(&bytes[..STORAGE_FRAME_BYTES]), Err(RemoteError::MalformedRequest));
}

// ------------------------=
// FUNC: authenticated_compute_executes_real_bounded_context_and_rechecks_revocation
// DESC: Exercises receiver admission, dequeue authority, bounded Execution Context work, typed response, cleanup, and live grant revocation without shell transport.
// ------------------=
#[test]
fn authenticated_compute_executes_real_bounded_context_and_rechecks_revocation() {
    for revoked in [false, true] {
        let mut f = Fixture::new(); let local = f.nodes.local_id().unwrap(); let payload = compute_payload(local);
        let grant = f.nodes.grant_remote(f.peer, OperationId::ComputeRequest as u32, 0, 1, 100, 5, 1).unwrap();
        let reference = f.nodes.sessions().iter().flatten().find(|session| session.id == f.session).unwrap().protocol_reference;
        let request = Envelope { kind: 1, error: 0, id: 61, correlation: 62, causation: 63, grant, lease: 14, payload: Payload::Compute(payload) };
        f.admit(data(f.peer, reference, request), 6).unwrap();
        if revoked { f.nodes.revoke_remote(grant, 7, 1).unwrap(); }
        let mut execution = super::super::super::execution::ExecutionManager::new(); let mut invoked = false;
        f.router.execute_remote_compute(&mut f.nodes, 8, |call| {
            invoked = true; super::super::super::compute::execute_remote_dispatch(call.payload, &mut execution, call.local, 8).map_err(|_| RemoteError::RemoteFailure)
        });
        assert_eq!(invoked, !revoked); assert_eq!(execution.count(), 0);
        let response = f.router.remote.responses.iter().flatten().next().copied().unwrap();
        if revoked {
            assert_eq!(error_from_byte(response.message.error), Ok(RemoteError::CapabilityRevoked));
        } else {
            let completed = response.message.payload.compute().unwrap();
            assert_eq!(completed.operation, OperationId::ComputeResult as u32); assert_eq!(completed.work_units, 0); assert_eq!(completed.used_ticks, 9);
        }
    }
}

#[test]
// ------------------------=
// FUNC: compute_policy_does_not_inherit_storage_read_authority
// DESC: Proves an exact compute grant remains insufficient when the independent compute policy category denies execution.
// ------------------=
fn compute_policy_does_not_inherit_storage_read_authority() {
    let mut f = Fixture::new();
    let mut policy = f.nodes.discovered_nodes()[0].unwrap().policy;
    assert_eq!(policy.categories[0], PolicyDecision::Allow);
    policy.categories[2] = PolicyDecision::Deny;
    f.nodes.update_policy(f.peer, policy, 5, 1).unwrap();
    let local = f.nodes.local_id().unwrap();
    let payload = compute_payload(local);
    let grant = f.nodes.grant_remote(f.peer, OperationId::ComputeRequest as u32, payload.scope, 1, 100, 5, 1).unwrap();
    let reference = f.nodes.sessions().iter().flatten().find(|session| session.id == f.session).unwrap().protocol_reference;
    let request = Envelope { kind: 1, error: 0, id: 64, correlation: 65, causation: 66, grant, lease: 14, payload: Payload::Compute(payload) };
    f.admit(data(f.peer, reference, request), 6).unwrap();
    let mut invoked = false;
    f.router.execute_remote_compute(&mut f.nodes, 7, |_| {
        invoked = true;
        Err(RemoteError::RemoteFailure)
    });
    assert!(!invoked);
    assert_eq!(f.router.remote.incoming_count(), 0);
    let response = f.router.remote.responses.iter().flatten().next().copied().unwrap();
    assert_eq!(error_from_byte(response.message.error), Ok(RemoteError::PolicyDenied));
}

#[test]
// ------------------------=
// FUNC: coordinator_binds_placed_task_to_remote_iop_result_and_releases_reservations
// DESC: Proves remote completion, cancellation, hop-timeout failover, fenced restart, and reservation release through the production coordinator.
// ------------------=
fn coordinator_binds_placed_task_to_remote_iop_result_and_releases_reservations() {
    use super::super::super::{
        compute::{ComputeDurability, ComputeLocality, ComputeRequestV1, ComputeService, ComputeState, NodeComputeObservation, RemoteComputeExecutor, WorkloadKind, COMPUTE_RESULT_CONTRACT_V1, COMPUTE_SCHEMA_VERSION},
        compute_operator::{ComputeCoordinator, RemoteComputeAuthority},
        fabric::resources::{Directory, Health, Resource, ResourceId, ResourceKind},
    };
    let mut f = Fixture::new();
    let local = f.nodes.local_id().unwrap();
    let scope = u64::from_le_bytes([7; 8]);
    let advertise_grant = f.nodes.grant_remote(f.peer, OperationId::ResourceAdvertise as u32, 0, 1, 100, 5, 1).unwrap();
    let request_grant = f.nodes.grant_remote(f.peer, OperationId::ComputeRequest as u32, scope, 1, 100, 5, 2).unwrap();
    let cancel_grant = f.nodes.grant_remote(f.peer, OperationId::ComputeCancel as u32, scope, 1, 100, 5, 3).unwrap();
    let mut directory = Directory::new();
    for (kind, marker, capacity) in [(ResourceKind::Compute, 1u8, 8u64), (ResourceKind::Memory, 2, 65_536)] {
        directory.advertise(&f.nodes, f.session, advertise_grant, 0, Resource { id: ResourceId([marker; 16]), owner: f.peer, kind, device: [marker + 2; 16], capacity, available: capacity, reserved: 0, health: Health::Healthy, online: true, capabilities: 1, generation: 1, sequence: 1, expires: 100 }, 5).unwrap();
    }
    let mut compute = ComputeService::new();
    compute.observe_node(&directory, NodeComputeObservation { node: f.peer, trust_domain: 4, latency_us: 120, latency_known: true, load_percent: 2, generation: 1 }).unwrap();
    let compute_capability = f.caps.grant(CapabilityType::ComputeUse, scope, 1, 0, f.caller, f.caller, Some(100), 0).unwrap();
    let request = ComputeRequestV1 { schema_version: COMPUTE_SCHEMA_VERSION, workload_kind: WorkloadKind::CpuChecksum, locality: ComputeLocality::RequireRemote, durability: ComputeDurability::Restartable, priority: 128, privacy_local_only: false, workload_id: [7; 16], input_refs: [[8; 16], [9; 16]], allowed_nodes: [f.peer, NodeId([0; 32])], allowed_node_count: 1, allowed_domains: [4, 0], allowed_domain_count: 1, memory_bytes: 4096, deadline: 90, correlation_id: 88, capability_ref: compute_capability, affinity: 0, anti_affinity: 0, work_units: 9, cpu_units: 2, restart_eligible: true, result_contract: COMPUTE_RESULT_CONTRACT_V1 };
    let task = compute.request(request, f.caller, &f.caps, &mut directory, local, 5).unwrap();
    let mut coordinator = ComputeCoordinator::new();
    coordinator.track(task, f.caller, [RemoteComputeAuthority { node: f.peer, request_grant, cancel_grant }, RemoteComputeAuthority { node: NodeId([0; 32]), request_grant: 0, cancel_grant: 0 }], 1).unwrap();
    let mut local_execution = super::super::super::execution::ExecutionManager::new();
    coordinator.poll(&mut compute, &mut f.router, &mut f.caps, &f.nodes, &mut local_execution, &mut directory, local, f.caller, 6);
    assert_eq!(compute.inspect(task).unwrap().state, ComputeState::Running);
    assert_eq!(local_execution.count(), 0);
    let pending = f.router.remote.pending.iter_mut().flatten().find(|pending| matches!(pending.request.message.payload, Payload::Compute(_))).unwrap();
    let envelope = pending.request.message;
    let mut remote_execution = super::super::super::execution::ExecutionManager::new();
    let mut remote_runtime = RemoteComputeExecutor::new();
    let completed = remote_runtime.execute(envelope.payload.compute().unwrap(), &mut remote_execution, f.peer, 7).unwrap();
    pending.result = Some(RemoteResult { request_id: envelope.id, correlation_id: envelope.correlation, causation_id: envelope.causation, result: Ok(Payload::Compute(completed)) });
    coordinator.poll(&mut compute, &mut f.router, &mut f.caps, &f.nodes, &mut local_execution, &mut directory, local, f.caller, 8);
    let final_state = compute.inspect(task).unwrap();
    assert_eq!(final_state.state, ComputeState::Completed);
    assert_eq!(final_state.node, f.peer);
    assert_eq!(final_state.accounting.used_cpu_ticks, 9);
    assert_eq!(coordinator.task_count(), 0);
    assert_eq!(remote_execution.count(), 0);
    assert_eq!(directory.reserved_for(ResourceId([1; 16]), 1), Some(0));
    assert_eq!(directory.reserved_for(ResourceId([2; 16]), 1), Some(0));

    let long_request = ComputeRequestV1 { correlation_id: 89, work_units: 128, ..request };
    let cancelled_task = compute.request(long_request, f.caller, &f.caps, &mut directory, local, 9).unwrap();
    coordinator.track(cancelled_task, f.caller, [RemoteComputeAuthority { node: f.peer, request_grant, cancel_grant }, RemoteComputeAuthority { node: NodeId([0; 32]), request_grant: 0, cancel_grant: 0 }], 1).unwrap();
    coordinator.poll(&mut compute, &mut f.router, &mut f.caps, &f.nodes, &mut local_execution, &mut directory, local, f.caller, 10);
    let pending_long = f.router.remote.pending.iter_mut().flatten().find(|pending| pending.request.message.payload.compute().is_ok_and(|payload| payload.task_id == cancelled_task && payload.operation == OperationId::ComputeRequest as u32)).unwrap();
    let long_envelope = pending_long.request.message;
    let running = remote_runtime.execute(long_envelope.payload.compute().unwrap(), &mut remote_execution, f.peer, 11).unwrap();
    assert_eq!((running.state, remote_runtime.active_count(), remote_execution.count()), (ComputeState::Running as u8, 1, 1));
    pending_long.result = Some(RemoteResult { request_id: long_envelope.id, correlation_id: long_envelope.correlation, causation_id: long_envelope.causation, result: Ok(Payload::Compute(running)) });
    coordinator.poll(&mut compute, &mut f.router, &mut f.caps, &f.nodes, &mut local_execution, &mut directory, local, f.caller, 12);
    let cancel_capability = f.caps.grant(CapabilityType::ComputeCancel, cancelled_task, 1, 0, f.caller, f.caller, Some(100), 0).unwrap();
    compute.cancel(cancelled_task, f.caller, cancel_capability, &f.caps, &mut local_execution, &mut directory, 13).unwrap();
    coordinator.poll(&mut compute, &mut f.router, &mut f.caps, &f.nodes, &mut local_execution, &mut directory, local, f.caller, 14);
    let pending_cancel = f.router.remote.pending.iter_mut().flatten().find(|pending| pending.request.message.payload.compute().is_ok_and(|payload| payload.operation == OperationId::ComputeCancel as u32)).unwrap();
    let cancel_envelope = pending_cancel.request.message;
    let cancelled = remote_runtime.execute(cancel_envelope.payload.compute().unwrap(), &mut remote_execution, f.peer, 15).unwrap();
    assert_eq!((remote_runtime.active_count(), remote_execution.count()), (0, 0));
    pending_cancel.result = Some(RemoteResult { request_id: cancel_envelope.id, correlation_id: cancel_envelope.correlation, causation_id: cancel_envelope.causation, result: Ok(Payload::Compute(cancelled)) });
    coordinator.poll(&mut compute, &mut f.router, &mut f.caps, &f.nodes, &mut local_execution, &mut directory, local, f.caller, 16);
    assert_eq!(compute.inspect(cancelled_task).unwrap().state, ComputeState::Cancelled);
    assert_eq!(coordinator.task_count(), 0);
    assert_eq!(directory.reserved_for(ResourceId([1; 16]), 1), Some(0));
    assert_eq!(directory.reserved_for(ResourceId([2; 16]), 1), Some(0));

    let mut remote_c = NodeRuntime::new();
    let peer_c = remote_c.initialize(&[92; 32], true).unwrap();
    f.nodes.discover(remote_c.advertise(30, 1, 1).unwrap(), 30).unwrap();
    let pairing_c = f.nodes.begin_pairing(peer_c, 31).unwrap();
    f.nodes.confirm_pairing(pairing_c.id, pairing_c.verification_code, true, 32, 1).unwrap();
    let (_, public_c) = super::super::super::crypto::NodeCrypto::agreement_keypair(&[93; 32]);
    let session_c = f.nodes.open_session(peer_c, &[94; 32], &public_c, b"coordinator-peer-c", 33, 1).unwrap();
    let advertise_c = f.nodes.grant_remote(peer_c, OperationId::ResourceAdvertise as u32, 0, 1, 200, 33, 4).unwrap();
    let request_c = f.nodes.grant_remote(peer_c, OperationId::ComputeRequest as u32, scope, 1, 200, 33, 5).unwrap();
    let cancel_c = f.nodes.grant_remote(peer_c, OperationId::ComputeCancel as u32, scope, 1, 200, 33, 6).unwrap();
    for (kind, marker, capacity) in [(ResourceKind::Compute, 3u8, 8u64), (ResourceKind::Memory, 4, 65_536)] {
        directory.advertise(&f.nodes, session_c, advertise_c, 0, Resource { id: ResourceId([marker; 16]), owner: peer_c, kind, device: [marker + 2; 16], capacity, available: capacity, reserved: 0, health: Health::Healthy, online: true, capabilities: 1, generation: 1, sequence: 1, expires: 200 }, 33).unwrap();
    }
    compute.observe_node(&directory, NodeComputeObservation { node: peer_c, trust_domain: 4, latency_us: 300, latency_known: true, load_percent: 2, generation: 1 }).unwrap();
    let failover_capability = f.caps.grant(CapabilityType::ComputeUse, scope, 1, 0, f.caller, f.caller, Some(200), 0).unwrap();
    let failover_request = ComputeRequestV1 { allowed_nodes: [f.peer, peer_c], allowed_node_count: 2, deadline: 190, correlation_id: 90, capability_ref: failover_capability, work_units: 9, ..request };
    let failover_task = compute.request(failover_request, f.caller, &f.caps, &mut directory, local, 34).unwrap();
    coordinator.track(failover_task, f.caller, [RemoteComputeAuthority { node: f.peer, request_grant, cancel_grant }, RemoteComputeAuthority { node: peer_c, request_grant: request_c, cancel_grant: cancel_c }], 2).unwrap();
    coordinator.poll(&mut compute, &mut f.router, &mut f.caps, &f.nodes, &mut local_execution, &mut directory, local, f.caller, 35);
    let pending_b = f.router.remote.pending.iter_mut().flatten().find(|pending| pending.request.message.payload.compute().is_ok_and(|payload| payload.task_id == failover_task)).unwrap();
    let first_envelope = pending_b.request.message;
    assert_eq!(first_envelope.payload.compute().unwrap().target_node, f.peer);
    pending_b.result = Some(RemoteResult { request_id: first_envelope.id, correlation_id: first_envelope.correlation,
        causation_id: first_envelope.causation, result: Err(RemoteError::DeadlineExceeded) });
    coordinator.poll(&mut compute, &mut f.router, &mut f.caps, &f.nodes, &mut local_execution, &mut directory, local, f.caller, 36);
    let retry = compute.inspect(failover_task).unwrap();
    assert_eq!((retry.state, retry.node, retry.epoch, retry.accounting.restart_count),
        (ComputeState::Running, f.peer, 1, 0));
    let pending_b = f.router.remote.pending.iter_mut().flatten().find(|pending| pending.request.message.payload.compute().is_ok_and(|payload| payload.task_id == failover_task)).unwrap();
    let stale_envelope = pending_b.request.message;
    directory.mark_peer_offline(f.peer);
    pending_b.result = Some(RemoteResult { request_id: stale_envelope.id, correlation_id: stale_envelope.correlation,
        causation_id: stale_envelope.causation, result: Err(RemoteError::DeadlineExceeded) });
    coordinator.poll(&mut compute, &mut f.router, &mut f.caps, &f.nodes, &mut local_execution, &mut directory, local, f.caller, 37);
    let replaced = compute.inspect(failover_task).unwrap();
    assert_eq!((replaced.state, replaced.node, replaced.epoch, replaced.accounting.restart_count),
        (ComputeState::Running, peer_c, 2, 1));
    coordinator.poll(&mut compute, &mut f.router, &mut f.caps, &f.nodes, &mut local_execution, &mut directory, local, f.caller, 38);
    let pending_c = f.router.remote.pending.iter_mut().flatten().find(|pending| pending.request.message.payload.compute().is_ok_and(|payload| payload.task_id == failover_task)).unwrap();
    let envelope_c = pending_c.request.message;
    assert_eq!(envelope_c.payload.compute().unwrap().target_node, peer_c);
    let completed_c = remote_runtime.execute(envelope_c.payload.compute().unwrap(), &mut remote_execution, peer_c, 38).unwrap();
    pending_c.result = Some(RemoteResult { request_id: envelope_c.id, correlation_id: envelope_c.correlation, causation_id: envelope_c.causation, result: Ok(Payload::Compute(completed_c)) });
    coordinator.poll(&mut compute, &mut f.router, &mut f.caps, &f.nodes, &mut local_execution, &mut directory, local, f.caller, 39);
    let recovered = compute.inspect(failover_task).unwrap();
    assert_eq!((recovered.state, recovered.node, recovered.epoch, recovered.accounting.restart_count), (ComputeState::Completed, peer_c, 2, 1));
    let stale_b = remote_runtime.execute(stale_envelope.payload.compute().unwrap(), &mut remote_execution, f.peer, 40).unwrap();
    assert_eq!(compute.accept_remote_result(failover_task, stale_b, &mut directory, 41), Err(super::super::super::compute::ComputeError::StaleResult));
    assert_eq!(compute.inspect(failover_task).unwrap(), recovered);
    assert_eq!(coordinator.task_count(), 0);
}

#[test]
// ------------------------=
// FUNC: remote_executor_retains_and_cancels_live_bounded_contexts
// DESC: Proves multi-slice work remains a real remote context and that typed cancellation and deadline expiry reclaim it.
// ------------------=
fn remote_executor_retains_and_cancels_live_bounded_contexts() {
    use super::super::super::compute::{ComputeState, RemoteComputeExecutor};
    let node = NodeId([44; 32]);
    let mut execution = super::super::super::execution::ExecutionManager::new();
    let mut remote = RemoteComputeExecutor::new();
    let mut request = compute_payload(node); request.work_units = 128;
    let first = remote.execute(request, &mut execution, node, 8).unwrap();
    assert_eq!((first.state, first.work_units, remote.active_count(), execution.count()), (ComputeState::Running as u8, 64, 1, 1));
    let mut altered = request; altered.work_units = 64; altered.used_ticks = 64; altered.memory_bytes = 8192;
    assert_eq!(remote.execute(altered, &mut execution, node, 9), Err(super::super::super::compute::ComputeError::InvalidRequest));
    assert_eq!((remote.active_count(), execution.count()), (1, 1));
    let mut cancel = request; cancel.operation = OperationId::ComputeCancel as u32;
    let cancelled = remote.execute(cancel, &mut execution, node, 10).unwrap();
    assert_eq!((cancelled.state, remote.active_count(), execution.count()), (ComputeState::Cancelled as u8, 0, 0));
    let _ = remote.execute(request, &mut execution, node, 11).unwrap();
    assert_eq!(remote.active_count(), 1);
    remote.expire(&mut execution, request.deadline);
    assert_eq!((remote.active_count(), execution.count()), (0, 0));
}

// ------------------------=
// FUNC: storage_requests_share_authenticated_iop_and_owned_completion
// DESC: Exercises the actual shared router across admission, execution, correlated response, typed mailbox isolation and replay rejection.
// ------------------=
#[test]
fn storage_requests_share_authenticated_iop_and_owned_completion() {
    let mut f = Fixture::new(); let payload = storage_payload();
    let grant = f.nodes.grant_remote(f.peer, payload.operation as u32, 0, 1, 100, 5, 1).unwrap();
    let cap = f.caps.grant(CapabilityType::ServiceCall, payload.operation as u64, 1, 0,
        f.caller, f.caller, Some(100), 0).unwrap();
    let id = f.router.request_remote_storage(&f.caps, &f.nodes, f.caller, cap, f.peer,
        grant, payload, 71, 72, 6, 30).unwrap();
    let pending = f.router.remote.pending.iter_mut().flatten().find(|p| p.request.message.id == id).unwrap();
    pending.sent = true;
    let request = pending.request;
    assert_eq!(encode(request.message).unwrap().1, STORAGE_FRAME_BYTES);
    f.admit(data(f.peer, request.reference, request.message), 7).unwrap();
    f.router.execute_remote_node(&mut f.nodes, 8);
    assert_eq!(f.router.remote.incoming_count(), 1);
    assert_eq!(f.router.execute_remote_storage(&mut f.nodes, 8, |call| {
        assert_eq!(call.peer, f.peer); assert_eq!(call.payload, payload);
        assert_eq!((call.request_id, call.correlation, call.causation), (id, 71, 72));
        Ok((call.payload, 19u64))
    }), Some(19));
    assert_eq!(f.router.remote.executed, 1);
    assert_eq!(f.admit(data(f.peer, request.reference, request.message), 8), Err(RemoteError::ReplayRejected));
    let index = f.router.remote.responses.iter().position(Option::is_some).unwrap();
    let response = f.router.remote.responses[index].take().unwrap();
    f.admit(data(f.peer, response.reference, response.message), 9).unwrap();
    assert!(f.router.remote.take_result(f.caller, id).is_none());
    let result = f.router.remote.take_storage_result(f.caller, id).unwrap();
    assert_eq!(result.result, Ok(payload));
    assert_eq!((result.request_id, result.correlation_id, result.causation_id), (id, 71, id));
    assert!(f.router.remote.take_storage_result(f.caller, id).is_none());
}

// ------------------------=
// FUNC: queued_storage_chunk_revalidates_revocation_scope_and_lease
// DESC: Proves an admitted chunk cannot execute after authority changes, using the same real remote dequeue gate as node operations.
// ------------------=
#[test]
fn queued_storage_chunk_revalidates_revocation_scope_and_lease() {
    for scenario in 0..4 {
        let mut f = Fixture::new(); let payload = storage_payload();
        let operation = if scenario == 1 { 0xe021 } else { payload.operation as u32 };
        let grant = f.nodes.grant_remote(f.peer, operation, 0, 1, 100, 5, 1).unwrap();
        let reference = f.nodes.sessions().iter().flatten().find(|s| s.id == f.session).unwrap().protocol_reference;
        let request = Envelope { kind: 1, error: 0, id: 1, correlation: 4, causation: 3, grant,
            lease: 10, payload: Payload::Storage(payload) };
        f.admit(data(f.peer, reference, request), 6).unwrap();
        let expected = match scenario {
            0 => { f.nodes.revoke_remote(grant, 7, 1).unwrap(); RemoteError::CapabilityRevoked },
            1 => RemoteError::CapabilityScopeDenied,
            2 => RemoteError::DeadlineExceeded,
            _ => { let mut policy = f.nodes.discovered_nodes()[0].unwrap().policy;
                policy.categories[0] = PolicyDecision::Deny;
                f.nodes.update_policy(f.peer, policy, 7, 1).unwrap(); RemoteError::PolicyDenied },
        };
        let result: Option<()> = f.router.execute_remote_storage(&mut f.nodes, if scenario == 2 { 17 } else { 8 },
            |_| panic!("unauthorized storage execution"));
        assert_eq!(result, None); assert_eq!(f.router.remote.executed, 0);
        let response = f.router.remote.responses.iter().flatten().next().unwrap();
        assert_eq!(response.message.error, expected as u8);
    }
}

#[test]
// ------------------------=
// FUNC: response_revalidates_revoked_caller_before_disclosure
// DESC: Rejects a correctly correlated queued response after caller capability revocation and releases its mailbox once.
// ------------------=
fn response_revalidates_revoked_caller_before_disclosure() {
    let mut f = Fixture::new();
    let id = f.request(10, 30).unwrap();
    let reply = f.reply(id);
    f.caps.revoke(f.cap).unwrap();
    assert_eq!(f.admit(reply, 11), Err(RemoteError::AccessDenied));
    assert_eq!(
        f.router.remote.take_result(f.caller, id).unwrap().result,
        Err(RemoteError::AccessDenied)
    );
    assert!(f.router.remote.take_result(f.caller, id).is_none());
}

#[test]
// ------------------------=
// FUNC: exact_response_identity_and_correlation
// DESC: Rejects wrong peer, session, operation, correlation, causation and selected subject without completing the caller.
// ------------------=
fn exact_response_identity_and_correlation() {
    let mut f = Fixture::new();
    let id = f.request(10, 30).unwrap();
    for mode in 0..6 {
        let mut reply = f.reply(id);
        match mode {
            0 => reply.peer = NodeId([9; 32]),
            1 => reply.reference[0] ^= 1,
            2 => reply.bytes[104..108]
                .copy_from_slice(&OperationId::NodePolicyRead.machine_id().to_le_bytes()),
            3 => reply.bytes[16] ^= 1,
            4 => reply.bytes[24] ^= 1,
            _ => reply.bytes[48] ^= 1,
        }
        assert!(f.admit(reply, 11).is_err());
        assert!(f.router.remote.take_result(f.caller, id).is_none());
    }
    let reply = f.reply(id);
    assert_eq!(f.admit(reply, 12), Ok(()));
    let duplicate = f.reply(id);
    assert_eq!(f.admit(duplicate, 12), Err(RemoteError::ReplayRejected));
    assert!(f
        .router
        .remote
        .take_result(f.caller, id)
        .unwrap()
        .result
        .is_ok());
    let late = data(
        f.peer,
        f.nodes
            .sessions()
            .iter()
            .flatten()
            .find(|s| s.id == f.session)
            .unwrap()
            .protocol_reference,
        Envelope {
            kind: 2,
            error: 0,
            id,
            correlation: 71,
            causation: id,
            grant: 1,
            lease: 10,
            payload: payload(f.nodes.local_id().unwrap(), OperationId::NodeTrustRead).into(),
        },
    );
    assert_eq!(f.admit(late, 13), Err(RemoteError::UnknownResponse));
}

#[test]
// ------------------------=
// FUNC: session_loss_cleans_pending_and_reconnect_cannot_resurrect_it
// DESC: Verifies typed session-loss failure, cleanup, fresh-reference isolation and rejection of old correlations after reconnect.
// ------------------=
fn session_loss_cleans_pending_and_reconnect_cannot_resurrect_it() {
    let mut f = Fixture::new();
    let id = f.request(10, 30).unwrap();
    let old = f.reply(id);
    f.nodes.close_session(f.session, 11, 1).unwrap();
    f.poll(11);
    assert_eq!(
        f.router.remote.take_result(f.caller, id).unwrap().result,
        Err(RemoteError::TransportClosed)
    );
    assert!(f.router.remote.pending.iter().all(Option::is_none));
    assert_eq!(f.admit(old, 12), Err(RemoteError::SessionNotFound));
    let (_, public) = super::super::super::crypto::NodeCrypto::agreement_keypair(&[68; 32]);
    f.session = f
        .nodes
        .open_session(f.peer, &[52; 32], &public, b"router-test-two", 13, 1)
        .unwrap();
    let new_id = f.request(14, 30).unwrap();
    assert_ne!(id, new_id);
    let mut stale_id = f.reply(new_id);
    stale_id.bytes[8..16].copy_from_slice(&id.to_le_bytes());
    stale_id.bytes[24..32].copy_from_slice(&id.to_le_bytes());
    assert_eq!(f.admit(stale_id, 15), Err(RemoteError::UnknownResponse));
    assert!(f.router.remote.take_result(f.caller, new_id).is_none());
    let valid = f.reply(new_id);
    f.admit(valid, 16).unwrap();
    assert!(f
        .router
        .remote
        .take_result(f.caller, new_id)
        .unwrap()
        .result
        .is_ok());
}

#[test]
// ------------------------=
// FUNC: late_reply_deadline_capacity_and_checked_ids
// DESC: Exercises strict deadline, bounded pending capacity, mailbox expiry and nonwrapping request identifiers.
// ------------------=
fn late_reply_deadline_capacity_and_checked_ids() {
    let mut f = Fixture::new();
    let id = f.request(10, 12).unwrap();
    let reply = f.reply(id);
    assert_eq!(f.admit(reply, 12), Err(RemoteError::DeadlineExceeded));
    f.poll(12);
    assert_eq!(
        f.router.remote.take_result(f.caller, id).unwrap().result,
        Err(RemoteError::DeadlineExceeded)
    );
    for _ in 0..CAPACITY {
        f.request(13, 20).unwrap();
    }
    assert_eq!(f.request(13, 20), Err(RemoteError::QueueFull));
    f.poll(20);
    f.poll(50);
    assert!(f.router.remote.pending.iter().all(Option::is_none));
    f.router.remote.next = u64::MAX - 1;
    assert_eq!(f.request(51, 60), Ok(u64::MAX - 1));
    assert_eq!(f.request(51, 60), Err(RemoteError::QueueFull));
    assert_eq!(f.request(51, 60), Err(RemoteError::QueueFull));
    assert_eq!(f.router.remote.pending.iter().flatten().count(), 1);
}

#[test]
// ------------------------=
// FUNC: duplicate_mutation_executes_only_once
// DESC: Proves duplicate requests before and after completion cannot advance policy or execute the service twice.
// ------------------=
fn duplicate_mutation_executes_only_once() {
    let mut f = Fixture::new();
    f.router.remote.set_mutation_service_ready(true); // Explicit HOST in-memory fixture only.
    let reference = f
        .nodes
        .sessions()
        .iter()
        .flatten()
        .find(|s| s.id == f.session)
        .unwrap()
        .protocol_reference;
    let message = Envelope {
        kind: 1,
        error: 0,
        id: 1,
        correlation: 2,
        causation: 3,
        grant: f.grant,
        lease: 20,
        payload: payload(f.peer, OperationId::NodePolicyUpdate).into(),
    };
    f.admit(data(f.peer, reference, message), 10).unwrap();
    assert_eq!(
        f.admit(data(f.peer, reference, message), 10),
        Err(RemoteError::ReplayRejected)
    );
    let before = f.nodes.discovered_nodes()[0].unwrap().policy.version;
    f.router.execute_remote_node_durable(&mut f.nodes, 11, &mut |_| true);
    assert_eq!(
        f.nodes.discovered_nodes()[0].unwrap().policy.version,
        before + 1
    );
    assert_eq!(
        f.nodes.discovered_nodes()[0].unwrap().policy.categories[0],
        PolicyDecision::Deny
    );
    assert_eq!(
        f.admit(data(f.peer, reference, message), 12),
        Err(RemoteError::ReplayRejected)
    );
    f.router.execute_remote_node(&mut f.nodes, 12);
    assert_eq!(f.router.remote.executed, 1);
    assert_eq!(
        f.nodes.discovered_nodes()[0].unwrap().policy.version,
        before + 1
    );
}

#[test]
// ------------------------=
// FUNC: durable_policy_commit_and_restart_checkpoint
// DESC: Exercises router-authorized staging, typed commit notice, persisted policy reconstruction and monotonically advancing checkpoints.
// ------------------=
fn durable_policy_commit_and_restart_checkpoint() {
    let mut f = Fixture::new();
    let reference = f
        .nodes
        .sessions()
        .iter()
        .flatten()
        .find(|s| s.id == f.session)
        .unwrap()
        .protocol_reference;
    let mut durable = [0; super::super::super::node::types::NODE_STATE_BYTES];
    for id in 1..=2 {
        let mut operation = payload(f.peer, OperationId::NodePolicyUpdate);
        operation.value = (id - 1) as u32;
        let request = Envelope {
            kind: 1,
            error: 0,
            id,
            correlation: 12,
            causation: 13,
            grant: f.grant,
            lease: 20,
            payload: operation.into(),
        };
        f.admit(data(f.peer, reference, request), 10).unwrap();
        let before = f.nodes.control_version();
        let notice = f
            .router
            .execute_remote_node_durable(&mut f.nodes, 11, &mut |bytes| {
                durable.copy_from_slice(bytes);
                true // HOST persistence adapter; not disk evidence.
            })
            .unwrap();
        assert_eq!(notice.version, before + 1);
        assert_eq!(notice.correlation, 12);
        assert_eq!(notice.causation, id);
        assert_eq!(notice.subject, f.peer);
        assert_eq!(f.nodes.control_version(), notice.version);
        let mut restored = NodeRuntime::new();
        restored.restore_state(&durable).unwrap();
        assert_eq!(restored.control_version(), notice.version);
        assert_eq!(
            restored.discovered_nodes()[0].unwrap().policy,
            f.nodes.discovered_nodes()[0].unwrap().policy
        );
    }
    assert_eq!(f.router.remote.executed, 2);
}

#[test]
// ------------------------=
// FUNC: durable_failed_persistence_cannot_change_authority_or_emit_commit_notice
// DESC: Forces durable write failure and verifies policy, sessions, grants, audit checkpoint and control checkpoint all remain unchanged.
// ------------------=
fn durable_failed_persistence_cannot_change_authority_or_emit_commit_notice() {
    let mut f = Fixture::new();
    let reference = f
        .nodes
        .sessions()
        .iter()
        .flatten()
        .find(|s| s.id == f.session)
        .unwrap()
        .protocol_reference;
    let before = f.nodes.encode_state().unwrap();
    let sessions = *f.nodes.sessions();
    let grants = *f.nodes.remote_grants();
    let mut request = payload(f.peer, OperationId::NodeTrustUpdate);
    request.value = 4; // Restricted, which would close sessions and revoke grants.
    let error = f
        .nodes
        .commit_control(OperationId::NodeTrustUpdate, request, 10, 1, 2, |_| false);
    assert_eq!(
        error,
        Err(super::super::super::node::control::CommitError::PersistenceFailed)
    );
    assert_eq!(f.nodes.encode_state().unwrap(), before);
    assert!(*f.nodes.sessions() == sessions);
    assert_eq!(*f.nodes.remote_grants(), grants);
    let request = Envelope {
        kind: 1,
        error: 0,
        id: 1,
        correlation: 2,
        causation: 3,
        grant: f.grant,
        lease: 20,
        payload: payload(f.peer, OperationId::NodePolicyUpdate).into(),
    };
    f.admit(data(f.peer, reference, request), 10).unwrap();
    assert!(f
        .router
        .execute_remote_node_durable(&mut f.nodes, 11, &mut |_| false)
        .is_none());
    assert_eq!(
        f.router
            .remote
            .responses
            .iter()
            .flatten()
            .next()
            .unwrap()
            .message
            .error,
        RemoteError::PersistenceFailed as u8
    );
    assert_eq!(f.router.remote.executed, 0);
    assert_eq!(f.nodes.control_version(), 0);
    assert_eq!(
        f.nodes.discovered_nodes()[0].unwrap().policy.categories[0],
        PolicyDecision::Allow
    );
}

#[test]
// ------------------------=
// FUNC: durable_dispatch_revalidates_before_invoking_storage
// DESC: Revokes an admitted peer grant and proves the durable writer is never called after execution authority is lost.
// ------------------=
fn durable_dispatch_revalidates_before_invoking_storage() {
    let mut f = Fixture::new();
    let reference = f
        .nodes
        .sessions()
        .iter()
        .flatten()
        .find(|s| s.id == f.session)
        .unwrap()
        .protocol_reference;
    let request = Envelope {
        kind: 1,
        error: 0,
        id: 1,
        correlation: 2,
        causation: 3,
        grant: f.grant,
        lease: 20,
        payload: payload(f.peer, OperationId::NodePolicyUpdate).into(),
    };
    f.admit(data(f.peer, reference, request), 10).unwrap();
    f.nodes.revoke_remote(f.grant, 11, 1).unwrap();
    let mut writes = 0;
    assert!(f
        .router
        .execute_remote_node_durable(&mut f.nodes, 12, &mut |_| {
            writes += 1;
            true
        })
        .is_none());
    assert_eq!(writes, 0);
    assert_eq!(f.nodes.control_version(), 0);
    assert_eq!(
        f.router
            .remote
            .responses
            .iter()
            .flatten()
            .next()
            .unwrap()
            .message
            .error,
        RemoteError::CapabilityRevoked as u8
    );
}

#[test]
// ------------------------=
// FUNC: policy_read_returns_every_committed_field_within_wire_bounds
// DESC: Reads through the remote router and reconstructs all twelve decisions, scope, lifetime, policy version and commit checkpoint.
// ------------------=
fn policy_read_returns_every_committed_field_within_wire_bounds() {
    let mut f = Fixture::new();
    let choices = [
        PolicyDecision::Allow,
        PolicyDecision::Allow,
        PolicyDecision::SessionOnly,
        PolicyDecision::Leased,
    ];
    let mut durable = [0; super::super::super::node::types::NODE_STATE_BYTES];
    for index in 0..12 {
        let mut op = payload(f.peer, OperationId::NodePolicyUpdate);
        op.flags = index;
        op.value = match choices[index as usize % 4] {
            PolicyDecision::Deny => 0,
            PolicyDecision::Allow => 1,
            PolicyDecision::SessionOnly => 2,
            PolicyDecision::Leased => 3,
        };
        op.scope = 93;
        op.lease_deadline = 190;
        f.nodes
            .commit_control(OperationId::NodePolicyUpdate, op, 10, 1, 2, |bytes| {
                durable.copy_from_slice(bytes);
                true
            })
            .unwrap();
    }
    let grant = f
        .nodes
        .grant_remote(
            f.peer,
            OperationId::NodePolicyRead.machine_id(),
            93,
            1,
            190,
            11,
            1,
        )
        .unwrap();
    let mut op = payload(f.peer, OperationId::NodePolicyRead);
    op.scope = 93;
    let reference = f
        .nodes
        .sessions()
        .iter()
        .flatten()
        .find(|s| s.id == f.session)
        .unwrap()
        .protocol_reference;
    f.admit(
        data(
            f.peer,
            reference,
            Envelope {
                kind: 1,
                error: 0,
                id: 1,
                correlation: 2,
                causation: 3,
                grant,
                lease: 20,
                payload: op.into(),
            },
        ),
        11,
    )
    .unwrap();
    f.router.execute_remote_node(&mut f.nodes, 12);
    let response = f
        .router
        .remote
        .responses
        .iter()
        .flatten()
        .next()
        .unwrap()
        .message;
    assert_eq!(response.error, 0);
    assert_eq!(encode(response).unwrap().1, FRAME_BYTES);
    let decoded = NodeOperationV1::decode(&response.payload.node().unwrap().encode()).unwrap();
    let mut restored = NodeRuntime::new();
    restored.restore_state(&durable).unwrap();
    let expected = restored.discovered_nodes()[0].unwrap().policy;
    assert_eq!(decoded.handle, restored.control_version());
    assert_eq!(decoded.scope, expected.scope);
    assert_eq!(decoded.lease_deadline, expected.expires_at);
    assert_eq!(decoded.value, expected.version);
    assert_eq!(decoded.flags, 12);
    for i in 0..12 {
        let actual = match (decoded.rights >> (i * 2)) & 3 {
            0 => PolicyDecision::Deny,
            1 => PolicyDecision::Allow,
            2 => PolicyDecision::SessionOnly,
            _ => PolicyDecision::Leased,
        };
        assert_eq!(actual, expected.categories[i]);
    }
}
