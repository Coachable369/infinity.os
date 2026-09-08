//! Bounded remote Node IOP admission and execution inside the normal IOP router.
//! NodeTransport authenticates bytes; it never invokes the node executor.
use super::super::node::{
    types::{NodeError, NodeId, PolicyDecision, SessionState, TrustState},
    wire_trust::{ReceivedData, WireTrust},
    NodeRuntime,
};
use super::{
    CapabilityManager, CapabilityType, IopRouter, NodeOperationV1, OperationId, SecurityIdentity,
};

const MAGIC: &[u8; 4] = b"IOP9";
const FRAME_BYTES: usize = 128;
const CAPACITY: usize = 8;
const MAX_LEASE: u64 = 30;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum RemoteError {
    MalformedRequest = 1,
    UnsupportedSchemaVersion,
    UnsupportedOperation,
    SessionNotFound,
    TransportClosed,
    TrustRequired,
    TrustRevoked,
    NodeBlocked,
    CapabilityRequired,
    CapabilityExpired,
    CapabilityRevoked,
    CapabilityScopeDenied,
    PolicyDenied,
    AccessDenied,
    DeadlineExceeded,
    QueueFull,
    InvalidState,
    NotFound,
    Conflict,
    ReplayRejected,
    UnknownResponse,
    RemoteFailure,
    ServiceUnavailable,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RemoteResult {
    pub request_id: u64,
    pub correlation_id: u64,
    pub causation_id: u64,
    pub result: Result<NodeOperationV1, RemoteError>,
}

#[derive(Clone, Copy)]
struct Envelope {
    kind: u8,
    error: u8,
    id: u64,
    correlation: u64,
    causation: u64,
    grant: u64,
    lease: u32,
    payload: NodeOperationV1,
}
#[derive(Clone, Copy)]
struct Request {
    peer: NodeId,
    reference: [u8; 16],
    message: Envelope,
    expires: u64,
}
#[derive(Clone, Copy)]
struct Pending {
    request: Request,
    caller: SecurityIdentity,
    capability: u64,
    sent: bool,
    result: Option<RemoteResult>,
}
#[derive(Clone, Copy)]
struct ReplayStream {
    peer: NodeId,
    reference: [u8; 16],
    highest: u64,
}

pub struct RemoteState {
    mutation_service_ready: bool,
    next: u64,
    pending: [Option<Pending>; CAPACITY],
    incoming: [Option<Request>; CAPACITY],
    responses: [Option<Request>; CAPACITY],
    streams: [Option<ReplayStream>; 4],
    pub rejected: u64,
    pub executed: u64,
}
impl RemoteState {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates fixed remote correlation, admission, response and replay tables without ambient authority.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            mutation_service_ready: false,
            next: 1,
            pending: [None; CAPACITY],
            incoming: [None; CAPACITY],
            responses: [None; CAPACITY],
            streams: [None; 4],
            rejected: 0,
            executed: 0,
        }
    }
    // ------------------------=
    // FUNC: set_mutation_service_ready
    // DESC: Enables writes only when the owning service supplies its commit lifecycle; this grants no peer authority.
    // ------------------=
    pub fn set_mutation_service_ready(&mut self, ready: bool) {
        self.mutation_service_ready = ready;
    }
    // ------------------------=
    // FUNC: take_result
    // DESC: Returns one caller-owned typed completion and releases its bounded active slot.
    // ------------------=
    pub fn take_result(&mut self, caller: SecurityIdentity, id: u64) -> Option<RemoteResult> {
        let index = self.pending.iter().position(|p| {
            p.as_ref()
                .map(|p| p.caller == caller && p.request.message.id == id && p.result.is_some())
                .unwrap_or(false)
        })?;
        self.pending[index].take()?.result
    }
    // ------------------------=
    // FUNC: incoming_count
    // DESC: Exposes bounded queue occupancy for operational diagnostics and dequeue-race tests.
    // ------------------=
    pub fn incoming_count(&self) -> usize {
        self.incoming.iter().flatten().count()
    }
}

