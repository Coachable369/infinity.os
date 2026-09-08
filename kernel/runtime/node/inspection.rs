//! Stateless, bounded pages over authoritative public records. No secret key material is encoded.
use super::*;
use super::super::iop::{IopError, NodeOperationV1, OperationId};

pub const PAGE_BYTES: usize = 24;
pub const NODE_DETAIL_BYTES: usize = 128;
pub const SESSION_DETAIL_BYTES: usize = 96;
pub const DOMAIN_DETAIL_BYTES: usize = 104;

// ------------------------=
// FUNC: inspect
// DESC: Selects an exact authoritative record and returns a checkpoint-bound page within the existing 80-byte IOP payload.
// ------------------=
pub fn inspect(nodes: &NodeRuntime, operation: OperationId, request: NodeOperationV1) -> Result<NodeOperationV1, IopError> {
    let mut data = [0u8; 640];
    let length = record(nodes, operation, request, &mut data)?;
    page(operation, request, &data[..length])
}

// ------------------------=
// FUNC: record
// DESC: Encodes one bounded public record without selecting a continuation or copying secrets.
// ------------------=
fn record(nodes: &NodeRuntime, operation: OperationId, request: NodeOperationV1, data: &mut [u8; 640]) -> Result<usize, IopError> {
    let length = match operation {
        OperationId::NodeList => {
            let mut count = 0;
            for node in nodes.discovered.iter().flatten() { data[count * 32..(count + 1) * 32].copy_from_slice(&node.id.0); count += 1; }
            count * 32
        }
        OperationId::NodeSessionList => {
            let mut count = 0;
            for session in nodes.sessions.iter().flatten() {
                let at = count * 40; data[at..at + 32].copy_from_slice(&session.peer.0); data[at + 32..at + 40].copy_from_slice(&session.id.to_le_bytes()); count += 1;
            }
            count * 40
        }
        OperationId::NodeDomainList => {
            let local = nodes.local_id.ok_or(IopError::InvalidPayload)?;
            let mut count = 0;
            for domain in nodes.domains.iter().flatten() { data[count * 32..(count + 1) * 32].copy_from_slice(&membership::domain_id(local, domain.peer).0); count += 1; }
            count * 32
        }
        OperationId::NodeDomainInspect => {
            let local = nodes.local_id.ok_or(IopError::InvalidPayload)?;
            let domain = nodes.domains.iter().flatten().find(|domain| membership::domain_id(local, domain.peer).0 == request.node_id).ok_or(IopError::InvalidPayload)?;
            let (a, b) = if local.0 < domain.peer.0 { (local, domain.peer) } else { (domain.peer, local) };
            data[..32].copy_from_slice(&a.0); data[32..64].copy_from_slice(&b.0);
            data[64..72].copy_from_slice(&domain.revision.to_le_bytes()); data[72..80].copy_from_slice(&domain.pending_revision.to_le_bytes());
            data[80] = domain.active as u8; data[81] = domain.approved_join as u8; data[82] = domain.desired as u8;
            data[88..96].copy_from_slice(&domain.correlation.to_le_bytes()); data[96..104].copy_from_slice(&nodes.control_version.to_le_bytes());
            DOMAIN_DETAIL_BYTES
        }
        OperationId::NodeInspect => {
            let node = nodes.discovered.iter().flatten().find(|node| node.id.0 == request.node_id).ok_or(IopError::InvalidPayload)?;
            data[..32].copy_from_slice(&node.id.0);
            data[32..64].copy_from_slice(&node.public_key);
            data[64..66].copy_from_slice(&node.protocol_min.to_le_bytes());
            data[66..68].copy_from_slice(&node.protocol_max.to_le_bytes());
            data[68..76].copy_from_slice(&node.service_bits.to_le_bytes());
            data[76..84].copy_from_slice(&node.last_seen.to_le_bytes());
            data[84] = reachability_to_u8(node.reachability);
            data[85] = trust_to_u8(node.trust);
            for index in 0..12 { data[86 + index] = decision_to_u8(node.policy.categories[index]); }
            data[98..106].copy_from_slice(&node.policy.scope.to_le_bytes());
            data[106..114].copy_from_slice(&node.policy.expires_at.to_le_bytes());
            data[114..118].copy_from_slice(&node.policy.version.to_le_bytes());
            data[118..126].copy_from_slice(&nodes.control_version.to_le_bytes());
            NODE_DETAIL_BYTES
        }
        OperationId::NodeSessionInspect => {
            // Read-selector occupies lease_deadline; policy scope remains an independent authorization restriction.
            let session = nodes.sessions.iter().flatten().find(|session| session.id == request.lease_deadline && session.peer.0 == request.node_id).ok_or(IopError::InvalidPayload)?;
            data[..32].copy_from_slice(&session.peer.0);
            data[32..40].copy_from_slice(&session.id.to_le_bytes());
            data[40..56].copy_from_slice(&session.protocol_reference);
            data[56] = match session.state { SessionState::Handshaking => 0, SessionState::Established => 1, SessionState::RekeyRequired => 2, SessionState::Closed => 3, SessionState::Failed => 4 };
            data[64..72].copy_from_slice(&session.send_sequence.to_le_bytes());
            data[72..80].copy_from_slice(&session.receive_sequence.to_le_bytes());
            data[80..88].copy_from_slice(&session.expires_at.to_le_bytes());
            data[88..96].copy_from_slice(&nodes.control_version.to_le_bytes());
            SESSION_DETAIL_BYTES
        }
        _ => return Err(IopError::InvalidPayload),
    };
    Ok(length)
}

