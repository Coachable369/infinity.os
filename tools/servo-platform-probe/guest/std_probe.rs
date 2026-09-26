//! Real staged Rust std execution on native runtime callbacks; no host runtime.
use core::{mem::MaybeUninit, sync::atomic::{AtomicUsize, Ordering}};
use infinity_servo_runtime_primitives::native::{Runtime, Hooks};
use std::{sync::{Arc, Mutex, Condvar}, time::Duration, thread, vec::Vec};
#[repr(align(16777216))]
struct Heap([u8; 16777216]);
static mut HEAP: Heap = Heap([0; 16777216]);
static mut RUNTIME: MaybeUninit<Runtime> = MaybeUninit::uninit();
static DROPS: AtomicUsize = AtomicUsize::new(0);
static ABI_COMPLETED: AtomicUsize = AtomicUsize::new(0);
struct Guard(Vec<u8>);
impl Drop for Guard {
    // ------------------------=
    // FUNC: drop
    // DESC: Makes real std thread-local destruction observable after joining.
    // ------------------=
    fn drop(&mut self) { assert_eq!(self.0.len(), 128); DROPS.fetch_add(1, Ordering::SeqCst); }
}
std::thread_local! { static LOCAL: Guard = Guard(std::vec![42; 128]); }
// ------------------------=
// FUNC: cpu
// DESC: Reads actual hardware CPU identity used to enforce single-owner native ABI access.
// ------------------=
pub(super) fn cpu() -> u64 {
    #[cfg(target_arch = "aarch64")]
    unsafe { let v: u64; core::arch::asm!("mrs {}, mpidr_el1", out(reg) v); v & 0xff00ffffff }
    #[cfg(target_arch = "x86_64")]
    { (core::arch::x86_64::__cpuid(1).ebx >> 24) as u64 }
}
// ------------------------=
// FUNC: monotonic
// DESC: Reads native architectural counter or HPET with hardware-reported frequency.
// ------------------=
pub(super) fn monotonic() -> u64 {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let ticks: u64; let frequency: u64;
        core::arch::asm!("isb", "mrs {}, cntvct_el0", "mrs {}, cntfrq_el0", out(reg) ticks, out(reg) frequency);
        ((ticks as u128 * 1_000_000_000) / frequency as u128) as u64
    }
    #[cfg(target_arch = "x86_64")]
    unsafe {
        let period = core::ptr::read_volatile(0xfed00000 as *const u64) >> 32;
        let ticks = core::ptr::read_volatile(0xfed000f0 as *const u64);
        ((ticks as u128 * period as u128) / 1_000_000) as u64
    }
}
// ------------------------=
// FUNC: utc
// DESC: Explicitly denies unavailable wall clock in this minimal fixture.
// ------------------=
fn utc() -> Option<(u64, u32)> { None }
// ------------------------=
// FUNC: entropy
// DESC: Explicitly denies entropy not provisioned in this thread-only fixture.
// ------------------=
fn entropy(bytes: &mut [u8]) -> bool {
    if ENTROPY_DENIED.load(core::sync::atomic::Ordering::Relaxed) { return false; }
    #[cfg(feature = "async-probe")]
    { super::entropy_probe::fill(bytes) }
    #[cfg(not(feature = "async-probe"))]
    { let _ = bytes; false }
}
static ENTROPY_DENIED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

