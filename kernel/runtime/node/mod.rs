//! Secure node identity, discovery, pairing, sessions, remote authority, and mesh membership.

pub mod types;
pub mod transport;
pub mod wire_trust;

use sha2::{Digest, Sha256};
use zeroize::Zeroize;
use super::crypto::{CryptoError, KeyRef, NodeCrypto};
use types::*;

pub const NODE_PROTOCOL_VERSION: u16 = 1;
pub const DISCOVERY_LEASE_TICKS: u64 = 30;
pub const PAIRING_LEASE_TICKS: u64 = 120;
pub const SESSION_LEASE_TICKS: u64 = 3600;

pub const AUDIT_NODE_PAIRED: u32 = 0xd001;
pub const AUDIT_NODE_REVOKED: u32 = 0xd002;
pub const AUDIT_SESSION_OPENED: u32 = 0xd003;
pub const AUDIT_SESSION_REJECTED: u32 = 0xd004;
pub const AUDIT_REMOTE_GRANT_CREATED: u32 = 0xd005;
pub const AUDIT_REMOTE_GRANT_REVOKED: u32 = 0xd006;
pub const AUDIT_MESH_MEMBERSHIP_CHANGED: u32 = 0xd007;
pub const AUDIT_NODE_BLOCKED: u32 = 0xd008;
pub const AUDIT_NODE_UNBLOCKED: u32 = 0xd009;
pub const AUDIT_POLICY_CHANGED: u32 = 0xd00a;

const NODE_STATE_MAGIC: &[u8; 8] = b"INFNOD01";
const NODE_RECORD_OFFSET: usize = 128;
const NODE_RECORD_BYTES: usize = 128;
const MEMBER_RECORD_OFFSET: usize = NODE_RECORD_OFFSET + MAX_DISCOVERED_NODES * NODE_RECORD_BYTES;
const MEMBER_RECORD_BYTES: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DiscoveryAdvertisement {
    pub node: NodeId,
    pub public_key: [u8; 32],
    pub protocol_min: u16,
    pub protocol_max: u16,
    pub service_bits: u64,
    pub sequence: u64,
    pub expires_at: u64,
    pub signature: [u8; 64],
}

impl DiscoveryAdvertisement {
    // ------------------------=
    // FUNC: transcript
    // DESC: Encodes the signed discovery fields into a deterministic architecture-neutral transcript.
    // ------------------=
    pub fn transcript(&self) -> [u8; 92] {
        let mut out = [0u8; 92];
        out[..32].copy_from_slice(&self.node.0);
        out[32..64].copy_from_slice(&self.public_key);
        out[64..66].copy_from_slice(&self.protocol_min.to_le_bytes());
        out[66..68].copy_from_slice(&self.protocol_max.to_le_bytes());
        out[68..76].copy_from_slice(&self.service_bits.to_le_bytes());
        out[76..84].copy_from_slice(&self.sequence.to_le_bytes());
        out[84..92].copy_from_slice(&self.expires_at.to_le_bytes());
        out
    }
}

pub struct NodeRuntime {
    crypto: NodeCrypto,
    identity_seed: [u8; 32],
    local_id: Option<NodeId>,
    key_ref: Option<KeyRef>,
    discovered: [Option<NodeDescriptor>; MAX_DISCOVERED_NODES],
    pairings: [Option<Pairing>; MAX_PAIRINGS],
    sessions: [Option<SecureSession>; MAX_SESSIONS],
    grants: [Option<RemoteGrant>; MAX_REMOTE_GRANTS],
    members: [Option<MeshMember>; MAX_MESH_MEMBERS],
    audit: [Option<AuditRecord>; MAX_AUDIT_RECORDS],
    next_id: u64,
    audit_sequence: u64,
    discovery_window: u64,
    discovery_count: u16,
}