#[derive(Clone, Copy)]
struct Snapshot {
    principal: [u8; 16], selected: [u8; 32], operation: OperationId,
    selector: u64, scope: u64, checkpoint: u64, expires: u64,
    token: u64, length: usize, bytes: [u8; 640],
}

pub struct InspectionCache { slots: [Option<Snapshot>; 4] }
impl InspectionCache {
    // ------------------------=
    // FUNC: new
    // DESC: Bounds concurrent remote inspection snapshots to four public records and no dynamic allocation.
    // ------------------=
    pub const fn new() -> Self { Self { slots: [None; 4] } }

    // ------------------------=
    // FUNC: inspect
    // DESC: Pins public telemetry across pages, bound to the authenticated session, selected object, policy scope, checkpoint and a 60-second expiry.
    // ------------------=
    pub fn inspect(&mut self, nodes: &NodeRuntime, principal: [u8; 16], operation: OperationId, request: NodeOperationV1, now: u64) -> Result<NodeOperationV1, IopError> {
        for slot in &mut self.slots { if slot.as_ref().map(|s| now >= s.expires || s.checkpoint != nodes.control_version()).unwrap_or(false) { *slot = None; } }
        if request.handle == 0 && request.flags == 0 {
            let mut bytes = [0; 640]; let length = record(nodes, operation, request, &mut bytes)?;
            let first = page(operation, request, &bytes[..length])?;
            let index = self.slots.iter().position(|s| s.as_ref().map(|s| s.principal == principal && s.selected == request.node_id && s.operation == operation).unwrap_or(true)).ok_or(IopError::Backpressure)?;
            self.slots[index] = Some(Snapshot { principal, selected: request.node_id, operation, selector: request.lease_deadline, scope: request.scope, checkpoint: nodes.control_version(), expires: now.saturating_add(60), token: first.handle, length, bytes });
            return Ok(first);
        }
        let snapshot = self.slots.iter().flatten().find(|s| s.principal == principal && s.selected == request.node_id && s.operation == operation && s.selector == request.lease_deadline && s.scope == request.scope && s.token == request.handle).ok_or(IopError::InvalidPayload)?;
        page(operation, request, &snapshot.bytes[..snapshot.length])
    }
}

// ------------------------=
// FUNC: page
// DESC: Rejects stale and misaligned continuations and encodes one exact, zero-padded public-record page.
// ------------------=
fn page(operation: OperationId, request: NodeOperationV1, data: &[u8]) -> Result<NodeOperationV1, IopError> {
    let offset = request.flags as usize;
    if (offset >= data.len() && !(offset == 0 && data.is_empty())) || offset % PAGE_BYTES != 0 { return Err(IopError::InvalidPayload); }
    let mut hash = Sha256::new();
    hash.update(b"InfinityOS inspection snapshot v1");
    hash.update(operation.machine_id().to_le_bytes());
    hash.update(request.node_id);
    hash.update(data);
    let digest = hash.finalize();
    let token = u64::from_le_bytes(digest[..8].try_into().unwrap()) | 1;
    if (offset != 0 || request.handle != 0) && request.handle != token { return Err(IopError::InvalidPayload); }
    let mut bytes = [0; PAGE_BYTES];
    let count = PAGE_BYTES.min(data.len() - offset);
    bytes[..count].copy_from_slice(&data[offset..offset + count]);
    Ok(NodeOperationV1 {
        node_id: request.node_id, operation: operation.machine_id(), schema_version: 1,
        handle: token,
        scope: u64::from_le_bytes(bytes[..8].try_into().unwrap()),
        lease_deadline: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
        rights: u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
        value: u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
        flags: ((data.len() as u32) << 16) | offset as u32,
    })
}

