//! Two-node, durable membership intent/acknowledgement. Join requires independent local approval on both nodes.
//! Revision order is (epoch, withdrawal): withdrawal wins a same-epoch conflict. Trust alone never approves Join.
use super::*;
use super::super::iop::{NodeOperationV1, OperationId};
use control::{CommitError, CommittedControl};

pub const MAX_DOMAINS: usize = 8;
const DOMAIN_OFFSET: usize = 8320;
const DOMAIN_BYTES: usize = 128;
pub const FRAME_BYTES: usize = 112;
pub const MAGIC: &[u8; 4] = b"M9DM";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Domain {
    pub peer: NodeId,
    pub revision: u64,
    pub pending_revision: u64,
    pub active: bool,
    pub approved_join: bool,
    pub desired: bool,
    pub correlation: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------=
    // FUNC: peers
    // DESC: Supplies explicit local trust and control policy for HOST transaction-boundary tests, not installed network evidence.
    // ------------------=
    fn peers() -> (NodeRuntime, NodeRuntime) {
        let mut a = NodeRuntime::new(); let mut b = NodeRuntime::new();
        let aid = a.initialize(&[91; 32], true).unwrap(); let bid = b.initialize(&[92; 32], true).unwrap();
        a.discover(b.advertise(1, 1, 1).unwrap(), 1).unwrap(); b.discover(a.advertise(1, 1, 1).unwrap(), 1).unwrap();
        for (nodes, peer) in [(&mut a, bid), (&mut b, aid)] {
            let pair = nodes.begin_pairing(peer, 2).unwrap(); nodes.confirm_pairing(pair.id, pair.verification_code, true, 3, 1).unwrap();
            let mut policy = nodes.discovered[0].unwrap().policy; policy.categories[1] = PolicyDecision::Allow; nodes.update_policy(peer, policy, 3, 1).unwrap();
        }
        (a, b)
    }

    // ------------------------=
    // FUNC: intent
    // DESC: Submits a typed local approval to the durable transaction implementation.
    // ------------------=
    fn intent(nodes: &mut NodeRuntime, peer: NodeId, active: bool) {
        let operation = if active { OperationId::NodeJoin } else { OperationId::NodeLeave };
        let request = NodeOperationV1 { node_id: peer.0, operation: operation.machine_id(), schema_version: 1, handle: 0, scope: 0, lease_deadline: 0, rights: 0, value: 0, flags: 0 };
        nodes.commit_domain_intent(operation, request, 4, 123, 124, |_| true).unwrap();
    }

    // ------------------------=
    // FUNC: membership_lost_ack_restart_and_duplicate_converge
    // DESC: Covers lost request, one-sided commit, lost ACK, both service reconstructions and idempotent duplicate delivery.
    // ------------------=
    #[test]
    fn membership_lost_ack_restart_and_duplicate_converge() {
        let (mut a, mut b) = peers(); let aid = a.local_id.unwrap(); let bid = b.local_id.unwrap();
        intent(&mut a, bid, true);
        let (_, first) = a.domain_proposal(0, 5).unwrap();
        assert_eq!(a.domains[0].unwrap().active, false);
        assert_eq!(a.domain_proposal(0, 8).unwrap().1.transaction, first.transaction);
        assert_eq!(b.receive_domain(aid, first, 5, |_| true), Err(NodeError::HumanApprovalRequired));
        intent(&mut b, aid, true);
        let (ack, notice) = b.receive_domain(aid, Message::decode(&first.encode()).unwrap(), 6, |_| true).unwrap();
        assert!(notice.is_some()); assert!(b.domains[0].unwrap().active);
        let adisk = a.encode_state().unwrap(); let bdisk = b.encode_state().unwrap();
        a = NodeRuntime::new(); b = NodeRuntime::new(); a.restore_state(&adisk).unwrap(); b.restore_state(&bdisk).unwrap();
        let retry = a.domain_proposal(0, 9).unwrap().1;
        assert_eq!(retry.transaction, first.transaction);
        let (duplicate_ack, duplicate_notice) = b.receive_domain(aid, retry, 9, |_| panic!("duplicate must not write storage")).unwrap();
        assert_eq!(duplicate_ack, ack); assert!(duplicate_notice.is_none());
        a.receive_domain(bid, duplicate_ack.unwrap(), 10, |_| true).unwrap();
        assert!(a.domains[0].unwrap().active); assert_eq!(a.domains[0].unwrap().revision, b.domains[0].unwrap().revision);
        assert!(a.domain_proposal(0, 11).is_none());
        assert!(a.receive_domain(bid, ack.unwrap(), 11, |_| panic!("duplicate ACK must not write storage")).unwrap().1.is_none());
        intent(&mut a, bid, false);
        let withdrawal = a.domain_proposal(0, 12).unwrap().1;
        let (ack, _) = b.receive_domain(aid, withdrawal, 12, |_| true).unwrap();
        a.receive_domain(bid, ack.unwrap(), 13, |_| true).unwrap();
        assert!(!a.domains[0].unwrap().active && !b.domains[0].unwrap().active);
        assert!(a.members.iter().flatten().all(|member| !member.enabled));
        let state = a.encode_state().unwrap(); intent(&mut a, bid, false); assert_eq!(a.encode_state().unwrap(), state);
        assert!(b.receive_domain(aid, first, 14, |_| panic!("stale request must not write storage")).is_err());
    }

    // ------------------------=
    // FUNC: membership_authority_failure_and_withdrawal_conflict
    // DESC: Rejects malformed/unauthorized requests, proves storage-failure atomicity, and gives withdrawal deterministic same-epoch precedence.
    // ------------------=
    #[test]
    fn membership_authority_failure_and_withdrawal_conflict() {
        let (mut a, mut b) = peers(); let aid = a.local_id.unwrap(); let bid = b.local_id.unwrap();
        intent(&mut a, bid, true); intent(&mut b, aid, true);
        let proposal = a.domain_proposal(0, 5).unwrap().1;
        let before = b.encode_state().unwrap();
        assert!(b.receive_domain(aid, proposal, 5, |_| false).is_err()); assert_eq!(b.encode_state().unwrap(), before);
        let mut bad = proposal; bad.transaction[0] ^= 1;
        assert!(b.receive_domain(aid, bad, 5, |_| panic!("invalid transaction")).is_err());
        assert!(b.receive_domain(NodeId([0; 32]), proposal, 5, |_| panic!("unauthorized peer")).is_err());
        intent(&mut b, aid, false);
        let withdrawal = b.domain_proposal(0, 5).unwrap().1;
        assert!(withdrawal.revision > proposal.revision);
        assert!(b.receive_domain(aid, proposal, 6, |_| panic!("withdrawal cannot grant join")).is_err());
        let (ack, _) = a.receive_domain(bid, withdrawal, 6, |_| true).unwrap();
        b.receive_domain(aid, ack.unwrap(), 7, |_| true).unwrap();
        assert!(!a.domains[0].unwrap().active && !b.domains[0].unwrap().active);
        assert_eq!(a.domains[0].unwrap().revision, b.domains[0].unwrap().revision);
        a.set_trust(bid, TrustState::Revoked, 8, 9).unwrap();
        assert!(a.receive_domain(bid, withdrawal, 9, |_| panic!("revoked peer")).is_err());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Message {
    pub acknowledge: bool,
    pub domain: NodeId,
    pub transaction: [u8; 32],
    pub revision: u64,
    pub base: u64,
    pub active: bool,
    pub correlation: u64,
}

// ------------------------=
// FUNC: domain_id
// DESC: Derives the same stable two-node domain identity at both independently identified participants.
// ------------------=
pub fn domain_id(local: NodeId, peer: NodeId) -> NodeId {
    let mut hash = Sha256::new();
    hash.update(b"InfinityOS two-node domain v1");
    let (a, b) = if local.0 < peer.0 { (local, peer) } else { (peer, local) };
    hash.update(a.0); hash.update(b.0);
    NodeId(hash.finalize().into())
}

// ------------------------=
// FUNC: transaction_id
// DESC: Produces an idempotent transaction identity bound to domain and ordered target revision.
// ------------------=
fn transaction_id(domain: NodeId, revision: u64) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"InfinityOS domain transaction v1"); hash.update(domain.0); hash.update(revision.to_le_bytes());
    hash.finalize().into()
}