impl IopRouter {
    // ------------------------=
    // FUNC: request_remote_node
    // DESC: Validates local IOP authority before correlating a versioned request to an authenticated peer session.
    // ------------------=
    pub fn request_remote_node(
        &mut self,
        capabilities: &CapabilityManager,
        nodes: &NodeRuntime,
        caller: SecurityIdentity,
        capability: u64,
        peer: NodeId,
        grant: u64,
        payload: NodeOperationV1,
        correlation: u64,
        causation: u64,
        now: u64,
        deadline: u64,
    ) -> Result<u64, RemoteError> {
        operation(payload.operation)?;
        NodeOperationV1::decode(&payload.encode()).map_err(|_| RemoteError::MalformedRequest)?;
        if deadline <= now || deadline - now > MAX_LEASE {
            return Err(RemoteError::DeadlineExceeded);
        }
        capabilities
            .validate(
                capability,
                caller,
                CapabilityType::ServiceCall,
                payload.operation as u64,
                1,
                0,
                now,
            )
            .map_err(|_| RemoteError::AccessDenied)?;
        let reference = nodes
            .sessions()
            .iter()
            .flatten()
            .find(|s| s.peer == peer && s.state == SessionState::Established && now < s.expires_at)
            .ok_or(RemoteError::SessionNotFound)?
            .protocol_reference;
        let index = self
            .remote
            .pending
            .iter()
            .position(Option::is_none)
            .ok_or(RemoteError::QueueFull)?;
        let id = self.remote.next;
        self.remote.next = id.checked_add(1).ok_or(RemoteError::QueueFull)?;
        self.remote.pending[index] = Some(Pending {
            request: Request {
                peer,
                reference,
                expires: deadline,
                message: Envelope {
                    kind: 1,
                    error: 0,
                    id,
                    correlation,
                    causation,
                    grant,
                    lease: (deadline - now) as u32,
                    payload,
                },
            },
            caller,
            capability,
            sent: false,
            result: None,
        });
        Ok(id)
    }
    // ------------------------=
    // FUNC: receive_remote_node
    // DESC: Admits authenticated protocol data into the normal router; never executes at admission time.
    // ------------------=
    pub fn receive_remote_node(
        &mut self,
        nodes: &NodeRuntime,
        data: ReceivedData,
        now: u64,
    ) -> Result<(), RemoteError> {
        let result = self.admit_remote_node(nodes, data, now);
        if result.is_err() {
            self.remote.rejected = self.remote.rejected.saturating_add(1);
        }
        result
    }
    // ------------------------=
    // FUNC: admit_remote_node
    // DESC: Binds source and target through the authenticated session, enforces framing, correlation and request replay limits.
    // ------------------=
    fn admit_remote_node(
        &mut self,
        nodes: &NodeRuntime,
        data: ReceivedData,
        now: u64,
    ) -> Result<(), RemoteError> {
        if data.length > data.bytes.len() {
            return Err(RemoteError::MalformedRequest);
        }
        let message = decode(&data.bytes[..data.length])?;
        let request = Request {
            peer: data.peer,
            reference: data.reference,
            message,
            expires: now.saturating_add(message.lease as u64),
        };
        validate_session(nodes, &request, now)?;
        if message.kind == 2 {
            let pending = self
                .remote
                .pending
                .iter_mut()
                .flatten()
                .find(|p| p.request.message.id == message.id)
                .ok_or(RemoteError::UnknownResponse)?;
            if pending.result.is_some() {
                return Err(RemoteError::ReplayRejected);
            }
            if now >= pending.request.expires {
                return Err(RemoteError::DeadlineExceeded);
            }
            if !pending.sent
                || pending.request.peer != data.peer
                || pending.request.reference != data.reference
                || pending.request.message.correlation != message.correlation
                || pending.request.message.id != message.causation
                || pending.request.message.payload.operation != message.payload.operation
            {
                return Err(RemoteError::UnknownResponse);
            }
            let result = if message.error == 0 {
                Ok(message.payload)
            } else {
                Err(error_from_byte(message.error)?)
            };
            pending.result = Some(RemoteResult {
                request_id: message.id,
                correlation_id: message.correlation,
                causation_id: message.causation,
                result,
            });
            return Ok(());
        }
        let Some(slot) = self.remote.incoming.iter().position(Option::is_none) else {
            if let Some(output) = self.remote.responses.iter().position(Option::is_none) {
                let mut rejected = request;
                rejected.message.kind = 2;
                rejected.message.causation = rejected.message.id;
                rejected.message.error = RemoteError::QueueFull as u8;
                rejected.expires = now.saturating_add(5);
                self.remote.responses[output] = Some(rejected);
            }
            return Err(RemoteError::QueueFull);
        };
        let stream = if let Some(index) = self.remote.streams.iter().position(|s| {
            s.map(|s| s.peer == data.peer && s.reference == data.reference)
                .unwrap_or(false)
        }) {
            index
        } else {
            self.remote
                .streams
                .iter()
                .position(|s| {
                    s.map(|s| {
                        !nodes.sessions().iter().flatten().any(|live| {
                            live.peer == s.peer
                                && live.protocol_reference == s.reference
                                && live.state == SessionState::Established
                                && now < live.expires_at
                        })
                    })
                    .unwrap_or(true)
                })
                .ok_or(RemoteError::QueueFull)?
        };
        if let Some(previous) = self.remote.streams[stream] {
            if previous.peer == data.peer
                && previous.reference == data.reference
                && message.id <= previous.highest
            {
                return Err(RemoteError::ReplayRejected);
            }
        }
        self.remote.streams[stream] = Some(ReplayStream {
            peer: data.peer,
            reference: data.reference,
            highest: message.id,
        });
        self.remote.incoming[slot] = Some(request);
        Ok(())
    }
    // ------------------------=
    // FUNC: execute_remote_node
    // DESC: Dequeues one request and revalidates live session, current trust, exact grant and policy before the shared service executor.
    // ------------------=
    pub fn execute_remote_node(&mut self, nodes: &mut NodeRuntime, now: u64) {
        let Some(output) = self.remote.responses.iter().position(Option::is_none) else {
            return;
        };
        let Some(index) = self.remote.incoming.iter().position(Option::is_some) else {
            return;
        };
        let mut request = self.remote.incoming[index].take().unwrap();
        let result = validate_authority(nodes, &request, now).and_then(|_| {
            let op = operation(request.message.payload.operation)?;
            if !is_read(op.machine_id()) && !self.remote.mutation_service_ready {
                return Err(RemoteError::ServiceUnavailable);
            }
            super::execute_node_operation(
                nodes,
                op,
                request.message.payload,
                now,
                request.message.correlation,
            )
            .map_err(|e| match e {
                super::IopError::AccessDenied => RemoteError::AccessDenied,
                _ => RemoteError::InvalidState,
            })
        });
        if result.is_ok() {
            self.remote.executed = self.remote.executed.saturating_add(1);
        }
        nodes.record(
            0xdb01,
            request.peer,
            now,
            request.message.correlation,
            result.as_ref().err().map(|e| *e as u8).unwrap_or(0),
        );
        request.message.kind = 2;
        request.message.causation = request.message.id;
        request.message.error = result.as_ref().err().map(|e| *e as u8).unwrap_or(0);
        if let Ok(value) = result {
            request.message.payload = value;
        }
        request.expires = now.saturating_add(5);
        self.remote.responses[output] = Some(request);
    }
    // ------------------------=
    // FUNC: poll_remote_node
    // DESC: Performs bounded receive/send and completion cleanup without waiting for peers or executing service mutations in transport.
    // ------------------=
    pub fn poll_remote_node(
        &mut self,
        capabilities: &CapabilityManager,
        nodes: &mut NodeRuntime,
        trust: &mut WireTrust,
        now: u64,
    ) {
        // Completed entries form a bounded, leased mailbox, not immortal active requests.
        for slot in &mut self.remote.pending {
            if slot
                .as_ref()
                .map(|p| p.result.is_some() && now >= p.request.expires.saturating_add(MAX_LEASE))
                .unwrap_or(false)
            {
                *slot = None;
            }
        }
        if let Some(data) = trust.receive_protocol(MAGIC) {
            let _ = self.receive_remote_node(nodes, data, now);
        }
        for slot in &mut self.remote.responses {
            let Some(request) = *slot else {
                continue;
            };
            if now >= request.expires || validate_session(nodes, &request, now).is_err() {
                *slot = None;
                continue;
            }
            if trust
                .send_data(nodes, request.peer, &encode(request.message), false, now)
                .is_ok()
            {
                *slot = None;
            }
            break;
        }
        for pending in self.remote.pending.iter_mut().flatten() {
            if pending.result.is_some() {
                continue;
            }
            let error = if now >= pending.request.expires {
                Some(RemoteError::DeadlineExceeded)
            } else if validate_session(nodes, &pending.request, now).is_err() {
                Some(RemoteError::TransportClosed)
            } else if capabilities
                .validate(
                    pending.capability,
                    pending.caller,
                    CapabilityType::ServiceCall,
                    pending.request.message.payload.operation as u64,
                    1,
                    0,
                    now,
                )
                .is_err()
            {
                Some(RemoteError::AccessDenied)
            } else {
                None
            };
            if let Some(error) = error {
                let m = pending.request.message;
                pending.result = Some(RemoteResult {
                    request_id: m.id,
                    correlation_id: m.correlation,
                    causation_id: m.id,
                    result: Err(error),
                });
            }
        }
        if let Some(p) = self
            .remote
            .pending
            .iter_mut()
            .flatten()
            .find(|p| !p.sent && p.result.is_none())
        {
            let mut message = p.request.message;
            message.lease = (p.request.expires - now) as u32;
            if trust
                .send_data(nodes, p.request.peer, &encode(message), false, now)
                .is_ok()
            {
                p.sent = true;
            }
        }
    }
}

