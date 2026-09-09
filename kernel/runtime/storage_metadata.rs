//! Bounded native metadata publication and fresh quorum reads. Network progress
//! runs only from the runtime pump; GUI/Console retain their exact session owner.
use super::*;
use fabric::{
    metadata::{Certificate, Group, ReadRound, ReaderGrant, Receipt, Record, SignedRecord},
    metadata_bundle::{Bundle, BUNDLE_BYTES},
};
use identity::StableId;
use iop::{
    remote::{AuthenticatedStorageRequest, RemoteError},
    storage_protocol::{Operation, StorageOperationV1},
};
use node::types::NodeId;
use sha2::{Digest, Sha256};
pub const CONFIG_BYTES: usize = 192;
#[derive(Clone, Copy)]
pub struct MutationIntent {
    pub anchor: Bundle,
    pub request: StorageOperationV1,
}
pub enum NativeRequest {
    Copy {
        anchor: Bundle,
        request: StorageOperationV1,
        overlay: Option<fabric::metadata_repair::RepairBundle>,
    },
    MutateOverlay {
        anchor: Bundle,
        request: StorageOperationV1,
        overlay: fabric::metadata_repair::RepairBundle,
    },
    PlacementOverlay {
        anchor: Bundle,
        expected: u64,
        next: fabric::manifest::Manifest,
        overlay: fabric::metadata_repair::RepairBundle,
    },
    PlacementMutate {
        anchor: Bundle,
        expected: u64,
        next: fabric::manifest::Manifest,
    },
    PendingMutation {
        index: usize,
    },
    FinalizeMutation {
        object: [u8; 16],
        request: StorageOperationV1,
        record: Record,
    },
    Source {
        object: [u8; 16],
        owner: NodeId,
        scope: u64,
    },
    Load {
        index: usize,
    },
    Stage {
        bundle: Bundle,
    },
    Publish {
        object: [u8; 16],
        certificate: Certificate,
    },
    Mutate {
        anchor: Bundle,
        request: StorageOperationV1,
    },
    Read {
        object: [u8; 16],
        record: Record,
        principal: [u8; 16],
        reader: NodeId,
        now: u64,
        offset: u64,
        length: u8,
        overlay: Option<fabric::metadata_repair::RepairBundle>,
    },
    Wire {
        request: AuthenticatedStorageRequest,
        now: u64,
    },
    ConfigLoad,
    ConfigSave([u8; CONFIG_BYTES]),
}
pub enum NativeReply {
    Pending(Option<MutationIntent>),
    Mutation {
        manifest: fabric::manifest::Manifest,
        response: StorageOperationV1,
    },
    Manifest(fabric::manifest::Manifest),
    Bundle(Option<Bundle>),
    Done,
    Bytes {
        data: [u8; 64],
        length: u8,
    },
    Wire(StorageOperationV1),
    Config([u8; CONFIG_BYTES]),
}
pub type NativeHandler = fn(NativeRequest) -> Result<NativeReply, RemoteError>;
#[derive(Clone, Copy)]
struct Peer {
    node: NodeId,
    grant: u64,
    expires: u64,
}
#[derive(Clone, Copy)]
struct Pending {
    id: u64,
    cap: u64,
    peer: NodeId,
    deadline: u64,
    request: StorageOperationV1,
}
#[derive(Clone, Copy)]
enum Phase {
    Find,
    Source,
    Stage,
    Begin,
    Chunk,
    Prepare,
    PublishLocal,
    PublishBegin,
    PublishAck,
    PublishChunk,
    PublishPeer,
    Inspect,
    Fetch,
    Observe,
    ReadWriteback,
    ReadBytes,
    Overlay,
    ReadLocal,
    RemoteRead,
    Mutate,
    FinalizeMutation,
    Done,
}
struct Job {
    user: StableId,
    session: StableId,
    id: u64,
    object: [u8; 16],
    path: [u8; 95],
    path_len: u8,
    bundle: Option<Bundle>,
    bytes: [u8; BUNDLE_BYTES],
    phase: Phase,
    index: usize,
    member: usize,
    at: usize,
    receipts: [Option<Receipt>; 3],
    round: Option<ReadRound>,
    read: Option<StorageOperationV1>,
    result: Option<Result<StorageOperationV1, RemoteError>>,
    expires: u64,
    fetch_generation: u64,
    mutation: Option<StorageOperationV1>,
    mutation_response: Option<StorageOperationV1>,
    overlay_id: u64,
    overlay: Option<fabric::metadata_repair::RepairBundle>,
    remote_source: usize,
    remote_at: usize,
    read_copied: usize,
    read_result: [u8; 64],
    recovery: bool,
    service_fresh: bool,
    intent_anchor: Option<Bundle>,
    placement: Option<fabric::manifest::Manifest>,
}
pub struct Service {
    pub(super) handler: Option<NativeHandler>,
    peers: [Option<Peer>; 3],
    loaded: bool,
    job: Option<Job>,
    pending: Option<Pending>,
    next: u64,
    pub last_request: u64,
    pub completed_request: u64,
    pub last_error: Option<RemoteError>,
    names: [Option<([u8; 16], [u8; 95], u8)>; 8],
    scan: usize,
    scan_tick: u64,
    namespace_ready: bool,
    scanned: u8,
    connect_deadlines: [u64; 3],
    read_cache: [u8; 16384],
    recovery_index: usize,
}
impl Service {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates one bounded metadata operation and no ambient remote authority.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            handler: None,
            peers: [None; 3],
            loaded: false,
            job: None,
            pending: None,
            next: 1,
            last_request: 0,
            completed_request: 0,
            last_error: None,
            names: [None; 8],
            scan: 0,
            scan_tick: 0,
            namespace_ready: false,
            scanned: 0,
            connect_deadlines: [0; 3],
            read_cache: [0; 16384],
            recovery_index: 0,
        }
    }
}
// ------------------------=
// FUNC: bound
// DESC: Identifies shared objects from the bounded durable namespace projection without blocking UI execution.
// ------------------=
pub fn bound(object: [u8; 16]) -> bool {
    with_runtime(|r| {
        r.storage_metadata
            .names
            .iter()
            .flatten()
            .any(|n| n.0 == object)
    })
    .unwrap_or(false)
}
// ------------------------=
// FUNC: warming
// DESC: Prevents an owner returning from reboot bypassing shared read fencing before its durable namespace catalog is known.
// ------------------=
pub fn warming() -> bool {
    with_runtime(|r| r.storage_metadata.handler.is_some() && !r.storage_metadata.namespace_ready)
        .unwrap_or(true)
}
// ------------------------=
// FUNC: bound_from
// DESC: Resolves the shared-object fence from an exclusively borrowed runtime without reentrant locking.
// ------------------=
pub(super) fn bound_from(r: &InfinityRuntime, object: [u8; 16]) -> bool {
    (r.storage_metadata.handler.is_some() && !r.storage_metadata.namespace_ready)
        || r.storage_metadata
            .names
            .iter()
            .flatten()
            .any(|n| n.0 == object)
}
// ------------------------=
// FUNC: lookup
// DESC: Resolves a replicated namespace binding; actual content access still performs fresh quorum authorization.
// ------------------=
pub fn lookup(path: &[u8]) -> Option<[u8; 16]> {
    with_runtime(|r| {
        r.storage_metadata
            .names
            .iter()
            .flatten()
            .find(|n| &n.1[..n.2 as usize] == path)
            .map(|n| n.0)
    })
    .flatten()
}
// ------------------------=
// FUNC: peer_grant
// DESC: Exposes only the explicitly persisted live metadata grant, never transfer authority or an ambient peer permission.
// ------------------=
pub(super) fn peer_grant(r: &InfinityRuntime, peer: NodeId, now: u64) -> Result<u64, RemoteError> {
    r.storage_metadata
        .peers
        .iter()
        .flatten()
        .find(|p| p.node == peer && p.expires > now)
        .map(|p| p.grant)
        .ok_or(RemoteError::AccessDenied)
}
// ------------------------=
// FUNC: fresh_start
// DESC: Starts an owned fresh metadata quorum barrier for a separately authorized repair operation.
// ------------------=
pub(super) fn fresh_start(
    r: &mut InfinityRuntime,
    user: StableId,
    session: StableId,
    object: [u8; 16],
) -> Result<u64, RemoteError> {
    let p = StorageOperationV1 {
        operation: Operation::ObjectRead,
        object,
        authority_generation: 1,
        manifest_generation: 0,
        object_version: 0,
        offset: 0,
        scope: 0,
        value: 0,
        length: 0,
        data: [0; 64],
    };
    start(r, user, session, object, &[], Some(p))
}
// ------------------------=
// FUNC: fresh_take
// DESC: Returns the exact owner-bound bundle only after fresh correlated R2 observations and durable W2 writeback; it grants no mutation rights.
// ------------------=
pub(super) fn fresh_take(
    r: &mut InfinityRuntime,
    user: StableId,
    session: StableId,
    id: u64,
) -> Result<Option<Bundle>, RemoteError> {
    if !storage_operator::authorized(r, user, session) {
        return Err(RemoteError::AccessDenied);
    }
    let j = r
        .storage_metadata
        .job
        .as_ref()
        .filter(|j| {
            j.id == id
                && j.user == user
                && j.session == session
                && j.read.is_some_and(|p| p.value == 0)
        })
        .ok_or(RemoteError::NotFound)?;
    let Some(result) = j.result else {
        return Ok(None);
    };
    let b = j.bundle;
    r.storage_metadata.job = None;
    result?;
    Ok(Some(b.ok_or(RemoteError::InvalidState)?))
}
// ------------------------=
// FUNC: fresh_cancel
// DESC: Retires only an exact owned internal read barrier and its pending service capability, including after session expiry.
// ------------------=
pub(super) fn fresh_cancel(r: &mut InfinityRuntime, user: StableId, session: StableId, id: u64) {
    if !r.storage_metadata.job.as_ref().is_some_and(|j| {
        j.id == id
            && j.user == user
            && j.session == session
            && !j.recovery
            && !j.service_fresh
            && j.read.is_some_and(|p| p.value == 0)
            && j.mutation.is_none()
    }) {
        return;
    }
    r.storage_metadata.job = None;
    if let Some(p) = r.storage_metadata.pending.take() {
        if let Some(caller) = r.service_identity(SERVICE_REPLICA_STORAGE) {
            r.iop.remote.discard(caller, p.id);
            let _ = r.capabilities.retire_leaf(p.cap, caller);
        }
    }
}
// ------------------------=
// FUNC: fresh_start_service
// DESC: Starts only a native service read barrier; signed writer delegation is checked before quorum use and completion.
// ------------------=
pub(super) fn fresh_start_service(
    r: &mut InfinityRuntime,
    object: [u8; 16],
) -> Result<u64, RemoteError> {
    r.service_identity(SERVICE_REPLICA_STORAGE)
        .ok_or(RemoteError::AccessDenied)?;
    let p = StorageOperationV1 {
        operation: Operation::ObjectRead,
        object,
        authority_generation: 1,
        manifest_generation: 0,
        object_version: 0,
        offset: 0,
        scope: 0,
        value: 0,
        length: 0,
        data: [0; 64],
    };
    start_owned(
        r,
        StableId([0; 16]),
        StableId([0; 16]),
        object,
        &[],
        Some(p),
        true,
    )
}
// ------------------------=
// FUNC: service_delegated
// DESC: Requires a live owner-signed repair delegation to this real local service node.
// ------------------=
fn service_delegated(r: &InfinityRuntime, b: &Bundle, now: u64) -> bool {
    r.service_identity(SERVICE_REPLICA_STORAGE).is_some()
        && b.repair_grants.iter().flatten().any(|g| {
            Some(g.writer) == r.nodes.local_id()
                && g.validate_anchor(&b.group, &b.value, now).is_ok()
        })
}
// ------------------------=
// FUNC: fresh_take_service
// DESC: Consumes only the exact completed service barrier and revalidates its signed lease.
// ------------------=
pub(super) fn fresh_take_service(
    r: &mut InfinityRuntime,
    id: u64,
) -> Result<Option<Bundle>, RemoteError> {
    let j = r
        .storage_metadata
        .job
        .as_ref()
        .filter(|j| j.id == id && j.service_fresh)
        .ok_or(RemoteError::NotFound)?;
    let Some(result) = j.result else {
        return Ok(None);
    };
    let b = j.bundle;
    r.storage_metadata.job = None;
    result?;
    let b = b.ok_or(RemoteError::InvalidState)?;
    if !service_delegated(r, &b, r.node_clock.ok_or(RemoteError::ServiceUnavailable)?) {
        return Err(RemoteError::AccessDenied);
    }
    Ok(Some(b))
}
// ------------------------=
// FUNC: register
// DESC: Binds the installed native metadata backend without installing a second RPC transport.
// ------------------=
pub fn register(handler: NativeHandler) {
    with_runtime(|r| {
        r.storage_metadata.handler = Some(handler);
        r.storage_metadata.loaded = false;
    });
}
// ------------------------=
// FUNC: principal
// DESC: Identifies the local storage service as the explicitly delegated reader; human access still requires an active authorized session.
// ------------------=
pub fn principal() -> [u8; 16] {
    let mut b = [0; 16];
    b[..8].copy_from_slice(&(SERVICE_REPLICA_STORAGE as u64).to_le_bytes());
    b[8..].copy_from_slice(b"POOLREAD");
    b
}
// ------------------------=
// FUNC: native
// DESC: Requires an exact short-lived service-owned metadata capability for each native callback and retires it immediately.
// ------------------=
#[inline(never)]
fn native(
    r: &mut InfinityRuntime,
    request: NativeRequest,
    now: u64,
) -> Result<NativeReply, RemoteError> {
    let service = r
        .service_identity(SERVICE_REPLICA_STORAGE)
        .ok_or(RemoteError::ServiceUnavailable)?;
    let handler = r
        .storage_metadata
        .handler
        .ok_or(RemoteError::ServiceUnavailable)?;
    let cap = r
        .capabilities
        .grant(
            CapabilityType::ServiceCall,
            Operation::PoolMetadata as u64,
            1,
            0,
            service,
            service,
            Some(now.saturating_add(1)),
            0,
        )
        .map_err(|_| RemoteError::QueueFull)?;
    let result = r
        .capabilities
        .validate(
            cap,
            service,
            CapabilityType::ServiceCall,
            Operation::PoolMetadata as u64,
            1,
            0,
            now,
        )
        .map_err(|_| RemoteError::AccessDenied)
        .and_then(|_| handler(request));
    let _ = r.capabilities.retire_leaf(cap, service);
    result
}
// ------------------------=
// FUNC: configure
// DESC: Persists an operator-approved exact metadata operation grant; it does not reuse transfer authority or establish new trust.
// ------------------=
pub fn configure(
    user: StableId,
    session: StableId,
    node: NodeId,
    grant: u64,
    lease: u64,
) -> Result<(), RemoteError> {
    with_runtime(|r| {
        if !storage_operator::authorized(r, user, session)
            || grant == 0
            || lease == 0
            || lease > 3600
        {
            return Err(RemoteError::AccessDenied);
        }
        if !r.storage_metadata.loaded || r.storage_metadata.job.is_some() {
            return Err(RemoteError::Conflict);
        }
        let now = r.node_clock.ok_or(RemoteError::ServiceUnavailable)?;
        if r.nodes.local_id() == Some(node) || r.nodes.paired_digest(node).is_none() {
            return Err(RemoteError::AccessDenied);
        }
        let mut peers = r.storage_metadata.peers;
        let i = peers
            .iter()
            .position(|p| p.is_some_and(|p| p.node == node))
            .or_else(|| peers.iter().position(Option::is_none))
            .ok_or(RemoteError::QueueFull)?;
        peers[i] = Some(Peer {
            node,
            grant,
            expires: now.saturating_add(lease),
        });
        let mut b = [0; CONFIG_BYTES];
        b[..8].copy_from_slice(b"INFMDG01");
        for (i, p) in peers.iter().enumerate() {
            if let Some(p) = p {
                let at = 16 + i * 48;
                b[at..at + 32].copy_from_slice(&p.node.0);
                b[at + 32..at + 40].copy_from_slice(&p.grant.to_le_bytes());
                b[at + 40..at + 48].copy_from_slice(&p.expires.to_le_bytes());
            }
        }
        native(r, NativeRequest::ConfigSave(b), now)?;
        r.storage_metadata.peers = peers;
        Ok(())
    })
    .ok_or(RemoteError::ServiceUnavailable)?
}
// ------------------------=
// FUNC: start
// DESC: Queues an exact owned metadata publication or read, returning immediately to the human interface.
// ------------------=
fn start(
    r: &mut InfinityRuntime,
    user: StableId,
    session: StableId,
    object: [u8; 16],
    path: &[u8],
    read: Option<StorageOperationV1>,
) -> Result<u64, RemoteError> {
    start_owned(r, user, session, object, path, read, false)
}
// ------------------------=
// FUNC: start_owned
// DESC: Separates a service-only read barrier from authenticated human requests without granting mutation authority.
// ------------------=
fn start_owned(
    r: &mut InfinityRuntime,
    user: StableId,
    session: StableId,
    object: [u8; 16],
    path: &[u8],
    read: Option<StorageOperationV1>,
    service_fresh: bool,
) -> Result<u64, RemoteError> {
    if !service_fresh && !storage_operator::authorized(r, user, session) {
        return Err(RemoteError::AccessDenied);
    }
    if object == [0; 16] || path.len() > 95 {
        return Err(RemoteError::MalformedRequest);
    }
    if r.storage_metadata.job.is_some() {
        return Err(RemoteError::QueueFull);
    }
    let now = r.node_clock.ok_or(RemoteError::ServiceUnavailable)?;
    let id = (3u64 << 62) | r.storage_metadata.next;
    r.storage_metadata.next = r
        .storage_metadata
        .next
        .checked_add(1)
        .ok_or(RemoteError::QueueFull)?;
    let mut p = [0; 95];
    p[..path.len()].copy_from_slice(path);
    r.storage_metadata.job = Some(Job {
        user,
        session,
        id,
        object,
        path: p,
        path_len: path.len() as u8,
        bundle: None,
        bytes: [0; BUNDLE_BYTES],
        phase: Phase::Find,
        index: 0,
        member: 0,
        at: 0,
        receipts: [None; 3],
        round: None,
        read,
        result: None,
        expires: now.saturating_add(900),
        fetch_generation: 0,
        mutation: None,
        mutation_response: None,
        overlay_id: 0,
        overlay: None,
        remote_source: 0,
        remote_at: 0,
        read_copied: 0,
        read_result: [0; 64],
        recovery: false,
        service_fresh,
        intent_anchor: None,
        placement: None,
    });
    r.storage_metadata.last_request = id;
    r.storage_last_observation = None;
    Ok(id)
}
// ------------------------=
// FUNC: recover_intent
// DESC: Admits only a durable native local-owner mutation intent as a service recovery job; it never synthesizes a human session or new write authority.
// ------------------=
#[inline(never)]
fn recover_intent(
    r: &mut InfinityRuntime,
    intent: MutationIntent,
    now: u64,
) -> Result<(), RemoteError> {
    let b = intent.anchor;
    b.validate().map_err(|_| RemoteError::AccessDenied)?;
    if Some(b.group.owner) != r.nodes.local_id()
        || intent.request.object != b.manifest.object
        || !matches!(
            intent.request.operation,
            Operation::ObjectUpdate
                | Operation::ObjectSetPolicy
                | Operation::ObjectDelete
                | Operation::PoolHeal
        )
    {
        return Err(RemoteError::AccessDenied);
    }
    let read = StorageOperationV1 {
        operation: Operation::ObjectRead,
        object: b.manifest.object,
        authority_generation: 1,
        manifest_generation: 0,
        object_version: 0,
        offset: 0,
        scope: 0,
        value: 0,
        length: 0,
        data: [0; 64],
    };
    let id = (3u64 << 62) | r.storage_metadata.next;
    r.storage_metadata.next = r
        .storage_metadata
        .next
        .checked_add(1)
        .ok_or(RemoteError::QueueFull)?;
    r.storage_metadata.job = Some(Job {
        user: StableId([0; 16]),
        session: StableId([0; 16]),
        id,
        object: b.manifest.object,
        path: b.path,
        path_len: b.path_len,
        bundle: Some(b),
        bytes: [0; BUNDLE_BYTES],
        phase: Phase::Find,
        index: 0,
        member: 0,
        at: 0,
        receipts: [None; 3],
        round: None,
        read: Some(read),
        result: None,
        expires: now.saturating_add(900),
        fetch_generation: 0,
        mutation: Some(intent.request),
        mutation_response: None,
        overlay_id: 0,
        overlay: None,
        remote_source: 0,
        remote_at: 0,
        read_copied: 0,
        read_result: [0; 64],
        recovery: true,
        service_fresh: false,
        intent_anchor: Some(b),
        placement: None,
    });
    Ok(())
}
// ------------------------=
// FUNC: queue_placement
// DESC: Queues an existing native coordinator placement CAS behind fresh metadata quorum, retaining service ownership and unchanged immutable object contents.
// ------------------=
pub(super) fn queue_placement(
    r: &mut InfinityRuntime,
    expected: u64,
    next: fabric::manifest::Manifest,
    now: u64,
) -> Result<(), RemoteError> {
    if Some(next.authority) != r.nodes.local_id() || next.generation != expected.saturating_add(1) {
        return Err(RemoteError::AccessDenied);
    }
    next.validate().map_err(|_| RemoteError::MalformedRequest)?;
    if r.storage_metadata.job.is_some() {
        return Err(RemoteError::QueueFull);
    }
    let mut encoded = [0; fabric::manifest::MANIFEST_BYTES];
    next.encode(&mut encoded)
        .map_err(|_| RemoteError::MalformedRequest)?;
    let mut operation = StorageOperationV1 {
        operation: Operation::PoolHeal,
        object: next.object,
        authority_generation: next.authority_generation,
        manifest_generation: expected,
        object_version: next.version,
        offset: 0,
        scope: 0,
        value: 0,
        length: 32,
        data: [0; 64],
    };
    operation.data[..32].copy_from_slice(&Sha256::digest(encoded));
    let mut read = operation;
    read.operation = Operation::ObjectRead;
    read.value = 0;
    read.length = 0;
    read.data = [0; 64];
    let id = (3u64 << 62) | r.storage_metadata.next;
    r.storage_metadata.next = r
        .storage_metadata
        .next
        .checked_add(1)
        .ok_or(RemoteError::QueueFull)?;
    r.storage_metadata.job = Some(Job {
        user: StableId([0; 16]),
        session: StableId([0; 16]),
        id,
        object: next.object,
        path: [0; 95],
        path_len: 0,
        bundle: None,
        bytes: [0; BUNDLE_BYTES],
        phase: Phase::Find,
        index: 0,
        member: 0,
        at: 0,
        receipts: [None; 3],
        round: None,
        read: Some(read),
        result: None,
        expires: now.saturating_add(900),
        fetch_generation: 0,
        mutation: Some(operation),
        mutation_response: None,
        overlay_id: 0,
        overlay: None,
        remote_source: 0,
        remote_at: 0,
        read_copied: 0,
        read_result: [0; 64],
        recovery: true,
        service_fresh: false,
        intent_anchor: None,
        placement: Some(next),
    });
    Ok(())
}
// ------------------------=
// FUNC: share
// DESC: Starts explicit signed namespace/read delegation publication to the configured fixed three-member metadata group.
// ------------------=
pub fn share(
    user: StableId,
    session: StableId,
    object: [u8; 16],
    path: &[u8],
) -> Result<u64, RemoteError> {
    with_runtime(|r| start(r, user, session, object, path, None))
        .ok_or(RemoteError::ServiceUnavailable)?
}
// ------------------------=
// FUNC: read
// DESC: Starts a fresh correlated metadata quorum read without any caller-selected physical peer.
// ------------------=
pub fn read(
    user: StableId,
    session: StableId,
    request: StorageOperationV1,
) -> Result<u64, RemoteError> {
    with_runtime(|r| {
        if !matches!(request.operation,Operation::ObjectRead|Operation::ObjectInspect)
            || request.length != 0
            || !(1..=64).contains(&request.value)
        {
            return Err(RemoteError::MalformedRequest);
        }
        start(r, user, session, request.object, &[], Some(request))
    })
    .ok_or(RemoteError::ServiceUnavailable)?
}
// ------------------------=
// FUNC: mutate
// DESC: Admits a shared mutation only through a fresh quorum barrier; final success waits for successor publication quorum.
// ------------------=
pub fn mutate(
    user: StableId,
    session: StableId,
    request: StorageOperationV1,
) -> Result<u64, RemoteError> {
    with_runtime(|r| mutate_start(r, user, session, request))
        .ok_or(RemoteError::ServiceUnavailable)?
}
// ------------------------=
// FUNC: mutate_start
// DESC: Admits the same authorized asynchronous mutation while its native caller already owns the runtime borrow.
// ------------------=
pub(super) fn mutate_start(
    r: &mut InfinityRuntime,
    user: StableId,
    session: StableId,
    request: StorageOperationV1,
) -> Result<u64, RemoteError> {
    if !matches!(
        request.operation,
        Operation::ObjectUpdate
            | Operation::ObjectSetPolicy
            | Operation::ObjectDelete
            | Operation::ObjectCopy
    ) {
        return Err(RemoteError::UnsupportedOperation);
    }
    let id = fresh_start(r, user, session, request.object)?;
    r.storage_metadata.job.as_mut().unwrap().mutation = Some(request);
    Ok(id)
}
// ------------------------=
// FUNC: take
// DESC: Consumes only the exact initiating still-authorized session's completion and publishes diagnostic state after completion.
// ------------------=
pub fn take(
    user: StableId,
    session: StableId,
    id: u64,
) -> Result<Option<StorageOperationV1>, RemoteError> {
    with_runtime(|r| mutate_take(r, user, session, id)).ok_or(RemoteError::ServiceUnavailable)?
}
// ------------------------=
// FUNC: mutate_take
// DESC: Consumes the exact authorized completion without recursively borrowing the runtime.
// ------------------=
pub(super) fn mutate_take(
    r: &mut InfinityRuntime,
    user: StableId,
    session: StableId,
    id: u64,
) -> Result<Option<StorageOperationV1>, RemoteError> {
    if !storage_operator::authorized(r, user, session) {
        return Err(RemoteError::AccessDenied);
    }
    let j = r
        .storage_metadata
        .job
        .as_ref()
        .filter(|j| j.user == user && j.session == session && j.id == id)
        .ok_or(RemoteError::NotFound)?;
    let Some(result) = j.result else {
        return Ok(None);
    };
    r.storage_metadata.job = None;
    r.storage_metadata.completed_request = id;
    r.storage_metadata.last_error = result.as_ref().err().copied();
    r.storage_last_observation = result.as_ref().ok().copied();
    result.map(Some)
}
// ------------------------=
// FUNC: mutate_cancel
// DESC: Detaches one exact UI requester and retires pending transport, leaving any admitted durable intent for service recovery.
// ------------------=
pub(super) fn mutate_cancel(r: &mut InfinityRuntime, user: StableId, session: StableId, id: u64) {
    let Some(j) = r.storage_metadata.job.as_ref().filter(|j| {
        j.id == id && j.user == user && j.session == session && !j.recovery && !j.service_fresh
    }) else {
        return;
    };
    let overlay = j.overlay_id;
    r.storage_metadata.job = None;
    if overlay != 0 {
        storage_metadata_repair::overlay_cancel(r, user, session, overlay);
    }
    if let Some(p) = r.storage_metadata.pending.take() {
        if let Some(caller) = r.service_identity(SERVICE_REPLICA_STORAGE) {
            r.iop.remote.discard(caller, p.id);
            let _ = r.capabilities.retire_leaf(p.cap, caller);
        }
    }
}
// ------------------------=
// FUNC: request
// DESC: Constructs a canonical fixed metadata operation envelope with no embedded human command.
// ------------------=
fn request(j: &Job, action: u64) -> StorageOperationV1 {
    StorageOperationV1 {
        operation: Operation::PoolMetadata,
        object: j.object,
        authority_generation: 1,
        manifest_generation: j.bundle.map_or(1, |b| b.value.record.generation),
        object_version: j.bundle.map_or(1, |b| b.value.record.generation),
        offset: 0,
        scope: 0,
        value: action,
        length: 0,
        data: [0; 64],
    }
}
// ------------------------=
// FUNC: send
// DESC: Sends one correlated metadata request through existing live session, scoped remote grant and IOP queue, never busy-waiting.
// ------------------=
fn send(
    r: &mut InfinityRuntime,
    peer: NodeId,
    payload: StorageOperationV1,
    now: u64,
) -> Result<(), RemoteError> {
    let approved = r
        .storage_metadata
        .peers
        .iter()
        .flatten()
        .find(|p| p.node == peer && p.expires > now)
        .ok_or(RemoteError::AccessDenied)?;
    let grant = approved.grant;
    let index = r
        .storage_metadata
        .peers
        .iter()
        .position(|p| p.is_some_and(|p| p.node == peer))
        .unwrap();
    if r.node_transport.trust.session(peer).is_none() {
        if r.nodes.paired_digest(peer).is_none() {
            return Err(RemoteError::TrustRequired);
        }
        let deadline = r.storage_metadata.connect_deadlines[index];
        if deadline == 0 {
            r.storage_metadata.connect_deadlines[index] = now.saturating_add(10);
            if let Some(link) = r.node_transport.peer_link(peer) {
                r.node_transport
                    .trust
                    .begin(&mut r.nodes, link, 0, true, now)
                    .map_err(|_| RemoteError::TrustRequired)?;
            }
            return Ok(());
        }
        if now < deadline {
            return Ok(());
        }
        r.storage_metadata.connect_deadlines[index] = 0;
        return Err(RemoteError::DeadlineExceeded);
    }
    r.storage_metadata.connect_deadlines[index] = 0;
    let caller = r
        .service_identity(SERVICE_REPLICA_STORAGE)
        .ok_or(RemoteError::ServiceUnavailable)?;
    let cap = r
        .capabilities
        .grant(
            CapabilityType::ServiceCall,
            Operation::PoolMetadata as u64,
            1,
            0,
            caller,
            caller,
            Some(now + 10),
            0,
        )
        .map_err(|_| RemoteError::QueueFull)?;
    let result = r
        .iop
        .next_node_request()
        .map_err(|_| RemoteError::QueueFull)
        .and_then(|id| {
            r.iop.request_remote_storage(
                &r.capabilities,
                &r.nodes,
                caller,
                cap,
                peer,
                grant,
                payload,
                id,
                id,
                now,
                now + 10,
            )
        });
    match result {
        Ok(id) => {
            r.storage_metadata.pending = Some(Pending {
                id,
                cap,
                peer,
                deadline: now + 10,
                request: payload,
            });
            Ok(())
        }
        Err(e) => {
            let _ = r.capabilities.retire_leaf(cap, caller);
            Err(e)
        }
    }
}
// ------------------------=
// FUNC: local_receipt
// DESC: Signs an exact native persistence receipt only after its caller observed successful durable Stage or Publish.
// ------------------=
fn local_receipt(r: &InfinityRuntime, b: Bundle, published: bool) -> Result<Receipt, RemoteError> {
    let local = r.nodes.local_id().ok_or(RemoteError::ServiceUnavailable)?;
    let member = b
        .group
        .members
        .iter()
        .position(|p| *p == local)
        .ok_or(RemoteError::AccessDenied)? as u8;
    let mut receipt = Receipt {
        member,
        digest: b.value.record.digest(),
        published,
        signature: [0; 64],
    };
    receipt.signature = r
        .nodes
        .sign_storage_metadata(local, &receipt.transcript())
        .map_err(|_| RemoteError::AccessDenied)?;
    Ok(receipt)
}
// ------------------------=
// FUNC: next_member
// DESC: Selects one distinct remote configured member, skipping the local node without changing quorum membership.
// ------------------=
fn next_member(j: &mut Job, local: NodeId) -> bool {
    let g = j.bundle.unwrap().group;
    while j.member < 3 && g.members[j.member] == local {
        j.member += 1;
    }
    j.member < 3
}
// ------------------------=
// FUNC: build_bundle
// DESC: Signs actual native manifest, stable namespace binding and exact service read grants under the explicitly approved metadata group.
// ------------------=
fn build_bundle(
    r: &InfinityRuntime,
    j: &Job,
    m: fabric::manifest::Manifest,
    now: u64,
) -> Result<Bundle, RemoteError> {
    let local = r.nodes.local_id().ok_or(RemoteError::ServiceUnavailable)?;
    let group = if let Some(b) = j.bundle {
        b.group
    } else {
        let mut members = [local; 3];
        let mut keys = [[0; 32]; 3];
        keys[0] = r
            .nodes
            .local_public_key()
            .ok_or(RemoteError::ServiceUnavailable)?;
        let mut n = 1;
        for p in r
            .storage_metadata
            .peers
            .iter()
            .flatten()
            .filter(|p| p.expires > now)
        {
            if n == 3 {
                break;
            }
            let peer = r
                .nodes
                .discovered_nodes()
                .iter()
                .flatten()
                .find(|d| d.id == p.node)
                .ok_or(RemoteError::AccessDenied)?;
            members[n] = p.node;
            keys[n] = peer.public_key;
            n += 1;
        }
        if n != 3 {
            return Err(RemoteError::AccessDenied);
        }
        Group {
            epoch: 1,
            owner: local,
            members,
            keys,
        }
    };
    group
        .validate()
        .map_err(|_| RemoteError::MalformedRequest)?;
    if group.owner != local {
        return Err(RemoteError::AccessDenied);
    }
    let previous = j.bundle.map(|b| b.value.record);
    let revocation = match previous {
        Some(p) if j.mutation.is_some() => {
            p.revocation.checked_add(1).ok_or(RemoteError::Conflict)?
        }
        Some(p) => p.revocation,
        None => 1,
    };
    let mut encoded = [0; fabric::manifest::MANIFEST_BYTES];
    m.encode(&mut encoded)
        .map_err(|_| RemoteError::MalformedRequest)?;
    let path = if j.path_len > 0 {
        &j.path[..j.path_len as usize]
    } else {
        j.bundle
            .as_ref()
            .map(|b| b.path())
            .ok_or(RemoteError::MalformedRequest)?
    };
    let record = Record {
        group: group.digest(),
        object: m.object,
        generation: previous.map_or(1, |p| p.generation + 1),
        version: m.version,
        previous: previous.map_or([0; 32], Record::digest),
        manifest: Sha256::digest(encoded).into(),
        namespace: fabric::metadata_bundle::namespace_digest(m.object, path)
            .map_err(|_| RemoteError::MalformedRequest)?,
        policy: fabric::metadata_bundle::policy_digest(&group, &m, revocation),
        revocation,
        deleted: j
            .mutation
            .is_some_and(|p| p.operation == Operation::ObjectDelete),
    };
    let value = SignedRecord {
        record,
        signature: r
            .nodes
            .sign_storage_metadata(local, &record.encode())
            .map_err(|_| RemoteError::AccessDenied)?,
    };
    let mut grants = [None; 3];
    for i in 0..3 {
        let mut grant = ReaderGrant {
            group: group.digest(),
            object: m.object,
            reader: group.members[i],
            principal: principal(),
            policy: record.policy,
            revocation,
            expires: r
                .storage_metadata
                .peers
                .iter()
                .flatten()
                .map(|p| p.expires)
                .min()
                .unwrap_or(now),
            signature: [0; 64],
        };
        grant.signature = r
            .nodes
            .sign_storage_metadata(local, &grant.transcript())
            .map_err(|_| RemoteError::AccessDenied)?;
        grants[i] = Some(grant);
    }
    let mut repair_grants = [None; 2];
    if !record.deleted {
        let mut destinations = [NodeId([0; 32]); 4];
        let mut n = 0;
        let mut expires = now.saturating_add(3600);
        for peer in r
            .storage_metadata
            .peers
            .iter()
            .flatten()
            .filter(|p| p.expires > now)
        {
            destinations[n] = peer.node;
            n += 1;
            expires = expires.min(peer.expires);
        }
        if n != 0 {
            let mut index = 0;
            for writer in group.members.iter().copied().filter(|n| *n != local) {
                let mut grant = fabric::metadata_repair::RepairGrant {
                    group: group.digest(),
                    anchor: record.digest(),
                    writer,
                    destinations,
                    expires,
                    signature: [0; 64],
                };
                grant.signature = r
                    .nodes
                    .sign_storage_metadata(local, &grant.transcript())
                    .map_err(|_| RemoteError::AccessDenied)?;
                grant
                    .validate_anchor(&group, &value, now)
                    .map_err(|_| RemoteError::AccessDenied)?;
                repair_grants[index] = Some(grant);
                index += 1;
            }
        }
    }
    let mut b = Bundle {
        group,
        value,
        certificate: None,
        manifest: m,
        path: [0; 95],
        path_len: path.len() as u8,
        grants,
        repair_grants,
    };
    b.path[..path.len()].copy_from_slice(path);
    b.validate().map_err(|_| RemoteError::MalformedRequest)?;
    Ok(b)
}
// ------------------------=
// FUNC: complete
// DESC: Publishes only exact typed completion data to its initiating owner-bound mailbox.
// ------------------=
fn complete(j: &mut Job, data: [u8; 64], length: u8) {
    if let Some(response) = j.mutation_response {
        let _ = response;
        j.phase = Phase::FinalizeMutation;
        return;
    }
    let mut p = j.read.unwrap_or_else(|| request(j, 3));
    p.data = data;
    p.length = length as u16;
    if let Some(b) = j.bundle {
        p.object_version = b.manifest.version;
        p.manifest_generation = b.manifest.generation;
    }
    j.result = Some(Ok(p));
    j.phase = Phase::Done;
}
// ------------------------=
// FUNC: inspect_reply
// DESC: Encodes a bounded ordinary inspection window only after the caller has completed fresh owner and repair metadata barriers.
// ------------------=
#[inline(never)]
fn inspect_reply(mut p:StorageOperationV1,m:&fabric::manifest::Manifest)->Result<StorageOperationV1,RemoteError>{
    if p.operation!=Operation::ObjectInspect||p.object!=m.object||p.length!=0||!(1..=64).contains(&p.value)||p.offset>=fabric::manifest::MANIFEST_BYTES as u64{return Err(RemoteError::MalformedRequest)}
    if p.authority_generation!=m.authority_generation||(p.manifest_generation!=0&&p.manifest_generation!=m.generation)||(p.object_version!=0&&p.object_version!=m.version){return Err(RemoteError::Conflict)}
    let mut encoded=[0;fabric::manifest::MANIFEST_BYTES];m.encode(&mut encoded).map_err(|_|RemoteError::PersistenceFailed)?;
    let at=p.offset as usize;let count=(encoded.len()-at).min(p.value as usize);p.data=[0;64];p.data[..count].copy_from_slice(&encoded[at..at+count]);p.length=count as u16;p.value=encoded.len() as u64;p.manifest_generation=m.generation;p.object_version=m.version;Ok(p)
}
// ------------------------=
// FUNC: remote_chunk
// DESC: Selects the signed immutable chunk covering the next unread byte without trusting a remote offset or hash.
// ------------------=
fn remote_chunk(j: &Job) -> Result<(u64, fabric::manifest::Chunk), RemoteError> {
    let p = j.read.ok_or(RemoteError::InvalidState)?;
    let m = j.bundle.ok_or(RemoteError::InvalidState)?.manifest;
    if p.offset
        .checked_add(p.value)
        .is_none_or(|end| end > m.length)
    {
        return Err(RemoteError::MalformedRequest);
    }
    let position = p.offset + j.read_copied as u64;
    let mut base = 0;
    for chunk in m.chunks.iter().flatten() {
        if position >= base && position < base + chunk.bytes as u64 {
            return Ok((base, *chunk));
        }
        base += chunk.bytes as u64;
    }
    Err(RemoteError::MalformedRequest)
}
// ------------------------=
// FUNC: remote_source
// DESC: Chooses distinct current verified effective placements, retaining logical owner identity and rejecting stale versions.
// ------------------=
fn remote_source(j: &mut Job, local: NodeId) -> Result<NodeId, RemoteError> {
    let b = j.bundle.ok_or(RemoteError::InvalidState)?;
    let m = j.overlay.map_or(b.manifest, |o| o.manifest);
    while j.remote_source < fabric::manifest::MAX_PLACEMENTS {
        let i = j.remote_source;
        if let Some(p) = m.placements[i] {
            if p.node != local
                && p.state == fabric::manifest::PlacementState::Verified
                && p.version == b.manifest.version
                && p.hash == b.manifest.hash
                && !m.placements[..i]
                    .iter()
                    .flatten()
                    .any(|old| old.node == p.node)
            {
                return Ok(p.node);
            }
        }
        j.remote_source += 1;
    }
    Err(RemoteError::NotFound)
}
// ------------------------=
// FUNC: advance_reply
// DESC: Applies one freshly correlated authenticated response; an observation from any earlier request cannot contribute to quorum.
// ------------------=
#[inline(never)]
fn advance_reply(
    r: &mut InfinityRuntime,
    j: &mut Job,
    p: Pending,
    response: StorageOperationV1,
    now: u64,
) -> Result<(), RemoteError> {
    if response.operation != Operation::PoolMetadata || response.object != j.object {
        return Err(RemoteError::UnknownResponse);
    }
    let mut b = j.bundle.ok_or(RemoteError::InvalidState)?;
    match j.phase {
        Phase::RemoteRead => {
            let (base, chunk) = remote_chunk(j)?;
            let n = (chunk.bytes as usize - j.remote_at).min(64);
            if response.object_version != b.value.record.generation
                || response.offset != base + j.remote_at as u64
                || response.length as usize != n
            {
                return Err(RemoteError::UnknownResponse);
            }
            r.storage_metadata.read_cache[j.remote_at..j.remote_at + n]
                .copy_from_slice(&response.data[..n]);
            j.remote_at += n;
            if j.remote_at == chunk.bytes as usize {
                if <[u8; 32]>::from(Sha256::digest(
                    &r.storage_metadata.read_cache[..j.remote_at],
                )) != chunk.hash
                {
                    return Err(RemoteError::RemoteFailure);
                }
                let read = j.read.unwrap();
                let at = (read.offset + j.read_copied as u64 - base) as usize;
                let take = (read.value as usize - j.read_copied).min(chunk.bytes as usize - at);
                j.read_result[j.read_copied..j.read_copied + take]
                    .copy_from_slice(&r.storage_metadata.read_cache[at..at + take]);
                j.read_copied += take;
                if j.read_copied == read.value as usize {
                    complete(j, j.read_result, j.read_copied as u8);
                } else {
                    j.remote_at = 0;
                    j.remote_source = 0;
                }
            }
        }
        Phase::Begin | Phase::PublishBegin => {
            j.at = 0;
            j.phase = if matches!(j.phase, Phase::Begin) {
                Phase::Chunk
            } else {
                Phase::PublishChunk
            };
        }
        Phase::Chunk | Phase::PublishChunk => {
            j.at += p.request.length as usize;
            if j.at == BUNDLE_BYTES {
                j.phase = if matches!(j.phase, Phase::Chunk) {
                    Phase::Prepare
                } else {
                    Phase::PublishPeer
                };
            }
        }
        Phase::Prepare | Phase::PublishPeer | Phase::PublishAck => {
            if response.length != 64 || response.offset != j.member as u64 {
                return Err(RemoteError::UnknownResponse);
            }
            let published = matches!(j.phase, Phase::PublishPeer | Phase::PublishAck);
            let receipt = Receipt {
                member: j.member as u8,
                digest: b.value.record.digest(),
                published,
                signature: response.data,
            };
            crate::runtime::crypto::NodeCrypto::verify(
                &b.group.keys[j.member],
                &receipt.transcript(),
                &receipt.signature,
            )
            .map_err(|_| RemoteError::AccessDenied)?;
            if !published {
                j.receipts[j.member] = Some(receipt);
                let local = b
                    .group
                    .members
                    .iter()
                    .position(|n| Some(*n) == r.nodes.local_id())
                    .unwrap();
                b.certificate = Some(Certificate {
                    value: b.value,
                    prepared: [j.receipts[local].unwrap(), receipt],
                });
                b.certificate
                    .unwrap()
                    .validate(&b.group)
                    .map_err(|_| RemoteError::AccessDenied)?;
                j.bundle = Some(b);
                j.bytes = b.encode().map_err(|_| RemoteError::MalformedRequest)?;
                j.phase = Phase::PublishLocal;
            } else {
                if let Some(round) = j.round.as_mut() {
                    round
                        .acknowledge_writeback(&b.group, receipt)
                        .map_err(|_| RemoteError::AccessDenied)?;
                }
                j.member += 1;
                if next_member(j, r.nodes.local_id().unwrap()) {
                    j.phase = if j.round.is_some() {
                        Phase::PublishAck
                    } else {
                        Phase::PublishBegin
                    };
                } else if j.read.is_some() {
                    j.phase = Phase::ReadBytes;
                } else {
                    complete(j, [0; 64], 0);
                }
            }
        }
        Phase::Inspect => {
            if response.length != 32 {
                return Err(RemoteError::UnknownResponse);
            }
            if response.data[..32] == b.value.record.digest() {
                j.round
                    .as_mut()
                    .unwrap()
                    .observe_authenticated(&b.group, p.peer, b.certificate)
                    .map_err(|_| RemoteError::AccessDenied)?;
                j.member += 1;
                j.phase = Phase::Inspect;
            } else {
                j.fetch_generation = response.object_version;
                j.at = 0;
                j.phase = Phase::Fetch;
            }
        }
        Phase::Fetch => {
            let n = (BUNDLE_BYTES - j.at).min(64);
            if response.length as usize != n || response.offset != j.at as u64 {
                return Err(RemoteError::UnknownResponse);
            }
            j.bytes[j.at..j.at + n].copy_from_slice(&response.data[..n]);
            j.at += n;
            if j.at == BUNDLE_BYTES {
                let fetched =
                    Bundle::decode(&j.bytes).map_err(|_| RemoteError::MalformedRequest)?;
                if fetched.group.digest() != b.group.digest()
                    || fetched.value.record.object != j.object
                    || fetched.value.record.generation != j.fetch_generation
                {
                    return Err(RemoteError::AccessDenied);
                }
                j.round
                    .as_mut()
                    .unwrap()
                    .observe_authenticated(&b.group, p.peer, fetched.certificate)
                    .map_err(|_| RemoteError::AccessDenied)?;
                if fetched.value.record.generation > b.value.record.generation {
                    j.bundle = Some(fetched);
                }
                j.member += 1;
                j.phase = Phase::Inspect;
            }
        }
        _ => return Err(RemoteError::InvalidState),
    }
    let _ = now;
    Ok(())
}

