//! Durable recipient retirement runs independently of object visibility and
//! requires a distinct explicitly granted delete operation on every retry.
use super::*;
use fabric::deletion::Deletion;
pub(super) struct Worker {
    cursor: usize,
    ticks: u8,
    next: u64,
    job: Option<(Deletion, usize)>,
    pending: Option<Pending>,
}
impl Worker {
    // ------------------------=
    // FUNC: new
    // DESC: Starts an empty bounded worker; the native outbox remains authoritative across restart.
    // ------------------=
    pub(super) const fn new() -> Self {
        Self {
            cursor: 0,
            ticks: 0,
            next: 0,
            job: None,
            pending: None,
        }
    }
}
// ------------------------=
// FUNC: retire_authority
// DESC: Persists explicit recipient-retirement authority without substituting a transfer or publication grant.
// ------------------=
pub fn retire_authority(
    user: StableId,
    session: StableId,
    peer: NodeId,
    grant: u64,
    lease: u64,
) -> Result<(), RemoteError> {
    let c = with_runtime(|r| {
        if !storage_operator::authorized(r, user, session)
            || grant == 0
            || !(1..=3600).contains(&lease)
        {
            return Err(RemoteError::AccessDenied);
        }
        if !r.storage_coordinator.loaded {
            return Err(RemoteError::ServiceUnavailable);
        }
        let now = r.node_clock.ok_or(RemoteError::ServiceUnavailable)?;
        let mut c = r.storage_coordinator.config;
        let i = c
            .peers
            .iter()
            .position(|p| p.is_some_and(|p| p.peer == peer))
            .or_else(|| c.peers.iter().position(Option::is_none))
            .ok_or(RemoteError::QueueFull)?;
        let mut p = c.peers[i].unwrap_or(Participation {
            peer,
            grants: [0; 5],
            expires: 0,
            advertise: 0,
            advertise_expires: 0,
            delete: 0,
            delete_expires: 0,
        });
        p.delete = grant;
        p.delete_expires = now.saturating_add(lease);
        c.peers[i] = Some(p);
        Ok(c)
    })
    .ok_or(RemoteError::ServiceUnavailable)??;
    configure(user, session, c)
}
// ------------------------=
// FUNC: acknowledge
// DESC: Removes a pending recipient obligation only after a successful authenticated retirement receipt or already-completed local deletion.
// ------------------=
fn acknowledge(
    r: &mut InfinityRuntime,
    d: Deletion,
    index: usize,
    now: u64,
) -> Result<(), RemoteError> {
    match native(
        r,
        NativeRequest::DeletionAck {
            object: d.object,
            owner: d.owner,
            scope: d.scope,
            generation: d.manifest_generation,
            placement: index,
        },
        now,
    )? {
        NativeReply::Committed => Ok(()),
        _ => Err(RemoteError::InvalidState),
    }
}
// ------------------------=
// FUNC: step
// DESC: Advances one native outbox query, exact remote retirement, or durable acknowledgment per scheduled tick.
// ------------------=
fn step(r: &mut InfinityRuntime, now: u64) -> Result<bool, RemoteError> {
    let caller = r
        .service_identity(SERVICE_REPLICA_STORAGE)
        .ok_or(RemoteError::ServiceUnavailable)?;
    if let Some(p) = r.storage_coordinator.deletion.pending {
        let done = r.iop.remote.take_storage_result(caller, p.request);
        if done.is_none() && now < p.deadline {
            return Ok(false);
        }
        r.iop.remote.discard(caller, p.request);
        let _ = r.capabilities.retire_leaf(p.capability, caller);
        r.storage_coordinator.deletion.pending = None;
        let response = done.ok_or(RemoteError::DeadlineExceeded)?.result?;
        let (d, index) = r
            .storage_coordinator
            .deletion
            .job
            .ok_or(RemoteError::InvalidState)?;
        if response.operation != Operation::ReplicaDelete
            || response.object != p.payload.object
            || response.object_version != p.payload.object_version
            || response.manifest_generation != p.payload.manifest_generation
            || response.value != 1
        {
            return Err(RemoteError::UnknownResponse);
        }
        acknowledge(r, d, index, now)?;
        r.storage_coordinator.deletion.job = None;
        return Ok(true);
    }
    if let Some((d, index)) = r.storage_coordinator.deletion.job {
        let placement = d.placements[index].ok_or(RemoteError::InvalidState)?;
        if Some(placement.node) == r.nodes.local_id() {
            acknowledge(r, d, index, now)?;
            r.storage_coordinator.deletion.job = None;
            return Ok(true);
        }
        let grant = grant(
            &r.storage_coordinator.config,
            placement.node,
            Operation::ReplicaDelete,
            now,
        )?;
        let p = StorageOperationV1 {
            operation: Operation::ReplicaDelete,
            object: d.object,
            authority_generation: d.authority_generation,
            manifest_generation: placement.admission_generation,
            object_version: placement.version,
            scope: d.scope,
            offset: 0,
            value: placement.admission_generation,
            length: 0,
            data: [0; 64],
        };
        let deadline = now.saturating_add(30);
        let cap = r
            .capabilities
            .grant(
                CapabilityType::ServiceCall,
                Operation::ReplicaDelete as u64,
                1,
                d.scope,
                caller,
                caller,
                Some(deadline),
                0,
            )
            .map_err(|_| RemoteError::QueueFull)?;
        let result = r
            .iop
            .next_node_request()
            .map_err(|_| RemoteError::QueueFull)
            .and_then(|id| {
                r.iop.request_remote_storage(
                    &r.capabilities,
                    &r.nodes,
                    caller,
                    cap,
                    placement.node,
                    grant,
                    p,
                    id,
                    id,
                    now,
                    deadline,
                )
            });
        match result {
            Ok(request) => {
                r.storage_coordinator.deletion.pending = Some(Pending {
                    request,
                    capability: cap,
                    payload: p,
                    deadline,
                })
            }
            Err(e) => {
                let _ = r.capabilities.retire_leaf(cap, caller);
                return Err(e);
            }
        }
        return Ok(true);
    }
    if now < r.storage_coordinator.deletion.next {
        return Ok(false);
    }
    let index = r.storage_coordinator.deletion.cursor;
    r.storage_coordinator.deletion.cursor = (index + 1) % 8;
    if index == 7 {
        r.storage_coordinator.deletion.next = now.saturating_add(2);
    }
    let owner = r.nodes.local_id().ok_or(RemoteError::ServiceUnavailable)?;
    let scope = r.storage_coordinator.config.scope;
    if let NativeReply::Deletion(Some(d)) = native(
        r,
        NativeRequest::DeletionLoad {
            index,
            owner,
            scope,
        },
        now,
    )? {
        if d.owner != owner || d.scope != scope {
            return Err(RemoteError::AccessDenied);
        }
        if let Some(index) = d.placements.iter().enumerate().position(|(i, p)| {
            d.acknowledged & (1 << i) == 0
                && p.is_some_and(|p| {
                    p.node == owner
                        || (grant(
                            &r.storage_coordinator.config,
                            p.node,
                            Operation::ReplicaDelete,
                            now,
                        )
                        .is_ok()
                            && r.fabric_resources
                                .entries()
                                .iter()
                                .flatten()
                                .any(|resource| {
                                    resource.id == p.resource
                                        && resource.owner == p.node
                                        && resource.online
                                        && resource.expires > now
                                }))
                })
        }) {
            r.storage_coordinator.deletion.job = Some((d, index));
        }
    }
    Ok(true)
}
// ------------------------=
// FUNC: poll
// DESC: Bounds background reclamation to one of sixteen scheduler ticks and retains failed obligations durably for later reconciliation.
// ------------------=
pub(super) fn poll(r: &mut InfinityRuntime, now: u64) -> bool {
    r.storage_coordinator.deletion.ticks = r.storage_coordinator.deletion.ticks.wrapping_add(1);
    if r.storage_coordinator.deletion.ticks % 16 != 0 {
        return false;
    }
    match step(r, now) {
        Ok(work) => work,
        Err(e) => {
            r.storage_coordinator.last_error = Some(e);
            r.storage_coordinator.deletion.job = None;
            r.storage_coordinator.deletion.next = now.saturating_add(2);
            true
        }
    }
}