impl NodeRuntime {
    // ------------------------=
    // FUNC: new
    // DESC: Creates bounded node services with no identity or ambient remote authority.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            crypto: NodeCrypto::new(), identity_seed: [0; 32], local_id: None, key_ref: None,
            discovered: [None; MAX_DISCOVERED_NODES], pairings: [None; MAX_PAIRINGS],
            sessions: [None; MAX_SESSIONS], grants: [None; MAX_REMOTE_GRANTS],
            members: [None; MAX_MESH_MEMBERS], audit: [None; MAX_AUDIT_RECORDS],
            next_id: 1, audit_sequence: 0, discovery_window: 0, discovery_count: 0,
        }
    }

    // ------------------------=
    // FUNC: initialize
    // DESC: Creates the persistent node identity from trusted firmware entropy.
    // ------------------=
    pub fn initialize(&mut self, entropy: &[u8; 32], valid: bool) -> Result<NodeId, NodeError> {
        if !valid || entropy.iter().all(|value| *value == 0) { return Err(NodeError::EntropyUnavailable); }
        let mut digest = Sha256::new();
        digest.update(b"InfinityOS Node Identity v1");
        digest.update(entropy);
        self.identity_seed = digest.finalize().into();
        let key_ref = self.crypto.initialize_seed(&self.identity_seed).map_err(map_crypto_error)?;
        let public = self.crypto.public_identity().map_err(map_crypto_error)?;
        let mut digest = Sha256::new();
        digest.update(b"InfinityOS NodeId v1");
        digest.update(public);
        let id = NodeId(digest.finalize().into());
        self.key_ref = Some(key_ref);
        self.local_id = Some(id);
        Ok(id)
    }

    // ------------------------=
    // FUNC: local_id
    // DESC: Returns the stable local node identity when cryptographic initialization succeeded.
    // ------------------=
    pub fn local_id(&self) -> Option<NodeId> { self.local_id }

    // ------------------------=
    // FUNC: advertise
    // DESC: Creates a signed, leased discovery advertisement that conveys no trust or authority.
    // ------------------=
    pub fn advertise(&self, service_bits: u64, sequence: u64, now: u64) -> Result<DiscoveryAdvertisement, NodeError> {
        let mut advertisement = DiscoveryAdvertisement {
            node: self.local_id.ok_or(NodeError::EntropyUnavailable)?,
            public_key: self.crypto.public_identity().map_err(map_crypto_error)?,
            protocol_min: NODE_PROTOCOL_VERSION, protocol_max: NODE_PROTOCOL_VERSION,
            service_bits, sequence, expires_at: now.saturating_add(DISCOVERY_LEASE_TICKS), signature: [0; 64],
        };
        advertisement.signature = self.crypto.sign(self.key_ref.ok_or(NodeError::EntropyUnavailable)?, &advertisement.transcript()).map_err(map_crypto_error)?;
        Ok(advertisement)
    }

    // ------------------------=
    // FUNC: discover
    // DESC: Validates, rate-limits, and records a signed peer advertisement without granting trust.
    // ------------------=
    pub fn discover(&mut self, advertisement: DiscoveryAdvertisement, now: u64) -> Result<(), NodeError> {
        if now >= advertisement.expires_at { return Err(NodeError::InvalidAdvertisement); }
        if advertisement.protocol_min > NODE_PROTOCOL_VERSION || advertisement.protocol_max < NODE_PROTOCOL_VERSION { return Err(NodeError::UnsupportedVersion); }
        if self.local_id == Some(advertisement.node) { return Err(NodeError::InvalidAdvertisement); }
        NodeCrypto::verify(&advertisement.public_key, &advertisement.transcript(), &advertisement.signature).map_err(map_crypto_error)?;
        self.record_discovery(advertisement, now)
    }

    // ------------------------=
    // FUNC: record_discovery
    // DESC: Commits authenticated discovery using receiver-local liveness time without granting authority.
    // ------------------=
    fn record_discovery(&mut self, advertisement: DiscoveryAdvertisement, now: u64) -> Result<(), NodeError> {
        if advertisement.protocol_min > NODE_PROTOCOL_VERSION || advertisement.protocol_max < NODE_PROTOCOL_VERSION { return Err(NodeError::UnsupportedVersion); }
        if self.local_id == Some(advertisement.node) { return Err(NodeError::InvalidAdvertisement); }
        let expected = node_id_from_public(&advertisement.public_key);
        if expected != advertisement.node { return Err(NodeError::InvalidAdvertisement); }
        let window = now / 10;
        if window != self.discovery_window { self.discovery_window = window; self.discovery_count = 0; }
        if self.discovery_count >= 32 { return Err(NodeError::ResourceLimit); }
        self.discovery_count += 1;
        if let Some(existing) = self.discovered.iter_mut().flatten().find(|node| node.id == advertisement.node) {
            if matches!(existing.trust, TrustState::Blocked | TrustState::Revoked) { return Err(NodeError::Blocked); }
            existing.last_seen = now;
            existing.reachability = Reachability::Online;
            existing.service_bits = advertisement.service_bits;
            return Ok(());
        }
        let slot = self.discovered.iter().position(Option::is_none).ok_or(NodeError::ResourceLimit)?;
        self.discovered[slot] = Some(NodeDescriptor {
            id: advertisement.node, public_key: advertisement.public_key,
            protocol_min: advertisement.protocol_min, protocol_max: advertisement.protocol_max,
            service_bits: advertisement.service_bits, last_seen: now,
            reachability: Reachability::Online, trust: TrustState::Untrusted,
            policy: NodeTrustPolicy::deny_all(),
        });
        Ok(())
    }

    // ------------------------=
    // FUNC: begin_pairing
    // DESC: Opens a short-lived explicit-verification pairing transaction for a discovered peer.
    // ------------------=
    pub fn begin_pairing(&mut self, peer: NodeId, now: u64) -> Result<Pairing, NodeError> {
        self.expire_pairings(now);
        let node_index = self.discovered.iter().position(|node| node.map(|value| value.id == peer).unwrap_or(false)).ok_or(NodeError::UnknownNode)?;
        let node = self.discovered[node_index].ok_or(NodeError::UnknownNode)?;
        if matches!(node.trust, TrustState::Blocked | TrustState::Revoked) { return Err(NodeError::Blocked); }
        if matches!(node.trust, TrustState::Trusted | TrustState::Restricted) { return Err(NodeError::NotTrusted); }
        if self.pairings.iter().flatten().any(|pairing| pairing.peer == peer && pairing.state == PairingState::AwaitingConfirmation && now < pairing.expires_at) {
            return Err(NodeError::ResourceLimit);
        }
        let slot = self.pairings.iter().position(|entry| entry.map(|pairing| pairing.state != PairingState::AwaitingConfirmation || now >= pairing.expires_at).unwrap_or(true)).ok_or(NodeError::ResourceLimit)?;
        let id = self.take_id();
        let fingerprint = fingerprint(&node.public_key);
        let verification_code = u32::from_le_bytes([fingerprint[0], fingerprint[5], fingerprint[10], fingerprint[15]]) % 1_000_000;
        let pairing = Pairing { id, peer, fingerprint, verification_code, expires_at: now.saturating_add(PAIRING_LEASE_TICKS), state: PairingState::AwaitingConfirmation };
        if let Some(node) = self.discovered[node_index].as_mut() { node.trust = TrustState::PairingPending; }
        self.pairings[slot] = Some(pairing);
        Ok(pairing)
    }

    // ------------------------=
    // FUNC: confirm_pairing
    // DESC: Promotes a peer only after an explicit human decision and matching out-of-band verification code.
    // ------------------=
    pub fn confirm_pairing(&mut self, pairing_id: u64, code: u32, human_approved: bool, now: u64, correlation_id: u64) -> Result<(), NodeError> {
        if !human_approved { return Err(NodeError::HumanApprovalRequired); }
        self.expire_pairings(now);
        let pairing = self.pairings.iter_mut().flatten().find(|pairing| pairing.id == pairing_id).ok_or(NodeError::PairingNotFound)?;
        if pairing.state != PairingState::AwaitingConfirmation {
            return Err(if pairing.state == PairingState::Expired { NodeError::PairingExpired } else { NodeError::PairingNotFound });
        }
        if now >= pairing.expires_at { pairing.state = PairingState::Expired; return Err(NodeError::PairingExpired); }
        if pairing.verification_code != code { return Err(NodeError::VerificationMismatch); }
        let peer = pairing.peer;
        let node = self.discovered.iter_mut().flatten().find(|node| node.id == peer).ok_or(NodeError::UnknownNode)?;
        if node.trust != TrustState::PairingPending { return Err(NodeError::NotTrusted); }
        if fingerprint(&node.public_key) != pairing.fingerprint { return Err(NodeError::IdentityMismatch); }
        if node.protocol_min > NODE_PROTOCOL_VERSION || node.protocol_max < NODE_PROTOCOL_VERSION { return Err(NodeError::UnsupportedVersion); }
        pairing.state = PairingState::Confirmed;
        node.trust = TrustState::Trusted;
        self.record(AUDIT_NODE_PAIRED, peer, now, correlation_id, 1);
        Ok(())
    }

    // ------------------------=
    // FUNC: revoke_trust
    // DESC: Revokes peer trust and immediately closes its sessions and remote grants.
    // ------------------=
    pub fn revoke_trust(&mut self, peer: NodeId, now: u64, correlation_id: u64) -> Result<(), NodeError> {
        self.discovered.iter_mut().flatten().find(|node| node.id == peer).ok_or(NodeError::UnknownNode)?.trust = TrustState::Revoked;
        self.cancel_pending_pairings(peer);
        for session in self.sessions.iter_mut().flatten().filter(|session| session.peer == peer) { session.state = SessionState::Closed; session.tx_key.zeroize(); session.rx_key.zeroize(); }
        for grant in self.grants.iter_mut().flatten().filter(|grant| grant.peer == peer) { grant.revoked = true; }
        self.record(AUDIT_NODE_REVOKED, peer, now, correlation_id, 1);
        Ok(())
    }

    // ------------------------=
    // FUNC: set_trust
    // DESC: Applies an explicit restricted, trusted, blocked, or untrusted state without granting capabilities.
    // ------------------=
    pub fn set_trust(&mut self, peer: NodeId, state: TrustState, now: u64, correlation_id: u64) -> Result<(), NodeError> {
        let node = self.discovered.iter_mut().flatten().find(|node| node.id == peer).ok_or(NodeError::UnknownNode)?;
        if matches!(state, TrustState::Trusted | TrustState::Restricted)
            && !matches!(node.trust, TrustState::Trusted | TrustState::Restricted) {
            return Err(NodeError::HumanApprovalRequired);
        }
        if matches!(state, TrustState::PairingPending | TrustState::Discovered | TrustState::Incompatible) {
            return Err(NodeError::CapabilityDenied);
        }
        node.trust = state;
        self.cancel_pending_pairings(peer);
        let event = if state == TrustState::Blocked { AUDIT_NODE_BLOCKED } else if matches!(state, TrustState::Untrusted | TrustState::Discovered) { AUDIT_NODE_UNBLOCKED } else { AUDIT_POLICY_CHANGED };
        if state != TrustState::Trusted {
            for session in self.sessions.iter_mut().flatten().filter(|session| session.peer == peer) { session.state = SessionState::Closed; session.tx_key.zeroize(); session.rx_key.zeroize(); }
            for grant in self.grants.iter_mut().flatten().filter(|grant| grant.peer == peer) { grant.revoked = true; }
        }
        self.record(event, peer, now, correlation_id, 1);
        Ok(())
    }

    // ------------------------=
    // FUNC: update_policy
    // DESC: Commits an authoritative per-node policy without expanding trust or capability authority.
    // ------------------=
    pub fn update_policy(&mut self, peer: NodeId, policy: NodeTrustPolicy, now: u64, correlation_id: u64) -> Result<(), NodeError> {
        let node = self.discovered.iter_mut().flatten().find(|node| node.id == peer).ok_or(NodeError::UnknownNode)?;
        node.policy = policy;
        self.record(AUDIT_POLICY_CHANGED, peer, now, correlation_id, 1);
        Ok(())
    }

    // ------------------------=
    // FUNC: cancel_pairing
    // DESC: Cancels a pending pairing transaction without creating partial trust.
    // ------------------=
    pub fn cancel_pairing(&mut self, pairing_id: u64) -> Result<(), NodeError> {
        let pairing = self.pairings.iter_mut().flatten().find(|pairing| pairing.id == pairing_id).ok_or(NodeError::PairingNotFound)?;
        if pairing.state != PairingState::AwaitingConfirmation { return Err(NodeError::PairingNotFound); }
        pairing.state = PairingState::Cancelled;
        if let Some(node) = self.discovered.iter_mut().flatten().find(|node| node.id == pairing.peer && node.trust == TrustState::PairingPending) { node.trust = TrustState::Untrusted; }
        Ok(())
    }

    // ------------------------=
    // FUNC: cancel_pending_pairings
    // DESC: Invalidates pending approvals when an explicit trust decision supersedes them.
    // ------------------=
    fn cancel_pending_pairings(&mut self, peer: NodeId) {
        for pairing in self.pairings.iter_mut().flatten().filter(|pairing| pairing.peer == peer && pairing.state == PairingState::AwaitingConfirmation) {
            pairing.state = PairingState::Cancelled;
        }
    }

    // ------------------------=
    // FUNC: expire_pairings
    // DESC: Expires approvals and their pending trust projection together without granting authority.
    // ------------------=
    fn expire_pairings(&mut self, now: u64) {
        for pairing in self.pairings.iter_mut().flatten() {
            if pairing.state == PairingState::AwaitingConfirmation && now >= pairing.expires_at {
                pairing.state = PairingState::Expired;
                if let Some(node) = self.discovered.iter_mut().flatten().find(|node| node.id == pairing.peer && node.trust == TrustState::PairingPending) {
                    node.trust = TrustState::Untrusted;
                }
            }
        }
    }

    // ------------------------=
    // FUNC: open_session
    // DESC: Creates a transcript-bound session only for an explicitly trusted peer.
    // ------------------=
    pub fn open_session(&mut self, peer: NodeId, local_secret: &[u8; 32], peer_ephemeral: &[u8; 32], transcript: &[u8], now: u64, correlation_id: u64) -> Result<u64, NodeError> {
        let trusted = self.discovered.iter().flatten().any(|node| node.id == peer && node.trust == TrustState::Trusted);
        if !trusted { self.record(AUDIT_SESSION_REJECTED, peer, now, correlation_id, 0); return Err(NodeError::NotTrusted); }
        let local = self.local_id.ok_or(NodeError::EntropyUnavailable)?;
        let (tx_key, rx_key, protocol_reference) = NodeCrypto::derive_duplex_keys(local_secret, peer_ephemeral, &local.0, &peer.0, transcript).map_err(map_crypto_error)?;
        if self.sessions.iter().flatten().any(|session| session.protocol_reference == protocol_reference) {
            return Err(NodeError::ReplayDetected);
        }
        let slot = self.sessions.iter().position(Option::is_none).ok_or(NodeError::ResourceLimit)?;
        let id = self.take_id();
        self.sessions[slot] = Some(SecureSession { id, peer, state: SessionState::Established,
            tx_key, rx_key, protocol_reference, send_sequence: 0,
            receive_sequence: 0, expires_at: now.saturating_add(SESSION_LEASE_TICKS) });
        self.record(AUDIT_SESSION_OPENED, peer, now, correlation_id, 1);
        Ok(id)
    }

    // ------------------------=
    // FUNC: protect
    // DESC: Encrypts one session payload with a monotonic sequence-derived nonce and authenticated metadata.
    // ------------------=
    pub fn protect(&mut self, session_id: u64, aad: &[u8], payload: &mut [u8], now: u64) -> Result<(u64, [u8; 16]), NodeError> {
        let session = self.sessions.iter_mut().flatten().find(|session| session.id == session_id).ok_or(NodeError::SessionNotFound)?;
        if session.state != SessionState::Established || now >= session.expires_at { return Err(NodeError::SessionExpired); }
        session.send_sequence = session.send_sequence.checked_add(1).ok_or(NodeError::SessionExpired)?;
        let sequence = session.send_sequence;
        let nonce = session_nonce(sequence);
        NodeCrypto::seal(&session.tx_key, &nonce, aad, payload).map(|tag| (sequence, tag)).map_err(map_crypto_error)
    }

    // ------------------------=
    // FUNC: unprotect
    // DESC: Rejects replay and authenticates a session payload before advancing receive state.
    // ------------------=
    pub fn unprotect(&mut self, session_id: u64, sequence: u64, aad: &[u8], payload: &mut [u8], tag: &[u8; 16], now: u64) -> Result<(), NodeError> {
        let session = self.sessions.iter_mut().flatten().find(|session| session.id == session_id).ok_or(NodeError::SessionNotFound)?;
        if session.state != SessionState::Established || now >= session.expires_at { return Err(NodeError::SessionExpired); }
        if sequence <= session.receive_sequence { return Err(NodeError::ReplayDetected); }
        let nonce = session_nonce(sequence);
        NodeCrypto::open(&session.rx_key, &nonce, aad, payload, tag).map_err(map_crypto_error)?;
        session.receive_sequence = sequence;
        Ok(())
    }

    // ------------------------=
    // FUNC: grant_remote
    // DESC: Grants a trusted peer one narrow operation scope with a mandatory expiration lease.
    // ------------------=
    pub fn grant_remote(&mut self, peer: NodeId, operation: u32, scope: u64, rights: u32, expires_at: u64, now: u64, correlation_id: u64) -> Result<u64, NodeError> {
        if expires_at <= now || rights == 0 { return Err(NodeError::CapabilityDenied); }
        if !self.discovered.iter().flatten().any(|node| node.id == peer && node.trust == TrustState::Trusted) { return Err(NodeError::NotTrusted); }
        let slot = self.grants.iter().position(Option::is_none).ok_or(NodeError::ResourceLimit)?;
        let id = self.take_id();
        self.grants[slot] = Some(RemoteGrant { id, peer, operation, scope, rights, expires_at, revoked: false });
        self.record(AUDIT_REMOTE_GRANT_CREATED, peer, now, correlation_id, 1);
        Ok(id)
    }

    // ------------------------=
    // FUNC: authorize_remote
    // DESC: Validates peer, operation, scope, rights, revocation, and lease at each remote call.
    // ------------------=
    pub fn authorize_remote(&self, grant_id: u64, peer: NodeId, operation: u32, scope: u64, rights: u32, now: u64) -> Result<(), NodeError> {
        let grant = self.grants.iter().flatten().find(|grant| grant.id == grant_id).ok_or(NodeError::CapabilityDenied)?;
        if grant.revoked { return Err(NodeError::CapabilityRevoked); }
        if now >= grant.expires_at { return Err(NodeError::CapabilityExpired); }
        if !self.discovered.iter().flatten().any(|node| node.id == peer && node.trust == TrustState::Trusted) { return Err(NodeError::NotTrusted); }
        if grant.peer != peer || grant.operation != operation || grant.scope != scope || rights & !grant.rights != 0 { return Err(NodeError::CapabilityDenied); }
        Ok(())
    }

    // ------------------------=
    // FUNC: revoke_remote
    // DESC: Revokes a remote capability immediately without restarting either node.
    // ------------------=
    pub fn revoke_remote(&mut self, grant_id: u64, now: u64, correlation_id: u64) -> Result<(), NodeError> {
        let grant = self.grants.iter_mut().flatten().find(|grant| grant.id == grant_id).ok_or(NodeError::CapabilityDenied)?;
        grant.revoked = true;
        let peer = grant.peer;
        self.record(AUDIT_REMOTE_GRANT_REVOKED, peer, now, correlation_id, 1);
        Ok(())
    }

    // ------------------------=
    // FUNC: close_session
    // DESC: Closes one node session and zeroizes its ephemeral traffic key immediately.
    // ------------------=
    pub fn close_session(&mut self, session_id: u64, now: u64, correlation_id: u64) -> Result<(), NodeError> {
        let session = self.sessions.iter_mut().flatten().find(|session| session.id == session_id).ok_or(NodeError::SessionNotFound)?;
        session.state = SessionState::Closed;
        session.tx_key.zeroize(); session.rx_key.zeroize();
        let peer = session.peer;
        self.record(AUDIT_SESSION_REJECTED, peer, now, correlation_id, 1);
        Ok(())
    }

    // ------------------------=
    // FUNC: join_mesh
    // DESC: Adds a trusted peer to explicit mesh membership without changing its trust state.
    // ------------------=
    pub fn join_mesh(&mut self, peer: NodeId, role: MeshRole, now: u64, correlation_id: u64) -> Result<(), NodeError> {
        if !self.discovered.iter().flatten().any(|node| node.id == peer && node.trust == TrustState::Trusted) { return Err(NodeError::NotTrusted); }
        if self.members.iter().flatten().any(|member| member.node == peer && member.enabled) { return Err(NodeError::AlreadyMember); }
        let slot = self.members.iter().position(Option::is_none).ok_or(NodeError::MeshFull)?;
        self.members[slot] = Some(MeshMember { node: peer, role, joined_at: now, last_heartbeat: now, enabled: true });
        self.record(AUDIT_MESH_MEMBERSHIP_CHANGED, peer, now, correlation_id, 1);
        Ok(())
    }

    // ------------------------=
    // FUNC: leave_mesh
    // DESC: Removes explicit mesh membership while preserving the independent node trust relationship.
    // ------------------=
    pub fn leave_mesh(&mut self, peer: NodeId, now: u64, correlation_id: u64) -> Result<(), NodeError> {
        let member = self.members.iter_mut().flatten().find(|member| member.node == peer && member.enabled).ok_or(NodeError::NotMember)?;
        member.enabled = false;
        self.record(AUDIT_MESH_MEMBERSHIP_CHANGED, peer, now, correlation_id, 1);
        Ok(())
    }

    // ------------------------=
    // FUNC: heartbeat
    // DESC: Updates observed liveness for a member without treating health as trust authority.
    // ------------------=
    pub fn heartbeat(&mut self, peer: NodeId, now: u64) -> Result<(), NodeError> {
        let member = self.members.iter_mut().flatten().find(|member| member.node == peer && member.enabled).ok_or(NodeError::NotMember)?;
        member.last_heartbeat = now;
        if let Some(node) = self.discovered.iter_mut().flatten().find(|node| node.id == peer) { node.reachability = Reachability::Online; node.last_seen = now; }
        Ok(())
    }

    // ------------------------=
    // FUNC: sweep
    // DESC: Expires stale discovery, pairing, session, capability, and health state deterministically.
    // ------------------=
    pub fn sweep(&mut self, now: u64) {
        for node in self.discovered.iter_mut().flatten() { if now.saturating_sub(node.last_seen) > DISCOVERY_LEASE_TICKS { node.reachability = Reachability::Offline; } }
        self.expire_pairings(now);
        for session in self.sessions.iter_mut().flatten() { if session.state == SessionState::Established && now >= session.expires_at { session.state = SessionState::Closed; session.tx_key.zeroize(); session.rx_key.zeroize(); } }
    }

    // ------------------------=
    // FUNC: discovered_nodes
    // DESC: Returns the bounded authoritative discovered-node table for GUI and Console projections.
    // ------------------=
    pub fn discovered_nodes(&self) -> &[Option<NodeDescriptor>; MAX_DISCOVERED_NODES] { &self.discovered }

    // ------------------------=
    // FUNC: mesh_members
    // DESC: Returns the bounded authoritative mesh-membership table for status projections.
    // ------------------=
    pub fn mesh_members(&self) -> &[Option<MeshMember>; MAX_MESH_MEMBERS] { &self.members }

    // ------------------------=
    // FUNC: audit_records
    // DESC: Returns the bounded structured security audit ring without secret key material.
    // ------------------=
    pub fn audit_records(&self) -> &[Option<AuditRecord>; MAX_AUDIT_RECORDS] { &self.audit }

    // ------------------------=
    // FUNC: pairings
    // DESC: Returns pending pairing metadata safe for the protected Trusted UI projection.
    // ------------------=
    pub fn pairings(&self) -> &[Option<Pairing>; MAX_PAIRINGS] { &self.pairings }

    // ------------------------=
    // FUNC: sessions
    // DESC: Returns session metadata while retaining all ephemeral traffic keys inside the Node Session service.
    // ------------------=
    pub fn sessions(&self) -> &[Option<SecureSession>; MAX_SESSIONS] { &self.sessions }

    // ------------------------=
    // FUNC: remote_grants
    // DESC: Returns bounded remote authority metadata without exporting capability secrets.
    // ------------------=
    pub fn remote_grants(&self) -> &[Option<RemoteGrant>; MAX_REMOTE_GRANTS] { &self.grants }

    // ------------------------=
    // FUNC: encode_state
    // DESC: Encodes persistent node identity, trust policy, membership, and integrity into a typed System object.
    // ------------------=
    pub fn encode_state(&self) -> Result<[u8; NODE_STATE_BYTES], NodeError> {
        let local_id = self.local_id.ok_or(NodeError::EntropyUnavailable)?;
        if self.identity_seed.iter().all(|value| *value == 0) { return Err(NodeError::EntropyUnavailable); }
        let mut out = [0u8; NODE_STATE_BYTES];
        out[..8].copy_from_slice(NODE_STATE_MAGIC);
        out[8..10].copy_from_slice(&1u16.to_le_bytes());
        out[10..12].copy_from_slice(&(NODE_STATE_BYTES as u16).to_le_bytes());
        out[16..48].copy_from_slice(&self.identity_seed);
        out[48..80].copy_from_slice(&local_id.0);
        out[80..82].copy_from_slice(&(self.discovered.iter().flatten().count() as u16).to_le_bytes());
        out[82..84].copy_from_slice(&(self.members.iter().flatten().count() as u16).to_le_bytes());
        out[88..96].copy_from_slice(&self.next_id.to_le_bytes());
        out[96..104].copy_from_slice(&self.audit_sequence.to_le_bytes());
        for (index, node) in self.discovered.iter().flatten().enumerate() {
            let at = NODE_RECORD_OFFSET + index * NODE_RECORD_BYTES;
            out[at..at + 32].copy_from_slice(&node.id.0);
            out[at + 32..at + 64].copy_from_slice(&node.public_key);
            out[at + 64] = trust_to_u8(node.trust);
            out[at + 65] = reachability_to_u8(node.reachability);
            out[at + 66..at + 68].copy_from_slice(&node.protocol_min.to_le_bytes());
            out[at + 68..at + 70].copy_from_slice(&node.protocol_max.to_le_bytes());
            out[at + 72..at + 80].copy_from_slice(&node.service_bits.to_le_bytes());
            out[at + 80..at + 88].copy_from_slice(&node.last_seen.to_le_bytes());
            for category in 0..12 { out[at + 88 + category] = decision_to_u8(node.policy.categories[category]); }
            out[at + 104..at + 112].copy_from_slice(&node.policy.scope.to_le_bytes());
            out[at + 112..at + 120].copy_from_slice(&node.policy.expires_at.to_le_bytes());
            out[at + 120..at + 124].copy_from_slice(&node.policy.version.to_le_bytes());
        }
        for (index, member) in self.members.iter().flatten().enumerate() {
            let at = MEMBER_RECORD_OFFSET + index * MEMBER_RECORD_BYTES;
            out[at..at + 32].copy_from_slice(&member.node.0);
            out[at + 32] = role_to_u8(member.role);
            out[at + 33] = member.enabled as u8;
            out[at + 40..at + 48].copy_from_slice(&member.joined_at.to_le_bytes());
            out[at + 48..at + 56].copy_from_slice(&member.last_heartbeat.to_le_bytes());
        }
        let checksum = state_crc32(&out[..NODE_STATE_BYTES - 4]);
        out[NODE_STATE_BYTES - 4..].copy_from_slice(&checksum.to_le_bytes());
        Ok(out)
    }

    // ------------------------=
    // FUNC: restore_state
    // DESC: Validates and restores the typed node trust object while keeping private identity material non-inspectable.
    // ------------------=
    pub fn restore_state(&mut self, input: &[u8]) -> Result<NodeId, NodeError> {
        if input.len() != NODE_STATE_BYTES || &input[..8] != NODE_STATE_MAGIC || u16::from_le_bytes([input[8], input[9]]) != 1 || u16::from_le_bytes([input[10], input[11]]) as usize != NODE_STATE_BYTES { return Err(NodeError::UnsupportedState); }
        let expected = u32::from_le_bytes(input[NODE_STATE_BYTES - 4..].try_into().map_err(|_| NodeError::StateCorrupt)?);
        if state_crc32(&input[..NODE_STATE_BYTES - 4]) != expected { return Err(NodeError::StateCorrupt); }
        self.identity_seed.copy_from_slice(&input[16..48]);
        let key_ref = self.crypto.initialize_seed(&self.identity_seed).map_err(map_crypto_error)?;
        let public = self.crypto.public_identity().map_err(map_crypto_error)?;
        let id = node_id_from_public(&public);
        if input[48..80] != id.0 { self.identity_seed.zeroize(); return Err(NodeError::IdentityMismatch); }
        self.key_ref = Some(key_ref);
        self.local_id = Some(id);
        self.discovered = [None; MAX_DISCOVERED_NODES];
        let count = u16::from_le_bytes([input[80], input[81]]) as usize;
        if count > MAX_DISCOVERED_NODES { return Err(NodeError::StateCorrupt); }
        for index in 0..count {
            let at = NODE_RECORD_OFFSET + index * NODE_RECORD_BYTES;
            let mut node_id = [0u8; 32]; node_id.copy_from_slice(&input[at..at + 32]);
            let mut public_key = [0u8; 32]; public_key.copy_from_slice(&input[at + 32..at + 64]);
            if node_id_from_public(&public_key).0 != node_id { return Err(NodeError::IdentityMismatch); }
            let mut categories = [PolicyDecision::Deny; 12];
            for category in 0..12 { categories[category] = u8_to_decision(input[at + 88 + category])?; }
            self.discovered[index] = Some(NodeDescriptor { id: NodeId(node_id), public_key, trust: u8_to_trust(input[at + 64])?, reachability: u8_to_reachability(input[at + 65])?, protocol_min: u16::from_le_bytes([input[at + 66], input[at + 67]]), protocol_max: u16::from_le_bytes([input[at + 68], input[at + 69]]), service_bits: u64::from_le_bytes(input[at + 72..at + 80].try_into().map_err(|_| NodeError::StateCorrupt)?), last_seen: u64::from_le_bytes(input[at + 80..at + 88].try_into().map_err(|_| NodeError::StateCorrupt)?), policy: NodeTrustPolicy { categories, scope: u64::from_le_bytes(input[at + 104..at + 112].try_into().map_err(|_| NodeError::StateCorrupt)?), expires_at: u64::from_le_bytes(input[at + 112..at + 120].try_into().map_err(|_| NodeError::StateCorrupt)?), version: u32::from_le_bytes(input[at + 120..at + 124].try_into().map_err(|_| NodeError::StateCorrupt)?) } });
        }
        self.members = [None; MAX_MESH_MEMBERS];
        let member_count = u16::from_le_bytes([input[82], input[83]]) as usize;
        if member_count > MAX_MESH_MEMBERS { return Err(NodeError::StateCorrupt); }
        for index in 0..member_count {
            let at = MEMBER_RECORD_OFFSET + index * MEMBER_RECORD_BYTES;
            let mut node_id = [0u8; 32]; node_id.copy_from_slice(&input[at..at + 32]);
            self.members[index] = Some(MeshMember { node: NodeId(node_id), role: u8_to_role(input[at + 32])?, enabled: input[at + 33] != 0, joined_at: u64::from_le_bytes(input[at + 40..at + 48].try_into().map_err(|_| NodeError::StateCorrupt)?), last_heartbeat: u64::from_le_bytes(input[at + 48..at + 56].try_into().map_err(|_| NodeError::StateCorrupt)?) });
        }
        self.next_id = u64::from_le_bytes(input[88..96].try_into().map_err(|_| NodeError::StateCorrupt)?).max(1);
        self.audit_sequence = u64::from_le_bytes(input[96..104].try_into().map_err(|_| NodeError::StateCorrupt)?);
        Ok(id)
    }

    // ------------------------=
    // FUNC: take_id
    // DESC: Allocates one nonzero runtime handle independently from stable node identity.
    // ------------------=
    fn take_id(&mut self) -> u64 { let id = self.next_id; self.next_id = self.next_id.wrapping_add(1).max(1); id }

    // ------------------------=
    // FUNC: record
    // DESC: Appends a structured bounded security record after the authoritative state change.
    // ------------------=
    pub(super) fn record(&mut self, event_type: u32, subject: NodeId, timestamp: u64, correlation_id: u64, result: u8) {
        self.audit_sequence = self.audit_sequence.wrapping_add(1).max(1);
        let index = (self.audit_sequence as usize - 1) % MAX_AUDIT_RECORDS;
        self.audit[index] = Some(AuditRecord { sequence: self.audit_sequence, event_type, subject, timestamp, correlation_id, result });
    }
}