// ------------------------=
// FUNC: operation
// DESC: Maps only supported remote service operations; pairing consent and unsynchronized membership cannot be remotely bypassed.
// ------------------=
fn operation(value: u32) -> Result<OperationId, RemoteError> {
    use OperationId::*;
    [
        NodeInspect,
        NodeTrustRead,
        NodeTrustUpdate,
        NodeRevokeTrust,
        NodeBlock,
        NodeUnblock,
        NodePolicyRead,
        NodePolicyUpdate,
        NodeHealth,
        NodeDiagnostics,
    ]
    .into_iter()
    .find(|o| o.machine_id() == value)
    .ok_or(RemoteError::UnsupportedOperation)
}
// ------------------------=
// FUNC: validate_session
// DESC: Requires the same live authenticated protocol reference; runtime-local handles are never transported.
// ------------------=
fn validate_session(nodes: &NodeRuntime, r: &Request, now: u64) -> Result<(), RemoteError> {
    if !nodes.sessions().iter().flatten().any(|s| {
        s.peer == r.peer
            && s.protocol_reference == r.reference
            && s.state == SessionState::Established
            && now < s.expires_at
    }) {
        return Err(RemoteError::SessionNotFound);
    }
    Ok(())
}
// ------------------------=
// FUNC: validate_authority
// DESC: Rechecks authority after dequeue, including self-scoped subject binding and deny-by-default node policy.
// ------------------=
fn validate_authority(nodes: &NodeRuntime, r: &Request, now: u64) -> Result<(), RemoteError> {
    if now >= r.expires {
        return Err(RemoteError::DeadlineExceeded);
    }
    validate_session(nodes, r, now)?;
    let peer = nodes
        .discovered_nodes()
        .iter()
        .flatten()
        .find(|p| p.id == r.peer)
        .ok_or(RemoteError::NotFound)?;
    match peer.trust {
        TrustState::Trusted => (),
        TrustState::Revoked => return Err(RemoteError::TrustRevoked),
        TrustState::Blocked => return Err(RemoteError::NodeBlocked),
        _ => return Err(RemoteError::TrustRequired),
    }
    if r.message.payload.node_id != r.peer.0 {
        return Err(RemoteError::CapabilityScopeDenied);
    }
    if !nodes
        .remote_grants()
        .iter()
        .flatten()
        .any(|g| g.id == r.message.grant)
    {
        return Err(RemoteError::CapabilityRequired);
    }
    nodes
        .authorize_remote(
            r.message.grant,
            r.peer,
            r.message.payload.operation,
            r.message.payload.scope,
            1,
            now,
        )
        .map_err(|e| match e {
            NodeError::CapabilityRevoked => RemoteError::CapabilityRevoked,
            NodeError::CapabilityExpired => RemoteError::CapabilityExpired,
            _ => RemoteError::CapabilityScopeDenied,
        })?;
    // Category 0 is peer metadata; category 1 is explicitly delegated node control.
    let category = if is_read(r.message.payload.operation) {
        0
    } else {
        1
    };
    if peer.policy.scope != r.message.payload.scope {
        return Err(RemoteError::PolicyDenied);
    }
    match peer.policy.categories[category] {
        PolicyDecision::Allow | PolicyDecision::SessionOnly => (),
        PolicyDecision::Leased if now < peer.policy.expires_at => (),
        _ => return Err(RemoteError::PolicyDenied),
    }
    Ok(())
}
// ------------------------=
// FUNC: encode
// DESC: Encodes a 128-byte architecture-neutral schema inside the existing authenticated 192-byte session payload limit.
// ------------------=
fn encode(m: Envelope) -> [u8; FRAME_BYTES] {
    let mut out = [0; FRAME_BYTES];
    out[..4].copy_from_slice(MAGIC);
    out[4] = 1;
    out[5] = m.kind;
    out[6] = m.error;
    out[8..16].copy_from_slice(&m.id.to_le_bytes());
    out[16..24].copy_from_slice(&m.correlation.to_le_bytes());
    out[24..32].copy_from_slice(&m.causation.to_le_bytes());
    out[32..40].copy_from_slice(&m.grant.to_le_bytes());
    out[40..44].copy_from_slice(&m.lease.to_le_bytes());
    out[48..].copy_from_slice(&m.payload.encode());
    out
}
// ------------------------=
// FUNC: decode
// DESC: Rejects malformed, oversized, reserved-field and unsupported-version frames before any queue or authority mutation.
// ------------------=
fn decode(bytes: &[u8]) -> Result<Envelope, RemoteError> {
    if bytes.len() != FRAME_BYTES
        || &bytes[..4] != MAGIC
        || bytes[7] != 0
        || bytes[44..48] != [0; 4]
    {
        return Err(RemoteError::MalformedRequest);
    }
    if bytes[4] != 1 {
        return Err(RemoteError::UnsupportedSchemaVersion);
    }
    if !matches!(bytes[5], 1 | 2) || (bytes[5] == 1 && bytes[6] != 0) {
        return Err(RemoteError::MalformedRequest);
    }
    let id = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    let lease = u32::from_le_bytes(bytes[40..44].try_into().unwrap());
    if id == 0 || lease == 0 || lease as u64 > MAX_LEASE {
        return Err(RemoteError::DeadlineExceeded);
    }
    let payload =
        NodeOperationV1::decode(&bytes[48..]).map_err(|_| RemoteError::MalformedRequest)?;
    Ok(Envelope {
        kind: bytes[5],
        error: bytes[6],
        id,
        correlation: u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
        causation: u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
        grant: u64::from_le_bytes(bytes[32..40].try_into().unwrap()),
        lease,
        payload,
    })
}
// ------------------------=
// FUNC: error_from_byte
// DESC: Decodes only explicit stable error discriminants without unsafe enum conversion.
// ------------------=
fn error_from_byte(value: u8) -> Result<RemoteError, RemoteError> {
    use RemoteError::*;
    [
        MalformedRequest,
        UnsupportedSchemaVersion,
        UnsupportedOperation,
        SessionNotFound,
        TransportClosed,
        TrustRequired,
        TrustRevoked,
        NodeBlocked,
        CapabilityRequired,
        CapabilityExpired,
        CapabilityRevoked,
        CapabilityScopeDenied,
        PolicyDenied,
        AccessDenied,
        DeadlineExceeded,
        QueueFull,
        InvalidState,
        NotFound,
        Conflict,
        ReplayRejected,
        UnknownResponse,
        RemoteFailure,
        ServiceUnavailable,
    ]
    .into_iter()
    .find(|e| *e as u8 == value)
    .ok_or(MalformedRequest)
}

// ------------------------=
// FUNC: is_read
// DESC: Separates metadata reads from mutations that require a ready commit and publication lifecycle.
// ------------------=
fn is_read(operation: u32) -> bool {
    matches!(operation, 0xd002 | 0xd006 | 0xd065 | 0xd067 | 0xd068)
}
