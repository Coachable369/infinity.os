//! Native, bounded application resource authority and accounting.
//!
//! The policy service deliberately separates requested resources from granted
//! authority. CPU scheduling weight, memory ceilings, message queues, and I/O
//! priority map to existing runtime enforcement. Hardware accelerators,
//! bandwidth shaping, and remote Pool allocation report their actual support.

use super::capability::{CapabilityId, CapabilityManager, CapabilityType};
use super::execution::{
    ContextHandle, ExecutionError, ExecutionManager, PriorityClass as RuntimePriority,
    ResourceBudget, SecurityIdentity,
};

pub const POLICY_SCHEMA_VERSION: u16 = 1;
pub const MAX_APPLICATION_POLICIES: usize = 16;
pub const MAX_RESOURCE_LEASES: usize = 24;
pub const PERSISTENCE_BYTES: usize = 512;

pub const EVENT_APP_POLICY_CHANGED: u32 = 0xe101;
pub const EVENT_APP_LIMIT_REACHED: u32 = 0xe102;
pub const EVENT_APP_THROTTLED: u32 = 0xe103;
pub const EVENT_APP_EXPANDED: u32 = 0xe104;
pub const EVENT_BURST_GRANTED: u32 = 0xe105;
pub const EVENT_BURST_REVOKED: u32 = 0xe106;
pub const EVENT_LEASE_GRANTED: u32 = 0xe111;
pub const EVENT_LEASE_REVOKED: u32 = 0xe112;
pub const EVENT_DEFAULT_POLICY_CHANGED: u32 = 0xe121;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ResourceMode {
    Restricted = 1,
    Balanced = 2,
    Expanded = 3,
    Custom = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LocalityPolicy {
    LocalOnly = 1,
    LocalPreferred = 2,
    PoolAllowed = 3,
    PoolPreferred = 4,
    PoolRequired = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnforcementState {
    Enforced,
    PartiallyEnforced,
    Advisory,
    Unsupported,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicySource {
    SystemSafety,
    ManagedPolicy,
    UserRestriction,
    ApplicationOverride,
    UserDefault,
    ManifestRequest,
    RuntimeAvailability,
    PowerOverlay,
    ThermalOverlay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundThrottle {
    Aggressive,
    Balanced,
    Light,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ResourceClass {
    Cpu,
    Memory,
    Gpu,
    Npu,
    StorageIo,
    Network,
    Background,
    RemotePool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemCapacity {
    pub logical_compute_units: u16,
    pub memory_bytes: u64,
    pub gpu_available: bool,
    pub npu_available: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemConditions {
    pub on_battery: bool,
    pub low_power: bool,
    pub thermal_pressure: bool,
    pub foreground: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApplicationManifestRequest {
    pub cpu_minimum: u16,
    pub cpu_preferred: u16,
    pub cpu_maximum: u16,
    pub memory_preferred: u64,
    pub memory_maximum: u64,
    pub gpu_requested: bool,
    pub npu_requested: bool,
    pub background_requested: bool,
    pub locality: LocalityPolicy,
}

impl ApplicationManifestRequest {
    // ------------------------=
    // FUNC: balanced
    // DESC: Produces a bounded ordinary application request that grants no authority by itself.
    // ------------------=
    pub const fn balanced() -> Self {
        Self {
            cpu_minimum: 1,
            cpu_preferred: 2,
            cpu_maximum: 4,
            memory_preferred: 4 * 1024 * 1024,
            memory_maximum: 8 * 1024 * 1024,
            gpu_requested: false,
            npu_requested: false,
            background_requested: true,
            locality: LocalityPolicy::LocalPreferred,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlobalResourceDefaults {
    pub mode: ResourceMode,
    pub allow_pool: bool,
    pub allow_background_pool: bool,
    pub prefer_local: bool,
    pub allow_burst: bool,
    pub reduce_on_battery: bool,
    pub disable_pool_low_power: bool,
    pub notify_expansion: bool,
    pub notify_throttling: bool,
    pub background_throttle: BackgroundThrottle,
}

impl GlobalResourceDefaults {
    // ------------------------=
    // FUNC: balanced
    // DESC: Returns safe capacity-aware defaults with remote Pool authority disabled.
    // ------------------=
    pub const fn balanced() -> Self {
        Self {
            mode: ResourceMode::Balanced,
            allow_pool: false,
            allow_background_pool: false,
            prefer_local: true,
            allow_burst: true,
            reduce_on_battery: true,
            disable_pool_low_power: true,
            notify_expansion: true,
            notify_throttling: true,
            background_throttle: BackgroundThrottle::Balanced,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApplicationPolicyOverride {
    pub app_id: AppId,
    pub mode: ResourceMode,
    pub cpu_maximum: u16,
    pub cpu_weight: u16,
    pub memory_hard_limit: u64,
    pub message_queue_limit: u16,
    pub storage_io_priority: u8,
    pub network_priority: u8,
    pub background_allowed: bool,
    pub burst_allowed: bool,
    pub locality: LocalityPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectiveApplicationResourcePolicy {
    pub app_id: AppId,
    pub mode: ResourceMode,
    pub inherited: bool,
    pub cpu_minimum: u16,
    pub cpu_preferred: u16,
    pub cpu_maximum: u16,
    pub cpu_weight: u16,
    pub memory_preferred: u64,
    pub memory_soft_limit: u64,
    pub memory_hard_limit: u64,
    pub message_queue_limit: u16,
    pub storage_io_priority: u8,
    pub network_priority: u8,
    pub background_allowed: bool,
    pub burst_allowed: bool,
    pub gpu_allowed: bool,
    pub npu_allowed: bool,
    pub locality: LocalityPolicy,
    pub limit_source: PolicySource,
    pub cpu_enforcement: EnforcementState,
    pub memory_enforcement: EnforcementState,
    pub gpu_enforcement: EnforcementState,
    pub npu_enforcement: EnforcementState,
    pub storage_enforcement: EnforcementState,
    pub network_enforcement: EnforcementState,
    pub background_enforcement: EnforcementState,
    pub pool_enforcement: EnforcementState,
}

impl EffectiveApplicationResourcePolicy {
    // ------------------------=
    // FUNC: runtime_budget
    // DESC: Projects authoritative policy into the runtime mechanisms that currently enforce it.
    // ------------------=
    pub const fn runtime_budget(self) -> ResourceBudget {
        ResourceBudget {
            memory_limit: self.memory_hard_limit,
            cpu_weight: self.cpu_weight,
            message_queue_limit: self.message_queue_limit,
            io_priority: self.storage_io_priority,
        }
    }

    // ------------------------=
    // FUNC: runtime_priority
    // DESC: Maps semantic foreground and background policy to the native scheduler class.
    // ------------------=
    pub const fn runtime_priority(self) -> RuntimePriority {
        if self.background_allowed && self.cpu_weight <= 40 {
            RuntimePriority::Background
        } else {
            RuntimePriority::Normal
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ApplicationResourceUsage {
    pub cpu_ticks: u64,
    pub memory_bytes: u64,
    pub storage_io_units: u64,
    pub network_bytes: u64,
    pub gpu_jobs: u64,
    pub npu_jobs: u64,
    pub background: bool,
    pub throttled: bool,
    pub burst_active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceLease {
    pub lease_id: u64,
    pub app_id: AppId,
    pub class: ResourceClass,
    pub granted_capacity: u64,
    pub created: u64,
    pub expires: u64,
    pub revocable: bool,
    pub active: bool,
    pub locality: LocalityPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourcePolicyError {
    AccessDenied,
    InvalidPolicy,
    UnknownApplication,
    Unsupported,
    Unavailable,
    LimitExceeded,
    Full,
    UnknownLease,
    ExpiredLease,
    RevokedLease,
    Runtime(ExecutionError),
    InvalidPersistence,
}

#[derive(Clone, Copy)]
pub struct ApplicationResourceManager {
    defaults: GlobalResourceDefaults,
    overrides: [Option<ApplicationPolicyOverride>; MAX_APPLICATION_POLICIES],
    usage: [Option<(AppId, ApplicationResourceUsage)>; MAX_APPLICATION_POLICIES],
    leases: [Option<ResourceLease>; MAX_RESOURCE_LEASES],
    next_lease_id: u64,
    revision: u64,
}

impl ApplicationResourceManager {
    // ------------------------=
    // FUNC: new
    // DESC: Creates the bounded native policy authority with Balanced and local-preferred defaults.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            defaults: GlobalResourceDefaults::balanced(),
            overrides: [None; MAX_APPLICATION_POLICIES],
            usage: [None; MAX_APPLICATION_POLICIES],
            leases: [None; MAX_RESOURCE_LEASES],
            next_lease_id: 1,
            revision: 1,
        }
    }

    // ------------------------=
    // FUNC: defaults
    // DESC: Returns the authoritative global policy inherited by applications without overrides.
    // ------------------=
    pub const fn defaults(&self) -> GlobalResourceDefaults {
        self.defaults
    }

    // ------------------------=
    // FUNC: revision
    // DESC: Exposes monotonic policy state for IEF reconciliation after missed events.
    // ------------------=
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    // ------------------------=
    // FUNC: update_defaults
    // DESC: Commits validated global defaults under separately scoped modification authority.
    // ------------------=
    pub fn update_defaults(
        &mut self,
        defaults: GlobalResourceDefaults,
        caller: SecurityIdentity,
        capability: CapabilityId,
        capabilities: &CapabilityManager,
        now: u64,
    ) -> Result<(), ResourcePolicyError> {
        capabilities
            .validate(
                capability,
                caller,
                CapabilityType::ResourcePolicyModify,
                0,
                1,
                0,
                now,
            )
            .map_err(|_| ResourcePolicyError::AccessDenied)?;
        if defaults.allow_background_pool && !defaults.allow_pool {
            return Err(ResourcePolicyError::InvalidPolicy);
        }
        self.defaults = defaults;
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    // ------------------------=
    // FUNC: set_override
    // DESC: Persists one AppId-bound override after capability validation and semantic checks.
    // ------------------=
    pub fn set_override(
        &mut self,
        policy: ApplicationPolicyOverride,
        caller: SecurityIdentity,
        capability: CapabilityId,
        capabilities: &CapabilityManager,
        now: u64,
    ) -> Result<(), ResourcePolicyError> {
        capabilities
            .validate(
                capability,
                caller,
                CapabilityType::ResourcePolicyModify,
                policy.app_id.0 as u64,
                1,
                0,
                now,
            )
            .map_err(|_| ResourcePolicyError::AccessDenied)?;
        validate_override(policy)?;
        if let Some(slot) = self
            .overrides
            .iter_mut()
            .find(|entry| entry.map(|value| value.app_id) == Some(policy.app_id))
        {
            *slot = Some(policy);
        } else {
            let slot = self
                .overrides
                .iter_mut()
                .find(|entry| entry.is_none())
                .ok_or(ResourcePolicyError::Full)?;
            *slot = Some(policy);
        }
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    // ------------------------=
    // FUNC: set_mode_from_trusted_ui
    // DESC: Commits a bounded preset selected through the non-spoofable system application menu.
    // ------------------=
    pub(crate) fn set_mode_from_trusted_ui(
        &mut self,
        app_id: AppId,
        mode: ResourceMode,
    ) -> Result<(), ResourcePolicyError> {
        if app_id.0 == 0 || mode == ResourceMode::Custom {
            return Err(ResourcePolicyError::InvalidPolicy);
        }
        let (cpu_maximum, cpu_weight, memory_hard_limit, queue, io, burst) = match mode {
            ResourceMode::Restricted => (1, 25, 4 * 1024 * 1024, 16, 1, false),
            ResourceMode::Balanced => (2, 100, 8 * 1024 * 1024, 32, 3, true),
            ResourceMode::Expanded => (4, 200, 8 * 1024 * 1024, 64, 5, true),
            ResourceMode::Custom => return Err(ResourcePolicyError::InvalidPolicy),
        };
        let policy = ApplicationPolicyOverride {
            app_id,
            mode,
            cpu_maximum,
            cpu_weight,
            memory_hard_limit,
            message_queue_limit: queue,
            storage_io_priority: io,
            network_priority: io,
            background_allowed: true,
            burst_allowed: burst,
            locality: LocalityPolicy::LocalOnly,
        };
        validate_override(policy)?;
        if let Some(slot) = self
            .overrides
            .iter_mut()
            .find(|entry| entry.map(|value| value.app_id) == Some(app_id))
        {
            *slot = Some(policy);
        } else {
            let slot = self
                .overrides
                .iter_mut()
                .find(|entry| entry.is_none())
                .ok_or(ResourcePolicyError::Full)?;
            *slot = Some(policy);
        }
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    // ------------------------=
    // FUNC: clear_override
    // DESC: Removes an explicit AppId override so the application inherits current defaults again.
    // ------------------=
    pub fn clear_override(&mut self, app_id: AppId) -> Result<(), ResourcePolicyError> {
        let slot = self
            .overrides
            .iter_mut()
            .find(|entry| entry.map(|value| value.app_id) == Some(app_id))
            .ok_or(ResourcePolicyError::UnknownApplication)?;
        *slot = None;
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    // ------------------------=
    // FUNC: override_for
    // DESC: Looks up persistent authority by stable AppId rather than display name.
    // ------------------=
    pub fn override_for(&self, app_id: AppId) -> Option<ApplicationPolicyOverride> {
        self.overrides
            .iter()
            .flatten()
            .copied()
            .find(|policy| policy.app_id == app_id)
    }

    // ------------------------=
    // FUNC: effective_policy
    // DESC: Resolves hard limits, defaults, manifest narrowing, overrides, and runtime overlays deterministically.
    // ------------------=
    pub fn effective_policy(
        &self,
        app_id: AppId,
        manifest: ApplicationManifestRequest,
        capacity: SystemCapacity,
        conditions: SystemConditions,
    ) -> Result<EffectiveApplicationResourcePolicy, ResourcePolicyError> {
        if app_id.0 == 0 || capacity.logical_compute_units == 0 || capacity.memory_bytes == 0 {
            return Err(ResourcePolicyError::InvalidPolicy);
        }
        let explicit = self.override_for(app_id);
        let mode = explicit.map(|value| value.mode).unwrap_or(self.defaults.mode);
        let (share, weight, queue, io) = mode_parameters(mode);
        let system_cpu = capacity.logical_compute_units.max(1);
        let policy_cpu = ((u32::from(system_cpu) * u32::from(share)) / 100)
            .max(1)
            .min(u32::from(system_cpu)) as u16;
        let requested_cpu = manifest.cpu_maximum.max(manifest.cpu_minimum).max(1);
        let mut cpu_maximum = policy_cpu.min(requested_cpu);
        let mut cpu_weight = explicit.map(|value| value.cpu_weight).unwrap_or(weight);
        let system_memory_ceiling = capacity.memory_bytes.saturating_mul(u64::from(share)) / 100;
        let request_memory = manifest.memory_maximum.max(manifest.memory_preferred).max(64 * 1024);
        let mut memory_hard_limit = explicit
            .map(|value| value.memory_hard_limit)
            .unwrap_or(system_memory_ceiling.min(request_memory))
            .min(capacity.memory_bytes)
            .max(64 * 1024);
        let mut source = if explicit.is_some() {
            PolicySource::ApplicationOverride
        } else if requested_cpu < policy_cpu || request_memory < system_memory_ceiling {
            PolicySource::ManifestRequest
        } else {
            PolicySource::UserDefault
        };
        let mut burst_allowed = explicit
            .map(|value| value.burst_allowed)
            .unwrap_or(self.defaults.allow_burst);
        let mut locality = explicit.map(|value| value.locality).unwrap_or_else(|| {
            if self.defaults.prefer_local {
                LocalityPolicy::LocalPreferred
            } else {
                LocalityPolicy::LocalOnly
            }
        });
        if (!self.defaults.allow_pool || conditions.low_power)
            && matches!(
                locality,
                LocalityPolicy::PoolAllowed
                    | LocalityPolicy::PoolPreferred
                    | LocalityPolicy::PoolRequired
            )
        {
            locality = if self.defaults.prefer_local {
                LocalityPolicy::LocalPreferred
            } else {
                LocalityPolicy::LocalOnly
            };
        }
        if (conditions.on_battery && self.defaults.reduce_on_battery) || conditions.low_power {
            cpu_maximum = cpu_maximum.min((system_cpu / 2).max(1));
            cpu_weight = cpu_weight.min(50);
            memory_hard_limit = memory_hard_limit.min(capacity.memory_bytes / 3);
            burst_allowed = false;
            source = PolicySource::PowerOverlay;
        }
        if conditions.thermal_pressure {
            cpu_maximum = cpu_maximum.min((system_cpu / 2).max(1));
            cpu_weight = cpu_weight.min(35);
            burst_allowed = false;
            source = PolicySource::ThermalOverlay;
        }
        let background_allowed = explicit
            .map(|value| value.background_allowed)
            .unwrap_or(manifest.background_requested);
        if !conditions.foreground && background_allowed {
            cpu_weight = cpu_weight.min(match self.defaults.background_throttle {
                BackgroundThrottle::Aggressive => 20,
                BackgroundThrottle::Balanced => 40,
                BackgroundThrottle::Light => 70,
            });
        }
        let gpu_allowed = manifest.gpu_requested && capacity.gpu_available;
        let npu_allowed = manifest.npu_requested && capacity.npu_available;
        Ok(EffectiveApplicationResourcePolicy {
            app_id,
            mode,
            inherited: explicit.is_none(),
            cpu_minimum: manifest.cpu_minimum.min(cpu_maximum),
            cpu_preferred: manifest.cpu_preferred.min(cpu_maximum),
            cpu_maximum,
            cpu_weight,
            memory_preferred: manifest.memory_preferred.min(memory_hard_limit),
            memory_soft_limit: memory_hard_limit.saturating_mul(3) / 4,
            memory_hard_limit,
            message_queue_limit: explicit.map(|value| value.message_queue_limit).unwrap_or(queue),
            storage_io_priority: explicit.map(|value| value.storage_io_priority).unwrap_or(io),
            network_priority: explicit.map(|value| value.network_priority).unwrap_or(3),
            background_allowed,
            burst_allowed,
            gpu_allowed,
            npu_allowed,
            locality,
            limit_source: source,
            cpu_enforcement: EnforcementState::PartiallyEnforced,
            memory_enforcement: EnforcementState::Enforced,
            gpu_enforcement: if capacity.gpu_available {
                EnforcementState::Advisory
            } else {
                EnforcementState::Unavailable
            },
            npu_enforcement: if capacity.npu_available {
                EnforcementState::Advisory
            } else {
                EnforcementState::Unavailable
            },
            storage_enforcement: EnforcementState::PartiallyEnforced,
            network_enforcement: EnforcementState::Advisory,
            background_enforcement: EnforcementState::PartiallyEnforced,
            pool_enforcement: match locality {
                LocalityPolicy::LocalOnly | LocalityPolicy::LocalPreferred => EnforcementState::Enforced,
                _ => EnforcementState::Unsupported,
            },
        })
    }

    // ------------------------=
    // FUNC: apply_to_context
    // DESC: Applies the enforceable projection atomically while preserving explicit advisory status elsewhere.
    // ------------------=
    pub fn apply_to_context(
        &self,
        execution: &mut ExecutionManager,
        handle: ContextHandle,
        effective: EffectiveApplicationResourcePolicy,
    ) -> Result<(), ResourcePolicyError> {
        execution
            .update_budget(handle, effective.runtime_budget(), effective.runtime_priority())
            .map_err(ResourcePolicyError::Runtime)
    }

    // ------------------------=
    // FUNC: account
    // DESC: Records bounded per-app usage and marks hard-ceiling pressure without fabricating unavailable metrics.
    // ------------------=
    pub fn account(
        &mut self,
        app_id: AppId,
        usage: ApplicationResourceUsage,
        effective: EffectiveApplicationResourcePolicy,
    ) -> Result<(), ResourcePolicyError> {
        let mut observed = usage;
        observed.throttled = observed.memory_bytes >= effective.memory_hard_limit
            || effective.cpu_weight <= 40
            || (!effective.background_allowed && observed.background);
        if let Some(slot) = self
            .usage
            .iter_mut()
            .find(|entry| entry.map(|value| value.0) == Some(app_id))
        {
            *slot = Some((app_id, observed));
        } else {
            let slot = self
                .usage
                .iter_mut()
                .find(|entry| entry.is_none())
                .ok_or(ResourcePolicyError::Full)?;
            *slot = Some((app_id, observed));
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: usage_for
    // DESC: Returns only actually recorded resource counters for an AppId.
    // ------------------=
    pub fn usage_for(&self, app_id: AppId) -> Option<ApplicationResourceUsage> {
        self.usage
            .iter()
            .flatten()
            .find(|entry| entry.0 == app_id)
            .map(|entry| entry.1)
    }

    // ------------------------=
    // FUNC: request_lease
    // DESC: Grants a bounded local resource lease only within effective policy and explicit use authority.
    // ------------------=
    pub fn request_lease(
        &mut self,
        app_id: AppId,
        class: ResourceClass,
        requested_capacity: u64,
        duration: u64,
        effective: EffectiveApplicationResourcePolicy,
        caller: SecurityIdentity,
        capability: CapabilityId,
        capabilities: &CapabilityManager,
        now: u64,
    ) -> Result<ResourceLease, ResourcePolicyError> {
        capabilities
            .validate(
                capability,
                caller,
                CapabilityType::ResourceUse,
                app_id.0 as u64,
                1,
                class as u64,
                now,
            )
            .map_err(|_| ResourcePolicyError::AccessDenied)?;
        if requested_capacity == 0 || duration == 0 {
            return Err(ResourcePolicyError::InvalidPolicy);
        }
        let ceiling = class_ceiling(effective, class)?;
        if requested_capacity > ceiling {
            return Err(ResourcePolicyError::LimitExceeded);
        }
        let slot = self
            .leases
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(ResourcePolicyError::Full)?;
        let lease = ResourceLease {
            lease_id: self.next_lease_id,
            app_id,
            class,
            granted_capacity: requested_capacity,
            created: now,
            expires: now.saturating_add(duration),
            revocable: true,
            active: true,
            locality: effective.locality,
        };
        self.next_lease_id = self.next_lease_id.saturating_add(1);
        *slot = Some(lease);
        Ok(lease)
    }

    // ------------------------=
    // FUNC: inspect_lease
    // DESC: Rejects unknown, revoked, and stale leases before returning current authority.
    // ------------------=
    pub fn inspect_lease(&self, lease_id: u64, now: u64) -> Result<ResourceLease, ResourcePolicyError> {
        let lease = self
            .leases
            .iter()
            .flatten()
            .find(|lease| lease.lease_id == lease_id)
            .copied()
            .ok_or(ResourcePolicyError::UnknownLease)?;
        if !lease.active {
            return Err(ResourcePolicyError::RevokedLease);
        }
        if now >= lease.expires {
            return Err(ResourcePolicyError::ExpiredLease);
        }
        Ok(lease)
    }

    // ------------------------=
    // FUNC: revoke_lease
    // DESC: Revokes active lease authority without restarting the holder.
    // ------------------=
    pub fn revoke_lease(&mut self, lease_id: u64) -> Result<(), ResourcePolicyError> {
        let lease = self
            .leases
            .iter_mut()
            .flatten()
            .find(|lease| lease.lease_id == lease_id)
            .ok_or(ResourcePolicyError::UnknownLease)?;
        lease.active = false;
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    // ------------------------=
    // FUNC: release_app_leases
    // DESC: Reclaims every temporary lease owned by an exiting application.
    // ------------------=
    pub fn release_app_leases(&mut self, app_id: AppId) -> usize {
        let mut released = 0usize;
        for lease in self.leases.iter_mut().flatten() {
            if lease.app_id == app_id && lease.active {
                lease.active = false;
                released += 1;
            }
        }
        released
    }

    // ------------------------=
    // FUNC: encode
    // DESC: Serializes defaults and explicit overrides into a versioned fixed-size System object payload.
    // ------------------=
    pub fn encode(&self) -> [u8; PERSISTENCE_BYTES] {
        let mut out = [0u8; PERSISTENCE_BYTES];
        out[..4].copy_from_slice(b"IRP1");
        out[4..6].copy_from_slice(&POLICY_SCHEMA_VERSION.to_le_bytes());
        out[6] = self.defaults.mode as u8;
        out[7] = defaults_flags(self.defaults);
        out[8] = self.defaults.background_throttle as u8;
        out[16..24].copy_from_slice(&self.revision.to_le_bytes());
        let mut at = 32usize;
        for policy in self.overrides.iter().flatten() {
            if at + 28 > out.len() {
                break;
            }
            out[at..at + 4].copy_from_slice(&policy.app_id.0.to_le_bytes());
            out[at + 4] = policy.mode as u8;
            out[at + 5] = policy.locality as u8;
            out[at + 6] = u8::from(policy.background_allowed);
            out[at + 7] = u8::from(policy.burst_allowed);
            out[at + 8..at + 10].copy_from_slice(&policy.cpu_maximum.to_le_bytes());
            out[at + 10..at + 12].copy_from_slice(&policy.cpu_weight.to_le_bytes());
            out[at + 12..at + 20].copy_from_slice(&policy.memory_hard_limit.to_le_bytes());
            out[at + 20..at + 22].copy_from_slice(&policy.message_queue_limit.to_le_bytes());
            out[at + 22] = policy.storage_io_priority;
            out[at + 23] = policy.network_priority;
            at += 28;
        }
        out[24] = ((at - 32) / 28) as u8;
        out
    }

    // ------------------------=
    // FUNC: decode
    // DESC: Restores validated version-one policy state without accepting malformed or unbounded entries.
    // ------------------=
    pub fn decode(input: &[u8]) -> Result<Self, ResourcePolicyError> {
        if input.len() != PERSISTENCE_BYTES || &input[..4] != b"IRP1" || u16::from_le_bytes([input[4], input[5]]) != 1 {
            return Err(ResourcePolicyError::InvalidPersistence);
        }
        let defaults = GlobalResourceDefaults {
            mode: decode_mode(input[6])?,
            allow_pool: input[7] & 1 != 0,
            allow_background_pool: input[7] & 2 != 0,
            prefer_local: input[7] & 4 != 0,
            allow_burst: input[7] & 8 != 0,
            reduce_on_battery: input[7] & 16 != 0,
            disable_pool_low_power: input[7] & 32 != 0,
            notify_expansion: input[7] & 64 != 0,
            notify_throttling: input[7] & 128 != 0,
            background_throttle: decode_background(input[8])?,
        };
        if defaults.allow_background_pool && !defaults.allow_pool {
            return Err(ResourcePolicyError::InvalidPersistence);
        }
        let count = input[24] as usize;
        if count > MAX_APPLICATION_POLICIES {
            return Err(ResourcePolicyError::InvalidPersistence);
        }
        let mut manager = Self::new();
        manager.defaults = defaults;
        manager.revision = u64::from_le_bytes(input[16..24].try_into().unwrap());
        let mut at = 32usize;
        for index in 0..count {
            let policy = ApplicationPolicyOverride {
                app_id: AppId(u32::from_le_bytes(input[at..at + 4].try_into().unwrap())),
                mode: decode_mode(input[at + 4])?,
                locality: decode_locality(input[at + 5])?,
                background_allowed: input[at + 6] != 0,
                burst_allowed: input[at + 7] != 0,
                cpu_maximum: u16::from_le_bytes(input[at + 8..at + 10].try_into().unwrap()),
                cpu_weight: u16::from_le_bytes(input[at + 10..at + 12].try_into().unwrap()),
                memory_hard_limit: u64::from_le_bytes(input[at + 12..at + 20].try_into().unwrap()),
                message_queue_limit: u16::from_le_bytes(input[at + 20..at + 22].try_into().unwrap()),
                storage_io_priority: input[at + 22],
                network_priority: input[at + 23],
            };
            validate_override(policy).map_err(|_| ResourcePolicyError::InvalidPersistence)?;
            manager.overrides[index] = Some(policy);
            at += 28;
        }
        Ok(manager)
    }
}

// ------------------------=
// FUNC: validate_override
// DESC: Rejects nonsensical or misleading quantitative application policy values.
// ------------------=
fn validate_override(policy: ApplicationPolicyOverride) -> Result<(), ResourcePolicyError> {
    if policy.app_id.0 == 0
        || policy.cpu_maximum == 0
        || policy.cpu_weight == 0
        || policy.cpu_weight > 1000
        || policy.memory_hard_limit < 64 * 1024
        || policy.message_queue_limit == 0
        || policy.storage_io_priority > 7
        || policy.network_priority > 7
    {
        return Err(ResourcePolicyError::InvalidPolicy);
    }
    Ok(())
}

// ------------------------=
// FUNC: mode_parameters
// DESC: Maps user-facing modes to capacity percentages and enforceable runtime weights.
// ------------------=
const fn mode_parameters(mode: ResourceMode) -> (u16, u16, u16, u8) {
    match mode {
        ResourceMode::Restricted => (20, 25, 8, 1),
        ResourceMode::Balanced => (50, 100, 32, 3),
        ResourceMode::Expanded => (85, 240, 64, 5),
        ResourceMode::Custom => (50, 100, 32, 3),
    }
}

// ------------------------=
// FUNC: class_ceiling
// DESC: Returns only real locally representable capacity and rejects unavailable remote classes.
// ------------------=
fn class_ceiling(policy: EffectiveApplicationResourcePolicy, class: ResourceClass) -> Result<u64, ResourcePolicyError> {
    match class {
        ResourceClass::Cpu => Ok(policy.cpu_maximum as u64),
        ResourceClass::Memory => Ok(policy.memory_hard_limit),
        ResourceClass::StorageIo => Ok(policy.storage_io_priority as u64 + 1),
        ResourceClass::Network => Ok(policy.network_priority as u64 + 1),
        ResourceClass::Background => Ok(u64::from(policy.background_allowed)),
        ResourceClass::Gpu if policy.gpu_allowed => Ok(1),
        ResourceClass::Npu if policy.npu_allowed => Ok(1),
        ResourceClass::Gpu | ResourceClass::Npu => Err(ResourcePolicyError::Unavailable),
        ResourceClass::RemotePool => Err(ResourcePolicyError::Unsupported),
    }
}

// ------------------------=
// FUNC: defaults_flags
// DESC: Packs global boolean controls into the stable persistence schema.
// ------------------=
const fn defaults_flags(defaults: GlobalResourceDefaults) -> u8 {
    (defaults.allow_pool as u8)
        | ((defaults.allow_background_pool as u8) << 1)
        | ((defaults.prefer_local as u8) << 2)
        | ((defaults.allow_burst as u8) << 3)
        | ((defaults.reduce_on_battery as u8) << 4)
        | ((defaults.disable_pool_low_power as u8) << 5)
        | ((defaults.notify_expansion as u8) << 6)
        | ((defaults.notify_throttling as u8) << 7)
}

// ------------------------=
// FUNC: decode_mode
// DESC: Validates one persistent user-facing resource mode.
// ------------------=
fn decode_mode(value: u8) -> Result<ResourceMode, ResourcePolicyError> {
    match value {
        1 => Ok(ResourceMode::Restricted),
        2 => Ok(ResourceMode::Balanced),
        3 => Ok(ResourceMode::Expanded),
        4 => Ok(ResourceMode::Custom),
        _ => Err(ResourcePolicyError::InvalidPersistence),
    }
}

// ------------------------=
// FUNC: decode_locality
// DESC: Validates one persistent local or future Pool locality preference.
// ------------------=
fn decode_locality(value: u8) -> Result<LocalityPolicy, ResourcePolicyError> {
    match value {
        1 => Ok(LocalityPolicy::LocalOnly),
        2 => Ok(LocalityPolicy::LocalPreferred),
        3 => Ok(LocalityPolicy::PoolAllowed),
        4 => Ok(LocalityPolicy::PoolPreferred),
        5 => Ok(LocalityPolicy::PoolRequired),
        _ => Err(ResourcePolicyError::InvalidPersistence),
    }
}

// ------------------------=
// FUNC: decode_background
// DESC: Validates one persistent background throttling level.
// ------------------=
fn decode_background(value: u8) -> Result<BackgroundThrottle, ResourcePolicyError> {
    match value {
        0 => Ok(BackgroundThrottle::Aggressive),
        1 => Ok(BackgroundThrottle::Balanced),
        2 => Ok(BackgroundThrottle::Light),
        _ => Err(ResourcePolicyError::InvalidPersistence),
    }
}
