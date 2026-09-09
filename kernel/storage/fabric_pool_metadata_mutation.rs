//! Bounded durable write-ahead state for shared mutations.
use super::*;
use crate::runtime::{
    fabric::{
        manifest::{Manifest, PlacementState, MANIFEST_BYTES},
        placement::{Availability, StorageClass},
    },
    iop::storage_protocol::{Operation, OPERATION_BYTES},
};
const ANCHOR: usize = 64;
const REQUEST: usize = ANCHOR + BUNDLE_BYTES;
const RESULT: usize = REQUEST + OPERATION_BYTES;
const INTENT_BYTES: usize = RESULT + MANIFEST_BYTES;
use crate::runtime::fabric::metadata_repair::{
    RepairAuthorization, RepairBundle, REPAIR_AUTHORIZATION_BYTES,
};
// ------------------------=
// FUNC: repair_projection
// DESC: Loads the exact durable certified placement head selected by a fresh quorum; unrelated, stale, or substituted repair authority is rejected.
// ------------------=
fn repair_projection<D: BlockDevice>(
    s: &mut ObjectStore<D>,
    anchor: Bundle,
    digest: [u8; 32],
) -> Result<Option<RepairAuthorization>, RemoteError> {
    let mut bytes = [0; REPAIR_AUTHORIZATION_BYTES];
    let n = backing::read_repair_payload(s, anchor.manifest.object, false, &mut bytes)
        .map_err(error)?;
    if n == 0 {
        return if digest == [0; 32] {
            Ok(None)
        } else {
            Err(RemoteError::Conflict)
        };
    }
    let a = RepairAuthorization::decode_committed(&bytes).map_err(|_| RemoteError::AccessDenied)?;
    if a.anchor.value != anchor.value {
        return if digest == [0; 32]
            && a.anchor.value.record.generation < anchor.value.record.generation
        {
            Ok(None)
        } else {
            Err(RemoteError::Conflict)
        };
    }
    if a.repair.certificate.is_none() || a.repair.value.digest() != digest {
        return Err(RemoteError::Conflict);
    }
    Ok(Some(a))
}
#[derive(Clone, Copy)]
struct Intent {
    anchor: Bundle,
    request: StorageOperationV1,
    result: Option<Manifest>,
    applied: bool,
    finalized: bool,
    overlay: [u8; 32],
}
impl Intent {
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes the exact admission anchor, request and optional applied result.
    // ------------------=
    fn encode(self) -> Result<[u8; INTENT_BYTES], RemoteError> {
        let mut b = [0; INTENT_BYTES];
        b[..8].copy_from_slice(b"INFPMUT1");
        b[8] = if self.finalized {
            3
        } else if self.applied {
            1
        } else if self.result.is_some() {
            2
        } else {
            0
        };
        b[16..48].copy_from_slice(&self.overlay);
        b[ANCHOR..REQUEST].copy_from_slice(&self.anchor.encode().map_err(error)?);
        b[REQUEST..RESULT].copy_from_slice(
            &self
                .request
                .encode()
                .map_err(|_| RemoteError::MalformedRequest)?,
        );
        if let Some(m) = self.result {
            let mut raw = [0; MANIFEST_BYTES];
            m.encode(&mut raw)
                .map_err(|_| RemoteError::PersistenceFailed)?;
            b[RESULT..].copy_from_slice(&raw);
        }
        Ok(b)
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Rejects corrupt pending state instead of replaying an invented mutation.
    // ------------------=
    fn decode(b: &[u8]) -> Result<Self, RemoteError> {
        if b.len() != INTENT_BYTES
            || &b[..8] != b"INFPMUT1"
            || b[8] > 3
            || b[9..16].iter().chain(b[48..ANCHOR].iter()).any(|v| *v != 0)
        {
            return Err(RemoteError::PersistenceFailed);
        }
        let i = Self {
            anchor: Bundle::decode(&b[ANCHOR..REQUEST]).map_err(error)?,
            request: StorageOperationV1::decode(&b[REQUEST..RESULT])
                .map_err(|_| RemoteError::PersistenceFailed)?,
            result: if b[8] != 0 {
                Some(Manifest::decode(&b[RESULT..]).map_err(|_| RemoteError::PersistenceFailed)?)
            } else {
                None
            },
            applied: b[8] == 1 || b[8] == 3,
            finalized: b[8] == 3,
            overlay: b[16..48].try_into().unwrap(),
        };
        if i.encode()?.as_slice() != b {
            return Err(RemoteError::PersistenceFailed);
        }
        Ok(i)
    }
}
// ------------------------=
// FUNC: load
// DESC: Reads the single linked intent without consuming another namespace entry.
// ------------------=
fn load<D: BlockDevice>(
    s: &mut ObjectStore<D>,
    object: [u8; 16],
) -> Result<Option<Intent>, RemoteError> {
    let mut b = [0; INTENT_BYTES];
    let n = backing::read_mutation_payload(s, object, &mut b).map_err(error)?;
    if n == 0 {
        Ok(None)
    } else {
        Intent::decode(&b[..n]).map(Some)
    }
}
// ------------------------=
// FUNC: same_request
// DESC: Compares canonical typed requests including authority scope and exact payload.
// ------------------=
fn same_request(a: StorageOperationV1, b: StorageOperationV1) -> bool {
    a.encode()
        .ok()
        .zip(b.encode().ok())
        .is_some_and(|(a, b)| a == b)
}
// ------------------------=
// FUNC: native_error
// DESC: Preserves native authority and generation failures as typed errors.
// ------------------=
fn native_error(e: ObjectError) -> RemoteError {
    match e {
        ObjectError::Unauthorized => RemoteError::AccessDenied,
        ObjectError::InvalidVersion => RemoteError::Conflict,
        ObjectError::InsufficientCapacity => RemoteError::QueueFull,
        _ => RemoteError::PersistenceFailed,
    }
}
// ------------------------=
// FUNC: expected
// DESC: Calculates the exact successor before modifying content or durable policy.
// ------------------=
fn expected(base: Manifest, p: StorageOperationV1) -> Result<Manifest, RemoteError> {
    if p.object != base.object
        || p.manifest_generation != base.generation
        || p.object_version != base.version
        || p.authority_generation != base.authority_generation
        || p.offset != 0
        || base.healing.is_some()
    {
        return Err(RemoteError::Conflict);
    }
    p.encode().map_err(|_| RemoteError::MalformedRequest)?;
    let mut m = base;
    m.generation = m.generation.checked_add(1).ok_or(RemoteError::Conflict)?;
    match p.operation {
        Operation::ObjectSetPolicy => {
            if p.length != 0 {
                return Err(RemoteError::MalformedRequest);
            }
            m.policy = match p.value {
                1 => StorageClass::Temporary,
                2 => StorageClass::Protected,
                3 => StorageClass::Critical,
                _ => return Err(RemoteError::MalformedRequest),
            };
            if m.policy == base.policy {
                return Ok(base);
            }
            m.minimum_available = m.minimum_available.min(m.policy.replicas() as u8);
        }
        Operation::ObjectDelete => {
            if p.length != 0 || p.value != 0 {
                return Err(RemoteError::MalformedRequest);
            }
        }
        Operation::ObjectUpdate => {
            if p.value != 0 {
                return Err(RemoteError::MalformedRequest);
            }
            m.version = m.version.checked_add(1).ok_or(RemoteError::Conflict)?;
            m.length = p.length as u64;
            m.hash = Sha256::digest(&p.data[..p.length as usize]).into();
            m.chunks.fill(None);
            if p.length > 0 {
                m.chunks[0] = Some(crate::runtime::fabric::manifest::Chunk {
                    content: m.hash[..16].try_into().unwrap(),
                    bytes: p.length as u32,
                    hash: m.hash,
                });
            }
            let at = base
                .placements
                .iter()
                .position(|v| {
                    v.is_some_and(|v| {
                        v.node == base.authority && v.hash == base.hash && v.version == base.version
                    })
                })
                .ok_or(RemoteError::InvalidState)?;
            for v in m.placements.iter_mut().flatten() {
                v.state = PlacementState::Stale;
            }
            let v = m.placements[at].as_mut().unwrap();
            v.hash = m.hash;
            v.version = m.version;
            v.state = PlacementState::Verified;
            v.admission_generation = m.generation;
        }
        _ => return Err(RemoteError::UnsupportedOperation),
    }
    Ok(m)
}
// ------------------------=
// FUNC: response
// DESC: Produces identical structured results for initial application and durable retries.
// ------------------=
fn response(mut p: StorageOperationV1, m: Manifest) -> NativeReply {
    p.data.fill(0);
    p.data[..16].copy_from_slice(&m.object);
    p.data[16..48].copy_from_slice(&m.hash);
    p.data[48] = m.policy.replicas() as u8;
    p.data[49] = match m.availability() {
        Availability::Healthy => 1,
        Availability::Degraded => 2,
        Availability::Offline => 3,
    };
    p.length = 50;
    p.value = m.length;
    p.manifest_generation = m.generation;
    p.object_version = m.version;
    p.authority_generation = m.authority_generation;
    NativeReply::Mutation {
        manifest: m,
        response: p,
    }
}
// ------------------------=
// FUNC: validate_placement
// DESC: Pins a placement-only successor to its complete manifest digest without allowing content or policy changes.
// ------------------=
fn validate_placement(
    base: Manifest,
    p: StorageOperationV1,
    next: Manifest,
) -> Result<(), RemoteError> {
    let mut raw = [0; MANIFEST_BYTES];
    next.encode(&mut raw)
        .map_err(|_| RemoteError::MalformedRequest)?;
    if p.operation != Operation::PoolHeal
        || p.object != base.object
        || p.object_version != base.version
        || p.authority_generation != base.authority_generation
        || p.manifest_generation != base.generation
        || p.offset != 0
        || p.value != 0
        || p.length != 32
        || p.data[..32] != <[u8; 32]>::from(Sha256::digest(raw))
        || next.object != base.object
        || next.version != base.version
        || next.length != base.length
        || next.hash != base.hash
        || next.chunks != base.chunks
        || next.policy != base.policy
        || next.minimum_available != base.minimum_available
        || next.authority != base.authority
        || next.authority_generation != base.authority_generation
    {
        return Err(RemoteError::Conflict);
    }
    p.encode().map_err(|_| RemoteError::MalformedRequest)?;
    base.successor(&next).map_err(|_| RemoteError::Conflict)
}
// ------------------------=
// FUNC: placement
// DESC: Records the complete placement proposal before owner-side CAS and uses the same replay and quorum-finalization journal as content mutations.
// ------------------=
pub(super) fn placement<D: BlockDevice>(
    s: &mut ObjectStore<D>,
    anchor: Bundle,
    expected_generation: u64,
    next: Manifest,
) -> Result<NativeReply, RemoteError> {
    placement_overlay(s, anchor, expected_generation, next, None)
}
// ------------------------=
// FUNC: placement_overlay
// DESC: Records an exact proposal and the freshly selected repair head before any certified local projection restoration.
// ------------------=
pub(super) fn placement_overlay<D: BlockDevice>(
    s: &mut ObjectStore<D>,
    anchor: Bundle,
    expected_generation: u64,
    next: Manifest,
    overlay: Option<RepairBundle>,
) -> Result<NativeReply, RemoteError> {
    anchor.validate().map_err(error)?;
    if anchor.certificate.is_none() || anchor.value.record.deleted {
        return Err(RemoteError::AccessDenied);
    }
    let mut raw = [0; MANIFEST_BYTES];
    next.encode(&mut raw)
        .map_err(|_| RemoteError::MalformedRequest)?;
    let mut data = [0; 64];
    data[..32].copy_from_slice(&Sha256::digest(raw));
    let p = StorageOperationV1 {
        operation: Operation::PoolHeal,
        object: next.object,
        authority_generation: next.authority_generation,
        manifest_generation: expected_generation,
        object_version: next.version,
        offset: 0,
        scope: 0,
        value: 0,
        length: 32,
        data,
    };
    let digest = overlay.map_or([0; 32], |o| o.value.digest());
    let proof = repair_projection(s, anchor, digest)?;
    let base = proof.map_or(anchor.manifest, |a| a.repair.manifest);
    if !(proof.is_some() && next == base && expected_generation == anchor.manifest.generation) {
        validate_placement(base, p, next)?;
    }
    if let Some(i) = load(s, p.object)?
        .filter(|i| !i.finalized || (i.anchor.value == anchor.value && same_request(i.request, p)))
    {
        if i.anchor.value != anchor.value || !same_request(i.request, p) || i.result != Some(next) {
            return Err(RemoteError::Conflict);
        }
    } else {
        if backing::read_bundle(s, p.object).map_err(error)?.value != anchor.value
            || s.pool_manifest(ObjectId(p.object), anchor.group.owner, 0)
                .map_err(native_error)?
                != anchor.manifest
        {
            return Err(RemoteError::Conflict);
        }
        backing::write_mutation_payload(
            s,
            p.object,
            &Intent {
                anchor,
                request: p,
                result: Some(next),
                applied: false,
                finalized: false,
                overlay: digest,
            }
            .encode()?,
        )
        .map_err(error)?;
    }
    mutate_overlay(s, anchor, p, overlay)
}
// ------------------------=
// FUNC: mutate
// DESC: Persists admission first, recovers interrupted application and retains the result until quorum finalization.
// ------------------=
pub(super) fn mutate<D: BlockDevice>(
    s: &mut ObjectStore<D>,
    anchor: Bundle,
    p: StorageOperationV1,
) -> Result<NativeReply, RemoteError> {
    mutate_overlay(s, anchor, p, None)
}
// ------------------------=
// FUNC: mutate_overlay
// DESC: Replays a durable exact mutation with its certified effective placement base, preserving restored destinations through content and policy changes.
// ------------------=
pub(super) fn mutate_overlay<D: BlockDevice>(
    s: &mut ObjectStore<D>,
    anchor: Bundle,
    p: StorageOperationV1,
    overlay: Option<RepairBundle>,
) -> Result<NativeReply, RemoteError> {
    anchor.validate().map_err(error)?;
    if anchor.certificate.is_none() || anchor.value.record.deleted {
        return Err(RemoteError::AccessDenied);
    }
    let retained = load(s, p.object)?
        .filter(|i| !i.finalized || (i.anchor.value == anchor.value && same_request(i.request, p)));
    if let Some(i) = retained {
        if i.anchor.value != anchor.value || !same_request(i.request, p) {
            return Err(RemoteError::Conflict);
        }
        if i.applied {
            return Ok(response(p, i.result.ok_or(RemoteError::PersistenceFailed)?));
        }
    }
    let digest = retained.map_or_else(
        || overlay.map_or([0; 32], |o| o.value.digest()),
        |i| i.overlay,
    );
    if overlay.is_some_and(|o| o.value.digest() != digest) {
        return Err(RemoteError::Conflict);
    }
    let proof = repair_projection(s, anchor, digest)?;
    let base = proof.map_or(anchor.manifest, |a| a.repair.manifest);
    if p.manifest_generation != anchor.manifest.generation
        && p.manifest_generation != base.generation
    {
        return Err(RemoteError::Conflict);
    }
    let mut effective_request = p;
    effective_request.manifest_generation = base.generation;
    let next = if p.operation == Operation::PoolHeal {
        let i = retained.ok_or(RemoteError::AccessDenied)?;
        let proposed = i.result.ok_or(RemoteError::PersistenceFailed)?;
        if !(proof.is_some()
            && proposed == base
            && p.manifest_generation == anchor.manifest.generation)
        {
            validate_placement(base, p, proposed)?;
        }
        proposed
    } else {
        expected(base, effective_request)?
    };
    let mut intent = match retained {
        Some(i) => {
            if i.anchor.value != anchor.value || !same_request(i.request, p) {
                return Err(RemoteError::Conflict);
            }
            i
        }
        None => {
            let actual = backing::read_bundle(s, p.object).map_err(error)?;
            if actual.value != anchor.value {
                return Err(RemoteError::Conflict);
            }
            let current = s
                .pool_manifest(ObjectId(p.object), anchor.group.owner, p.scope)
                .map_err(native_error)?;
            if current != anchor.manifest && current != base {
                return Err(RemoteError::Conflict);
            }
            let i = Intent {
                anchor,
                request: p,
                result: None,
                applied: false,
                finalized: false,
                overlay: digest,
            };
            backing::write_mutation_payload(s, p.object, &i.encode()?).map_err(error)?;
            i
        }
    };
    if intent.applied {
        let m = intent.result.ok_or(RemoteError::PersistenceFailed)?;
        if m != next {
            return Err(RemoteError::Conflict);
        }
        return Ok(response(p, m));
    }
    let applied = match s.pool_manifest(ObjectId(p.object), anchor.group.owner, p.scope) {
        Ok(m) if m == next && p.operation != Operation::ObjectDelete => true,
        Err(ObjectError::NotFound) if p.operation == Operation::ObjectDelete => true,
        Ok(m) if m == anchor.manifest || m == base => false,
        _ => return Err(RemoteError::Conflict),
    };
    if !applied {
        if let Some(proof) = proof {
            s.with_pool_mutation_authority(ObjectId(p.object), |s| {
                s.pool_reconcile_certified_manifest(
                    ObjectId(p.object),
                    anchor.group.owner,
                    p.scope,
                    &proof,
                )
            })
            .map_err(native_error)?;
        }
        if next == base && p.operation == Operation::PoolHeal {
            intent.result = Some(next);
            intent.applied = true;
            backing::write_mutation_payload(s, p.object, &intent.encode()?).map_err(error)?;
            return Ok(response(p, next));
        }
        s.with_pool_mutation_authority(ObjectId(p.object), |s| match p.operation {
            Operation::PoolHeal => s.pool_commit_manifest(
                ObjectId(p.object),
                anchor.group.owner,
                p.scope,
                effective_request.manifest_generation,
                &next,
            ),
            Operation::ObjectDelete => s.pool_delete(
                ObjectId(p.object),
                anchor.group.owner,
                p.scope,
                effective_request.manifest_generation,
            ),
            Operation::ObjectSetPolicy => s
                .pool_set_policy(
                    ObjectId(p.object),
                    anchor.group.owner,
                    p.scope,
                    effective_request.manifest_generation,
                    next.policy,
                )
                .and_then(|m| {
                    if m == next {
                        Ok(())
                    } else {
                        Err(ObjectError::InvalidVersion)
                    }
                }),
            Operation::ObjectUpdate => {
                let local = base
                    .placements
                    .iter()
                    .flatten()
                    .find(|v| v.node == anchor.group.owner)
                    .ok_or(ObjectError::InvalidVersion)?;
                s.pool_update(
                    ObjectId(p.object),
                    anchor.group.owner,
                    p.scope,
                    effective_request.manifest_generation,
                    &p.data[..p.length as usize],
                    local.node,
                    local.resource,
                    local.device,
                    local.generation,
                )
                .and_then(|m| {
                    if m == next {
                        Ok(())
                    } else {
                        Err(ObjectError::InvalidVersion)
                    }
                })
            }
            _ => Err(ObjectError::InvalidObject),
        })
        .map_err(native_error)?;
    }
    intent.result = Some(next);
    intent.applied = true;
    backing::write_mutation_payload(s, p.object, &intent.encode()?).map_err(error)?;
    Ok(response(p, next))
}
// ------------------------=
// FUNC: pending
// DESC: Enumerates only durable intents attached to existing committed metadata for bounded service recovery.
// ------------------=
pub(super) fn pending<D: BlockDevice>(
    s: &mut ObjectStore<D>,
    index: usize,
) -> Result<NativeReply, RemoteError> {
    let Some(b) = backing::load_bundle(s, index).map_err(error)? else {
        return Ok(NativeReply::Pending(None));
    };
    Ok(NativeReply::Pending(
        load(s, b.manifest.object)?
            .filter(|i| !i.finalized)
            .map(|i| crate::runtime::storage_metadata::MutationIntent {
                anchor: i.anchor,
                request: i.request,
            }),
    ))
}
// ------------------------=
// FUNC: finalize
// DESC: Retires an applied intent only for its exact certified successor after the caller's publication-quorum barrier.
// ------------------=
pub(super) fn finalize<D: BlockDevice>(
    s: &mut ObjectStore<D>,
    object: [u8; 16],
    request: StorageOperationV1,
    record: crate::runtime::fabric::metadata::Record,
) -> Result<NativeReply, RemoteError> {
    let b = backing::read_bundle(s, object).map_err(error)?;
    if b.certificate.is_none() || b.value.record != record || request.object != object {
        return Err(RemoteError::Conflict);
    }
    let Some(mut i) = load(s, object)? else {
        return Err(RemoteError::Conflict);
    };
    if !same_request(i.request, request)
        || !i.applied
        || i.result != Some(b.manifest)
        || record.previous != i.anchor.value.record.digest()
        || record.generation
            != i.anchor
                .value
                .record
                .generation
                .checked_add(1)
                .ok_or(RemoteError::Conflict)?
        || record.deleted != (request.operation == Operation::ObjectDelete)
    {
        return Err(RemoteError::Conflict);
    }
    if !i.finalized {
        i.finalized = true;
        backing::write_mutation_payload(s, object, &i.encode()?).map_err(error)?;
    }
    Ok(NativeReply::Done)
}
