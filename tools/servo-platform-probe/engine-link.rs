#![no_std]
#![no_main]

// ------------------------=
// FUNC: infinity_browser_link_probe
// DESC: Retains actual engine initialization for link diagnostics; never boot this probe.
// ------------------=
#[no_mangle]
pub extern "C" fn infinity_browser_link_probe() -> ! {
    // Retain the real native provider; deliberately do not install fake hooks.
    let _ = unsafe { infinity_servo_runtime_primitives::native::Runtime::allocated() };
    core::mem::forget(servo::ServoBuilder::default().build());
    loop { core::hint::spin_loop(); }
}
