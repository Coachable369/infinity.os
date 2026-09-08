//! Bounded security-journal persistence. Traffic keys and live grants never enter this format.
use super::*;

const AUDIT_OFFSET: usize = MEMBER_RECORD_OFFSET + MAX_MESH_MEMBERS * MEMBER_RECORD_BYTES;
const AUDIT_BYTES: usize = 72;
const RECEIPTS_OFFSET: usize = AUDIT_OFFSET + MAX_AUDIT_RECORDS * AUDIT_BYTES;
const _: () = assert!(AUDIT_OFFSET + MAX_AUDIT_RECORDS * AUDIT_BYTES <= NODE_STATE_BYTES - 4);

impl Drop for NodeRuntime {
    // ------------------------=
    // FUNC: drop
    // DESC: Zeroizes persistent seed copies and ephemeral traffic keys when replacing or discarding a service instance.
    // ------------------=
    fn drop(&mut self) {
        self.identity_seed.zeroize();
        self.clear_ephemeral();
    }
}

impl NodeRuntime {
    // ------------------------=
    // FUNC: commit_discovery
    // DESC: Durably stages new peers and recovered/changed descriptors; unchanged liveness counters remain transient telemetry.
    // ------------------=
    pub(super) fn commit_discovery(&mut self, advertisement: DiscoveryAdvertisement, now: u64, persist: fn(&[u8; NODE_STATE_BYTES]) -> bool) -> Result<(), NodeError> {
        let changed = !self.discovered.iter().flatten().any(|node| node.id == advertisement.node && node.reachability == Reachability::Online && node.service_bits == advertisement.service_bits);
        if !changed { return self.record_discovery(advertisement, now); }
        let mut staged = self.clone();
        staged.record_discovery(advertisement, now)?;
        staged.control_version = staged.control_version.checked_add(1).ok_or(NodeError::ResourceLimit)?;
        staged.record(0xda10, advertisement.node, now, now, 0);
        let bytes = zeroize::Zeroizing::new(staged.encode_state()?);
        if !persist(&bytes) { return Err(NodeError::StateCorrupt); }
        *self = staged;
        Ok(())
    }

    // ------------------------=
    // FUNC: commit_offline
    // DESC: Persists one observed offline transition before changing the live projection or notifying subscribers.
    // ------------------=
    pub(super) fn commit_offline(&mut self, peer: NodeId, now: u64, persist: fn(&[u8; NODE_STATE_BYTES]) -> bool) -> Result<(), NodeError> {
        let mut staged = self.clone();
        let node = staged.discovered.iter_mut().flatten().find(|node| node.id == peer).ok_or(NodeError::UnknownNode)?;
        node.reachability = Reachability::Offline;
        staged.control_version = staged.control_version.checked_add(1).ok_or(NodeError::ResourceLimit)?;
        staged.record(0xda11, peer, now, now, 0);
        let bytes = zeroize::Zeroizing::new(staged.encode_state()?);
        if !persist(&bytes) { return Err(NodeError::StateCorrupt); }
        *self = staged;
        Ok(())
    }
    // ------------------------=
    // FUNC: paired_digest
    // DESC: Returns a public two-party pairing receipt only for a currently trusted exact peer; it is never a traffic key.
    // ------------------=
    pub fn paired_digest(&self, peer: NodeId) -> Option<[u8; 32]> {
        let index = self.discovered.iter().position(|node| node.map(|node| node.id == peer && node.trust == TrustState::Trusted).unwrap_or(false))?;
        (self.paired_digests[index] != [0; 32]).then_some(self.paired_digests[index])
    }

    // ------------------------=
    // FUNC: encode_pairing_receipts
    // DESC: Stores confirmed public transcript digests in canonical peer-record order, excluding live offers and ephemeral secrets.
    // ------------------=
    pub(super) fn encode_pairing_receipts(&self, output: &mut [u8; NODE_STATE_BYTES]) {
        for (index, node) in self.discovered.iter().flatten().enumerate() {
            if let Some(digest) = self.paired_digest(node.id) {
                let at = RECEIPTS_OFFSET + index * 32;
                output[at..at + 32].copy_from_slice(&digest);
            }
        }
    }

