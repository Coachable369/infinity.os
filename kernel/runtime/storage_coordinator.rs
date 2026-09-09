//! Service-owned automatic Pool distribution. One object and one authenticated
//! request are active; every pump performs at most one native or wire action.
use super::*;
use fabric::{
    healing,
    manifest::{Manifest, PlacementState},
    replica::{Checkpoint, ReplicaDescriptor, ReplicaState},
    resources::Resource,
};
use identity::StableId;
use iop::{
    remote::RemoteError,
    storage_protocol::{Operation, StorageOperationV1},
};
use node::types::NodeId;
use sha2::{Digest, Sha256};
#[path = "storage_pool_publication.rs"]
mod publication;
pub use publication::advertise;
#[path = "storage_pool_deletion.rs"]
mod deletion;
pub use deletion::retire_authority;
use publication::Publisher;

pub const CONFIG_BYTES: usize = 512;
#[derive(Clone, Copy)]
pub enum NativeRequest {
    Load {
        index: usize,
        owner: NodeId,
        scope: u64,
    },
    Commit {
        expected: u64,
        scope: u64,
        manifest: Manifest,
    },
    Read {
        object: [u8; 16],
        owner: NodeId,
        scope: u64,
        version: u64,
        offset: u64,
        length: u8,
    },
    LocalResource {
        owner: NodeId,
    },
    ConfigLoad,
    ConfigSave([u8; CONFIG_BYTES]),
    DeletionLoad {
        index: usize,
        owner: NodeId,
        scope: u64,
    },
    DeletionAck {
        object: [u8; 16],
        owner: NodeId,
        scope: u64,
        generation: u64,
        placement: usize,
    },
}
#[derive(Clone, Copy)]
pub enum NativeReply {
    Manifest(Option<Manifest>),
    Committed,
    Bytes { data: [u8; 64], length: u8 },
    Resource(Resource),
    Config([u8; CONFIG_BYTES]),
    Deletion(Option<fabric::deletion::Deletion>),
}
pub type NativeHandler = fn(NativeRequest) -> Result<NativeReply, RemoteError>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Participation {
    pub peer: NodeId,
    pub grants: [u64; 5],
    pub expires: u64,
    pub advertise: u64,
    pub advertise_expires: u64,
    pub delete: u64,
    pub delete_expires: u64,
}
#[derive(Clone, Copy)]
pub struct Configuration {
    pub scope: u64,
    pub peers: [Option<Participation>; 4],
}
impl Configuration {
    // ------------------------=
    // FUNC: empty
    // DESC: Starts with no persistent network authority or implicit participants.
    // ------------------=
    pub const fn empty() -> Self {
        Self {
            scope: 0,
            peers: [None; 4],
        }
    }
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes explicit scoped grants and leases without session or key material.
    // ------------------=
    pub fn encode(self) -> Result<[u8; CONFIG_BYTES], RemoteError> {
        let mut out = [0; CONFIG_BYTES];
        out[..8].copy_from_slice(b"INFPCF03");
        out[8..16].copy_from_slice(&self.scope.to_le_bytes());
        for (i, p) in self.peers.iter().enumerate() {
            if let Some(p) = p {
                if p.peer.0 == [0; 32]
                    || (p.grants != [0; 5] && (p.grants.contains(&0) || p.expires == 0))
                    || (p.grants == [0; 5] && p.advertise == 0 && p.delete == 0)
                    || ((p.advertise == 0) != (p.advertise_expires == 0))
                    || ((p.delete == 0) != (p.delete_expires == 0))
                    || self.peers[..i].iter().flatten().any(|q| q.peer == p.peer)
                {
                    return Err(RemoteError::MalformedRequest);
                }
                let at = 32 + i * 120;
                out[at] = 1;
                out[at + 8..at + 40].copy_from_slice(&p.peer.0);
                for n in 0..5 {
                    out[at + 40 + n * 8..at + 48 + n * 8]
                        .copy_from_slice(&p.grants[n].to_le_bytes());
                }
                out[at + 80..at + 88].copy_from_slice(&p.expires.to_le_bytes());
                out[at + 88..at + 96].copy_from_slice(&p.advertise.to_le_bytes());
                out[at + 96..at + 104].copy_from_slice(&p.advertise_expires.to_le_bytes());
                out[at + 104..at + 112].copy_from_slice(&p.delete.to_le_bytes());
                out[at + 112..at + 120].copy_from_slice(&p.delete_expires.to_le_bytes());
            }
        }
        Ok(out)
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Requires canonical bounded persistent configuration; corrupt state cannot enable authority.
    // ------------------=
    pub fn decode(bytes: &[u8; CONFIG_BYTES]) -> Result<Self, RemoteError> {
        let old = &bytes[..8] == b"INFPCF01";
        let modern = &bytes[..8] == b"INFPCF03";
        if !old && !modern && &bytes[..8] != b"INFPCF02" {
            return Err(RemoteError::MalformedRequest);
        }
        let mut c = Self::empty();
        c.scope = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        for i in 0..4 {
            let at = 32 + i * if modern { 120 } else { 112 };
            if bytes[at] == 1 {
                let mut grants = [0; 5];
                for n in 0..5 {
                    grants[n] = u64::from_le_bytes(
                        bytes[at + 40 + n * 8..at + 48 + n * 8].try_into().unwrap(),
                    );
                }
                c.peers[i] = Some(Participation {
                    peer: NodeId(bytes[at + 8..at + 40].try_into().unwrap()),
                    grants,
                    expires: u64::from_le_bytes(bytes[at + 80..at + 88].try_into().unwrap()),
                    advertise: u64::from_le_bytes(bytes[at + 88..at + 96].try_into().unwrap()),
                    advertise_expires: u64::from_le_bytes(
                        bytes[at + 96..at + 104].try_into().unwrap(),
                    ),
                    delete: if modern {
                        u64::from_le_bytes(bytes[at + 104..at + 112].try_into().unwrap())
                    } else {
                        0
                    },
                    delete_expires: if modern {
                        u64::from_le_bytes(bytes[at + 112..at + 120].try_into().unwrap())
                    } else {
                        0
                    },
                });
            }
        }
        let canonical = c.encode()?;
        let mut expected = [0; CONFIG_BYTES];
        expected[..32].copy_from_slice(&canonical[..32]);
        expected[..8].copy_from_slice(&bytes[..8]);
        let stride = if modern { 120 } else { 112 };
        for i in 0..4 {
            expected[32 + i * stride..32 + (i + 1) * stride]
                .copy_from_slice(&canonical[32 + i * 120..32 + i * 120 + stride]);
        }
        if expected != *bytes || (old && c.peers.iter().flatten().any(|p| p.advertise != 0)) {
            return Err(RemoteError::MalformedRequest);
        }
        Ok(c)
    }
}
#[derive(Clone, Copy)]
enum Phase {
    Begin,
    Read,
    WaitRead,
    Send,
    Commit,
    InspectReturned,
}
#[derive(Clone, Copy)]
struct Job {
    manifest: Manifest,
    destination: Resource,
    phase: Phase,
    offset: u64,
    data: [u8; 64],
    length: u8,
}
#[derive(Clone, Copy)]
struct Pending {
    request: u64,
    capability: u64,
    payload: StorageOperationV1,
    deadline: u64,
}
pub struct Coordinator {
    handler: Option<NativeHandler>,
    loaded: bool,
    config: Configuration,
    index: usize,
    next_scan: u64,
    local_refresh: bool,
    job: Option<Job>,
    pending: Option<Pending>,
    reader: Option<ReadJob>,
    read_result: Option<Result<([u8; 64], u8), RemoteError>>,
    notice: Option<iop::storage_protocol::StorageCommit>,
    cache: ReadCache,
    public_read: Option<PublicRead>,
    next_read: u64,
    pub completed: u64,
    pub last_read: u64,
    publisher: Publisher,
    read_tried: u8,
    deletion: deletion::Worker,
    pub last_error: Option<RemoteError>,
}
impl Coordinator {
    // ------------------------=
    // FUNC: publication_completed
    // DESC: Reports actual authenticated persistent-publisher acknowledgments, never scheduled attempts.
    // ------------------=
    pub fn publication_completed(&self) -> u64 {
        self.publisher.completed
    }
    // ------------------------=
    // FUNC: publication_last_error
    // DESC: Reports the persistent publisher's last actual authority or transport failure separately from replication work.
    // ------------------=
    pub fn publication_last_error(&self) -> Option<RemoteError> {
        self.publisher.last_error
    }
    // ------------------------=
    // FUNC: new
    // DESC: Creates a fixed single-job coordinator with no default participation.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            handler: None,
            loaded: false,
            config: Configuration::empty(),
            index: 0,
            next_scan: 0,
            local_refresh: true,
            job: None,
            pending: None,
            reader: None,
            read_result: None,
            notice: None,
            cache: ReadCache::new(),
            public_read: None,
            next_read: 1,
            completed: 0,
            last_read: 0,
            publisher: Publisher::new(),
            read_tried: 0,
            deletion: deletion::Worker::new(),
            last_error: None,
        }
    }
}
#[derive(Clone, Copy)]
enum PublicReadPhase {
    Find(usize),
    Local(Manifest),
    Remote,
}
/// ObjectRead value flag: request verified remote resolution without selecting a peer.
/// This is not a simulated local failure or proof of authority-node-loss survival.
pub const REMOTE_VERIFIED: u64 = 1 << 63;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadSource {
    LocalPreferred,
    RemoteVerified,
}
// ------------------------=
// FUNC: read_preference
// DESC: Decodes the bounded native ObjectRead length and explicit source preference.
// ------------------=
pub fn read_preference(value: u64) -> Result<(u8, ReadSource), RemoteError> {
    let length = value & !REMOTE_VERIFIED;
    if !(1..=64).contains(&length) {
        return Err(RemoteError::MalformedRequest);
    }
    Ok((
        length as u8,
        if value & REMOTE_VERIFIED != 0 {
            ReadSource::RemoteVerified
        } else {
            ReadSource::LocalPreferred
        },
    ))
}
#[derive(Clone, Copy)]
struct PublicRead {
    user: StableId,
    session: StableId,
    id: u64,
    request: StorageOperationV1,
    length: u8,
    source: ReadSource,
    phase: PublicReadPhase,
    result: Option<Result<StorageOperationV1, RemoteError>>,
    expires: u64,
}
// ------------------------=
// FUNC: submit_read
// DESC: Admits an owner-bound placement-independent object read, returning immediately; local validation and any verified remote fallback run only in bounded polls.
// ------------------=
pub fn submit_read(
    user: StableId,
    session: StableId,
    request: StorageOperationV1,
) -> Result<u64, RemoteError> {
    with_runtime(|r| submit_read_from(r, user, session, request))
        .ok_or(RemoteError::ServiceUnavailable)?
}
// ------------------------=
// FUNC: submit_read_from
// DESC: Admits a bounded source preference only after live session authorization.
// ------------------=
fn submit_read_from(
    r: &mut InfinityRuntime,
    user: StableId,
    session: StableId,
    request: StorageOperationV1,
) -> Result<u64, RemoteError> {
    if !storage_operator::authorized(r, user, session) {
        return Err(RemoteError::AccessDenied);
    }
    request
        .encode()
        .map_err(|_| RemoteError::MalformedRequest)?;
    let (length, source) = read_preference(request.value)?;
    if request.operation != Operation::ObjectRead
        || request.length != 0
        || request.scope != r.storage_coordinator.config.scope
    {
        return Err(RemoteError::MalformedRequest);
    }
    if r.storage_coordinator.public_read.is_some()
        || r.storage_coordinator.reader.is_some()
        || r.storage_coordinator.read_result.is_some()
    {
        return Err(RemoteError::QueueFull);
    }
    let now = r.node_clock.ok_or(RemoteError::ServiceUnavailable)?;
    let id = (1u64 << 63) | r.storage_coordinator.next_read;
    r.storage_coordinator.next_read = r
        .storage_coordinator
        .next_read
        .checked_add(1)
        .filter(|n| *n < (1u64 << 63))
        .ok_or(RemoteError::QueueFull)?;
    r.storage_coordinator.last_read = id;
    r.storage_coordinator.public_read = Some(PublicRead {
        user,
        session,
        id,
        request,
        length,
        source,
        phase: PublicReadPhase::Find(0),
        result: None,
        expires: now.saturating_add(300),
    });
    Ok(id)
}
// ------------------------=
// FUNC: take_read
// DESC: Returns a completed placement-independent read only to its originating still-authenticated operator session and consumes it exactly once.
// ------------------=
pub fn take_read(
    user: StableId,
    session: StableId,
    id: u64,
) -> Result<Option<StorageOperationV1>, RemoteError> {
    with_runtime(|r| take_read_from(r, user, session, id)).ok_or(RemoteError::ServiceUnavailable)?
}
// ------------------------=
// FUNC: take_read_from
// DESC: Consumes only the exact originating user, session and request completion.
// ------------------=
fn take_read_from(
    r: &mut InfinityRuntime,
    user: StableId,
    session: StableId,
    id: u64,
) -> Result<Option<StorageOperationV1>, RemoteError> {
    if !storage_operator::authorized(r, user, session) {
        return Err(RemoteError::AccessDenied);
    }
    let p = r
        .storage_coordinator
        .public_read
        .filter(|p| p.id == id && p.user == user && p.session == session)
        .ok_or(RemoteError::NotFound)?;
    let Some(result) = p.result else {
        return Ok(None);
    };
    r.storage_coordinator.public_read = None;
    result.map(Some)
}
// ------------------------=
// FUNC: public_read_pump
// DESC: Resolves the same ObjectId without physical-placement input, prefers verified local data and preserves asynchronous read ownership through fallback.
// ------------------=
fn public_read_pump(r: &mut InfinityRuntime, mut p: PublicRead, now: u64) -> PublicRead {
    if p.result.is_some() {
        return p;
    }
    let result = (|| -> Result<(), RemoteError> {
        match p.phase {
            PublicReadPhase::Find(index) => {
                let owner = r.nodes.local_id().ok_or(RemoteError::ServiceUnavailable)?;
                match native(
                    r,
                    NativeRequest::Load {
                        index,
                        owner,
                        scope: p.request.scope,
                    },
                    now,
                )? {
                    NativeReply::Manifest(Some(m)) if m.object == p.request.object => {
                        if m.version != p.request.object_version
                            || m.generation != p.request.manifest_generation
                            || m.authority_generation != p.request.authority_generation
                            || m.authority != owner
                        {
                            return Err(RemoteError::Conflict);
                        }
                        if p.request
                            .offset
                            .checked_add(p.length as u64)
                            .is_none_or(|end| end > m.length)
                        {
                            return Err(RemoteError::MalformedRequest);
                        }
                        if p.source == ReadSource::RemoteVerified {
                            // An explicit remote request must revalidate a live grant and
                            // actually fetch bytes, not reuse an earlier cached result.
                            r.storage_coordinator.cache.valid = false;
                            begin_remote_read(r, m, p.request.offset, p.length, now)?;
                            p.phase = PublicReadPhase::Remote;
                        } else {
                            p.phase = PublicReadPhase::Local(m);
                        }
                    }
                    _ if index < 7 => p.phase = PublicReadPhase::Find(index + 1),
                    _ => return Err(RemoteError::NotFound),
                }
            }
            PublicReadPhase::Local(m) => {
                match native(
                    r,
                    NativeRequest::Read {
                        object: m.object,
                        owner: m.authority,
                        scope: p.request.scope,
                        version: m.version,
                        offset: p.request.offset,
                        length: p.length,
                    },
                    now,
                ) {
                    Ok(NativeReply::Bytes { data, length }) if length == p.length => {
                        let mut reply = p.request;
                        reply.data = data;
                        reply.length = length as u16;
                        p.result = Some(Ok(reply));
                    }
                    Err(
                        RemoteError::NotFound
                        | RemoteError::InvalidState
                        | RemoteError::ServiceUnavailable,
                    ) => {
                        begin_remote_read(r, m, p.request.offset, p.length, now)?;
                        p.phase = PublicReadPhase::Remote;
                    }
                    Err(e) => return Err(e),
                    _ => return Err(RemoteError::InvalidState),
                }
            }
            PublicReadPhase::Remote => {
                if let Some(result) = take_remote_read(r) {
                    let (data, length) = result?;
                    let mut reply = p.request;
                    reply.data = data;
                    reply.length = length as u16;
                    p.result = Some(Ok(reply));
                }
            }
        }
        Ok(())
    })();
    if let Err(e) = result {
        p.result = Some(Err(e));
    }
    p
}
struct ReadCache {
    object: [u8; 16],
    version: u64,
    hash: [u8; 32],
    base: u64,
    bytes: [u8; 16384],
    valid: bool,
}
impl ReadCache {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves one bounded verified immutable-content cache, never a size-dependent object buffer.
    // ------------------=
    const fn new() -> Self {
        Self {
            object: [0; 16],
            version: 0,
            hash: [0; 32],
            base: 0,
            bytes: [0; 16384],
            valid: false,
        }
    }
}
struct ReadJob {
    manifest: Manifest,
    placement: fabric::manifest::Placement,
    base: u64,
    size: u64,
    position: u64,
    requested: u64,
    length: u8,
    result: [u8; 64],
    digest: Sha256,
    expected: [u8; 32],
    pending: Option<Pending>,
}
// ------------------------=
// FUNC: begin_remote_read
// DESC: Starts a bounded manifest-directed fallback after caller authority was checked by the IOP owner; returned bytes remain withheld until their entire immutable chunk verifies.
// ------------------=
pub(super) fn begin_remote_read(
    r: &mut InfinityRuntime,
    manifest: Manifest,
    offset: u64,
    length: u8,
    now: u64,
) -> Result<(), RemoteError> {
    r.storage_coordinator.read_tried = 0;
    start_remote_read(r, manifest, offset, length, now)
}
// ------------------------=
// FUNC: start_remote_read
// DESC: Selects an untried verified source from the same immutable manifest; retry always restarts the chunk digest and never mixes source versions.
// ------------------=
fn start_remote_read(
    r: &mut InfinityRuntime,
    manifest: Manifest,
    offset: u64,
    length: u8,
    now: u64,
) -> Result<(), RemoteError> {
    if r.storage_coordinator.reader.is_some() || r.storage_coordinator.read_result.is_some() {
        return Err(RemoteError::QueueFull);
    }
    manifest
        .validate()
        .map_err(|_| RemoteError::MalformedRequest)?;
    if length == 0
        || length > 64
        || offset
            .checked_add(length as u64)
            .is_none_or(|end| end > manifest.length)
    {
        return Err(RemoteError::MalformedRequest);
    }
    let mut base = 0;
    let mut selected = None;
    for chunk in manifest.chunks.iter().flatten() {
        let end = base + chunk.bytes as u64;
        if offset >= base && offset + length as u64 <= end {
            selected = Some((*chunk, base));
            break;
        }
        base = end;
    }
    let (chunk, base) = selected.ok_or(RemoteError::MalformedRequest)?;
    let cache = &mut r.storage_coordinator.cache;
    if cache.valid
        && cache.object == manifest.object
        && cache.version == manifest.version
        && cache.hash == chunk.hash
        && cache.base == base
    {
        let mut data = [0; 64];
        let at = (offset - base) as usize;
        data[..length as usize].copy_from_slice(&cache.bytes[at..at + length as usize]);
        r.storage_coordinator.read_result = Some(Ok((data, length)));
        return Ok(());
    }
    cache.valid = false;
    cache.object = manifest.object;
    cache.version = manifest.version;
    cache.hash = chunk.hash;
    cache.base = base;
    let placement = manifest
        .placements
        .iter()
        .enumerate()
        .filter_map(|(i, p)| p.map(|p| (i, p)))
        .find(|(i, p)| {
            if r.storage_coordinator.read_tried & (1 << i) != 0 {
                return false;
            }
            p.state == PlacementState::Verified
                && p.version == manifest.version
                && p.hash == manifest.hash
                && Some(p.node) != r.nodes.local_id()
                && grant(
                    &r.storage_coordinator.config,
                    p.node,
                    Operation::ObjectRead,
                    now,
                )
                .is_ok()
                && r.fabric_resources.entries().iter().flatten().any(|x| {
                    x.id == p.resource
                        && x.owner == p.node
                        && x.generation == p.generation
                        && x.online
                        && x.expires > now
                })
        })
        .map(|(_, p)| p)
        .ok_or(RemoteError::NotFound)?;
    r.storage_coordinator.reader = Some(ReadJob {
        manifest,
        placement,
        base,
        size: chunk.bytes as u64,
        position: 0,
        requested: offset,
        length,
        result: [0; 64],
        digest: Sha256::new(),
        expected: chunk.hash,
        pending: None,
    });
    Ok(())
}
// ------------------------=
// FUNC: take_remote_read
// DESC: Consumes the verified fallback result once; unverified intermediate transport data never reaches the caller.
// ------------------=
pub(super) fn take_remote_read(
    r: &mut InfinityRuntime,
) -> Option<Result<([u8; 64], u8), RemoteError>> {
    r.storage_coordinator.read_result.take()
}
// ------------------------=
// FUNC: read_pump
// DESC: Performs at most one sixty-four-byte authenticated range request or completion, validating the manifest chunk digest before releasing the requested slice.
// ------------------=
fn read_pump(
    r: &mut InfinityRuntime,
    mut job: ReadJob,
    now: u64,
) -> Result<Option<ReadJob>, RemoteError> {
    let caller = r
        .service_identity(SERVICE_REPLICA_STORAGE)
        .ok_or(RemoteError::ServiceUnavailable)?;
    if let Some(p) = job.pending {
        let response = r.iop.remote.take_storage_result(caller, p.request);
        if response.is_none() && now < p.deadline {
            return Ok(Some(job));
        }
        r.iop.remote.discard(caller, p.request);
        let _ = r.capabilities.retire_leaf(p.capability, caller);
        job.pending = None;
        let response = response.ok_or(RemoteError::DeadlineExceeded)?.result?;
        if response.operation != Operation::ObjectRead
            || response.object != job.manifest.object
            || response.object_version != job.manifest.version
            || response.manifest_generation != job.placement.admission_generation
            || response.offset != job.base + job.position
            || response.length as u64 != p.payload.value
        {
            return Err(RemoteError::UnknownResponse);
        }
        let at = job.position as usize;
        let n = response.length as usize;
        r.storage_coordinator.cache.bytes[at..at + n].copy_from_slice(&response.data[..n]);
        if absorb_read(&mut job, &response.data[..response.length as usize])? {
            r.storage_coordinator.cache.valid = true;
            r.storage_coordinator.read_result = Some(Ok((job.result, job.length)));
            return Ok(None);
        }
        return Ok(Some(job));
    }
    let grant = grant(
        &r.storage_coordinator.config,
        job.placement.node,
        Operation::ObjectRead,
        now,
    )?;
    let mut p = StorageOperationV1 {
        operation: Operation::ObjectRead,
        object: job.manifest.object,
        authority_generation: job.manifest.authority_generation,
        manifest_generation: job.placement.admission_generation,
        object_version: job.manifest.version,
        scope: r.storage_coordinator.config.scope,
        offset: job.base + job.position,
        value: (job.size - job.position).min(64),
        length: 33,
        data: [0; 64],
    };
    p.data[..32].copy_from_slice(&job.manifest.hash);
    p.data[32] = 1;
    let deadline = now.saturating_add(30);
    let cap = r
        .capabilities
        .grant(
            CapabilityType::ServiceCall,
            Operation::ObjectRead as u64,
            1,
            p.scope,
            caller,
            caller,
            Some(deadline),
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
                job.placement.node,
                grant,
                p,
                id,
                id,
                now,
                deadline,
            )
        });
    match result {
        Ok(request) => {
            job.pending = Some(Pending {
                request,
                capability: cap,
                payload: p,
                deadline,
            })
        }
        Err(e) => {
            let _ = r.capabilities.retire_leaf(cap, caller);
            return Err(e);
        }
    }
    Ok(Some(job))
}

