//! Bounded subscriber projection reconstructed solely through typed IOP reads.
use super::super::iop::{IopError, NodeOperationV1, OperationId};
use super::{inspection::{page_data, PAGE_BYTES, NODE_DETAIL_BYTES, DOMAIN_DETAIL_BYTES}, membership::MAX_DOMAINS, types::MAX_DISCOVERED_NODES};

#[derive(Clone)]
pub struct Projection {
    pub version: u64,
    pub stale: bool,
    pub gaps: u64,
    pub refreshes: u64,
    pub node_count: usize,
    pub domain_count: usize,
    pub nodes: [[u8; NODE_DETAIL_BYTES]; MAX_DISCOVERED_NODES],
    pub domains: [[u8; DOMAIN_DETAIL_BYTES]; MAX_DOMAINS],
}

impl Projection {
    // ------------------------=
    // FUNC: new
    // DESC: Starts stale so startup/restart always obtains authoritative state, rather than assuming an empty cache is current.
    // ------------------=
    pub const fn new() -> Self { Self { version: 0, stale: true, gaps: 0, refreshes: 0, node_count: 0, domain_count: 0, nodes: [[0; NODE_DETAIL_BYTES]; MAX_DISCOVERED_NODES], domains: [[0; DOMAIN_DETAIL_BYTES]; MAX_DOMAINS] } }

    // ------------------------=
    // FUNC: observe
    // DESC: Detects service-checkpoint gaps independently of unrelated IEF topic sequences and never treats events as authoritative state.
    // ------------------=
    pub fn observe(&mut self, checkpoint: u64) {
        if checkpoint > self.version {
            if checkpoint > self.version.saturating_add(1) { self.gaps = self.gaps.saturating_add(1); }
            self.stale = true;
        }
    }

    // ------------------------=
    // FUNC: reconcile
    // DESC: Rebuilds into a candidate via bounded typed reads and installs it only if the final authoritative checkpoint still matches.
    // ------------------=
    pub fn reconcile(&mut self, mut query: impl FnMut(NodeOperationV1) -> Result<NodeOperationV1, IopError>) -> Result<(), IopError> {
        self.stale = true;
        let before = query(request([0; 32], OperationId::NodeDiscoverStatus))?.scope;
        let mut candidate = Self::new();
        candidate.gaps = self.gaps; candidate.refreshes = self.refreshes.saturating_add(1);
        let mut ids = [0; MAX_DISCOVERED_NODES * 32];
        let size = collect(request([0; 32], OperationId::NodeList), &mut ids, &mut query)?;
        if size % 32 != 0 { return Err(IopError::InvalidPayload); }
        candidate.node_count = size / 32;
        for index in 0..candidate.node_count {
            let mut id = [0; 32]; id.copy_from_slice(&ids[index * 32..(index + 1) * 32]);
            if collect(request(id, OperationId::NodeInspect), &mut candidate.nodes[index], &mut query)? != NODE_DETAIL_BYTES { return Err(IopError::InvalidPayload); }
        }
        let size = collect(request([0; 32], OperationId::NodeDomainList), &mut ids[..MAX_DOMAINS * 32], &mut query)?;
        if size % 32 != 0 { return Err(IopError::InvalidPayload); }
        candidate.domain_count = size / 32;
        for index in 0..candidate.domain_count {
            let mut id = [0; 32]; id.copy_from_slice(&ids[index * 32..(index + 1) * 32]);
            if collect(request(id, OperationId::NodeDomainInspect), &mut candidate.domains[index], &mut query)? != DOMAIN_DETAIL_BYTES { return Err(IopError::InvalidPayload); }
        }
        if query(request([0; 32], OperationId::NodeDiscoverStatus))?.scope != before { return Err(IopError::InvalidPayload); }
        candidate.version = before; candidate.stale = false; *self = candidate;
        Ok(())
    }
}

// ------------------------=
// FUNC: request
// DESC: Constructs a canonical read request, leaving authorization to the caller's IOP broker.
// ------------------=
pub fn request(id: [u8; 32], operation: OperationId) -> NodeOperationV1 {
    NodeOperationV1 { node_id: id, operation: operation.machine_id(), schema_version: 1, handle: 0, scope: 0, lease_deadline: 0, rights: 0, value: 0, flags: 0 }
}

