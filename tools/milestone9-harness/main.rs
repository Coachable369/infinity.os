#[path = "../../kernel/runtime/crypto.rs"]
mod crypto;
#[path = "../../kernel/runtime/node/mod.rs"]
mod node;
#[path = "../../kernel/runtime/execution.rs"]
mod execution;
#[path = "../../kernel/runtime/capability.rs"]
mod capability;
#[path = "../../kernel/runtime/iop.rs"]
mod iop;
#[path = "../../kernel/runtime/event.rs"]
mod event;

use node::types::{MeshRole, NodeError, TrustState};
use node::NodeRuntime;
mod pairing_acceptance;

// ------------------------=
// FUNC: dispatch_request
// DESC: Sends one capability-authorized NodeOperationV1 request through IOP and returns its typed response.
// ------------------=
fn dispatch_request(
    router: &mut iop::IopRouter,
    capabilities: &mut capability::CapabilityManager,
    nodes: &mut NodeRuntime,
    operation: iop::OperationId,
    request: iop::NodeOperationV1,
    request_id: u64,
    now: u64,
) -> Result<iop::NodeOperationV1, iop::IopError> {
    use capability::CapabilityType;
    use execution::SecurityIdentity;
    let caller = SecurityIdentity([0x51; 16]);
    let service = SecurityIdentity([0x52; 16]);
    let issuer = SecurityIdentity([0x53; 16]);
    let capability = capabilities
        .grant(
            CapabilityType::ServiceCall,
            operation.machine_id() as u64,
            1,
            0,
            issuer,
            caller,
            Some(now + 100),
            0,
        )
        .unwrap();
    let message = iop::IopMessage::request(
        operation,
        request_id,
        caller,
        capability,
        now + 50,
        request_id,
        &request.encode(),
    )?;
    router.send(2, message, capabilities, now)?;
    iop::dispatch_node_operation(router, capabilities, nodes, operation, 2, 1, service, now)?;
    iop::NodeOperationV1::decode(router.receive(1, now)?.bytes())
}

// ------------------------=
// FUNC: node_request
// DESC: Creates a canonical version-one node request for the selected stable Node ID.
// ------------------=
fn node_request(node_id: node::types::NodeId, operation: iop::OperationId) -> iop::NodeOperationV1 {
    iop::NodeOperationV1 {
        node_id: node_id.0,
        handle: 0,
        scope: 0,
        lease_deadline: 0,
        operation: operation.machine_id(),
        rights: 0,
        value: 0,
        flags: 0,
        schema_version: 1,
    }
}

