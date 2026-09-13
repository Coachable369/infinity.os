use infinity_http::task::{Error, Task};
use std::{
    cell::Cell,
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

struct Pending {
    drops: Rc<Cell<usize>>,
    address: usize,
}
impl Future for Pending {
    type Output = usize;
    // ------------------------=
    // FUNC: poll
    // DESC: Verifies that a suspended future stays at the same address until completion.
    // ------------------=
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<usize> {
        let address = &*self as *const Self as usize;
        if self.address == 0 {
            self.address = address;
            Poll::Pending
        } else {
            assert_eq!(self.address, address);
            Poll::Ready(42)
        }
    }
}
impl Drop for Pending {
    // ------------------------=
    // FUNC: drop
    // DESC: Tracks exact resource release for rejected, cancelled and completed work.
    // ------------------=
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}
#[test]
// ------------------------=
// FUNC: pinned_task_completion_and_cancellation_release_once
// DESC: Exercises pending state, stable address, busy rejection, completion, reuse and repeated cancellation.
// ------------------=
fn pinned_task_completion_and_cancellation_release_once() {
    let drops = Rc::new(Cell::new(0));
    let mut task = core::pin::pin!(Task::<usize, 256>::new());
    task.as_mut()
        .start(Pending {
            drops: drops.clone(),
            address: 0,
        })
        .unwrap();
    assert_eq!(
        task.as_mut().start(Pending {
            drops: drops.clone(),
            address: 0
        }),
        Err(Error::Busy)
    );
    assert_eq!(drops.get(), 1);
    let mut cx = Context::from_waker(Waker::noop());
    assert!(task.as_mut().poll(&mut cx).is_pending());
    assert_eq!(task.as_mut().poll(&mut cx), Poll::Ready(42));
    assert!(!task.active());
    assert_eq!(drops.get(), 2);
    task.as_mut()
        .start(Pending {
            drops: drops.clone(),
            address: 0,
        })
        .unwrap();
    task.as_mut().cancel();
    task.as_mut().cancel();
    assert_eq!(drops.get(), 3);
}
#[test]
// ------------------------=
// FUNC: task_capacity_rejection_drops_uninstalled_future
// DESC: Rejects insufficient storage without leaving work installed or leaking its captured resources.
// ------------------=
fn task_capacity_rejection_drops_uninstalled_future() {
    let drops = Rc::new(Cell::new(0));
    let mut task = core::pin::pin!(Task::<usize, 1>::new());
    assert_eq!(
        task.as_mut().start(Pending {
            drops: drops.clone(),
            address: 0
        }),
        Err(Error::Capacity)
    );
    assert!(!task.active());
    assert_eq!(drops.get(), 1);
}