// ------------------------=
// FUNC: collect
// DESC: Enforces selected object, checkpoint, length and offset consistency while assembling an exact bounded record or collection.
// ------------------=
pub fn collect(mut input: NodeOperationV1, output: &mut [u8], query: &mut impl FnMut(NodeOperationV1) -> Result<NodeOperationV1, IopError>) -> Result<usize, IopError> {
    let first = query(input)?;
    let total = (first.flags >> 16) as usize;
    if total > output.len() || first.flags & 0xffff != 0 || first.node_id != input.node_id || first.operation != input.operation { return Err(IopError::InvalidPayload); }
    if total == 0 { return Ok(0); }
    input.handle = first.handle;
    for offset in (0..total).step_by(PAGE_BYTES) {
        input.flags = offset as u32;
        let page = if offset == 0 { first } else { query(input)? };
        if page.handle != first.handle || page.flags != ((total as u32) << 16 | offset as u32) || page.node_id != input.node_id || page.operation != input.operation { return Err(IopError::InvalidPayload); }
        let count = PAGE_BYTES.min(total - offset);
        output[offset..offset + count].copy_from_slice(&page_data(page)[..count]);
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{NodeRuntime, types::{NodeId, NODE_STATE_BYTES}};
    use super::super::super::{iop::{IopRouter, IopMessage, dispatch_node_operation}, capability::{CapabilityManager, CapabilityType}, execution::SecurityIdentity};

    // ------------------------=
    // FUNC: query
    // DESC: Exercises the real bounded read dispatcher with a narrowly scoped capability and consumes its correlated reply.
    // ------------------=
    fn query(nodes: &mut NodeRuntime, request: NodeOperationV1) -> Result<NodeOperationV1, IopError> {
        let operation = [OperationId::NodeList, OperationId::NodeInspect, OperationId::NodeDiscoverStatus, OperationId::NodeDomainList, OperationId::NodeDomainInspect].into_iter().find(|op| op.machine_id() == request.operation).ok_or(IopError::InvalidPayload)?;
        let caller = SecurityIdentity([1; 16]); let service = SecurityIdentity([2; 16]);
        let mut caps = CapabilityManager::new(); let mut router = IopRouter::new();
        router.register_endpoint(1, caller)?; router.register_endpoint(2, service)?;
        let cap = caps.grant(CapabilityType::ServiceCall, operation.machine_id() as u64, 1, 0, service, caller, Some(100), 0)?;
        router.send(2, IopMessage::request(operation, 1, caller, cap, 90, 3, &request.encode())?, &caps, 1)?;
        let result = dispatch_node_operation(&mut router, &caps, nodes, operation, 2, 1, service, 1)?;
        let reply = router.receive(1, 1)?;
        assert_eq!((reply.header.correlation_id, reply.header.causation_id), (3, 1));
        Ok(result)
    }

    // ------------------------=
    // FUNC: lost_checkpoint_and_restart_reconstruct_exact_records
    // DESC: Drops intermediate and final event hints, then restores trust, policy and offline state via authoritative IOP including after service restart.
    // ------------------=
    #[test]
    fn lost_checkpoint_and_restart_reconstruct_exact_records() {
        let mut nodes = NodeRuntime::new(); nodes.initialize(&[31; 32], true).unwrap();
        let mut peer = NodeRuntime::new(); let id = peer.initialize(&[32; 32], true).unwrap();
        let mut disk = [0; NODE_STATE_BYTES];
        nodes.commit_discovery(peer.advertise(1, 1, 1).unwrap(), 1, |_| true).unwrap();
        let mut projection = Projection::new();
        projection.reconcile(|request| query(&mut nodes, request)).unwrap();
        assert_eq!(projection.node_count, 1); assert!(!projection.stale);
        let before = projection.version;
        for operation in [OperationId::NodeBlock, OperationId::NodeUnblock] {
            nodes.commit_control(operation, request(id.0, operation), 2, 11, 12, |bytes| { disk.copy_from_slice(bytes); true }).unwrap();
        }
        // No event was delivered for either commit.
        projection.observe(query(&mut nodes, request([0; 32], OperationId::NodeDiscoverStatus)).unwrap().scope);
        assert!(projection.stale); assert_eq!(projection.gaps, 1); assert_eq!(projection.version, before);
        projection.reconcile(|request| query(&mut nodes, request)).unwrap();
        assert_eq!(projection.version, nodes.control_version());
        let mut restored = NodeRuntime::new(); restored.restore_state(&disk).unwrap();
        let mut restarted = Projection::new();
        restarted.reconcile(|request| query(&mut restored, request)).unwrap();
        assert_eq!(restarted.nodes[0][85..], projection.nodes[0][85..]);
        nodes.commit_offline(id, 5, |_| true).unwrap();
        projection.observe(nodes.control_version());
        projection.reconcile(|request| query(&mut nodes, request)).unwrap();
        assert_eq!(projection.nodes[0][84], super::super::reachability_to_u8(super::super::types::Reachability::Offline));
        assert_ne!(id, NodeId([0; 32]));
    }

    // ------------------------=
    // FUNC: failed_or_inconsistent_refresh_preserves_last_valid_projection
    // DESC: Checks failed read, wrong object and mutation during pagination never install a partial projection or clear staleness.
    // ------------------=
    #[test]
    fn failed_or_inconsistent_refresh_preserves_last_valid_projection() {
        let mut nodes = NodeRuntime::new(); nodes.initialize(&[41; 32], true).unwrap();
        let mut peer = NodeRuntime::new(); let id = peer.initialize(&[42; 32], true).unwrap();
        nodes.discover(peer.advertise(1, 1, 1).unwrap(), 1).unwrap();
        let mut projection = Projection::new(); projection.reconcile(|request| query(&mut nodes, request)).unwrap();
        let before = projection.nodes; let version = projection.version;
        for failure in 0..3 {
            let mut reads = 0;
            let result = projection.reconcile(|input| {
                reads += 1;
                if reads == 4 {
                    if failure == 0 { return Err(IopError::AccessDenied); }
                    if failure == 1 { let mut wrong = query(&mut nodes, input)?; wrong.node_id = [99; 32]; return Ok(wrong); }
                    nodes.commit_control(OperationId::NodeBlock, request(id.0, OperationId::NodeBlock), 2, 1, 1, |_| true).unwrap();
                }
                query(&mut nodes, input)
            });
            assert!(result.is_err()); assert!(projection.stale); assert_eq!(projection.version, version); assert_eq!(projection.nodes, before);
        }
    }
}