// ------------------------=
// FUNC: iop_node_management_operations
// DESC: Exercises node inventory, trust, mesh, policy, audit, malformed, unauthorized, and expired IOP behavior.
// ------------------=
fn iop_node_management_operations() {
    use capability::{CapabilityManager, CapabilityType};
    use execution::SecurityIdentity;
    use iop::{IopError, IopMessage, IopRouter, NodeOperationV1, OperationId};

    let mut local = NodeRuntime::new();
    let mut remote = NodeRuntime::new();
    local.initialize(&[0x81; 32], true).unwrap();
    let remote_id = remote.initialize(&[0x82; 32], true).unwrap();
    local.discover(remote.advertise(1, 1, 1).unwrap(), 1).unwrap();
    let pairing = local.begin_pairing(remote_id, 2).unwrap();
    local.confirm_pairing(pairing.id, pairing.verification_code, true, 3, 1).unwrap();

    let mut capabilities = CapabilityManager::new();
    let mut router = IopRouter::new();
    router.register_endpoint(1, SecurityIdentity([0x51; 16])).unwrap();
    router.register_endpoint(2, SecurityIdentity([0x52; 16])).unwrap();

    let listed = dispatch_request(&mut router, &mut capabilities, &mut local, OperationId::NodeList, node_request(remote_id, OperationId::NodeList), 100, 4).unwrap();
    assert_eq!(listed.value, 1);
    let inspected = dispatch_request(&mut router, &mut capabilities, &mut local, OperationId::NodeInspect, node_request(remote_id, OperationId::NodeInspect), 101, 5).unwrap();
    assert_eq!(inspected.value, 3);

    let mut restricted = node_request(remote_id, OperationId::NodeTrustUpdate);
    restricted.value = 4;
    dispatch_request(&mut router, &mut capabilities, &mut local, OperationId::NodeTrustUpdate, restricted, 102, 6).unwrap();
    assert_eq!(local.discovered_nodes()[0].unwrap().trust, TrustState::Restricted);
    let mut trusted = restricted;
    trusted.value = 3;
    dispatch_request(&mut router, &mut capabilities, &mut local, OperationId::NodeTrustUpdate, trusted, 103, 7).unwrap();

    let mut policy = node_request(remote_id, OperationId::NodePolicyUpdate);
    policy.flags = 2;
    policy.value = 2;
    policy.scope = 0x44;
    policy.lease_deadline = 80;
    dispatch_request(&mut router, &mut capabilities, &mut local, OperationId::NodePolicyUpdate, policy, 104, 8).unwrap();
    assert_eq!(local.discovered_nodes()[0].unwrap().policy.categories[2], node::types::PolicyDecision::SessionOnly);

    dispatch_request(&mut router, &mut capabilities, &mut local, OperationId::NodeJoin, node_request(remote_id, OperationId::NodeJoin), 105, 9).unwrap();
    assert_eq!(local.mesh_members().iter().flatten().filter(|member| member.enabled).count(), 1);
    dispatch_request(&mut router, &mut capabilities, &mut local, OperationId::NodeLeave, node_request(remote_id, OperationId::NodeLeave), 106, 10).unwrap();
    assert_eq!(local.mesh_members().iter().flatten().filter(|member| member.enabled).count(), 0);
    let audits = dispatch_request(&mut router, &mut capabilities, &mut local, OperationId::NodeAuditList, node_request(remote_id, OperationId::NodeAuditList), 107, 11).unwrap();
    assert!(audits.value > 0);

    let caller = SecurityIdentity([0x51; 16]);
    let unauthorized = IopMessage::request(OperationId::NodeInspect, 108, caller, 9999, 60, 108, &node_request(remote_id, OperationId::NodeInspect).encode()).unwrap();
    assert_eq!(router.send(2, unauthorized, &capabilities, 12), Err(IopError::AccessDenied));
    let expired_capability = capabilities.grant(CapabilityType::ServiceCall, OperationId::NodeInspect.machine_id() as u64, 1, 0, caller, caller, Some(20), 0).unwrap();
    let expired = IopMessage::request(OperationId::NodeInspect, 109, caller, expired_capability, 20, 109, &node_request(remote_id, OperationId::NodeInspect).encode()).unwrap();
    assert_eq!(router.send(2, expired, &capabilities, 20), Err(IopError::DeadlineExceeded));

    let valid = node_request(remote_id, OperationId::NodeInspect).encode();
    assert_eq!(NodeOperationV1::decode(&valid[..79]), Err(IopError::InvalidPayload));
    let mut wrong_version = valid;
    wrong_version[72] = 2;
    assert_eq!(NodeOperationV1::decode(&wrong_version), Err(IopError::InvalidPayload));
    let mut reserved = valid;
    reserved[79] = 1;
    assert_eq!(NodeOperationV1::decode(&reserved), Err(IopError::InvalidPayload));

    let revoke_request = node_request(remote_id, OperationId::NodeRevokeTrust);
    let queued_capability = capabilities.grant(CapabilityType::ServiceCall, OperationId::NodeRevokeTrust.machine_id() as u64, 1, 0, caller, caller, Some(60), 0).unwrap();
    let queued = IopMessage::request(OperationId::NodeRevokeTrust, 111, caller, queued_capability, 60, 111, &revoke_request.encode()).unwrap();
    router.send(2, queued, &capabilities, 12).unwrap();
    capabilities.revoke(queued_capability).unwrap();
    assert_eq!(iop::dispatch_node_operation(&mut router, &capabilities, &mut local, OperationId::NodeRevokeTrust, 2, 1, SecurityIdentity([0x52; 16]), 13), Err(IopError::AccessDenied));
    assert_eq!(local.discovered_nodes()[0].unwrap().trust, TrustState::Trusted);

    dispatch_request(&mut router, &mut capabilities, &mut local, OperationId::NodeRevokeTrust, node_request(remote_id, OperationId::NodeRevokeTrust), 110, 13).unwrap();
    assert_eq!(local.discovered_nodes()[0].unwrap().trust, TrustState::Revoked);
}

