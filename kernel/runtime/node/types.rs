//! Stable, architecture-neutral machine types for node trust and mesh membership.

pub const MAX_DISCOVERED_NODES: usize = 16;
pub const MAX_TRUSTED_NODES: usize = 16;
pub const MAX_PAIRINGS: usize = 4;
pub const MAX_SESSIONS: usize = 16;
pub const MAX_REMOTE_GRANTS: usize = 32;
pub const MAX_MESH_MEMBERS: usize = 16;
pub const MAX_AUDIT_RECORDS: usize = 64;
pub const LEGACY_NODE_STATE_BYTES: usize = 4096;
pub const NODE_STATE_BYTES: usize = 8192;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct NodeId(pub [u8; 32]);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrustState { Discovered, Untrusted, PairingPending, Trusted, Restricted, Revoked, Blocked, Incompatible }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reachability { Unknown, Online, Degraded, Offline }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PairingState { AwaitingConfirmation, Confirmed, Cancelled, Expired, Failed }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SessionState { Handshaking, Established, RekeyRequired, Closed, Failed }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MeshRole { Member, Operator, Gateway, Compute, Storage }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PolicyDecision { Deny, Allow, SessionOnly, Leased }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NodeTrustPolicy {
    pub categories: [PolicyDecision; 12],
    pub scope: u64,
    pub expires_at: u64,
    pub version: u32,
}

impl NodeTrustPolicy {
    // ------------------------=
    // FUNC: deny_all
    // DESC: Creates a policy with no remote authority in any category.
    // ------------------=
    pub const fn deny_all() -> Self {
        Self { categories: [PolicyDecision::Deny; 12], scope: 0, expires_at: 0, version: 1 }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NodeDescriptor {
    pub id: NodeId,
    pub public_key: [u8; 32],
    pub protocol_min: u16,
    pub protocol_max: u16,
    pub service_bits: u64,
    pub last_seen: u64,
    pub reachability: Reachability,
    pub trust: TrustState,
    pub policy: NodeTrustPolicy,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pairing {
    pub id: u64,
    pub peer: NodeId,
    pub fingerprint: [u8; 16],
    pub verification_code: u32,
    pub expires_at: u64,
    pub state: PairingState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SecureSession {
    pub id: u64,
    pub peer: NodeId,
    pub state: SessionState,
    pub protocol_reference: [u8; 16],
    pub(super) tx_key: [u8; 32],
    pub(super) rx_key: [u8; 32],
    pub send_sequence: u64,
    pub receive_sequence: u64,
    pub expires_at: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RemoteGrant {
    pub id: u64,
    pub peer: NodeId,
    pub operation: u32,
    pub scope: u64,
    pub rights: u32,
    pub expires_at: u64,
    pub revoked: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MeshMember {
    pub node: NodeId,
    pub role: MeshRole,
    pub joined_at: u64,
    pub last_heartbeat: u64,
    pub enabled: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AuditRecord {
    pub sequence: u64,
    pub event_type: u32,
    pub subject: NodeId,
    pub timestamp: u64,
    pub correlation_id: u64,
    pub result: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NodeError {
    EntropyUnavailable, InvalidAdvertisement, SignatureInvalid, UnsupportedVersion,
    UnknownNode, NotTrusted, Blocked, PairingNotFound, PairingExpired, VerificationMismatch,
    SessionNotFound, SessionExpired, ReplayDetected, AuthenticationFailed, CapabilityDenied,
    CapabilityExpired, CapabilityRevoked, MeshFull, AlreadyMember, NotMember, ResourceLimit,
    IdentityMismatch, DuplicateIdentity, ProtocolDowngrade, StateCorrupt, UnsupportedState,
    HumanApprovalRequired,
}