// ------------------------=
// FUNC: absorb_read
// DESC: Accumulates a bounded manifest chunk and releases no successful result until the complete expected integrity digest matches.
// ------------------=
fn absorb_read(job: &mut ReadJob, bytes: &[u8]) -> Result<bool, RemoteError> {
    if bytes.is_empty()
        || bytes.len() > 64
        || job
            .position
            .checked_add(bytes.len() as u64)
            .is_none_or(|end| end > job.size)
    {
        return Err(RemoteError::MalformedRequest);
    }
    job.digest.update(bytes);
    let start = job.base + job.position;
    let end = start + bytes.len() as u64;
    let a = start.max(job.requested);
    let b = end.min(job.requested + job.length as u64);
    if a < b {
        job.result[(a - job.requested) as usize..(b - job.requested) as usize]
            .copy_from_slice(&bytes[(a - start) as usize..(b - start) as usize]);
    }
    job.position += bytes.len() as u64;
    if job.position < job.size {
        return Ok(false);
    }
    let actual: [u8; 32] = job.digest.clone().finalize().into();
    if actual != job.expected {
        return Err(RemoteError::InvalidState);
    }
    Ok(true)
}
// ------------------------=
// FUNC: register
// DESC: Registers the installed native storage service implementation, not an alternate remote transport.
// ------------------=
pub fn register(handler: NativeHandler) {
    with_runtime(|r| {
        r.storage_coordinator.handler = Some(handler);
        r.storage_coordinator.loaded = false;
    });
}
// ------------------------=
// FUNC: native
// DESC: Revalidates an exact short-lived service-owned capability before each bounded native backend operation and retires it afterward.
// ------------------=
fn native(
    r: &mut InfinityRuntime,
    request: NativeRequest,
    now: u64,
) -> Result<NativeReply, RemoteError> {
    let handler = r
        .storage_coordinator
        .handler
        .ok_or(RemoteError::ServiceUnavailable)?;
    let service = r
        .service_identity(SERVICE_REPLICA_STORAGE)
        .ok_or(RemoteError::ServiceUnavailable)?;
    let op = match request {
        NativeRequest::Read { .. } => Operation::ObjectRead,
        NativeRequest::Commit { .. } => Operation::PoolHeal,
        NativeRequest::LocalResource { .. } => Operation::ResourceInspect,
        NativeRequest::DeletionAck { .. } => Operation::ReplicaDelete,
        _ => Operation::PoolInspect,
    };
    let scope = r.storage_coordinator.config.scope;
    let cap = r
        .capabilities
        .grant(
            CapabilityType::ServiceCall,
            op as u64,
            1,
            scope,
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
            op as u64,
            1,
            scope,
            now,
        )
        .map_err(|_| RemoteError::AccessDenied)
        .and_then(|_| handler(request));
    let _ = r.capabilities.retire_leaf(cap, service);
    result
}
// ------------------------=
// FUNC: configure
// DESC: Commits explicitly approved participation before enabling automatic work; every subsequent remote operation still validates the peer-issued grant.
// ------------------=
pub fn configure(
    user: StableId,
    session: StableId,
    config: Configuration,
) -> Result<(), RemoteError> {
    with_runtime(|r| {
        if !storage_operator::authorized(r, user, session) {
            return Err(RemoteError::AccessDenied);
        }
        if r.storage_coordinator.job.is_some() {
            return Err(RemoteError::Conflict);
        }
        let now = r.node_clock.ok_or(RemoteError::ServiceUnavailable)?;
        let bytes = config.encode()?;
        if config
            .peers
            .iter()
            .flatten()
            .any(|p| Some(p.peer) == r.nodes.local_id())
        {
            return Err(RemoteError::AccessDenied);
        }
        match native(r, NativeRequest::ConfigSave(bytes), now)? {
            NativeReply::Committed => {
                r.storage_coordinator.config = config;
                r.storage_coordinator.loaded = true;
                Ok(())
            }
            _ => Err(RemoteError::InvalidState),
        }
    })
    .ok_or(RemoteError::ServiceUnavailable)?
}
// ------------------------=
// FUNC: participate
// DESC: Adds or renews one explicitly consented peer while preserving the bounded existing configuration and requiring the same native capability gate as all configuration changes.
// ------------------=
pub fn participate(
    user: StableId,
    session: StableId,
    peer: NodeId,
    grants: [u64; 5],
    lease: u64,
) -> Result<(), RemoteError> {
    let config = with_runtime(|r| {
        if !storage_operator::authorized(r, user, session) || !(1..=3600).contains(&lease) {
            return Err(RemoteError::AccessDenied);
        }
        let now = r.node_clock.ok_or(RemoteError::ServiceUnavailable)?;
        if !r.storage_coordinator.loaded {
            return Err(RemoteError::ServiceUnavailable);
        }
        let mut config = r.storage_coordinator.config;
        let slot = config
            .peers
            .iter()
            .position(|p| p.is_some_and(|p| p.peer == peer))
            .or_else(|| config.peers.iter().position(Option::is_none))
            .ok_or(RemoteError::QueueFull)?;
        config.peers[slot] = Some(Participation {
            peer,
            grants,
            expires: now.saturating_add(lease),
            advertise: config.peers[slot].map(|p| p.advertise).unwrap_or(0),
            advertise_expires: config.peers[slot].map(|p| p.advertise_expires).unwrap_or(0),
            delete: config.peers[slot].map(|p| p.delete).unwrap_or(0),
            delete_expires: config.peers[slot].map(|p| p.delete_expires).unwrap_or(0),
        });
        Ok(config)
    })
    .ok_or(RemoteError::ServiceUnavailable)??;
    configure(user, session, config)
}

