//! Bounded AP jobs. The BSP owns matrix/background queues. Native pure-math
//! callers may borrow idle APs through atomic leases without entering those queues.
//! APs never touch the engine, framebuffer, firmware, or service locks.
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
// One mailbox per secondary CPU; BSP remains dedicated to input/services.
const COUNT: usize = 64;
#[cfg(test)]
const ROWS: usize = 4096;
const WIDTH: usize = 12288;
const CACHE_FLOATS: usize = (WIDTH + WIDTH / 256 * 9 * 4) / 4;
const MATRIX_ROWS: usize = 131072;
// Smaller claims bound straggler/cancellation latency without changing row math.
const CHUNK_ROWS: usize = 32;
const RESERVED: usize = 5;
const PURE_MATH: usize = 6;
const MAX_PURE_HELPERS: usize = 3;
static NEXT_ROW: AtomicUsize = AtomicUsize::new(0);
static CANCELLED: AtomicBool = AtomicBool::new(false);
// Exclusive row ranges are assigned by NEXT_ROW; BSP reads only after all APs
// publish completion. No worker retains a pointer into engine-owned output.
static mut MATRIX_OUTPUT: [f32; MATRIX_ROWS] = [0.0; MATRIX_ROWS];
struct Job {
    background: Option<unsafe fn()>,
    pure: *const PureJob,
    data: *const u8,
    kind: u32,
    width: usize,
    rows: usize,
    finished: u64,
    elapsed: u64,
    input: [f32; WIDTH],
    cache: [f32; CACHE_FLOATS],
}
struct PureJob {
    task: unsafe extern "C" fn(*mut core::ffi::c_void),
    context: *mut core::ffi::c_void,
    remaining: AtomicUsize,
}
struct Slot {
    state: AtomicUsize,
    job: UnsafeCell<Job>,
}
// SAFETY: state release/acquire transfers exclusive access to each job buffer.
unsafe impl Sync for Slot {}
impl Slot {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates a static bounded AP mailbox.
    // ------------------=
    const fn new() -> Self {
        Self {
            state: AtomicUsize::new(0),
            job: UnsafeCell::new(Job {
                background: None,
                pure: core::ptr::null(),
                data: core::ptr::null(),
                kind: 0,
                width: 0,
                rows: 0,
                finished: 0,
                elapsed: 0,
                input: [0.0; WIDTH],
                cache: [0.0; CACHE_FLOATS],
            }),
        }
    }
}
static SLOTS: [Slot; COUNT] = [const { Slot::new() }; COUNT];
static READY: AtomicUsize = AtomicUsize::new(0);
#[cfg(not(target_os = "none"))]
static HOST_THREADS: [std::sync::Mutex<Option<std::thread::Thread>>; COUNT] =
    [const { std::sync::Mutex::new(None) }; COUNT];
// ------------------------=
// FUNC: wake_host
// DESC: Mirrors the native event wakeup for host benchmarks without burning idle host cores.
// ------------------=
#[cfg(not(target_os = "none"))]
fn wake_host(index: usize) {
    if let Some(thread) = HOST_THREADS[index].lock().unwrap().as_ref() { thread.unpark(); }
}
static COMPLETED: AtomicUsize = AtomicUsize::new(0);
static PURE_COMPLETED: AtomicUsize = AtomicUsize::new(0);
static COMPUTE_TICKS: AtomicU64 = AtomicU64::new(0);
static IDLE_TICKS: AtomicU64 = AtomicU64::new(0);
#[cfg(all(target_arch = "x86_64", target_os = "none"))]
static TSC_HZ: AtomicU64 = AtomicU64::new(0);
static Q4_TICKS: AtomicU64 = AtomicU64::new(0);
static Q6_TICKS: AtomicU64 = AtomicU64::new(0);

// ------------------------=
// FUNC: clock_ns
// DESC: Reads the native monotonic clock for bounded profiling without service calls.
// ------------------=
pub fn clock_ns() -> u64 {
    ticks_ns(counter())
}
// ------------------------=
// FUNC: ticks_ns
// DESC: Converts native counter ticks to nanoseconds; host harnesses return zero.
// ------------------=
fn ticks_ns(ticks: u64) -> u64 {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    {
        let frequency = TSC_HZ.load(Ordering::Acquire);
        if frequency != 0 {
            return ticks / frequency * 1_000_000_000 + ticks % frequency * 1_000_000_000 / frequency;
        }
    }
    #[cfg(all(target_arch = "aarch64", target_os = "none"))]
    unsafe {
        let frequency: u64;
        core::arch::asm!("mrs {0}, cntfrq_el0", out(reg) frequency, options(nomem, nostack));
        if frequency != 0 {
            return ticks / frequency * 1_000_000_000 + ticks % frequency * 1_000_000_000 / frequency;
        }
    }
    let _ = ticks;
    0
}
// ------------------------=
// FUNC: kernel_profile_ns
// DESC: Reads summed AP compute time by quantization and completed-job wait time.
// ------------------=
pub fn kernel_profile_ns() -> [u64; 3] {
    [ticks_ns(Q4_TICKS.load(Ordering::Relaxed)), ticks_ns(Q6_TICKS.load(Ordering::Relaxed)),
        ticks_ns(IDLE_TICKS.load(Ordering::Relaxed))]
}

