//! Read-only fabric management projection. All consumers use the existing
//! resource directory; no second registry or cached capacity authority exists.
use super::resources::{FabricCapacity, ResourceError, RESOURCE_KINDS};
use crate::runtime::{InfinityRuntime, identity::{StableId, SessionState, MAX_SESSIONS, SESSION_IDENTITY_MANAGE}};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapacitySnapshot {
    pub observed_at: u64,
    pub categories: [FabricCapacity; 11],
}

// ------------------------=
// FUNC: inspect_capacity
// DESC: Authenticates the active operator before projecting authoritative local and fabric capacity for GUI, CLI and AI clients.
// ------------------=
pub fn inspect_capacity(user: StableId, session: StableId) -> Result<CapacitySnapshot, ResourceError> {
    crate::runtime::with_runtime(|runtime| {
        let now = runtime.node_clock.ok_or(ResourceError::Stale)?;
        inspect_capacity_from(runtime, user, session, now)
    }).ok_or(ResourceError::NotFound)?
}

// ------------------------=
// FUNC: inspect_capacity_from
// DESC: Shares session authorization and capacity projection without granting resource access or changing allocations.
// ------------------=
pub(crate) fn inspect_capacity_from(runtime: &InfinityRuntime, user: StableId, session: StableId, now: u64) -> Result<CapacitySnapshot, ResourceError> {
    let authorized = runtime.identity.console_session_authorized(session, user)
        && (0..MAX_SESSIONS).filter_map(|i| runtime.identity.session_nth(i)).any(|candidate| {
            candidate.id == session && candidate.user == user && candidate.state == SessionState::Active
                && candidate.capabilities & SESSION_IDENTITY_MANAGE != 0
        });
    if !authorized { return Err(ResourceError::AccessDenied); }
    let local = runtime.nodes.local_id().ok_or(ResourceError::NotFound)?;
    let mut snapshot = CapacitySnapshot { observed_at: now, categories: [FabricCapacity::default(); 11] };
    for (index, kind) in RESOURCE_KINDS.iter().enumerate() {
        snapshot.categories[index] = runtime.fabric_resources.capacity(*kind, local, now)?;
    }
    Ok(snapshot)
}
