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
#[path = "iop_remote_payload.rs"]
mod payload;
use payload::Payload;
use super::storage_protocol::StorageOperationV1;

const MAGIC: &[u8; 4] = b"IOP9";
const FRAME_BYTES: usize = 128;
const STORAGE_FRAME_BYTES: usize = 48 + super::storage_protocol::OPERATION_BYTES;
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
    PersistenceFailed,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RemoteResult<T = NodeOperationV1> {
    pub request_id: u64,
    pub correlation_id: u64,
    pub causation_id: u64,
    pub result: Result<T, RemoteError>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthenticatedStorageRequest {
    pub local: NodeId,
    pub peer: NodeId,
    pub session_reference: [u8; 16],
    pub grant: u64,
    pub request_id: u64,
    pub correlation: u64,
    pub causation: u64,
    pub payload: StorageOperationV1,
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
    payload: Payload,
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
    result: Option<RemoteResult<Payload>>,
}
#[derive(Clone, Copy)]
struct ReplayStream {
    peer: NodeId,
    reference: [u8; 16],
    highest: u64,
}

pub struct RemoteState {
    inspection: super::super::node::inspection::InspectionCache,
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
            inspection: super::super::node::inspection::InspectionCache::new(),
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
                .map(|p| p.caller == caller && p.request.message.id == id && p.result.is_some()
                     && matches!(p.request.message.payload, Payload::Node(_)))
                .unwrap_or(false)
        })?;
        let result = self.pending[index].take()?.result?;
        Some(RemoteResult { request_id: result.request_id, correlation_id: result.correlation_id,
            causation_id: result.causation_id, result: result.result.and_then(Payload::node) })
    }
    // ------------------------=
    // FUNC: take_storage_result
    // DESC: Collects one caller-owned storage completion from the same bounded remote mailbox without consuming another service's result.
    // ------------------=
    pub fn take_storage_result(&mut self, caller: SecurityIdentity, id: u64) -> Option<RemoteResult<StorageOperationV1>> {
        let index = self.pending.iter().position(|p| p.as_ref().is_some_and(|p|
            p.caller == caller && p.request.message.id == id && p.result.is_some()
            && matches!(p.request.message.payload, Payload::Storage(_))))?;
        let result = self.pending[index].take()?.result?;
        Some(RemoteResult { request_id: result.request_id, correlation_id: result.correlation_id,
            causation_id: result.causation_id, result: result.result.and_then(Payload::storage) })
    }
    // ------------------------=
    // FUNC: incoming_count
    // DESC: Exposes bounded queue occupancy for operational diagnostics and dequeue-race tests.
    // ------------------=
    pub fn incoming_count(&self) -> usize {
        self.incoming.iter().flatten().count()
    }
    // ------------------------=
    // FUNC: discard
    // DESC: Removes only a caller-owned pending request when its submitting session ends or result retention expires.
    // ------------------=
    pub fn discard(&mut self, caller: SecurityIdentity, id: u64) {
        if let Some(index) = self.pending.iter().position(|entry| entry.as_ref().map(|p|
            p.caller == caller && p.request.message.id == id).unwrap_or(false)) {
            self.pending[index] = None;
        }
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
        self.request_remote_payload(capabilities, nodes, caller, capability, peer, grant,
            Payload::Node(payload), correlation, causation, now, deadline)
    }
    // ------------------------=
    // FUNC: request_remote_storage
    // DESC: Uses the existing secure-session, correlation and bounded-mailbox lifecycle for versioned storage IOP requests; no parallel RPC transport exists.
    // ------------------=
    pub fn request_remote_storage(&mut self, capabilities: &CapabilityManager, nodes: &NodeRuntime,
        caller: SecurityIdentity, capability: u64, peer: NodeId, grant: u64, payload: StorageOperationV1,
        correlation: u64, causation: u64, now: u64, deadline: u64) -> Result<u64, RemoteError> {
        payload.encode().map_err(|_| RemoteError::MalformedRequest)?;
        self.request_remote_payload(capabilities, nodes, caller, capability, peer, grant,
            Payload::Storage(payload), correlation, causation, now, deadline)
    }
    // ------------------------=
    // FUNC: request_remote_payload
    // DESC: Admits all remote native service payloads through one bounded, locally capability-validated request table.
    // ------------------=
    fn request_remote_payload(&mut self, capabilities: &CapabilityManager, nodes: &NodeRuntime,
        caller: SecurityIdentity, capability: u64, peer: NodeId, grant: u64, payload: Payload,
        correlation: u64, causation: u64, now: u64, deadline: u64) -> Result<u64, RemoteError> {
        if deadline <= now || deadline - now > MAX_LEASE {
            return Err(RemoteError::DeadlineExceeded);
        }
        capabilities
            .validate(
                capability,
                caller,
                CapabilityType::ServiceCall,
                payload.operation() as u64,
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
        capabilities: &CapabilityManager,
        nodes: &NodeRuntime,
        data: ReceivedData,
        now: u64,
    ) -> Result<(), RemoteError> {
        let result = self.admit_remote_node(capabilities, nodes, data, now);
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
        capabilities: &CapabilityManager,
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
                || !pending.request.message.payload.same_target(message.payload)
            {
                return Err(RemoteError::UnknownResponse);
            }
            // A reply is not a durable entitlement to disclose a result. Authority
            // can be revoked while the authenticated reply waits in the RX queue.
            if capabilities
                .validate(
                    pending.capability,
                    pending.caller,
                    CapabilityType::ServiceCall,
                    pending.request.message.payload.operation() as u64,
                    1,
                    0,
                    now,
                )
                .is_err()
            {
                pending.result = Some(RemoteResult {
                    request_id: message.id,
                    correlation_id: message.correlation,
                    causation_id: message.id,
                    result: Err(RemoteError::AccessDenied),
                });
                return Err(RemoteError::AccessDenied);
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
        self.execute_remote_node_inner(nodes, now, None);
    }
    // ------------------------=
    // FUNC: execute_remote_node_durable
    // DESC: Uses the same dequeue-time checks with a real durable writer; returns a notice only after commit for post-commit IEF.
    // ------------------=
    pub fn execute_remote_node_durable(
        &mut self,
        nodes: &mut NodeRuntime,
        now: u64,
        persist: &mut dyn FnMut(&[u8; super::super::node::types::NODE_STATE_BYTES]) -> bool,
    ) -> Option<super::super::node::control::CommittedControl> {
        self.execute_remote_node_inner(nodes, now, Some(persist))
    }
    // ------------------------=
    // FUNC: execute_remote_node_inner
    // DESC: Reserves response capacity and authorizes the actual execution before staging any durable mutation.
    // ------------------=
    fn execute_remote_node_inner(
        &mut self,
        nodes: &mut NodeRuntime,
        now: u64,
        mut persist: Option<
            &mut dyn FnMut(&[u8; super::super::node::types::NODE_STATE_BYTES]) -> bool,
        >,
    ) -> Option<super::super::node::control::CommittedControl> {
        let Some(output) = self.remote.responses.iter().position(Option::is_none) else {
            return None;
        };
        let Some(index) = self.remote.incoming.iter().position(|r| r.is_some_and(|r| matches!(r.message.payload, Payload::Node(_)))) else {
            return None;
        };
        let mut request = self.remote.incoming[index].take().unwrap();
        let mut committed = None;
        let result = validate_authority(nodes, &request, now).and_then(|_| {
            let payload = request.message.payload.node()?;
            let op = operation(payload.operation)?;
            if !is_read(op.machine_id()) {
                if let Some(writer) = persist.as_mut() {
                    let (response, notice) = nodes
                        .commit_control(
                            op,
                            payload,
                            now,
                            request.message.correlation,
                            request.message.id,
                            |bytes| writer(bytes),
                        )
                        .map_err(|e| match e {
                            super::super::node::control::CommitError::PersistenceFailed => {
                                RemoteError::PersistenceFailed
                            }
                            _ => RemoteError::InvalidState,
                        })?;
                    committed = Some(notice);
                    return Ok(response);
                }
                return Err(RemoteError::ServiceUnavailable);
            }
            if matches!(op, OperationId::NodeInspect | OperationId::NodeSessionInspect | OperationId::NodeDomainInspect) {
                let mut payload = payload;
                if op == OperationId::NodeSessionInspect {
                    // The authenticated wire reference selects this session, never a sender-local handle.
                    if payload.lease_deadline != 0 { return Err(RemoteError::CapabilityScopeDenied); }
                    payload.lease_deadline = nodes.sessions().iter().flatten().find(|s| s.peer == request.peer && s.protocol_reference == request.reference).ok_or(RemoteError::SessionNotFound)?.id;
                }
                return self.remote.inspection.inspect(nodes, request.reference, op, payload, now).map_err(|_| RemoteError::InvalidState);
            }
            super::execute_node_operation(
                nodes,
                op,
                payload,
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
            request.message.payload = Payload::Node(value);
        }
        request.expires = now.saturating_add(5);
        self.remote.responses[output] = Some(request);
        committed
    }
    // ------------------------=
    // FUNC: execute_remote_storage
    // DESC: Revalidates every storage operation, including chunks, after dequeue and reserves reply capacity before invoking the owning object-authority/transaction service.
    // ------------------=
    pub fn execute_remote_storage<T>(&mut self, nodes: &mut NodeRuntime, now: u64,
        execute: impl FnOnce(AuthenticatedStorageRequest) -> Result<(StorageOperationV1, T), RemoteError>) -> Option<T> {
        let output = self.remote.responses.iter().position(Option::is_none)?;
        let index = self.remote.incoming.iter().position(|r| r.is_some_and(|r| matches!(r.message.payload, Payload::Storage(_))))?;
        let mut request = self.remote.incoming[index].take()?;
        let mut committed = None;
        let result = validate_authority(nodes, &request, now).and_then(|_| {
            let payload = request.message.payload.storage()?;
            let (mut response, notice) = execute(AuthenticatedStorageRequest { local: nodes.local_id().ok_or(RemoteError::InvalidState)?, peer: request.peer,
                session_reference: request.reference, grant: request.message.grant,
                request_id: request.message.id, correlation: request.message.correlation,
                causation: request.message.causation, payload })?;
            if response.operation==super::storage_protocol::Operation::PoolMetadata && matches!(payload.value,2|3|7) {
                if response.length!=32||response.offset>2{return Err(RemoteError::RemoteFailure)}
                let mut transcript=[0;48];transcript[..8].copy_from_slice(b"INFPMACK");transcript[8]=response.offset as u8;transcript[9]=u8::from(matches!(payload.value,3|7));transcript[16..].copy_from_slice(&response.data[..32]);
                response.data=nodes.sign_storage_metadata(nodes.local_id().ok_or(RemoteError::InvalidState)?,&transcript).map_err(|_|RemoteError::RemoteFailure)?;response.length=64;
            }
            if response.operation==super::storage_protocol::Operation::PoolMetadata && payload.value==25 {
                if response.length!=64{return Err(RemoteError::RemoteFailure)}
                let local=nodes.local_id().ok_or(RemoteError::InvalidState)?;
                let mut transcript=[0;128];transcript[..8].copy_from_slice(b"INFPRAV1");transcript[8..40].copy_from_slice(&response.data[..32]);transcript[40..72].copy_from_slice(&local.0);transcript[72..104].copy_from_slice(&response.data[32..64]);
                response.data=nodes.sign_storage_metadata(local,&transcript).map_err(|_|RemoteError::RemoteFailure)?;
            }
            response.encode().map_err(|_| RemoteError::RemoteFailure)?;
            if !request.message.payload.same_target(Payload::Storage(response)) { return Err(RemoteError::RemoteFailure); }
            committed = Some(notice);
            Ok(response)
        });
        if result.is_ok() { self.remote.executed = self.remote.executed.saturating_add(1); }
        nodes.record(0xdb01, request.peer, now, request.message.correlation,
            result.as_ref().err().map(|e| *e as u8).unwrap_or(0));
        request.message.kind = 2;
        request.message.causation = request.message.id;
        request.message.error = result.as_ref().err().map(|e| *e as u8).unwrap_or(0);
        if let Ok(payload) = result { request.message.payload = Payload::Storage(payload); }
        request.expires = now.saturating_add(5);
        self.remote.responses[output] = Some(request);
        committed
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
            let _ = self.receive_remote_node(capabilities, nodes, data, now);
        }
        for slot in &mut self.remote.responses {
            let Some(request) = *slot else {
                continue;
            };
            if now >= request.expires || validate_session(nodes, &request, now).is_err() {
                *slot = None;
                continue;
            }
            if let Ok((bytes, length)) = encode(request.message) {
                if trust.send_data(nodes, request.peer, &bytes[..length], false, now).is_ok() {
                    *slot = None;
                }
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
                    pending.request.message.payload.operation() as u64,
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
            if let Ok((bytes, length)) = encode(message) {
                if trust.send_data(nodes, p.request.peer, &bytes[..length], false, now).is_ok() {
                    p.sent = true;
                }
            }
        }
    }
}

// ------------------------=
// FUNC: operation
// DESC: Maps only supported remote service operations; pairing consent and unsynchronized membership cannot be remotely bypassed.
// ------------------=
pub(super) fn operation(value: u32) -> Result<OperationId, RemoteError> {
    use OperationId::*;
    [
        NodeInspect,
        NodeSessionInspect,
        NodeDomainInspect,
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
    let selected = match r.message.payload {
        Payload::Node(p) if p.operation == OperationId::NodeDomainInspect.machine_id() =>
            nodes.local_id().map(|local| super::super::node::membership::domain_id(local, r.peer).0 == p.node_id).unwrap_or(false),
        Payload::Node(p) => p.node_id == r.peer.0,
        // Storage's full ObjectId is validated by its owning service, never
        // reinterpreted as a node identity or replaced with a 64-bit scope.
        Payload::Storage(_) => true,
    };
    if !selected {
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
            r.message.payload.operation(),
            r.message.payload.scope(),
            1,
            now,
        )
        .map_err(|e| match e {
            NodeError::CapabilityRevoked => RemoteError::CapabilityRevoked,
            NodeError::CapabilityExpired => RemoteError::CapabilityExpired,
            _ => RemoteError::CapabilityScopeDenied,
        })?;
    // Category 0 is peer metadata; category 1 is explicitly delegated node control.
    let category = match r.message.payload {
        Payload::Node(p) => if is_read(p.operation) { 0 } else { 1 },
        Payload::Storage(_) => 0,
    };
    if peer.policy.scope != r.message.payload.scope() {
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
// DESC: Preserves the 128-byte node schema and adds a bounded versioned storage payload inside the same authenticated IOP envelope.
// ------------------=
fn encode(m: Envelope) -> Result<([u8; 192], usize), RemoteError> {
    let mut out = [0; 192];
    out[..4].copy_from_slice(MAGIC);
    out[5] = m.kind;
    out[6] = m.error;
    out[8..16].copy_from_slice(&m.id.to_le_bytes());
    out[16..24].copy_from_slice(&m.correlation.to_le_bytes());
    out[24..32].copy_from_slice(&m.causation.to_le_bytes());
    out[32..40].copy_from_slice(&m.grant.to_le_bytes());
    out[40..44].copy_from_slice(&m.lease.to_le_bytes());
    let length = match m.payload {
        Payload::Node(p) => { out[4] = 1; out[48..FRAME_BYTES].copy_from_slice(&p.encode()); FRAME_BYTES },
        Payload::Storage(p) => { out[4] = 2; out[48..STORAGE_FRAME_BYTES].copy_from_slice(
            &p.encode().map_err(|_| RemoteError::MalformedRequest)?); STORAGE_FRAME_BYTES },
    };
    Ok((out, length))
}
// ------------------------=
// FUNC: decode
// DESC: Rejects malformed, oversized, reserved-field and unsupported-version frames before any queue or authority mutation.
// ------------------=
fn decode(bytes: &[u8]) -> Result<Envelope, RemoteError> {
    if !matches!(bytes.len(), FRAME_BYTES | STORAGE_FRAME_BYTES)
        || &bytes[..4] != MAGIC
        || bytes[7] != 0
        || bytes[44..48] != [0; 4]
    {
        return Err(RemoteError::MalformedRequest);
    }
    if !matches!(bytes[4], 1 | 2) {
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
    let payload = match bytes[4] {
        1 if bytes.len() == FRAME_BYTES => Payload::Node(NodeOperationV1::decode(&bytes[48..]).map_err(|_| RemoteError::MalformedRequest)?),
        2 if bytes.len() == STORAGE_FRAME_BYTES => Payload::Storage(StorageOperationV1::decode(&bytes[48..]).map_err(|_| RemoteError::MalformedRequest)?),
        _ => return Err(RemoteError::MalformedRequest),
    };
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
        PersistenceFailed,
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
    matches!(operation, 0xd002 | 0xd006 | 0xd014 | 0xd064 | 0xd065 | 0xd067 | 0xd068)
}

#[cfg(test)]
#[path = "iop_remote_tests.rs"]
mod tests;