impl Message {
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes bounded membership state in authenticated session payloads, independent of Rust ABI.
    // ------------------=
    pub fn encode(self) -> [u8; FRAME_BYTES] {
        let mut data = [0; FRAME_BYTES];
        data[..4].copy_from_slice(MAGIC); data[4] = 1; data[5] = self.acknowledge as u8; data[6] = self.active as u8;
        data[8..40].copy_from_slice(&self.domain.0); data[40..72].copy_from_slice(&self.transaction);
        data[72..80].copy_from_slice(&self.revision.to_le_bytes()); data[80..88].copy_from_slice(&self.base.to_le_bytes()); data[88..96].copy_from_slice(&self.correlation.to_le_bytes());
        data
    }

    // ------------------------=
    // FUNC: decode
    // DESC: Rejects noncanonical, oversized, stale-format and inconsistent revision/transaction frames before any state mutation.
    // ------------------=
    pub fn decode(data: &[u8]) -> Result<Self, NodeError> {
        if data.len() != FRAME_BYTES || &data[..4] != MAGIC || data[4] != 1 || data[5] > 1 || data[6] > 1 || data[7] != 0 || data[96..].iter().any(|byte| *byte != 0) { return Err(NodeError::UnsupportedState); }
        let mut domain = [0; 32]; domain.copy_from_slice(&data[8..40]);
        let mut transaction = [0; 32]; transaction.copy_from_slice(&data[40..72]);
        let revision = u64::from_le_bytes(data[72..80].try_into().unwrap());
        let base = u64::from_le_bytes(data[80..88].try_into().unwrap());
        let active = data[6] == 1;
        if revision < 2 || (revision & 1 == 0) != active || transaction != transaction_id(NodeId(domain), revision) || revision / 2 != base / 2 + 1 { return Err(NodeError::UnsupportedState); }
        Ok(Self { acknowledge: data[5] == 1, domain: NodeId(domain), transaction, revision, base, active, correlation: u64::from_le_bytes(data[88..96].try_into().unwrap()) })
    }
}

