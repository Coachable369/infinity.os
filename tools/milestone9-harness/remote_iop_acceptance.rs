use super::{advance, Fixture};
use crate::{
    capability::CapabilityType,
    iop::{remote::RemoteError, NodeOperationV1, OperationId},
    node::types::PolicyDecision,
};

// ------------------------=
// FUNC: request
// DESC: Creates a caller-authorized typed remote request without injecting protocol messages or peer runtime state.
// ------------------=
fn request(
    a: &mut Fixture,
    peer: crate::node::types::NodeId,
    grant: u64,
    op: OperationId,
    value: u32,
    now: u64,
) -> u64 {
    let capability = a
        .capabilities
        .grant(
            CapabilityType::ServiceCall,
            op.machine_id() as u64,
            1,
            0,
            a.owner,
            a.owner,
            Some(now + 100),
            0,
        )
        .unwrap();
    let mut payload = crate::node_request(a.nodes.local_id().unwrap(), op);
    payload.value = value;
    a.iop
        .request_remote_node(
            &a.capabilities,
            &a.nodes,
            a.owner,
            capability,
            peer,
            grant,
            payload,
            0xabc,
            0xdef,
            now,
            now + 30,
        )
        .unwrap()
}
// ------------------------=
// FUNC: result
// DESC: Consumes one correlated caller-owned completion and verifies causation survives the wire.
// ------------------=
fn result(a: &mut Fixture, id: u64) -> Result<NodeOperationV1, RemoteError> {
    let result = a.iop.remote.take_result(a.owner, id).unwrap();
    assert_eq!(result.correlation_id, 0xabc);
    assert_eq!(result.causation_id, id);
    assert_eq!(result.request_id, id);
    assert!(a.iop.remote.take_result(a.owner, id).is_none());
    result.result
}
// ------------------------=
// FUNC: run
// DESC: Verifies actual-frame remote reads, narrow grants, policy behavior and post-dequeue revocation on an already authenticated duplex fixture.
// ------------------=
pub fn run(a: &mut Fixture, b: &mut Fixture, now: &mut u64) {
    let aid = a.nodes.local_id().unwrap();
    let bid = b.nodes.local_id().unwrap();
    for (f, peer) in [(&mut *a, bid), (&mut *b, aid)] {
        let mut policy = f
            .nodes
            .discovered_nodes()
            .iter()
            .flatten()
            .find(|p| p.id == peer)
            .unwrap()
            .policy;
        policy.categories[0] = PolicyDecision::Allow;
        policy.categories[1] = PolicyDecision::Allow;
        f.nodes.update_policy(peer, policy, *now, 0).unwrap();
    }
    inspect_pages(a, b, now, OperationId::NodeInspect, crate::node::inspection::NODE_DETAIL_BYTES);
    inspect_pages(a, b, now, OperationId::NodeSessionInspect, crate::node::inspection::SESSION_DETAIL_BYTES);
    let ar = b
        .nodes
        .grant_remote(
            aid,
            OperationId::NodeTrustRead.machine_id(),
            0,
            1,
            *now + 300,
            *now,
            0,
        )
        .unwrap();
    let br = a
        .nodes
        .grant_remote(
            bid,
            OperationId::NodeTrustRead.machine_id(),
            0,
            1,
            *now + 300,
            *now,
            0,
        )
        .unwrap();
    let id = request(a, bid, ar, OperationId::NodeTrustRead, 0, *now);
    advance(a, b, now, 8);
    assert_eq!(result(a, id).unwrap().value, 3);
    let id = request(b, aid, br, OperationId::NodeTrustRead, 0, *now);
    advance(a, b, now, 8);
    assert_eq!(result(b, id).unwrap().value, 3);
    let id = request(a, bid, ar, OperationId::NodePolicyUpdate, 0, *now);
    advance(a, b, now, 8);
    assert_eq!(result(a, id), Err(RemoteError::CapabilityScopeDenied));
    let mutation = b
        .nodes
        .grant_remote(
            aid,
            OperationId::NodePolicyUpdate.machine_id(),
            0,
            1,
            *now + 300,
            *now,
            0,
        )
        .unwrap();
    b.iop.remote.set_mutation_service_ready(false);
    b.execute_remote = false;
    let id = request(a, bid, mutation, OperationId::NodePolicyUpdate, 0, *now);
    advance(a, b, now, 5);
    b.iop.execute_remote_node(&mut b.nodes, *now);
    b.execute_remote = true;
    advance(a, b, now, 8);
    assert_eq!(result(a, id), Err(RemoteError::ServiceUnavailable));
    b.iop.remote.set_mutation_service_ready(true);
    b.execute_remote = false;
    let id = request(a, bid, mutation, OperationId::NodePolicyUpdate, 0, *now);
    advance(a, b, now, 5);
    assert_eq!(b.iop.remote.incoming_count(), 1);
    let before = b.iop.remote.executed;
    b.nodes.revoke_remote(mutation, *now, 0).unwrap();
    b.execute_remote = true;
    advance(a, b, now, 8);
    assert_eq!(result(a, id), Err(RemoteError::CapabilityRevoked));
    assert_eq!(b.iop.remote.executed, before);
    assert_eq!(
        b.nodes
            .discovered_nodes()
            .iter()
            .flatten()
            .find(|p| p.id == aid)
            .unwrap()
            .policy
            .categories[0],
        PolicyDecision::Allow
    );
    let mutation = b
        .nodes
        .grant_remote(
            aid,
            OperationId::NodePolicyUpdate.machine_id(),
            0,
            1,
            *now + 300,
            *now,
            0,
        )
        .unwrap();
    let id = request(a, bid, mutation, OperationId::NodePolicyUpdate, 0, *now);
    advance(a, b, now, 8);
    assert!(result(a, id).is_ok());
    let id = request(a, bid, ar, OperationId::NodeTrustRead, 0, *now);
    advance(a, b, now, 8);
    assert_eq!(result(a, id), Err(RemoteError::PolicyDenied));
    let id = request(a, bid, mutation, OperationId::NodePolicyUpdate, 1, *now);
    advance(a, b, now, 8);
    assert!(result(a, id).is_ok());
    let id = request(a, bid, ar, OperationId::NodeTrustRead, 0, *now);
    advance(a, b, now, 8);
    assert!(result(a, id).is_ok());
    let id = request(a, bid, u64::MAX, OperationId::NodeTrustRead, 0, *now);
    advance(a, b, now, 8);
    assert_eq!(result(a, id), Err(RemoteError::CapabilityRequired));
    let expired = b
        .nodes
        .grant_remote(
            aid,
            OperationId::NodeTrustRead.machine_id(),
            0,
            1,
            *now + 1,
            *now,
            0,
        )
        .unwrap();
    advance(a, b, now, 2);
    let id = request(a, bid, expired, OperationId::NodeTrustRead, 0, *now);
    advance(a, b, now, 8);
    assert_eq!(result(a, id), Err(RemoteError::CapabilityExpired));
    // Malformed and unknown-response frames are still authenticated by the actual
    // session service and carried through production Ethernet/UDP before rejection.
    let mut frame = [0u8; 129];
    frame[..4].copy_from_slice(b"IOP9");
    frame[4] = 1;
    frame[5] = 2;
    frame[8..16].copy_from_slice(&99999u64.to_le_bytes());
    frame[40..44].copy_from_slice(&10u32.to_le_bytes());
    frame[48..128].copy_from_slice(&crate::node_request(aid, OperationId::NodeTrustRead).encode());
    for mode in 0..5 {
        let mut bad = frame;
        let length = match mode {
            0 => 127,
            1 => 129,
            _ => 128,
        };
        if mode == 2 {
            bad[4] = 2;
        }
        if mode == 3 {
            bad[120] = 2;
        }
        let before = b.iop.remote.rejected;
        let executed = b.iop.remote.executed;
        a.transport
            .trust
            .send_data(&mut a.nodes, bid, &bad[..length], false, *now)
            .unwrap();
        advance(a, b, now, 4);
        assert!(b.iop.remote.rejected > before);
        assert_eq!(b.iop.remote.executed, executed);
    }
    // Caller abandonment and peer non-execution cannot grow request tables or
    // keep authority alive beyond the receiver's bounded deadline.
    b.execute_remote = false;
    let mut ids = [0; 8];
    for id in &mut ids {
        *id = request(a, bid, ar, OperationId::NodeTrustRead, 0, *now);
    }
    let cap = a
        .capabilities
        .grant(
            CapabilityType::ServiceCall,
            OperationId::NodeTrustRead.machine_id() as u64,
            1,
            0,
            a.owner,
            a.owner,
            Some(*now + 100),
            0,
        )
        .unwrap();
    assert_eq!(
        a.iop.request_remote_node(
            &a.capabilities,
            &a.nodes,
            a.owner,
            cap,
            bid,
            ar,
            crate::node_request(aid, OperationId::NodeTrustRead),
            1,
            1,
            *now,
            *now + 30
        ),
        Err(RemoteError::QueueFull)
    );
    let before = b.iop.remote.executed;
    advance(a, b, now, 40);
    for id in ids {
        assert_eq!(result(a, id), Err(RemoteError::DeadlineExceeded));
    }
    b.execute_remote = true;
    advance(a, b, now, 8);
    assert_eq!(b.iop.remote.executed, before);
}