// ------------------------=
// FUNC: counter
// DESC: Reads the shared ARM counter without logging, allocation, or firmware calls.
// ------------------=
fn counter() -> u64 {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    unsafe {
        core::arch::x86_64::_mm_lfence();
        return core::arch::x86_64::_rdtsc();
    }
    #[cfg(all(target_arch = "aarch64", target_os = "none"))]
    unsafe {
        let value: u64;
        core::arch::asm!("mrs {0}, cntvct_el0", out(reg) value, options(nomem, nostack));
        return value;
    }
    #[cfg(not(all(any(target_arch = "aarch64", target_arch = "x86_64"), target_os = "none")))]
    0
}
// ------------------------=
// FUNC: infinity_speech_clock_ns
// DESC: Supplies the shared native speech engine with the calibrated x86 monotonic counter.
// ------------------=
#[cfg(all(target_arch = "x86_64", target_os = "none"))]
#[no_mangle]
pub extern "C" fn infinity_speech_clock_ns() -> u64 { clock_ns() }
// ------------------------=
// FUNC: profile_ms
// DESC: Reports cumulative summed worker compute and completed-result waiting time in milliseconds.
// ------------------=
pub fn profile_ms() -> (u64, u64) {
    (ticks_ns(COMPUTE_TICKS.load(Ordering::Relaxed)) / 1_000_000,
     ticks_ns(IDLE_TICKS.load(Ordering::Relaxed)) / 1_000_000)
}
// BSP-only submission state. No pointers into movable engine memory are retained.
static mut PENDING: (usize, usize, bool) = (0, 0, false);
// BSP-owned single waiting task. Matrix collection releases its inputs before
// dispatching this task, so streaming inference cannot repeatedly win every AP.
static mut WAITING_BACKGROUND: Option<unsafe fn()> = None;

// ------------------------=
// FUNC: online
// DESC: Reports CPUs that actually entered the native worker loop, not firmware's advertised count.
// ------------------=
pub fn online() -> usize {
    READY.load(Ordering::Acquire).count_ones() as usize
}
// ------------------------=
// FUNC: completed
// DESC: Reports completed native AP jobs for installed-system diagnostics.
// ------------------=
pub fn completed() -> usize {
    COMPLETED.load(Ordering::Relaxed)
}
// ------------------------=
// FUNC: parallel_completed
// DESC: Reports completed borrowed helper callbacks, excluding work performed by their calling owner.
// ------------------=
pub fn parallel_completed() -> usize {
    PURE_COMPLETED.load(Ordering::Relaxed)
}

// ------------------------=
// FUNC: run_host_worker
// DESC: Exercises the production mailbox and kernels in the native benchmark without firmware startup.
// ------------------=
#[cfg(not(target_os = "none"))]
pub unsafe fn run_host_worker(index: usize) {
    assert!(index < COUNT);
    worker_entry((index + 1) as *mut u8);
}

// ------------------------=
// FUNC: stop_host_workers
// DESC: Retires benchmark work before asking host workers to exit through the existing mailbox protocol.
// ------------------=
#[cfg(not(target_os = "none"))]
pub unsafe fn stop_host_workers() {
    drain();
    let ready = READY.load(Ordering::Acquire);
    for (i, slot) in SLOTS.iter().enumerate() {
        if ready & (1 << i) != 0 {
            while slot.state.compare_exchange(0, 3, Ordering::AcqRel, Ordering::Acquire).is_err() {
                core::hint::spin_loop();
            }
            wake_host(i);
        }
    }
}

// ------------------------=
// FUNC: background
// DESC: Submits one bounded non-rendering native task to an idle AP without occupying the UI thread.
// ------------------=
/// Safety: BSP-only; task must own its static buffers, honor cancellation and
/// deadlines, and never access UI, firmware, runtime locks, or matrix queues.
/// A task may use infinity_speech_parallel only for its separately documented
/// allocation-free pure-math contract.
pub unsafe fn background(task: unsafe fn()) -> bool {
    if WAITING_BACKGROUND.is_some() { return false; }
    let ready = READY.load(Ordering::Acquire);
    for (index, slot) in SLOTS.iter().enumerate() {
        if ready & (1 << index) != 0 && slot.state.compare_exchange(0, RESERVED,
            Ordering::AcqRel, Ordering::Acquire).is_ok() {
            (*slot.job.get()).background = Some(task);
            slot.state.store(4, Ordering::Release);
            #[cfg(not(target_os = "none"))]
            wake_host(index);
            #[cfg(all(target_arch = "aarch64", target_os = "none"))]
            core::arch::asm!("sev", options(nomem, nostack));
            return true;
        }
    }
    if PENDING.0 != 0 {
        WAITING_BACKGROUND = Some(task);
        return true;
    }
    false
}

