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
