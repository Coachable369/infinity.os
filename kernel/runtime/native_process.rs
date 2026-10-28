//! One bounded user slice per scheduler visit. Hardware entry and services remain
//! injected native adapters; terminal processes can never re-enter user mode.
use super::execution::SecurityIdentity;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trap {
    Preempted,
    Call(u64, [u64; 6]),
    Fault(u64, u64, u64),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Runnable,
    Exited(i64),
    Faulted(u64, u64, u64),
    Cancelled,
    Deadline,
    Budget,
    PlatformFailure,
}

pub trait Executor {
    // ------------------------=
    // FUNC: enter
    // DESC: Executes at most one hardware-timer-bounded user slice and returns its trap.
    // ------------------=
    fn enter(&mut self) -> Result<Trap, ()>;
    // ------------------------=
    // FUNC: result
    // DESC: Supplies a service result to the private user register frame.
    // ------------------=
    fn result(&mut self, value: i64);
}

pub struct Process<E: Executor> {
    executor: E,
    owner: SecurityIdentity,
    state: State,
    remaining: u64,
    deadline: Option<u64>,
    visits: u64,
    calls: u64,
}

impl<E: Executor> Process<E> {
    // ------------------------=
    // FUNC: new
    // DESC: Binds process identity and finite scheduling budget from the trusted launch context.
    // ------------------=
    pub fn new(executor: E, owner: SecurityIdentity, slices: u64, deadline: Option<u64>) -> Self {
        Self {
            executor,
            owner,
            state: State::Runnable,
            remaining: slices,
            deadline,
            visits: 0,
            calls: 0,
        }
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Stops future execution without changing an already recorded terminal outcome.
    // ------------------=
    pub fn cancel(&mut self) {
        if self.state == State::Runnable {
            self.state = State::Cancelled;
        }
    }
    // ------------------------=
    // FUNC: counters
    // DESC: Reports actual hardware visits and dispatched service calls for resource accounting.
    // ------------------=
    pub fn counters(&self) -> (u64, u64) {
        (self.visits, self.calls)
    }
    // ------------------------=
    // FUNC: step
    // DESC: Executes one slice, handles lifecycle traps locally, and dispatches one service with trusted process identity.
    // ------------------=
    pub fn step<F: FnMut(SecurityIdentity, u64, [u64; 6]) -> i64>(
        &mut self,
        now: u64,
        mut service: F,
    ) -> State {
        if self.state != State::Runnable {
            return self.state;
        }
        if self.deadline.is_some_and(|deadline| now >= deadline) {
            self.state = State::Deadline;
            return self.state;
        }
        if self.remaining == 0 {
            self.state = State::Budget;
            return self.state;
        }
        self.remaining -= 1;
        self.visits += 1;
        match self.executor.enter() {
            Err(()) => self.state = State::PlatformFailure,
            Ok(Trap::Fault(code, address, pc)) => self.state = State::Faulted(code, address, pc),
            Ok(Trap::Preempted) => (),
            Ok(Trap::Call(0x1000, args)) => self.state = State::Exited(args[0] as i64),
            Ok(Trap::Call(0x1001, _)) => self.executor.result(0),
            Ok(Trap::Call(operation, args)) => {
                self.calls += 1;
                self.executor.result(service(self.owner, operation, args));
            }
        }
        self.state
    }
}