impl NodeRuntime {
    // ------------------------=
    // FUNC: domains
    // DESC: Exposes authoritative bounded domain records, including pending intent rather than pretending local intent is joined state.
    // ------------------=
    pub fn domains(&self) -> &[Option<Domain>; MAX_DOMAINS] { &self.domains }

    // ------------------------=
    // FUNC: domain_authorized
    // DESC: Requires current trusted peer identity and explicitly allowed node-control policy; discovery and trust never grant membership consent.
    // ------------------=
    fn domain_authorized(&self, peer: NodeId, now: u64) -> bool {
        self.discovered.iter().flatten().any(|node| node.id == peer && node.trust == TrustState::Trusted && node.policy.scope == 0 && match node.policy.categories[1] { PolicyDecision::Allow | PolicyDecision::SessionOnly => true, PolicyDecision::Leased => now < node.policy.expires_at, _ => false })
    }

    // ------------------------=
    // FUNC: commit_domain_intent
    // DESC: Records explicit local membership consent or withdrawal durably before any wire proposal can be sent.
    // ------------------=
    pub fn commit_domain_intent(&mut self, operation: OperationId, request: NodeOperationV1, now: u64, correlation: u64, causation: u64, persist: impl FnOnce(&[u8; NODE_STATE_BYTES]) -> bool) -> Result<(NodeOperationV1, CommittedControl), CommitError> {
        let peer = NodeId(request.node_id);
        if !matches!(operation, OperationId::NodeJoin | OperationId::NodeLeave) || request.operation != operation.machine_id() || request.schema_version != 1 || !self.domain_authorized(peer, now) { return Err(CommitError::InvalidOperation); }
        let active = operation == OperationId::NodeJoin;
        let mut staged = self.clone();
        let index = staged.domains.iter().position(|domain| domain.map(|domain| domain.peer == peer).unwrap_or(false)).or_else(|| staged.domains.iter().position(Option::is_none)).ok_or(CommitError::InvalidState)?;
        let mut domain = staged.domains[index].unwrap_or(Domain { peer, revision: 0, pending_revision: 0, active: false, approved_join: false, desired: false, correlation });
        if request.handle != 0 && request.handle != domain.revision { return Err(CommitError::InvalidState); }
        if domain.pending_revision == 0 && domain.active == active {
            return Ok((request, CommittedControl { version: self.control_version, subject: peer, operation, value: active as u32, correlation, causation }));
        }
        domain.pending_revision = (domain.revision / 2).checked_add(1).and_then(|epoch| epoch.checked_mul(2)).and_then(|revision| revision.checked_add(!active as u64)).ok_or(CommitError::VersionExhausted)?;
        domain.desired = active; domain.approved_join = active; domain.correlation = correlation;
        staged.domains[index] = Some(domain);
        let notice = staged.persist_domain(peer, operation, 2, now, correlation, causation, persist)?;
        *self = staged;
        Ok((request, notice))
    }

