use super::execution::{
    ContextHandle, ExecutionManager, MemoryRegion, PriorityClass, ResourceBudget,
};

pub const MAX_SERVICES: usize = 35;
pub const MAX_DEPENDENCIES: usize = 4;
pub const MAX_OPERATIONS: usize = 24;
pub const SERVICE_RUNTIME: u32 = 1;
pub const SERVICE_DEVICE: u32 = 2;
pub const SERVICE_STORAGE: u32 = 3;
pub const SERVICE_OBJECT: u32 = 4;
pub const SERVICE_NAMESPACE: u32 = 5;
pub const SERVICE_EVENT: u32 = 6;
pub const SERVICE_CONSOLE: u32 = 7;
pub const SERVICE_INSTALLER: u32 = 8;
pub const SERVICE_LOCAL_ML: u32 = 9;
pub const SERVICE_AI: u32 = 10;
pub const SERVICE_VOICE: u32 = 11;
pub const SERVICE_AGENT: u32 = 12;
pub const SERVICE_ORGANIZATION: u32 = 13;
pub const SERVICE_IDENTITY: u32 = 14;
pub const SERVICE_AUTHENTICATION: u32 = 15;
pub const SERVICE_SESSION: u32 = 16;
pub const SERVICE_SETTINGS: u32 = 17;
pub const SERVICE_ONBOARDING: u32 = 18;
pub const SERVICE_SHELL: u32 = 19;
pub const SERVICE_FONT: u32 = 20;
pub const SERVICE_INFINITY_UI: u32 = 21;
pub const SERVICE_SKIN_REGISTRY: u32 = 22;
pub const SERVICE_WINDOW_SERVER: u32 = 23;
pub const SERVICE_CLIPBOARD: u32 = 24;
pub const SERVICE_NETWORK: u32 = 25;
pub const SERVICE_NETWORK_POLICY: u32 = 26;
pub const SERVICE_NETWORK_TRANSPORT: u32 = 27;
pub const SERVICE_NETWORK_DISCOVERY: u32 = 28;
pub const SERVICE_CRYPTO: u32 = 29;
pub const SERVICE_NODE_IDENTITY: u32 = 30;
pub const SERVICE_NODE_DISCOVERY: u32 = 31;
pub const SERVICE_NODE_TRUST: u32 = 32;
pub const SERVICE_MESH: u32 = 33;
pub const SERVICE_NODE_AUDIT: u32 = 34;
pub const SERVICE_REPLICA_STORAGE: u32 = 35;
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ServiceState {
    Defined,
    Starting,
    Ready,
    Degraded,
    Stopping,
    Stopped,
    Failed,
    Restarting,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Criticality {
    Critical,
    Important,
    NonCritical,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RestartPolicy {
    Never,
    OnFailure,
    Always,
    BoundedRetry { maximum: u8 },
}
#[derive(Clone, Copy)]
pub struct ServiceManifest {
    pub format_version: u16,
    pub service_id: u32,
    pub service_type: u32,
    pub image_id: u32,
    pub version: u16,
    pub dependencies: [u32; MAX_DEPENDENCIES],
    pub dependency_count: u8,
    pub operations: [u32; MAX_OPERATIONS],
    pub operation_count: u8,
    pub required_capability_types: u64,
    pub resources: ResourceBudget,
    pub restart_policy: RestartPolicy,
    pub criticality: Criticality,
}
#[derive(Clone, Copy)]
pub struct ServiceInstance {
    pub manifest: ServiceManifest,
    pub state: ServiceState,
    pub context: Option<ContextHandle>,
    pub restart_attempts: u8,
    pub restart_at: u64,
    pub ready_at: u64,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ServiceError {
    Full,
    Duplicate,
    Unknown,
    DependencyCycle,
    DependencyUnavailable,
    ContextFailure,
    NotRestartable,
}
pub struct ServiceManager {
    services: [Option<ServiceInstance>; MAX_SERVICES],
    degraded: bool,
}
impl ServiceManager {
    // ------------------------=
    // FUNC: new
    // DESC: Creates and initializes a new instance.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            services: [None; MAX_SERVICES],
            degraded: false,
        }
    }
    // ------------------------=
    // FUNC: define
    // DESC: Implements the define operation.
    // ------------------=
    pub fn define(&mut self, manifest: ServiceManifest) -> Result<(), ServiceError> {
        if self
            .services
            .iter()
            .flatten()
            .any(|s| s.manifest.service_id == manifest.service_id)
        {
            return Err(ServiceError::Duplicate);
        }
        let slot = self
            .services
            .iter()
            .position(Option::is_none)
            .ok_or(ServiceError::Full)?;
        self.services[slot] = Some(ServiceInstance {
            manifest,
            state: ServiceState::Defined,
            context: None,
            restart_attempts: 0,
            restart_at: 0,
            ready_at: 0,
        });
        if self.has_cycle() {
            self.services[slot] = None;
            return Err(ServiceError::DependencyCycle);
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: has_cycle
    // DESC: Reports whether has cycle.
    // ------------------=
    fn has_cycle(&self) -> bool {
        for service in self.services.iter().flatten() {
            let mut stack = [0u32; MAX_SERVICES];
            let mut len = 1;
            stack[0] = service.manifest.service_id;
            for _ in 0..MAX_SERVICES {
                let current = stack[len - 1];
                let Some(node) = self.inspect(current) else {
                    break;
                };
                let mut advanced = false;
                for dep in &node.manifest.dependencies[..node.manifest.dependency_count as usize] {
                    if *dep == service.manifest.service_id {
                        return true;
                    }
                    if !stack[..len].contains(dep) && len < stack.len() {
                        stack[len] = *dep;
                        len += 1;
                        advanced = true;
                        break;
                    }
                }
                if !advanced {
                    break;
                }
            }
        }
        false
    }
    // ------------------------=
    // FUNC: dependencies_ready
    // DESC: Implements the dependencies ready operation.
    // ------------------=
    fn dependencies_ready(&self, id: u32) -> bool {
        let Some(service) = self.inspect(id) else {
            return false;
        };
        service.manifest.dependencies[..service.manifest.dependency_count as usize]
            .iter()
            .all(|dep| {
                self.inspect(*dep)
                    .map(|d| d.state == ServiceState::Ready)
                    .unwrap_or(false)
            })
    }
    // ------------------------=
    // FUNC: start_ready
    // DESC: Initializes start ready state.
    // ------------------=
    pub fn start_ready(&mut self, contexts: &mut ExecutionManager, now: u64) -> usize {
        let mut started = 0;
        loop {
            let mut changed = false;
            for index in 0..self.services.len() {
                let Some(instance) = self.services[index] else {
                    continue;
                };
                if !matches!(
                    instance.state,
                    ServiceState::Defined | ServiceState::Stopped | ServiceState::Restarting
                ) || !self.dependencies_ready(instance.manifest.service_id)
                    || now < instance.restart_at
                {
                    continue;
                }
                let memory = MemoryRegion {
                    base: 0x1000_0000 + instance.manifest.service_id as u64 * 0x0100_0000,
                    length: instance.manifest.resources.memory_limit,
                };
                match contexts.create(
                    instance.manifest.service_id,
                    instance.manifest.image_id,
                    memory,
                    instance.manifest.service_id as u16,
                    priority(instance.manifest.criticality),
                    instance.manifest.resources,
                ) {
                    Ok(handle) => {
                        let service = self.services[index].as_mut().unwrap();
                        service.context = Some(handle);
                        service.state = ServiceState::Starting;
                        service.ready_at = now;
                        started += 1;
                        changed = true
                    }
                    Err(_) => self.services[index].as_mut().unwrap().state = ServiceState::Failed,
                }
            }
            if !changed {
                break;
            }
        }
        started
    }
    // ------------------------=
    // FUNC: announce_ready
    // DESC: Implements the announce ready operation.
    // ------------------=
    pub fn announce_ready(&mut self, id: u32) -> Result<(), ServiceError> {
        let service = self.inspect_mut(id).ok_or(ServiceError::Unknown)?;
        if service.state != ServiceState::Starting {
            return Err(ServiceError::DependencyUnavailable);
        }
        service.state = ServiceState::Ready;
        service.restart_attempts = 0;
        Ok(())
    }
    // ------------------------=
    // FUNC: stop
    // DESC: Implements the stop operation.
    // ------------------=
    pub fn stop(&mut self, id: u32, contexts: &mut ExecutionManager) -> Result<(), ServiceError> {
        let service = self.inspect_mut(id).ok_or(ServiceError::Unknown)?;
        service.state = ServiceState::Stopping;
        if let Some(context) = service.context.take() {
            let _ = contexts.destroy(context);
        }
        service.state = ServiceState::Stopped;
        Ok(())
    }
    // ------------------------=
    // FUNC: fail
    // DESC: Implements the fail operation.
    // ------------------=
    pub fn fail(
        &mut self,
        id: u32,
        contexts: &mut ExecutionManager,
        now: u64,
    ) -> Result<(), ServiceError> {
        let critical;
        let retry;
        {
            let service = self.inspect_mut(id).ok_or(ServiceError::Unknown)?;
            service.state = ServiceState::Failed;
            if let Some(context) = service.context.take() {
                let _ = contexts.fail(context);
                let _ = contexts.destroy(context);
            }
            critical = service.manifest.criticality == Criticality::Critical;
            retry = match service.manifest.restart_policy {
                RestartPolicy::Never => false,
                RestartPolicy::OnFailure | RestartPolicy::Always => true,
                RestartPolicy::BoundedRetry { maximum } => service.restart_attempts < maximum,
            };
            if retry {
                service.restart_attempts = service.restart_attempts.saturating_add(1);
                let shift = service.restart_attempts.saturating_sub(1).min(5);
                service.restart_at = now.saturating_add(1000u64 << shift);
                service.state = ServiceState::Restarting;
            }
        }
        if critical {
            self.degraded = true;
        }
        if retry {
            Ok(())
        } else {
            Err(ServiceError::NotRestartable)
        }
    }
    // ------------------------=
    // FUNC: restart
    // DESC: Implements the restart operation.
    // ------------------=
    pub fn restart(
        &mut self,
        id: u32,
        contexts: &mut ExecutionManager,
        now: u64,
    ) -> Result<(), ServiceError> {
        self.stop(id, contexts)?;
        let service = self.inspect_mut(id).unwrap();
        service.state = ServiceState::Restarting;
        service.restart_at = now;
        Ok(())
    }
    // ------------------------=
    // FUNC: provider
    // DESC: Implements the provider operation.
    // ------------------=
    pub fn provider(&self, operation: u32) -> Option<u32> {
        self.services
            .iter()
            .flatten()
            .find(|s| {
                s.state == ServiceState::Ready
                    && s.manifest.operations[..s.manifest.operation_count as usize]
                        .contains(&operation)
            })
            .map(|s| s.manifest.service_id)
    }
    // ------------------------=
    // FUNC: inspect
    // DESC: Implements the inspect operation.
    // ------------------=
    pub fn inspect(&self, id: u32) -> Option<&ServiceInstance> {
        self.services
            .iter()
            .flatten()
            .find(|s| s.manifest.service_id == id)
    }
    // ------------------------=
    // FUNC: inspect_mut
    // DESC: Reads inspect mut data.
    // ------------------=
    fn inspect_mut(&mut self, id: u32) -> Option<&mut ServiceInstance> {
        self.services
            .iter_mut()
            .flatten()
            .find(|s| s.manifest.service_id == id)
    }
    // ------------------------=
    // FUNC: nth
    // DESC: Calculates and returns nth.
    // ------------------=
    pub fn nth(&self, index: usize) -> Option<&ServiceInstance> {
        self.services.iter().flatten().nth(index)
    }
    // ------------------------=
    // FUNC: count
    // DESC: Implements the count operation.
    // ------------------=
    pub fn count(&self) -> usize {
        self.services.iter().flatten().count()
    }
    // ------------------------=
    // FUNC: degraded
    // DESC: Implements the degraded operation.
    // ------------------=
    pub fn degraded(&self) -> bool {
        self.degraded
    }
}
// ------------------------=
// FUNC: priority
// DESC: Implements the priority operation.
// ------------------=
fn priority(criticality: Criticality) -> PriorityClass {
    match criticality {
        Criticality::Critical => PriorityClass::Critical,
        Criticality::Important => PriorityClass::System,
        Criticality::NonCritical => PriorityClass::Normal,
    }
}

// ------------------------=
// FUNC: manifest
// DESC: Implements the manifest operation.
// ------------------=
pub const fn manifest<const N: usize>(
    service_id: u32,
    dependencies: [u32; MAX_DEPENDENCIES],
    dependency_count: u8,
    operations: [u32; N],
    operation_count: u8,
    restart_policy: RestartPolicy,
    criticality: Criticality,
) -> ServiceManifest {
    assert!(N <= MAX_OPERATIONS && operation_count as usize <= N);
    let mut registered = [0; MAX_OPERATIONS];
    let mut index = 0;
    while index < N { registered[index] = operations[index]; index += 1; }
    ServiceManifest {
        format_version: 1,
        service_id,
        service_type: service_id,
        image_id: service_id,
        version: 1,
        dependencies,
        dependency_count,
        operations: registered,
        operation_count,
        required_capability_types: 0,
        resources: ResourceBudget {
            memory_limit: 1024 * 1024,
            cpu_weight: 100,
            message_queue_limit: 8,
            io_priority: 1,
        },
        restart_policy,
        criticality,
    }
}
