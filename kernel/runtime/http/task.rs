//! A pinned, allocation-free future slot for the single-threaded network service.
use core::{
    future::Future,
    marker::{PhantomData, PhantomPinned},
    mem::{align_of, size_of, MaybeUninit},
    pin::Pin,
    task::{Context, Poll},
};

#[repr(C, align(64))]
struct Storage<const N: usize>([MaybeUninit<u8>; N]);

pub struct Task<T, const N: usize> {
    storage: Storage<N>,
    poll: Option<unsafe fn(*mut u8, &mut Context<'_>) -> Poll<T>>,
    destroy: Option<unsafe fn(*mut u8)>,
    _pin: PhantomPinned,
    _single_thread: PhantomData<*mut ()>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Busy,
    Capacity,
}

impl<T, const N: usize> Task<T, N> {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves bounded task storage without constructing or polling any future.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            storage: Storage([MaybeUninit::uninit(); N]),
            poll: None,
            destroy: None,
            _pin: PhantomPinned,
            _single_thread: PhantomData,
        }
    }
    // ------------------------=
    // FUNC: start
    // DESC: Installs a static future only after pinning its storage; rejects oversized, over-aligned or overlapping work.
    // ------------------=
    pub fn start<F: Future<Output = T> + 'static>(
        self: Pin<&mut Self>,
        future: F,
    ) -> Result<(), Error> {
        // SAFETY: Storage never moves after pinning; no pinned field is moved here.
        let this = unsafe { self.get_unchecked_mut() };
        if this.poll.is_some() {
            return Err(Error::Busy);
        }
        if size_of::<F>() > N || align_of::<F>() > align_of::<Storage<N>>() {
            return Err(Error::Capacity);
        }
        unsafe {
            this.storage.0.as_mut_ptr().cast::<F>().write(future);
        }
        this.poll = Some(poll_future::<F>);
        this.destroy = Some(drop_future::<F>);
        Ok(())
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Runs at most one future poll; completed futures are destroyed in place before returning their result.
    // ------------------=
    pub fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<T> {
        let this = unsafe { self.get_unchecked_mut() };
        let Some(poll) = this.poll else {
            return Poll::Pending;
        };
        let result = unsafe { poll(this.storage.0.as_mut_ptr().cast(), context) };
        if result.is_ready() {
            this.clear();
        }
        result
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Drops suspended work in place, releasing all references before task buffers may be reused.
    // ------------------=
    pub fn cancel(self: Pin<&mut Self>) {
        unsafe { self.get_unchecked_mut() }.clear();
    }
    // ------------------------=
    // FUNC: active
    // DESC: Reports whether the slot owns an unfinished request.
    // ------------------=
    pub fn active(&self) -> bool {
        self.poll.is_some()
    }
    // ------------------------=
    // FUNC: clear
    // DESC: Removes callbacks before destroying the future, preventing double destruction on cancellation or completion.
    // ------------------=
    fn clear(&mut self) {
        self.poll = None;
        if let Some(destroy) = self.destroy.take() {
            unsafe {
                destroy(self.storage.0.as_mut_ptr().cast());
            }
        }
    }
}
impl<T, const N: usize> Drop for Task<T, N> {
    // ------------------------=
    // FUNC: drop
    // DESC: Destroys pending work before its pinned backing allocation is released.
    // ------------------=
    fn drop(&mut self) {
        self.clear();
    }
}
// ------------------------=
// FUNC: poll_future
// DESC: Reconstructs the concrete pinned future under the storage size, alignment and lifetime invariants established by start.
// ------------------=
unsafe fn poll_future<F: Future>(pointer: *mut u8, context: &mut Context<'_>) -> Poll<F::Output> {
    Pin::new_unchecked(&mut *pointer.cast::<F>()).poll(context)
}
// ------------------------=
// FUNC: drop_future
// DESC: Destroys the concrete future without moving its pinned state.
// ------------------=
unsafe fn drop_future<F>(pointer: *mut u8) {
    pointer.cast::<F>().drop_in_place();
}
