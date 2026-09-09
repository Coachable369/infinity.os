//! Authenticated human remote requests over the ordinary native IOP router.
//! Eight owned slots; no host networking, injected results, or ambient authority.
use super::*;
use identity::{StableId, SessionState, MAX_SESSIONS, SESSION_IDENTITY_MANAGE};
use iop::{NodeOperationV1, OperationId, remote::{RemoteError, RemoteResult}};

#[derive(Clone, Copy)]
struct Pending {
    user: StableId,
    session: StableId,
    request: u64,
    capability: u64,
    retain_until: u64,
    wire_request: u64,
    peer: node::types::NodeId,
    grant: u64,
    payload: NodeOperationV1,
    deadline: u64,
    correlation: u64,
    result: Option<RemoteResult>,
    detail: [u8; 128],
    detail_length: usize,
    offset: usize,
    token: u64,
}

pub struct OperatorRequests {
    pending: [Option<Pending>; 8],
    pub last_submitted: u64,
    pub last_completion: Option<RemoteResult>,
    pub last_detail: [u8; 128],
    pub last_detail_length: usize,
}
impl OperatorRequests {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates a fixed operator request table and empty read-only diagnostic observations.
    // ------------------=
    pub const fn new() -> Self {
        Self { pending: [None; 8], last_submitted: 0, last_completion: None,
            last_detail: [0; 128], last_detail_length: 0 }
    }
}

// ------------------------=
// FUNC: authorized
// DESC: Binds remote operator activity to the complete active authenticated user and session identity.
// ------------------=
fn authorized(runtime: &InfinityRuntime, user: StableId, session: StableId) -> bool {
    (0..MAX_SESSIONS).filter_map(|index| runtime.identity.session_nth(index)).any(|candidate|
        candidate.id == session && candidate.user == user && candidate.state == SessionState::Active
        && candidate.capabilities & SESSION_IDENTITY_MANAGE != 0)
}

// ------------------------=
// FUNC: prune
// DESC: Retires local request authority after session loss or bounded result retention; never extends a transport deadline.
// ------------------=
pub(super) fn prune(runtime: &mut InfinityRuntime, now: u64) {
    let Some(caller) = runtime.service_identity(SERVICE_CONSOLE) else { return; };
    let Some(issuer) = runtime.service_identity(SERVICE_NODE_TRUST) else { return; };
    for index in 0..8 {
        let Some(pending) = runtime.node_operator.pending[index] else { continue; };
        if now >= pending.retain_until || !authorized(runtime, pending.user, pending.session) {
            runtime.iop.remote.discard(caller, pending.wire_request);
            let _ = runtime.capabilities.retire_leaf(pending.capability, issuer);
            runtime.node_operator.pending[index] = None;
        } else {
            advance(runtime, index, now);
        }
    }
}

// ------------------------=
// FUNC: advance
// DESC: Collects real wire completions and assembles exact pinned inspection pages under the original capability, owner and deadline.
// ------------------=
fn advance(runtime: &mut InfinityRuntime, index: usize, now: u64) {
    let Some(mut pending) = runtime.node_operator.pending[index] else { return; };
    if pending.result.is_some() { return; }
    let Some(caller) = runtime.service_identity(SERVICE_CONSOLE) else { return; };
    let Some(mut result) = runtime.iop.remote.take_result(caller, pending.wire_request) else { return; };
    let inspection = pending.payload.operation == OperationId::NodeInspect.machine_id()
        || pending.payload.operation == OperationId::NodeDomainInspect.machine_id();
    if inspection {
        if let Ok(page) = result.result {
            match append_page(&mut pending, page) {
                Ok(false) => {
                    let mut next = pending.payload;
                    next.handle = pending.token;
                    next.flags = pending.offset as u32;
                    match runtime.iop.request_remote_node(&runtime.capabilities, &runtime.nodes,
                        caller, pending.capability, pending.peer, pending.grant, next,
                        pending.correlation, result.request_id, now, pending.deadline) {
                        Ok(request) => {
                            pending.wire_request = request;
                            runtime.node_operator.pending[index] = Some(pending);
                            return;
                        }
                        Err(error) => result.result = Err(error),
                    }
                }
                Ok(true) => {}
                Err(error) => result.result = Err(error),
            }
        }
    }
    result.request_id = pending.request;
    result.correlation_id = pending.correlation;
    pending.result = Some(result);
    runtime.node_operator.pending[index] = Some(pending);
}

