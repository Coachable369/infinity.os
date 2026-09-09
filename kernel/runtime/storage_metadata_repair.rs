//! Delegated repair service boundary. Native storage remains responsible for
//! durable owner-anchor validation and exact writer/destination authorization.
use super::{
    fabric::metadata_repair::{RepairAuthorization, RepairBundle},
    iop::{
        remote::{AuthenticatedStorageRequest, RemoteError},
        storage_protocol::StorageOperationV1,
    },
    node::types::NodeId,
};

pub enum NativeRequest {
    Load {
        object: [u8; 16],
        anchor: [u8; 32],
        now: u64,
    },
    Stage {
        authorization: RepairAuthorization,
        now: u64,
    },
    Publish {
        authorization: RepairAuthorization,
        now: u64,
    },
    Read {
        authorization: RepairAuthorization,
        writer: NodeId,
        offset: u64,
        length: u8,
        now: u64,
    },
    Wire {
        request: AuthenticatedStorageRequest,
        now: u64,
    },
}
pub enum NativeReply {
    Overlay(Option<RepairBundle>),
    Done,
    Bytes { data: [u8; 64], length: u8 },
    Wire(StorageOperationV1),
}
pub type NativeHandler = fn(NativeRequest) -> Result<NativeReply, RemoteError>;