// ------------------------=
// FUNC: entropy_roundtrip
// DESC: Exercises all pinned entropy adapters with success and service denial in the guest.
// ------------------=
#[cfg(feature = "async-probe")]
fn entropy_roundtrip() {
    let mut bytes = [0u8; 37];
    assert!(getrandom2::getrandom(&mut bytes).is_ok());
    assert!(getrandom3::fill(&mut bytes).is_ok());
    assert!(getrandom4::fill(&mut bytes).is_ok());
    ENTROPY_DENIED.store(true, core::sync::atomic::Ordering::Relaxed);
    assert!(getrandom2::getrandom(&mut bytes).is_err());
    assert!(getrandom3::fill(&mut bytes).is_err());
    assert!(getrandom4::fill(&mut bytes).is_err());
    ENTROPY_DENIED.store(false, core::sync::atomic::Ordering::Relaxed);
}
// ------------------------=
// FUNC: pump
// DESC: Yields a processor hint in the disposable no-device fixture; production must service real events.
// ------------------=
fn pump() {
    #[cfg(feature = "socket-probe")]
    super::socket_probe::pump();
    core::hint::spin_loop();
}
// ------------------------=
// FUNC: initialize
// DESC: Installs the native provider before the first std allocation.
// ------------------=
pub unsafe fn initialize() {
    #[cfg(target_arch = "x86_64")]
    { let config = 0xfed00010 as *mut u64; config.write_volatile(config.read_volatile() | 1); }
    let heap = core::slice::from_raw_parts_mut(core::ptr::addr_of_mut!(HEAP.0).cast::<u8>(), 16777216);
    let runtime = core::ptr::addr_of_mut!(RUNTIME).cast::<Runtime>();
    runtime.write(Runtime::new(heap, Hooks { cpu, monotonic, utc, entropy, pump }).unwrap());
    assert!(Runtime::install(runtime));
    std::panic::set_hook(std::boxed::Box::new(|_| super::finish(1, 0, 0)));
}
// ------------------------=
// FUNC: run
// DESC: Executes std threads, TLS, mutex/condvar, sleeps and joins entirely on native guest stacks.
// ------------------=
pub fn run() {
    abi_limits();
    allocation_roundtrip();
    stack_bounds_roundtrip();
    #[cfg(feature = "async-probe")]
    entropy_roundtrip();
    #[cfg(feature = "socket-probe")]
    super::socket_probe::run();
    #[cfg(feature = "mio-probe")]
    mio_roundtrip();
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let first_pair = pair.clone();
    let first = thread::Builder::new().name("consumer".into()).stack_size(65536).spawn(move || {
        LOCAL.with(|g| assert_eq!(g.0[0], 42));
        let (lock, cv) = &*first_pair;
        let guard = cv.wait_while(lock.lock().unwrap(), |ready| !*ready).unwrap();
        assert!(*guard); 123u64
    }).unwrap();
    let second_pair = pair.clone();
    let second = thread::Builder::new().name("producer".into()).stack_size(65536).spawn(move || {
        LOCAL.with(|g| assert_eq!(g.0[127], 42));
        thread::sleep(Duration::from_millis(2));
        *second_pair.0.lock().unwrap() = true;
        second_pair.1.notify_one();
        456u64
    }).unwrap();
    assert_eq!(first.join().unwrap(), 123);
    assert_eq!(second.join().unwrap(), 456);
    assert_eq!(DROPS.load(Ordering::SeqCst), 2);
    let before = std::time::Instant::now(); thread::sleep(Duration::from_millis(1));
    assert!(before.elapsed() >= Duration::from_millis(1));
    let locked = pair.0.lock().unwrap();
    let (_guard, outcome) = pair.1.wait_timeout(locked, Duration::from_millis(1)).unwrap();
    assert!(outcome.timed_out());
    assert_eq!(thread::available_parallelism().unwrap().get(), 1);
}

