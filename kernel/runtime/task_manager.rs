use super::execution::{
    ContextHandle, ContextState, ExecutionError, ExecutionManager, MemoryRegion, PriorityClass,
    ResourceBudget, ResourceUsage,
};

pub const APPLICATION_SERVICE_ID: u32 = 0;
pub const IMAGE_FILE_NAVIGATOR: u32 = 0xa001;
pub const IMAGE_TEXT_EDITOR: u32 = 0xa002;
pub const IMAGE_COMMAND_WINDOW: u32 = 0xa003;
pub const IMAGE_TASK_MANAGER: u32 = 0xa004;
pub const DEFAULT_APP_MEMORY_LIMIT: u64 = 8 * 1024 * 1024;
pub const FILE_NAVIGATOR_INSTALLED_BYTES: u64 = 224 * 1024;
pub const TEXT_EDITOR_INSTALLED_BYTES: u64 = 176 * 1024;
pub const COMMAND_WINDOW_INSTALLED_BYTES: u64 = 128 * 1024;
pub const TASK_MANAGER_INSTALLED_BYTES: u64 = 208 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskManagerError {
    UnknownTask,
    UnknownApplication,
    ProtectedSystemTask,
    InvalidBudget,
    InvalidState,
    Runtime(ExecutionError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskSnapshot {
    pub handle: ContextHandle,
    pub service_identity: u32,
    pub image_identity: u32,
    pub state: ContextState,
    pub priority: PriorityClass,
    pub budget: ResourceBudget,
    pub usage: ResourceUsage,
    pub installed_bytes: u64,
    pub cpu_share_percent: u8,
}

#[derive(Clone, Copy)]
pub struct TaskManager {
    previous_handles: [u16; super::execution::MAX_CONTEXTS],
    previous_ticks: [u64; super::execution::MAX_CONTEXTS],
    cpu_share_percent: [u8; super::execution::MAX_CONTEXTS],
}

impl TaskManager {
    // ------------------------=
    // FUNC: new
    // DESC: Creates the authority adapter and its bounded scheduler telemetry sampler.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            previous_handles: [0; super::execution::MAX_CONTEXTS],
            previous_ticks: [0; super::execution::MAX_CONTEXTS],
            cpu_share_percent: [0; super::execution::MAX_CONTEXTS],
        }
    }

    // ------------------------=
    // FUNC: task_count
    // DESC: Reports every authoritative application and service execution context.
    // ------------------=
    pub fn task_count(&self, execution: &ExecutionManager) -> usize {
        execution.count()
    }

    // ------------------------=
    // FUNC: task_nth
    // DESC: Returns one typed monitoring snapshot without exposing mutable context storage.
    // ------------------=
    pub fn task_nth(&self, execution: &ExecutionManager, index: usize) -> Option<TaskSnapshot> {
        execution
            .nth(index)
            .map(|context| snapshot(context, self.cpu_share_percent[index]))
    }

    // ------------------------=
    // FUNC: inspect
    // DESC: Resolves one task handle to its current typed monitoring snapshot.
    // ------------------=
    pub fn inspect(
        &self,
        execution: &ExecutionManager,
        handle: ContextHandle,
    ) -> Result<TaskSnapshot, TaskManagerError> {
        for index in 0..execution.count() {
            if let Some(context) = execution.nth(index) {
                if context.handle == handle {
                    return Ok(snapshot(context, self.cpu_share_percent[index]));
                }
            }
        }
        Err(TaskManagerError::UnknownTask)
    }

    // ------------------------=
    // FUNC: sample_cpu
    // DESC: Samples authoritative scheduler tick deltas into per-task CPU-share percentages.
    // ------------------=
    pub fn sample_cpu(&mut self, execution: &ExecutionManager) {
        let mut deltas = [0u64; super::execution::MAX_CONTEXTS];
        let mut total = 0u64;
        for index in 0..execution.count() {
            if let Some(context) = execution.nth(index) {
                let delta = if self.previous_handles[index] == context.handle.0 {
                    context.usage.cpu_ticks.saturating_sub(self.previous_ticks[index])
                } else {
                    context.usage.cpu_ticks
                };
                deltas[index] = delta;
                total = total.saturating_add(delta);
                self.previous_handles[index] = context.handle.0;
                self.previous_ticks[index] = context.usage.cpu_ticks;
            }
        }
        for index in 0..super::execution::MAX_CONTEXTS {
            self.cpu_share_percent[index] = if total == 0 {
                0
            } else {
                deltas[index].saturating_mul(100).saturating_div(total).min(100) as u8
            };
        }
    }

    // ------------------------=
    // FUNC: launch
    // DESC: Creates a runnable isolated context for one installed application image.
    // ------------------=
    pub fn launch(
        &self,
        execution: &mut ExecutionManager,
        image_identity: u32,
    ) -> Result<ContextHandle, TaskManagerError> {
        if !is_application_image(image_identity) {
            return Err(TaskManagerError::UnknownApplication);
        }
        let slot = (0..super::execution::MAX_CONTEXTS)
            .find(|index| {
                let base = app_memory_base(*index);
                !(0..execution.count()).any(|task| {
                    execution
                        .nth(task)
                        .map(|context| context.memory.base == base)
                        .unwrap_or(false)
                })
            })
            .ok_or(TaskManagerError::Runtime(ExecutionError::Full))?;
        let handle = execution
            .create(
                APPLICATION_SERVICE_ID,
                image_identity,
                MemoryRegion {
                    base: app_memory_base(slot),
                    length: DEFAULT_APP_MEMORY_LIMIT,
                },
                0x7000u16.saturating_add(slot as u16),
                PriorityClass::Normal,
                ResourceBudget {
                    memory_limit: DEFAULT_APP_MEMORY_LIMIT,
                    cpu_weight: 100,
                    message_queue_limit: 32,
                    io_priority: 3,
                },
            )
            .map_err(TaskManagerError::Runtime)?;
        execution
            .set_state(handle, ContextState::Runnable)
            .map_err(TaskManagerError::Runtime)?;
        Ok(handle)
    }

    // ------------------------=
    // FUNC: end
    // DESC: Stops an application task while protecting system service contexts.
    // ------------------=
    pub fn end(
        &self,
        execution: &mut ExecutionManager,
        handle: ContextHandle,
    ) -> Result<(), TaskManagerError> {
        require_application(execution, handle)?;
        execution
            .set_state(handle, ContextState::Stopped)
            .map_err(TaskManagerError::Runtime)
    }

    // ------------------------=
    // FUNC: relaunch
    // DESC: Reissues application runtime identity and clears usage before returning it to Runnable.
    // ------------------=
    pub fn relaunch(
        &self,
        execution: &mut ExecutionManager,
        handle: ContextHandle,
    ) -> Result<(), TaskManagerError> {
        require_application(execution, handle)?;
        execution
            .relaunch(handle)
            .map_err(TaskManagerError::Runtime)
    }

    // ------------------------=
    // FUNC: pause
    // DESC: Moves a runnable application into an explicit waiting state.
    // ------------------=
    pub fn pause(
        &self,
        execution: &mut ExecutionManager,
        handle: ContextHandle,
    ) -> Result<(), TaskManagerError> {
        let task = require_application(execution, handle)?;
        if !matches!(task.state, ContextState::Runnable | ContextState::Running) {
            return Err(TaskManagerError::InvalidState);
        }
        execution
            .set_state(handle, ContextState::Waiting)
            .map_err(TaskManagerError::Runtime)
    }

    // ------------------------=
    // FUNC: resume
    // DESC: Returns a paused application to runnable scheduling.
    // ------------------=
    pub fn resume(
        &self,
        execution: &mut ExecutionManager,
        handle: ContextHandle,
    ) -> Result<(), TaskManagerError> {
        let task = require_application(execution, handle)?;
        if task.state != ContextState::Waiting {
            return Err(TaskManagerError::InvalidState);
        }
        execution
            .set_state(handle, ContextState::Runnable)
            .map_err(TaskManagerError::Runtime)
    }

    // ------------------------=
    // FUNC: throttle
    // DESC: Applies validated CPU, memory, queue, I/O, and priority constraints to an application.
    // ------------------=
    pub fn throttle(
        &self,
        execution: &mut ExecutionManager,
        handle: ContextHandle,
        budget: ResourceBudget,
        priority: PriorityClass,
    ) -> Result<(), TaskManagerError> {
        require_application(execution, handle)?;
        if budget.cpu_weight == 0
            || budget.cpu_weight > 1000
            || budget.memory_limit == 0
            || budget.message_queue_limit == 0
            || budget.io_priority > 7
        {
            return Err(TaskManagerError::InvalidBudget);
        }
        execution
            .update_budget(handle, budget, priority)
            .map_err(|error| match error {
                ExecutionError::BudgetExceeded => TaskManagerError::InvalidBudget,
                other => TaskManagerError::Runtime(other),
            })
    }
}