    // ------------------------=
    // FUNC: decode_pairing_receipts
    // DESC: Restores only receipts bound to persisted trusted peer records, never sessions or approval in progress.
    // ------------------=
    pub(super) fn decode_pairing_receipts(&mut self, input: &[u8]) -> Result<(), NodeError> {
        for index in 0..MAX_DISCOVERED_NODES {
            let at = RECEIPTS_OFFSET + index * 32;
            let mut digest = [0; 32];
            digest.copy_from_slice(&input[at..at + 32]);
            if digest != [0; 32] && !self.discovered[index].map(|node| node.trust == TrustState::Trusted).unwrap_or(false) { return Err(NodeError::StateCorrupt); }
            self.paired_digests[index] = digest;
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: commit_wire_pairing
    // DESC: Stages confirmed two-party trust and its public receipt, durably commits them together, then installs live state.
    // ------------------=
    pub(super) fn commit_wire_pairing(&mut self, pairing: u64, code: u32, digest: [u8; 32], now: u64, correlation: u64, persist: fn(&[u8; NODE_STATE_BYTES]) -> bool) -> Result<(), NodeError> {
        let mut staged = self.clone();
        staged.confirm_pairing(pairing, code, true, now, correlation)?;
        let peer = staged.pairings.iter().flatten().find(|entry| entry.id == pairing).ok_or(NodeError::PairingNotFound)?.peer;
        let index = staged.discovered.iter().position(|entry| entry.map(|node| node.id == peer).unwrap_or(false)).ok_or(NodeError::UnknownNode)?;
        staged.paired_digests[index] = digest;
        staged.control_version = staged.control_version.checked_add(1).ok_or(NodeError::ResourceLimit)?;
        let bytes = zeroize::Zeroizing::new(staged.encode_state()?);
        if !persist(&bytes) { return Err(NodeError::StateCorrupt); }
        *self = staged;
        Ok(())
    }
    // ------------------------=
    // FUNC: encode_audit
    // DESC: Persists the bounded ring in physical slot order, retaining sequence and causal metadata.
    // ------------------=
    pub(super) fn encode_audit(&self, output: &mut [u8; NODE_STATE_BYTES]) {
        for (slot, record) in self.audit.iter().enumerate() {
            let Some(record) = record else { continue };
            let at = AUDIT_OFFSET + slot * AUDIT_BYTES;
            output[at..at + 8].copy_from_slice(&record.sequence.to_le_bytes());
            output[at + 8..at + 12].copy_from_slice(&record.event_type.to_le_bytes());
            output[at + 12..at + 44].copy_from_slice(&record.subject.0);
            output[at + 44..at + 52].copy_from_slice(&record.timestamp.to_le_bytes());
            output[at + 52..at + 60].copy_from_slice(&record.correlation_id.to_le_bytes());
            output[at + 60] = record.result;
        }
    }

    // ------------------------=
    // FUNC: decode_audit
    // DESC: Rejects malformed, stale or misplaced records before publishing the restored service.
    // ------------------=
    pub(super) fn decode_audit(&mut self, input: &[u8]) -> Result<(), NodeError> {
        for slot in 0..MAX_AUDIT_RECORDS {
            let at = AUDIT_OFFSET + slot * AUDIT_BYTES;
            let bytes = &input[at..at + AUDIT_BYTES];
            let sequence = u64::from_le_bytes(bytes[..8].try_into().unwrap());
            if sequence == 0 {
                if bytes.iter().any(|byte| *byte != 0) {
                    return Err(NodeError::StateCorrupt);
                }
                continue;
            }
            if sequence > self.audit_sequence
                || self.audit_sequence - sequence >= MAX_AUDIT_RECORDS as u64
                || (sequence - 1) % MAX_AUDIT_RECORDS as u64 != slot as u64
                || bytes[61..].iter().any(|byte| *byte != 0)
            {
                return Err(NodeError::StateCorrupt);
            }
            let mut subject = [0; 32];
            subject.copy_from_slice(&bytes[12..44]);
            self.audit[slot] = Some(AuditRecord {
                sequence,
                event_type: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
                subject: NodeId(subject),
                timestamp: u64::from_le_bytes(bytes[44..52].try_into().unwrap()),
                correlation_id: u64::from_le_bytes(bytes[52..60].try_into().unwrap()),
                result: bytes[60],
            });
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: clear_ephemeral
    // DESC: Erases traffic keys and discards all live sessions, grants and pending local confirmations on restore.
    // ------------------=
    pub(super) fn clear_ephemeral(&mut self) {
        for session in self.sessions.iter_mut().flatten() {
            session.tx_key.zeroize();
            session.rx_key.zeroize();
        }
        self.sessions = [None; MAX_SESSIONS];
        self.grants = [None; MAX_REMOTE_GRANTS];
        self.pairings = [None; MAX_PAIRINGS];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------=
    // FUNC: fixture
    // DESC: Creates a production node service with an explicitly confirmed local test peer.
    // ------------------=
    fn fixture() -> NodeRuntime {
        let mut nodes = NodeRuntime::new();
        nodes.initialize(&[71; 32], true).unwrap();
        let mut peer = NodeRuntime::new();
        let id = peer.initialize(&[72; 32], true).unwrap();
        nodes.discover(peer.advertise(1, 1, 1).unwrap(), 1).unwrap();
        let pairing = nodes.begin_pairing(id, 2).unwrap();
        nodes
            .confirm_pairing(pairing.id, pairing.verification_code, true, 3, 77)
            .unwrap();
        nodes
    }

    // ------------------------=
    // FUNC: durable_audit_ring_survives_restart_and_continues
    // DESC: Verifies bounded wrapped audit records, identity and the next sequence across an actual service reconstruction.
    // ------------------=
    #[test]
    fn durable_audit_ring_survives_restart_and_continues() {
        let mut nodes = fixture();
        let peer = nodes.discovered[0].unwrap().id;
        for index in 0..100 {
            nodes.record(AUDIT_POLICY_CHANGED, peer, index, index + 900, 0);
        }
        let encoded = nodes.encode_state().unwrap();
        let mut restored = NodeRuntime::new();
        assert_eq!(
            restored.restore_state(&encoded).unwrap(),
            nodes.local_id.unwrap()
        );
        assert_eq!(restored.audit, nodes.audit);
        assert_eq!(restored.discovered, nodes.discovered);
        assert_eq!(restored.audit.iter().flatten().count(), MAX_AUDIT_RECORDS);
        restored.record(AUDIT_NODE_BLOCKED, peer, 101, 9999, 0);
        assert_eq!(restored.audit_sequence, nodes.audit_sequence + 1);
        assert_eq!(
            restored.audit[(nodes.audit_sequence % MAX_AUDIT_RECORDS as u64) as usize]
                .unwrap()
                .correlation_id,
            9999
        );
    }

    // ------------------------=
    // FUNC: durable_restore_rejection_is_atomic
    // DESC: Corrupts a late record with a valid CRC and proves identity, trust, audit and encoded state remain unchanged.
    // ------------------=
    #[test]
    fn durable_restore_rejection_is_atomic() {
        let mut live = fixture();
        let before = live.encode_state().unwrap();
        let mut incoming = fixture().encode_state().unwrap();
        incoming[AUDIT_OFFSET + 61] = 1;
        let crc = state_crc32(&incoming[..NODE_STATE_BYTES - 4]);
        incoming[NODE_STATE_BYTES - 4..].copy_from_slice(&crc.to_le_bytes());
        assert_eq!(live.restore_state(&incoming), Err(NodeError::StateCorrupt));
        assert_eq!(live.encode_state().unwrap(), before);
        for length in [0, 8, 11, 128, NODE_STATE_BYTES - 1] {
            assert!(live.restore_state(&incoming[..length]).is_err());
            assert_eq!(live.encode_state().unwrap(), before);
        }
    }

    // ------------------------=
    // FUNC: durable_legacy_migration_preserves_identity_and_policy
    // DESC: Restores the previous binary schema without rotating identity or fabricating missing legacy audit records.
    // ------------------=
    #[test]
    fn durable_legacy_migration_preserves_identity_and_policy() {
        let original = fixture();
        let encoded = original.encode_state().unwrap();
        let mut legacy = [0u8; LEGACY_NODE_STATE_BYTES];
        legacy[..AUDIT_OFFSET].copy_from_slice(&encoded[..AUDIT_OFFSET]);
        legacy[8..10].copy_from_slice(&1u16.to_le_bytes());
        legacy[10..12].copy_from_slice(&(LEGACY_NODE_STATE_BYTES as u16).to_le_bytes());
        let crc = state_crc32(&legacy[..LEGACY_NODE_STATE_BYTES - 4]);
        legacy[LEGACY_NODE_STATE_BYTES - 4..].copy_from_slice(&crc.to_le_bytes());
        let mut restored = NodeRuntime::new();
        assert_eq!(
            restored.restore_state(&legacy).unwrap(),
            original.local_id.unwrap()
        );
        assert_eq!(restored.discovered, original.discovered);
        assert_eq!(restored.audit_sequence, original.audit_sequence);
        assert_eq!(restored.audit.iter().flatten().count(), 0);
        assert_eq!(restored.encode_state().unwrap().len(), NODE_STATE_BYTES);
    }

    // ------------------------=
    // FUNC: durable_v2_migration_preserves_audit_without_fabricating_receipts
    // DESC: Reconstructs the previous audit-bearing format and verifies that absent secure receipts and live authority stay absent.
    // ------------------=
    #[test]
    fn durable_v2_migration_preserves_audit_without_fabricating_receipts() {
        let mut original = fixture();
        original.control_version = 41;
        let peer = original.discovered[0].unwrap().id;
        for index in 0..100 {
            original.record(AUDIT_POLICY_CHANGED, peer, index, index + 900, 0);
        }
        let encoded = original.encode_state().unwrap();
        let mut previous = [0u8; V2_NODE_STATE_BYTES];
        previous[..RECEIPTS_OFFSET].copy_from_slice(&encoded[..RECEIPTS_OFFSET]);
        previous[8..10].copy_from_slice(&2u16.to_le_bytes());
        previous[10..12].copy_from_slice(&(V2_NODE_STATE_BYTES as u16).to_le_bytes());
        let crc = state_crc32(&previous[..V2_NODE_STATE_BYTES - 4]);
        previous[V2_NODE_STATE_BYTES - 4..].copy_from_slice(&crc.to_le_bytes());
        let mut restored = NodeRuntime::new();
        assert_eq!(restored.restore_state(&previous).unwrap(), original.local_id.unwrap());
        assert_eq!(restored.discovered, original.discovered);
        assert_eq!(restored.audit, original.audit);
        assert_eq!(restored.audit_sequence, original.audit_sequence);
        assert_eq!(restored.control_version, 41);
        assert_eq!(restored.paired_digest(peer), None);
        assert_eq!(restored.sessions.iter().flatten().count(), 0);
        assert_eq!(restored.grants.iter().flatten().count(), 0);
        let upgraded = restored.encode_state().unwrap();
        let mut restarted = NodeRuntime::new();
        restarted.restore_state(&upgraded).unwrap();
        assert_eq!(restarted.encode_state().unwrap(), upgraded);
    }

    // ------------------------=
    // FUNC: durable_restore_drops_ephemeral_authority
    // DESC: Restoring persisted trust never restores or retains live session keys, grants or pending pairing approval.
    // ------------------=
    #[test]
    fn durable_restore_drops_ephemeral_authority() {
        let mut nodes = fixture();
        let peer = nodes.discovered[0].unwrap().id;
        let (_, public) = NodeCrypto::agreement_keypair(&[45; 32]);
        nodes
            .open_session(peer, &[44; 32], &public, b"persistence-test", 4, 1)
            .unwrap();
        nodes.grant_remote(peer, 0xd002, 0, 1, 100, 4, 1).unwrap();
        let encoded = nodes.encode_state().unwrap();
        assert_eq!(nodes.sessions.iter().flatten().count(), 1);
        nodes.restore_state(&encoded).unwrap();
        assert_eq!(nodes.sessions.iter().flatten().count(), 0);
        assert_eq!(nodes.grants.iter().flatten().count(), 0);
        assert_eq!(nodes.pairings.iter().flatten().count(), 0);
    }
}