// ------------------------=
// FUNC: stack_bounds_roundtrip
// DESC: Verifies live local addresses, distinct worker stacks and fail-closed root-stack queries.
// ------------------=
fn stack_bounds_roundtrip() {
    use infinity_servo_runtime_primitives::native::infinity_std_stack_bounds;
    let (mut low, mut high) = (77, 88);
    assert_eq!(unsafe { infinity_std_stack_bounds(&mut low, &mut high) }, 95);
    assert_eq!((low, high), (77, 88));
    let first = thread::Builder::new().stack_size(65536).spawn(probe_stack_bounds).unwrap();
    let second = thread::Builder::new().stack_size(65536).spawn(probe_stack_bounds).unwrap();
    let a = first.join().unwrap();
    let b = second.join().unwrap();
    assert!(a.1 <= b.0 || b.1 <= a.0);
}
// ------------------------=
// FUNC: probe_stack_bounds
// DESC: Requires the active stack range to contain an actual local value across a cooperative sleep.
// ------------------=
fn probe_stack_bounds() -> (usize, usize) {
    use infinity_servo_runtime_primitives::native::infinity_std_stack_bounds;
    let (mut low, mut high) = (0, 0);
    assert_eq!(unsafe { infinity_std_stack_bounds(&mut low, &mut high) }, 0);
    assert_eq!(high - low, 65536);
    let local = &low as *const usize as usize;
    assert!(local >= low && local < high);
    thread::sleep(Duration::from_millis(1));
    let (mut next_low, mut next_high) = (0, 0);
    assert_eq!(unsafe { infinity_std_stack_bounds(&mut next_low, &mut next_high) }, 0);
    assert_eq!((low, high), (next_low, next_high));
    (low, high)
}
// ------------------------=
// FUNC: allocation_roundtrip
// DESC: Checks C/Rust heap sharing, alignment, resizing, exhaustion and full reclamation.
// ------------------=
fn allocation_roundtrip() { unsafe {
    use infinity_servo_runtime_primitives::native::*;
    let baseline = Runtime::allocated();
    extern "C" { fn infinity_c_allocator_test() -> i32; }
    assert_eq!(infinity_c_allocator_test(), 0);
    assert_eq!(Runtime::allocated(), baseline);
    for align in [1, 8, 16, 64, 4096] {
        let pointer = infinity_std_allocate(97, align);
        assert!(!pointer.is_null());
        assert_eq!(pointer as usize % align, 0);
        assert_eq!(infinity_std_usable_size(pointer), 97);
        pointer.write_bytes(0x5a, 97);
        infinity_std_deallocate(pointer, 97, align);
    }
    let original = infinity_c_malloc(33);
    assert!(!original.is_null());
    assert_eq!(original as usize % 16, 0);
    original.write_bytes(0x71, 33);
    assert!(infinity_c_realloc(original, usize::MAX).is_null());
    assert_eq!(infinity_std_usable_size(original), 33);
    let grown = infinity_c_realloc(original, 200);
    assert!(!grown.is_null());
    assert!(core::slice::from_raw_parts(grown, 33).iter().all(|b| *b == 0x71));
    let shrunk = infinity_c_realloc(grown, 7);
    assert!(!shrunk.is_null());
    assert!(core::slice::from_raw_parts(shrunk, 7).iter().all(|b| *b == 0x71));
    assert!(infinity_c_realloc(shrunk, 0).is_null());
    infinity_c_free(core::ptr::null_mut());
    let zero = infinity_c_malloc(0);
    assert!(!zero.is_null());
    infinity_c_free(zero);
    assert_eq!(Runtime::allocated(), baseline);
} }

// ------------------------=
// FUNC: abi_entry
// DESC: Records execution only for callbacks whose ownership was successfully transferred.
// ------------------=
extern "C" fn abi_entry(argument: *mut u8) {
    assert!((argument as usize) < 16);
    ABI_COMPLETED.fetch_add(1, Ordering::SeqCst);
}
// ------------------------=
// FUNC: abi_limits
// DESC: Verifies slot exhaustion and invalid allocations preserve ownership and all joined stacks are reclaimed.
// ------------------=
fn abi_limits() { unsafe {
    use infinity_servo_runtime_primitives::native::*;
    let baseline = Runtime::allocated();
    assert!(infinity_std_allocate(usize::MAX, 16).is_null());
    let mut ids = [0; 16];
    for (index, id) in ids.iter_mut().enumerate() {
        assert_eq!(infinity_std_thread_create(65536, abi_entry, index as *mut u8, id), 0);
    }
    let mut rejected = 0;
    assert_eq!(infinity_std_thread_create(65536, abi_entry, 99 as *mut u8, &mut rejected), 11);
    assert_eq!(rejected, 0);
    for id in ids { assert_eq!(infinity_std_thread_join(id), 0); }
    assert_eq!(ABI_COMPLETED.load(Ordering::SeqCst), 16);
    assert_eq!(Runtime::allocated(), baseline);
} }

