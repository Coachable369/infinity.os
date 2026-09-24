//! One bounded typed resource observation in the existing Resource.Advertise IOP envelope.
//! The receiver supplies authenticated owner identity and the local lease clock.
use super::resources::*;
use crate::runtime::{node::types::NodeId, iop::storage_protocol::{Operation, StorageOperationV1}};

// ------------------------=
// FUNC: decode
// DESC: Decodes canonical storage, compute, memory, and accelerator advertisements with bounded leases and real capacity accounting.
// ------------------=
pub(crate) fn decode(p: StorageOperationV1, owner: NodeId, now: u64) -> Result<Resource, ResourceError> {
    if p.encode().is_err() || p.operation != Operation::ResourceAdvertise || p.length != 48
        || p.data[46..48] != 1u16.to_le_bytes()
        || p.object != p.data[..16] || !(1..=60).contains(&p.value)
        || owner.0 == [0; 32] { return Err(ResourceError::Invalid); }
    let health = match p.data[44] { 1 => Health::Healthy, 2 => Health::Degraded, 3 => Health::Failed,
        _ => return Err(ResourceError::Invalid) };
    let capabilities = u32::from_le_bytes(p.data[40..44].try_into().unwrap());
    if capabilities & !1 != 0 { return Err(ResourceError::Invalid); }
    let kind = match p.data[45] { 1 => ResourceKind::Storage, 2 => ResourceKind::Compute, 3 => ResourceKind::Memory, 4 => ResourceKind::Accelerator, _ => return Err(ResourceError::Invalid) };
    Ok(Resource { id: ResourceId(p.object), owner, device: p.data[16..32].try_into().unwrap(),
        kind, capacity: p.object_version, available: p.offset,
        reserved: u64::from_le_bytes(p.data[32..40].try_into().unwrap()), health,
        online: health != Health::Failed, capabilities, generation: p.authority_generation,
        sequence: p.manifest_generation, expires: now.checked_add(p.value).ok_or(ResourceError::Invalid)? })
}

// ------------------------=
// FUNC: encode
// DESC: Encodes an actually observed resource into the shared bounded advertisement schema and lease.
// ------------------=
pub(crate) fn encode(resource: Resource, lease: u64) -> Result<StorageOperationV1, ResourceError> {
    if resource.id.0 == [0; 16] || resource.device == [0; 16] || resource.capacity == 0 || resource.available > resource.capacity || resource.reserved > resource.capacity.saturating_sub(resource.available) || resource.generation == 0 || resource.sequence == 0 || !(1..=60).contains(&lease) { return Err(ResourceError::Invalid); }
    let mut data = [0; 64]; data[..16].copy_from_slice(&resource.id.0); data[16..32].copy_from_slice(&resource.device); data[32..40].copy_from_slice(&resource.reserved.to_le_bytes()); data[40..44].copy_from_slice(&resource.capabilities.to_le_bytes()); data[44] = match resource.health { Health::Healthy => 1, Health::Degraded => 2, Health::Failed => 3 }; data[45] = match resource.kind { ResourceKind::Storage => 1, ResourceKind::Compute => 2, ResourceKind::Memory => 3, ResourceKind::Accelerator => 4 }; data[46..48].copy_from_slice(&1u16.to_le_bytes());
    Ok(StorageOperationV1 { operation: Operation::ResourceAdvertise, object: resource.id.0, authority_generation: resource.generation, manifest_generation: resource.sequence, object_version: resource.capacity, offset: resource.available, scope: 0, value: lease, length: 48, data })
}
