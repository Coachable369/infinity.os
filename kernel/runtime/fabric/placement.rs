use super::resources::{Directory, Resource, ResourceId};
use super::super::node::types::NodeId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageClass { Temporary, Protected, Critical }
impl StorageClass {
    // ------------------------=
    // FUNC: replicas
    // DESC: Defines explicit independent-node redundancy; this is object policy, never pathname policy.
    // ------------------=
    pub const fn replicas(self) -> usize { match self { Self::Temporary => 1, Self::Protected => 2, Self::Critical => 3 } }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability { Healthy, Degraded, Offline }

// ------------------------=
// FUNC: availability
// DESC: Distinguishes known content with no valid placement from degraded but still readable content.
// ------------------=
pub fn availability(class: StorageClass, verified_independent_replicas: usize) -> Availability {
    if verified_independent_replicas == 0 { Availability::Offline }
    else if verified_independent_replicas < class.replicas() { Availability::Degraded }
    else { Availability::Healthy }
}

// ------------------------=
// FUNC: select
// DESC: Deterministically chooses eligible storage on a new node failure domain, considering current capacity reservations.
// ------------------=
pub fn select(directory: &Directory, occupied_nodes: &[NodeId], excluded: &[ResourceId], bytes: u64, now: u64) -> Option<Resource> {
    if bytes == 0 { return None; }
    directory.entries().iter().enumerate().filter_map(|(index, entry)| {
        let resource = (*entry)?;
        if directory.usable(index, now) < bytes || occupied_nodes.contains(&resource.owner) || excluded.contains(&resource.id) { None }
        else { Some(resource) }
    }).min_by_key(|resource| resource.id)
}
