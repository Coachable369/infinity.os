//! One bounded typed resource observation in the existing storage IOP envelope.
//! The receiver supplies authenticated owner identity and the local lease clock.
use super::resources::*;
use crate::runtime::{node::types::NodeId, iop::storage_protocol::{Operation, StorageOperationV1}};

// ------------------------=
// FUNC: decode
// DESC: Decodes only canonical storage advertisements with bounded leases and real capacity accounting; discovery never grants authority.
// ------------------=
pub(crate) fn decode(p: StorageOperationV1, owner: NodeId, now: u64) -> Result<Resource, ResourceError> {
    if p.encode().is_err() || p.operation != Operation::ResourceAdvertise || p.length != 48
        || p.data[45] != 1 || p.data[46..48] != 1u16.to_le_bytes()
        || p.object != p.data[..16] || !(1..=60).contains(&p.value)
        || owner.0 == [0; 32] { return Err(ResourceError::Invalid); }
    let health = match p.data[44] { 1 => Health::Healthy, 2 => Health::Degraded, 3 => Health::Failed,
        _ => return Err(ResourceError::Invalid) };
    let capabilities = u32::from_le_bytes(p.data[40..44].try_into().unwrap());
    if capabilities & !1 != 0 { return Err(ResourceError::Invalid); }
    Ok(Resource { id: ResourceId(p.object), owner, device: p.data[16..32].try_into().unwrap(),
        kind: ResourceKind::Storage, capacity: p.object_version, available: p.offset,
        reserved: u64::from_le_bytes(p.data[32..40].try_into().unwrap()), health,
        online: health != Health::Failed, capabilities, generation: p.authority_generation,
        sequence: p.manifest_generation, expires: now.checked_add(p.value).ok_or(ResourceError::Invalid)? })
}