// ------------------------=
// FUNC: infinity_speech_parallel
// DESC: Borrows up to three idle APs for one allocation-free callback and joins them before returning.
// ------------------=
/// Safety: called off the BSP by a bounded native job. The callback owns an
/// atomic work queue and writes only disjoint output; it may not allocate, use
/// TLS, access runtime/firmware locks, unwind, abort, or dispatch nested work.
/// It must return on cancellation, with each unit of work bounded. Context and
/// referenced buffers stay alive until this call returns. No unclaimed AP is
/// awaited, and the first online AP remains available for the browser/services.
#[no_mangle]
pub unsafe extern "C" fn infinity_speech_parallel(
    task: unsafe extern "C" fn(*mut core::ffi::c_void),
    context: *mut core::ffi::c_void,
) -> usize {
    let job = PureJob { task, context, remaining: AtomicUsize::new(0) };
    let ready = READY.load(Ordering::Acquire);
    let reserved = ready.trailing_zeros() as usize;
    let mut helpers = 0;
    for (index, slot) in SLOTS.iter().enumerate() {
        if helpers == MAX_PURE_HELPERS { break; }
        if index == reserved || ready & (1 << index) == 0 || slot.state.compare_exchange(
            0, RESERVED, Ordering::AcqRel, Ordering::Acquire).is_err() { continue; }
        (*slot.job.get()).pure = &job;
        job.remaining.fetch_add(1, Ordering::Relaxed);
        helpers += 1;
        slot.state.store(PURE_MATH, Ordering::Release);
        #[cfg(not(target_os = "none"))]
        wake_host(index);
    }
    #[cfg(all(target_arch = "aarch64", target_os = "none"))]
    if helpers != 0 { core::arch::asm!("sev", options(nomem, nostack)); }
    task(context);
    // Never abandon a callback that can still access the caller's stack or
    // tensor buffers, including after cancellation. Callbacks do not wait for
    // one another, so an accepted cohort cannot deadlock at a graph barrier.
    while job.remaining.load(Ordering::Acquire) != 0 {
        #[cfg(not(target_os = "none"))]
        std::thread::yield_now();
        #[cfg(target_os = "none")]
        core::hint::spin_loop();
    }
    helpers
}

// ------------------------=
// FUNC: dispatch_waiting_background
// DESC: Gives an accepted latency-sensitive task the first freed AP at a matrix boundary.
// ------------------=
unsafe fn dispatch_waiting_background() {
    if let Some(task) = WAITING_BACKGROUND.take() {
        if !background(task) { WAITING_BACKGROUND = Some(task); }
    }
}

// ------------------------=
// FUNC: poll_background
// DESC: Retires abandoned matrix ownership and advances accepted background work without waiting.
// ------------------=
pub fn poll_background() {
    unsafe {
        // A cancelled engine no longer calls rows(), so its completed mailboxes
        // must be reclaimed here before an accepted speech task can run.
        if PENDING.2 { collect_completed_matrix(); }
        dispatch_waiting_background();
    }
}

// ------------------------=
// FUNC: collect_completed_matrix
// DESC: Nonblockingly retires a completed matrix while preserving its result ownership and profiling.
// ------------------=
unsafe fn collect_completed_matrix() -> Option<(usize, bool)> {
    let (mask, start, discarded) = PENDING;
    if mask == 0 || (0..COUNT).any(|i| mask & (1 << i) != 0
        && SLOTS[i].state.load(Ordering::Acquire) != 2) {
        return None;
    }
    for (i, slot) in SLOTS.iter().enumerate() {
        if mask & (1 << i) == 0 { continue; }
        let job = &*slot.job.get();
        COMPUTE_TICKS.fetch_add(job.elapsed, Ordering::Relaxed);
        if job.kind == 12 {
            Q4_TICKS.fetch_add(job.elapsed, Ordering::Relaxed);
        } else if job.kind == 14 {
            Q6_TICKS.fetch_add(job.elapsed, Ordering::Relaxed);
        }
        IDLE_TICKS.fetch_add(counter().saturating_sub(job.finished), Ordering::Relaxed);
        slot.state.store(0, Ordering::Release);
    }
    PENDING = (0, 0, false);
    Some((start, discarded))
}

// ------------------------=
// FUNC: initialize
// DESC: Starts native workers through the target's versioned bootstrap adapter.
// ------------------=
pub unsafe fn initialize(address: u64) {
    #[cfg(target_os = "none")]
    if address != 0 {
        #[repr(C)]
        struct Bridge {
            version: u64,
            start: unsafe extern "efiapi" fn(unsafe extern "efiapi" fn(*mut u8), u64) -> u64,
        }
        let bridge = &*(address as *const Bridge);
        #[cfg(target_arch = "x86_64")]
        if bridge.version == 2 {
            let frequency = *((address as *const u64).add(2));
            if !(1_000_000..=10_000_000_000).contains(&frequency) { return; }
            TSC_HZ.store(frequency, Ordering::Release);
            (bridge.start)(worker_entry, COUNT as u64);
        }
        #[cfg(target_arch = "aarch64")]
        if bridge.version == 1 {
            (bridge.start)(worker_entry, COUNT as u64);
        }
    }
    let _ = address;
}

