//! Bounded service-owned repair scheduling. Cached metadata is a scheduling hint
//! only: the selected job must independently complete fresh owner/repair quorums.
use super::*;
use fabric::{
    manifest::{Manifest, PlacementState},
    metadata_bundle::Bundle,
    resources::{Health, Resource, ResourceKind, MAX_RESOURCES},
};
use iop::remote::RemoteError;
use node::types::{NodeId, SessionState};
enum Phase {
    Scan,
    Inspect(Bundle),
    Fresh { request: u64, destination: NodeId },
    Repair { request: u64 },
}
pub struct Service {
    phase: Phase,
    index: usize,
    next_tick: u64,
    pub started: u64,
    pub completed: u64,
    pub last_error: Option<RemoteError>,
}
impl Service {
    // ------------------------=
    // FUNC: new
    // DESC: Initializes one bounded repair scheduling slot without ambient user or network authority.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            phase: Phase::Scan,
            index: 0,
            next_tick: 0,
            started: 0,
            completed: 0,
            last_error: None,
        }
    }
    // ------------------------=
    // FUNC: due
    // DESC: Bounds scheduling work to one transition every five seconds, including unchanged and failed observations.
    // ------------------=
    fn due(&mut self, now: u64) -> bool {
        if now < self.next_tick {
            return false;
        }
        self.next_tick = now.saturating_add(5);
        true
    }
}
// ------------------------=
// FUNC: choose
// DESC: Selects only an explicitly delegated healthy destination after observed owner loss and an actual protection deficit.
// ------------------=
fn choose(
    bundle: &Bundle,
    m: &Manifest,
    resources: &[Option<Resource>; MAX_RESOURCES],
    local: NodeId,
    owner_connected: bool,
    now: u64,
    approved: impl Fn(NodeId) -> bool,
) -> Option<NodeId> {
    if local == bundle.group.owner
        || owner_connected
        || bundle.value.record.deleted
        || !resources
            .iter()
            .flatten()
            .any(|r| r.owner == bundle.group.owner && (!r.online || r.expires <= now))
    {
        return None;
    }
    let certificate = bundle.certificate?;
    let grant = bundle
        .repair_grants
        .iter()
        .flatten()
        .find(|g| g.writer == local && g.validate(&bundle.group, &certificate, now).is_ok())?;
    let healthy = |node: NodeId, resource: fabric::resources::ResourceId, generation: u64| {
        resources.iter().flatten().any(|r| {
            r.owner == node
                && r.id == resource
                && r.generation == generation
                && r.online
                && r.expires > now
                && r.health == Health::Healthy
        })
    };
    let available = m
        .placements
        .iter()
        .flatten()
        .filter(|p| {
            p.state == PlacementState::Verified
                && p.version == m.version
                && p.hash == m.hash
                && healthy(p.node, p.resource, p.generation)
        })
        .count();
    if available >= m.policy.replicas()
        || !m.placements.iter().flatten().any(|p| {
            p.node == local
                && p.state == PlacementState::Verified
                && healthy(p.node, p.resource, p.generation)
        })
    {
        return None;
    }
    resources
        .iter()
        .flatten()
        .filter(|r| {
            r.kind == ResourceKind::Storage
                && r.online
                && r.expires > now
                && r.health == Health::Healthy
                && r.capabilities & 1 != 0
                && r.available >= m.length
                && grant.destinations.contains(&r.owner)
                && approved(r.owner)
                && !m.placements.iter().flatten().any(|p| {
                    p.node == r.owner
                        && p.state == PlacementState::Verified
                        && healthy(p.node, p.resource, p.generation)
                })
        })
        .min_by_key(|r| (r.owner.0, r.id.0))
        .map(|r| r.owner)
}
// ------------------------=
// FUNC: load
// DESC: Uses a short-lived typed service capability for exactly one durable metadata observation.
// ------------------=
fn load(r: &mut InfinityRuntime, index: usize, now: u64) -> Result<Option<Bundle>, RemoteError> {
    let identity = r
        .service_identity(SERVICE_REPLICA_STORAGE)
        .ok_or(RemoteError::ServiceUnavailable)?;
    let handler = r
        .storage_metadata
        .handler
        .ok_or(RemoteError::ServiceUnavailable)?;
    let cap = r
        .capabilities
        .grant(
            CapabilityType::ServiceCall,
            iop::storage_protocol::Operation::PoolMetadata as u64,
            1,
            0,
            identity,
            identity,
            Some(now.saturating_add(1)),
            0,
        )
        .map_err(|_| RemoteError::QueueFull)?;
    let result = r
        .capabilities
        .validate(
            cap,
            identity,
            CapabilityType::ServiceCall,
            iop::storage_protocol::Operation::PoolMetadata as u64,
            1,
            0,
            now,
        )
        .map_err(|_| RemoteError::AccessDenied)
        .and_then(|_| handler(storage_metadata::NativeRequest::Load { index }));
    let _ = r.capabilities.retire_leaf(cap, identity);
    match result? {
        storage_metadata::NativeReply::Bundle(b) => Ok(b),
        _ => Err(RemoteError::InvalidState),
    }
}
// ------------------------=
// FUNC: poll
// DESC: Advances at most one service-owned scheduling transition outside UI execution; all actual repair still requires fresh quorums and leased authority.
// ------------------=
#[inline(never)]
pub(super) fn poll(r: &mut InfinityRuntime, now: u64) {
    if !r.storage_metadata_auto.due(now) {
        return;
    }
    let phase = core::mem::replace(&mut r.storage_metadata_auto.phase, Phase::Scan);
    let result = (|| match phase {
        Phase::Scan => {
            let i = r.storage_metadata_auto.index;
            r.storage_metadata_auto.index = (i + 1) % 8;
            if let Some(b) = load(r, i, now)? {
                r.storage_metadata_auto.phase = Phase::Inspect(b)
            }
            Ok(())
        }
        Phase::Inspect(b) => {
            let local = r.nodes.local_id().ok_or(RemoteError::ServiceUnavailable)?;
            let connected = r.nodes.sessions().iter().flatten().any(|s| {
                s.peer == b.group.owner
                    && s.state == SessionState::Established
                    && now < s.expires_at
            });
            let effective = storage_metadata_repair::observed_overlay(r, b, now)?
                .map(|o| o.manifest)
                .unwrap_or(b.manifest);
            if let Some(destination) = choose(
                &b,
                &effective,
                r.fabric_resources.entries(),
                local,
                connected,
                now,
                |node| storage_metadata::peer_grant(r, node, now).is_ok(),
            ) {
                let request = storage_metadata::fresh_start_service(r, b.manifest.object)?;
                r.storage_metadata_auto.phase = Phase::Fresh {
                    request,
                    destination,
                };
            }
            Ok(())
        }
        Phase::Fresh {
            request,
            destination,
        } => {
            match storage_metadata::fresh_take_service(r, request)? {
                None => {
                    r.storage_metadata_auto.phase = Phase::Fresh {
                        request,
                        destination,
                    }
                }
                Some(b) => {
                    let id = storage_metadata_repair::begin_automatic(r, b, destination)?;
                    r.storage_metadata_auto.started =
                        r.storage_metadata_auto.started.saturating_add(1);
                    r.storage_metadata_auto.phase = Phase::Repair { request: id };
                }
            }
            Ok(())
        }
        Phase::Repair { request } => {
            if storage_metadata_repair::take_automatic(r, request)?.is_none() {
                r.storage_metadata_auto.phase = Phase::Repair { request }
            } else {
                r.storage_metadata_auto.completed =
                    r.storage_metadata_auto.completed.saturating_add(1);
                r.storage_metadata_auto.last_error = None;
            }
            Ok(())
        }
    })();
    if let Err(e) = result {
        r.storage_metadata_auto.last_error = Some(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{
        crypto::{KeyRef, NodeCrypto},
        fabric::{
            manifest::Placement,
            metadata::{Certificate, Group, Receipt, Record, SignedRecord},
            metadata_bundle::{namespace_digest, policy_digest},
            metadata_repair::RepairGrant,
            placement::StorageClass,
            resources::ResourceId,
        },
    };
    use sha2::{Digest, Sha256};
    // ------------------------=
    // FUNC: fixture
    // DESC: Builds real owner signatures and distinct typed resource observations for scheduling behavior tests.
    // ------------------=
    fn fixture() -> (Bundle, [Option<Resource>; MAX_RESOURCES], NodeId, NodeId) {
        let keys: [(NodeCrypto, KeyRef); 3] = core::array::from_fn(|i| {
            let mut c = NodeCrypto::new();
            let k = c.initialize(&[i as u8 + 41; 32], true).unwrap();
            (c, k)
        });
        let public = keys.each_ref().map(|v| v.0.public_identity().unwrap());
        let members = public.map(|k| {
            let mut h = Sha256::new();
            h.update(b"InfinityOS NodeId v1");
            h.update(k);
            NodeId(h.finalize().into())
        });
        let group = Group {
            epoch: 1,
            owner: members[0],
            members,
            keys: public,
        };
        let destination = NodeId([19; 32]);
        let mut m = Manifest {
            object: [1; 16],
            version: 1,
            length: 0,
            hash: Sha256::digest([]).into(),
            policy: StorageClass::Protected,
            minimum_available: 1,
            generation: 1,
            authority: members[0],
            authority_generation: 1,
            chunks: [None; 64],
            placements: [None; 8],
            healing: None,
        };
        let mut resources = [None; MAX_RESOURCES];
        for i in 0..3 {
            let node = if i == 2 { destination } else { members[i] };
            let id = ResourceId([i as u8 + 1; 16]);
            resources[i] = Some(Resource {
                id,
                owner: node,
                kind: ResourceKind::Storage,
                device: [i as u8 + 1; 16],
                capacity: 1024,
                available: 1024,
                reserved: 0,
                health: Health::Healthy,
                online: i != 0,
                capabilities: 1,
                generation: 1,
                sequence: 1,
                expires: 100,
            });
            if i < 2 {
                m.placements[i] = Some(Placement {
                    node,
                    resource: id,
                    device: [i as u8 + 1; 16],
                    generation: 1,
                    version: 1,
                    hash: m.hash,
                    state: PlacementState::Verified,
                    admission_generation: 1,
                })
            }
        }
        let mut raw = [0; 5376];
        m.encode(&mut raw).unwrap();
        let record = Record {
            group: group.digest(),
            object: m.object,
            generation: 1,
            version: 1,
            previous: [0; 32],
            manifest: Sha256::digest(raw).into(),
            namespace: namespace_digest(m.object, b"/shared").unwrap(),
            policy: policy_digest(&group, &m, 1),
            revocation: 1,
            deleted: false,
        };
        let value = SignedRecord {
            record,
            signature: keys[0].0.sign(keys[0].1, &record.encode()).unwrap(),
        };
        let certificate = Certificate {
            value,
            prepared: core::array::from_fn(|i| {
                let mut r = Receipt {
                    member: i as u8,
                    digest: record.digest(),
                    published: false,
                    signature: [0; 64],
                };
                r.signature = keys[i].0.sign(keys[i].1, &r.transcript()).unwrap();
                r
            }),
        };
        let mut grant = RepairGrant {
            group: group.digest(),
            anchor: record.digest(),
            writer: members[1],
            destinations: [
                destination,
                NodeId([0; 32]),
                NodeId([0; 32]),
                NodeId([0; 32]),
            ],
            expires: 50,
            signature: [0; 64],
        };
        grant.signature = keys[0].0.sign(keys[0].1, &grant.transcript()).unwrap();
        let mut path = [0; 95];
        path[..7].copy_from_slice(b"/shared");
        let b = Bundle {
            group,
            value,
            certificate: Some(certificate),
            manifest: m,
            path,
            path_len: 7,
            grants: [None; 3],
            repair_grants: [Some(grant), None],
        };
        (b, resources, members[1], destination)
    }
    // ------------------------=
    // FUNC: automatic_schedule_requires_observed_loss_and_live_authority
    // DESC: Rejects guessed loss, connected owner, absent approval, expired delegation and already healthy protection; admits an observed deficit only.
    // ------------------=
    #[test]
    fn automatic_schedule_requires_observed_loss_and_live_authority() {
        let (b, mut r, local, d) = fixture();
        assert_eq!(
            choose(&b, &b.manifest, &r, local, false, 1, |n| n == d),
            Some(d)
        );
        assert_eq!(choose(&b, &b.manifest, &r, local, true, 1, |_| true), None);
        assert_eq!(
            choose(&b, &b.manifest, &r, local, false, 1, |_| false),
            None
        );
        assert_eq!(
            choose(&b, &b.manifest, &r, local, false, 50, |_| true),
            None
        );
        r[0] = None;
        assert_eq!(choose(&b, &b.manifest, &r, local, false, 1, |_| true), None);
        let (b, r, local, _) = fixture();
        let mut m = b.manifest;
        m.placements[2] = Some(Placement {
            node: r[2].unwrap().owner,
            resource: r[2].unwrap().id,
            device: r[2].unwrap().device,
            generation: 1,
            version: 1,
            hash: m.hash,
            state: PlacementState::Verified,
            admission_generation: 1,
        });
        assert_eq!(choose(&b, &m, &r, local, false, 1, |_| true), None);
    }
    // ------------------------=
    // FUNC: scheduling_cadence_is_bounded
    // DESC: Prevents repeated runtime ticks from performing repeated native scans or starting duplicate scheduling work.
    // ------------------=
    #[test]
    fn scheduling_cadence_is_bounded() {
        let mut s = Service::new();
        assert!(s.due(0));
        for t in 0..5 {
            assert!(!s.due(t))
        }
        assert!(s.due(5));
        assert!(!s.due(5));
        assert!(s.due(10));
    }
}