// ------------------------=
// FUNC: iop_pairing_round_trip
// DESC: Sends pairing operations through bounded IOP queues and proves human approval is enforced by the service adapter.
// ------------------=
fn iop_pairing_round_trip() {
    use capability::{CapabilityManager, CapabilityType};
    use execution::SecurityIdentity;
    use iop::{IopMessage, IopRouter, NodeOperationV1, OperationId, NODE_OPERATION_HUMAN_APPROVED};

    let mut local = NodeRuntime::new();
    let mut remote = NodeRuntime::new();
    local.initialize(&[0x71; 32], true).unwrap();
    let remote_id = remote.initialize(&[0x72; 32], true).unwrap();
    local.discover(remote.advertise(1, 1, 10).unwrap(), 10).unwrap();
    let caller = SecurityIdentity([0x31; 16]);
    let service = SecurityIdentity([0x32; 16]);
    let issuer = SecurityIdentity([0x33; 16]);
    let mut capabilities = CapabilityManager::new();
    let pair_capability = capabilities.grant(CapabilityType::ServiceCall, OperationId::NodePairBegin.machine_id() as u64, 1, 0, issuer, caller, Some(100), 0).unwrap();
    let confirm_capability = capabilities.grant(CapabilityType::ServiceCall, OperationId::NodePairConfirm.machine_id() as u64, 1, 0, issuer, caller, Some(100), 0).unwrap();
    let mut router = IopRouter::new();
    router.register_endpoint(1, caller).unwrap();
    router.register_endpoint(2, service).unwrap();

    let begin = NodeOperationV1 { node_id: remote_id.0, handle: 0, scope: 0, lease_deadline: 0, operation: OperationId::NodePairBegin.machine_id(), rights: 0, value: 0, flags: 0, schema_version: 1 };
    let message = IopMessage::request(OperationId::NodePairBegin, 1, caller, pair_capability, 90, 44, &begin.encode()).unwrap();
    router.send(2, message, &capabilities, 11).unwrap();
    iop::dispatch_node_operation(&mut router, &capabilities, &mut local, OperationId::NodePairBegin, 2, 1, service, 11).unwrap();
    let begin_result = NodeOperationV1::decode(router.receive(1, 11).unwrap().bytes()).unwrap();

    let denied = NodeOperationV1 { node_id: remote_id.0, handle: begin_result.handle, scope: 0, lease_deadline: 0, operation: OperationId::NodePairConfirm.machine_id(), rights: 0, value: begin_result.value, flags: 0, schema_version: 1 };
    let denied_message = IopMessage::request(OperationId::NodePairConfirm, 2, caller, confirm_capability, 90, 45, &denied.encode()).unwrap();
    router.send(2, denied_message, &capabilities, 12).unwrap();
    assert_eq!(iop::dispatch_node_operation(&mut router, &capabilities, &mut local, OperationId::NodePairConfirm, 2, 1, service, 12), Err(iop::IopError::AccessDenied));
    assert_eq!(local.discovered_nodes()[0].unwrap().trust, TrustState::PairingPending);

    let approved = NodeOperationV1 { flags: NODE_OPERATION_HUMAN_APPROVED, ..denied };
    let approved_message = IopMessage::request(OperationId::NodePairConfirm, 3, caller, confirm_capability, 90, 46, &approved.encode()).unwrap();
    router.send(2, approved_message, &capabilities, 13).unwrap();
    iop::dispatch_node_operation(&mut router, &capabilities, &mut local, OperationId::NodePairConfirm, 2, 1, service, 13).unwrap();
    assert_eq!(local.discovered_nodes()[0].unwrap().trust, TrustState::Trusted);
}

