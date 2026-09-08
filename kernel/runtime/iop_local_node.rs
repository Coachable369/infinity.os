//! Local human clients use the ordinary bounded router and the same durable service as remote IOP.
use super::*;
use super::super::node::{control::CommittedControl, types::NODE_STATE_BYTES, NodeRuntime};

impl IopRouter {
    // ------------------------=
    // FUNC: poll_membership
    // DESC: Processes at most one authenticated membership frame and one durable retry per second; no frame-loop disk scans or UI redraws.
    // ------------------=
    pub fn poll_membership(&mut self, nodes: &mut NodeRuntime, wire: &mut super::super::node::wire_trust::WireTrust, now: u64, mut persist: impl FnMut(&[u8; NODE_STATE_BYTES]) -> bool) -> Option<CommittedControl> {
        use super::super::node::membership::{Message, MAGIC, MAX_DOMAINS};
        if self.membership_tick == Some(now) { return None; }
        self.membership_tick = Some(now);
        let mut notice = None;
        if let Some(frame) = wire.receive_protocol(MAGIC) {
            // receive_protocol carries the authenticated peer and current session reference.
            let live = nodes.sessions().iter().flatten().any(|session| session.peer == frame.peer && session.protocol_reference == frame.reference && session.state == super::super::node::types::SessionState::Established && now < session.expires_at);
            if live {
                if let Ok(message) = Message::decode(&frame.bytes[..frame.length]) {
                    if let Ok((reply, committed)) = nodes.receive_domain(frame.peer, message, now, |bytes| persist(bytes)) {
                        notice = committed;
                        if let Some(reply) = reply { let _ = wire.send_data(nodes, frame.peer, &reply.encode(), false, now); }
                    }
                }
            }
        }
        let index = self.membership_cursor;
        self.membership_cursor = (index + 1) % MAX_DOMAINS;
        if let Some((peer, message)) = nodes.domain_proposal(index, now) { let _ = wire.send_data(nodes, peer, &message.encode(), false, now); }
        notice
    }
    // ------------------------=
    // FUNC: next_node_request
    // DESC: Allocates a checked router-owned local correlation identity without reusing remote request IDs.
    // ------------------=
    pub fn next_node_request(&mut self) -> Result<u64, IopError> {
        let id = self.next_local_request;
        self.next_local_request = id.checked_add(1).ok_or(IopError::Backpressure)?;
        Ok(id)
    }
    // ------------------------=
    // FUNC: ensure_owned_endpoint
    // DESC: Reuses only an endpoint owned by the exact identity; endpoint IDs cannot redirect replies to another client.
    // ------------------=
    pub fn ensure_owned_endpoint(&mut self, id: u16, owner: SecurityIdentity) -> Result<(), IopError> {
        match self.endpoints.iter().flatten().find(|endpoint| endpoint.id == id) {
            Some(endpoint) if endpoint.owner == owner => Ok(()),
            Some(_) => Err(IopError::AccessDenied),
            None => self.register_endpoint(id, owner),
        }
    }

    // ------------------------=
    // FUNC: dispatch_durable_node
    // DESC: Revalidates dequeued authority and reserves the owned reply slot before a durable commit; no direct-mutation fallback exists.
    // ------------------=
    pub fn dispatch_durable_node(
        &mut self, capabilities: &CapabilityManager, nodes: &mut NodeRuntime,
        operation: OperationId, service_endpoint: u16, response_endpoint: u16,
        service: SecurityIdentity, now: u64,
        persist: impl FnOnce(&[u8; NODE_STATE_BYTES]) -> bool,
    ) -> Result<(NodeOperationV1, CommittedControl), IopError> {
        self.dispatch_node_transaction(capabilities, nodes, operation, service_endpoint, response_endpoint, service, now, |nodes, request, correlation, causation| nodes.commit_control(operation, request, now, correlation, causation, persist))
    }