// ------------------------=
// FUNC: node_id_from_public
// DESC: Derives a stable node identifier from a public key without encoding a storage location.
// ------------------=
fn node_id_from_public(public: &[u8; 32]) -> NodeId {
    let mut digest = Sha256::new();
    digest.update(b"InfinityOS NodeId v1");
    digest.update(public);
    NodeId(digest.finalize().into())
}

// ------------------------=
// FUNC: fingerprint
// DESC: Produces the compact human-verifiable fingerprint shown by Trusted UI.
// ------------------=
fn fingerprint(public: &[u8; 32]) -> [u8; 16] {
    let digest = Sha256::digest(public);
    let mut output = [0u8; 16];
    output.copy_from_slice(&digest[..16]);
    output
}

// ------------------------=
// FUNC: session_nonce
// DESC: Encodes the direction-local sequence; directional keys carry session identity, never local handles.
// ------------------=
fn session_nonce(sequence: u64) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[..4].copy_from_slice(b"IN92");
    nonce[4..].copy_from_slice(&sequence.to_le_bytes());
    nonce
}

// ------------------------=
// FUNC: state_crc32
// DESC: Computes the integrity checksum for the architecture-neutral node-state object.
// ------------------=
fn state_crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 { crc = (crc >> 1) ^ (0xedb8_8320u32 & 0u32.wrapping_sub(crc & 1)); }
    }
    !crc
}

