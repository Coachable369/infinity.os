#![allow(dead_code)]

#[path = "../kernel/runtime/mod.rs"]
mod runtime;
#[path = "../kernel/ui/mod.rs"]
mod ui;

mod console {
    #[derive(Clone, Copy)]
    pub enum ConsoleKey {
        Character(u8),
        Backspace,
        Delete,
        Left,
        Right,
        Home,
        End,
        Enter,
    }
}

// ------------------------=
// FUNC: output_text
// DESC: Supplies the kernel diagnostic boundary required by the host behavior harness.
// ------------------=
fn output_text(_: &[u8]) {}

use runtime::capability::{CapabilityManager, CapabilityType};
use runtime::execution::{ExecutionManager, MemoryRegion, PriorityClass, ResourceBudget, SecurityIdentity};
use runtime::resource_policy::{
    AppId, ApplicationManifestRequest, ApplicationPolicyOverride, ApplicationResourceManager,
    ApplicationResourceUsage, BackgroundThrottle, EnforcementState, GlobalResourceDefaults,
    LocalityPolicy, PolicySource, ResourceClass, ResourceMode, ResourcePolicyError,
    SystemCapacity, SystemConditions,
};

// ------------------------=
// FUNC: identity
// DESC: Creates a deterministic machine identity for authority tests.
// ------------------=
fn identity(value: u8) -> SecurityIdentity {
    SecurityIdentity([value; 16])
}

// ------------------------=
// FUNC: capacity
// DESC: Provides observed local resources without claiming GPU or NPU availability.
// ------------------=
fn capacity() -> SystemCapacity {
    SystemCapacity {
        logical_compute_units: 8,
        memory_bytes: 16 * 1024 * 1024,
        gpu_available: false,
        npu_available: false,
    }
}

// ------------------------=
// FUNC: conditions
// DESC: Produces an ordinary foreground power and thermal state.
// ------------------=
fn conditions() -> SystemConditions {
    SystemConditions {
        on_battery: false,
        low_power: false,
        thermal_pressure: false,
        foreground: true,
    }
}

// ------------------------=
// FUNC: custom_override
// DESC: Creates one explicit AppId-bound policy for persistence and enforcement tests.
// ------------------=
fn custom_override(app_id: AppId, mode: ResourceMode) -> ApplicationPolicyOverride {
    ApplicationPolicyOverride {
        app_id,
        mode,
        cpu_maximum: 3,
        cpu_weight: if mode == ResourceMode::Restricted { 25 } else { 180 },
        memory_hard_limit: 3 * 1024 * 1024,
        message_queue_limit: 9,
        storage_io_priority: 2,
        network_priority: 2,
        background_allowed: true,
        burst_allowed: mode != ResourceMode::Restricted,
        locality: LocalityPolicy::LocalOnly,
    }
}

