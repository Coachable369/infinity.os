//! Lease-bound local service discovery. Discovery reveals presence only and
//! never grants call authority or peer trust.

use super::types::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DiscoveredService {
    pub service_identity: u32,
    pub service_type: u32,
    pub endpoint: Endpoint,
    pub metadata_schema: u16,
    pub metadata_hash: u64,
    pub expires_at: u64,
    pub healthy: bool,
    pub future_node_identity: Option<[u8; 16]>,
}

pub struct DiscoveryManager {
    entries: [Option<DiscoveredService>; MAX_DISCOVERED_SERVICES],
    enabled: bool,
    dropped: u64,
}

impl DiscoveryManager {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a bounded discovery cache.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            entries: [None; MAX_DISCOVERED_SERVICES],
            enabled: true,
            dropped: 0,
        }
    }

    // ------------------------=
    // FUNC: set_enabled
    // DESC: Applies the active profile discovery posture.
    // ------------------=
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    // ------------------------=
    // FUNC: advertise
    // DESC: Publishes or refreshes an authorized typed service advertisement lease.
    // ------------------=
    pub fn advertise(&mut self, service: DiscoveredService, now: u64) -> Result<(), NetworkError> {
        if !self.enabled {
            return Err(NetworkError::NetworkUnavailable);
        }
        if service.expires_at <= now {
            return Err(NetworkError::Conflict);
        }
        if let Some(slot) = self
            .entries
            .iter()
            .position(|v| v.map(|e| e.service_identity) == Some(service.service_identity))
        {
            self.entries[slot] = Some(service);
            return Ok(());
        }
        let Some(slot) = self.entries.iter().position(Option::is_none) else {
            self.dropped = self.dropped.saturating_add(1);
            return Err(NetworkError::ResourceLimitExceeded);
        };
        self.entries[slot] = Some(service);
        Ok(())
    }

    // ------------------------=
    // FUNC: expire
    // DESC: Removes expired advertisements to prevent zombie discovery state.
    // ------------------=
    pub fn expire(&mut self, now: u64) {
        for entry in &mut self.entries {
            if entry.map(|v| v.expires_at <= now).unwrap_or(false) {
                *entry = None;
            }
        }
    }

    // ------------------------=
    // FUNC: count
    // DESC: Returns the number of live discovered services.
    // ------------------=
    pub fn count(&self) -> usize {
        self.entries.iter().flatten().count()
    }

    // ------------------------=
    // FUNC: nth
    // DESC: Returns one typed discovery result without granting authority.
    // ------------------=
    pub fn nth(&self, index: usize) -> Option<&DiscoveredService> {
        self.entries.iter().flatten().nth(index)
    }

    // ------------------------=
    // FUNC: dropped
    // DESC: Returns advertisements rejected by the bounded cache.
    // ------------------=
    pub const fn dropped(&self) -> u64 {
        self.dropped
    }
}
