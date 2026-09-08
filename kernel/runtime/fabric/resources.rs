use super::super::node::{NodeRuntime, types::{NodeId, SessionState}};

pub const MAX_RESOURCES: usize = 32;
pub const ADVERTISE_OPERATION: u32 = super::super::iop::OperationId::ResourceAdvertise as u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResourceId(pub [u8; 16]);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceKind { Storage, Compute, Memory, Accelerator }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Health { Healthy, Degraded, Failed }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resource {
    pub id: ResourceId,
    pub owner: NodeId,
    pub kind: ResourceKind,
    pub device: [u8; 16],
    pub capacity: u64,
    pub available: u64,
    pub health: Health,
    pub online: bool,
    pub capabilities: u32,
    pub generation: u64,
    pub sequence: u64,
    pub expires: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceError { AccessDenied, Invalid, Stale, Conflict, Full, NotFound, Capacity }

#[derive(Clone)]
pub struct Directory { entries: [Option<Resource>; MAX_RESOURCES], reserved: [u64; MAX_RESOURCES] }
impl Directory {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates a bounded inventory with no implicit resources or authority.
    // ------------------=
    pub const fn new() -> Self { Self { entries: [None; MAX_RESOURCES], reserved: [0; MAX_RESOURCES] } }

    // ------------------------=
    // FUNC: advertise
    // DESC: Admits an authenticated peer resource only under a current exact advertise grant and established native session.
    // ------------------=
    pub fn advertise(&mut self, nodes: &NodeRuntime, session: u64, grant: u64, scope: u64, resource: Resource, now: u64) -> Result<bool, ResourceError> {
        let peer = nodes.sessions().iter().flatten().find(|s| s.id == session
            && s.state == SessionState::Established && now < s.expires_at)
            .ok_or(ResourceError::AccessDenied)?.peer;
        if peer != resource.owner { return Err(ResourceError::AccessDenied); }
        nodes.authorize_remote(grant, peer, ADVERTISE_OPERATION, scope, 1, now)
            .map_err(|_| ResourceError::AccessDenied)?;
        self.apply(resource, now)
    }

    // ------------------------=
    // FUNC: apply
    // DESC: Reconciles validated inventory without allowing stale generations, identity reassignment or duplicate allocation.
    // ------------------=
    pub(super) fn apply(&mut self, resource: Resource, now: u64) -> Result<bool, ResourceError> {
        if resource.id.0 == [0; 16] || resource.owner.0 == [0; 32] || resource.device == [0; 16]
            || resource.capacity == 0 || resource.available > resource.capacity
            || resource.generation == 0 || resource.sequence == 0 || resource.expires <= now {
            return Err(ResourceError::Invalid);
        }
        let slot = if let Some(index) = self.entries.iter().position(|r| r.map(|r| r.id) == Some(resource.id)) {
            let previous = self.entries[index].unwrap();
            if previous.owner != resource.owner || previous.device != resource.device || previous.kind != resource.kind {
                return Err(ResourceError::Conflict);
            }
            if (resource.generation, resource.sequence) < (previous.generation, previous.sequence) { return Err(ResourceError::Stale); }
            if (resource.generation, resource.sequence) == (previous.generation, previous.sequence) {
                return if previous == resource { Ok(false) } else { Err(ResourceError::Conflict) };
            }
            // Outstanding transfers must be reconciled before reusing a replaced device.
            if resource.generation != previous.generation && self.reserved[index] != 0 { return Err(ResourceError::Conflict); }
            index
        } else {
            self.entries.iter().position(Option::is_none).ok_or(ResourceError::Full)?
        };
        self.entries[slot] = Some(resource);
        Ok(true)
    }

    // ------------------------=
    // FUNC: entries
    // DESC: Exposes immutable typed inventory; expired advertisements remain known but cannot be placement candidates.
    // ------------------=
    pub fn entries(&self) -> &[Option<Resource>; MAX_RESOURCES] { &self.entries }

    // ------------------------=
    // FUNC: restore_reservations
    // DESC: Reconstructs only durable generation-bound claims after reboot; offline capacity remains reserved and invalid recovery cannot partially replace accounting.
    // ------------------=
    pub(super) fn restore_reservations(&mut self, claims: impl Iterator<Item = (ResourceId, u64, u64)>) -> Result<(), ResourceError> {
        let mut restored = [0u64; MAX_RESOURCES];
        for (id, generation, bytes) in claims {
            if bytes == 0 { return Err(ResourceError::Invalid); }
            let index = self.entries.iter().position(|r| r.is_some_and(|r|
                r.id == id && r.generation == generation)).ok_or(ResourceError::Stale)?;
            restored[index] = restored[index].checked_add(bytes).ok_or(ResourceError::Capacity)?;
            if restored[index] > self.entries[index].unwrap().capacity { return Err(ResourceError::Capacity); }
        }
        self.reserved = restored;
        Ok(())
    }

    // ------------------------=
    // FUNC: usable
    // DESC: Computes unreserved capacity only for currently healthy, writable storage advertisements.
    // ------------------=
    pub fn usable(&self, index: usize, now: u64) -> u64 {
        self.entries.get(index).copied().flatten().filter(|r| r.online && r.expires > now
            && r.health == Health::Healthy && r.kind == ResourceKind::Storage && r.capabilities & 1 != 0)
            .map(|r| r.available.saturating_sub(self.reserved[index])).unwrap_or(0)
    }

    // ------------------------=
    // FUNC: reserve
    // DESC: Reserves bounded transfer capacity so concurrent planned replicas cannot spend the same advertised bytes.
    // ------------------=
    pub(super) fn reserve(&mut self, id: ResourceId, generation: u64, bytes: u64, now: u64) -> Result<(), ResourceError> {
        let index = self.entries.iter().position(|r| r.map(|r| (r.id, r.generation)) == Some((id, generation))).ok_or(ResourceError::NotFound)?;
        if bytes == 0 || self.usable(index, now) < bytes { return Err(ResourceError::Capacity); }
        self.reserved[index] = self.reserved[index].checked_add(bytes).ok_or(ResourceError::Capacity)?;
        Ok(())
    }

    // ------------------------=
    // FUNC: release
    // DESC: Releases one transfer reservation only against its original device generation.
    // ------------------=
    pub(super) fn release(&mut self, id: ResourceId, generation: u64, bytes: u64) -> Result<(), ResourceError> {
        let index = self.entries.iter().position(|r| r.map(|r| (r.id, r.generation)) == Some((id, generation))).ok_or(ResourceError::NotFound)?;
        self.reserved[index] = self.reserved[index].checked_sub(bytes).ok_or(ResourceError::Conflict)?;
        Ok(())
    }
}
