//! Single-owner native std ABI. Initialization requires an explicitly granted
//! heap and real platform hooks. All execution must remain on the bound worker
//! CPU; this module does not create/reserve that CPU or provide preemption.
use core::{alloc::Layout, ptr::{self, NonNull}, sync::atomic::{AtomicPtr, AtomicU32, Ordering}};
use crate::{arena::Arena, executor::Executor, wait::Outcome, Destructor};
pub const MAX_THREADS: usize = 64;
const LIMIT: usize = MAX_THREADS;
pub struct Hooks {
    pub cpu: fn() -> u64,
    pub monotonic: fn() -> u64,
    pub utc: fn() -> Option<(u64, u32)>,
    pub entropy: fn(&mut [u8]) -> bool,
    /// Service events and idle outside all runtime borrows. Must not block ready work.
    pub pump: fn(),
}
#[derive(Clone, Copy)]
struct Record { id: u64, pointer: *mut u8, size: usize, entry: Option<extern "C" fn(*mut u8)>, data: *mut u8, error: i32, c_error: i32, name: [u8; 32] }
const EMPTY: Record = Record { id: 0, pointer: ptr::null_mut(), size: 0, entry: None, data: ptr::null_mut(), error: 0, c_error: 0, name: [0; 32] };
pub struct Runtime {
    executor: Executor<LIMIT>, arena: Arena<'static>, hooks: Hooks, cpu: u64,
    records: [Record; LIMIT], root_error: i32, root_c_error: i32, root_wait: usize, root_woken: bool,
}
static ACTIVE: AtomicPtr<Runtime> = AtomicPtr::new(ptr::null_mut());
impl Runtime {
    // ------------------------=
    // FUNC: new
    // DESC: Builds a runtime using only granted memory and explicit native service callbacks.
    // ------------------=
    pub fn new(bytes: &'static mut [u8], hooks: Hooks) -> Option<Self> {
        Some(Self { executor: Executor::new(), arena: Arena::new(bytes)?, cpu: (hooks.cpu)(), hooks,
            records: [EMPTY; LIMIT], root_error: 0, root_c_error: 0, root_wait: 0, root_woken: false })
    }
    // ------------------------=
    // FUNC: install
    // DESC: Publishes one stationary runtime; rejects replacement while native threads or allocations may exist.
    // ------------------=
    /// # Safety
    /// Runtime and granted heap must live permanently after success. Caller is
    /// its exclusive CPU owner; no interrupt handler may invoke this ABI.
    pub unsafe fn install(runtime: *mut Self) -> bool {
        !runtime.is_null() && ACTIVE.compare_exchange(ptr::null_mut(), runtime, Ordering::AcqRel, Ordering::Acquire).is_ok()
    }
    // ------------------------=
    // FUNC: allocated
    // DESC: Reads heap accounting on the serialized owner CPU.
    // ------------------=
    pub unsafe fn allocated() -> usize { let p = active(); if p.is_null() { 0 } else { (*p).arena.allocated() } }
    // ------------------------=
    // FUNC: peak_allocated
    // DESC: Returns high-water heap ownership on the serialized engine CPU without logging or locks.
    // ------------------=
    pub unsafe fn peak_allocated() -> usize { let p=active();if p.is_null(){0}else{(*p).arena.peak_allocated()} }
    // ------------------------=
    // FUNC: failed_request
    // DESC: Reads the latest exhausted allocation on the serialized engine CPU.
    // ------------------=
    pub unsafe fn failed_request() -> usize { let p=active();if p.is_null(){0}else{(*p).arena.failed_request()} }
    // ------------------------=
    // FUNC: diagnostic_thread_name
    // DESC: Copies a test-only bounded thread label without allocating or lending runtime state.
    // ------------------=
    #[cfg(infinity_component_trace)]
    pub unsafe fn diagnostic_thread_name(id:u64)->[u8;32] {
        let p=active(); if p.is_null(){return [0;32];}
        (*p).records.iter().find(|record|record.id==id).map_or([0;32],|record|record.name)
    }
}
// ------------------------=
// FUNC: active
// DESC: Rejects calls from CPUs other than the explicitly bound execution owner.
// ------------------=
unsafe fn active() -> *mut Runtime {
    let p = ACTIVE.load(Ordering::Acquire);
    if p.is_null() || ((*p).hooks.cpu)() != (*p).cpu { ptr::null_mut() } else { p }
}
// ------------------------=
// FUNC: reap
// DESC: Returns stacks only after the executor observes joined or detached completion.
// ------------------=
unsafe fn reap(p: *mut Runtime) {
    for index in 0..LIMIT {
        let record = (*p).records[index];
        if record.id != 0 && !(*p).executor.contains(record.id) {
            (*p).arena.release(NonNull::new(record.pointer).unwrap(), Layout::from_size_align(record.size, 16).unwrap());
            (*p).records[index] = EMPTY;
        }
    }
}
// ------------------------=
// FUNC: drive
// DESC: Services events and one runnable thread outside allocator/record borrows.
// ------------------=
unsafe fn drive(p: *mut Runtime) {
    let pump = (*p).hooks.pump; pump();
    let now = ((*p).hooks.monotonic)();
    (*p).executor.dispatch(now).expect("native owner dispatch");
    reap(p);
}
// ------------------------=
// FUNC: entry
// DESC: Calls the exact closure transferred by successful native thread creation.
// ------------------=
extern "C" fn entry(index: usize) { unsafe {
    let p = active(); assert!(!p.is_null());
    let record = (*p).records[index];
    record.entry.expect("native entry")(record.data);
} }
// ------------------------=
// FUNC: infinity_std_allocate
// DESC: Allocates from the granted owner-local heap, failing closed on invalid layout or exhaustion.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_allocate(size: usize, align: usize) -> *mut u8 {
    let p = active(); if p.is_null() { return ptr::null_mut(); }
    let Ok(payload) = Layout::from_size_align(size, align) else { return ptr::null_mut(); };
    let Ok((layout, offset)) = Layout::new::<AllocationHeader>().extend(payload) else { return ptr::null_mut(); };
    let Some(base) = (*p).arena.allocate(layout) else { return ptr::null_mut(); };
    let result = base.as_ptr().add(offset);
    result.sub(core::mem::size_of::<AllocationHeader>()).cast::<AllocationHeader>().write_unaligned(
        AllocationHeader { base: base.as_ptr(), layout, payload });
    result
}
#[derive(Clone, Copy)]
struct AllocationHeader { base: *mut u8, layout: Layout, payload: Layout }
// ------------------------=
// FUNC: infinity_std_deallocate
// DESC: Returns a live allocation with its original layout on the owning CPU.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_deallocate(pointer: *mut u8, size: usize, align: usize) {
    let p = active(); assert!(!p.is_null());
    let header = pointer.sub(core::mem::size_of::<AllocationHeader>()).cast::<AllocationHeader>().read_unaligned();
    assert_eq!(header.payload, Layout::from_size_align(size, align).expect("layout"));
    (*p).arena.release(NonNull::new(header.base).expect("allocation"), header.layout);
}

