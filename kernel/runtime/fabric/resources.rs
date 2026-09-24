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
    pub reserved: u64,
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
pub struct Directory { entries: [Option<Resource>; MAX_RESOURCES], reserved: [u64; MAX_RESOURCES], offline_notices: u32 }
impl Directory {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates a bounded inventory with no implicit resources or authority.
    // ------------------=
    pub const fn new() -> Self { Self { entries: [None; MAX_RESOURCES], reserved: [0; MAX_RESOURCES], offline_notices: 0 } }

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
            || resource.reserved > resource.capacity.saturating_sub(resource.available)
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
                // An identical observation retried later is idempotent, but
                // cannot renew its lease or resurrect an offline resource.
                let mut repeated = resource; repeated.expires = previous.expires;
                return if previous == repeated { Ok(false) } else { Err(ResourceError::Conflict) };
            }
            // Outstanding transfers must be reconciled before reusing a replaced device.
            if resource.generation != previous.generation && self.reserved[index] != 0 { return Err(ResourceError::Conflict); }
            index
        } else {
            self.entries.iter().position(Option::is_none).ok_or(ResourceError::Full)?
        };
        self.entries[slot] = Some(resource);
        // A newer authenticated online observation supersedes a queued loss.
        if resource.online { self.offline_notices &= !(1u32 << slot); }
        Ok(true)
    }

    // ------------------------=
    // FUNC: entries
    // DESC: Exposes immutable typed inventory; expired advertisements remain known but cannot be placement candidates.
    // ------------------=
    pub fn entries(&self) -> &[Option<Resource>; MAX_RESOURCES] { &self.entries }

    // ------------------------=
    // FUNC: retire_local_compute_identity
    // DESC: Removes only unreserved boot-local compute and memory observations owned by a superseded node identity.
    // ------------------=
    pub(crate) fn retire_local_compute_identity(&mut self, owner: NodeId) -> Result<usize, ResourceError> {
        let mut retired = 0;
        for index in 0..self.entries.len() {
            if self.entries[index].is_some_and(|resource| resource.owner == owner
                && matches!(resource.kind, ResourceKind::Compute | ResourceKind::Memory)) {
                if self.reserved[index] != 0 { return Err(ResourceError::Conflict); }
                self.entries[index] = None;
                self.offline_notices &= !(1u32 << index);
                retired += 1;
            }
        }
        Ok(retired)
    }

    // ------------------------=
    // FUNC: observe_local
    // DESC: Admits only the exact local node's measured native storage observation; callers cannot use this path to invent peer advertisements.
    // ------------------=
    pub(crate) fn observe_local(&mut self, resource: Resource, local: NodeId, now: u64) -> Result<bool, ResourceError> {
        if local.0 == [0;32] || resource.owner != local { return Err(ResourceError::AccessDenied); }
        self.apply(resource, now)
    }

    // ------------------------=
    // FUNC: restore_one_reservation
    // DESC: Recovers the single active coordinator claim idempotently and rejects conflicting inventory accounting instead of silently replacing it.
    // ------------------=
    pub(crate) fn restore_one_reservation(&mut self, id: ResourceId, generation: u64, bytes: u64) -> Result<(), ResourceError> {
        let i = self.entries.iter().position(|r|r.is_some_and(|r|r.id==id && r.generation==generation)).ok_or(ResourceError::Stale)?;
        if bytes==0 || bytes>self.entries[i].unwrap().capacity { return Err(ResourceError::Capacity); }
        if self.reserved[i]!=0 && self.reserved[i]!=bytes { return Err(ResourceError::Conflict); }
        self.reserved[i]=bytes;Ok(())
    }

    // ------------------------=
    // FUNC: accept_storage_advertisement
    // DESC: Applies a bounded advertisement already authenticated by the shared IOP router; its owner can only be the authenticated sender.
    // ------------------=
    pub(crate) fn accept_storage_advertisement(&mut self,
        request: crate::runtime::iop::remote::AuthenticatedStorageRequest, now: u64) -> Result<bool, ResourceError> {
        let resource = super::resource_protocol::decode(request.payload, request.peer, now)?;
        self.apply(resource, now)
    }
    // ------------------------=
    // FUNC: mark_peer_offline
    // DESC: Retains known resource and reservation identity when its owner disappears while excluding it from new placement immediately.
    // ------------------=
    pub(crate) fn mark_peer_offline(&mut self, peer: NodeId) {
        for (index, entry) in self.entries.iter_mut().enumerate() {
            if let Some(resource) = entry.as_mut().filter(|r| r.owner == peer && r.online) {
                resource.online = false; self.offline_notices |= 1u32 << index;
            }
        }
    }
    // ------------------------=
    // FUNC: expire
    // DESC: Marks expired observations offline without deleting placements or releasing outstanding durable reservations.
    // ------------------=
    pub(crate) fn expire(&mut self, now: u64) {
        for (index, entry) in self.entries.iter_mut().enumerate() {
            if let Some(resource) = entry.as_mut().filter(|r| r.expires <= now && r.online) {
                resource.online = false; self.offline_notices |= 1u32 << index;
            }
        }
    }

    // ------------------------=
    // FUNC: offline_notice
    // DESC: Peeks one coalesced committed loss without allocating or flooding IEF; failed delivery retains the observation for a later event-loop tick.
    // ------------------=
    pub(crate) fn offline_notice(&self) -> Option<Resource> {
        if self.offline_notices == 0 { return None; }
        self.entries[self.offline_notices.trailing_zeros() as usize]
    }

    // ------------------------=
    // FUNC: acknowledge_offline
    // DESC: Clears only the exact delivered observation; a changed generation or newer health state cannot be acknowledged by an old publisher.
    // ------------------=
    pub(crate) fn acknowledge_offline(&mut self, delivered: Resource) {
        if let Some(index) = self.entries.iter().position(|entry| *entry == Some(delivered)) {
            self.offline_notices &= !(1u32 << index);
        }
    }

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
        self.usable_kind(index, ResourceKind::Storage, now)
    }

    // ------------------------=
    // FUNC: usable_kind
    // DESC: Computes unreserved capacity for one healthy advertised resource kind without inventing coordinator-local capacity.
    // ------------------=
    pub(crate) fn usable_kind(&self, index: usize, kind: ResourceKind, now: u64) -> u64 {
        self.entries.get(index).copied().flatten().filter(|r| r.online && r.expires > now
            && r.health == Health::Healthy && r.kind == kind && r.capabilities & 1 != 0)
            .map(|r| r.available.saturating_sub(self.reserved[index])).unwrap_or(0)
    }

    // ------------------------=
    // FUNC: reserve
    // DESC: Reserves bounded transfer capacity so concurrent planned replicas cannot spend the same advertised bytes.
    // ------------------=
    pub(super) fn reserve(&mut self, id: ResourceId, generation: u64, bytes: u64, now: u64) -> Result<(), ResourceError> {
        self.reserve_kind(id, generation, ResourceKind::Storage, bytes, now)
    }

    // ------------------------=
    // FUNC: reserve_kind
    // DESC: Atomically reserves capacity from an exact live resource generation and kind.
    // ------------------=
    pub(crate) fn reserve_kind(&mut self, id: ResourceId, generation: u64, kind: ResourceKind, amount: u64, now: u64) -> Result<(), ResourceError> {
        let index = self.entries.iter().position(|r| r.map(|r| (r.id, r.generation)) == Some((id, generation))).ok_or(ResourceError::NotFound)?;
        if amount == 0 || self.usable_kind(index, kind, now) < amount { return Err(ResourceError::Capacity); }
        self.reserved[index] = self.reserved[index].checked_add(amount).ok_or(ResourceError::Capacity)?;
        Ok(())
    }

    // ------------------------=
    // FUNC: release
    // DESC: Releases one transfer reservation only against its original device generation.
    // ------------------=
    pub(crate) fn release(&mut self, id: ResourceId, generation: u64, bytes: u64) -> Result<(), ResourceError> {
        let index = self.entries.iter().position(|r| r.map(|r| (r.id, r.generation)) == Some((id, generation))).ok_or(ResourceError::NotFound)?;
        self.reserved[index] = self.reserved[index].checked_sub(bytes).ok_or(ResourceError::Conflict)?;
        Ok(())
    }


    // ------------------------=
    // FUNC: reserved_for
    // DESC: Exposes exact generation-bound reservation accounting for typed runtime diagnostics and behavioral verification.
    // ------------------=
    pub fn reserved_for(&self, id: ResourceId, generation: u64) -> Option<u64> {
        self.entries.iter().position(|r| r.map(|r| (r.id, r.generation)) == Some((id, generation)))
            .map(|index| self.reserved[index])
    }
}