// ------------------------=
// FUNC: snapshot
// DESC: Copies observable execution state into a stable Task Manager row.
// ------------------=
fn snapshot(context: &super::execution::ExecutionContext, cpu_share_percent: u8) -> TaskSnapshot {
    TaskSnapshot {
        handle: context.handle,
        service_identity: context.service_identity,
        image_identity: context.image_identity,
        state: context.state,
        priority: context.priority,
        budget: context.budget,
        usage: context.usage,
        installed_bytes: installed_image_bytes(context.image_identity),
        cpu_share_percent,
    }
}

// ------------------------=
// FUNC: require_application
// DESC: Resolves a mutable lifecycle target while rejecting protected system work.
// ------------------=
fn require_application(
    execution: &ExecutionManager,
    handle: ContextHandle,
) -> Result<TaskSnapshot, TaskManagerError> {
    let context = execution.get(handle).ok_or(TaskManagerError::UnknownTask)?;
    if context.service_identity != APPLICATION_SERVICE_ID {
        return Err(TaskManagerError::ProtectedSystemTask);
    }
    Ok(snapshot(context, 0))
}

// ------------------------=
// FUNC: is_application_image
// DESC: Validates one installed native application image identity.
// ------------------=
pub const fn is_application_image(image: u32) -> bool {
    matches!(
        image,
        IMAGE_FILE_NAVIGATOR | IMAGE_TEXT_EDITOR | IMAGE_COMMAND_WINDOW | IMAGE_TASK_MANAGER
    )
}

// ------------------------=
// FUNC: installed_image_bytes
// DESC: Returns the registered on-disk footprint of one built-in application image.
// ------------------=
pub const fn installed_image_bytes(image: u32) -> u64 {
    match image {
        IMAGE_FILE_NAVIGATOR => FILE_NAVIGATOR_INSTALLED_BYTES,
        IMAGE_TEXT_EDITOR => TEXT_EDITOR_INSTALLED_BYTES,
        IMAGE_COMMAND_WINDOW => COMMAND_WINDOW_INSTALLED_BYTES,
        IMAGE_TASK_MANAGER => TASK_MANAGER_INSTALLED_BYTES,
        _ => 0,
    }
}

// ------------------------=
// FUNC: app_memory_base
// DESC: Reserves one disjoint fixed application address-space region per context slot.
// ------------------=
const fn app_memory_base(slot: usize) -> u64 {
    0x4000_0000 + slot as u64 * 0x0100_0000
}