// ------------------------=
// FUNC: infinity_std_usable_size
// DESC: Reports the actual valid payload extent for a live native allocation.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_usable_size(pointer: *mut u8) -> usize {
    if pointer.is_null() || active().is_null() { return 0; }
    pointer.sub(core::mem::size_of::<AllocationHeader>()).cast::<AllocationHeader>().read_unaligned().payload.size()
}
// ------------------------=
// FUNC: infinity_c_malloc
// DESC: Provides C-aligned memory from the same governed native heap as Rust.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_c_malloc(size: usize) -> *mut u8 {
    infinity_std_allocate(size.max(1), 16)
}
// ------------------------=
// FUNC: infinity_c_free
// DESC: Frees a live native C allocation and accepts null as a no-op.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_c_free(pointer: *mut u8) {
    if pointer.is_null() { return; }
    assert!(!active().is_null());
    let header = pointer.sub(core::mem::size_of::<AllocationHeader>()).cast::<AllocationHeader>().read_unaligned();
    infinity_std_deallocate(pointer, header.payload.size(), header.payload.align());
}
// ------------------------=
// FUNC: infinity_c_realloc
// DESC: Preserves original bytes and ownership on failure, with checked size and native accounting.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_c_realloc(pointer: *mut u8, size: usize) -> *mut u8 {
    if pointer.is_null() { return infinity_c_malloc(size); }
    if size == 0 { infinity_c_free(pointer); return ptr::null_mut(); }
    if active().is_null() { return ptr::null_mut(); }
    let old_size = infinity_std_usable_size(pointer);
    let replacement = infinity_c_malloc(size);
    if !replacement.is_null() {
        ptr::copy_nonoverlapping(pointer, replacement, old_size.min(size));
        infinity_c_free(pointer);
    }
    replacement
}
// ------------------------=
// FUNC: infinity_std_stack_bounds
// DESC: Reports the current live worker's leased stack; rejects unknown root stacks instead of guessing.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_stack_bounds(low: *mut usize, high: *mut usize) -> i32 {
    let p = active();
    if p.is_null() || low.is_null() || high.is_null() { return 22; }
    let Some(id) = (*p).executor.current_id() else { return 95; };
    let Some(record) = (*p).records.iter().find(|r| r.id == id) else { return 22; };
    let start = record.pointer as usize;
    let Some(end) = start.checked_add(record.size) else { return 75; };
    low.write(start); high.write(end); 0
}
// ------------------------=
// FUNC: infinity_std_thread_create
// DESC: Allocates a bounded private stack and transfers the callback only after successful registration.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_thread_create(size: usize, callback: extern "C" fn(*mut u8), data: *mut u8, id: *mut u64) -> i32 {
    let p = active(); if p.is_null() || id.is_null() { return 22; }
    reap(p);
    let Some(index) = (*p).records.iter().position(|r| r.id == 0) else { return 11; };
    let size = size.max(16384);
    let Ok(layout) = Layout::from_size_align(size, 16) else { return 22; };
    let Some(pointer) = (*p).arena.allocate(layout) else { return 12; };
    match (*p).executor.spawn(core::slice::from_raw_parts_mut(pointer.as_ptr(), size), entry, index) {
        Ok(thread) => {
            (*p).records[index] = Record { id: thread, pointer: pointer.as_ptr(), size, entry: Some(callback), data, ..EMPTY };
            id.write(thread); 0
        }
        Err(_) => { (*p).arena.release(pointer, layout); 11 }
    }
}
// ------------------------=
// FUNC: infinity_std_thread_join
// DESC: Joins from a native thread or drives continuations from the root owner until completion.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_thread_join(id: u64) -> i32 {
    let p = active(); if p.is_null() { return 22; }
    if (*p).executor.current_id().is_some() {
        #[cfg(infinity_component_trace)]
        {
            unsafe extern "C" { fn infinity_browser_tls_trace(thread:u64,callback:usize,done:u32); }
            infinity_browser_tls_trace((*p).executor.current_id().unwrap_or(0),id as usize,2);
        }
        let result=if (*p).executor.join(id).is_ok() { 0 } else { 22 };
        #[cfg(infinity_component_trace)]
        {
            unsafe extern "C" { fn infinity_browser_tls_trace(thread:u64,callback:usize,done:u32); }
            infinity_browser_tls_trace((*p).executor.current_id().unwrap_or(0),id as usize,3);
        }
        return result;
    }
    loop {
        match (*p).executor.try_join(id) {
            Ok(true) => { reap(p); return 0; }
            Ok(false) => drive(p), Err(_) => return 22,
        }
    }
}
// ------------------------=
// FUNC: infinity_std_thread_detach
// DESC: Releases the handle while preserving active execution and delayed stack reclamation.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_thread_detach(id: u64) {
    let p = active(); assert!(!p.is_null());
    (*p).executor.detach(id).expect("joinable native handle"); reap(p);
}
// ------------------------=
// FUNC: infinity_std_thread_id
// DESC: Returns distinct stable native thread IDs with a reserved root identity.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_thread_id() -> u64 { let p = active(); if p.is_null() { 0 } else { (*p).executor.current_id().unwrap_or(u64::MAX) } }
// ------------------------=
// FUNC: infinity_std_parallelism
// DESC: Reports this runtime's actual single-CPU execution grant, not host hardware capacity.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_parallelism() -> usize { usize::from(!active().is_null()) }
// ------------------------=
// FUNC: infinity_std_yield
// DESC: Yields native threads or dispatches one continuation when called by the root owner.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_yield() {
    let p = active(); assert!(!p.is_null());
    if (*p).executor.current_id().is_some() { (*p).executor.yield_now().unwrap(); } else { drive(p); }
}
// ------------------------=
// FUNC: infinity_std_sleep
// DESC: Uses the supplied real monotonic clock to suspend rather than spin on a thread stack.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_sleep(nanoseconds: u64) -> i32 {
    let p = active(); if p.is_null() { return 22; }
    let deadline = ((*p).hooks.monotonic)().saturating_add(nanoseconds);
    if (*p).executor.current_id().is_some() { return if (*p).executor.sleep_until(deadline).is_ok() { 0 } else { 22 }; }
    while ((*p).hooks.monotonic)() < deadline { drive(p); } 0
}
// ------------------------=
// FUNC: infinity_std_wait
// DESC: Parks native threads or services work during root waits with serialized wake registration.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_wait(address: *const AtomicU32, expected: u32, nanoseconds: u64, timed: bool) -> i32 {
    let p = active(); if p.is_null() || address.is_null() { return 22; }
    let now = ((*p).hooks.monotonic)(); let deadline = timed.then(|| now.saturating_add(nanoseconds));
    if (*p).executor.current_id().is_some() {
        return match (*p).executor.wait(&*address, expected, deadline, now) {
            Ok(Outcome::TimedOut) => 110, Ok(_) => 0, Err(_) => 22,
        };
    }
    if (*p).root_wait != 0 { return 22; }
    (*p).root_wait = address as usize; (*p).root_woken = false;
    let result = loop {
        if (*address).load(Ordering::Acquire) != expected || (*p).root_woken { break 0; }
        if deadline.is_some_and(|d| ((*p).hooks.monotonic)() >= d) { break 110; }
        drive(p);
    };
    (*p).root_wait = 0; (*p).root_woken = false; result
}
// ------------------------=
// FUNC: infinity_std_wake
// DESC: Wakes the registered root waiter or selected native waiters without cross-CPU state mutation.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_wake(address: *const AtomicU32, all: bool) -> i32 {
    let p = active(); if p.is_null() || address.is_null() { return -22; }
    let mut count = 0;
    if (*p).root_wait == address as usize && !(*p).root_woken { (*p).root_woken = true; count = 1; }
    if all || count == 0 { count += (*p).executor.wake(&*address, all) as i32; }
    count
}
// ------------------------=
// FUNC: infinity_std_tls_create
// DESC: Allocates a native key for the current runtime, preserving failure semantics.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_tls_create(dtor: Option<Destructor>, key: *mut usize) -> i32 {
    let p = active(); if p.is_null() || key.is_null() { return 22; }
    match (*p).executor.key_create(dtor) { Ok(value) => { key.write(value); 0 }, Err(_) => 11 }
}
// ------------------------=
// FUNC: infinity_std_tls_destroy
// DESC: Revokes a key without running another stack's destructors.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_tls_destroy(key: usize) { let p = active(); assert!(!p.is_null()); (*p).executor.key_destroy(key).unwrap(); }
// ------------------------=
// FUNC: infinity_c_tls_destroy
// DESC: Returns an error for invalid C keys rather than panicking on caller input.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_c_tls_destroy(key: usize) -> i32 {
    let p = active();
    if p.is_null() { return 22; }
    if (*p).executor.key_destroy(key).is_ok() { 0 } else { 22 }
}
// ------------------------=
// FUNC: infinity_std_tls_get
// DESC: Gets the active independent stack's TLS value.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_tls_get(key: usize) -> *mut u8 { let p = active(); if p.is_null() { ptr::null_mut() } else { (*p).executor.tls_get(key).unwrap_or(0) as *mut u8 } }
// ------------------------=
// FUNC: infinity_std_tls_set
// DESC: Sets TLS only for a valid key in the current execution identity.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_tls_set(key: usize, value: *mut u8) -> i32 {
    let p = active(); if p.is_null() { return 22; } if (*p).executor.tls_set(key, value as usize).is_ok() { 0 } else { 22 }
}
// ------------------------=
// FUNC: infinity_std_thread_name
// DESC: Stores bounded diagnostic names without allocating or changing authority.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_thread_name(bytes: *const u8, length: usize) {
    let p = active(); if p.is_null() || bytes.is_null() { return; }
    let Some(id) = (*p).executor.current_id() else { return; };
    if let Some(r) = (*p).records.iter_mut().find(|r| r.id == id) {
        r.name.fill(0); let length = length.min(r.name.len());
        r.name[..length].copy_from_slice(core::slice::from_raw_parts(bytes, length));
    }
}
// ------------------------=
// FUNC: infinity_std_last_error
// DESC: Reads only the current execution identity's adapter error state.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_last_error() -> i32 {
    let p = active(); if p.is_null() { return 22; }
    let Some(id) = (*p).executor.current_id() else { return (*p).root_error; };
    (*p).records.iter().find(|r| r.id == id).map_or(22, |r| r.error)
}
// ------------------------=
// FUNC: infinity_c_errno_location
// DESC: Exposes stable errno storage for the current native identity, rejecting an absent or foreign owner.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_c_errno_location() -> *mut i32 {
    let p = active(); if p.is_null() { return ptr::null_mut(); }
    let Some(id) = (*p).executor.current_id() else { return &mut (*p).root_c_error; };
    (*p).records.iter_mut().find(|r| r.id == id).map_or(ptr::null_mut(), |r| &mut r.c_error)
}
// ------------------------=
// FUNC: infinity_std_clock
// DESC: Returns real normalized monotonic or UTC values; missing UTC is explicitly unsupported.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_clock(clock: u32, seconds: *mut u64, nanos: *mut u32) -> i32 {
    let p = active(); if p.is_null() || seconds.is_null() || nanos.is_null() { return 22; }
    let value = match clock { 0 => { let n = ((*p).hooks.monotonic)(); Some((n / 1_000_000_000, (n % 1_000_000_000) as u32)) },
        1 => ((*p).hooks.utc)(), _ => None };
    let Some((s, n)) = value else { return 95; }; if n >= 1_000_000_000 { return 22; }
    seconds.write(s); nanos.write(n); 0
}
// ------------------------=
// FUNC: infinity_std_entropy
// DESC: Delegates complete fills to the native entropy service and never manufactures random bytes.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_std_entropy(bytes: *mut u8, length: usize) -> i32 {
    let p = active(); if p.is_null() || bytes.is_null() { return 22; }
    if ((*p).hooks.entropy)(core::slice::from_raw_parts_mut(bytes, length)) { 0 } else { 95 }
}
