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
    credits: [(u16, i64); super::execution::MAX_CONTEXTS],
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
            credits: [(0, 0); super::execution::MAX_CONTEXTS],
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
            let mut selected = None;
            let mut best = i64::MIN;
            let mut first_distance = usize::MAX;
            let mut total = 0;
            for (slot, context) in contexts.slots() {
                if context.priority != priority || context.state != ContextState::Runnable { continue; }
                let credit = &mut self.credits[slot];
                if credit.0 != context.handle.0 { *credit = (context.handle.0, 0); }
                // Task Manager weights apply only to applications. System services retain equal shares in their class.
                let weight = if context.service_identity == super::task_manager::APPLICATION_SERVICE_ID {
                    i64::from(context.budget.cpu_weight.max(1))
                } else { 100 };
                credit.1 += weight;
                total += weight;
                let distance = (slot + super::execution::MAX_CONTEXTS - self.cursor) % super::execution::MAX_CONTEXTS;
                if credit.1 > best || (credit.1 == best && distance < first_distance) {
                    best = credit.1;
                    first_distance = distance;
                    selected = Some((slot, context.handle));
                }
            }
            if let Some((slot, handle)) = selected {
                self.credits[slot].1 -= total;
                self.cursor = (slot + 1) % super::execution::MAX_CONTEXTS;
                self.priority_phase = (self.priority_phase + 1) % CLASSES.len();
                let _ = contexts.set_state(handle, ContextState::Running);
                let _ = contexts.account_cpu_tick(handle);
                return Some(handle);
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