    // ------------------------=
    // FUNC: dispatch_node_transaction
    // DESC: Owns the shared admission, execution-time authority and reply-capacity boundary for durable node and wire transactions.
    // ------------------=
    pub fn dispatch_node_transaction(
        &mut self, capabilities: &CapabilityManager, nodes: &mut NodeRuntime,
        operation: OperationId, service_endpoint: u16, response_endpoint: u16,
        service: SecurityIdentity, now: u64,
        commit: impl FnOnce(&mut NodeRuntime, NodeOperationV1, u64, u64) -> Result<(NodeOperationV1, CommittedControl), super::super::node::control::CommitError>,
    ) -> Result<(NodeOperationV1, CommittedControl), IopError> {
        if !self.endpoints.iter().flatten().any(|endpoint| endpoint.id == service_endpoint && endpoint.owner == service) { return Err(IopError::AccessDenied); }
        let message = self.receive(service_endpoint, now)?;
        if self.is_cancelled(message.header.request_id) { return Err(IopError::Cancelled); }
        if message.header.message_type != MessageType::Request || message.header.operation_type_id != operation.machine_id() { return Err(IopError::InvalidPayload); }
        let request = NodeOperationV1::decode(message.bytes())?;
        if request.operation != operation.machine_id() { return Err(IopError::InvalidPayload); }
        capabilities.validate(message.header.capability_ref, message.header.caller_identity, CapabilityType::ServiceCall, operation.machine_id() as u64, 1, 0, now)?;
        let reply = self.endpoints.iter().flatten().find(|endpoint| endpoint.id == response_endpoint).ok_or(IopError::UnknownEndpoint)?;
        if reply.owner != message.header.caller_identity { return Err(IopError::AccessDenied); }
        if reply.len == ENDPOINT_QUEUE_CAPACITY { return Err(IopError::Backpressure); }
        let (response, notice) = commit(nodes, request, message.header.correlation_id, message.header.request_id).map_err(|error| match error {
            super::super::node::control::CommitError::PersistenceFailed => IopError::PersistenceFailed,
            _ => IopError::InvalidPayload,
        })?;
        // The synchronous service holds exclusive router access: the reserved slot cannot be stolen.
        self.respond(response_endpoint, &message, service, &response.encode(), now)?;
        Ok((response, notice))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------=
    // FUNC: local_router
    // DESC: Builds an explicit capability and authenticated test peer for the real local router path.
    // ------------------=
    fn local_router() -> (IopRouter, CapabilityManager, NodeRuntime, NodeOperationV1, u64) {
        let mut router = IopRouter::new();
        router.register_endpoint(1, SecurityIdentity([1; 16])).unwrap();
        router.register_endpoint(2, SecurityIdentity([2; 16])).unwrap();
        let mut nodes = NodeRuntime::new();
        nodes.initialize(&[11; 32], true).unwrap();
        let mut peer = NodeRuntime::new();
        let id = peer.initialize(&[12; 32], true).unwrap();
        nodes.discover(peer.advertise(1, 1, 1).unwrap(), 1).unwrap();
        let pair = nodes.begin_pairing(id, 2).unwrap();
        nodes.confirm_pairing(pair.id, pair.verification_code, true, 3, 1).unwrap();
        let op = OperationId::NodePolicyUpdate;
        let mut caps = CapabilityManager::new();
        let cap = caps.grant(CapabilityType::ServiceCall, op.machine_id() as u64, 1, 0, SecurityIdentity([2; 16]), SecurityIdentity([1; 16]), Some(100), 0).unwrap();
        let request = NodeOperationV1 { node_id: id.0, handle: 0, scope: 42, lease_deadline: 99, operation: op.machine_id(), rights: 0, value: 2, flags: 4, schema_version: 1 };
        let message = IopMessage::request(op, 55, SecurityIdentity([1; 16]), cap, 100, 88, &request.encode()).unwrap();
        router.send(2, message, &caps, 4).unwrap();
        (router, caps, nodes, request, cap)
    }

    // ------------------------=
    // FUNC: durable_local_router_commit_is_visible_to_reads_and_restart
    // DESC: Verifies local IOP commit, causal response and checkpoint notice, authoritative policy read and persisted reconstruction.
    // ------------------=
    #[test]
    fn durable_local_router_commit_is_visible_to_reads_and_restart() {
        let (mut router, mut caps, mut nodes, mut request, cap) = local_router();
        let mut disk = [0; NODE_STATE_BYTES];
        let (_, notice) = router.dispatch_durable_node(&caps, &mut nodes, OperationId::NodePolicyUpdate, 2, 1, SecurityIdentity([2; 16]), 5, |bytes| { disk.copy_from_slice(bytes); true }).unwrap();
        assert_eq!((notice.version, notice.correlation, notice.causation), (1, 88, 55));
        let response = router.receive(1, 5).unwrap();
        assert_eq!((response.header.correlation_id, response.header.causation_id), (88, 55));
        request.operation = OperationId::NodePolicyRead.machine_id();
        let read = execute_node_operation(&mut nodes, OperationId::NodePolicyRead, request, 5, 90).unwrap();
        assert_eq!((read.rights >> 8) & 3, 2);
        assert_eq!((read.scope, read.lease_deadline, read.handle), (42, 99, 1));
        let mut restored = NodeRuntime::new();
        restored.restore_state(&disk).unwrap();
        assert_eq!(execute_node_operation(&mut restored, OperationId::NodePolicyRead, request, 6, 91).unwrap(), read);
        caps.retire_leaf(cap, SecurityIdentity([2; 16])).unwrap();
        assert!(caps.get(cap).is_none());
    }

    // ------------------------=
    // FUNC: durable_local_router_failures_never_commit
    // DESC: Injects revoked authority, missing reply ownership, full reply queues and persistence failure before the commit boundary.
    // ------------------=
    #[test]
    fn durable_local_router_failures_never_commit() {
        for mode in 0..4 {
            let (mut router, mut caps, mut nodes, request, cap) = local_router();
            let before = nodes.encode_state().unwrap();
            if mode == 0 { caps.revoke(cap).unwrap(); }
            if mode == 1 { router.endpoints[0].as_mut().unwrap().owner = SecurityIdentity([9; 16]); }
            if mode == 2 {
                let message = IopMessage::request(OperationId::NodePolicyUpdate, 9, SecurityIdentity([1; 16]), cap, 100, 9, &request.encode()).unwrap();
                for _ in 0..ENDPOINT_QUEUE_CAPACITY { router.endpoints[0].as_mut().unwrap().push(message).unwrap(); }
            }
            let mut writes = 0;
            let result = router.dispatch_durable_node(&caps, &mut nodes, OperationId::NodePolicyUpdate, 2, 1, SecurityIdentity([2; 16]), 5, |_| { writes += 1; false });
            assert!(result.is_err());
            assert_eq!(writes, if mode == 3 { 1 } else { 0 });
            assert_eq!(nodes.encode_state().unwrap(), before);
            assert_eq!(nodes.control_version(), 0);
        }
    }
}