// ------------------------=
// FUNC: node_event_delivery_after_commit
// DESC: Proves a committed node state change can be published, correlated, delivered, and suppressed after subscription revocation.
// ------------------=
fn node_event_delivery_after_commit() {
    use capability::{CapabilityManager, CapabilityType};
    use event::{EventClass, EventFabric, EventFilter, OverflowPolicy, RoutingDomain};
    use execution::SecurityIdentity;

    const NODE_PAIRED: u32 = 0x9e003;
    let source = SecurityIdentity([0x41; 16]);
    let observer = SecurityIdentity([0x42; 16]);
    let mut capabilities = CapabilityManager::new();
    let publish = capabilities
        .grant(CapabilityType::EventPublish, NODE_PAIRED as u64, 1, 0, source, source, Some(100), 0)
        .unwrap();
    let subscribe = capabilities
        .grant(CapabilityType::EventSubscribe, NODE_PAIRED as u64, 1, 0, source, observer, Some(100), 0)
        .unwrap();
    let mut fabric = EventFabric::new();
    let lease = fabric
        .subscribe(observer, subscribe, EventFilter { type_id: NODE_PAIRED, scope: None }, 100, OverflowPolicy::DropOldest, 2, &capabilities, 1)
        .unwrap();

    let committed_node = [0x73; 16];
    fabric
        .publish(EventClass::Record, RoutingDomain::Mesh, NODE_PAIRED, source, 0, 77, 76, &committed_node, 220, 2, &capabilities, publish)
        .unwrap();
    let delivered = fabric.receive(lease, 2).unwrap();
    assert_eq!(delivered.correlation_id, 77);
    assert_eq!(delivered.causation_id, 76);
    assert_eq!(&delivered.payload[..delivered.payload_len as usize], &committed_node);

    capabilities.revoke(subscribe).unwrap();
    fabric
        .publish(EventClass::Record, RoutingDomain::Mesh, NODE_PAIRED, source, 0, 78, 77, &committed_node, 220, 3, &capabilities, publish)
        .unwrap();
    assert_eq!(fabric.receive(lease, 3), Err(event::EventError::QueueFull));
}

// ------------------------=
// FUNC: pairing_lifecycle_failures
// DESC: Verifies cancellation and expiry cannot create trust even with the correct verification code.
// ------------------=
fn pairing_lifecycle_failures() {
    let mut local = NodeRuntime::new();
    let mut remote = NodeRuntime::new();
    local.initialize(&[0x61; 32], true).unwrap();
    let remote_id = remote.initialize(&[0x62; 32], true).unwrap();
    local.discover(remote.advertise(1, 1, 1).unwrap(), 1).unwrap();

    let cancelled = local.begin_pairing(remote_id, 2).unwrap();
    local.cancel_pairing(cancelled.id).unwrap();
    assert_eq!(local.confirm_pairing(cancelled.id, cancelled.verification_code, true, 3, 1), Err(NodeError::PairingNotFound));
    assert_ne!(local.discovered_nodes()[0].unwrap().trust, TrustState::Trusted);

    let expired = local.begin_pairing(remote_id, 4).unwrap();
    assert_eq!(local.confirm_pairing(expired.id, expired.verification_code, true, expired.expires_at, 2), Err(NodeError::PairingExpired));
    assert_ne!(local.discovered_nodes()[0].unwrap().trust, TrustState::Trusted);
}

// ------------------------=
// FUNC: paired_nodes
// DESC: Creates two independent nodes and completes explicit code-confirmed trust for the behavioral harness.
// ------------------=
fn paired_nodes() -> (NodeRuntime, NodeRuntime, node::types::NodeId, node::types::NodeId) {
    let mut a = NodeRuntime::new();
    let mut b = NodeRuntime::new();
    let a_id = a.initialize(&[0x11; 32], true).unwrap();
    let b_id = b.initialize(&[0x22; 32], true).unwrap();
    assert_ne!(a_id, b_id);
    a.discover(b.advertise(0x3, 1, 10).unwrap(), 10).unwrap();
    b.discover(a.advertise(0x5, 1, 10).unwrap(), 10).unwrap();
    assert_eq!(a.discovered_nodes()[0].unwrap().trust, TrustState::Untrusted);
    let pairing = a.begin_pairing(b_id, 11).unwrap();
    assert_eq!(a.confirm_pairing(pairing.id, pairing.verification_code, false, 12, 7), Err(NodeError::HumanApprovalRequired));
    assert_eq!(a.discovered_nodes()[0].unwrap().trust, TrustState::PairingPending);
    assert_eq!(a.confirm_pairing(pairing.id, pairing.verification_code + 1, true, 12, 7), Err(NodeError::VerificationMismatch));
    a.confirm_pairing(pairing.id, pairing.verification_code, true, 12, 7).unwrap();
    let pairing = b.begin_pairing(a_id, 11).unwrap();
    b.confirm_pairing(pairing.id, pairing.verification_code, true, 12, 7).unwrap();
    (a, b, a_id, b_id)
}

