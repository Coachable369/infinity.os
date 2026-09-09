//! Explicitly authorized, session-owned resource publication. One slot per tick;
//! no network wait, invented capacity, implicit peer trust, or unlimited retry.
use super::*;
use identity::StableId;
use node::types::NodeId;
use iop::{remote::RemoteError, storage_protocol::{Operation, StorageOperationV1}};

#[derive(Clone, Copy)]
struct Subscription {
    user: StableId, session: StableId, peer: NodeId, grant: u64,
    next: u64, request: Option<u64>,
}
pub struct Publisher {
    entries: [Option<Subscription>; 4], cursor: usize,
    pub last_error: Option<RemoteError>, pub completed: u64,
}
impl Publisher {
    // ------------------------=
    // FUNC: new
    // DESC: Creates four empty explicit publication subscriptions with no default network authority.
    // ------------------=
    pub const fn new() -> Self {
        Self { entries: [None; 4], cursor: 0, last_error: None, completed: 0 }
    }
}

// ------------------------=
// FUNC: start
// DESC: Starts bounded periodic publication only for an authenticated operator and an explicit peer-issued advertisement grant.
// ------------------=
pub fn start(user: StableId, session: StableId, peer: NodeId, grant: u64) -> Result<(), RemoteError> {
    with_runtime(|r| start_from(r, user, session, peer, grant)).ok_or(RemoteError::ServiceUnavailable)?
}

// ------------------------=
// FUNC: start_from
// DESC: Rejects unowned or duplicate destinations and preserves in-flight ownership rather than replacing a live subscription.
// ------------------=
pub(super) fn start_from(r: &mut InfinityRuntime, user: StableId, session: StableId,
    peer: NodeId, grant: u64) -> Result<(), RemoteError> {
    if !storage_operator::authorized(r, user, session) || grant == 0 || peer.0 == [0; 32]
        || Some(peer) == r.nodes.local_id() { return Err(RemoteError::AccessDenied); }
    let now = r.node_clock.ok_or(RemoteError::ServiceUnavailable)?;
    if r.storage_advertiser.entries.iter().flatten().any(|s| s.peer == peer) {
        return Err(RemoteError::Conflict);
    }
    let slot = r.storage_advertiser.entries.iter().position(Option::is_none).ok_or(RemoteError::QueueFull)?;
    r.storage_advertiser.entries[slot] = Some(Subscription { user, session, peer, grant, next: now, request: None });
    Ok(())
}

// ------------------------=
// FUNC: advertisement
// DESC: Converts an actual native observation to a sixty-second lease without altering its identity, capacity or reboot-monotonic sequence.
// ------------------=
fn advertisement(mut p: StorageOperationV1, owner: NodeId, now: u64) -> Result<StorageOperationV1, RemoteError> {
    if p.operation != Operation::ResourceInspect { return Err(RemoteError::MalformedRequest); }
    p.operation = Operation::ResourceAdvertise;
    p.object.copy_from_slice(&p.data[..16]); p.value = 60;
    let measured = fabric::resource_protocol::decode(p, owner, now).map_err(|_| RemoteError::MalformedRequest)?;
    if measured.id.0 == [0; 16] || measured.device == [0; 16] || measured.capacity == 0
        || measured.available > measured.capacity
        || measured.reserved > measured.capacity.saturating_sub(measured.available)
        || measured.generation == 0 || measured.sequence == 0 { return Err(RemoteError::MalformedRequest); }
    Ok(p)
}

// ------------------------=
// FUNC: poll
// DESC: Advances at most one subscription, renews only after authenticated completion, and stops after any authority, transport or observation failure.
// ------------------=
pub(super) fn poll(r: &mut InfinityRuntime, now: u64) {
    let index = r.storage_advertiser.cursor;
    r.storage_advertiser.cursor = (index + 1) % 4;
    let Some(mut sub) = r.storage_advertiser.entries[index] else { return; };
    let result = (|| -> Result<(), RemoteError> {
        if !storage_operator::authorized(r, sub.user, sub.session) { return Err(RemoteError::AccessDenied); }
        if let Some(id) = sub.request {
            if let Some(done) = storage_operator::take_from(r, sub.user, sub.session, id)? {
                let response = done.result?;
                if response.operation != Operation::ResourceAdvertise { return Err(RemoteError::UnknownResponse); }
                sub.request = None; sub.next = now.saturating_add(20);
                r.storage_advertiser.completed = r.storage_advertiser.completed.saturating_add(1);
            }
        } else if now >= sub.next {
            let observed = storage_client::read(r, now).map_err(|_| RemoteError::ServiceUnavailable)?;
            let owner = r.nodes.local_id().ok_or(RemoteError::ServiceUnavailable)?;
            let payload = advertisement(observed, owner, now)?;
            sub.request = Some(storage_operator::submit_to(r, sub.user, sub.session, sub.peer, sub.grant, payload)?);
        }
        Ok(())
    })();
    match result {
        Ok(()) => r.storage_advertiser.entries[index] = Some(sub),
        Err(error) => {
            r.storage_advertiser.last_error = Some(error);
            r.storage_advertiser.entries[index] = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: observation_conversion_preserves_measured_identity_and_bounds
    // DESC: Checks canonical wire roundtrip, identity and capacity preservation, expiry and rejection of malformed observations.
    // ------------------=
    #[test]
    fn observation_conversion_preserves_measured_identity_and_bounds() {
        let owner = NodeId([3; 32]);
        let mut p = StorageOperationV1 { operation: Operation::ResourceInspect, object: [0; 16],
            authority_generation: 2, manifest_generation: 0x100000002, object_version: 65536,
            offset: 32768, scope: 0, value: 0, length: 48, data: [0; 64] };
        p.data[..16].fill(1); p.data[16..32].fill(2);
        p.data[32..40].copy_from_slice(&4096u64.to_le_bytes());
        p.data[40..44].copy_from_slice(&1u32.to_le_bytes());
        p.data[44..48].copy_from_slice(&[1, 1, 1, 0]);
        let a = advertisement(p, owner, 10).unwrap();
        assert_eq!(StorageOperationV1::decode(&a.encode().unwrap()).unwrap(), a);
        let decoded = fabric::resource_protocol::decode(a, owner, 10).unwrap();
        assert_eq!(decoded.id.0, [1; 16]); assert_eq!(decoded.device, [2; 16]);
        assert_eq!(decoded.capacity, 65536); assert_eq!(decoded.available, 32768);
        assert_eq!(decoded.reserved, 4096); assert_eq!(decoded.sequence, 0x100000002);
        assert_eq!(decoded.expires, 70);
        p.length = 47;
        assert_eq!(advertisement(p, owner, 10), Err(RemoteError::MalformedRequest));
        p.length = 48; p.data[44] = 0;
        assert_eq!(advertisement(p, owner, 10), Err(RemoteError::MalformedRequest));
        p.data[44] = 1; p.operation = Operation::ObjectRead;
        assert_eq!(advertisement(p, owner, 10), Err(RemoteError::MalformedRequest));
        p.operation = Operation::ResourceInspect; p.offset = 65537;
        assert_eq!(advertisement(p, owner, 10), Err(RemoteError::MalformedRequest));
    }
}