    // ------------------------=
    // FUNC: domain_proposal
    // DESC: Reconstructs a retry from durable intent; retries preserve transaction identity across disconnect and reboot.
    // ------------------=
    pub fn domain_proposal(&self, index: usize, now: u64) -> Option<(NodeId, Message)> {
        let domain = self.domains.get(index).copied().flatten()?;
        if domain.pending_revision == 0 || !self.domain_authorized(domain.peer, now) { return None; }
        let id = domain_id(self.local_id?, domain.peer);
        Some((domain.peer, Message { acknowledge: false, domain: id, transaction: transaction_id(id, domain.pending_revision), revision: domain.pending_revision, base: domain.revision, active: domain.desired, correlation: domain.correlation }))
    }

    // ------------------------=
    // FUNC: receive_domain
    // DESC: Validates exact authenticated peer, local approval and revision order, commits before ACK, and tolerates duplicate/lost acknowledgements.
    // ------------------=
    pub fn receive_domain(&mut self, peer: NodeId, message: Message, now: u64, persist: impl FnOnce(&[u8; NODE_STATE_BYTES]) -> bool) -> Result<(Option<Message>, Option<CommittedControl>), NodeError> {
        if !self.domain_authorized(peer, now) || self.local_id.map(|local| domain_id(local, peer)) != Some(message.domain) { return Err(NodeError::CapabilityDenied); }
        // Validate even typed callers, not only decoded wire packets.
        let message = Message::decode(&message.encode())?;
        let index = self.domains.iter().position(|domain| domain.map(|domain| domain.peer == peer).unwrap_or(false)).or_else(|| (!message.active && !message.acknowledge).then(|| self.domains.iter().position(Option::is_none)).flatten()).ok_or(NodeError::HumanApprovalRequired)?;
        let current = self.domains[index].unwrap_or(Domain { peer, revision: 0, pending_revision: 0, active: false, approved_join: false, desired: false, correlation: message.correlation });
        if message.revision == current.revision && message.active == current.active {
            return Ok(((!message.acknowledge).then_some(Message { acknowledge: true, ..message }), None));
        }
        if message.revision < current.revision || message.base / 2 > current.revision / 2 || message.revision / 2 < current.revision / 2 { return Err(NodeError::UnsupportedState); }
        if message.acknowledge && (current.pending_revision == 0 || message.revision < current.pending_revision || message.revision / 2 != current.pending_revision / 2) { return Err(NodeError::UnsupportedState); }
        if message.active && (!current.approved_join || current.pending_revision == 0 || !current.desired) { return Err(NodeError::HumanApprovalRequired); }
        let mut staged = self.clone();
        staged.domains[index] = Some(Domain { revision: message.revision, pending_revision: 0, active: message.active, desired: message.active, approved_join: message.active, correlation: message.correlation, ..current });
        let member_index = staged.members.iter().position(|member| member.map(|member| member.node == peer).unwrap_or(false)).or_else(|| staged.members.iter().position(Option::is_none)).ok_or(NodeError::MeshFull)?;
        staged.members[member_index] = Some(MeshMember { node: peer, role: MeshRole::Member, joined_at: now, last_heartbeat: now, enabled: message.active });
        let operation = if message.active { OperationId::NodeJoin } else { OperationId::NodeLeave };
        let notice = staged.persist_domain(peer, operation, message.active as u32, now, message.correlation, message.revision, persist).map_err(|_| NodeError::StateCorrupt)?;
        *self = staged;
        Ok(((!message.acknowledge).then_some(Message { acknowledge: true, ..message }), Some(notice)))
    }

