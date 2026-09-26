use crate::{ffi::CStr, io, num::NonZero, thread::ThreadInit, time::Duration};
unsafe extern "C" {
    fn infinity_std_thread_create(stack: usize, entry: extern "C" fn(*mut u8), data: *mut u8, id: *mut u64) -> i32;
    fn infinity_std_thread_join(id: u64) -> i32;
    fn infinity_std_thread_detach(id: u64);
    fn infinity_std_thread_id() -> u64;
    fn infinity_std_parallelism() -> usize;
    fn infinity_std_yield();
    fn infinity_std_sleep(nanoseconds: u64) -> i32;
    fn infinity_std_thread_name(bytes: *const u8, length: usize);
}
pub const DEFAULT_MIN_STACK_SIZE: usize = 2 * 1024 * 1024;
pub struct Thread { id: u64 }
impl Thread {
    // ------------------------=
    // FUNC: new
    // DESC: Transfers initializer ownership only after native thread creation succeeds.
    // ------------------=
    pub unsafe fn new(stack: usize, init: Box<ThreadInit>) -> io::Result<Self> {
        let data = Box::into_raw(init);
        let mut id = 0;
        let error = unsafe { infinity_std_thread_create(stack, start, data.cast(), &mut id) };
        if error != 0 {
            unsafe { drop(Box::from_raw(data)); }
            return Err(io::Error::from_raw_os_error(error));
        }
        assert_ne!(id, 0);
        Ok(Self { id })
    }
    // ------------------------=
    // FUNC: join
    // DESC: Waits for native completion without double-detaching the consumed handle.
    // ------------------=
    pub fn join(self) {
        assert_eq!(unsafe { infinity_std_thread_join(self.id) }, 0);
        crate::mem::forget(self);
    }
}
impl Drop for Thread {
    // ------------------------=
    // FUNC: drop
    // DESC: Releases an unjoined thread handle without terminating the running thread.
    // ------------------=
    fn drop(&mut self) { unsafe { infinity_std_thread_detach(self.id); } }
}
// ------------------------=
// FUNC: start
// DESC: Initializes Rust thread state; native wrapper must run TLS destructors on return.
// ------------------=
extern "C" fn start(data: *mut u8) {
    let init = unsafe { Box::from_raw(data.cast::<ThreadInit>()) };
    init.init()();
}
// ------------------------=
// FUNC: available_parallelism
// DESC: Reports native resource-governed concurrency, never a fabricated processor count.
// ------------------=
pub fn available_parallelism() -> io::Result<NonZero<usize>> {
    NonZero::new(unsafe { infinity_std_parallelism() }).ok_or(io::Error::UNKNOWN_THREAD_COUNT)
}
// ------------------------=
// FUNC: current_os_id
// DESC: Returns the native execution thread identity.
// ------------------=
pub fn current_os_id() -> Option<u64> { let id = unsafe { infinity_std_thread_id() }; (id != 0).then_some(id) }
// ------------------------=
// FUNC: yield_now
// DESC: Yields through the scheduler instead of spinning on the desktop thread.
// ------------------=
pub fn yield_now() { unsafe { infinity_std_yield(); } }
// ------------------------=
// FUNC: sleep
// DESC: Requests a bounded native monotonic wait and rejects unsupported waits.
// ------------------=
pub fn sleep(duration: Duration) {
    let nanos = u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX);
    assert_eq!(unsafe { infinity_std_sleep(nanos) }, 0);
}
// ------------------------=
// FUNC: set_name
// DESC: Forwards diagnostic naming without granting additional authority.
// ------------------=
pub fn set_name(name: &CStr) { unsafe { infinity_std_thread_name(name.as_ptr().cast(), name.to_bytes().len()); } }
