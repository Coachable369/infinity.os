//! Non-blocking UI operation state and cancellation tokens.

pub const MAX_ASYNC_TASKS: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TaskToken {
    pub id: u32,
    pub generation: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AsyncState {
    Idle,
    Pending { started_at: u64 },
    Ready { completed_at: u64 },
    Failed { error_type: u32 },
    Cancelled,
}

#[derive(Clone, Copy)]
pub struct AsyncTask {
    pub token: TaskToken,
    pub owner_element: super::input::ElementId,
    pub state: AsyncState,
    pub deadline: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AsyncError {
    Full,
    Unknown,
    Stale,
}

pub struct AsyncUiModel {
    tasks: [Option<AsyncTask>; MAX_ASYNC_TASKS],
    next_id: u32,
    generation: u32,
}

impl AsyncUiModel {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a bounded non-blocking task registry for UI service requests.
    // ------------------=
    pub const fn new() -> Self {
        Self { tasks: [None; MAX_ASYNC_TASKS], next_id: 1, generation: 1 }
    }

    // ------------------------=
    // FUNC: begin
    // DESC: Starts one cancellable asynchronous UI operation with an explicit deadline.
    // ------------------=
    pub fn begin(&mut self, owner_element: super::input::ElementId, now: u64, deadline: u64) -> Result<TaskToken, AsyncError> {
        let slot = self.tasks.iter_mut().find(|entry| entry.is_none()).ok_or(AsyncError::Full)?;
        let token = TaskToken { id: self.next_id, generation: self.generation };
        self.next_id = self.next_id.wrapping_add(1).max(1);
        *slot = Some(AsyncTask { token, owner_element, state: AsyncState::Pending { started_at: now }, deadline });
        Ok(token)
    }

    // ------------------------=
    // FUNC: complete
    // DESC: Completes only the current generation of a pending task so stale responses are ignored.
    // ------------------=
    pub fn complete(&mut self, token: TaskToken, now: u64, error_type: Option<u32>) -> Result<(), AsyncError> {
        let task = self.tasks.iter_mut().flatten().find(|task| task.token.id == token.id).ok_or(AsyncError::Unknown)?;
        if task.token != token {
            return Err(AsyncError::Stale);
        }
        task.state = error_type.map(|value| AsyncState::Failed { error_type: value }).unwrap_or(AsyncState::Ready { completed_at: now });
        Ok(())
    }

    // ------------------------=
    // FUNC: cancel_owner
    // DESC: Cancels outstanding work when its semantic element or window disappears.
    // ------------------=
    pub fn cancel_owner(&mut self, owner: super::input::ElementId) -> usize {
        let mut cancelled = 0;
        for task in self.tasks.iter_mut().flatten().filter(|task| task.owner_element == owner) {
            if matches!(task.state, AsyncState::Pending { .. }) {
                task.state = AsyncState::Cancelled;
                cancelled += 1;
            }
        }
        cancelled
    }
}
