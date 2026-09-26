//! Explicit owner-local TCP service binding for the staged std adapter.
//! Callbacks are nonblocking and may not yield or reenter this ABI. The native
//! service owns capability resolution, deadlines, buffers and handle generations.
use core::{ptr, sync::atomic::{AtomicPtr, AtomicBool, Ordering}};
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Status {
    pub flags: u32, // connected=1, readable=2, writable=4
    pub local: [u8; 4], pub remote: [u8; 4],
    pub local_port: u16, pub remote_port: u16,
}
pub struct Provider {
    pub cpu: fn() -> u64,
    pub owner: u64,
    pub open: fn([u8; 4], u16) -> Result<u64, i32>,
    pub status: fn(u64) -> Result<Status, i32>,
    pub read: fn(u64, &mut [u8], bool) -> Result<usize, i32>,
    pub write: fn(u64, &[u8]) -> Result<usize, i32>,
    pub shutdown: fn(u64, u32) -> Result<(), i32>,
    pub close: fn(u64),
}
static PROVIDER: AtomicPtr<Provider> = AtomicPtr::new(ptr::null_mut());
static BUSY: AtomicBool = AtomicBool::new(false);
struct Guard;
impl Drop for Guard {
    // ------------------------=
    // FUNC: drop
    // DESC: Releases the non-reentrant callback gate before any caller can park or switch stacks.
    // ------------------=
    fn drop(&mut self) { BUSY.store(false, Ordering::Release); }
}
// ------------------------=
// FUNC: install
// DESC: Binds a reviewed service once; no network authority exists before binding.
// ------------------=
pub fn install(provider: &'static Provider) -> bool {
    (provider.cpu)() == provider.owner && PROVIDER.compare_exchange(ptr::null_mut(),
        provider as *const Provider as *mut Provider, Ordering::AcqRel, Ordering::Acquire).is_ok()
}
// ------------------------=
// FUNC: enter
// DESC: Rejects foreign CPUs, recursion, and missing native services instead of falling back to a host.
// ------------------=
fn enter() -> Result<(&'static Provider, Guard), i32> {
    let pointer = PROVIDER.load(Ordering::Acquire);
    if pointer.is_null() { return Err(95); }
    let provider = unsafe { &*pointer };
    if (provider.cpu)() != provider.owner { return Err(13); }
    if BUSY.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() { return Err(11); }
    Ok((provider, Guard))
}
// ------------------------=
// FUNC: infinity_tcp_open
// DESC: Starts one authorized IPv4 connection and returns only a nonzero service handle.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_tcp_open(address: *const u8, port: u16, handle: *mut u64) -> i32 {
    if address.is_null() || handle.is_null() || port == 0 { return 22; }
    let (p, _guard) = match enter() { Ok(p) => p, Err(e) => return e };
    let mut bytes = [0; 4]; bytes.copy_from_slice(core::slice::from_raw_parts(address, 4));
    match (p.open)(bytes, port) { Ok(id) if id != 0 => { handle.write(id); 0 }, Ok(_) => 5, Err(e) => e }
}
// ------------------------=
// FUNC: infinity_tcp_status
// DESC: Reads live native connection and address state without waiting.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_tcp_status(handle: u64, out: *mut Status) -> i32 {
    if out.is_null() { return 22; }
    let (p, _guard) = match enter() { Ok(p) => p, Err(e) => return e };
    match (p.status)(handle) { Ok(status) => { out.write(status); 0 }, Err(e) => e }
}
// ------------------------=
// FUNC: infinity_tcp_read
// DESC: Reads or peeks directly into the caller buffer under the service's authority checks.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_tcp_read(handle: u64, bytes: *mut u8, length: usize, peek: bool, count: *mut usize) -> i32 {
    if count.is_null() || bytes.is_null() || length > isize::MAX as usize { return 22; }
    let (p, _guard) = match enter() { Ok(p) => p, Err(e) => return e };
    match (p.read)(handle, core::slice::from_raw_parts_mut(bytes, length), peek) {
        Ok(n) if n <= length => { count.write(n); 0 }, Ok(_) => 5, Err(e) => e,
    }
}
// ------------------------=
// FUNC: infinity_tcp_write
// DESC: Enqueues available native transmit capacity without copying into an unbounded staging buffer.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_tcp_write(handle: u64, bytes: *const u8, length: usize, count: *mut usize) -> i32 {
    if count.is_null() || bytes.is_null() || length > isize::MAX as usize { return 22; }
    let (p, _guard) = match enter() { Ok(p) => p, Err(e) => return e };
    match (p.write)(handle, core::slice::from_raw_parts(bytes, length)) {
        Ok(n) if n <= length => { count.write(n); 0 }, Ok(_) => 5, Err(e) => e,
    }
}
// ------------------------=
// FUNC: infinity_tcp_shutdown
// DESC: Requests an implemented native half-close; unsupported modes fail explicitly.
// ------------------=
#[no_mangle]
pub extern "C" fn infinity_tcp_shutdown(handle: u64, mode: u32) -> i32 {
    let (p, _guard) = match enter() { Ok(p) => p, Err(e) => return e };
    match (p.shutdown)(handle, mode) { Ok(()) => 0, Err(e) => e }
}
// ------------------------=
// FUNC: infinity_tcp_close
// DESC: Releases the last std owner of a native handle without waiting for peer traffic.
// ------------------=
#[no_mangle]
pub extern "C" fn infinity_tcp_close(handle: u64) {
    if let Ok((p, _guard)) = enter() { (p.close)(handle); }
}