// ------------------------=
// FUNC: step
// DESC: Advances at most one native transaction or one wire window, with quorum failures remaining explicit.
// ------------------=
#[inline(never)]
fn step(r: &mut InfinityRuntime, j: &mut Job, now: u64) -> Result<(), RemoteError> {
    let local = r.nodes.local_id().ok_or(RemoteError::ServiceUnavailable)?;
    match j.phase {
        Phase::Find => {
            if j.index == 8 {
                if j.read.is_some() {
                    return Err(RemoteError::NotFound);
                }
                j.phase = Phase::Source;
                return Ok(());
            }
            let reply = native(r, NativeRequest::Load { index: j.index }, now)?;
            j.index += 1;
            if let NativeReply::Bundle(Some(b)) = reply {
                if b.value.record.object == j.object {
                    if j.service_fresh && !service_delegated(r, &b, now) {
                        return Err(RemoteError::AccessDenied);
                    }
                    j.bundle = Some(b);
                    if j.read.is_some() {
                        let mut round = ReadRound::new(j.object);
                        round
                            .observe_authenticated(&b.group, local, b.certificate)
                            .map_err(|_| RemoteError::AccessDenied)?;
                        j.round = Some(round);
                        j.member = 0;
                        j.phase = Phase::Inspect;
                    } else {
                        j.phase = Phase::Source;
                    }
                }
            }
        }
        Phase::Source => {
            let NativeReply::Manifest(m) = native(
                r,
                NativeRequest::Source {
                    object: j.object,
                    owner: local,
                    scope: 0,
                },
                now,
            )?
            else {
                return Err(RemoteError::InvalidState);
            };
            let b = build_bundle(r, j, m, now)?;
            j.bytes = b.encode().map_err(|_| RemoteError::MalformedRequest)?;
            j.bundle = Some(b);
            j.phase = Phase::Stage;
        }
        Phase::Stage => {
            let b = j.bundle.unwrap();
            native(r, NativeRequest::Stage { bundle: b }, now)?;
            if j.read.is_some() {
                j.phase = Phase::PublishLocal;
            } else {
                let receipt = local_receipt(r, b, false)?;
                j.receipts[receipt.member as usize] = Some(receipt);
                j.member = 0;
                next_member(j, local);
                j.phase = Phase::Begin;
            }
        }
        Phase::PublishLocal => {
            let b = j.bundle.unwrap();
            native(
                r,
                NativeRequest::Publish {
                    object: j.object,
                    certificate: b.certificate.ok_or(RemoteError::InvalidState)?,
                },
                now,
            )?;
            if let Some(round) = j.round.as_mut() {
                round
                    .acknowledge_writeback(&b.group, local_receipt(r, b, true)?)
                    .map_err(|_| RemoteError::AccessDenied)?;
            }
            j.member = 0;
            next_member(j, local);
            j.phase = if j.round.is_some() {
                Phase::PublishAck
            } else {
                Phase::PublishBegin
            };
        }
        Phase::Inspect => {
            if !next_member(j, local) {
                let cert = j
                    .round
                    .as_ref()
                    .unwrap()
                    .selected()
                    .map_err(|_| RemoteError::ServiceUnavailable)?;
                let b = j.bundle.unwrap();
                if cert.value != b.value {
                    return Err(RemoteError::Conflict);
                }
                j.bytes = b.encode().map_err(|_| RemoteError::MalformedRequest)?;
                j.phase = Phase::Stage;
            } else {
                send(
                    r,
                    j.bundle.unwrap().group.members[j.member],
                    request(j, 4),
                    now,
                )?;
            }
        }
        Phase::Begin | Phase::PublishBegin => {
            let mut p = request(j, 0);
            p.offset = BUNDLE_BYTES as u64;
            p.length = 32;
            p.data[..32].copy_from_slice(&Sha256::digest(j.bytes));
            send(r, j.bundle.unwrap().group.members[j.member], p, now)?;
        }
        Phase::PublishAck => {
            let mut p = request(j, 7);
            p.length = 32;
            p.data[..32].copy_from_slice(&j.bundle.unwrap().value.record.digest());
            send(r, j.bundle.unwrap().group.members[j.member], p, now)?;
        }
        Phase::Chunk | Phase::PublishChunk => {
            let mut p = request(j, 1);
            p.offset = j.at as u64;
            p.length = (BUNDLE_BYTES - j.at).min(64) as u16;
            p.data[..p.length as usize].copy_from_slice(&j.bytes[j.at..j.at + p.length as usize]);
            send(r, j.bundle.unwrap().group.members[j.member], p, now)?;
        }
        Phase::Prepare | Phase::PublishPeer => {
            send(
                r,
                j.bundle.unwrap().group.members[j.member],
                request(
                    j,
                    if matches!(j.phase, Phase::Prepare) {
                        2
                    } else {
                        3
                    },
                ),
                now,
            )?;
        }
        Phase::Fetch => {
            let mut p = request(j, 5);
            p.object_version = j.fetch_generation;
            p.offset = j.at as u64;
            send(r, j.bundle.unwrap().group.members[j.member], p, now)?;
        }
        Phase::ReadBytes => {
            let b = j.bundle.unwrap();
            if j.recovery {
                j.round
                    .as_ref()
                    .unwrap()
                    .confirmed(&b.group)
                    .map_err(|_| RemoteError::AccessDenied)?;
                j.overlay_id = storage_metadata_repair::overlay_start_recovery(r, b)?;
                j.phase = Phase::Overlay;
                return Ok(());
            }
            let grant = b
                .authorize(local, principal(), now)
                .map_err(|_| RemoteError::AccessDenied)?;
            let _record = j
                .round
                .as_ref()
                .unwrap()
                .readable(&b.group, &grant, local, principal(), now)
                .map_err(|_| RemoteError::AccessDenied)?;
            let p = j.read.unwrap();
            if j.mutation.is_some() {
                j.overlay_id = storage_metadata_repair::overlay_start(r, j.user, j.session, b)?;
                j.phase = Phase::Overlay;
            } else if p.value == 0 {
                complete(j, [0; 64], 0);
            } else {
                j.overlay_id = storage_metadata_repair::overlay_start(r, j.user, j.session, b)?;
                j.phase = Phase::Overlay;
            }
        }
        Phase::Overlay => {
            let ready = if j.recovery {
                storage_metadata_repair::overlay_take_recovery(r, j.overlay_id)?
            } else {
                storage_metadata_repair::overlay_take(r, j.user, j.session, j.overlay_id)?
            };
            if let Some(overlay) = ready {
                j.overlay = overlay;
                j.phase = if j.mutation.is_some() {
                    Phase::Mutate
                } else {
                    Phase::ReadLocal
                };
            }
        }
        Phase::ReadLocal => {
            let b = j.bundle.unwrap();
            let p = j.read.unwrap();
            if p.operation==Operation::ObjectInspect {
                b.authorize(local,principal(),now).map_err(|_|RemoteError::AccessDenied)?;
                let manifest=j.overlay.map_or(b.manifest,|o|o.manifest);
                j.result=Some(inspect_reply(p,&manifest));
                return Ok(());
            }
            let record = b.value.record;
            let result = native(
                r,
                NativeRequest::Read {
                    object: j.object,
                    record,
                    principal: principal(),
                    reader: local,
                    now,
                    offset: p.offset,
                    length: p.value as u8,
                    overlay: j.overlay,
                },
                now,
            );
            match result {
                Ok(NativeReply::Bytes { data, length }) => complete(j, data, length),
                Err(RemoteError::RemoteFailure | RemoteError::NotFound) => {
                    j.remote_source = 0;
                    j.remote_at = 0;
                    j.read_copied = 0;
                    remote_chunk(j)?;
                    j.phase = Phase::RemoteRead;
                }
                Err(e) => return Err(e),
                _ => return Err(RemoteError::InvalidState),
            }
        }
        Phase::RemoteRead => {
            let peer = remote_source(j, local)?;
            let (base, chunk) = remote_chunk(j)?;
            let mut p = request(j, 6);
            p.offset = base + j.remote_at as u64;
            p.length = 1;
            p.data[0] = (chunk.bytes as usize - j.remote_at).min(64) as u8;
            send(r, peer, p, now)?;
        }
        Phase::Mutate => {
            let fresh = j.bundle.unwrap();
            let anchor = j.intent_anchor.unwrap_or(fresh);
            if anchor.group.owner != local {
                return Err(RemoteError::AccessDenied);
            }
            if j.mutation
                .is_some_and(|p| p.operation == Operation::ObjectCopy)
            {
                let NativeReply::Mutation { response, .. } = native(
                    r,
                    NativeRequest::Copy {
                        anchor,
                        request: j.mutation.unwrap(),
                        overlay: j.overlay,
                    },
                    now,
                )?
                else {
                    return Err(RemoteError::InvalidState);
                };
                j.result = Some(Ok(response));
                j.phase = Phase::Done;
                return Ok(());
            }
            if fresh.value.record != anchor.value.record
                && !(fresh.value.record.generation
                    == anchor.value.record.generation.saturating_add(1)
                    && fresh.value.record.previous == anchor.value.record.digest())
            {
                return Err(RemoteError::Conflict);
            }
            let operation = if let Some(mut next) = j.placement {
                if let Some(overlay) = j.overlay {
                    if overlay.manifest.generation > anchor.manifest.generation {
                        next = overlay.manifest;
                        let p = j.mutation.as_mut().unwrap();
                        p.manifest_generation = anchor.manifest.generation;
                        let mut bytes = [0; fabric::manifest::MANIFEST_BYTES];
                        next.encode(&mut bytes)
                            .map_err(|_| RemoteError::MalformedRequest)?;
                        p.data[..32].copy_from_slice(&Sha256::digest(bytes));
                    }
                    NativeRequest::PlacementOverlay {
                        anchor,
                        expected: j.mutation.unwrap().manifest_generation,
                        next,
                        overlay,
                    }
                } else {
                    NativeRequest::PlacementMutate {
                        anchor,
                        expected: j.mutation.unwrap().manifest_generation,
                        next,
                    }
                }
            } else if let Some(overlay) = j.overlay {
                NativeRequest::MutateOverlay {
                    anchor,
                    request: j.mutation.unwrap(),
                    overlay,
                }
            } else {
                NativeRequest::Mutate {
                    anchor,
                    request: j.mutation.unwrap(),
                }
            };
            let NativeReply::Mutation { manifest, response } = native(r, operation, now)? else {
                return Err(RemoteError::InvalidState);
            };
            if fresh.value.record != anchor.value.record {
                if fresh.manifest != manifest
                    || fresh.value.record.deleted
                        != (j.mutation.unwrap().operation == Operation::ObjectDelete)
                {
                    return Err(RemoteError::Conflict);
                }
                j.mutation_response = Some(response);
                j.phase = Phase::FinalizeMutation;
                return Ok(());
            }
            let b = build_bundle(r, j, manifest, now)?;
            j.mutation_response = Some(response);
            j.bundle = Some(b);
            j.bytes = b.encode().map_err(|_| RemoteError::MalformedRequest)?;
            j.read = None;
            j.round = None;
            j.receipts = [None; 3];
            j.member = 0;
            j.phase = Phase::Stage;
        }
        Phase::FinalizeMutation => {
            let b = j.bundle.unwrap();
            native(
                r,
                NativeRequest::FinalizeMutation {
                    object: j.object,
                    request: j.mutation.unwrap(),
                    record: b.value.record,
                },
                now,
            )?;
            j.result = Some(Ok(j.mutation_response.ok_or(RemoteError::InvalidState)?));
            j.phase = Phase::Done;
        }
        Phase::Done => {}
        _ => return Err(RemoteError::InvalidState),
    }
    Ok(())
}
// ------------------------=
// FUNC: skip_failed_member
// DESC: Allows a read quorum to survive one unavailable member without counting failed or replayed observations.
// ------------------=
fn skip_failed_member(j: &mut Job, local: NodeId) -> bool {
    if j.read.is_none() {
        return false;
    }
    match j.phase {
        Phase::PublishAck => {
            j.phase = Phase::PublishBegin;
            true
        }
        Phase::RemoteRead => {
            j.remote_source += 1;
            j.remote_at = 0;
            remote_source(j, local).is_ok()
        }
        Phase::Inspect | Phase::Fetch => {
            j.member += 1;
            j.phase = Phase::Inspect;
            true
        }
        Phase::PublishBegin | Phase::PublishChunk | Phase::PublishPeer => {
            j.member += 1;
            j.phase = if next_member(j, local) {
                Phase::PublishBegin
            } else {
                Phase::ReadBytes
            };
            true
        }
        _ => false,
    }
}
// ------------------------=
// FUNC: poll
// DESC: Pumps one bounded owned metadata operation; asynchronous network work never runs in a UI callback.
// ------------------=
#[inline(never)]
pub fn poll(r: &mut InfinityRuntime, now: u64) {
    if r.storage_metadata.handler.is_none() {
        return;
    }
    if !r.storage_metadata.loaded {
        poll_config(r, now);
        return;
    }
    if r.storage_metadata.job.is_none() {
        poll_catalog(r, now);
        return;
    }
    poll_active(r, now);
}
// ------------------------=
// FUNC: poll_config
// DESC: Loads bounded startup configuration without reserving an active job or transaction frame.
// ------------------=
#[inline(never)]
fn poll_config(r: &mut InfinityRuntime, now: u64) {
    if let Ok(NativeReply::Config(b)) = native(r, NativeRequest::ConfigLoad, now) {
        if &b[..8] == b"INFMDG01" {
            for i in 0..3 {
                let at = 16 + i * 48;
                let mut node = [0; 32];
                node.copy_from_slice(&b[at..at + 32]);
                let grant = u64::from_le_bytes(b[at + 32..at + 40].try_into().unwrap());
                let expires = u64::from_le_bytes(b[at + 40..at + 48].try_into().unwrap());
                if node != [0; 32] && grant != 0 {
                    r.storage_metadata.peers[i] = Some(Peer {
                        node: NodeId(node),
                        grant,
                        expires,
                    });
                }
            }
        }
    }
    r.storage_metadata.loaded = true;
}
// ------------------------=
// FUNC: poll_catalog
// DESC: Scans at most one durable namespace or recovery slot without copying an active job onto the idle stack.
// ------------------=
#[inline(never)]
fn poll_catalog(r: &mut InfinityRuntime, now: u64) {
    if now >= r.storage_metadata.scan_tick {
        let i = r.storage_metadata.scan;
        r.storage_metadata.scan = (i + 1) % 9;
        r.storage_metadata.scan_tick = now.saturating_add(1);
        if i == 8 {
            let index = r.storage_metadata.recovery_index;
            r.storage_metadata.recovery_index = (index + 1) % 8;
            if let Ok(NativeReply::Pending(Some(intent))) =
                native(r, NativeRequest::PendingMutation { index }, now)
            {
                if let Err(e) = recover_intent(r, intent, now) {
                    r.storage_metadata.last_error = Some(e);
                }
            }
            return;
        }
        if let Ok(NativeReply::Bundle(bundle)) = native(r, NativeRequest::Load { index: i }, now) {
            r.storage_metadata.names[i] =
                bundle.map(|b| (b.value.record.object, b.path, b.path_len));
            r.storage_metadata.scanned |= 1u8 << i;
            r.storage_metadata.namespace_ready = r.storage_metadata.scanned == u8::MAX;
        }
    }
}
// ------------------------=
// FUNC: poll_active
// DESC: Reserves the bounded active-job frame only after a real request exists, separate from idle and bootstrap paths.
// ------------------=
#[inline(never)]
fn poll_active(r: &mut InfinityRuntime, now: u64) {
    let Some(mut j) = r.storage_metadata.job.take() else {
        return;
    };
    if matches!(j.phase, Phase::Done) {
        if j.recovery {
            r.storage_metadata.last_error = j.result.and_then(Result::err);
            r.storage_metadata.completed_request = j.id;
            return;
        }
        r.storage_metadata.job = Some(j);
        return;
    }
    let result =
        if !j.recovery && !j.service_fresh && !storage_operator::authorized(r, j.user, j.session) {
            Err(RemoteError::AccessDenied)
        } else if now >= j.expires {
            Err(RemoteError::DeadlineExceeded)
        } else if let Some(p) = r.storage_metadata.pending {
            let caller = r.service_identity(SERVICE_REPLICA_STORAGE).unwrap();
            let reply = r.iop.remote.take_storage_result(caller, p.id);
            if reply.is_none() && now < p.deadline {
                r.storage_metadata.job = Some(j);
                return;
            }
            r.iop.remote.discard(caller, p.id);
            let _ = r.capabilities.retire_leaf(p.cap, caller);
            r.storage_metadata.pending = None;
            reply
                .ok_or(RemoteError::DeadlineExceeded)
                .and_then(|reply| reply.result)
                .and_then(|reply| advance_reply(r, &mut j, p, reply, now))
        } else {
            step(r, &mut j, now)
        };
    if let Err(e) = result {
        if let Some(p) = r.storage_metadata.pending.take() {
            if let Some(caller) = r.service_identity(SERVICE_REPLICA_STORAGE) {
                r.iop.remote.discard(caller, p.id);
                let _ = r.capabilities.retire_leaf(p.cap, caller);
            }
        }
        if !skip_failed_member(&mut j, r.nodes.local_id().unwrap_or(NodeId([0; 32]))) {
            if j.overlay_id != 0 {
                if j.recovery {
                    storage_metadata_repair::overlay_cancel_recovery(r, j.overlay_id);
                } else {
                    storage_metadata_repair::overlay_cancel(r, j.user, j.session, j.overlay_id);
                }
            }
            j.result = Some(Err(e));
            j.phase = Phase::Done;
        }
    }
    r.storage_metadata.job = Some(j);
}
#[cfg(test)]
#[path = "storage_metadata_tests.rs"]
mod tests;
