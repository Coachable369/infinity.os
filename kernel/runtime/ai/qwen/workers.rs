//! Bounded AP jobs. Only the BSP submits/polls; APs touch immutable weights and
//! private activations and disjoint staging rows, never the engine, framebuffer or services.
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
static NEXT_ROW: AtomicUsize = AtomicUsize::new(0);
static CANCELLED: AtomicBool = AtomicBool::new(false);
// Exclusive row ranges are assigned by NEXT_ROW; BSP reads only after all APs
// publish completion. No worker retains a pointer into engine-owned output.
static mut MATRIX_OUTPUT: [f32; MATRIX_ROWS] = [0.0; MATRIX_ROWS];
struct Job {
    background: Option<unsafe fn()>,
    data: *const u8,
    kind: u32,
    width: usize,
    rows: usize,
    finished: u64,
    elapsed: u64,
    input: [f32; WIDTH],
    cache: [f32; CACHE_FLOATS],
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
            while slot.state.load(Ordering::Acquire) == 4 { core::hint::spin_loop(); }
            slot.state.store(3, Ordering::Release);
            wake_host(i);
        }
    }
}

// ------------------------=
// FUNC: background
// DESC: Submits one bounded non-rendering native task to an idle AP without occupying the UI thread.
// ------------------=
/// Safety: BSP-only; task must own its static buffers, honor cancellation and
/// deadlines, and never access UI, firmware, runtime locks, or matrix mailboxes.
pub unsafe fn background(task: unsafe fn()) -> bool {
    let ready = READY.load(Ordering::Acquire);
    for (index, slot) in SLOTS.iter().enumerate() {
        if ready & (1 << index) != 0 && slot.state.load(Ordering::Acquire) == 0 {
            (*slot.job.get()).background = Some(task);
            slot.state.store(4, Ordering::Release);
            #[cfg(not(target_os = "none"))]
            wake_host(index);
            #[cfg(all(target_arch = "aarch64", target_os = "none"))]
            core::arch::asm!("sev", options(nomem, nostack));
            return true;
        }
    }
    false
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
        let (mask, start, discarded) = PENDING;
        if mask != 0 {
            if (0..COUNT).any(|i| mask & (1 << i) != 0
                && SLOTS[i].state.load(Ordering::Acquire) != 2) {
                return Some(false);
            }
            for (i, slot) in SLOTS.iter().enumerate() {
                if mask & (1 << i) == 0 {
                    continue;
                }
                let job = &*slot.job.get();
                COMPUTE_TICKS.fetch_add(job.elapsed, Ordering::Relaxed);
                if job.kind == 12 {
                    Q4_TICKS.fetch_add(job.elapsed, Ordering::Relaxed);
                } else if job.kind == 14 {
                    Q6_TICKS.fetch_add(job.elapsed, Ordering::Relaxed);
                }
                IDLE_TICKS.fetch_add(counter().saturating_sub(job.finished), Ordering::Relaxed);
                slot.state.store(0, Ordering::Relaxed);
            }
            PENDING = (0, 0, false);
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
        let mut mask: usize = 0;
        for (i, slot) in SLOTS.iter().enumerate() {
            if ready & (1 << i) == 0 || slot.state.load(Ordering::Acquire) != 0 {
                continue;
            }
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
            slot.state.store(0, Ordering::Relaxed);
        }
        PENDING = (0, 0, false);
    }
}

#[cfg(test)]
mod tests {
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
        let threads: Vec<_> = (1..=6)
            .map(|id| std::thread::spawn(move || unsafe { worker_entry(id as *mut u8) }))
            .collect();
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
        assert!(completed() >= 10);
        BACKGROUND_RELEASE.store(true, Ordering::Release);
        unsafe { stop_host_workers(); }
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(online(), 0);
    }
}
