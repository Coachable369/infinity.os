//! Bounded owner-local resolver boundary. No hostname authority exists until a
//! reviewed provider is installed. Callbacks must be nonblocking and nonreentrant.
use core::{ptr, sync::atomic::{AtomicBool, AtomicPtr, Ordering}};

pub struct Provider {
    pub cpu: fn() -> u64,
    pub owner: u64,
    pub begin: fn(&str) -> Result<u64, i32>,
    pub poll: fn(u64) -> Result<[u8; 4], i32>,
    pub cancel: fn(u64),
}
static PROVIDER: AtomicPtr<Provider> = AtomicPtr::new(ptr::null_mut());
static BUSY: AtomicBool = AtomicBool::new(false);
struct Guard;
impl Drop for Guard {
    // ------------------------=
    // FUNC: drop
    // DESC: Releases the callback gate before the caller can yield its native stack.
    // ------------------=
    fn drop(&mut self) { BUSY.store(false, Ordering::Release); }
}
// ------------------------=
// FUNC: install
// DESC: Installs one permanent capability-governed resolver on its owning CPU.
// ------------------=
pub fn install(provider: &'static Provider) -> bool {
    (provider.cpu)() == provider.owner && PROVIDER.compare_exchange(ptr::null_mut(),
        provider as *const Provider as *mut Provider, Ordering::AcqRel, Ordering::Acquire).is_ok()
}
// ------------------------=
// FUNC: enter
// DESC: Rejects missing authority, cross-CPU calls and recursive callbacks.
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
// FUNC: infinity_dns_begin
// DESC: Validates a bounded ASCII hostname before submitting an authorized asynchronous A query.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_dns_begin(bytes: *const u8, length: usize, output: *mut u64) -> i32 {
    if bytes.is_null() || output.is_null() || length == 0 || length > 253 { return 22; }
    output.write(0);
    let Ok(name) = core::str::from_utf8(core::slice::from_raw_parts(bytes, length)) else { return 22; };
    let name = name.strip_suffix('.').unwrap_or(name);
    if name.is_empty() || name.split('.').any(|label| label.is_empty() || label.len() > 63 ||
        label.starts_with('-') || label.ends_with('-') ||
        !label.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')) { return 22; }
    let (provider, _guard) = match enter() { Ok(value) => value, Err(error) => return error };
    match (provider.begin)(name) {
        Ok(id) if id != 0 => { output.write(id); 0 },
        Ok(_) => 5, Err(error) => error,
    }
}
// ------------------------=
// FUNC: infinity_dns_poll
// DESC: Copies one completed IPv4 result and preserves the caller buffer while pending or denied.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_dns_poll(id: u64, output: *mut u8) -> i32 {
    if id == 0 || output.is_null() { return 22; }
    let (provider, _guard) = match enter() { Ok(value) => value, Err(error) => return error };
    match (provider.poll)(id) {
        Ok(address) => { ptr::copy_nonoverlapping(address.as_ptr(), output, 4); 0 },
        Err(error) => error,
    }
}
// ------------------------=
// FUNC: infinity_dns_cancel
// DESC: Releases the query on success, error, timeout or dropped work without granting new authority.
// ------------------=
#[no_mangle]
pub extern "C" fn infinity_dns_cancel(id: u64) {
    if id != 0 { if let Ok((provider, _guard)) = enter() { (provider.cancel)(id); } }
}
