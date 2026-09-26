#![no_std]
#![no_main]

// ------------------------=
// FUNC: infinity_browser_link_probe
// DESC: Retains actual engine initialization for link diagnostics; never boot this probe.
// ------------------=
#[no_mangle]
pub extern "C" fn infinity_browser_link_probe() -> ! {
    core::mem::forget(servo::ServoBuilder::default().build());
    loop { core::hint::spin_loop(); }
}
