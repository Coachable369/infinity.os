//! Read-only Settings projection. One typed service query per pump, no renderer I/O.
use super::*;
use identity::StableId;
use iop::storage_protocol::{Operation, StorageOperationV1};
#[path = "storage_view_shared.rs"]
mod shared;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ObjectSummary {
    pub id: [u8; 16],
    pub version: u64,
    pub generation: u64,
    pub authority: u64,
    pub bytes: u64,
    pub desired: u8,
    pub verified: u8,
    pub offline: u8,
    pub stale: u8,
    pub corrupt: u8,
    pub healing: bool,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NodeSummary {
    pub id: [u8; 32],
    pub online: bool,
    pub capacity: u64,
    pub reserved: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PlacementSummary {
    pub node: [u8; 32],
    pub resource: [u8; 16],
    pub device: [u8; 16],
    pub state: u8,
    pub version: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Snapshot {
    pub objects: [Option<ObjectSummary>; 8],
    pub count: usize,
    pub selected: usize,
    pub capacity: u64,
    pub eligible: u64,
    pub reserved: u64,
    pub nodes: usize,
    pub ready: bool,
    pub failed: bool,
    pub node_rows: [Option<NodeSummary>; 32],
    pub node_count: usize,
    pub selected_node: usize,
    pub placements: [Option<PlacementSummary>; 8],
    pub selected_placement: usize,
}
impl Snapshot {
    // ------------------------=
    // FUNC: empty
    // DESC: Starts without invented capacity, nodes or successful observations.
    // ------------------=
    pub const fn empty() -> Self {
        Self {
            objects: [None; 8],
            count: 0,
            selected: 0,
            capacity: 0,
            eligible: 0,
            reserved: 0,
            nodes: 0,
            ready: false,
            failed: false,
            node_rows: [None; 32],
            node_count: 0,
            selected_node: 0,
            placements: [None; 8],
            selected_placement: 0,
        }
    }
}
pub struct View {
    owner: Option<(StableId, StableId)>,
    next: u64,
    phase: usize,
    staged: Snapshot,
    pub snapshot: Snapshot,
    pub revision: u64,
    policy: Option<(ObjectSummary, u8)>,
    shared: shared::State,
}
impl View {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates a fixed projection and no background work until an operator opens it.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            owner: None,
            next: 0,
            phase: 0,
            staged: Snapshot::empty(),
            snapshot: Snapshot::empty(),
            revision: 0,
            policy: None,
            shared: shared::State::new(),
        }
    }
    // ------------------------=
    // FUNC: queue_policy
    // DESC: Queues only valid selected-object intent; actual authority and generation are rechecked by the shared broker on the next tick.
    // ------------------=
    fn queue_policy(&mut self, policy: u8) {
        if (1..=3).contains(&policy)
            && self.owner.is_some()
            && self.snapshot.ready
            && !self.snapshot.failed
        {
            if let Some(object) = self.snapshot.objects[self.snapshot.selected.min(7)] {
                self.policy = Some((object, policy));
                self.next = 0;
            }
        }
    }
}
// ------------------------=
// FUNC: open
// DESC: Requests a fresh owner-authorized projection without reading disk in the window-open path.
// ------------------=
pub fn open(user: StableId, session: StableId) {
    with_runtime(|r| {
        shared::cancel(r);
        let v = &mut r.storage_view;
        if v.owner != Some((user, session)) {
            v.snapshot = Snapshot::empty();
            v.revision += 1;
        }
        v.owner = Some((user, session));
        v.next = 0;
        v.phase = 0;
    });
}
// ------------------------=
// FUNC: close
// DESC: Stops hidden-window polling and discards pending user mutations.
// ------------------=
pub fn close() {
    with_runtime(|r| {
        shared::cancel(r);
        r.storage_view.owner = None;
        r.storage_view.policy = None;
    });
}
// ------------------------=
// FUNC: visibility
// DESC: Starts or suspends Settings refresh without repeatedly restarting an active bounded scan.
// ------------------=
pub fn visibility(user: StableId, session: StableId, visible: bool) -> u64 {
    with_runtime(|r| {
        let authorized = storage_client::authorize(r, user, session).is_ok();
        if !visible || !authorized || r.storage_view.owner != Some((user, session)) {
            shared::cancel(r);
        }
        let v = &mut r.storage_view;
        if visible && authorized && v.owner != Some((user, session)) {
            v.owner = Some((user, session));
            v.phase = 0;
            v.next = 0;
            v.snapshot = Snapshot::empty();
            v.revision += 1;
        }
        if !visible || !authorized {
            v.owner = None;
            v.policy = None;
            if !authorized && v.snapshot != Snapshot::empty() {
                v.snapshot = Snapshot::empty();
                v.revision += 1;
            }
        }
        v.revision
    })
    .unwrap_or(0)
}
// ------------------------=
// FUNC: snapshot
// DESC: Returns only immutable already-observed values; painting never invokes native storage.
// ------------------=
pub fn snapshot() -> Snapshot {
    with_runtime(|r| r.storage_view.snapshot).unwrap_or(Snapshot::empty())
}
// ------------------------=
// FUNC: select_next
// DESC: Moves selection among observed objects without changing any object identity or policy.
// ------------------=
pub fn select_next() {
    with_runtime(|r| {
        shared::cancel(r);
        let v = &mut r.storage_view;
        if v.snapshot.count > 0 {
            v.snapshot.selected = (v.snapshot.selected + 1) % v.snapshot.count;
            v.snapshot.placements = [None; 8];
            v.snapshot.selected_placement = 0;
            v.phase = 0;
            v.next = 0;
            v.revision += 1;
        }
    });
}
// ------------------------=
// FUNC: select_node
// DESC: Cycles measured node identities using ordinary keyboard or pointer activation without issuing storage operations.
// ------------------=
pub fn select_node() {
    with_runtime(|r| {
        let v = &mut r.storage_view;
        if v.snapshot.node_count > 0 {
            v.snapshot.selected_node = (v.snapshot.selected_node + 1) % v.snapshot.node_count;
            v.revision += 1;
        }
    });
}
// ------------------------=
// FUNC: select_placement
// DESC: Cycles known physical placements without confusing offline replicas with deleted records.
// ------------------=
pub fn select_placement() {
    with_runtime(|r| {
        let v = &mut r.storage_view;
        for step in 1..=8 {
            let next = (v.snapshot.selected_placement + step) % 8;
            if v.snapshot.placements[next].is_some() {
                v.snapshot.selected_placement = next;
                v.revision += 1;
                break;
            }
        }
    });
}
// ------------------------=
// FUNC: set_policy
// DESC: Queues a generation-fenced mutation for the authenticated Settings owner through the same IOP service as Console.
// ------------------=
pub fn set_policy(policy: u8) {
    with_runtime(|r| r.storage_view.queue_policy(policy));
}
// ------------------------=
// FUNC: poll
// DESC: Revalidates live operator authority and performs at most one bounded IOP operation before publishing a coherent immutable projection.
// ------------------=
pub(super) fn poll(r: &mut InfinityRuntime, now: u64) {
    let Some((user, session)) = r.storage_view.owner else {
        return;
    };
    if storage_client::authorize(r, user, session).is_err() {
        shared::cancel(r);
        let v = &mut r.storage_view;
        v.owner = None;
        v.policy = None;
        if v.snapshot != Snapshot::empty() {
            v.snapshot = Snapshot::empty();
            v.revision += 1;
        }
        return;
    }
    if now < r.storage_view.next {
        return;
    }
    if shared::mutation_poll(r, user, session) {
        return;
    }
    if r.storage_view.phase >= 17 {
        shared::poll(r, user, session, now);
        return;
    }
    let mut p = StorageOperationV1 {
        operation: Operation::PoolInspect,
        object: [0; 16],
        authority_generation: 0,
        manifest_generation: 0,
        object_version: 0,
        offset: 0,
        scope: 0,
        value: 1,
        length: 0,
        data: [0; 64],
    };
    if let Some((object, policy)) = r.storage_view.policy.take() {
        p.operation = Operation::ObjectSetPolicy;
        p.object = object.id;
        p.authority_generation = object.authority;
        p.manifest_generation = object.generation;
        p.object_version = object.version;
        p.value = policy as u64;
        if shared::policy(r, user, session, p) {
            return;
        }
        if storage_client::perform(r, now, p).is_err() {
            r.storage_view.snapshot.failed = true;
            r.storage_view.revision += 1;
        }
        r.storage_view.phase = 0;
        return;
    }
    let phase = r.storage_view.phase;
    if phase == 0 {
        r.storage_view.staged = Snapshot::empty();
        r.storage_view.staged.selected = r.storage_view.snapshot.selected;
        r.storage_view.staged.selected_node = r.storage_view.snapshot.selected_node;
        r.storage_view.staged.selected_placement = r.storage_view.snapshot.selected_placement;
        p.operation = Operation::ResourceInspect;
        p.value = 0;
    } else if phase <= 8 {
        p.offset = (phase - 1) as u64;
    } else {
        let Some(object) = r.storage_view.staged.objects[r.storage_view.staged.selected.min(7)]
        else {
            finish_local(r, now);
            return;
        };
        p.object = object.id;
        p.manifest_generation = object.generation;
        p.value = 2;
        p.offset = (phase - 9) as u64;
    }
    let result = storage_client::perform(r, now, p);
    if let Err(_) = result {
        if !r.storage_view.snapshot.failed {
            r.storage_view.snapshot.failed = true;
            r.storage_view.revision += 1;
        }
        r.storage_view.next = now.saturating_add(2);
        r.storage_view.phase = 0;
        return;
    }
    let reply = result.unwrap();
    if phase == 0 {
        let mut nodes = [[0; 32]; 32];
        let mut count = 0;
        let mut capacity = 0u64;
        let mut eligible = 0u64;
        let mut reserved = 0u64;
        for (index, resource) in r
            .fabric_resources
            .entries()
            .iter()
            .enumerate()
            .filter_map(|(i, x)| x.map(|x| (i, x)))
        {
            let s = &mut r.storage_view.staged;
            let position = s
                .node_rows
                .iter()
                .position(|n| n.is_some_and(|n| n.id == resource.owner.0))
                .unwrap_or(s.node_count);
            if position < 32 {
                let row = s.node_rows[position].get_or_insert(NodeSummary {
                    id: resource.owner.0,
                    online: false,
                    capacity: 0,
                    reserved: 0,
                });
                row.online |= resource.online && resource.expires > now;
                row.capacity = row.capacity.saturating_add(resource.capacity);
                row.reserved = row.reserved.saturating_add(resource.reserved);
                s.node_count = s.node_count.max(position + 1);
            }
            if resource.online && resource.expires > now {
                capacity = capacity.saturating_add(resource.capacity);
                eligible = eligible.saturating_add(r.fabric_resources.usable(index, now));
                reserved = reserved.saturating_add(resource.reserved);
                if !nodes[..count].contains(&resource.owner.0) {
                    nodes[count] = resource.owner.0;
                    count += 1;
                }
            }
        }
        // Before directory initialization, the local service observation is still real.
        if count == 0 && reply.length == 48 {
            capacity = reply.object_version;
            eligible = reply.offset;
            reserved = u64::from_le_bytes(reply.data[32..40].try_into().unwrap());
            count = 1;
            if let Some(local) = r.nodes.local_id() {
                let s = &mut r.storage_view.staged;
                let position = s
                    .node_rows
                    .iter()
                    .position(|n| n.is_some_and(|n| n.id == local.0))
                    .unwrap_or(s.node_count);
                if position < 32 {
                    s.node_rows[position] = Some(NodeSummary {
                        id: local.0,
                        online: true,
                        capacity,
                        reserved,
                    });
                    s.node_count = s.node_count.max(position + 1);
                }
            }
        }
        let s = &mut r.storage_view.staged;
        s.capacity = capacity;
        s.eligible = eligible;
        s.reserved = reserved;
        s.nodes = count;
    } else if phase <= 8 {
        let s = &mut r.storage_view.staged;
        s.count = (reply.value as usize).min(8);
        if reply.length == 64 {
            s.objects[phase - 1] = Some(ObjectSummary {
                id: reply.data[..16].try_into().unwrap(),
                version: reply.object_version,
                generation: reply.manifest_generation,
                authority: reply.authority_generation,
                bytes: u64::from_le_bytes(reply.data[56..64].try_into().unwrap()),
                desired: reply.data[48],
                verified: reply.data[49],
                offline: reply.data[50],
                stale: reply.data[51],
                corrupt: reply.data[52],
                healing: reply.data[53] != 0,
            });
        }
    } else if reply.length == 64 {
        r.storage_view.staged.placements[phase - 9] = Some(PlacementSummary {
            node: reply.data[..32].try_into().unwrap(),
            resource: reply.data[32..48].try_into().unwrap(),
            device: reply.data[48..64].try_into().unwrap(),
            state: reply.value as u8,
            version: reply.object_version,
        });
    }
    if phase == 16 || (phase > 0 && r.storage_view.staged.count == 0) {
        finish_local(r, now);
    } else if phase <= 8 && phase > 0 && (phase == 8 || phase >= r.storage_view.staged.count) {
        let v = &mut r.storage_view;
        v.staged.selected = v.staged.selected.min(v.staged.count.saturating_sub(1));
        v.phase = 9;
    } else {
        r.storage_view.phase += 1;
    }
}
// ------------------------=
// FUNC: finish_local
// DESC: Defers publication until authorized shared metadata has been coherently observed when its native service exists.
// ------------------=
fn finish_local(r: &mut InfinityRuntime, now: u64) {
    if r.storage_metadata.handler.is_some() {
        shared::begin(r);
        r.storage_view.phase = 17;
    } else {
        publish(r, now);
    }
}
// ------------------------=
// FUNC: publish
// DESC: Publishes a complete fixed projection only when observed values change, avoiding periodic full redraw on unchanged state.
// ------------------=
fn publish(r: &mut InfinityRuntime, now: u64) {
    let v = &mut r.storage_view;
    v.staged.ready = true;
    v.staged.selected = v.staged.selected.min(v.staged.count.saturating_sub(1));
    v.staged.selected_node = v
        .staged
        .selected_node
        .min(v.staged.node_count.saturating_sub(1));
    if v.staged.placements[v.staged.selected_placement].is_none() {
        v.staged.selected_placement = v
            .staged
            .placements
            .iter()
            .position(Option::is_some)
            .unwrap_or(0);
    }
    if v.snapshot != v.staged {
        v.snapshot = v.staged;
        v.revision += 1;
    }
    v.phase = 0;
    v.next = now.saturating_add(if r.storage_metadata.handler.is_some() {
        30
    } else {
        2
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static CALLS: AtomicU64 = AtomicU64::new(0);
    static POLICY: AtomicU64 = AtomicU64::new(2);
    static GENERATION: AtomicU64 = AtomicU64::new(1);
    // ------------------------=
    // FUNC: authoritative_fixture
    // DESC: Supplies typed service state and generation-fenced policy transitions; this is a host fixture, not installed storage evidence.
    // ------------------=
    fn authoritative_fixture(
        request: iop::remote::AuthenticatedStorageRequest,
    ) -> Result<
        (
            StorageOperationV1,
            Option<iop::storage_protocol::StorageCommit>,
        ),
        iop::remote::RemoteError,
    > {
        use iop::remote::RemoteError;
        assert_ne!(request.grant, 0);
        assert_ne!(request.peer.0, [0; 32]);
        CALLS.fetch_add(1, Ordering::SeqCst);
        let mut p = request.payload;
        p.data = [0; 64];
        match p.operation {
            Operation::ResourceInspect => {
                p.length = 48;
                p.object_version = 65536;
                p.offset = 32768;
                p.data[..16].fill(7);
                p.data[16..32].fill(8);
            }
            Operation::PoolInspect if p.value == 1 => {
                p.value = 1;
                p.length = 64;
                p.data[..16].fill(4);
                p.object_version = 2;
                p.manifest_generation = GENERATION.load(Ordering::SeqCst);
                p.authority_generation = 1;
                p.data[48] = POLICY.load(Ordering::SeqCst) as u8;
                p.data[49] = 1;
                p.data[50] = 1;
                p.data[56..64].copy_from_slice(&50000u64.to_le_bytes());
            }
            Operation::PoolInspect if p.value == 2 => {
                if p.offset == 0 {
                    p.length = 64;
                    p.data[..32].copy_from_slice(&request.peer.0);
                    p.data[32..48].fill(7);
                    p.data[48..64].fill(8);
                    p.value = 2;
                    p.object_version = 2;
                } else {
                    p.length = 0;
                }
            }
            Operation::ObjectSetPolicy => {
                if p.manifest_generation != GENERATION.load(Ordering::SeqCst) {
                    return Err(RemoteError::Conflict);
                }
                POLICY.store(p.value, Ordering::SeqCst);
                p.manifest_generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
                p.length = 0;
            }
            _ => return Err(RemoteError::UnsupportedOperation),
        }
        Ok((p, None))
    }
    // ------------------------=
    // FUNC: projection_uses_typed_policy_parity_and_bounded_refresh
    // DESC: Verifies one-operation ticks, GUI-policy/Console-observation agreement, unchanged refresh suppression and session-revocation privacy.
    // ------------------=
    #[test]
    fn projection_uses_typed_policy_parity_and_bounded_refresh() {
        CALLS.store(0, Ordering::SeqCst);
        POLICY.store(2, Ordering::SeqCst);
        GENERATION.store(1, Ordering::SeqCst);
        let mut r = InfinityRuntime::new(false);
        r.define_bootstrap().unwrap();
        r.start_all(0);
        r.nodes.initialize(&[63; 32], true).unwrap();
        r.node_clock = Some(10);
        r.identity.create_machine(b"pool-view", 64, 1, 0).unwrap();
        let user = r
            .identity
            .create_user(b"operator", b"Operator", 0)
            .unwrap()
            .id;
        r.identity.create_password(user, b"Fixture901", 0).unwrap();
        let session = r
            .identity
            .create_session(user, b"Fixture901", 1)
            .unwrap()
            .id;
        r.storage_handler = Some(authoritative_fixture);
        r.start_all(10);
        r.storage_view.owner = Some((user, session));
        for _ in 0..12 {
            let before = CALLS.load(Ordering::SeqCst);
            poll(&mut r, 10);
            assert!(CALLS.load(Ordering::SeqCst) - before <= 1);
        }
        assert!(r.storage_view.snapshot.ready);
        assert_eq!(r.storage_view.snapshot.count, 1);
        assert_eq!(r.storage_view.snapshot.objects[0].unwrap().bytes, 50000);
        assert_eq!(r.storage_view.snapshot.placements[0].unwrap().state, 2);
        assert_eq!(r.storage_view.snapshot.node_count, 1);
        let rev = r.storage_view.revision;
        for _ in 0..12 {
            poll(&mut r, 12);
        }
        assert_eq!(r.storage_view.revision, rev);
        r.storage_view.queue_policy(3);
        poll(&mut r, 13);
        assert_eq!(POLICY.load(Ordering::SeqCst), 3);
        for _ in 0..12 {
            poll(&mut r, 13);
        }
        let gui = r.storage_view.snapshot.objects[0].unwrap();
        assert_eq!(gui.desired, 3);
        assert_eq!(gui.generation, 2);
        let query = StorageOperationV1 {
            operation: Operation::PoolInspect,
            object: [0; 16],
            authority_generation: 0,
            manifest_generation: 0,
            object_version: 0,
            offset: 0,
            scope: 0,
            value: 1,
            length: 0,
            data: [0; 64],
        };
        let console = storage_client::perform(&mut r, 13, query).unwrap();
        assert_eq!(console.data[48], gui.desired);
        assert_eq!(console.manifest_generation, gui.generation);
        // The service advances with no delivered event: periodic typed reads
        // reconstruct the projection without a restart or repeated redraw.
        POLICY.store(2, Ordering::SeqCst);
        GENERATION.store(3, Ordering::SeqCst);
        for _ in 0..12 {
            let before = CALLS.load(Ordering::SeqCst);
            poll(&mut r, 16);
            assert!(CALLS.load(Ordering::SeqCst) - before <= 1);
        }
        assert_eq!(r.storage_view.snapshot.objects[0].unwrap().desired, 2);
        assert_eq!(r.storage_view.snapshot.objects[0].unwrap().generation, 3);
        let recovered = r.storage_view.revision;
        for _ in 0..12 {
            poll(&mut r, 18);
        }
        assert_eq!(r.storage_view.revision, recovered);
        let calls = CALLS.load(Ordering::SeqCst);
        r.identity.lock_session(session, user).unwrap();
        r.storage_view.queue_policy(1);
        poll(&mut r, 14);
        assert_eq!(CALLS.load(Ordering::SeqCst), calls);
        assert!(r.storage_view.snapshot == Snapshot::empty());
        assert!(r.storage_view.policy.is_none());
        assert_eq!(POLICY.load(Ordering::SeqCst), 2);
    }
}