// ------------------------=
// FUNC: trust_to_u8
// DESC: Encodes one stable trust-state discriminator for persistent state.
// ------------------=
fn trust_to_u8(value: TrustState) -> u8 { match value { TrustState::Discovered => 0, TrustState::Untrusted => 1, TrustState::PairingPending => 2, TrustState::Trusted => 3, TrustState::Restricted => 4, TrustState::Revoked => 5, TrustState::Blocked => 6, TrustState::Incompatible => 7 } }

// ------------------------=
// FUNC: u8_to_trust
// DESC: Rejects unknown persistent trust-state discriminators.
// ------------------=
fn u8_to_trust(value: u8) -> Result<TrustState, NodeError> { match value { 0 => Ok(TrustState::Discovered), 1 => Ok(TrustState::Untrusted), 2 => Ok(TrustState::PairingPending), 3 => Ok(TrustState::Trusted), 4 => Ok(TrustState::Restricted), 5 => Ok(TrustState::Revoked), 6 => Ok(TrustState::Blocked), 7 => Ok(TrustState::Incompatible), _ => Err(NodeError::StateCorrupt) } }

// ------------------------=
// FUNC: reachability_to_u8
// DESC: Encodes one stable reachability-state discriminator.
// ------------------=
fn reachability_to_u8(value: Reachability) -> u8 { match value { Reachability::Unknown => 0, Reachability::Online => 1, Reachability::Degraded => 2, Reachability::Offline => 3 } }

