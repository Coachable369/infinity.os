//! Fixed shared-object projection; durable hints never substitute for fresh quorums.
use super::*;
use fabric::{
    manifest::{Manifest, PlacementState},
    metadata_bundle::Bundle,
};
use iop::remote::RemoteError;
enum Pending {
    None,
    Start([u8; 16]),
    Fresh(u64),
    OverlayStart(Bundle),
    Overlay(u64, Bundle),
}
pub(super) struct State {
    index: usize,
    pending: Pending,
    ids: [[u8; 16]; 8],
    count: usize,
    mutation: Option<u64>,
}
impl State {
    // ------------------------=
    // FUNC: new
    // DESC: Initializes eight bounded identities and at most one owned asynchronous observation.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            index: 0,
            pending: Pending::None,
            ids: [[0; 16]; 8],
            count: 0,
            mutation: None,
        }
    }
}
// ------------------------=
// FUNC: cancel
// DESC: Cancels only this view's exact fresh-read jobs; unrelated Console or repair work remains untouched.
// ------------------=
pub(super) fn cancel(r: &mut InfinityRuntime) {
    let pending = core::mem::replace(&mut r.storage_view.shared.pending, Pending::None);
    if let Some((u, s)) = r.storage_view.owner {
        if let Some(id) = r.storage_view.shared.mutation.take() {
            storage_metadata::mutate_cancel(r, u, s, id)
        }
        match pending {
            Pending::Fresh(id) => storage_metadata::fresh_cancel(r, u, s, id),
            Pending::Overlay(id, _) => storage_metadata_repair::overlay_cancel(r, u, s, id),
            _ => {}
        }
    }
}
// ------------------------=
// FUNC: policy
// DESC: Routes known shared policy changes through fresh quorum mutation rather than the local-only service shortcut.
// ------------------=
pub(super) fn policy(
    r: &mut InfinityRuntime,
    u: StableId,
    s: StableId,
    p: StorageOperationV1,
) -> bool {
    if !r.storage_view.shared.ids[..r.storage_view.shared.count].contains(&p.object) {
        return false;
    }
    match storage_metadata::mutate_start(r, u, s, p) {
        Ok(id) => r.storage_view.shared.mutation = Some(id),
        Err(RemoteError::QueueFull) => {
            if let Some(o) = r
                .storage_view
                .snapshot
                .objects
                .iter()
                .flatten()
                .find(|o| o.id == p.object)
                .copied()
            {
                r.storage_view.policy = Some((o, p.value as u8));
            }
        }
        Err(_) => {
            r.storage_view.snapshot.failed = true;
            r.storage_view.revision += 1;
        }
    }
    true
}
// ------------------------=
// FUNC: mutation_poll
// DESC: Consumes only the initiating Settings mutation completion and schedules a new coherent projection afterward.
// ------------------=
pub(super) fn mutation_poll(r: &mut InfinityRuntime, u: StableId, s: StableId) -> bool {
    let Some(id) = r.storage_view.shared.mutation else {
        return false;
    };
    match storage_metadata::mutate_take(r, u, s, id) {
        Ok(None) => {}
        result => {
            r.storage_view.shared.mutation = None;
            r.storage_view.phase = 0;
            r.storage_view.next = 0;
            if result.is_err() {
                r.storage_view.snapshot.failed = true;
                r.storage_view.revision += 1;
            }
        }
    }
    true
}
// ------------------------=
// FUNC: begin
// DESC: Starts one catalog scan while retaining published identities until the replacement snapshot commits.
// ------------------=
pub(super) fn begin(r: &mut InfinityRuntime) {
    r.storage_view.shared.index = 0;
    r.storage_view.shared.pending = Pending::None;
}
// ------------------------=
// FUNC: load
// DESC: Reads one catalog entry under an ephemeral real service capability and releases it before returning.
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
            Operation::PoolMetadata as u64,
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
            Operation::PoolMetadata as u64,
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
// FUNC: remove
// DESC: Removes an unavailable or tombstoned shared identity without leaving stale rows or placement details.
// ------------------=
fn remove(s: &mut Snapshot, id: [u8; 16]) {
    if let Some(i) = s.objects.iter().position(|o| o.is_some_and(|o| o.id == id)) {
        for n in i..7 {
            s.objects[n] = s.objects[n + 1]
        }
        s.objects[7] = None;
        s.count = s.count.saturating_sub(1);
        if s.selected == i {
            s.placements = [None; 8]
        }
        s.selected = s.selected.min(s.count.saturating_sub(1));
    }
}
// ------------------------=
// FUNC: merge
// DESC: Projects only certified effective immutable state; observed resource loss changes health without inventing unknown node status.
// ------------------=
fn merge(
    s: &mut Snapshot,
    m: &Manifest,
    resources: &[Option<fabric::resources::Resource>; 32],
    now: u64,
) {
    let position = s
        .objects
        .iter()
        .position(|o| o.is_some_and(|o| o.id == m.object))
        .or_else(|| s.objects.iter().position(Option::is_none));
    let Some(index) = position else { return };
    let mut row = ObjectSummary {
        id: m.object,
        version: m.version,
        generation: m.generation,
        authority: m.authority_generation,
        bytes: m.length,
        desired: m.policy.replicas() as u8,
        verified: 0,
        offline: 0,
        stale: 0,
        corrupt: 0,
        healing: false,
    };
    if index == s.selected {
        s.placements = [None; 8]
    }
    for (i, p) in m
        .placements
        .iter()
        .enumerate()
        .filter_map(|(i, p)| p.map(|p| (i, p)))
    {
        let state = fabric::observed::state(m, &p, resources, now);
        match state {
            PlacementState::Verified => row.verified += 1,
            PlacementState::Offline => row.offline += 1,
            PlacementState::Stale => row.stale += 1,
            PlacementState::Corrupt => row.corrupt += 1,
            _ => {}
        }
        if index == s.selected && i < 8 {
            s.placements[i] = Some(PlacementSummary {
                node: p.node.0,
                resource: p.resource.0,
                device: p.device,
                state: state as u8,
                version: p.version,
            })
        }
    }
    let counts = fabric::observed::summary(m, resources, now);
    row.verified = counts[49]; row.offline = counts[50]; row.stale = counts[51]; row.corrupt = counts[52];
    row.healing = counts[53] != 0;
    s.objects[index] = Some(row);
    s.count = s.objects.iter().flatten().count();
}
// ------------------------=
// FUNC: poll
// DESC: Performs one bounded native observation or asynchronous job transition and publishes only after the complete authorized scan.
// ------------------=
#[inline(never)]
pub(super) fn poll(r: &mut InfinityRuntime, u: StableId, s: StableId, now: u64) {
    let pending = core::mem::replace(&mut r.storage_view.shared.pending, Pending::None);
    match pending {
        Pending::None => {
            let i = r.storage_view.shared.index;
            if i >= 8 {
                publish(r, now);
                return;
            }
            r.storage_view.shared.index += 1;
            if let Ok(Some(b)) = load(r, i, now) {
                let id = b.manifest.object;
                // Even locally enumerated shared rows must not expose unauthorized metadata.
                remove(&mut r.storage_view.staged, id);
                if b.certificate.is_some()
                    && !b.value.record.deleted
                    && r.nodes
                        .local_id()
                        .is_some_and(|n| b.authorize(n, storage_metadata::principal(), now).is_ok())
                {
                    r.storage_view.shared.pending = Pending::Start(id);
                }
            }
        }
        Pending::Start(object) => match storage_metadata::fresh_start(r, u, s, object) {
            Ok(id) => r.storage_view.shared.pending = Pending::Fresh(id),
            Err(RemoteError::QueueFull) => {}
            Err(_) => {}
        },
        Pending::Fresh(id) => match storage_metadata::fresh_take(r, u, s, id) {
            Ok(None) => r.storage_view.shared.pending = Pending::Fresh(id),
            Ok(Some(b)) if !b.value.record.deleted => {
                r.storage_view.shared.pending = Pending::OverlayStart(b)
            }
            _ => {}
        },
        Pending::OverlayStart(b) => match storage_metadata_repair::overlay_start(r, u, s, b) {
            Ok(id) => r.storage_view.shared.pending = Pending::Overlay(id, b),
            Err(RemoteError::QueueFull) => {}
            Err(_) => {}
        },
        Pending::Overlay(id, b) => match storage_metadata_repair::overlay_take(r, u, s, id) {
            Ok(None) => r.storage_view.shared.pending = Pending::Overlay(id, b),
            Ok(Some(overlay)) => {
                merge(
                    &mut r.storage_view.staged,
                    &overlay.map_or(b.manifest, |o| o.manifest),
                    r.fabric_resources.entries(),
                    now,
                );
                let state = &mut r.storage_view.shared;
                if !state.ids[..state.count].contains(&b.manifest.object) && state.count < 8 {
                    state.ids[state.count] = b.manifest.object;
                    state.count += 1;
                }
            }
            Err(_) => {}
        },
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use fabric::{
        manifest::Placement,
        placement::StorageClass,
        resources::{Health, Resource, ResourceId, ResourceKind},
    };
    use node::types::NodeId;
    // ------------------------=
    // FUNC: summaries_follow_observed_loss_deduplicate_and_remove_tombstones
    // DESC: Exercises effective placement summaries, actual expiry, stable deduplication and removed identities without rendered text or storage shortcuts.
    // ------------------=
    #[test]
    fn summaries_follow_observed_loss_deduplicate_and_remove_tombstones() {
        let mut m = Manifest {
            object: [1; 16],
            version: 2,
            length: 0,
            hash: [2; 32],
            policy: StorageClass::Protected,
            minimum_available: 1,
            generation: 3,
            authority: NodeId([3; 32]),
            authority_generation: 1,
            chunks: [None; 64],
            placements: [None; 8],
            healing: None,
        };
        m.placements[0] = Some(Placement {
            node: NodeId([4; 32]),
            resource: ResourceId([5; 16]),
            device: [6; 16],
            generation: 1,
            version: 2,
            hash: m.hash,
            state: PlacementState::Verified,
            admission_generation: 1,
        });
        let mut s = Snapshot::empty();
        let mut resources = [None; 32];
        merge(&mut s, &m, &resources, 10);
        assert_eq!(s.objects[0].unwrap().verified, 1);
        assert_eq!(s.objects[0].unwrap().offline, 0);
        resources[0] = Some(Resource {
            id: ResourceId([5; 16]),
            owner: NodeId([4; 32]),
            kind: ResourceKind::Storage,
            device: [6; 16],
            capacity: 100,
            available: 100,
            reserved: 0,
            health: Health::Healthy,
            online: true,
            capabilities: 1,
            generation: 1,
            sequence: 1,
            expires: 10,
        });
        merge(&mut s, &m, &resources, 10);
        assert_eq!(s.count, 1);
        assert_eq!(s.objects[0].unwrap().verified, 0);
        assert_eq!(s.objects[0].unwrap().offline, 1);
        assert_eq!(s.placements[0].unwrap().state, 3);
        let wire = fabric::observed::summary(&m, &resources, 10);
        let row = s.objects[0].unwrap();
        assert_eq!((row.desired,row.verified,row.offline,row.stale,row.corrupt,row.healing),(wire[48],wire[49],wire[50],wire[51],wire[52],wire[53]!=0));
        assert_eq!(m.placements[0].unwrap().state, PlacementState::Verified);
        assert_eq!(fabric::observed::summary(&m,&resources,9)[49],1);
        let before = s;
        merge(&mut s, &m, &resources, 10);
        assert!(s == before);
        remove(&mut s, m.object);
        assert_eq!(s.count, 0);
        assert!(s.placements.iter().all(Option::is_none));
    }
    // ------------------------=
    // FUNC: shared_scan_publishes_once_and_cancel_clears_only_owned_pending_state
    // DESC: Confirms coherent snapshot publication, long refresh cadence and repeated cancellation without renderer I/O or revision churn.
    // ------------------=
    #[test]
    fn shared_scan_publishes_once_and_cancel_clears_only_owned_pending_state() {
        let mut r = InfinityRuntime::new(false);
        r.storage_view.shared.index = 8;
        r.storage_view.phase = 17;
        r.storage_view.shared.pending = Pending::Start([7; 16]);
        cancel(&mut r);
        assert!(matches!(r.storage_view.shared.pending, Pending::None));
        publish(&mut r, 10);
        let rev = r.storage_view.revision;
        publish(&mut r, 10);
        assert_eq!(r.storage_view.revision, rev);
        assert!(r.storage_view.snapshot.ready);
        assert_eq!(r.storage_view.next, 12);
        r.storage_metadata.handler = Some(empty_catalog);
        publish(&mut r, 10);
        assert_eq!(r.storage_view.next, 40);
        assert_eq!(r.storage_view.revision, rev);
    }
    // ------------------------=
    // FUNC: empty_catalog
    // DESC: Supplies a typed empty native catalog for refresh cadence verification without disk or network operations.
    // ------------------=
    fn empty_catalog(
        _: storage_metadata::NativeRequest,
    ) -> Result<storage_metadata::NativeReply, RemoteError> {
        Ok(storage_metadata::NativeReply::Bundle(None))
    }
}