// ------------------------=
// FUNC: main
// DESC: Exercises node identity, discovery, trust, secure sessions, remote authority, membership, persistence, and bounds through behavior.
// ------------------=
fn main() {
    pairing_acceptance::run();
    iop_pairing_round_trip();
    iop_node_management_operations();
    node_event_delivery_after_commit();
    pairing_lifecycle_failures();
    assert_eq!(NodeRuntime::new().initialize(&[0; 32], false), Err(NodeError::EntropyUnavailable));
    let (mut a, mut b, a_id, b_id) = paired_nodes();

    let secret_a = [0x31; 32];
    let secret_b = [0x42; 32];
    let (_, public_a) = crypto::NodeCrypto::agreement_keypair(&secret_a);
    let (_, public_b) = crypto::NodeCrypto::agreement_keypair(&secret_b);
    let session_a = a.open_session(b_id, &secret_a, &public_b, b"iop-v1/a-b", 20, 8).unwrap();
    let session_b = b.open_session(a_id, &secret_b, &public_a, b"iop-v1/a-b", 20, 8).unwrap();
    assert_eq!(session_a, session_b);
    let aad = b"typed-operation";
    let mut outgoing = *b"hello remote node";
    let (sequence, tag) = a.protect(session_a, aad, &mut outgoing, 21).unwrap();
    let ciphertext = outgoing;
    b.unprotect(session_b, sequence, aad, &mut outgoing, &tag, 21).unwrap();
    assert_eq!(&outgoing, b"hello remote node");
    let mut replay = ciphertext;
    assert_eq!(b.unprotect(session_b, sequence, aad, &mut replay, &tag, 21), Err(NodeError::ReplayDetected));

    let grant = a.grant_remote(b_id, 0x3002, 0x77, 1, 50, 22, 9).unwrap();
    assert!(a.authorize_remote(grant, b_id, 0x3002, 0x77, 1, 23).is_ok());
    assert_eq!(a.authorize_remote(grant, b_id, 0x3002, 0x78, 1, 23), Err(NodeError::CapabilityDenied));
    a.revoke_remote(grant, 24, 9).unwrap();
    assert_eq!(a.authorize_remote(grant, b_id, 0x3002, 0x77, 1, 24), Err(NodeError::CapabilityRevoked));

    a.join_mesh(b_id, MeshRole::Member, 25, 10).unwrap();
    assert_eq!(a.mesh_members().iter().flatten().count(), 1);
    a.leave_mesh(b_id, 26, 10).unwrap();
    assert_eq!(a.discovered_nodes()[0].unwrap().trust, TrustState::Trusted);

    let encoded = a.encode_state().unwrap();
    let mut restored = NodeRuntime::new();
    assert_eq!(restored.restore_state(&encoded).unwrap(), a_id);
    let mut corrupt = encoded;
    corrupt[140] ^= 0x80;
    assert_eq!(NodeRuntime::new().restore_state(&corrupt), Err(NodeError::StateCorrupt));

    restored.revoke_trust(b_id, 30, 11).unwrap();
    assert_eq!(restored.discovered_nodes()[0].unwrap().trust, TrustState::Revoked);
    assert_eq!(restored.open_session(b_id, &secret_a, &public_b, b"iop-v1/a-b", 31, 12), Err(NodeError::NotTrusted));

    let mut bounded = NodeRuntime::new();
    bounded.initialize(&[0x55; 32], true).unwrap();
    for index in 0..32u64 {
        let mut peer = NodeRuntime::new();
        let mut entropy = [0u8; 32];
        entropy[..8].copy_from_slice(&(index + 100).to_le_bytes());
        peer.initialize(&entropy, true).unwrap();
        let result = bounded.discover(peer.advertise(0, 1, 1).unwrap(), 1);
        if index < node::types::MAX_DISCOVERED_NODES as u64 { assert!(result.is_ok()); }
        else { assert_eq!(result, Err(NodeError::ResourceLimit)); }
    }
}
