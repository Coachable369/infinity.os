//! Independent-stack, bounded cooperative execution for a dedicated native owner.
//! Not preemptive and not wired to the kernel/std ABI yet. Never dispatch engine
//! code on the desktop thread. All unsafe methods require a stationary executor,
//! one serialized owner CPU, no interrupt re-entry, and exclusive live stacks.
//! No mutable borrow of shared executor state survives a context switch/callback.
use core::{cell::UnsafeCell, marker::PhantomData, ptr, sync::atomic::AtomicU32};
use crate::{context::{Context, switch}, wait::{Waits, Outcome}, Keys, Values, Teardown, Destructor};
pub const MAX_TLS_KEYS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { Full, Invalid, Busy, NoCurrent, Stack, Exhausted }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State { Vacant, Runnable, Running, Waiting, Sleeping(u64), Joining(u64), Complete }
struct Thread {
    context: Context, owner: usize, id: u64, state: State, detached: bool,
    entry: Option<extern "C" fn(usize)>, argument: usize, low: usize, high: usize,
    result: Option<Outcome>, values: Values<MAX_TLS_KEYS>, joiner: Option<u64>,
}
impl Thread {
    // ------------------------=
    // FUNC: empty
    // DESC: Initializes an unused bounded thread slot.
    // ------------------=
    const fn empty() -> Self { Self { context: Context::empty(), owner: 0, id: 0,
        state: State::Vacant, detached: false, entry: None, argument: 0, low: 0, high: 0,
        result: None, values: Values::new(), joiner: None } }
}
struct Inner<const N: usize> {
    threads: [Thread; N], root: Context, current: Option<usize>, cursor: usize,
    next_id: u64, waits: Waits<N>, keys: Keys<MAX_TLS_KEYS>, root_values: Values<MAX_TLS_KEYS>,
}
pub struct Executor<const N: usize> { inner: UnsafeCell<Inner<N>>, owner_only: PhantomData<*mut ()> }
impl<const N: usize> Executor<N> {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an allocation-free executor with no runnable work.
    // ------------------=
    pub const fn new() -> Self { Self { inner: UnsafeCell::new(Inner {
        threads: [const { Thread::empty() }; N], root: Context::empty(), current: None,
        cursor: 0, next_id: 1, waits: Waits::new(), keys: Keys::new(), root_values: Values::new(),
    }), owner_only: PhantomData } }
    // ------------------------=
    // FUNC: spawn
    // DESC: Transfers entry execution only after validating capacity and disjoint leased stack storage.
    // ------------------=
    /// # Safety
    /// Executor and stack must not move or be freed while live. The stack is
    /// exclusively leased until joined, or detached completion is observed.
    /// Entry must not unwind. Calls must follow the single-owner module contract.
    pub unsafe fn spawn(&self, stack: &mut [u8], entry: extern "C" fn(usize), argument: usize) -> Result<u64, Error> {
        let inner = &mut *self.inner.get();
        let low = stack.as_mut_ptr() as usize;
        let high = low.checked_add(stack.len()).ok_or(Error::Stack)?;
        if stack.len() < 4096 || inner.threads.iter().any(|t| t.state != State::Vacant && low < t.high && high > t.low) {
            return Err(Error::Stack);
        }
        let slot = inner.threads.iter_mut().find(|t| t.state == State::Vacant).ok_or(Error::Full)?;
        let next = inner.next_id.checked_add(1).ok_or(Error::Exhausted)?;
        let id = inner.next_id;
        *slot = Thread::empty();
        slot.owner = self as *const Self as usize;
        slot.id = id; slot.entry = Some(entry); slot.argument = argument;
        slot.low = low; slot.high = high;
        let argument = slot as *mut Thread as usize;
        if !slot.context.initialize_argument(stack, start::<N>, argument) { return Err(Error::Stack); }
        slot.state = State::Runnable;
        inner.next_id = next;
        Ok(id)
    }
    // ------------------------=
    // FUNC: dispatch
    // DESC: Runs one fair runnable continuation; expires waits using the owner's monotonic clock.
    // ------------------=
    pub unsafe fn dispatch(&self, now: u64) -> Result<bool, Error> {
        let (root, next, index) = {
            let inner = &mut *self.inner.get();
            if inner.current.is_some() { return Err(Error::Busy); }
            inner.waits.expire(now);
            for t in &mut inner.threads {
                if t.state == State::Waiting {
                    if let Some(result) = inner.waits.take(t.id) { t.result = Some(result); t.state = State::Runnable; }
                }
                if matches!(t.state, State::Sleeping(d) if d <= now) { t.state = State::Runnable; }
            }
            let Some(index) = (0..N).map(|offset| (inner.cursor + offset) % N)
                .find(|&i| inner.threads[i].state == State::Runnable) else { return Ok(false); };
            inner.cursor = (index + 1) % N;
            inner.current = Some(index); inner.threads[index].state = State::Running;
            (ptr::addr_of_mut!(inner.root), ptr::addr_of!(inner.threads[index].context), index)
        };
        switch(root, next);
        let inner = &mut *self.inner.get();
        inner.current = None;
        let t = &mut inner.threads[index];
        if t.state == State::Complete && t.detached { t.state = State::Vacant; }
        Ok(true)
    }
    // ------------------------=
    // FUNC: suspend
    // DESC: Saves the current independent stack and returns control to its owner dispatcher.
    // ------------------=
    unsafe fn suspend(&self, state: State) -> Result<(), Error> {
        let (old, root) = {
            let inner = &mut *self.inner.get();
            let index = inner.current.ok_or(Error::NoCurrent)?;
            inner.threads[index].state = state;
            (ptr::addr_of_mut!(inner.threads[index].context), ptr::addr_of!(inner.root))
        };
        switch(old, root);
        Ok(())
    }
    // ------------------------=
    // FUNC: yield_now
    // DESC: Makes the calling continuation eligible for the next fair dispatch.
    // ------------------=
    pub unsafe fn yield_now(&self) -> Result<(), Error> { self.suspend(State::Runnable) }
    // ------------------------=
    // FUNC: sleep_until
    // DESC: Parks until an absolute monotonic deadline; no polling on the suspended stack.
    // ------------------=
    pub unsafe fn sleep_until(&self, deadline: u64) -> Result<(), Error> { self.suspend(State::Sleeping(deadline)) }
    // ------------------------=
    // FUNC: wait
    // DESC: Atomically registers against serialized wakes and resumes with the observed outcome.
    // ------------------=
    pub unsafe fn wait(&self, word: &AtomicU32, expected: u32, deadline: Option<u64>, now: u64) -> Result<Outcome, Error> {
        {
            let inner = &mut *self.inner.get();
            let index = inner.current.ok_or(Error::NoCurrent)?;
            let id = inner.threads[index].id;
            if let Some(result) = inner.waits.prepare(id, word, expected, deadline, now).map_err(|_| Error::Busy)? { return Ok(result); }
        }
        self.suspend(State::Waiting)?;
        let inner = &mut *self.inner.get();
        let index = inner.current.ok_or(Error::NoCurrent)?;
        inner.threads[index].result.take().ok_or(Error::Invalid)
    }
    // ------------------------=
    // FUNC: wake
    // DESC: Delivers a serialized wake; parked owners become runnable at the next dispatch.
    // ------------------=
    pub unsafe fn wake(&self, word: &AtomicU32, all: bool) -> usize { (*self.inner.get()).waits.wake(word, all) }
    // ------------------------=
    // FUNC: try_join
    // DESC: Consumes a completed joinable handle or reports pending without blocking the dispatcher.
    // ------------------=
    pub unsafe fn try_join(&self, id: u64) -> Result<bool, Error> {
        let inner = &mut *self.inner.get();
        let caller = inner.current.map(|i| inner.threads[i].id).unwrap_or(0);
        let t = inner.threads.iter_mut().find(|t| t.id == id && t.state != State::Vacant).ok_or(Error::Invalid)?;
        if t.detached || caller == id || t.joiner.is_some_and(|owner| owner != caller) { return Err(Error::Invalid); }
        if t.state != State::Complete { return Ok(false); }
        t.state = State::Vacant;
        Ok(true)
    }
    // ------------------------=
    // FUNC: join
    // DESC: Parks a native thread until a joinable target completes; rejects self/cyclic joins.
    // ------------------=
    pub unsafe fn join(&self, id: u64) -> Result<(), Error> {
        let caller = self.current_id().ok_or(Error::NoCurrent)?;
        {
            let inner = &mut *self.inner.get();
            let mut next = id;
            for _ in 0..=N {
                if next == caller { return Err(Error::Invalid); }
                let t = inner.threads.iter().find(|t| t.id == next && t.state != State::Vacant).ok_or(Error::Invalid)?;
                if let State::Joining(target) = t.state { next = target; } else { break; }
            }
            let t = inner.threads.iter_mut().find(|t| t.id == id).ok_or(Error::Invalid)?;
            if t.detached || t.joiner.is_some_and(|owner| owner != caller) { return Err(Error::Invalid); }
            t.joiner = Some(caller);
        }
        if self.try_join(id)? { return Ok(()); }
        self.suspend(State::Joining(id))?;
        if self.try_join(id)? { Ok(()) } else { Err(Error::Invalid) }
    }
    // ------------------------=
    // FUNC: detach
    // DESC: Releases join ownership without stopping an active thread or reclaiming its running stack.
    // ------------------=
    pub unsafe fn detach(&self, id: u64) -> Result<(), Error> {
        let inner = &mut *self.inner.get();
        let t = inner.threads.iter_mut().find(|t| t.id == id && t.state != State::Vacant).ok_or(Error::Invalid)?;
        if t.detached || t.joiner.is_some() { return Err(Error::Invalid); }
        t.detached = true;
        if t.state == State::Complete { t.state = State::Vacant; }
        Ok(())
    }
    // ------------------------=
    // FUNC: current_id
    // DESC: Reports the actual selected native thread, not the dispatcher identity.
    // ------------------=
    pub unsafe fn current_id(&self) -> Option<u64> { let i = &*self.inner.get(); i.current.map(|n| i.threads[n].id) }
    // ------------------------=
    // FUNC: contains
    // DESC: Allows stack owners to observe joined or detached completion before reclaiming storage.
    // ------------------=
    pub unsafe fn contains(&self, id: u64) -> bool { (*self.inner.get()).threads.iter().any(|t| t.id == id && t.state != State::Vacant) }
    // ------------------------=
    // FUNC: key_create
    // DESC: Allocates a process-local TLS key with optional bounded exit teardown.
    // ------------------=
    pub unsafe fn key_create(&self, destructor: Option<Destructor>) -> Result<usize, crate::Error> { (*self.inner.get()).keys.create(destructor) }
    // ------------------------=
    // FUNC: key_destroy
    // DESC: Revokes a key without invoking callbacks on another thread's values.
    // ------------------=
    pub unsafe fn key_destroy(&self, key: usize) -> Result<(), crate::Error> { (*self.inner.get()).keys.destroy(key) }
    // ------------------------=
    // FUNC: tls_get
    // DESC: Reads TLS belonging to the currently selected stack or dispatcher.
    // ------------------=
    pub unsafe fn tls_get(&self, key: usize) -> Result<usize, crate::Error> {
        let i = &*self.inner.get(); let values = i.current.map(|n| &i.threads[n].values).unwrap_or(&i.root_values);
        i.keys.get(values, key)
    }
    // ------------------------=
    // FUNC: tls_set
    // DESC: Writes TLS only for the current execution identity.
    // ------------------=
    pub unsafe fn tls_set(&self, key: usize, value: usize) -> Result<(), crate::Error> {
        let i = &mut *self.inner.get(); let values = match i.current { Some(n) => &mut i.threads[n].values, None => &mut i.root_values };
        i.keys.set(values, key, value)
    }
    // ------------------------=
    // FUNC: complete
    // DESC: Runs reentrant bounded TLS teardown before waking joiners and permanently suspending the finished stack.
    // ------------------=
    unsafe fn complete(&self) -> ! {
        let mut teardown = Teardown::new();
        loop {
            let callback = {
                let i = &mut *self.inner.get(); let n = i.current.expect("native thread");
                teardown.next(&i.keys, &mut i.threads[n].values)
            };
            let Some((callback, value)) = callback else { break; };
            callback(value as *mut u8);
        }
        {
            let i = &mut *self.inner.get(); let n = i.current.expect("native thread"); let id = i.threads[n].id;
            i.waits.cancel(id);
            for t in &mut i.threads { if t.state == State::Joining(id) { t.state = State::Runnable; } }
        }
        let _ = self.suspend(State::Complete);
        panic!("completed native thread resumed")
    }
}
// ------------------------=
// FUNC: start
// DESC: Enters the captured native closure on its own stack and guarantees exit teardown on normal return.
// ------------------=
extern "C" fn start<const N: usize>(argument: usize) -> ! {
    unsafe {
        let slot = argument as *const Thread;
        let owner = (*slot).owner as *const Executor<N>;
        let entry = (*slot).entry.expect("thread entry"); let argument = (*slot).argument;
        entry(argument);
        (*owner).complete()
    }
}
