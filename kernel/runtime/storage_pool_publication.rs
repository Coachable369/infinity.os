//! Persistent explicitly approved resource publication and trusted reconnect.
use super::*;

pub(super) struct Publisher {
    cursor: usize,
    ticks: u8,
    due: [u64; 4],
    reconnect: [u64; 4],
    prepared: Option<(usize, StorageOperationV1)>,
    pending: Option<(usize, Pending)>,
    pub completed: u64,
    pub last_error: Option<RemoteError>,
}
impl Publisher {
    // ------------------------=
    // FUNC: new
    // DESC: Creates bounded ephemeral transmission state; approved participation alone is persisted.
    // ------------------=
    pub(super) const fn new() -> Self {
        Self {
            cursor: 0,
            ticks: 0,
            due: [0; 4],
            reconnect: [0; 4],
            prepared: None,
            pending: None,
            completed: 0,
            last_error: None,
        }
    }
}
// ------------------------=
// FUNC: advertise
// DESC: Persists the operator's exact advertisement grant with a bounded local lease without storing an authenticated session or cryptographic key.
// ------------------=
pub fn advertise(
    user: StableId,
    session: StableId,
    peer: NodeId,
    grant: u64,
) -> Result<(), RemoteError> {
    advertise_policy(user,session,peer,grant,false)
}
// ------------------------=
// FUNC: advertise_durable
// DESC: Applies explicitly confirmed until-revoked publication only under a durable peer-issued policy identity.
// ------------------=
pub fn advertise_durable(user:StableId,session:StableId,peer:NodeId,grant:u64)->Result<(),RemoteError>{advertise_policy(user,session,peer,grant,true)}
// ------------------------=
// FUNC: advertise_policy
// DESC: Commits the selected local publication lifetime without broadening the remote policy scope.
// ------------------=
fn advertise_policy(user:StableId,session:StableId,peer:NodeId,grant:u64,durable:bool)->Result<(),RemoteError>{
    let config = with_runtime(|r| {
        if !storage_operator::authorized(r, user, session) || grant == 0 || (durable && grant&node::durable::TAG==0) {
            return Err(RemoteError::AccessDenied);
        }
        if !r.storage_coordinator.loaded {
            return Err(RemoteError::ServiceUnavailable);
        }
        let now = r.node_clock.ok_or(RemoteError::ServiceUnavailable)?;
        let mut c = r.storage_coordinator.config;
        let slot = c
            .peers
            .iter()
            .position(|p| p.is_some_and(|p| p.peer == peer))
            .or_else(|| c.peers.iter().position(Option::is_none))
            .ok_or(RemoteError::QueueFull)?;
        let mut p = c.peers[slot].unwrap_or(Participation {
            peer,
            grants: [0; 5],
            expires: 0,
            advertise: 0,
            advertise_expires: 0,
            delete: 0,
            delete_expires: 0,
        });
        p.advertise = grant;
        p.advertise_expires = if durable {u64::MAX}else{now.saturating_add(3600)};
        c.peers[slot] = Some(p);
        Ok(c)
    })
    .ok_or(RemoteError::ServiceUnavailable)??;
    configure(user, session, config)
}
// ------------------------=
// FUNC: packet
// DESC: Encodes measured native resource identity and capacity exactly; only the explicit resource lease is newly supplied.
// ------------------=
fn packet(resource: Resource) -> StorageOperationV1 {
    let mut p = StorageOperationV1 {
        operation: Operation::ResourceAdvertise,
        object: resource.id.0,
        authority_generation: resource.generation,
        manifest_generation: resource.sequence,
        object_version: resource.capacity,
        offset: resource.available,
        scope: 0,
        value: 60,
        length: 48,
        data: [0; 64],
    };
    p.data[..16].copy_from_slice(&resource.id.0);
    p.data[16..32].copy_from_slice(&resource.device);
    p.data[32..40].copy_from_slice(&resource.reserved.to_le_bytes());
    p.data[40..44].copy_from_slice(&resource.capabilities.to_le_bytes());
    p.data[44] = match resource.health {
        fabric::resources::Health::Healthy => 1,
        fabric::resources::Health::Degraded => 2,
        fabric::resources::Health::Failed => 3,
    };
    p.data[45] = resource.online as u8;
    p.data[46..48].copy_from_slice(&1u16.to_le_bytes());
    p
}
// ------------------------=
// FUNC: step
// DESC: Performs one reconnect, native observation, request enqueue or completion without blocking the desktop or creating new trust.
// ------------------=
fn step(r: &mut InfinityRuntime, now: u64) -> Result<bool, RemoteError> {
    let caller = r
        .service_identity(SERVICE_REPLICA_STORAGE)
        .ok_or(RemoteError::ServiceUnavailable)?;
    if let Some((index, p)) = r.storage_coordinator.publisher.pending {
        let done = r.iop.remote.take_storage_result(caller, p.request);
        if done.is_none() && now < p.deadline {
            return Ok(false);
        }
        r.iop.remote.discard(caller, p.request);
        let _ = r.capabilities.retire_leaf(p.capability, caller);
        r.storage_coordinator.publisher.pending = None;
        r.storage_coordinator.publisher.due[index] = now.saturating_add(20);
        let reply = done.ok_or(RemoteError::DeadlineExceeded)?.result?;
        if reply != p.payload {
            return Err(RemoteError::UnknownResponse);
        }
        r.storage_coordinator.publisher.completed =
            r.storage_coordinator.publisher.completed.saturating_add(1);
        return Ok(true);
    }
    if let Some((index, p)) = r.storage_coordinator.publisher.prepared.take() {
        let peer = r.storage_coordinator.config.peers[index]
            .ok_or(RemoteError::AccessDenied)?
            .peer;
        let grant = grant(
            &r.storage_coordinator.config,
            peer,
            Operation::ResourceAdvertise,
            now,
        )?;
        let deadline = now.saturating_add(30);
        let cap = r
            .capabilities
            .grant(
                CapabilityType::ServiceCall,
                Operation::ResourceAdvertise as u64,
                1,
                p.scope,
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
                    peer,
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
                r.storage_coordinator.publisher.pending = Some((
                    index,
                    Pending {
                        request,
                        capability: cap,
                        payload: p,
                        deadline,
                    },
                ))
            }
            Err(e) => {
                let _ = r.capabilities.retire_leaf(cap, caller);
                r.storage_coordinator.publisher.due[index] = now.saturating_add(5);
                return Err(e);
            }
        }
        return Ok(true);
    }
    let index = r.storage_coordinator.publisher.cursor;
    r.storage_coordinator.publisher.cursor = (index + 1) % 4;
    let Some(peer) = r.storage_coordinator.config.peers[index] else {
        return Ok(false);
    };
    let active = (peer.expires > now && peer.grants != [0; 5])
        || (peer.advertise_expires > now && peer.advertise != 0)
        || (peer.delete_expires > now && peer.delete != 0);
    if !active {
        return Ok(false);
    }
    if r.node_transport.trust.session(peer.peer).is_none() {
        if now < r.storage_coordinator.publisher.reconnect[index] {
            return Ok(false);
        }
        r.storage_coordinator.publisher.reconnect[index] = now.saturating_add(10);
        if r.nodes.paired_digest(peer.peer).is_none() {
            return Err(RemoteError::TrustRequired);
        }
        let Some(link) = r.node_transport.peer_link(peer.peer) else {
            return Ok(false);
        };
        // reconnect=true verifies existing paired transcript and current trust;
        // it never calls first-pairing confirmation or grants new peer authority.
        r.node_transport
            .trust
            .begin(
                &mut r.nodes,
                link,
                r.storage_coordinator.config.scope,
                true,
                now,
            )
            .map_err(|_| RemoteError::TrustRequired)?;
        return Ok(true);
    }
    if peer.advertise == 0
        || peer.advertise_expires <= now
        || now < r.storage_coordinator.publisher.due[index]
    {
        return Ok(false);
    }
    let owner = r.nodes.local_id().ok_or(RemoteError::ServiceUnavailable)?;
    let NativeReply::Resource(resource) = native(r, NativeRequest::LocalResource { owner }, now)?
    else {
        return Err(RemoteError::InvalidState);
    };
    if resource.owner != owner {
        return Err(RemoteError::AccessDenied);
    }
    let mut p = packet(resource);
    p.scope = r.storage_coordinator.config.scope;
    r.storage_coordinator.publisher.prepared = Some((index, p));
    Ok(true)
}
// ------------------------=
// FUNC: poll
// DESC: Allows at most one publisher action per eight scheduler polls; temporary failures retain approved configuration for bounded reconnect rather than requiring manual reconfiguration.
// ------------------=
pub(super) fn poll(r: &mut InfinityRuntime, now: u64) -> bool {
    r.storage_coordinator.publisher.ticks = r.storage_coordinator.publisher.ticks.wrapping_add(1);
    if r.storage_coordinator.publisher.ticks % 8 != 0 {
        return false;
    }
    match step(r, now) {
        Ok(work) => work,
        Err(e) => {
            r.storage_coordinator.publisher.last_error = Some(e);
            r.storage_coordinator.last_error = Some(e);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: persisted
    // DESC: Returns an explicit persisted publication fixture without any user session or negotiated peer key.
    // ------------------=
    fn persisted(_: NativeRequest) -> Result<NativeReply, RemoteError> {
        let mut c = Configuration::empty();
        c.peers[0] = Some(Participation {
            peer: NodeId([3; 32]),
            grants: [0; 5],
            expires: 0,
            advertise: node::durable::TAG | 91,
            advertise_expires: u64::MAX,
            delete: 0,
            delete_expires: 0,
        });
        Ok(NativeReply::Config(c.encode()?))
    }
    // ------------------------=
    // FUNC: finite_persisted
    // DESC: Supplies a genuine prior-boot uptime lease which must not gain time after restart.
    // ------------------=
    fn finite_persisted(p:NativeRequest)->Result<NativeReply,RemoteError>{let NativeReply::Config(bytes)=persisted(p)? else{return Err(RemoteError::InvalidState)};let mut c=Configuration::decode(&bytes)?;c.peers[0].as_mut().unwrap().advertise_expires=100;Ok(NativeReply::Config(c.encode()?))}
    // ------------------------=
    // FUNC: restart_restores_only_approved_publication_and_requires_existing_trust
    // DESC: A fresh service loads persistent publication with no login but cannot reconnect an unpaired peer, issue transfer authority, or extend an expired lease.
    // ------------------=
    #[test]
    fn restart_restores_only_approved_publication_and_requires_existing_trust() {
        let mut r = InfinityRuntime::new(false);
        r.define_bootstrap().unwrap();
        r.start_all(0);
        r.nodes.initialize(&[52; 32], true).unwrap();
        r.storage_coordinator.handler = Some(persisted);
        super::super::poll(&mut r, 1);
        assert!(r.storage_coordinator.loaded);
        assert_eq!(
            grant(
                &r.storage_coordinator.config,
                NodeId([3; 32]),
                Operation::ResourceAdvertise,
                99
            ),
            Ok(node::durable::TAG | 91)
        );
        assert_eq!(
            grant(
                &r.storage_coordinator.config,
                NodeId([3; 32]),
                Operation::TransferBegin,
                2
            ),
            Err(RemoteError::AccessDenied)
        );
        assert_eq!(step(&mut r, 2), Err(RemoteError::TrustRequired));
        assert!(r.storage_coordinator.publisher.pending.is_none());
        r.storage_coordinator.config.peers[0].as_mut().unwrap().advertise_expires=100;
        assert_eq!(
            grant(
                &r.storage_coordinator.config,
                NodeId([3; 32]),
                Operation::ResourceAdvertise,
                100
            ),
            Err(RemoteError::AccessDenied)
        );
        let bytes = r.storage_coordinator.config.encode().unwrap();
        let restored = Configuration::decode(&bytes).unwrap();
        assert_eq!(restored.peers, r.storage_coordinator.config.peers);
        r.storage_coordinator.loaded=false;r.storage_coordinator.handler=Some(finite_persisted);
        super::super::poll(&mut r,1);
        assert!(r.storage_coordinator.config.peers.iter().all(Option::is_none));
    }
}
