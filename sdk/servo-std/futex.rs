use crate::{sync::atomic::AtomicU32, time::Duration};
pub type Futex = AtomicU32;
pub type SmallFutex = AtomicU32;
pub type Primitive = u32;
pub type SmallPrimitive = u32;
unsafe extern "C" {
    fn infinity_std_wait(address: *const AtomicU32, expected: u32, nanoseconds: u64, timed: bool) -> i32;
    fn infinity_std_wake(address: *const AtomicU32, all: bool) -> i32;
}
// ------------------------=
// FUNC: futex_wait
// DESC: Delegates atomic compare-and-sleep; native results distinguish timeout from wake/race.
// ------------------=
pub fn futex_wait(address: &AtomicU32, expected: u32, timeout: Option<Duration>) -> bool {
    let nanos = timeout.map(|v| u64::try_from(v.as_nanos()).unwrap_or(u64::MAX)).unwrap_or(0);
    match unsafe { infinity_std_wait(address, expected, nanos, timeout.is_some()) } {
        0 | 4 | 11 => true,
        110 => false,
        _ => panic!("native wait service failed"),
    }
}
// ------------------------=
// FUNC: futex_wake
// DESC: Wakes at most one native waiter and propagates service failure.
// ------------------=
pub fn futex_wake(address: &AtomicU32) -> bool {
    let count = unsafe { infinity_std_wake(address, false) };
    assert!(count >= 0);
    count != 0
}
// ------------------------=
// FUNC: futex_wake_all
// DESC: Wakes all waiters in the owning process's authorized wait set.
// ------------------=
pub fn futex_wake_all(address: &AtomicU32) { assert!(unsafe { infinity_std_wake(address, true) } >= 0); }