// ------------------------=
// FUNC: persist
// DESC: Publishes a manifest successor only after its native generation compare-and-swap commits.
// ------------------=
fn persist(
    r: &mut InfinityRuntime,
    expected: u64,
    manifest: &Manifest,
    now: u64,
) -> Result<(), fabric::manifest::ManifestError> {
    let scope = r.storage_coordinator.config.scope;
    match native(
        r,
        NativeRequest::Commit {
            expected,
            scope,
            manifest: *manifest,
        },
        now,
    ) {
        Ok(NativeReply::Committed) => {
            r.storage_coordinator.notice = Some(iop::storage_protocol::StorageCommit {
                event: iop::storage_protocol::EVENT_OBJECT_CHANGED,
                object: manifest.object,
                generation: manifest.generation,
                copied: manifest.length,
                state: if manifest.healing.is_some() {
                    4
                } else {
                    match manifest.availability() {
                        fabric::placement::Availability::Healthy => 1,
                        fabric::placement::Availability::Degraded => 2,
                        fabric::placement::Availability::Offline => 3,
                    }
                },
                correlation: manifest
                    .healing
                    .map(|c| c.token)
                    .unwrap_or(manifest.generation),
                causation: expected,
            });
            Ok(())
        }
        Err(RemoteError::Conflict) => Err(fabric::manifest::ManifestError::Stale),
        _ => Err(fabric::manifest::ManifestError::Storage),
    }
}
// ------------------------=
// FUNC: grant
// DESC: Selects only the exact explicitly configured operation grant under its unexpired participation lease.
// ------------------=
fn grant(c: &Configuration, peer: NodeId, op: Operation, now: u64) -> Result<u64, RemoteError> {
    if op == Operation::ReplicaDelete {
        return c
            .peers
            .iter()
            .flatten()
            .find(|p| p.peer == peer && p.delete != 0 && p.delete_expires > now)
            .map(|p| p.delete)
            .ok_or(RemoteError::AccessDenied);
    }
    if op == Operation::ResourceAdvertise {
        return c
            .peers
            .iter()
            .flatten()
            .find(|p| p.peer == peer && p.advertise != 0 && p.advertise_expires > now)
            .map(|p| p.advertise)
            .ok_or(RemoteError::AccessDenied);
    }
    let i = match op {
        Operation::TransferBegin => 0,
        Operation::TransferChunk => 1,
        Operation::TransferCommit => 2,
        Operation::ReplicaInspect => 3,
        Operation::ObjectRead => 4,
        _ => return Err(RemoteError::AccessDenied),
    };
    c.peers
        .iter()
        .flatten()
        .find(|p| p.peer == peer && p.expires > now && p.grants[i] != 0)
        .map(|p| p.grants[i])
        .ok_or(RemoteError::AccessDenied)
}
// ------------------------=
// FUNC: submit
// DESC: Enqueues one authenticated native IOP request without waiting or granting broad transport authority.
// ------------------=
fn submit(
    r: &mut InfinityRuntime,
    peer: NodeId,
    p: StorageOperationV1,
    now: u64,
) -> Result<(), RemoteError> {
    let grant = grant(&r.storage_coordinator.config, peer, p.operation, now)?;
    let caller = r
        .service_identity(SERVICE_REPLICA_STORAGE)
        .ok_or(RemoteError::ServiceUnavailable)?;
    let deadline = now.saturating_add(30);
    let cap = r
        .capabilities
        .grant(
            CapabilityType::ServiceCall,
            p.operation as u64,
            1,
            p.scope,
            caller,
            caller,
            Some(deadline),
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
                p,
                id,
                id,
                now,
                deadline,
            )
        });
    match result {
        Ok(request) => {
            r.storage_coordinator.pending = Some(Pending {
                request,
                capability: cap,
                payload: p,
                deadline,
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
// FUNC: packet
// DESC: Binds each transfer step to the immutable version and durable claim token that admitted its recipient binding.
// ------------------=
fn packet(job: Job, scope: u64) -> StorageOperationV1 {
    let claim = job.manifest.healing.unwrap();
    StorageOperationV1 {
        operation: Operation::TransferBegin,
        object: job.manifest.object,
        authority_generation: job.manifest.authority_generation,
        manifest_generation: claim.token,
        object_version: job.manifest.version,
        scope,
        offset: job.offset,
        value: claim.token,
        length: 0,
        data: [0; 64],
    }
}
// ------------------------=
// FUNC: pump
// DESC: Advances one bounded state transition; recipient checkpoints determine resumed offsets and only a durable verified receipt can publish a placement.
// ------------------=
fn pump(r: &mut InfinityRuntime, now: u64) -> Result<(), RemoteError> {
    if !r.storage_coordinator.loaded {
        let config = match native(r, NativeRequest::ConfigLoad, now) {
            Ok(NativeReply::Config(b)) => Configuration::decode(&b)?,
            Err(RemoteError::NotFound) => Configuration::empty(),
            Err(e) => return Err(e),
            _ => return Err(RemoteError::InvalidState),
        };
        r.storage_coordinator.config = config;
        r.storage_coordinator.loaded = true;
        return Ok(());
    }
    if let Some(p) = r.storage_coordinator.pending {
        let caller = r
            .service_identity(SERVICE_REPLICA_STORAGE)
            .ok_or(RemoteError::ServiceUnavailable)?;
        let result = r.iop.remote.take_storage_result(caller, p.request);
        if result.is_none() && now < p.deadline {
            return Ok(());
        }
        r.iop.remote.discard(caller, p.request);
        let _ = r.capabilities.retire_leaf(p.capability, caller);
        r.storage_coordinator.pending = None;
        let reply = result.ok_or(RemoteError::DeadlineExceeded)?.result?;
        let mut job = r.storage_coordinator.job.ok_or(RemoteError::InvalidState)?;
        if matches!(job.phase, Phase::InspectReturned) {
            let prior = job
                .manifest
                .placements
                .iter()
                .flatten()
                .find(|x| x.resource == job.destination.id)
                .copied()
                .ok_or(RemoteError::InvalidState)?;
            if reply.operation != Operation::ReplicaInspect
                || reply.object != job.manifest.object
                || reply.object_version != prior.version
                || reply.manifest_generation != prior.admission_generation
                || reply.length != 49
                || reply.data[0] != 4
                || reply.data[9..41] != prior.hash
            {
                return Err(RemoteError::UnknownResponse);
            }
            let mut updated = job.manifest;
            let expected = updated.generation;
            let d = r.fabric_resources.clone();
            healing::reconcile(
                &mut updated,
                &d,
                job.manifest.authority,
                expected,
                now,
                fabric::manifest::Placement {
                    state: PlacementState::Verified,
                    ..prior
                },
                |g, m| persist(r, g, m, now),
            )
            .map_err(|_| RemoteError::Conflict)?;
            r.storage_coordinator.job = None;
            return Ok(());
        }
        if reply.operation != p.payload.operation
            || reply.object != job.manifest.object
            || reply.object_version != job.manifest.version
            || reply.manifest_generation != job.manifest.healing.unwrap().token
            || reply.length != 49
            || reply.data[1..9] != job.manifest.length.to_le_bytes()
            || reply.data[9..41] != job.manifest.hash
            || reply.offset > job.manifest.length
        {
            return Err(RemoteError::UnknownResponse);
        }
        job.offset = reply.offset;
        if reply.data[0] == 4 {
            let m = job.manifest;
            let claim = m.healing.unwrap();
            let mut updated = m;
            let mut d = r.fabric_resources.clone();
            let receipt = Checkpoint {
                descriptor: ReplicaDescriptor {
                    job: claim.token,
                    object: m.object,
                    version: m.version,
                    resource: job.destination.id,
                    generation: job.destination.generation,
                    bytes: m.length,
                    hash: m.hash,
                },
                copied: reply.offset,
                state: ReplicaState::Available,
            };
            healing::complete(
                &mut updated,
                &mut d,
                m.authority,
                m.generation,
                now,
                claim.token,
                receipt,
                |g, m| persist(r, g, m, now),
            )
            .map_err(|_| RemoteError::Conflict)?;
            r.fabric_resources = d;
            r.storage_coordinator.job = None;
            r.storage_coordinator.completed += 1;
        } else {
            job.phase = if job.offset == job.manifest.length {
                Phase::Commit
            } else {
                Phase::Read
            };
            r.storage_coordinator.job = Some(job);
        }
        return Ok(());
    }
    if let Some(mut job) = r.storage_coordinator.job {
        let scope = r.storage_coordinator.config.scope;
        match job.phase {
            Phase::InspectReturned => {
                let p = job
                    .manifest
                    .placements
                    .iter()
                    .flatten()
                    .find(|x| x.resource == job.destination.id)
                    .ok_or(RemoteError::InvalidState)?;
                submit(
                    r,
                    p.node,
                    StorageOperationV1 {
                        operation: Operation::ReplicaInspect,
                        object: job.manifest.object,
                        authority_generation: job.manifest.authority_generation,
                        manifest_generation: p.admission_generation,
                        object_version: p.version,
                        scope,
                        offset: 0,
                        value: 0,
                        length: 0,
                        data: [0; 64],
                    },
                    now,
                )?;
            }
            Phase::Read => {
                let length = (job.manifest.length - job.offset).min(64) as u8;
                match native(
                    r,
                    NativeRequest::Read {
                        object: job.manifest.object,
                        owner: job.manifest.authority,
                        scope,
                        version: job.manifest.version,
                        offset: job.offset,
                        length,
                    },
                    now,
                ) {
                    Ok(NativeReply::Bytes { data, length: n }) if n == length => {
                        job.data = data;
                        job.length = n;
                        job.phase = Phase::Send;
                        r.storage_coordinator.job = Some(job);
                    }
                    Err(
                        RemoteError::NotFound
                        | RemoteError::InvalidState
                        | RemoteError::ServiceUnavailable,
                    ) => {
                        begin_remote_read(r, job.manifest, job.offset, length, now)?;
                        job.phase = Phase::WaitRead;
                        r.storage_coordinator.job = Some(job);
                    }
                    Err(e) => return Err(e),
                    _ => return Err(RemoteError::InvalidState),
                }
            }
            Phase::WaitRead => {
                if let Some(result) = take_remote_read(r) {
                    let (data, length) = result?;
                    job.data = data;
                    job.length = length;
                    job.phase = Phase::Send;
                    r.storage_coordinator.job = Some(job);
                }
            }
            _ => {
                let mut p = packet(job, scope);
                match job.phase {
                    Phase::Begin => {
                        p.offset = job.manifest.length;
                        p.length = 56;
                        p.data[..16].copy_from_slice(&job.destination.id.0);
                        p.data[16..24].copy_from_slice(&job.destination.generation.to_le_bytes());
                        p.data[24..56].copy_from_slice(&job.manifest.hash);
                    }
                    Phase::Send => {
                        p.operation = Operation::TransferChunk;
                        p.length = job.length as u16;
                        p.data = job.data;
                    }
                    Phase::Commit => p.operation = Operation::TransferCommit,
                    Phase::Read | Phase::WaitRead | Phase::InspectReturned => unreachable!(),
                }
                submit(r, job.destination.owner, p, now)?;
            }
        }
        return Ok(());
    }
    if now < r.storage_coordinator.next_scan {
        return Ok(());
    }
    if r.storage_coordinator.local_refresh {
        let owner = r.nodes.local_id().ok_or(RemoteError::InvalidState)?;
        if let NativeReply::Resource(mut resource) =
            native(r, NativeRequest::LocalResource { owner }, now)?
        {
            resource.expires = now.saturating_add(60);
            r.fabric_resources
                .observe_local(
                    resource,
                    r.nodes.local_id().ok_or(RemoteError::InvalidState)?,
                    now,
                )
                .map_err(|_| RemoteError::Conflict)?;
        } else {
            return Err(RemoteError::InvalidState);
        }
        r.storage_coordinator.local_refresh = false;
        return Ok(());
    }
    let owner = r.nodes.local_id().ok_or(RemoteError::ServiceUnavailable)?;
    let index = r.storage_coordinator.index;
    let scope = r.storage_coordinator.config.scope;
    r.storage_coordinator.index = (index + 1) % 8;
    if index == 7 {
        r.storage_coordinator.next_scan = now.saturating_add(2);
        r.storage_coordinator.local_refresh = true;
    }
    let NativeReply::Manifest(Some(mut m)) = native(
        r,
        NativeRequest::Load {
            index,
            owner,
            scope,
        },
        now,
    )?
    else {
        return Ok(());
    };
    if m.authority != owner {
        return Ok(());
    }
    let mut d = r.fabric_resources.clone();
    // Unapproved destinations remain observable but cannot participate in placement.
    for resource in *d.entries() {
        if let Some(resource) = resource {
            if resource.owner != owner
                && grant(
                    &r.storage_coordinator.config,
                    resource.owner,
                    Operation::TransferBegin,
                    now,
                )
                .is_err()
            {
                d.mark_peer_offline(resource.owner);
            }
        }
    }
    let expected = m.generation;
    if healing::observe_loss(&mut m, &d, owner, expected, now, |g, m| {
        persist(r, g, m, now)
    })
    .map_err(|_| RemoteError::Conflict)?
    {
        return Ok(());
    }
    if m.healing.is_none() {
        if let Some(destination) = m
            .placements
            .iter()
            .flatten()
            .filter(|p| p.state == PlacementState::Offline)
            .find_map(|p| {
                d.entries()
                    .iter()
                    .flatten()
                    .find(|x| {
                        x.id == p.resource
                            && x.generation == p.generation
                            && x.online
                            && x.expires > now
                            && grant(
                                &r.storage_coordinator.config,
                                x.owner,
                                Operation::ReplicaInspect,
                                now,
                            )
                            .is_ok()
                    })
                    .copied()
            })
        {
            r.storage_coordinator.job = Some(Job {
                manifest: m,
                destination,
                phase: Phase::InspectReturned,
                offset: 0,
                data: [0; 64],
                length: 0,
            });
            return Ok(());
        }
    }
    let destination = if let Some(claim) = m.healing {
        let Some(dest) = d
            .entries()
            .iter()
            .flatten()
            .find(|x| {
                x.id == claim.destination
                    && x.generation == claim.destination_generation
                    && x.online
                    && x.expires > now
            })
            .copied()
        else {
            return Ok(());
        };
        if claim.expires <= now {
            let mut next = m;
            next.generation += 1;
            next.healing.as_mut().unwrap().expires = now.saturating_add(3600);
            persist(r, m.generation, &next, now).map_err(|_| RemoteError::Conflict)?;
            m = next;
        }
        d.restore_one_reservation(
            claim.destination,
            claim.destination_generation,
            claim.reserved,
        )
        .map_err(|_| RemoteError::Conflict)?;
        dest
    } else {
        let expected = m.generation;
        match healing::begin(
            &mut m,
            &mut d,
            owner,
            expected,
            now,
            now.saturating_add(3600),
            |g, m| persist(r, g, m, now),
        ) {
            Ok(work) => work.destination,
            Err(
                healing::HealError::NoWork
                | healing::HealError::NoCapacity
                | healing::HealError::NoSource,
            ) => return Ok(()),
            Err(_) => return Err(RemoteError::Conflict),
        }
    };
    r.fabric_resources = d;
    r.storage_coordinator.job = Some(Job {
        manifest: m,
        destination,
        phase: Phase::Begin,
        offset: 0,
        data: [0; 64],
        length: 0,
    });
    Ok(())
}
// ------------------------=
// FUNC: poll
// DESC: Pumps once and retains durable claims after failures for checkpoint resume, with bounded retry pacing and no UI wait.
// ------------------=
pub(super) fn poll(r: &mut InfinityRuntime, now: u64) {
    if r.storage_coordinator.loaded && publication::poll(r, now) {
        return;
    }
    if r.storage_coordinator.loaded && deletion::poll(r, now) {
        return;
    }
    if let Some(p) = r.storage_coordinator.public_read {
        if now >= p.expires || !storage_operator::authorized(r, p.user, p.session) {
            if let Some(job) = r.storage_coordinator.reader.take() {
                if let Some(p) = job.pending {
                    if let Some(caller) = r.service_identity(SERVICE_REPLICA_STORAGE) {
                        r.iop.remote.discard(caller, p.request);
                        let _ = r.capabilities.retire_leaf(p.capability, caller);
                    }
                }
            }
            r.storage_coordinator.read_result = None;
            r.storage_coordinator.public_read = None;
        }
    }
    if let Some(notice) = r.storage_coordinator.notice {
        if publish_storage_commit_from(r, notice, now) {
            r.storage_coordinator.notice = None;
        }
    }
    if let Some(job) = r.storage_coordinator.reader.take() {
        let retry = (job.manifest, job.requested, job.length, job.placement.node);
        match read_pump(r, job, now) {
            Ok(next) => r.storage_coordinator.reader = next,
            Err(e) => {
                for (i, p) in retry.0.placements.iter().enumerate() {
                    if p.is_some_and(|p| p.node == retry.3) {
                        r.storage_coordinator.read_tried |= 1 << i;
                    }
                }
                if start_remote_read(r, retry.0, retry.1, retry.2, now).is_err() {
                    r.storage_coordinator.read_result = Some(Err(e));
                }
            }
        }
        return;
    }
    if let Some(p) = r.storage_coordinator.public_read {
        if p.result.is_none() {
            let updated = public_read_pump(r, p, now);
            r.storage_coordinator.public_read = Some(updated);
            return;
        }
    }
    if let Err(error) = pump(r, now) {
        r.storage_coordinator.last_error = Some(error);
        if r.storage_coordinator.job.is_some() {
            r.storage_coordinator.index = (r.storage_coordinator.index + 7) % 8;
        }
        r.storage_coordinator.job = None;
        r.storage_coordinator.next_scan = now.saturating_add(2);
    }
}

#[cfg(test)]
#[path = "storage_coordinator_tests.rs"]
mod tests;
