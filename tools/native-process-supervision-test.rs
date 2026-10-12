#[path = "../kernel/runtime/execution.rs"] mod execution;
#[path = "../kernel/runtime/native_process.rs"] mod native_process;
use native_process::{Executor, Process, State, Trap};
use execution::SecurityIdentity;
use std::{cell::RefCell, rc::Rc};

struct Fake { events: Vec<Result<Trap, ()>>, visits: Rc<RefCell<usize>>, results: Rc<RefCell<Vec<i64>>> }
impl Executor for Fake {
    // ------------------------=
    // FUNC: enter
    // DESC: Supplies deterministic trap outcomes to test supervision independently of hardware.
    // ------------------=
    fn enter(&mut self) -> Result<Trap, ()> {
        let mut visits = self.visits.borrow_mut();
        let event = self.events.get(*visits).copied().unwrap_or(Ok(Trap::Preempted));
        *visits += 1; event
    }
    // ------------------------=
    // FUNC: result
    // DESC: Records ABI results without a user register frame.
    // ------------------=
    fn result(&mut self, result: i64) { self.results.borrow_mut().push(result); }
}

// ------------------------=
// FUNC: main
// DESC: Verifies bounded dispatch, trusted identity, exit, cancellation, deadlines, budget exhaustion and platform faults.
// ------------------=
fn main() {
    let owner = SecurityIdentity([7;16]);
    let visits = Rc::new(RefCell::new(0));
    let results = Rc::new(RefCell::new(Vec::new()));
    let fake = Fake { events: vec![Ok(Trap::Preempted), Ok(Trap::Call(0x1001, [0;6])),
        Ok(Trap::Call(0x120b, [9;6])), Ok(Trap::Call(0x1000, [54;6]))], visits: visits.clone(), results: results.clone() };
    let mut process = Process::new(fake, owner, 10, Some(100));
    let mut calls = 0;
    for tick in 0..4 {
        let state = process.step(tick, |identity, operation, args| {
            assert_eq!(identity, owner); assert_eq!(operation, 0x120b); assert_eq!(args, [9;6]); calls += 1; -13
        });
        assert_eq!(*visits.borrow(), tick as usize + 1);
        assert_eq!(state, if tick == 3 { State::Exited(54) } else { State::Runnable });
    }
    assert_eq!(calls, 1);
    assert_eq!(&*results.borrow(), &[0, -13]);
    assert_eq!(process.counters(), (4, 1));
    process.cancel();
    assert_eq!(process.step(200, |_, _, _| panic!("terminal dispatch")), State::Exited(54));
    assert_eq!(*visits.borrow(), 4);
    for (event, expected) in [(Err(()), State::PlatformFailure),
        (Ok(Trap::Fault(13, 4096, 8192)), State::Faulted(13, 4096, 8192))] {
        let visits = Rc::new(RefCell::new(0));
        let mut process = Process::new(Fake { events: vec![event], visits: visits.clone(), results: results.clone() }, owner, 2, None);
        assert_eq!(process.step(0, |_, _, _| panic!("fault dispatch")), expected);
        assert_eq!(process.step(1, |_, _, _| panic!("fault reentry")), expected);
        assert_eq!(*visits.borrow(), 1);
    }
    for mode in 0..3 {
        let visits = Rc::new(RefCell::new(0));
        let mut process = Process::new(Fake { events: vec![], visits: visits.clone(), results: results.clone() },
            owner, if mode == 0 { 0 } else { 1 }, if mode == 1 { Some(5) } else { None });
        if mode == 2 { process.cancel(); }
        assert_eq!(process.step(5, |_, _, _| panic!("denied dispatch")),
            [State::Budget, State::Deadline, State::Cancelled][mode]);
        assert_eq!(*visits.borrow(), 0);
    }
    let visits = Rc::new(RefCell::new(0));
    let mut process = Process::new(Fake { events: vec![], visits: visits.clone(), results }, owner, 2, None);
    assert_eq!(process.step(0, |_, _, _| -88), State::Runnable);
    assert_eq!(process.step(0, |_, _, _| -88), State::Runnable);
    assert_eq!(process.step(0, |_, _, _| -88), State::Budget);
    assert_eq!(*visits.borrow(), 2);
}
