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
}

pub struct OperatorRequests {
    pending: [Option<Pending>; 8],
    pub last_submitted: u64,
    pub last_completion: Option<RemoteResult>,
}
impl OperatorRequests {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates a fixed operator request table and empty read-only diagnostic observations.
    // ------------------=
    pub const fn new() -> Self {
        Self { pending: [None; 8], last_submitted: 0, last_completion: None }
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
            runtime.iop.remote.discard(caller, pending.request);
            let _ = runtime.capabilities.retire_leaf(pending.capability, issuer);
            runtime.node_operator.pending[index] = None;
        }
    }
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
            correlation, correlation, now, deadline));
        match result {
            Ok(request) => {
                runtime.node_operator.pending[index] = Some(Pending { user, session, request, capability: cap, retain_until: deadline.saturating_add(300) });
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
        let caller = runtime.service_identity(SERVICE_CONSOLE).ok_or(RemoteError::AccessDenied)?;
        let issuer = runtime.service_identity(SERVICE_NODE_TRUST).ok_or(RemoteError::AccessDenied)?;
        let Some(result) = runtime.iop.remote.take_result(caller, request) else { return Ok(None); };
        if let Some(pending) = runtime.node_operator.pending[index].take() {
            let _ = runtime.capabilities.retire_leaf(pending.capability, issuer);
        }
        runtime.node_operator.last_completion = Some(result);
        Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
