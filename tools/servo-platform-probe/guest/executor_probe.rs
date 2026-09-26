//! Actual lifecycle execution on native stacks, shared by both target guests.
use infinity_servo_runtime_primitives::{executor::{Executor, Error}, wait::Outcome};
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
#[repr(align(16))]
struct Stack([u8; 32768]);
static mut STACKS: [Stack; 3] = [const { Stack([0; 32768]) }; 3];
static mut EXECUTOR: Executor<3> = Executor::new();
static mut KEY: usize = 0;
static mut A: u64 = 0;
static WORD: AtomicU32 = AtomicU32::new(0);
static DESTRUCTORS: AtomicUsize = AtomicUsize::new(0);
static COMPLETED: AtomicUsize = AtomicUsize::new(0);

// ------------------------=
// FUNC: executor
// DESC: Borrows a stationary executor only on the serialized guest CPU.
// ------------------=
unsafe fn executor() -> &'static Executor<3> { &*core::ptr::addr_of!(EXECUTOR) }
// ------------------------=
// FUNC: destructor
// DESC: Checks cleared-before-callback TLS and yields during exit cleanup without aliasing registry state.
// ------------------=
unsafe extern "C" fn destructor(value: *mut u8) {
    assert!(value as usize == 11 || value as usize == 22);
    assert_eq!(executor().tls_get(KEY), Ok(0));
    executor().yield_now().unwrap();
    DESTRUCTORS.fetch_add(1, Ordering::SeqCst);
}
// ------------------------=
// FUNC: first
// DESC: Exercises private TLS, repeated yield, self-join rejection and a blocking producer wake.
// ------------------=
extern "C" fn first(argument: usize) { unsafe {
    assert_eq!(argument, 11);
    let e = executor();
    e.tls_set(KEY, argument).unwrap();
    assert_eq!(e.join(e.current_id().unwrap()), Err(Error::Invalid));
    for _ in 0..100 { e.yield_now().unwrap(); assert_eq!(e.tls_get(KEY), Ok(11)); }
    assert_eq!(e.wait(&WORD, 0, None, 0), Ok(Outcome::Woken));
    assert_eq!(WORD.load(Ordering::Acquire), 1);
    COMPLETED.fetch_add(1, Ordering::SeqCst);
} }
// ------------------------=
// FUNC: second
// DESC: Runs after detach, preserves independent TLS and wakes the parked consumer.
// ------------------=
extern "C" fn second(argument: usize) { unsafe {
    assert_eq!(argument, 22);
    let e = executor(); e.tls_set(KEY, argument).unwrap();
    for _ in 0..100 { e.yield_now().unwrap(); assert_eq!(e.tls_get(KEY), Ok(22)); }
    WORD.store(1, Ordering::Release);
    assert_eq!(e.wake(&WORD, false), 1);
    assert_eq!(e.wake(&WORD, true), 0);
    COMPLETED.fetch_add(1, Ordering::SeqCst);
} }
// ------------------------=
// FUNC: joiner
// DESC: Joins a real suspended thread then sleeps and times out without busy polling on its own stack.
// ------------------=
extern "C" fn joiner(_: usize) { unsafe {
    let e = executor(); assert_eq!(e.tls_get(KEY), Ok(0));
    e.join(A).unwrap();
    assert_eq!(e.try_join(A), Err(Error::Invalid));
    e.sleep_until(100).unwrap();
    assert_eq!(e.wait(&WORD, 1, Some(110), 100), Ok(Outcome::TimedOut));
    COMPLETED.fetch_add(1, Ordering::SeqCst);
} }
// ------------------------=
// FUNC: replacement
// DESC: Confirms reused slots start with empty TLS and correct new entry data.
// ------------------=
extern "C" fn replacement(argument: usize) { unsafe {
    assert_eq!(argument, 44); assert_eq!(executor().tls_get(KEY), Ok(0));
    COMPLETED.fetch_add(1, Ordering::SeqCst);
} }
// ------------------------=
// FUNC: stack
// DESC: Provides each exclusively leased guest stack once until completion is observed.
// ------------------=
unsafe fn stack(index: usize) -> &'static mut [u8] {
    let slot = core::ptr::addr_of_mut!(STACKS).cast::<Stack>().add(index);
    core::slice::from_raw_parts_mut(core::ptr::addr_of_mut!((*slot).0).cast::<u8>(), 32768)
}
// ------------------------=
// FUNC: run
// DESC: Verifies lifecycle, bounds, stale handles, blocking waits and TLS teardown using actual context execution.
// ------------------=
pub unsafe fn run() {
    let e = executor(); KEY = e.key_create(Some(destructor)).unwrap(); e.tls_set(KEY, 99).unwrap();
    A = e.spawn(stack(0), first, 11).unwrap();
    let b = e.spawn(stack(1), second, 22).unwrap();
    let c = e.spawn(stack(2), joiner, 0).unwrap();
    assert_eq!(e.try_join(A), Ok(false));
    e.detach(b).unwrap(); assert_eq!(e.try_join(b), Err(Error::Invalid));
    for _ in 0..1000 { if !e.dispatch(0).unwrap() { break; } }
    assert_eq!(COMPLETED.load(Ordering::SeqCst), 2);
    assert_eq!(DESTRUCTORS.load(Ordering::SeqCst), 2);
    assert!(!e.contains(A)); assert!(!e.contains(b)); assert!(e.contains(c));
    assert!(!e.dispatch(99).unwrap());
    assert!(e.dispatch(100).unwrap()); assert!(!e.dispatch(109).unwrap());
    assert!(e.dispatch(110).unwrap()); assert_eq!(e.try_join(c), Ok(true));
    assert_eq!(COMPLETED.load(Ordering::SeqCst), 3);
    assert_eq!(e.tls_get(KEY), Ok(99));
    let d = e.spawn(stack(0), replacement, 44).unwrap(); let old = A; assert_ne!(d, old);
    e.detach(d).unwrap(); assert!(e.dispatch(110).unwrap()); assert!(!e.contains(d));
    assert_eq!(e.try_join(A), Err(Error::Invalid));
    assert_eq!(COMPLETED.load(Ordering::SeqCst), 4);
    e.key_destroy(KEY).unwrap(); assert!(e.tls_get(KEY).is_err());
}