// ------------------------=
// FUNC: u8_to_reachability
// DESC: Rejects unknown persistent reachability-state discriminators.
// ------------------=
fn u8_to_reachability(value: u8) -> Result<Reachability, NodeError> { match value { 0 => Ok(Reachability::Unknown), 1 => Ok(Reachability::Online), 2 => Ok(Reachability::Degraded), 3 => Ok(Reachability::Offline), _ => Err(NodeError::StateCorrupt) } }

// ------------------------=
// FUNC: decision_to_u8
// DESC: Encodes one stable per-node policy decision.
// ------------------=
fn decision_to_u8(value: PolicyDecision) -> u8 { match value { PolicyDecision::Deny => 0, PolicyDecision::Allow => 1, PolicyDecision::SessionOnly => 2, PolicyDecision::Leased => 3 } }

// ------------------------=
// FUNC: u8_to_decision
// DESC: Rejects unknown persistent policy decisions.
// ------------------=
fn u8_to_decision(value: u8) -> Result<PolicyDecision, NodeError> { match value { 0 => Ok(PolicyDecision::Deny), 1 => Ok(PolicyDecision::Allow), 2 => Ok(PolicyDecision::SessionOnly), 3 => Ok(PolicyDecision::Leased), _ => Err(NodeError::StateCorrupt) } }

