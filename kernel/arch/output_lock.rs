use core::sync::atomic::{AtomicBool, Ordering};

static LOCKED: AtomicBool = AtomicBool::new(false);

// ------------------------=
// FUNC: serialized
// DESC: Keeps one complete diagnostic record contiguous across concurrent processor output.
// ------------------=
pub fn serialized<F: FnOnce()>(writer: F) {
    while LOCKED
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        core::hint::spin_loop();
    }
    writer();
    LOCKED.store(false, Ordering::Release);
}