// ------------------------=
// FUNC: inspect_pages
// DESC: Reconstructs a full remote selected record over encrypted Ethernet frames while discovery and session counters continue advancing.
// ------------------=
fn inspect_pages(a: &mut Fixture, b: &mut Fixture, now: &mut u64, operation: OperationId, length: usize) {
    let aid = a.nodes.local_id().unwrap(); let bid = b.nodes.local_id().unwrap();
    let grant = b.nodes.grant_remote(aid, operation.machine_id(), 0, 1, *now + 300, *now, 0).unwrap();
    let cap = a.capabilities.grant(CapabilityType::ServiceCall, operation.machine_id() as u64, 1, 0, a.owner, a.owner, Some(*now + 300), 0).unwrap();
    let mut input = crate::node_request(aid, operation);
    let mut bytes = [0; 128];
    for offset in (0..length).step_by(crate::node::inspection::PAGE_BYTES) {
        input.flags = offset as u32;
        let id = a.iop.request_remote_node(&a.capabilities, &a.nodes, a.owner, cap, bid, grant, input, 0xabc, 0xdef, *now, *now + 30).unwrap();
        advance(a, b, now, 8);
        let response = result(a, id).unwrap();
        assert_eq!(response.node_id, aid.0); assert_eq!(response.flags >> 16, length as u32); assert_eq!(response.flags & 0xffff, offset as u32);
        if offset != 0 { assert_eq!(response.handle, input.handle); }
        input.handle = response.handle;
        let count = 24.min(length - offset);
        bytes[offset..offset + count].copy_from_slice(&crate::node::inspection::page_data(response)[..count]);
    }
    assert_eq!(&bytes[..32], &aid.0);
    if operation == OperationId::NodeInspect { assert_eq!(&bytes[32..64], &a.nodes.advertise(1, 1, *now).unwrap().public_key); }
    else {
        let session = b.nodes.sessions().iter().flatten().find(|s| s.peer == aid && s.state == crate::node::types::SessionState::Established).unwrap();
        assert_eq!(&bytes[40..56], &session.protocol_reference); assert_eq!(bytes[56], 1);
        assert!(u64::from_le_bytes(bytes[72..80].try_into().unwrap()) < session.receive_sequence);
    }
    a.capabilities.retire_leaf(cap, a.owner).unwrap();
    b.nodes.revoke_remote(grant, *now, 0).unwrap();
}
