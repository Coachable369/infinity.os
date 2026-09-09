//! Bounded human-owned remote storage requests over the existing secure IOP
//! transport. No network work or response wait runs inside the UI submission.
use super::*;
use identity::{StableId, SessionState, MAX_SESSIONS, SESSION_IDENTITY_MANAGE};
use iop::{remote::{RemoteError, RemoteResult}, storage_protocol::StorageOperationV1};
use node::types::NodeId;

#[derive(Clone, Copy)]
struct Pending {
    user: StableId, session: StableId, id: u64, capability: u64, retain_until: u64,
    result: Option<RemoteResult<StorageOperationV1>>,
}
pub struct StorageRequests {
    pending: [Option<Pending>; 8],
    pub last_submitted: u64,
    pub last_completion: Option<RemoteResult<StorageOperationV1>>,
}
impl StorageRequests {
    // ------------------------=
    // FUNC: new
    // DESC: Creates exactly eight owned completion slots and empty non-secret diagnostic observations.
    // ------------------=
    pub const fn new() -> Self { Self { pending: [None; 8], last_submitted: 0, last_completion: None } }
}

// ------------------------=
// FUNC: authorized
// DESC: Revalidates the full current privileged operator session instead of retaining authority after lock or logout.
// ------------------=
fn authorized(runtime: &InfinityRuntime, user: StableId, session: StableId) -> bool {
    (0..MAX_SESSIONS).filter_map(|i| runtime.identity.session_nth(i)).any(|s|
        s.id == session && s.user == user && s.state == SessionState::Active
            && s.capabilities & SESSION_IDENTITY_MANAGE != 0)
}

// ------------------------=
// FUNC: prune
// DESC: Collects only actual authenticated router completions and retires requests after session loss or bounded retention; each tick visits at most eight slots.
// ------------------=
pub(super) fn prune(runtime: &mut InfinityRuntime, now: u64) {
    let Some(caller) = runtime.service_identity(SERVICE_SETTINGS) else { return; };
    let Some(issuer) = runtime.service_identity(SERVICE_REPLICA_STORAGE) else { return; };
    for index in 0..8 {
        let Some(mut pending) = runtime.storage_operator.pending[index] else { continue; };
        if now >= pending.retain_until || !authorized(runtime, pending.user, pending.session) {
            runtime.iop.remote.discard(caller, pending.id);
            let _ = runtime.capabilities.retire_leaf(pending.capability, issuer);
            runtime.storage_operator.pending[index] = None;
        } else if pending.result.is_none() {
            pending.result = runtime.iop.remote.take_storage_result(caller, pending.id);
            runtime.storage_operator.pending[index] = Some(pending);
        }
    }
}

// ------------------------=
// FUNC: submit
// DESC: Admits one exact storage operation under explicit peer-issued authority and returns immediately with its native request identifier.
// ------------------=
pub fn submit(user: StableId, session: StableId, peer: NodeId, grant: u64,
    payload: StorageOperationV1) -> Result<u64, RemoteError> {
    with_runtime(|r| submit_to(r, user, session, peer, grant, payload)).ok_or(RemoteError::ServiceUnavailable)?
}

// ------------------------=
// FUNC: submit_to
// DESC: Uses the same owned secure IOP queues and per-operation capability checks for production and behavioral tests, with a fixed thirty-second deadline.
// ------------------=
pub(super) fn submit_to(runtime: &mut InfinityRuntime, user: StableId, session: StableId,
    peer: NodeId, grant: u64, payload: StorageOperationV1) -> Result<u64, RemoteError> {
    if !authorized(runtime, user, session) { return Err(RemoteError::AccessDenied); }
    payload.encode().map_err(|_| RemoteError::MalformedRequest)?;
    if grant == 0 || peer.0 == [0; 32] { return Err(RemoteError::AccessDenied); }
    let now = runtime.node_clock.ok_or(RemoteError::ServiceUnavailable)?;
    prune(runtime, now);
    let slot = runtime.storage_operator.pending.iter().position(Option::is_none).ok_or(RemoteError::QueueFull)?;
    let caller = runtime.service_identity(SERVICE_SETTINGS).ok_or(RemoteError::ServiceUnavailable)?;
    let issuer = runtime.service_identity(SERVICE_REPLICA_STORAGE).ok_or(RemoteError::ServiceUnavailable)?;
    let deadline = now.checked_add(30).ok_or(RemoteError::DeadlineExceeded)?;
    let cap = runtime.capabilities.grant(CapabilityType::ServiceCall, payload.operation as u64,
        1, payload.scope, issuer, caller, Some(deadline), 0).map_err(|_| RemoteError::QueueFull)?;
    let submitted = runtime.iop.next_node_request().map_err(|_| RemoteError::QueueFull)
        .and_then(|correlation| runtime.iop.request_remote_storage(&runtime.capabilities, &runtime.nodes,
            caller, cap, peer, grant, payload, correlation, correlation, now, deadline));
    match submitted {
        Ok(id) => {
            runtime.storage_operator.pending[slot] = Some(Pending { user, session, id, capability: cap,
                retain_until: deadline.saturating_add(120), result: None });
            runtime.storage_operator.last_submitted = id;
            Ok(id)
        },
        Err(error) => { let _ = runtime.capabilities.retire_leaf(cap, issuer); Err(error) },
    }
}

// ------------------------=
// FUNC: take_result
// DESC: Retrieves a completed typed storage result only for its originating authenticated session; never parses Console output or synthesizes success.
// ------------------=
pub fn take_result(user: StableId, session: StableId, request: u64)
    -> Result<Option<RemoteResult<StorageOperationV1>>, RemoteError> {
    with_runtime(|r| take_from(r, user, session, request)).ok_or(RemoteError::ServiceUnavailable)?
}

// ------------------------=
// FUNC: take_from
// DESC: Releases completed request authority and updates diagnostic observations only after the owner's actual mailbox has completed.
// ------------------=
pub(super) fn take_from(runtime: &mut InfinityRuntime, user: StableId, session: StableId, request: u64)
    -> Result<Option<RemoteResult<StorageOperationV1>>, RemoteError> {
    if !authorized(runtime, user, session) { return Err(RemoteError::AccessDenied); }
    prune(runtime, runtime.node_clock.ok_or(RemoteError::ServiceUnavailable)?);
    let index = runtime.storage_operator.pending.iter().position(|p| p.is_some_and(|p|
        p.user == user && p.session == session && p.id == request)).ok_or(RemoteError::NotFound)?;
    let pending = runtime.storage_operator.pending[index].unwrap();
    let Some(result) = pending.result else { return Ok(None); };
    let issuer = runtime.service_identity(SERVICE_REPLICA_STORAGE).ok_or(RemoteError::ServiceUnavailable)?;
    let _ = runtime.capabilities.retire_leaf(pending.capability, issuer);
    runtime.storage_operator.pending[index] = None;
    runtime.storage_operator.last_completion = Some(result);
    runtime.storage_last_observation = result.result.ok();
    Ok(Some(result))
}
