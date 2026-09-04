use super::execution::{ContextHandle, ContextState, ExecutionManager, PriorityClass};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WakeReason {
    Message,
    Timer,
    DependencyReady,
}

pub struct Scheduler {
    cursor: usize,
    priority_phase: usize,
    idle_ticks: u64,
}

impl Scheduler {
    // ------------------------=
    // FUNC: new
    // DESC: Creates and initializes a new instance.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            cursor: 0,
            priority_phase: 0,
            idle_ticks: 0,
        }
    }
    // ------------------------=
    // FUNC: next
    // DESC: Implements the next operation.
    // ------------------=
    pub fn next(&mut self, contexts: &mut ExecutionManager) -> Option<ContextHandle> {
        let count = contexts.count();
        if count == 0 {
            self.idle_ticks = self.idle_ticks.saturating_add(1);
            return None;
        }
        // Weighted class rotation gives system work more opportunities without
        // starving normal/background contexts.
        const CLASSES: [PriorityClass; 8] = [
            PriorityClass::Critical,
            PriorityClass::System,
            PriorityClass::Normal,
            PriorityClass::Critical,
            PriorityClass::System,
            PriorityClass::Normal,
            PriorityClass::Background,
            PriorityClass::Normal,
        ];
        for class_offset in 0..CLASSES.len() {
            let priority = CLASSES[(self.priority_phase + class_offset) % CLASSES.len()];
            for offset in 0..count {
                let index = (self.cursor + offset) % count;
                if let Some(context) = contexts.nth(index) {
                    if context.state == ContextState::Runnable && context.priority == priority {
                        let handle = context.handle;
                        self.cursor = (index + 1) % count;
                        self.priority_phase = (self.priority_phase + 1) % CLASSES.len();
                        let _ = contexts.set_state(handle, ContextState::Running);
                        let _ = contexts.account_cpu_tick(handle);
                        return Some(handle);
                    }
                }
            }
        }
        self.idle_ticks = self.idle_ticks.saturating_add(1);
        None
    }
    // ------------------------=
    // FUNC: yield_context
    // DESC: Implements the yield context operation.
    // ------------------=
    pub fn yield_context(&mut self, contexts: &mut ExecutionManager, handle: ContextHandle) {
        let _ = contexts.set_state(handle, ContextState::Runnable);
    }
    // ------------------------=
    // FUNC: block
    // DESC: Implements the block operation.
    // ------------------=
    pub fn block(&mut self, contexts: &mut ExecutionManager, handle: ContextHandle) {
        let _ = contexts.set_state(handle, ContextState::Waiting);
    }
    // ------------------------=
    // FUNC: wake
    // DESC: Implements the wake operation.
    // ------------------=
    pub fn wake(
        &mut self,
        contexts: &mut ExecutionManager,
        handle: ContextHandle,
        _reason: WakeReason,
    ) {
        if contexts
            .get(handle)
            .map(|c| c.state == ContextState::Waiting)
            .unwrap_or(false)
        {
            let _ = contexts.set_state(handle, ContextState::Runnable);
        }
    }
    // ------------------------=
    // FUNC: idle_ticks
    // DESC: Implements the idle ticks operation.
    // ------------------=
    pub fn idle_ticks(&self) -> u64 {
        self.idle_ticks
    }
}
