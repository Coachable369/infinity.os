//! Complete browser component restart; supervisor and native services remain live.
use infinity_browser_core::{recovery::{Boundary, Image}, worker::Host};
const STACK: usize = 2 * 1024 * 1024;
// The x86 component carries larger initialized relocation tables. Keep this
// bounded reservation in sync with each target linker's checkpoint assertion.
const CHECKPOINT: usize = if cfg!(target_arch = "x86_64") { 8 * 1024 * 1024 } else { 2 * 1024 * 1024 };
static mut BOUNDARY: Boundary<STACK> = Boundary::new();
static mut IMAGE: Image<CHECKPOINT> = Image::new();
unsafe extern "C" {
    static mut infinity_browser_private_browser_data_start: u8;
    static mut infinity_browser_private_browser_data_end: u8;
    static mut infinity_browser_private_browser_bss_start: u8;
    static mut infinity_browser_private_browser_bss_end: u8;
    fn infinity_browser_private_infinity_browser_run(host: *mut Host) -> u32;
}
// ------------------------=
// FUNC: enter
// DESC: Restores the relocated private image and heap only after the previous worker and all network loans have ended.
// ------------------=
pub unsafe fn enter(host: *mut Host) -> u32 {
    let data = core::ptr::addr_of_mut!(infinity_browser_private_browser_data_start);
    let bss = core::ptr::addr_of_mut!(infinity_browser_private_browser_bss_start);
    let Some(data_len) = (core::ptr::addr_of!(infinity_browser_private_browser_data_end) as usize).checked_sub(data as usize) else { return 5; };
    let Some(bss_len) = (core::ptr::addr_of!(infinity_browser_private_browser_bss_end) as usize).checked_sub(bss as usize) else { return 5; };
    if data_len == 0 || bss_len == 0 { return 5; }
    if !(&mut *core::ptr::addr_of_mut!(IMAGE)).prepare(
        core::slice::from_raw_parts_mut(data, data_len),
        core::slice::from_raw_parts_mut(bss, bss_len),
        core::slice::from_raw_parts_mut((*host).heap, (*host).heap_length)) { return 5; }
    Boundary::enter(core::ptr::addr_of_mut!(BOUNDARY), run, host as usize).unwrap_or(5)
}
// ------------------------=
// FUNC: run
// DESC: Calls the private component on a discardable stack; normal termination uses the same teardown boundary.
// ------------------=
extern "C" fn run(argument: usize) -> ! { unsafe {
    let result = infinity_browser_private_infinity_browser_run(argument as *mut Host);
    leave(result)
}}
// ------------------------=
// FUNC: leave
// DESC: Abandons all private cooperative continuations and returns to the OS worker without unwinding damaged engine state.
// ------------------=
pub unsafe fn leave(result: u32) -> ! {
    Boundary::leave(core::ptr::addr_of_mut!(BOUNDARY), result)
}
