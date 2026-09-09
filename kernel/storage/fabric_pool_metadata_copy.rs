//! Fresh shared-source copy admission; immutable source identity is never mutated.
use super::*;
use crate::runtime::{
    fabric::{
        manifest::PlacementState,
        metadata_repair::{RepairAuthorization, RepairBundle, REPAIR_AUTHORIZATION_BYTES},
        placement::Availability,
    },
    iop::storage_protocol::Operation,
};
// ------------------------=
// FUNC: copy
// DESC: Copies only a fresh certified immutable source, preserving atomic nonce retries and granting no authority to alter its metadata chain.
// ------------------=
pub(super) fn copy<D: BlockDevice>(
    s: &mut ObjectStore<D>,
    anchor: Bundle,
    p: StorageOperationV1,
    overlay: Option<RepairBundle>,
) -> Result<NativeReply, RemoteError> {
    anchor.validate().map_err(error)?;
    if anchor.certificate.is_none()
        || anchor.value.record.deleted
        || p.operation != Operation::ObjectCopy
        || p.object != anchor.manifest.object
        || p.length != 0
        || p.offset != 0
        || p.value == 0
    {
        return Err(RemoteError::AccessDenied);
    }
    p.encode().map_err(|_| RemoteError::MalformedRequest)?;
    if backing::read_bundle(s, p.object).map_err(error)?.value != anchor.value {
        return Err(RemoteError::Conflict);
    }
    let mut raw = [0; REPAIR_AUTHORIZATION_BYTES];
    let count = backing::read_repair_payload(s, p.object, false, &mut raw).map_err(error)?;
    let effective = if let Some(o) = overlay {
        if count != raw.len() {
            return Err(RemoteError::Conflict);
        }
        let durable = RepairAuthorization::decode_committed(&raw).map_err(error)?;
        if durable.anchor.value != anchor.value || durable.repair != o {
            return Err(RemoteError::Conflict);
        }
        o.manifest
    } else {
        if count != 0 {
            return Err(RemoteError::Conflict);
        }
        anchor.manifest
    };
    if p.object_version != effective.version
        || p.authority_generation != effective.authority_generation
        || !(p.manifest_generation == effective.generation
            || p.manifest_generation == anchor.manifest.generation)
    {
        return Err(RemoteError::Conflict);
    }
    let current = s
        .pool_manifest(ObjectId(p.object), anchor.group.owner, p.scope)
        .map_err(|_| RemoteError::NotFound)?;
    if current.object != effective.object
        || current.version != effective.version
        || current.hash != effective.hash
        || current.length != effective.length
        || current.chunks != effective.chunks
        || current.policy != effective.policy
        || current.minimum_available != effective.minimum_available
        || current.authority != effective.authority
        || current.authority_generation != effective.authority_generation
    {
        return Err(RemoteError::Conflict);
    }
    let local = current
        .placements
        .iter()
        .flatten()
        .find(|v| {
            v.node == anchor.group.owner
                && v.version == current.version
                && v.hash == current.hash
                && !matches!(v.state, PlacementState::Corrupt | PlacementState::Staging)
        })
        .ok_or(RemoteError::NotFound)?;
    // pool_copy hashes the complete immutable source before creating a new identity.
    let m = s
        .with_pool_mutation_authority(ObjectId(p.object), |s| {
            s.pool_copy(
                ObjectId(p.object),
                anchor.group.owner,
                p.scope,
                current.generation,
                p.value,
                local.node,
                local.resource,
                local.device,
                local.generation,
            )
        })
        .map_err(|e| match e {
            ObjectError::InvalidVersion => RemoteError::Conflict,
            ObjectError::InsufficientCapacity => RemoteError::QueueFull,
            _ => RemoteError::PersistenceFailed,
        })?;
    let mut out = p;
    out.data.fill(0);
    out.data[..16].copy_from_slice(&m.object);
    out.data[16..48].copy_from_slice(&m.hash);
    out.data[48] = m.policy.replicas() as u8;
    out.data[49] = match m.availability() {
        Availability::Healthy => 1,
        Availability::Degraded => 2,
        Availability::Offline => 3,
    };
    out.length = 50;
    out.manifest_generation = m.generation;
    out.object_version = m.version;
    out.authority_generation = m.authority_generation;
    out.value = m.length;
    Ok(NativeReply::Mutation {
        manifest: m,
        response: out,
    })
}
