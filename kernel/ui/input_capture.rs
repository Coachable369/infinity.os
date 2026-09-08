//! Optional non-dispatching device capture during long software raster passes.
use core::sync::atomic::{AtomicUsize, Ordering};
static CAPTURE: AtomicUsize = AtomicUsize::new(0);
// ------------------------=
// FUNC: install
// DESC: Registers a trusted driver capture callback only after hardware initialization is complete.
// ------------------=
pub(crate) fn install(callback: fn()) { CAPTURE.store(callback as usize, Ordering::Release); }
// ------------------------=
// FUNC: poll
// DESC: Captures a bounded amount of raw input without reentering Console, runtime, or presentation.
// ------------------=
pub fn poll() {
    let callback = CAPTURE.load(Ordering::Acquire);
    if callback != 0 { unsafe { core::mem::transmute::<usize, fn()>(callback)() } }
}