// ------------------------=
// FUNC: worker_entry
// DESC: Runs quantized rows on an AP using private buffers and release/acquire mailbox ownership.
// ------------------=
unsafe extern "efiapi" fn worker_entry(argument: *mut u8) {
    let index = argument as usize - 1;
    if index >= COUNT {
        return;
    }
    #[cfg(all(target_arch = "aarch64", target_os = "none"))]
    {
        let level: u64;
        let mut control: u64;
        core::arch::asm!("msr daifset, #0xf", "mrs {0}, CurrentEL", out(reg) level);
        if level == 4 {
            core::arch::asm!("mrs {0}, cpacr_el1",out(reg) control);
            control |= 3 << 20;
            core::arch::asm!("msr cpacr_el1, {0}","isb",in(reg) control);
        } else if level == 8 {
            core::arch::asm!("mrs {0}, cptr_el2",out(reg) control);
            control &= !(1 << 10);
            core::arch::asm!("msr cptr_el2, {0}","isb",in(reg) control);
        } else {
            return;
        }
    }
    #[cfg(not(target_os = "none"))]
    { *HOST_THREADS[index].lock().unwrap() = Some(std::thread::current()); }
    READY.fetch_or(1 << index, Ordering::Release);
    let slot = &SLOTS[index];
    loop {
        match slot.state.load(Ordering::Acquire) {
            1 => {
                let job = &mut *slot.job.get();
                let started = counter();
                extern "C" {
                    fn infinity_qwen_dot_rows_cached(
                        kind: u32,
                        data: *const u8,
                        input: *const f32,
                        width: usize,
                        rows: usize,
                        output: *mut f32,
                        scratch: *mut f32,
                        refresh: i32,
                    );
                }
                let stride = job.width / 256 * if job.kind == 12 { 144 } else { 210 };
                let mut refresh = 1;
                while !CANCELLED.load(Ordering::Acquire) {
                    let at = NEXT_ROW.fetch_add(CHUNK_ROWS, Ordering::Relaxed);
                    if at >= job.rows { break; }
                    infinity_qwen_dot_rows_cached(job.kind, job.data.add(at * stride),
                        job.input.as_ptr(), job.width, CHUNK_ROWS.min(job.rows - at),
                        (&raw mut MATRIX_OUTPUT).cast::<f32>().add(at),
                        job.cache.as_mut_ptr(), refresh);
                    refresh = 0;
                }
                job.finished = counter();
                job.elapsed = job.finished.saturating_sub(started);
                COMPLETED.fetch_add(1, Ordering::Relaxed);
                slot.state.store(2, Ordering::Release);
                #[cfg(all(target_arch = "aarch64", target_os = "none"))]
                core::arch::asm!("sev", options(nomem, nostack));
            }
            3 => {
                READY.fetch_and(!(1 << index), Ordering::Release);
                return;
            }
            4 => {
                if let Some(task) = (*slot.job.get()).background { task(); }
                (*slot.job.get()).background = None;
                slot.state.store(0, Ordering::Release);
            }
            PURE_MATH => {
                let job = (*slot.job.get()).pure;
                ((*job).task)((*job).context);
                (*slot.job.get()).pure = core::ptr::null();
                PURE_COMPLETED.fetch_add(1, Ordering::Relaxed);
                // This is the final access to the borrowed descriptor. The
                // owner may retire it as soon as the acquire join sees zero.
                (*job).remaining.fetch_sub(1, Ordering::Release);
                slot.state.store(0, Ordering::Release);
            }
            _ => {
                #[cfg(all(target_arch = "aarch64", target_os = "none"))]
                core::arch::asm!("wfe", options(nomem, nostack));
                #[cfg(not(target_os = "none"))]
                std::thread::park();
                #[cfg(all(target_arch = "x86_64", target_os = "none"))]
                core::hint::spin_loop();
            }
        }
    }
}

// ------------------------=
// FUNC: discard
// DESC: Invalidates pending results without reclaiming buffers still owned by workers.
// ------------------=
pub fn discard() {
    CANCELLED.store(true, Ordering::Release);
    unsafe {
        PENDING.2 = true;
    }
}

// ------------------------=
// FUNC: rows
// DESC: Publishes one matrix queue to all available APs and collects disjoint staging rows; unsupported shapes fall back.
// ------------------=
/// Safety: BSP-only, one engine; weights must remain alive until collection or
/// drain. While pending, callers must keep the same destination geometry unless
/// discard was called. Workers never retain activation/destination references.
pub unsafe fn rows(
    kind: u32,
    data: &[u8],
    input: &[f32],
    output: &mut [f32],
    cursor: &mut usize,
) -> Option<bool> {
    unsafe {
        if PENDING.0 != 0 {
            let Some((start, discarded)) = collect_completed_matrix() else {
                return Some(false);
            };
            dispatch_waiting_background();
            if !discarded {
                core::ptr::copy_nonoverlapping(
                    (&raw const MATRIX_OUTPUT).cast::<f32>().add(start),
                    output.as_mut_ptr().add(start), output.len() - start);
                *cursor = 0;
                return Some(true);
            }
        }
        let ready = READY.load(Ordering::Acquire);
        if ready == 0
            || !matches!(kind, 12 | 14)
            || input.is_empty()
            || input.len() > WIDTH
            || input.len() % 256 != 0
            || *cursor >= output.len()
            || output.len() > MATRIX_ROWS
        {
            return None;
        }
        let stride = input.len() / 256 * if kind == 12 { 144 } else { 210 };
        if data.len() < output.len().checked_mul(stride)? {
            return None;
        }
        NEXT_ROW.store(*cursor, Ordering::Relaxed);
        CANCELLED.store(false, Ordering::Release);
        let idle = SLOTS.iter().enumerate().filter(|(i, slot)| ready & (1 << i) != 0
            && slot.state.load(Ordering::Acquire) == 0).count();
        // Keep one AP available for the permanent browser worker and latency-sensitive
        // speech jobs. A single-AP target still makes forward progress through the
        // existing matrix-boundary handoff.
        let mut reserve = idle > 1;
        let mut mask: usize = 0;
        for (i, slot) in SLOTS.iter().enumerate() {
            if ready & (1 << i) == 0 || slot.state.load(Ordering::Acquire) != 0 {
                continue;
            }
            if reserve { reserve = false; continue; }
            if slot.state.compare_exchange(0, RESERVED,
                Ordering::AcqRel, Ordering::Acquire).is_err() { continue; }
            let job = &mut *slot.job.get();
            job.rows = output.len();
            job.width = input.len();
            job.kind = kind;
            job.data = data.as_ptr();
            job.input[..input.len()].copy_from_slice(input);
            mask |= 1 << i;
            slot.state.store(1, Ordering::Release);
            #[cfg(not(target_os = "none"))]
            wake_host(i);
        }
        if mask == 0 { return None; }
        PENDING = (mask, *cursor, false);
        #[cfg(all(target_arch = "aarch64", target_os = "none"))]
        core::arch::asm!("sev", options(nomem, nostack));
        Some(false)
    }
}