// ------------------------=
// FUNC: role_to_u8
// DESC: Encodes one stable mesh-role discriminator.
// ------------------=
fn role_to_u8(value: MeshRole) -> u8 { match value { MeshRole::Member => 0, MeshRole::Operator => 1, MeshRole::Gateway => 2, MeshRole::Compute => 3, MeshRole::Storage => 4 } }

// ------------------------=
// FUNC: u8_to_role
// DESC: Rejects unknown persistent mesh-role discriminators.
// ------------------=
fn u8_to_role(value: u8) -> Result<MeshRole, NodeError> { match value { 0 => Ok(MeshRole::Member), 1 => Ok(MeshRole::Operator), 2 => Ok(MeshRole::Gateway), 3 => Ok(MeshRole::Compute), 4 => Ok(MeshRole::Storage), _ => Err(NodeError::StateCorrupt) } }

// ------------------------=
// FUNC: map_crypto_error
// DESC: Converts protected-key failures into the stable native node error contract.
// ------------------=
fn map_crypto_error(error: CryptoError) -> NodeError {
    match error {
        CryptoError::EntropyUnavailable | CryptoError::KeyUnavailable => NodeError::EntropyUnavailable,
        CryptoError::InvalidSignature => NodeError::SignatureInvalid,
        CryptoError::InvalidCiphertext | CryptoError::OutputTooSmall => NodeError::AuthenticationFailed,
    }
}
