//! Native parking_lot backend for the browser's cooperative executor.
use core::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

extern "C" {
    // ------------------------=
    // FUNC: infinity_std_wait
    // DESC: Atomically compares and parks through the existing browser executor.
    // ------------------=
    fn infinity_std_wait(address:*const AtomicU32, expected:u32, nanoseconds:u64, timed:bool)->i32;
    // ------------------------=
    // FUNC: infinity_std_wake
    // DESC: Wakes a registered native waiter after publishing its released state.
    // ------------------=
    fn infinity_std_wake(address:*const AtomicU32, all:bool)->i32;
}

pub struct ThreadParker { state:AtomicU32 }
pub struct UnparkHandle { state:*const AtomicU32 }

impl super::ThreadParkerT for ThreadParker {
    type UnparkHandle=UnparkHandle;
    const IS_CHEAP_TO_CONSTRUCT:bool=true;
    // ------------------------=
    // FUNC: new
    // DESC: Creates an unlocked waiter without allocating or consulting a host OS.
    // ------------------=
    fn new()->Self {Self{state:AtomicU32::new(0)}}
    // ------------------------=
    // FUNC: prepare_park
    // DESC: Arms the waiter before parking_lot publishes it in the locked queue.
    // ------------------=
    unsafe fn prepare_park(&self) {self.state.store(1,Ordering::Relaxed);}
    // ------------------------=
    // FUNC: timed_out
    // DESC: Distinguishes timeout from an unpark racing with the deadline under the queue lock.
    // ------------------=
    unsafe fn timed_out(&self)->bool {self.state.load(Ordering::Relaxed)!=0}
    // ------------------------=
    // FUNC: park
    // DESC: Releases execution to the lock owner instead of spinning indefinitely on the same native CPU.
    // ------------------=
    unsafe fn park(&self) {
        while self.state.load(Ordering::Acquire)!=0 {
            assert_eq!(infinity_std_wait(&self.state,1,0,false),0);
        }
    }
    // ------------------------=
    // FUNC: park_until
    // DESC: Preserves absolute-deadline and spurious-wake semantics with the native monotonic clock.
    // ------------------=
    unsafe fn park_until(&self,deadline:Instant)->bool {
        while self.state.load(Ordering::Acquire)!=0 {
            let now=Instant::now();
            if now>=deadline {return false;}
            let ns=(deadline-now).as_nanos().min(u64::MAX as u128) as u64;
            let result=infinity_std_wait(&self.state,1,ns,true);
            assert!(result==0 || result==110);
        }
        true
    }
    // ------------------------=
    // FUNC: unpark_lock
    // DESC: Publishes release under the parking queue lock; the actual wake follows queue unlock.
    // ------------------=
    unsafe fn unpark_lock(&self)->UnparkHandle {
        self.state.store(0,Ordering::Release);
        UnparkHandle{state:&self.state}
    }
}
impl super::UnparkHandleT for UnparkHandle {
    // ------------------------=
    // FUNC: unpark
    // DESC: Notifies the native waiter using parking_lot's stable thread-data address contract.
    // ------------------=
    unsafe fn unpark(self) {assert!(infinity_std_wake(self.state,false)>=0);}
}
// ------------------------=
// FUNC: thread_yield
// DESC: Runs another browser executor thread during bounded lock acquisition backoff.
// ------------------=
pub fn thread_yield() {std::thread::yield_now();}