// ------------------------=
// FUNC: page_data
// DESC: Reconstructs the binary page body without interpreting rendered copy or native ABI layout.
// ------------------=
pub fn page_data(response: NodeOperationV1) -> [u8; PAGE_BYTES] {
    let mut data = [0; PAGE_BYTES];
    data[..8].copy_from_slice(&response.scope.to_le_bytes());
    data[8..16].copy_from_slice(&response.lease_deadline.to_le_bytes());
    data[16..20].copy_from_slice(&response.rights.to_le_bytes());
    data[20..24].copy_from_slice(&response.value.to_le_bytes());
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------=
    // FUNC: request
    // DESC: Creates an exact selected-object request with an initial empty continuation.
    // ------------------=
    fn request(peer: NodeId, operation: OperationId) -> NodeOperationV1 {
        NodeOperationV1 { node_id: peer.0, operation: operation.machine_id(), schema_version: 1, handle: 0, scope: 0, lease_deadline: 0, flags: 0, rights: 0, value: 0 }
    }

    // ------------------------=
    // FUNC: inspection_pages_reconstruct_exact_node_and_reject_stale_cursor
    // DESC: Reconstructs every page, verifies identity and policy fields, and tests malformed and stale continuation boundaries.
    // ------------------=
    #[test]
    fn inspection_pages_reconstruct_exact_node_and_reject_stale_cursor() {
        let mut local = NodeRuntime::new();
        local.initialize(&[41; 32], true).unwrap();
        let mut peer = NodeRuntime::new();
        let id = peer.initialize(&[42; 32], true).unwrap();
        local.discover(peer.advertise(0x1234, 1, 1).unwrap(), 1).unwrap();
        let mut query = request(id, OperationId::NodeInspect);
        let first = inspect(&local, OperationId::NodeInspect, query).unwrap();
        let mut complete = [0; NODE_DETAIL_BYTES];
        for offset in (0..NODE_DETAIL_BYTES).step_by(PAGE_BYTES) {
            query.flags = offset as u32;
            query.handle = first.handle;
            let result = inspect(&local, OperationId::NodeInspect, query).unwrap();
            assert_eq!(result.encode().len(), 80);
            assert_eq!(result.flags >> 16, NODE_DETAIL_BYTES as u32);
            assert_eq!(result.flags & 0xffff, offset as u32);
            let count = PAGE_BYTES.min(NODE_DETAIL_BYTES - offset);
            complete[offset..offset + count].copy_from_slice(&page_data(result)[..count]);
            assert!(page_data(result)[count..].iter().all(|byte| *byte == 0));
        }
        assert_eq!(&complete[..32], &id.0);
        assert_eq!(&complete[32..64], &peer.crypto.public_identity().unwrap());
        assert_eq!(u64::from_le_bytes(complete[68..76].try_into().unwrap()), 0x1234);
        assert_eq!(complete[85], trust_to_u8(TrustState::Untrusted));
        let mut restored = NodeRuntime::new();
        restored.restore_state(&local.encode_state().unwrap()).unwrap();
        assert_eq!(inspect(&restored, OperationId::NodeInspect, query), inspect(&local, OperationId::NodeInspect, query));
        for offset in [1, 23, 128, 144, u32::MAX] {
            query.flags = offset;
            assert!(inspect(&local, OperationId::NodeInspect, query).is_err());
        }
        query.flags = 24;
        query.handle = 0;
        assert!(inspect(&local, OperationId::NodeInspect, query).is_err());
        query.handle = first.handle;
        local.set_trust(id, TrustState::Blocked, 3, 9).unwrap();
        assert!(inspect(&local, OperationId::NodeInspect, query).is_err());
        query = request(NodeId([99; 32]), OperationId::NodeInspect);
        assert!(inspect(&local, OperationId::NodeInspect, query).is_err());
    }

    // ------------------------=
    // FUNC: inspection_session_is_selected_and_never_exposes_keys
    // DESC: Reconstructs selected session metadata, rejects another peer and handles session disappearance after restore.
    // ------------------=
    #[test]
    fn inspection_session_is_selected_and_never_exposes_keys() {
        let mut local = NodeRuntime::new();
        local.initialize(&[61; 32], true).unwrap();
        let mut peer = NodeRuntime::new();
        let id = peer.initialize(&[62; 32], true).unwrap();
        local.discover(peer.advertise(1, 1, 1).unwrap(), 1).unwrap();
        let pair = local.begin_pairing(id, 2).unwrap();
        local.confirm_pairing(pair.id, pair.verification_code, true, 3, 1).unwrap();
        let (_, public) = NodeCrypto::agreement_keypair(&[64; 32]);
        let session = local.open_session(id, &[63; 32], &public, b"inspect", 4, 1).unwrap();
        let mut query = request(id, OperationId::NodeSessionInspect);
        query.lease_deadline = session;
        let mut data = [0; SESSION_DETAIL_BYTES];
        for offset in (0..SESSION_DETAIL_BYTES).step_by(PAGE_BYTES) {
            query.flags = offset as u32;
            let response = inspect(&local, OperationId::NodeSessionInspect, query).unwrap();
            query.handle = response.handle;
            data[offset..offset + PAGE_BYTES].copy_from_slice(&page_data(response));
        }
        assert_eq!(&data[..32], &id.0);
        assert_eq!(u64::from_le_bytes(data[32..40].try_into().unwrap()), session);
        assert_eq!(&data[40..56], &local.sessions[0].unwrap().protocol_reference);
        assert_eq!(data[56], 1);
        assert!(data[57..64].iter().all(|byte| *byte == 0));
        assert_eq!(u64::from_le_bytes(data[80..88].try_into().unwrap()), local.sessions[0].unwrap().expires_at);
        query.node_id = [99; 32];
        assert!(inspect(&local, OperationId::NodeSessionInspect, query).is_err());
        query.node_id = id.0;
        local.restore_state(&local.encode_state().unwrap()).unwrap();
        assert!(inspect(&local, OperationId::NodeSessionInspect, query).is_err());
    }
}