// ------------------------=
// FUNC: main
// DESC: Verifies policy precedence, capability authority, enforcement, leases, accounting, and persistence behavior.
// ------------------=
fn main() {
    let app = AppId(runtime::task_manager::IMAGE_FILE_NAVIGATOR);
    let caller = identity(7);
    let issuer = identity(1);
    let mut capabilities = CapabilityManager::new();
    let modify = capabilities
        .grant(
            CapabilityType::ResourcePolicyModify,
            app.0 as u64,
            1,
            0,
            issuer,
            caller,
            None,
            0,
        )
        .unwrap();
    let modify_defaults = capabilities
        .grant(
            CapabilityType::ResourcePolicyModify,
            0,
            1,
            0,
            issuer,
            caller,
            None,
            0,
        )
        .unwrap();
    let use_cpu = capabilities
        .grant(
            CapabilityType::ResourceUse,
            app.0 as u64,
            1,
            ResourceClass::Cpu as u64,
            issuer,
            caller,
            Some(100),
            0,
        )
        .unwrap();
    let mut manager = ApplicationResourceManager::new();
    assert_eq!(manager.defaults().mode, ResourceMode::Balanced);
    assert!(!manager.defaults().allow_pool);

    let manifest = ApplicationManifestRequest::balanced();
    let inherited = manager
        .effective_policy(app, manifest, capacity(), conditions())
        .unwrap();
    assert_eq!(inherited.mode, ResourceMode::Balanced);
    assert!(inherited.inherited);
    assert_eq!(inherited.memory_enforcement, EnforcementState::Enforced);
    assert_eq!(inherited.pool_enforcement, EnforcementState::Enforced);
    assert_eq!(inherited.gpu_enforcement, EnforcementState::Unavailable);

    assert_eq!(
        manager.set_override(custom_override(app, ResourceMode::Restricted), issuer, modify, &capabilities, 1),
        Err(ResourcePolicyError::AccessDenied)
    );
    manager
        .set_override(custom_override(app, ResourceMode::Restricted), caller, modify, &capabilities, 1)
        .unwrap();
    let restricted = manager
        .effective_policy(app, manifest, capacity(), conditions())
        .unwrap();
    assert_eq!(restricted.mode, ResourceMode::Restricted);
    assert!(!restricted.inherited);
    assert_eq!(restricted.limit_source, PolicySource::ApplicationOverride);
    assert_eq!(restricted.cpu_weight, 25);
    assert!(!restricted.burst_allowed);

    let mut execution = ExecutionManager::new();
    let handle = execution
        .create(
            0,
            app.0,
            MemoryRegion { base: 0x100000, length: 8 * 1024 * 1024 },
            0x7000,
            PriorityClass::Normal,
            ResourceBudget { memory_limit: 8 * 1024 * 1024, cpu_weight: 100, message_queue_limit: 32, io_priority: 3 },
        )
        .unwrap();
    manager.apply_to_context(&mut execution, handle, restricted).unwrap();
    assert_eq!(execution.get(handle).unwrap().budget, restricted.runtime_budget());
    assert_eq!(
        execution.account_memory(handle, restricted.memory_hard_limit + 1),
        Err(runtime::execution::ExecutionError::BudgetExceeded)
    );

    let broad_manifest = ApplicationManifestRequest {
        cpu_maximum: 64,
        memory_maximum: 128 * 1024 * 1024,
        locality: LocalityPolicy::PoolRequired,
        ..manifest
    };
    let still_restricted = manager
        .effective_policy(app, broad_manifest, capacity(), conditions())
        .unwrap();
    assert_eq!(still_restricted.cpu_maximum, 1);
    assert_eq!(still_restricted.memory_hard_limit, 3 * 1024 * 1024);
    assert_eq!(still_restricted.locality, LocalityPolicy::LocalOnly);

    let narrow_manifest = ApplicationManifestRequest {
        cpu_maximum: 1,
        memory_preferred: 1024 * 1024,
        memory_maximum: 1024 * 1024,
        ..manifest
    };
    let narrowed = manager
        .effective_policy(AppId(runtime::task_manager::IMAGE_TEXT_EDITOR), narrow_manifest, capacity(), conditions())
        .unwrap();
    assert_eq!(narrowed.cpu_maximum, 1);
    assert_eq!(narrowed.memory_hard_limit, 1024 * 1024);
    assert_eq!(narrowed.limit_source, PolicySource::ManifestRequest);

    let low_power = manager
        .effective_policy(
            app,
            manifest,
            capacity(),
            SystemConditions { on_battery: true, low_power: true, thermal_pressure: false, foreground: false },
        )
        .unwrap();
    assert!(!low_power.burst_allowed);
    assert!(low_power.cpu_weight <= 40);
    assert_eq!(low_power.limit_source, PolicySource::PowerOverlay);

    manager
        .account(
            app,
            ApplicationResourceUsage { memory_bytes: restricted.memory_hard_limit, background: true, ..ApplicationResourceUsage::default() },
            restricted,
        )
        .unwrap();
    assert!(manager.usage_for(app).unwrap().throttled);

    let lease = manager
        .request_lease(app, ResourceClass::Cpu, 1, 10, restricted, caller, use_cpu, &capabilities, 5)
        .unwrap();
    assert_eq!(manager.inspect_lease(lease.lease_id, 6).unwrap().app_id, app);
    assert_eq!(
        manager.request_lease(app, ResourceClass::Cpu, 99, 10, restricted, caller, use_cpu, &capabilities, 5),
        Err(ResourcePolicyError::LimitExceeded)
    );
    assert_eq!(
        manager.request_lease(app, ResourceClass::RemotePool, 1, 10, restricted, caller, use_cpu, &capabilities, 5),
        Err(ResourcePolicyError::Unsupported)
    );
    manager.revoke_lease(lease.lease_id).unwrap();
    assert_eq!(manager.inspect_lease(lease.lease_id, 6), Err(ResourcePolicyError::RevokedLease));

    let mut defaults = GlobalResourceDefaults::balanced();
    defaults.mode = ResourceMode::Restricted;
    defaults.background_throttle = BackgroundThrottle::Aggressive;
    manager.update_defaults(defaults, caller, modify_defaults, &capabilities, 8).unwrap();
    let newly_inherited = manager
        .effective_policy(AppId(runtime::task_manager::IMAGE_COMMAND_WINDOW), manifest, capacity(), conditions())
        .unwrap();
    assert_eq!(newly_inherited.mode, ResourceMode::Restricted);
    assert_eq!(manager.override_for(app).unwrap().mode, ResourceMode::Restricted);

    let bytes = manager.encode();
    let restored = ApplicationResourceManager::decode(&bytes).unwrap();
    assert_eq!(restored.defaults(), defaults);
    assert_eq!(restored.override_for(app), manager.override_for(app));
    assert!(matches!(
        ApplicationResourceManager::decode(&bytes[..511]),
        Err(ResourcePolicyError::InvalidPersistence)
    ));
}