// ------------------------=
// FUNC: drain
// DESC: Retires outstanding jobs before model storage can be released; never used during ordinary cancellation.
// ------------------=
pub fn drain() {
    unsafe {
        let mask = PENDING.0;
        for (i, slot) in SLOTS.iter().enumerate() {
            if mask & (1 << i) == 0 {
                continue;
            }
            while slot.state.load(Ordering::Acquire) != 2 {
                core::hint::spin_loop();
            }
            slot.state.store(0, Ordering::Release);
        }
        PENDING = (0, 0, false);
        dispatch_waiting_background();
    }
}

#[cfg(test)]
mod tests {
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    static PURE_CONTEXT: core::sync::atomic::AtomicPtr<PureFixture> =
        core::sync::atomic::AtomicPtr::new(core::ptr::null_mut());
    struct PureFixture {
        next: AtomicUsize,
        visits: [AtomicUsize; 512],
        active: AtomicUsize,
        finished: AtomicUsize,
        released: AtomicBool,
        cancel: AtomicBool,
        helpers: AtomicUsize,
        done: AtomicBool,
    }
    impl PureFixture {
        // ------------------------=
        // FUNC: new
        // DESC: Allocates bounded atomic observation state for native helper ownership tests.
        // ------------------=
        fn new(released: bool) -> Self {
            Self { next: AtomicUsize::new(0), visits: [const { AtomicUsize::new(0) }; 512],
                active: AtomicUsize::new(0), finished: AtomicUsize::new(0),
                released: AtomicBool::new(released), cancel: AtomicBool::new(false),
                helpers: AtomicUsize::new(0), done: AtomicBool::new(false) }
        }
    }
    // ------------------------=
    // FUNC: pure_fixture_work
    // DESC: Claims disjoint observable tasks and honors cancellation without retaining the borrowed context.
    // ------------------=
    unsafe extern "C" fn pure_fixture_work(context: *mut core::ffi::c_void) {
        let fixture = &*(context.cast::<PureFixture>());
        fixture.active.fetch_add(1, Ordering::AcqRel);
        while !fixture.released.load(Ordering::Acquire) && !fixture.cancel.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        while !fixture.cancel.load(Ordering::Acquire) {
            let next = fixture.next.fetch_add(1, Ordering::Relaxed);
            if next >= fixture.visits.len() { break; }
            fixture.visits[next].fetch_add(1, Ordering::Relaxed);
            std::thread::yield_now();
        }
        fixture.active.fetch_sub(1, Ordering::Relaxed);
        fixture.finished.fetch_add(1, Ordering::Release);
    }
    // ------------------------=
    // FUNC: pure_fixture_owner
    // DESC: Exercises helper borrowing from an actual background AP, including its owner slot exclusion.
    // ------------------=
    unsafe fn pure_fixture_owner() {
        let context = PURE_CONTEXT.load(Ordering::Acquire);
        let helpers = infinity_speech_parallel(pure_fixture_work, context.cast());
        (*context).helpers.store(helpers, Ordering::Relaxed);
        (*context).done.store(true, Ordering::Release);
    }
    // ------------------------=
    // FUNC: await_condition
    // DESC: Bounds behavioral test coordination without sleeps or production timeout changes.
    // ------------------=
    fn await_condition(condition: impl Fn() -> bool) {
        let started = std::time::Instant::now();
        while !condition() {
            assert!(started.elapsed().as_secs() < 5);
            std::thread::yield_now();
        }
    }
    // ------------------------=
    // FUNC: start_fixture_workers
    // DESC: Starts a fresh isolated worker cohort after every preceding job and AP has retired.
    // ------------------=
    fn start_fixture_workers(count: usize) -> Vec<std::thread::JoinHandle<()>> {
        assert_eq!(online(), 0);
        for slot in &SLOTS { slot.state.store(0, Ordering::Release); }
        let threads: Vec<_> = (1..=count)
            .map(|id| std::thread::spawn(move || unsafe { worker_entry(id as *mut u8) })).collect();
        await_condition(|| online() == count);
        threads
    }
    // ------------------------=
    // FUNC: finish_fixture_workers
    // DESC: Joins every retired host AP before another test may reuse static scheduler state.
    // ------------------=
    fn finish_fixture_workers(threads: Vec<std::thread::JoinHandle<()>>) {
        unsafe { stop_host_workers(); }
        for thread in threads { thread.join().unwrap(); }
        assert_eq!(online(), 0);
    }
    // ------------------------=
    // FUNC: pure_helpers_preserve_ownership_and_quiesce
    // DESC: Verifies native borrowing fallback, BSP progress, concurrent claims, repeated buffers, and cancellation lifetime.
    // ------------------=
    #[test]
    fn pure_helpers_preserve_ownership_and_quiesce() {
        let _guard = TEST_LOCK.lock().unwrap();
        let mut empty = PureFixture::new(true);
        let before = parallel_completed();
        assert_eq!(unsafe { infinity_speech_parallel(pure_fixture_work, (&mut empty as *mut PureFixture).cast()) }, 0);
        assert!(empty.visits.iter().all(|count| count.load(Ordering::Acquire) == 1));
        assert_eq!(parallel_completed(), before);

        // The single online AP is both the caller and protected service slot.
        let threads = start_fixture_workers(1);
        let mut single = PureFixture::new(true);
        PURE_CONTEXT.store(&mut single, Ordering::Release);
        assert!(unsafe { background(pure_fixture_owner) });
        await_condition(|| single.done.load(Ordering::Acquire));
        assert_eq!(single.helpers.load(Ordering::Acquire), 0);
        assert!(single.visits.iter().all(|count| count.load(Ordering::Acquire) == 1));
        assert_eq!(parallel_completed(), before);
        finish_fixture_workers(threads);

        let threads = start_fixture_workers(3);
        // Keep the first online AP idle while an off-BSP caller borrows the
        // other two. A real background callback must still obtain that AP.
        let reserved = std::sync::Arc::new(PureFixture::new(false));
        let owner_context = reserved.clone();
        let owner = std::thread::spawn(move || unsafe {
            infinity_speech_parallel(pure_fixture_work,
                std::sync::Arc::as_ptr(&owner_context).cast_mut().cast())
        });
        await_condition(|| reserved.active.load(Ordering::Acquire) == 3);
        QUEUED_BACKGROUND_RAN.store(false, Ordering::Release);
        assert!(unsafe { background(queued_background) });
        await_condition(|| QUEUED_BACKGROUND_RAN.load(Ordering::Acquire));
        assert_eq!(reserved.finished.load(Ordering::Acquire), 0);
        reserved.released.store(true, Ordering::Release);
        assert_eq!(owner.join().unwrap(), 2);
        assert_eq!(reserved.active.load(Ordering::Acquire), 0);
        assert_eq!(reserved.finished.load(Ordering::Acquire), 3);
        assert!(reserved.visits.iter().all(|count| count.load(Ordering::Acquire) == 1));
        await_condition(|| SLOTS.iter().take(3).all(|slot| slot.state.load(Ordering::Acquire) == 0));

        // A four-vCPU machine has only three APs. A permanent browser owns
        // one, the speech owner uses the second, and only one helper remains.
        BACKGROUND_RELEASE.store(false, Ordering::Release);
        BACKGROUND_STARTED.store(false, Ordering::Release);
        assert!(unsafe { background(held_background) });
        await_condition(|| BACKGROUND_STARTED.load(Ordering::Acquire));
        let mut constrained = PureFixture::new(false);
        PURE_CONTEXT.store(&mut constrained, Ordering::Release);
        assert!(unsafe { background(pure_fixture_owner) });
        await_condition(|| constrained.active.load(Ordering::Acquire) == 2);
        let mut busy = PureFixture::new(true);
        assert_eq!(unsafe { infinity_speech_parallel(pure_fixture_work, (&mut busy as *mut PureFixture).cast()) }, 0);
        assert!(busy.visits.iter().all(|count| count.load(Ordering::Acquire) == 1));
        constrained.released.store(true, Ordering::Release);
        await_condition(|| constrained.done.load(Ordering::Acquire));
        assert_eq!(constrained.helpers.load(Ordering::Acquire), 1);
        assert_eq!(constrained.active.load(Ordering::Acquire), 0);
        assert_eq!(constrained.finished.load(Ordering::Acquire), 2);
        assert!(constrained.visits.iter().all(|count| count.load(Ordering::Acquire) == 1));
        BACKGROUND_RELEASE.store(true, Ordering::Release);
        assert_eq!(parallel_completed() - before, 3);
        finish_fixture_workers(threads);

        let before = parallel_completed();
        let threads = start_fixture_workers(6);
        let data = vec![0u8; 256 * 144];
        let input = vec![0.125f32; 256];
        for cancelled in [false, true] {
            let mut fixture = PureFixture::new(false);
            PURE_CONTEXT.store(&mut fixture, Ordering::Release);
            assert!(unsafe { background(pure_fixture_owner) });
            await_condition(|| fixture.active.load(Ordering::Acquire) == 4);
            // All borrowed contexts remain live, yet BSP matrix and ordinary
            // background submissions and collection must continue independently.
            let mut output = vec![f32::NAN; 256];
            let mut cursor = 0;
            assert_eq!(unsafe { rows(12, &data, &input, &mut output, &mut cursor) }, Some(false));
            BACKGROUND_RELEASE.store(false, Ordering::Release);
            BACKGROUND_STARTED.store(false, Ordering::Release);
            assert!(unsafe { background(held_background) });
            await_condition(|| BACKGROUND_STARTED.load(Ordering::Acquire));
            let mut busy = PureFixture::new(true);
            assert_eq!(unsafe { infinity_speech_parallel(pure_fixture_work, (&mut busy as *mut PureFixture).cast()) }, 0);
            assert!(busy.visits.iter().all(|count| count.load(Ordering::Acquire) == 1));
            QUEUED_BACKGROUND_RAN.store(false, Ordering::Release);
            assert!(unsafe { background(queued_background) });
            let deadline = std::time::Instant::now();
            while unsafe { rows(12, &data, &input, &mut output, &mut cursor) } != Some(true) {
                assert!(deadline.elapsed().as_secs() < 5);
                std::thread::yield_now();
            }
            await_condition(|| QUEUED_BACKGROUND_RAN.load(Ordering::Acquire));
            assert!(output.iter().all(|value| *value == 0.0));
            assert!(!fixture.done.load(Ordering::Acquire));
            if cancelled { fixture.cancel.store(true, Ordering::Release); }
            else { fixture.released.store(true, Ordering::Release); }
            await_condition(|| fixture.done.load(Ordering::Acquire));
            assert_eq!(fixture.helpers.load(Ordering::Acquire), 3);
            assert_eq!(fixture.active.load(Ordering::Acquire), 0);
            assert_eq!(fixture.finished.load(Ordering::Acquire), 4);
            assert!(fixture.visits.iter().all(|count| count.load(Ordering::Acquire) == usize::from(!cancelled)));
            BACKGROUND_RELEASE.store(true, Ordering::Release);
            await_condition(|| SLOTS.iter().take(6).all(|slot| slot.state.load(Ordering::Acquire) == 0));
        }
        assert_eq!(parallel_completed() - before, 6);

        // Repeated actual AP owners compete with BSP matrix claims. Reclaiming
        // a fixture after done must never leave helpers writing its old buffer.
        let mut helper_total = 6;
        for _ in 0..64 {
            let mut fixture = PureFixture::new(true);
            PURE_CONTEXT.store(&mut fixture, Ordering::Release);
            assert!(unsafe { background(pure_fixture_owner) });
            let mut output = vec![f32::NAN; 256];
            let mut cursor = 0;
            let deadline = std::time::Instant::now();
            loop {
                let result = unsafe { rows(12, &data, &input, &mut output, &mut cursor) };
                if result == Some(true) { break; }
                assert!(deadline.elapsed().as_secs() < 5);
                std::thread::yield_now();
            }
            await_condition(|| fixture.done.load(Ordering::Acquire));
            let helpers = fixture.helpers.load(Ordering::Acquire);
            assert!(helpers <= MAX_PURE_HELPERS);
            helper_total += helpers;
            assert_eq!(fixture.active.load(Ordering::Acquire), 0);
            assert_eq!(fixture.finished.load(Ordering::Acquire), helpers + 1);
            assert!(fixture.visits.iter().all(|count| count.load(Ordering::Acquire) == 1));
            assert!(output.iter().all(|value| *value == 0.0));
        }
        assert_eq!(parallel_completed() - before, helper_total);
        finish_fixture_workers(threads);
    }
    static QUEUED_BACKGROUND_RAN: AtomicBool = AtomicBool::new(false);
    // ------------------------=
    // FUNC: queued_background
    // DESC: Observes execution of a task accepted while every available AP belonged to a matrix.
    // ------------------=
    unsafe fn queued_background() { QUEUED_BACKGROUND_RAN.store(true, Ordering::Release); }
    use super::*;
    static BACKGROUND_STARTED: AtomicBool = AtomicBool::new(false);
    static BACKGROUND_RELEASE: AtomicBool = AtomicBool::new(false);
    // ------------------------=
    // FUNC: held_background
    // DESC: Occupies one AP while the production matrix scheduler uses the remaining workers.
    // ------------------=
    unsafe fn held_background() {
        BACKGROUND_STARTED.store(true, Ordering::Release);
        while !BACKGROUND_RELEASE.load(Ordering::Acquire) { std::thread::yield_now(); }
    }
    extern "C" {
        fn infinity_qwen_dot(
            kind: u32,
            data: *const u8,
            input: *const f32,
            width: usize,
            output: *mut f32,
        );
    }
    // ------------------------=
    // FUNC: outputs_match
    // DESC: Requires exact Q6 output and bounded normalized Q4 activation-quantization error.
    // ------------------=
    fn outputs_match(kind: u32, expected: &[f32], actual: &[f32]) -> bool {
        if expected.len() != actual.len() {
            return false;
        }
        if kind == 14 {
            return expected.iter().zip(actual).all(|(a, b)| a.to_bits() == b.to_bits());
        }
        let (error, reference) = expected.iter().zip(actual).fold((0.0f64, 0.0f64), |sum, (a, b)| {
            let delta = *a as f64 - *b as f64;
            (sum.0 + delta * delta, sum.1 + *a as f64 * *a as f64)
        });
        actual.iter().all(|value| value.is_finite())
            && (error / (reference + expected.len() as f64)).sqrt() <= 0.02
    }
    // ------------------------=
    // FUNC: concurrent_rows_and_cancellation
    // DESC: Compares real concurrent kernels with direct execution and retires cancelled jobs before reusing buffers.
    // ------------------=
    #[test]
    fn concurrent_rows_and_cancellation() {
        let _guard = TEST_LOCK.lock().unwrap();
        BACKGROUND_RELEASE.store(false, Ordering::Release);
        BACKGROUND_STARTED.store(false, Ordering::Release);
        let threads = start_fixture_workers(6);
        let deadline = std::time::Instant::now();
        while online() != 6 {
            assert!(deadline.elapsed().as_secs() < 5);
            std::thread::yield_now();
        }
        assert!(unsafe { background(held_background) });
        while !BACKGROUND_STARTED.load(Ordering::Acquire) {
            assert!(deadline.elapsed().as_secs() < 5);
            std::thread::yield_now();
        }
        for kind in [12, 14] {
            let stride = if kind == 12 { 144 } else { 210 };
            let count = 6 * ROWS + 1;
            let mut data = vec![0u8; count * stride];
            for (i, b) in data.iter_mut().enumerate() {
                *b = (i * 37 + 19) as u8;
            }
            for row in data.chunks_mut(stride) {
                let at = if kind == 12 { 0 } else { 208 };
                row[at..at + 2].copy_from_slice(&0x3800u16.to_le_bytes());
                if kind == 12 {
                    row[2..4].copy_from_slice(&0x3400u16.to_le_bytes());
                }
            }
            let input: Vec<f32> = (0..256).map(|i| (i as f32 - 128.0) / 128.0).collect();
            let mut expected = vec![0.0; count];
            for (row, out) in expected.iter_mut().enumerate() {
                unsafe {
                    infinity_qwen_dot(
                        kind,
                        data.as_ptr().add(row * stride),
                        input.as_ptr(),
                        256,
                        out,
                    );
                }
            }
            let mut actual = vec![0.0; count];
            let mut cursor = 0;
            assert_eq!(
                unsafe { rows(kind, &data, &input, &mut actual, &mut cursor) },
                Some(false)
            );
            QUEUED_BACKGROUND_RAN.store(false, Ordering::Release);
            assert!(unsafe { background(queued_background) });
            while !QUEUED_BACKGROUND_RAN.load(Ordering::Acquire) {
                assert!(deadline.elapsed().as_secs() < 5);
                std::thread::yield_now();
            }
            discard();
            // New geometry is deliberately smaller than the abandoned job.
            let mut tiny = vec![0.0; 1];
            let deadline = std::time::Instant::now();
            while unsafe { rows(kind, &data, &input, &mut tiny, &mut cursor) } != Some(true) {
                assert!(deadline.elapsed().as_secs() < 5);
                std::thread::yield_now();
            }
            assert!(outputs_match(kind, &expected[..1], &tiny));
            let deadline = std::time::Instant::now();
            while unsafe { rows(kind, &data, &input, &mut actual, &mut cursor) } != Some(true) {
                assert!(deadline.elapsed().as_secs() < 5);
                std::thread::yield_now();
            }
            assert!(outputs_match(kind, &expected, &actual));
            drain();
            // A partial matrix must preserve rows before the starting cursor.
            let prefix = 17;
            cursor = prefix;
            actual.fill(f32::NAN);
            let deadline = std::time::Instant::now();
            while unsafe { rows(kind, &data, &input, &mut actual, &mut cursor) } != Some(true) {
                assert!(deadline.elapsed().as_secs() < 5);
                std::thread::yield_now();
            }
            assert!(actual[..prefix].iter().all(|value| value.is_nan()));
            assert!(outputs_match(kind, &expected[prefix..], &actual[prefix..]));
            let mut oversized = vec![0.0; MATRIX_ROWS + 1];
            assert_eq!(unsafe { rows(kind, &[], &input, &mut oversized, &mut cursor) }, None);
            // Exercise the compact per-worker cache with the largest activation.
            let wide_input = vec![0.125f32; WIDTH];
            let wide_rows = CHUNK_ROWS * 7 + 3;
            let wide_data = data[..stride].repeat(wide_rows * WIDTH / 256);
            let mut wide_expected = vec![0.0; wide_rows];
            for (row, out) in wide_expected.iter_mut().enumerate() {
                unsafe { infinity_qwen_dot(kind, wide_data.as_ptr().add(row * WIDTH / 256 * stride),
                    wide_input.as_ptr(), WIDTH, out); }
            }
            let mut wide_actual = vec![f32::NAN; wide_rows];
            let deadline = std::time::Instant::now();
            while unsafe { rows(kind, &wide_data, &wide_input, &mut wide_actual, &mut cursor) } != Some(true) {
                assert!(deadline.elapsed().as_secs() < 5);
                std::thread::yield_now();
            }
            assert!(outputs_match(kind, &wide_expected, &wide_actual));
        }
        // Model cancellation stops rows() polling. Occupy the reserved AP as a
        // permanent browser would, then require only the ordinary service poll
        // to retire cancelled matrices and run a queued speech-shaped task.
        let data = vec![0u8; 256 * 144];
        let input = vec![0.125f32; 256];
        let mut abandoned = vec![f32::NAN; 256];
        let mut cursor = 0;
        assert_eq!(unsafe { rows(12, &data, &input, &mut abandoned, &mut cursor) }, Some(false));
        assert!(unsafe { background(held_background) });
        discard();
        QUEUED_BACKGROUND_RAN.store(false, Ordering::Release);
        assert!(unsafe { background(queued_background) });
        let deadline = std::time::Instant::now();
        while !QUEUED_BACKGROUND_RAN.load(Ordering::Acquire) {
            poll_background();
            assert!(deadline.elapsed().as_secs() < 5);
            std::thread::yield_now();
        }
        assert!(abandoned.iter().all(|value| value.is_nan()));
        assert_eq!(unsafe { PENDING.0 }, 0);
        // Repeated polls do not republish the abandoned result, and the same
        // worker pool can execute a fresh matrix after the speech callback.
        poll_background();
        let mut next = vec![f32::NAN; 1];
        while unsafe { rows(12, &data, &input, &mut next, &mut cursor) } != Some(true) {
            assert!(deadline.elapsed().as_secs() < 5);
            std::thread::yield_now();
        }
        assert_eq!(next, [0.0]);
        assert!(completed() >= 10);
        BACKGROUND_RELEASE.store(true, Ordering::Release);
        unsafe { stop_host_workers(); }
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(online(), 0);
    }
}
