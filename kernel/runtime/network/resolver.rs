//! Deadline-aware bounded resolver cache. Host-local and fixture records are
//! implemented; wire DNS remains behind the adapter boundary.

use super::types::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResolutionSource {
    Local,
    Cache,
    TestFixture,
    Dns,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ResolveResult {
    pub addresses: [Option<IpAddress>; 4],
    pub count: u8,
    pub source: ResolutionSource,
    pub expiry: u64,
}

#[derive(Clone, Copy)]
struct ResolverEntry {
    name_hash: u64,
    result: ResolveResult,
    negative: bool,
}

pub struct Resolver {
    entries: [Option<ResolverEntry>; MAX_RESOLVER_ENTRIES],
    servers: [Option<IpAddress>; 2],
    enabled: bool,
    failures: u64,
}

impl Resolver {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty bounded resolver in enabled state.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            entries: [None; MAX_RESOLVER_ENTRIES],
            servers: [None; 2],
            enabled: true,
            failures: 0,
        }
    }

    // ------------------------=
    // FUNC: set_enabled
    // DESC: Enables or disables remote resolution without affecting local names.
    // ------------------=
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    // ------------------------=
    // FUNC: enabled
    // DESC: Returns whether remote name resolution is enabled by active configuration.
    // ------------------=
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    // ------------------------=
    // FUNC: set_server
    // DESC: Sets or clears one bounded typed resolver endpoint without introducing text configuration.
    // ------------------=
    pub fn set_server(
        &mut self,
        index: usize,
        address: Option<IpAddress>,
    ) -> Result<(), NetworkError> {
        let slot = self
            .servers
            .get_mut(index)
            .ok_or(NetworkError::ResourceLimitExceeded)?;
        if *slot != address {
            *slot = address;
            self.entries.fill(None);
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: server
    // DESC: Returns one configured resolver endpoint for Settings and native inspection.
    // ------------------=
    pub fn server(&self, index: usize) -> Option<IpAddress> {
        self.servers.get(index).copied().flatten()
    }

    // ------------------------=
    // FUNC: install_fixture
    // DESC: Installs a bounded deterministic resolver record for tests and platform adapters.
    // ------------------=
    pub fn install_fixture(
        &mut self,
        name_hash: u64,
        addresses: &[IpAddress],
        expiry: u64,
    ) -> Result<(), NetworkError> {
        if addresses.len() > 4 {
            return Err(NetworkError::ResourceLimitExceeded);
        }
        let mut resolved = [None; 4];
        for (index, address) in addresses.iter().enumerate() {
            resolved[index] = Some(*address);
        }
        self.insert(ResolverEntry {
            name_hash,
            result: ResolveResult {
                addresses: resolved,
                count: addresses.len() as u8,
                source: ResolutionSource::TestFixture,
                expiry,
            },
            negative: false,
        })
    }

    // ------------------------=
    // FUNC: install_negative
    // DESC: Installs an expiring negative result without unbounded retry state.
    // ------------------=
    pub fn install_negative(&mut self, name_hash: u64, expiry: u64) -> Result<(), NetworkError> {
        self.insert(ResolverEntry {
            name_hash,
            result: ResolveResult {
                addresses: [None; 4],
                count: 0,
                source: ResolutionSource::Cache,
                expiry,
            },
            negative: true,
        })
    }

    // ------------------------=
    // FUNC: insert
    // DESC: Replaces an existing cache key or inserts into a bounded free slot.
    // ------------------=
    fn insert(&mut self, entry: ResolverEntry) -> Result<(), NetworkError> {
        if let Some(slot) = self
            .entries
            .iter()
            .position(|v| v.map(|e| e.name_hash) == Some(entry.name_hash))
        {
            self.entries[slot] = Some(entry);
            return Ok(());
        }
        let slot = self
            .entries
            .iter()
            .position(Option::is_none)
            .ok_or(NetworkError::ResourceLimitExceeded)?;
        self.entries[slot] = Some(entry);
        Ok(())
    }

    // ------------------------=
    // FUNC: resolve
    // DESC: Resolves local or cached names under an explicit monotonic deadline.
    // ------------------=
    pub fn resolve(
        &mut self,
        name_hash: u64,
        now: u64,
        deadline: u64,
    ) -> Result<ResolveResult, NetworkError> {
        if deadline <= now {
            self.failures = self.failures.saturating_add(1);
            return Err(NetworkError::ResolutionTimeout);
        }
        if name_hash == 1 {
            return Ok(ResolveResult {
                addresses: [Some(IpAddress::V4([127, 0, 0, 1])), None, None, None],
                count: 1,
                source: ResolutionSource::Local,
                expiry: u64::MAX,
            });
        }
        if let Some(entry) = self
            .entries
            .iter()
            .flatten()
            .find(|v| v.name_hash == name_hash && now < v.result.expiry)
            .copied()
        {
            if entry.negative {
                self.failures = self.failures.saturating_add(1);
                return Err(NetworkError::NameResolutionFailed);
            }
            let mut result = entry.result;
            if result.source != ResolutionSource::TestFixture {
                result.source = ResolutionSource::Cache;
            }
            return Ok(result);
        }
        self.failures = self.failures.saturating_add(1);
        if self.enabled {
            Err(NetworkError::ResolverUnavailable)
        } else {
            Err(NetworkError::NetworkUnavailable)
        }
    }

    // ------------------------=
    // FUNC: expire
    // DESC: Reclaims expired positive and negative cache entries.
    // ------------------=
    pub fn expire(&mut self, now: u64) {
        for entry in &mut self.entries {
            if entry.map(|v| v.result.expiry <= now).unwrap_or(false) {
                *entry = None;
            }
        }
    }

    // ------------------------=
    // FUNC: count
    // DESC: Returns the current bounded cache population.
    // ------------------=
    pub fn count(&self) -> usize {
        self.entries.iter().flatten().count()
    }

    // ------------------------=
    // FUNC: failures
    // DESC: Returns observed resolver failures without fabricated timing data.
    // ------------------=
    pub const fn failures(&self) -> u64 {
        self.failures
    }
}