    // ------------------------=
    // FUNC: persist_domain
    // DESC: Commits the candidate journal, projection and audit under one checkpoint before replacing live state or emitting an event.
    // ------------------=
    fn persist_domain(&mut self, peer: NodeId, operation: OperationId, value: u32, now: u64, correlation: u64, causation: u64, persist: impl FnOnce(&[u8; NODE_STATE_BYTES]) -> bool) -> Result<CommittedControl, CommitError> {
        self.control_version = self.control_version.checked_add(1).ok_or(CommitError::VersionExhausted)?;
        self.record(AUDIT_MESH_MEMBERSHIP_CHANGED, peer, now, correlation, value as u8);
        let bytes = zeroize::Zeroizing::new(self.encode_state().map_err(|_| CommitError::InvalidState)?);
        if !persist(&bytes) { return Err(CommitError::PersistenceFailed); }
        Ok(CommittedControl { version: self.control_version, subject: peer, operation, value, correlation, causation })
    }

    // ------------------------=
    // FUNC: encode_domains
    // DESC: Persists bounded domain state and retry intent, never transport/session references.
    // ------------------=
    pub(super) fn encode_domains(&self, output: &mut [u8; NODE_STATE_BYTES]) {
        for (index, domain) in self.domains.iter().flatten().enumerate() {
            let at = DOMAIN_OFFSET + index * DOMAIN_BYTES;
            output[at] = 1; output[at + 1] = domain.active as u8; output[at + 2] = domain.approved_join as u8; output[at + 3] = domain.desired as u8;
            output[at + 8..at + 40].copy_from_slice(&domain.peer.0);
            output[at + 40..at + 48].copy_from_slice(&domain.revision.to_le_bytes()); output[at + 48..at + 56].copy_from_slice(&domain.pending_revision.to_le_bytes()); output[at + 56..at + 64].copy_from_slice(&domain.correlation.to_le_bytes());
        }
    }

    // ------------------------=
    // FUNC: decode_domains
    // DESC: Rejects malformed and duplicate persisted domain identities before publishing a reconstructed service.
    // ------------------=
    pub(super) fn decode_domains(&mut self, input: &[u8]) -> Result<(), NodeError> {
        for index in 0..MAX_DOMAINS {
            let data = &input[DOMAIN_OFFSET + index * DOMAIN_BYTES..DOMAIN_OFFSET + (index + 1) * DOMAIN_BYTES];
            if data[0] == 0 { if data.iter().any(|byte| *byte != 0) { return Err(NodeError::StateCorrupt); } continue; }
            if data[0] != 1 || data[1..4].iter().any(|byte| *byte > 1) || data[4..8].iter().chain(data[64..].iter()).any(|byte| *byte != 0) { return Err(NodeError::StateCorrupt); }
            let mut peer = [0; 32]; peer.copy_from_slice(&data[8..40]);
            if !self.discovered.iter().flatten().any(|node| node.id.0 == peer) || self.domains.iter().flatten().any(|domain| domain.peer.0 == peer) { return Err(NodeError::StateCorrupt); }
            let domain = Domain { peer: NodeId(peer), active: data[1] != 0, approved_join: data[2] != 0, desired: data[3] != 0, revision: u64::from_le_bytes(data[40..48].try_into().unwrap()), pending_revision: u64::from_le_bytes(data[48..56].try_into().unwrap()), correlation: u64::from_le_bytes(data[56..64].try_into().unwrap()) };
            if domain.revision == 1 || (domain.revision == 0 && domain.active) || (domain.revision >= 2 && (domain.revision & 1 == 0) != domain.active) || domain.approved_join != domain.desired { return Err(NodeError::StateCorrupt); }
            if domain.pending_revision == 0 {
                if domain.desired != domain.active { return Err(NodeError::StateCorrupt); }
            } else if domain.pending_revision < 2 || domain.pending_revision / 2 != domain.revision / 2 + 1 || (domain.pending_revision & 1 == 0) != domain.desired { return Err(NodeError::StateCorrupt); }
            self.domains[index] = Some(domain);
        }
        Ok(())
    }
}