// ------------------------=
// FUNC: append_page
// DESC: Rejects mismatched object, operation, size, offset or snapshot identity before copying a bounded public inspection page.
// ------------------=
fn append_page(pending: &mut Pending, page: NodeOperationV1) -> Result<bool, RemoteError> {
    use node::inspection::{page_data, PAGE_BYTES, NODE_DETAIL_BYTES, DOMAIN_DETAIL_BYTES};
    let expected = if pending.payload.operation == OperationId::NodeInspect.machine_id() {
        NODE_DETAIL_BYTES
    } else { DOMAIN_DETAIL_BYTES };
    if page.node_id != pending.payload.node_id || page.operation != pending.payload.operation
        || page.schema_version != 1 || (page.flags >> 16) as usize != expected
        || (page.flags & 0xffff) as usize != pending.offset || page.handle == 0
        || (pending.offset != 0 && page.handle != pending.token)
        || pending.offset >= expected || expected > pending.detail.len()
    { return Err(RemoteError::MalformedRequest); }
    pending.token = page.handle;
    let count = PAGE_BYTES.min(expected - pending.offset);
    pending.detail[pending.offset..pending.offset + count].copy_from_slice(&page_data(page)[..count]);
    pending.offset += count;
    if pending.offset == expected { pending.detail_length = expected; }
    Ok(pending.offset == expected)
}

// ------------------------=
// FUNC: submit
// DESC: Queues one exact-operation request owned by an authenticated operator, with a 30-second deadline and existing remote peer grant enforcement.
// ------------------=
pub fn submit(user: StableId, session: StableId, peer: node::types::NodeId, grant: u64, payload: NodeOperationV1) -> Result<u64, RemoteError> {
    with_runtime(|runtime| submit_to(runtime, user, session, peer, grant, payload)).ok_or(RemoteError::ServiceUnavailable)?
}

// ------------------------=
// FUNC: submit_to
// DESC: Applies identical operator admission to one exclusively borrowed native runtime, including behavioral fixtures.
// ------------------=
pub(super) fn submit_to(runtime: &mut InfinityRuntime, user: StableId, session: StableId, peer: node::types::NodeId, grant: u64, mut payload: NodeOperationV1) -> Result<u64, RemoteError> {
        if !authorized(runtime, user, session) { return Err(RemoteError::AccessDenied); }
        let now = runtime.node_clock.ok_or(RemoteError::ServiceUnavailable)?;
        prune(runtime, now);
        let index = runtime.node_operator.pending.iter().position(Option::is_none).ok_or(RemoteError::QueueFull)?;
        let local = runtime.nodes.local_id().ok_or(RemoteError::ServiceUnavailable)?;
        payload.node_id = if payload.operation == OperationId::NodeDomainInspect.machine_id() {
            node::membership::domain_id(local, peer).0
        } else { local.0 };
        payload.rights = 1;
        let caller = runtime.service_identity(SERVICE_CONSOLE).ok_or(RemoteError::AccessDenied)?;
        let issuer = runtime.service_identity(SERVICE_NODE_TRUST).ok_or(RemoteError::AccessDenied)?;
        let deadline = now.checked_add(30).ok_or(RemoteError::DeadlineExceeded)?;
        let cap = runtime.capabilities.grant(CapabilityType::ServiceCall, payload.operation as u64,
            1, 0, issuer, caller, Some(deadline), 0).map_err(|_| RemoteError::QueueFull)?;
        let correlation = runtime.iop.next_node_request().map_err(|_| RemoteError::QueueFull);
        let result = correlation.and_then(|correlation| runtime.iop.request_remote_node(
            &runtime.capabilities, &runtime.nodes, caller, cap, peer, grant, payload,
            correlation, correlation, now, deadline).map(|request| (request, correlation)));
        match result {
            Ok((request, correlation)) => {
                runtime.node_operator.pending[index] = Some(Pending { user, session, request,
                    capability: cap, retain_until: deadline.saturating_add(300), wire_request: request,
                    peer, grant, payload, deadline, correlation, result: None, detail: [0; 128],
                    detail_length: 0, offset: 0, token: 0 });
                runtime.node_operator.last_submitted = request;
                Ok(request)
            }
            Err(error) => { let _ = runtime.capabilities.retire_leaf(cap, issuer); Err(error) }
        }
}