// ------------------------=
// FUNC: mio_roundtrip
// DESC: Exercises the actual Mio Poll/Waker API on native std threads with timeout and coalescing checks.
// ------------------=
#[cfg(feature = "mio-probe")]
fn mio_roundtrip() {
    let mut poll = mio::Poll::new().unwrap();
    let registry = poll.registry().try_clone().unwrap();
    let wake = Arc::new(mio::Waker::new(&registry, mio::Token(73)).unwrap());
    let sender = wake.clone();
    let worker = thread::Builder::new().stack_size(65536).spawn(move || {
        thread::sleep(Duration::from_millis(1)); sender.wake().unwrap(); sender.wake().unwrap();
    }).unwrap();
    let mut events = mio::Events::with_capacity(4);
    poll.poll(&mut events, Some(Duration::from_secs(1))).unwrap();
    assert_eq!(events.iter().count(), 1);
    let event = events.iter().next().unwrap(); assert_eq!(event.token(), mio::Token(73));
    assert!(event.is_readable()); assert!(!event.is_writable());
    worker.join().unwrap();
    poll.poll(&mut events, Some(Duration::ZERO)).unwrap(); assert!(events.is_empty());
    let before = std::time::Instant::now();
    poll.poll(&mut events, Some(Duration::from_millis(1))).unwrap();
    assert!(events.is_empty()); assert!(before.elapsed() >= Duration::from_millis(1));
    use mio::infinity::{NativeSource, Readiness};
    let none = Readiness { readable: false, writable: false };
    let read = Readiness { readable: true, writable: false };
    let write = Readiness { readable: false, writable: true };
    let mut source = NativeSource::new(write);
    registry.register(&mut source, mio::Token(9), mio::Interest::READABLE | mio::Interest::WRITABLE).unwrap();
    poll.poll(&mut events, Some(Duration::ZERO)).unwrap();
    let event = events.iter().next().unwrap(); assert_eq!(event.token(), mio::Token(9)); assert!(event.is_writable());
    source.observe(write).unwrap(); poll.poll(&mut events, Some(Duration::ZERO)).unwrap(); assert!(events.is_empty());
    source.observe(none).unwrap(); source.observe(read).unwrap();
    poll.poll(&mut events, Some(Duration::ZERO)).unwrap(); assert!(events.iter().next().unwrap().is_readable());
    registry.reregister(&mut source, mio::Token(10), mio::Interest::READABLE).unwrap();
    poll.poll(&mut events, Some(Duration::ZERO)).unwrap(); assert_eq!(events.iter().next().unwrap().token(), mio::Token(10));
    let foreign = mio::Poll::new().unwrap(); assert!(foreign.registry().deregister(&mut source).is_err());
    source.observe(none).unwrap(); source.observe(read).unwrap(); registry.deregister(&mut source).unwrap();
    poll.poll(&mut events, Some(Duration::ZERO)).unwrap(); assert!(events.is_empty());
    let mut sources = Vec::new();
    for index in 0..31 {
        let mut source = NativeSource::new(none);
        registry.register(&mut source, mio::Token(index), mio::Interest::READABLE).unwrap(); sources.push(source);
    }
    let mut overflow = NativeSource::new(none);
    assert_eq!(registry.register(&mut overflow, mio::Token(99), mio::Interest::READABLE).unwrap_err().kind(), std::io::ErrorKind::OutOfMemory);
    drop(sources);
    registry.register(&mut overflow, mio::Token(99), mio::Interest::READABLE).unwrap();
}