use super::*;
use fabric::{
    manifest::{Placement, PlacementState, MANIFEST_BYTES},
    metadata::Receipt,
    metadata_bundle::Bundle,
    metadata_repair::{
        RepairAvailability, RepairCertificate, RepairPublication, RepairReadRound, RepairRecord,
        REPAIR_AUTHORIZATION_BYTES,
    },
    resources::{Health, ResourceKind},
};
use identity::StableId;
use iop::storage_protocol::Operation;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Owner,
    LocalHead,
    Head,
    Fetch,
    Resolve,
    Begin,
    Chunk,
    End,
    Build,
    Read,
    Copy,
    Commit,
    Inspect,
    Attest,
    Stage,
    Publish,
    Done,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Upload {
    Writeback,
    Destination,
    Prepare,
    Publish,
}
#[derive(Clone, Copy)]
struct Pending {
    id: u64,
    cap: u64,
    peer: NodeId,
    request: StorageOperationV1,
    deadline: u64,
}
struct Job {
    id: u64,
    user: StableId,
    session: StableId,
    object: [u8; 16],
    destination: NodeId,
    fresh: u64,
    anchor: Option<Bundle>,
    authorization: Option<RepairAuthorization>,
    round: Option<RepairReadRound>,
    phase: Phase,
    upload: Upload,
    member: usize,
    at: usize,
    offset: u64,
    bytes: [u8; REPAIR_AUTHORIZATION_BYTES],
    data: [u8; 64],
    length: u8,
    receipt: Option<Receipt>,
    publication: Option<RepairPublication>,
    deadline: u64,
    digest: Sha256,
    head_sequence: u64,
    head_digest: [u8; 32],
    result: Option<Result<u64, RemoteError>>,
}
pub struct Service {
    pub handler: Option<NativeHandler>,
    job: Option<Job>,
    pending: Option<Pending>,
    next: u64,
    pub completed: u64,
    pub last_error: Option<RemoteError>,
}
impl Service {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates one repair and one wire request; no authority or mutable owner identity is synthesized.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            handler: None,
            job: None,
            pending: None,
            next: 1,
            completed: 0,
            last_error: None,
        }
    }
}
// ------------------------=
// FUNC: register
// DESC: Installs the native repair backend shared by installed boot and authenticated remote dispatch.
// ------------------=
pub fn register(handler: NativeHandler) {
    with_runtime(|r| r.storage_metadata_repair.handler = Some(handler));
}
// ------------------------=
// FUNC: begin
// DESC: Starts an operator-owned fresh quorum repair, never accepting caller-provided physical bytes or a forged owner identity.
// ------------------=
pub fn begin(
    user: StableId,
    session: StableId,
    object: [u8; 16],
    destination: NodeId,
) -> Result<u64, RemoteError> {
    with_runtime(|r| {
        if !storage_operator::authorized(r, user, session) {
            return Err(RemoteError::AccessDenied);
        }
        if r.storage_metadata_repair.job.is_some() {
            return Err(RemoteError::QueueFull);
        }
        if object == [0; 16] || destination.0 == [0; 32] || Some(destination) == r.nodes.local_id()
        {
            return Err(RemoteError::MalformedRequest);
        }
        let now = r.node_clock.ok_or(RemoteError::ServiceUnavailable)?;
        let fresh = storage_metadata::fresh_start(r, user, session, object)?;
        let id = (1u64 << 61) | r.storage_metadata_repair.next;
        r.storage_metadata_repair.next = r
            .storage_metadata_repair
            .next
            .checked_add(1)
            .ok_or(RemoteError::QueueFull)?;
        r.storage_metadata_repair.job = Some(Job {
            id,
            user,
            session,
            object,
            destination,
            fresh,
            anchor: None,
            authorization: None,
            round: None,
            phase: Phase::Owner,
            upload: Upload::Destination,
            member: 0,
            at: 0,
            offset: 0,
            bytes: [0; REPAIR_AUTHORIZATION_BYTES],
            data: [0; 64],
            length: 0,
            receipt: None,
            publication: None,
            deadline: now.saturating_add(3600),
            digest: Sha256::new(),
            head_sequence: 0,
            head_digest: [0; 32],
            result: None,
        });
        Ok(id)
    })
    .ok_or(RemoteError::ServiceUnavailable)?
}
// ------------------------=
// FUNC: take
// DESC: Consumes only the initiating still-authorized session's verified repair publication result.
// ------------------=
pub fn take(user: StableId, session: StableId, id: u64) -> Result<Option<u64>, RemoteError> {
    with_runtime(|r| {
        if !storage_operator::authorized(r, user, session) {
            return Err(RemoteError::AccessDenied);
        }
        let j = r
            .storage_metadata_repair
            .job
            .as_ref()
            .filter(|j| j.id == id && j.user == user && j.session == session)
            .ok_or(RemoteError::NotFound)?;
        let Some(result) = j.result else {
            return Ok(None);
        };
        r.storage_metadata_repair.job = None;
        result.map(Some)
    })
    .ok_or(RemoteError::ServiceUnavailable)?
}
// ------------------------=
// FUNC: native
// DESC: Validates and retires an exact service capability around each single bounded native persistence operation.
// ------------------=
fn native(r: &mut InfinityRuntime, p: NativeRequest, now: u64) -> Result<NativeReply, RemoteError> {
    let caller = r
        .service_identity(SERVICE_REPLICA_STORAGE)
        .ok_or(RemoteError::ServiceUnavailable)?;
    let handler = r
        .storage_metadata_repair
        .handler
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
            Some(now.saturating_add(1)),
            0,
        )
        .map_err(|_| RemoteError::QueueFull)?;
    let result = r
        .capabilities
        .validate(
            cap,
            caller,
            CapabilityType::ServiceCall,
            Operation::PoolMetadata as u64,
            1,
            0,
            now,
        )
        .map_err(|_| RemoteError::AccessDenied)
        .and_then(|_| handler(p));
    let _ = r.capabilities.retire_leaf(cap, caller);
    result
}
// ------------------------=
// FUNC: packet
// DESC: Creates a canonical fixed repair envelope bound to the certified object and version.
// ------------------=
fn packet(j: &Job, action: u64) -> StorageOperationV1 {
    let b = j.anchor.unwrap();
    StorageOperationV1 {
        operation: Operation::PoolMetadata,
        object: j.object,
        authority_generation: b.manifest.authority_generation,
        manifest_generation: j
            .authorization
            .map_or(b.manifest.generation, |a| a.repair.manifest.generation),
        object_version: b.manifest.version,
        offset: 0,
        scope: 0,
        value: action,
        length: 0,
        data: [0; 64],
    }
}
// ------------------------=
// FUNC: send
// DESC: Enqueues one ordinary authenticated IOP request with an explicit live metadata grant and bounded deadline.
// ------------------=
fn send(
    r: &mut InfinityRuntime,
    peer: NodeId,
    p: StorageOperationV1,
    now: u64,
) -> Result<(), RemoteError> {
    let grant = storage_metadata::peer_grant(r, peer, now)?;
    let caller = r
        .service_identity(SERVICE_REPLICA_STORAGE)
        .ok_or(RemoteError::ServiceUnavailable)?;
    let deadline = now.saturating_add(30);
    let cap = r
        .capabilities
        .grant(
            CapabilityType::ServiceCall,
            Operation::PoolMetadata as u64,
            1,
            0,
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
        Ok(id) => {
            r.storage_metadata_repair.pending = Some(Pending {
                id,
                cap,
                peer,
                request: p,
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
// FUNC: member
// DESC: Walks fixed authenticated metadata membership without substituting unavailable peers or counting timeouts.
// ------------------=
fn member(j: &mut Job, local: NodeId) -> Option<NodeId> {
    let g = j.anchor.unwrap().group;
    while j.member < 3 && g.members[j.member] == local {
        j.member += 1;
    }
    g.members.get(j.member).copied()
}
// ------------------------=
// FUNC: observe
// DESC: Feeds only a correlated authenticated head response into the fresh repair read quorum.
// ------------------=
fn observe(
    j: &mut Job,
    peer: NodeId,
    value: Option<RepairBundle>,
    now: u64,
) -> Result<(), RemoteError> {
    let b = j.anchor.unwrap();
    j.round
        .as_mut()
        .ok_or(RemoteError::InvalidState)?
        .observe_authenticated(
            &b.group,
            &b.certificate.ok_or(RemoteError::InvalidState)?,
            &b.manifest,
            peer,
            value,
            now,
        )
        .map_err(|_| RemoteError::Conflict)
}
// ------------------------=
// FUNC: receipt
// DESC: Signs a repair receipt only after the native durable stage or publication callback has succeeded.
// ------------------=
fn receipt(r: &InfinityRuntime, j: &Job, published: bool) -> Result<Receipt, RemoteError> {
    let a = j.authorization.ok_or(RemoteError::InvalidState)?;
    let local = r.nodes.local_id().ok_or(RemoteError::ServiceUnavailable)?;
    let index = a
        .anchor
        .group
        .members
        .iter()
        .position(|p| *p == local)
        .ok_or(RemoteError::AccessDenied)?;
    let mut value = Receipt {
        member: index as u8,
        digest: a.repair.value.digest(),
        published,
        signature: [0; 64],
    };
    value.signature = r
        .nodes
        .sign_storage_metadata(local, &value.transcript())
        .map_err(|_| RemoteError::AccessDenied)?;
    Ok(value)
}
// ------------------------=
// FUNC: upload
// DESC: Stages one canonical signed authorization blob for a bounded native or remote repair phase.
// ------------------=
fn upload(j: &mut Job, kind: Upload, now: u64) -> Result<(), RemoteError> {
    j.bytes = j
        .authorization
        .ok_or(RemoteError::InvalidState)?
        .encode(now)
        .map_err(|_| RemoteError::AccessDenied)?;
    j.upload = kind;
    j.at = 0;
    j.phase = Phase::Begin;
    Ok(())
}

// ------------------------=
// FUNC: build
// DESC: Constructs a placement-only successor from fresh quorum state, explicit signed delegation and a real eligible destination resource.
// ------------------=
fn build(r: &InfinityRuntime, j: &mut Job, now: u64) -> Result<(), RemoteError> {
    let b = j.anchor.ok_or(RemoteError::InvalidState)?;
    let local = r.nodes.local_id().ok_or(RemoteError::ServiceUnavailable)?;
    let previous = j
        .round
        .as_ref()
        .ok_or(RemoteError::InvalidState)?
        .resolved()
        .map_err(|_| RemoteError::ServiceUnavailable)?;
    let grant = b
        .repair_grants
        .iter()
        .flatten()
        .find(|g| g.writer == local && g.destinations.contains(&j.destination))
        .copied()
        .ok_or(RemoteError::AccessDenied)?;
    grant
        .validate(
            &b.group,
            &b.certificate.ok_or(RemoteError::InvalidState)?,
            now,
        )
        .map_err(|_| RemoteError::AccessDenied)?;
    let mut manifest = previous.map_or(b.manifest, |a| a.manifest);
    if manifest.length > 262144
        || manifest
            .placements
            .iter()
            .flatten()
            .any(|p| p.node == j.destination)
    {
        return Err(RemoteError::Conflict);
    }
    let resource = r
        .fabric_resources
        .entries()
        .iter()
        .flatten()
        .find(|v| {
            v.owner == j.destination
                && v.kind == ResourceKind::Storage
                && v.health == Health::Healthy
                && v.online
                && v.expires > now
                && v.available.saturating_sub(v.reserved) >= manifest.length
        })
        .ok_or(RemoteError::ServiceUnavailable)?;
    storage_metadata::peer_grant(r, j.destination, now)?;
    let slot = manifest
        .placements
        .iter()
        .position(Option::is_none)
        .or_else(|| {
            manifest.placements.iter().position(|p| {
                p.is_some_and(|p| {
                    p.node != local
                        && r.nodes.discovered_nodes().iter().flatten().any(|n| {
                            n.id == p.node && n.reachability == node::types::Reachability::Offline
                        })
                })
            })
        })
        .ok_or(RemoteError::QueueFull)?;
    let sequence = previous
        .map_or(Some(1), |p| p.value.sequence.checked_add(1))
        .ok_or(RemoteError::Conflict)?;
    manifest.generation = b
        .manifest
        .generation
        .checked_add(sequence)
        .ok_or(RemoteError::Conflict)?;
    manifest.healing = None;
    manifest.placements[slot] = Some(Placement {
        node: j.destination,
        resource: resource.id,
        device: resource.device,
        generation: resource.generation,
        version: manifest.version,
        hash: manifest.hash,
        state: PlacementState::Verified,
        admission_generation: manifest.generation,
    });
    let mut bytes = [0; MANIFEST_BYTES];
    manifest
        .encode(&mut bytes)
        .map_err(|_| RemoteError::MalformedRequest)?;
    let mut value = RepairRecord {
        anchor: grant.anchor,
        grant: grant.digest(),
        sequence,
        previous: previous.map_or([0; 32], |p| p.value.digest()),
        manifest: Sha256::digest(bytes).into(),
        signature: [0; 64],
    };
    value.signature = r
        .nodes
        .sign_storage_metadata(local, &value.transcript())
        .map_err(|_| RemoteError::AccessDenied)?;
    value
        .successor(previous.map(|p| p.value))
        .map_err(|_| RemoteError::Conflict)?;
    j.authorization = Some(RepairAuthorization {
        anchor: b,
        repair: RepairBundle {
            grant,
            value,
            certificate: None,
            availability: None,
            manifest,
        },
    });
    upload(j, Upload::Destination, now)
}
// ------------------------=
// FUNC: step
// DESC: Performs at most one native operation or one network enqueue; all waiting is deferred to a future runtime tick.
// ------------------=
fn step(r: &mut InfinityRuntime, j: &mut Job, now: u64) -> Result<(), RemoteError> {
    let local = r.nodes.local_id().ok_or(RemoteError::ServiceUnavailable)?;
    match j.phase {
        Phase::Owner => {
            if let Some(b) = storage_metadata::fresh_take(r, j.user, j.session, j.fresh)? {
                if b.group.owner == local {
                    return Err(RemoteError::AccessDenied);
                }
                j.round = Some(
                    RepairReadRound::new(
                        &b.group,
                        &b.certificate.ok_or(RemoteError::InvalidState)?,
                    )
                    .map_err(|_| RemoteError::AccessDenied)?,
                );
                j.anchor = Some(b);
                j.phase = Phase::LocalHead;
            }
        }
        Phase::LocalHead => {
            let b = j.anchor.unwrap();
            let NativeReply::Overlay(value) = native(
                r,
                NativeRequest::Load {
                    object: j.object,
                    anchor: b.value.record.digest(),
                    now,
                },
                now,
            )?
            else {
                return Err(RemoteError::InvalidState);
            };
            observe(j, local, value, now)?;
            j.member = 0;
            j.phase = Phase::Head;
        }
        Phase::Head => {
            if let Some(peer) = member(j, local) {
                let mut p = packet(j, 14);
                p.length = 32;
                p.data[..32].copy_from_slice(&j.anchor.unwrap().value.record.digest());
                send(r, peer, p, now)?;
            } else {
                j.phase = Phase::Resolve;
            }
        }
        Phase::Fetch => {
            let peer = member(j, local).ok_or(RemoteError::InvalidState)?;
            let mut p = packet(j, 15);
            p.offset = j.at as u64;
            p.manifest_generation = j.head_sequence;
            p.length = 32;
            p.data[..32].copy_from_slice(&j.head_digest);
            send(r, peer, p, now)?;
        }
        Phase::Resolve => {
            let selected = j
                .round
                .as_ref()
                .unwrap()
                .selected()
                .map_err(|_| RemoteError::ServiceUnavailable)?;
            if let Some(repair) = selected {
                j.authorization = Some(RepairAuthorization {
                    anchor: j.anchor.unwrap(),
                    repair,
                });
                native(
                    r,
                    NativeRequest::Publish {
                        authorization: j.authorization.unwrap(),
                        now,
                    },
                    now,
                )?;
                let value = receipt(r, j, true)?;
                j.round
                    .as_mut()
                    .unwrap()
                    .acknowledge_writeback(&j.anchor.unwrap().group, value)
                    .map_err(|_| RemoteError::Conflict)?;
                j.member = 0;
                upload(j, Upload::Writeback, now)?;
            } else {
                j.phase = Phase::Build;
            }
        }
        Phase::Build => build(r, j, now)?,
        Phase::Begin | Phase::Chunk | Phase::End => {
            let peer = if j.upload == Upload::Destination {
                j.destination
            } else {
                member(j, local).ok_or(RemoteError::ServiceUnavailable)?
            };
            let mut p = packet(
                j,
                match j.phase {
                    Phase::Begin => 10,
                    Phase::Chunk => 11,
                    _ => {
                        if j.upload == Upload::Prepare {
                            12
                        } else if j.upload == Upload::Destination {
                            20
                        } else {
                            13
                        }
                    }
                },
            );
            if j.phase == Phase::Begin {
                p.offset = REPAIR_AUTHORIZATION_BYTES as u64;
                p.length = 32;
                p.data[..32].copy_from_slice(&Sha256::digest(j.bytes));
            } else if j.phase == Phase::Chunk {
                p.offset = j.at as u64;
                p.length = (REPAIR_AUTHORIZATION_BYTES - j.at).min(64) as u16;
                p.data[..p.length as usize]
                    .copy_from_slice(&j.bytes[j.at..j.at + p.length as usize]);
            } else if j.upload == Upload::Destination {
                p.offset = j.anchor.unwrap().manifest.length;
                p.length = 32;
                p.data[..32].copy_from_slice(&j.anchor.unwrap().manifest.hash);
            }
            send(r, peer, p, now)?;
        }
        Phase::Read => {
            let a = j.authorization.unwrap();
            let remaining = a.anchor.manifest.length - j.offset;
            if remaining == 0 {
                let hash: [u8; 32] = j.digest.clone().finalize().into();
                if hash != a.anchor.manifest.hash {
                    return Err(RemoteError::RemoteFailure);
                }
                j.phase = Phase::Commit;
            } else {
                let count = remaining.min(64) as u8;
                let NativeReply::Bytes { data, length } = native(
                    r,
                    NativeRequest::Read {
                        authorization: a,
                        writer: local,
                        offset: j.offset,
                        length: count,
                        now,
                    },
                    now,
                )?
                else {
                    return Err(RemoteError::InvalidState);
                };
                if length != count {
                    return Err(RemoteError::RemoteFailure);
                }
                j.data = data;
                j.length = length;
                j.phase = Phase::Copy;
            }
        }
        Phase::Copy => {
            let mut p = packet(j, 21);
            p.offset = j.offset;
            p.length = j.length as u16;
            p.data = j.data;
            send(r, j.destination, p, now)?;
        }
        Phase::Commit | Phase::Inspect | Phase::Attest => {
            send(
                r,
                j.destination,
                packet(
                    j,
                    if j.phase == Phase::Commit {
                        22
                    } else if j.phase == Phase::Inspect {
                        23
                    } else {
                        25
                    },
                ),
                now,
            )?;
        }
        Phase::Stage => {
            native(
                r,
                NativeRequest::Stage {
                    authorization: j.authorization.unwrap(),
                    now,
                },
                now,
            )?;
            j.receipt = Some(receipt(r, j, false)?);
            j.member = 0;
            upload(j, Upload::Prepare, now)?;
        }
        Phase::Publish => {
            let a = j.authorization.unwrap();
            native(
                r,
                NativeRequest::Publish {
                    authorization: a,
                    now,
                },
                now,
            )?;
            let value = receipt(r, j, true)?;
            let mut publication = RepairPublication::new(
                &a.anchor.group,
                &a.anchor.certificate.unwrap(),
                &a.repair.grant,
                &a.anchor.manifest,
                &a.repair.manifest,
                a.repair.certificate.unwrap(),
                now,
            )
            .map_err(|_| RemoteError::AccessDenied)?;
            publication
                .acknowledge(&a.anchor.group, value)
                .map_err(|_| RemoteError::Conflict)?;
            j.publication = Some(publication);
            j.member = 0;
            upload(j, Upload::Publish, now)?;
        }
        Phase::Done => {}
    }
    Ok(())
}
// ------------------------=
// FUNC: reply
// DESC: Validates exact correlated response semantics and advances only after durable transfer or signed quorum acknowledgements.
// ------------------=
fn reply(
    r: &InfinityRuntime,
    j: &mut Job,
    p: Pending,
    v: StorageOperationV1,
    now: u64,
) -> Result<(), RemoteError> {
    if v.operation != Operation::PoolMetadata || v.object != j.object {
        return Err(RemoteError::UnknownResponse);
    }
    match j.phase {
        Phase::Head => {
            if v.length != 32 {
                return Err(RemoteError::MalformedRequest);
            }
            if v.offset == 0 {
                if v.data[..32] != [0; 32] {
                    return Err(RemoteError::Conflict);
                }
                observe(j, p.peer, None, now)?;
                j.member += 1;
            } else {
                j.head_sequence = v.offset;
                j.head_digest.copy_from_slice(&v.data[..32]);
                j.at = 0;
                j.phase = Phase::Fetch;
            }
        }
        Phase::Fetch => {
            let n = (REPAIR_AUTHORIZATION_BYTES - j.at).min(64);
            if v.length as usize != n {
                return Err(RemoteError::MalformedRequest);
            }
            j.bytes[j.at..j.at + n].copy_from_slice(&v.data[..n]);
            j.at += n;
            if j.at == REPAIR_AUTHORIZATION_BYTES {
                let a = RepairAuthorization::decode(now, &j.bytes)
                    .map_err(|_| RemoteError::AccessDenied)?;
                if a.anchor.value != j.anchor.unwrap().value
                    || a.repair.value.sequence != j.head_sequence
                    || a.repair.value.digest() != j.head_digest
                {
                    return Err(RemoteError::Conflict);
                }
                observe(j, p.peer, Some(a.repair), now)?;
                j.member += 1;
                j.phase = Phase::Head;
            }
        }
        Phase::Begin => {
            j.at = 0;
            j.phase = Phase::Chunk;
        }
        Phase::Chunk => {
            j.at += p.request.length as usize;
            if j.at == REPAIR_AUTHORIZATION_BYTES {
                j.phase = Phase::End;
            }
        }
        Phase::End => {
            if j.upload == Upload::Destination {
                j.offset = 0;
                j.phase = Phase::Read;
                return Ok(());
            }
            let mut a = j.authorization.unwrap();
            let index = a
                .anchor
                .group
                .members
                .iter()
                .position(|n| *n == p.peer)
                .ok_or(RemoteError::AccessDenied)?;
            if v.length != 64 || v.offset != index as u64 {
                return Err(RemoteError::UnknownResponse);
            }
            let value = Receipt {
                member: index as u8,
                digest: a.repair.value.digest(),
                published: j.upload != Upload::Prepare,
                signature: v.data,
            };
            crypto::NodeCrypto::verify(
                &a.anchor.group.keys[index],
                &value.transcript(),
                &value.signature,
            )
            .map_err(|_| RemoteError::AccessDenied)?;
            match j.upload {
                Upload::Writeback => {
                    j.round
                        .as_mut()
                        .unwrap()
                        .acknowledge_writeback(&a.anchor.group, value)
                        .map_err(|_| RemoteError::Conflict)?;
                    j.round
                        .as_ref()
                        .unwrap()
                        .resolved()
                        .map_err(|_| RemoteError::ServiceUnavailable)?;
                    j.phase = Phase::Build;
                }
                Upload::Prepare => {
                    a.repair.certificate = Some(RepairCertificate {
                        value: a.repair.value,
                        prepared: [j.receipt.ok_or(RemoteError::InvalidState)?, value],
                    });
                    a.encode(now).map_err(|_| RemoteError::AccessDenied)?;
                    j.authorization = Some(a);
                    j.phase = Phase::Publish;
                }
                Upload::Publish => {
                    let publication = j.publication.as_mut().ok_or(RemoteError::InvalidState)?;
                    publication
                        .acknowledge(&a.anchor.group, value)
                        .map_err(|_| RemoteError::Conflict)?;
                    publication
                        .committed()
                        .map_err(|_| RemoteError::ServiceUnavailable)?;
                    j.result = Some(Ok(a.repair.manifest.generation));
                    j.phase = Phase::Done;
                }
                Upload::Destination => return Err(RemoteError::InvalidState),
            }
        }
        Phase::Copy => {
            let next = acknowledged_end(j.offset, j.length, v.offset)?;
            j.digest.update(&j.data[..j.length as usize]);
            j.offset = next;
            j.phase = Phase::Read;
        }
        Phase::Commit => {
            if v.value > 1 {
                return Err(RemoteError::MalformedRequest);
            }
            if v.value == 1 {
                j.phase = Phase::Inspect;
            }
        }
        Phase::Inspect => {
            let a = j.authorization.unwrap();
            let destination = a
                .repair
                .manifest
                .placements
                .iter()
                .flatten()
                .find(|p| p.node == j.destination)
                .ok_or(RemoteError::InvalidState)?;
            if v.value != 1
                || v.length != 64
                || v.offset != a.anchor.manifest.length
                || v.object_version != a.anchor.manifest.version
                || v.authority_generation != destination.generation
                || v.data[..16] != destination.resource.0
                || v.data[16..32] != destination.device
                || v.data[32..64] != a.anchor.manifest.hash
            {
                return Err(RemoteError::RemoteFailure);
            }
            j.phase = Phase::Attest;
        }
        Phase::Attest => {
            if v.length != 64 {
                return Err(RemoteError::UnknownResponse);
            }
            let mut a = j.authorization.unwrap();
            let placement = a
                .repair
                .manifest
                .placements
                .iter()
                .flatten()
                .find(|p| p.node == j.destination)
                .ok_or(RemoteError::InvalidState)?;
            let key = r
                .nodes
                .discovered_nodes()
                .iter()
                .flatten()
                .find(|p| p.id == j.destination)
                .ok_or(RemoteError::AccessDenied)?
                .public_key;
            let proof = RepairAvailability {
                repair: a.repair.value.digest(),
                node: j.destination,
                resource: placement.resource.0,
                device: placement.device,
                public_key: key,
                signature: v.data,
            };
            proof
                .validate(&a.repair.value, placement)
                .map_err(|_| RemoteError::AccessDenied)?;
            a.repair.availability = Some(proof);
            a.repair
                .validate_available(&a.anchor.manifest)
                .map_err(|_| RemoteError::AccessDenied)?;
            j.authorization = Some(a);
            j.phase = Phase::Stage;
        }
        _ => return Err(RemoteError::InvalidState),
    }
    Ok(())
}
// ------------------------=
// FUNC: acknowledged_end
// DESC: Advances transfer position only for an exact bounded contiguous receiver acknowledgement, never a future offset or wrapping count.
// ------------------=
fn acknowledged_end(offset: u64, length: u8, reported: u64) -> Result<u64, RemoteError> {
    if length == 0 || length > 64 {
        return Err(RemoteError::MalformedRequest);
    }
    let end = offset
        .checked_add(length as u64)
        .ok_or(RemoteError::MalformedRequest)?;
    if reported != end {
        return Err(RemoteError::Conflict);
    }
    Ok(end)
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: contiguous_copy_acknowledgements
    // DESC: Drives a 256KiB bounded transfer through exact recipient offsets and rejects gaps without advancing caller state.
    // ------------------=
    #[test]
    fn contiguous_copy_acknowledgements() {
        let mut offset = 0;
        for _ in 0..4096 {
            offset = acknowledged_end(offset, 64, offset + 64).unwrap();
        }
        assert_eq!(offset, 262144);
        assert_eq!(
            acknowledged_end(offset, 64, offset + 128),
            Err(RemoteError::Conflict)
        );
        assert_eq!(
            acknowledged_end(offset, 64, offset),
            Err(RemoteError::Conflict)
        );
    }
    // ------------------------=
    // FUNC: invalid_copy_window_is_rejected
    // DESC: Rejects zero, oversized and overflowing windows before any copied-state transition can occur.
    // ------------------=
    #[test]
    fn invalid_copy_window_is_rejected() {
        assert_eq!(
            acknowledged_end(0, 0, 0),
            Err(RemoteError::MalformedRequest)
        );
        assert_eq!(
            acknowledged_end(0, 65, 65),
            Err(RemoteError::MalformedRequest)
        );
        assert_eq!(
            acknowledged_end(u64::MAX, 1, 0),
            Err(RemoteError::MalformedRequest)
        );
    }
}
// ------------------------=
// FUNC: skip_unavailable
// DESC: Skips a genuinely unavailable fixed quorum member without treating it as a successful observation or vote.
// ------------------=
fn skip_unavailable(j: &mut Job, e: RemoteError) -> bool {
    if !matches!(
        e,
        RemoteError::SessionNotFound
            | RemoteError::TransportClosed
            | RemoteError::DeadlineExceeded
            | RemoteError::ServiceUnavailable
    ) {
        return false;
    }
    match j.phase {
        Phase::Head | Phase::Fetch => {
            j.member += 1;
            j.phase = Phase::Head;
            true
        }
        Phase::Begin | Phase::Chunk | Phase::End if j.upload != Upload::Destination => {
            j.member += 1;
            j.at = 0;
            j.phase = Phase::Begin;
            j.member < 3
        }
        _ => false,
    }
}
// ------------------------=
// FUNC: poll
// DESC: Executes one bounded repair transition per runtime tick, preserving session revocation, fixed queues and explicit completion errors.
// ------------------=
pub fn poll(r: &mut InfinityRuntime, now: u64) {
    if r.storage_metadata_repair.handler.is_none() {
        return;
    }
    let Some(mut j) = r.storage_metadata_repair.job.take() else {
        return;
    };
    if j.phase == Phase::Done {
        r.storage_metadata_repair.job = Some(j);
        return;
    }
    let result = if !storage_operator::authorized(r, j.user, j.session) {
        Err(RemoteError::AccessDenied)
    } else if now >= j.deadline {
        Err(RemoteError::DeadlineExceeded)
    } else if let Some(p) = r.storage_metadata_repair.pending {
        let caller = r.service_identity(SERVICE_REPLICA_STORAGE).unwrap();
        let value = r.iop.remote.take_storage_result(caller, p.id);
        if value.is_none() && now < p.deadline {
            r.storage_metadata_repair.job = Some(j);
            return;
        }
        r.iop.remote.discard(caller, p.id);
        let _ = r.capabilities.retire_leaf(p.cap, caller);
        r.storage_metadata_repair.pending = None;
        value
            .ok_or(RemoteError::DeadlineExceeded)
            .and_then(|v| v.result)
            .and_then(|v| reply(r, &mut j, p, v, now))
    } else {
        step(r, &mut j, now)
    };
    if let Err(e) = result {
        if !skip_unavailable(&mut j, e) {
            j.result = Some(Err(e));
            j.phase = Phase::Done;
            r.storage_metadata_repair.last_error = Some(e);
        }
    }
    if j.phase == Phase::Done {
        if let Some(p) = r.storage_metadata_repair.pending.take() {
            if let Some(caller) = r.service_identity(SERVICE_REPLICA_STORAGE) {
                r.iop.remote.discard(caller, p.id);
                let _ = r.capabilities.retire_leaf(p.cap, caller);
            }
        }
        r.storage_metadata_repair.completed = j.id;
    }
    r.storage_metadata_repair.job = Some(j);
}