// ------------------------=
// FUNC: take_result
// DESC: Returns only the submitting operator's actual typed completion and releases its local capability and bounded slot.
// ------------------=
pub fn take_result(user: StableId, session: StableId, request: u64) -> Result<Option<RemoteResult>, RemoteError> {
    with_runtime(|runtime| take_result_from(runtime, user, session, request)).ok_or(RemoteError::ServiceUnavailable)?
}

// ------------------------=
// FUNC: take_result_from
// DESC: Collects the same caller-owned result from one exclusively borrowed native runtime.
// ------------------=
pub(super) fn take_result_from(runtime: &mut InfinityRuntime, user: StableId, session: StableId, request: u64) -> Result<Option<RemoteResult>, RemoteError> {
        if !authorized(runtime, user, session) { return Err(RemoteError::AccessDenied); }
        let index = runtime.node_operator.pending.iter().position(|entry| entry.map(|p|
            p.user == user && p.session == session && p.request == request).unwrap_or(false))
            .ok_or(RemoteError::NotFound)?;
        let issuer = runtime.service_identity(SERVICE_NODE_TRUST).ok_or(RemoteError::AccessDenied)?;
        advance(runtime, index, runtime.node_clock.unwrap_or(0));
        let Some(result) = runtime.node_operator.pending[index].and_then(|pending| pending.result) else { return Ok(None); };
        runtime.node_operator.last_detail_length = 0;
        if let Some(pending) = runtime.node_operator.pending[index].take() {
            let _ = runtime.capabilities.retire_leaf(pending.capability, issuer);
            if result.result.is_ok() {
                runtime.node_operator.last_detail = pending.detail;
                runtime.node_operator.last_detail_length = pending.detail_length;
            }
        }
        runtime.node_operator.last_completion = Some(result);
        Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------=
    // FUNC: unavailable_storage_fixture
    // DESC: Marks the host service endpoint present without fabricating storage or wire results; this test exercises ownership and timeout only.
    // ------------------=
    fn unavailable_storage_fixture(_: iop::remote::AuthenticatedStorageRequest)
        -> Result<(iop::storage_protocol::StorageOperationV1, Option<iop::storage_protocol::StorageCommit>), RemoteError> {
        Err(RemoteError::ServiceUnavailable)
    }

    // ------------------------=
    // FUNC: storage_operator_owns_bounded_requests_and_retires_authority
    // DESC: Exercises the shared remote storage router with session-private collection, bounded admission, lock revocation and real deadline completions.
    // ------------------=
    #[test]
    fn storage_operator_owns_bounded_requests_and_retires_authority() {
        use super::super::storage_operator::{submit_to, take_from, prune};
        use iop::storage_protocol::{Operation, StorageOperationV1};
        let (mut runtime, user, a, b, peer) = operator_fixture();
        runtime.storage_handler = Some(unavailable_storage_fixture); runtime.start_all(10);
        let payload = StorageOperationV1 { operation: Operation::ResourceInspect, object: [0; 16],
            authority_generation: 0, manifest_generation: 0, object_version: 0,
            offset: 0, scope: 0, value: 0, length: 0, data: [0; 64] };
        let baseline = runtime.capabilities.count();
        assert_eq!(submit_to(&mut runtime, user, a, peer, 0, payload), Err(RemoteError::AccessDenied));
        let first = submit_to(&mut runtime, user, a, peer, 1, payload).unwrap();
        assert_eq!(take_from(&mut runtime, user, b, first), Err(RemoteError::NotFound));
        assert_eq!(take_from(&mut runtime, user, a, first), Ok(None));
        for _ in 1..8 { submit_to(&mut runtime, user, a, peer, 1, payload).unwrap(); }
        assert_eq!(submit_to(&mut runtime, user, a, peer, 1, payload), Err(RemoteError::QueueFull));
        assert_eq!(runtime.capabilities.count(), baseline + 8);
        runtime.identity.lock_session(a, user).unwrap(); prune(&mut runtime, 11);
        assert_eq!(runtime.capabilities.count(), baseline);
        assert_eq!(take_from(&mut runtime, user, a, first), Err(RemoteError::AccessDenied));
        let second = submit_to(&mut runtime, user, b, peer, 1, payload).unwrap();
        runtime.iop.poll_remote_node(&runtime.capabilities, &mut runtime.nodes, &mut runtime.node_transport.trust, 41);
        runtime.node_clock = Some(41);
        let result = take_from(&mut runtime, user, b, second).unwrap().unwrap();
        assert_eq!(result.request_id, second); assert_eq!(result.result, Err(RemoteError::DeadlineExceeded));
        assert_eq!(runtime.capabilities.count(), baseline);
        assert_eq!(take_from(&mut runtime, user, b, second), Err(RemoteError::NotFound));
    }

    // ------------------------=
    // FUNC: operator_fixture
    // DESC: Creates bootstrapped HOST services with independently authenticated sessions and an explicitly identified local session fixture, not installed acceptance.
    // ------------------=
    fn operator_fixture() -> (InfinityRuntime, StableId, StableId, StableId, node::types::NodeId) {
        let mut runtime = InfinityRuntime::new(false);
        runtime.define_bootstrap().unwrap(); runtime.start_all(0);
        runtime.identity.create_machine(b"operator-fixture", 64, 1, 0).unwrap();
        let user = runtime.identity.create_user(b"operator", b"Operator", 0).unwrap().id;
        runtime.identity.create_password(user, b"Fixture901", 0).unwrap();
        let a = runtime.identity.create_session(user, b"Fixture901", 1).unwrap().id;
        let b = runtime.identity.create_session(user, b"Fixture901", 1).unwrap().id;
        runtime.nodes.initialize(&[61; 32], true).unwrap();
        let mut other = node::NodeRuntime::new();
        let peer = other.initialize(&[62; 32], true).unwrap();
        runtime.nodes.discover(other.advertise(1, 1, 1).unwrap(), 1).unwrap();
        let pair = runtime.nodes.begin_pairing(peer, 2).unwrap();
        runtime.nodes.confirm_pairing(pair.id, pair.verification_code, true, 3, 3).unwrap();
        let (_, public) = crypto::NodeCrypto::agreement_keypair(&[64; 32]);
        runtime.nodes.open_session(peer, &[63; 32], &public, b"operator-host-fixture", 4, 4).unwrap();
        runtime.node_clock = Some(10);
        (runtime, user, a, b, peer)
    }

    // ------------------------=
    // FUNC: advertisement_subscriptions_stop_on_missing_backend_and_session_loss
    // DESC: Exercises real subscription ownership and bounded poll transitions without injecting network success or fabricated device inventory.
    // ------------------=
    #[test]
    fn advertisement_subscriptions_stop_on_missing_backend_and_session_loss() {
        use super::super::storage_advertiser::{start_from, poll};
        let (mut runtime, user, session, _, peer) = operator_fixture();
        assert_eq!(start_from(&mut runtime, user, session, peer, 0), Err(RemoteError::AccessDenied));
        start_from(&mut runtime, user, session, peer, 1).unwrap();
        assert_eq!(start_from(&mut runtime, user, session, peer, 1), Err(RemoteError::Conflict));
        poll(&mut runtime, 10);
        assert_eq!(runtime.storage_advertiser.last_error, Some(RemoteError::ServiceUnavailable));
        assert_eq!(runtime.storage_operator.last_submitted, 0);
        start_from(&mut runtime, user, session, peer, 1).unwrap();
        runtime.identity.lock_session(session, user).unwrap();
        for _ in 0..4 { poll(&mut runtime, 11); }
        assert_eq!(runtime.storage_advertiser.last_error, Some(RemoteError::AccessDenied));
        assert_eq!(runtime.storage_advertiser.completed, 0);
        assert_eq!(start_from(&mut runtime, user, session, peer, 1), Err(RemoteError::AccessDenied));
    }

    // ------------------------=
    // FUNC: operator_ownership_limits_and_revocation
    // DESC: Checks bounded operator requests, session-private results, local authority reclamation on lock, and actual router timeout results.
    // ------------------=
    #[test]
    fn operator_ownership_limits_and_revocation() {
        let (mut runtime, user, a, b, peer) = operator_fixture();
        let payload = node::reconciliation::request([0; 32], OperationId::NodeInspect);
        let baseline = runtime.capabilities.count();
        let first = submit_to(&mut runtime, user, a, peer, 1, payload).unwrap();
        assert_eq!(take_result_from(&mut runtime, user, b, first), Err(RemoteError::NotFound));
        assert_eq!(take_result_from(&mut runtime, user, a, first), Ok(None));
        for _ in 1..8 { submit_to(&mut runtime, user, a, peer, 1, payload).unwrap(); }
        assert_eq!(runtime.capabilities.count(), baseline + 8);
        assert_eq!(submit_to(&mut runtime, user, a, peer, 1, payload), Err(RemoteError::QueueFull));
        runtime.identity.lock_session(a, user).unwrap();
        prune(&mut runtime, 11);
        assert_eq!(runtime.capabilities.count(), baseline);
        assert!(runtime.node_operator.pending.iter().all(Option::is_none));
        assert_eq!(submit_to(&mut runtime, user, a, peer, 1, payload), Err(RemoteError::AccessDenied));
        let second = submit_to(&mut runtime, user, b, peer, 1, payload).unwrap();
        runtime.iop.poll_remote_node(&runtime.capabilities, &mut runtime.nodes, &mut runtime.node_transport.trust, 41);
        let result = take_result_from(&mut runtime, user, b, second).unwrap().unwrap();
        assert_eq!(result.request_id, second);
        assert_eq!(result.result, Err(RemoteError::DeadlineExceeded));
        assert_eq!(runtime.capabilities.count(), baseline);
        assert_eq!(take_result_from(&mut runtime, user, b, second), Err(RemoteError::NotFound));
        submit_to(&mut runtime, user, b, peer, 1, payload).unwrap();
        prune(&mut runtime, 340);
        assert_eq!(runtime.capabilities.count(), baseline);
    }

    // ------------------------=
    // FUNC: operator_inspection_pages_are_complete_and_pinned
    // DESC: Assembles real authoritative inspection bytes and rejects page identity, schema, size, offset and token substitution.
    // ------------------=
    #[test]
    fn operator_inspection_pages_are_complete_and_pinned() {
        let (mut runtime, user, session, _, peer) = operator_fixture();
        let payload = node::reconciliation::request(peer.0, OperationId::NodeInspect);
        submit_to(&mut runtime, user, session, peer, 1, payload).unwrap();
        let mut pending = runtime.node_operator.pending[0].unwrap();
        pending.payload = payload;
        let first = node::inspection::inspect(&runtime.nodes, OperationId::NodeInspect, payload).unwrap();
        for kind in 0..5 {
            let mut changed = first;
            match kind {
                0 => changed.node_id[0] ^= 1,
                1 => changed.operation = OperationId::NodeDomainInspect.machine_id(),
                2 => changed.schema_version = 2,
                3 => changed.flags ^= 1 << 16,
                _ => changed.flags |= 24,
            }
            let mut candidate = pending;
            assert_eq!(append_page(&mut candidate, changed), Err(RemoteError::MalformedRequest));
            assert_eq!(candidate.offset, 0);
            assert_eq!(candidate.detail, [0; 128]);
        }
        assert_eq!(append_page(&mut pending, first), Ok(false));
        let mut next = payload;
        next.handle = first.handle;
        next.flags = 24;
        let mut wrong = node::inspection::inspect(&runtime.nodes, OperationId::NodeInspect, next).unwrap();
        wrong.handle ^= 2;
        assert_eq!(append_page(&mut pending, wrong), Err(RemoteError::MalformedRequest));
        while pending.detail_length == 0 {
            next.flags = pending.offset as u32;
            let page = node::inspection::inspect(&runtime.nodes, OperationId::NodeInspect, next).unwrap();
            append_page(&mut pending, page).unwrap();
        }
        let mut expected = [0; 128];
        node::reconciliation::collect(payload, &mut expected, &mut |input|
            node::inspection::inspect(&runtime.nodes, OperationId::NodeInspect, input)).unwrap();
        assert_eq!(pending.detail_length, 128);
        assert_eq!(pending.detail, expected);
        assert_eq!(pending.deadline, 40);
        assert_eq!(pending.detail[..32], peer.0);
    }
}
