//! Fixed-capacity, nonblocking cross-CPU messages for the native browser worker.
//! Contention is reported, never spun on from the desktop input/render path.
use core::{cell::UnsafeCell, mem::MaybeUninit, sync::atomic::{AtomicBool, Ordering}};

#[derive(Debug, PartialEq, Eq)]
pub enum SendError<T> { Busy(T), Full(T) }
#[derive(Debug, PartialEq, Eq)]
pub struct Busy;
struct State<T: Copy, const N: usize> { slots: [MaybeUninit<T>; N], head: usize, length: usize }
pub struct Mailbox<T: Copy, const N: usize> { locked: AtomicBool, state: UnsafeCell<State<T,N>> }
// Every access to state is protected by one successful acquire; release makes
// fully initialized message copies visible. T cannot contain a destructor and
// Send requires its contents to be transferable between the two native CPUs.
unsafe impl<T: Copy + Send, const N: usize> Sync for Mailbox<T,N> {}
struct Guard<'a>(&'a AtomicBool);
impl Drop for Guard<'_> {
    // ------------------------=
    // FUNC: drop
    // DESC: Publishes queue mutations and relinquishes ownership without spinning.
    // ------------------=
    fn drop(&mut self) { self.0.store(false,Ordering::Release); }
}
impl<T: Copy, const N: usize> Mailbox<T,N> {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty allocation-free mailbox with compile-time capacity.
    // ------------------=
    pub const fn new() -> Self {
        Self { locked:AtomicBool::new(false),state:UnsafeCell::new(State {
            slots:[const {MaybeUninit::uninit()};N],head:0,length:0,
        }) }
    }
    // ------------------------=
    // FUNC: acquire
    // DESC: Tries exactly once so a busy worker cannot stall the desktop.
    // ------------------=
    fn acquire(&self)->Result<Guard<'_>,Busy> {
        self.locked.compare_exchange(false,true,Ordering::Acquire,Ordering::Relaxed)
            .map(|_|Guard(&self.locked)).map_err(|_|Busy)
    }
    // ------------------------=
    // FUNC: try_send
    // DESC: Publishes one complete message or returns it intact for later retry.
    // ------------------=
    pub fn try_send(&self,value:T)->Result<(),SendError<T>> {
        let _guard=self.acquire().map_err(|_|SendError::Busy(value))?;
        let state=unsafe {&mut *self.state.get()};
        if state.length==N {return Err(SendError::Full(value));}
        let until_wrap=N-state.head;
        let index=if state.length>=until_wrap {state.length-until_wrap}else{state.head+state.length};
        state.slots[index].write(value);state.length+=1;Ok(())
    }
    // ------------------------=
    // FUNC: try_take
    // DESC: Copies out the oldest accepted message without lending shared storage.
    // ------------------=
    pub fn try_take(&self)->Result<Option<T>,Busy> {
        let _guard=self.acquire()?;
        let state=unsafe {&mut *self.state.get()};
        if state.length==0 {return Ok(None);}
        let value=unsafe {state.slots[state.head].assume_init()};
        state.head=(state.head+1)%N;state.length-=1;Ok(Some(value))
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    // ------------------------=
    // FUNC: bounded_wraparound_and_contention_preserve_messages
    // DESC: Verifies backpressure, exact ordering and single-attempt contention behavior.
    // ------------------=
    #[test]
    fn bounded_wraparound_and_contention_preserve_messages() {
        let queue=Mailbox::<u32,3>::new();
        for round in 0..20 {
            for value in 0..3 {assert_eq!(queue.try_send(round*3+value),Ok(()));}
            assert_eq!(queue.try_send(99),Err(SendError::Full(99)));
            queue.locked.store(true,Ordering::Relaxed);
            assert_eq!(queue.try_send(100),Err(SendError::Busy(100)));
            assert_eq!(queue.try_take(),Err(Busy));
            queue.locked.store(false,Ordering::Release);
            for value in 0..3 {assert_eq!(queue.try_take(),Ok(Some(round*3+value)));}
            assert_eq!(queue.try_take(),Ok(None));
        }
        let empty=Mailbox::<u32,0>::new();
        assert_eq!(empty.try_send(1),Err(SendError::Full(1)));
        assert_eq!(empty.try_take(),Ok(None));
    }
    // ------------------------=
    // FUNC: concurrent_copies_are_complete_and_exactly_once
    // DESC: Transfers patterned messages between real threads to detect torn or duplicate payloads.
    // ------------------=
    #[test]
    fn concurrent_copies_are_complete_and_exactly_once() {
        let queue=Mailbox::<[u64;8],8>::new();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                for sequence in 0..2000 {
                    let payload=[sequence;8];
                    while queue.try_send(payload).is_err() {std::thread::yield_now();}
                }
            });
            for sequence in 0..2000 {
                loop {
                    if let Ok(Some(payload))=queue.try_take() {assert_eq!(payload,[sequence;8]);break;}
                    std::thread::yield_now();
                }
            }
        });
        assert_eq!(queue.try_take(),Ok(None));
    }
}
