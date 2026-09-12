//! Bounded AP jobs. Only the BSP submits/polls; APs touch immutable weights and
//! private activation/output buffers, never the engine, framebuffer or services.
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};
const COUNT: usize = 4;
const ROWS: usize = 256;
const WIDTH: usize = 12288;
struct Job {
    data: *const u8,
    kind: u32,
    width: usize,
    rows: usize,
    input: [f32; WIDTH],
    output: [f32; ROWS],
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
                data: core::ptr::null(),
                kind: 0,
                width: 0,
                rows: 0,
                input: [0.0; WIDTH],
                output: [0.0; ROWS],
            }),
        }
    }
}
static SLOTS: [Slot; COUNT] = [const { Slot::new() }; COUNT];
static READY: AtomicUsize = AtomicUsize::new(0);
static COMPLETED: AtomicUsize = AtomicUsize::new(0);
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
// FUNC: initialize
// DESC: Requests asynchronous AP startup through the versioned firmware bridge on ARM.
// ------------------=
pub unsafe fn initialize(address: u64) {
    #[cfg(all(target_arch = "aarch64", target_os = "none"))]
    if address != 0 {
        #[repr(C)]
        struct Bridge {
            version: u64,
            start: unsafe extern "efiapi" fn(unsafe extern "efiapi" fn(*mut u8), u64) -> u64,
        }
        let bridge = &*(address as *const Bridge);
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
    READY.fetch_or(1 << index, Ordering::Release);
    let slot = &SLOTS[index];
    loop {
        match slot.state.load(Ordering::Acquire) {
            1 => {
                let job = &mut *slot.job.get();
                let stride = job.width / 256 * if job.kind == 12 { 144 } else { 210 };
                extern "C" {
                    fn infinity_qwen_dot(
                        kind: u32,
                        data: *const u8,
                        input: *const f32,
                        width: usize,
                        output: *mut f32,
                    );
                }
                for row in 0..job.rows {
                    infinity_qwen_dot(
                        job.kind,
                        job.data.add(row * stride),
                        job.input.as_ptr(),
                        job.width,
                        &mut job.output[row],
                    );
                }
                COMPLETED.fetch_add(1, Ordering::Relaxed);
                slot.state.store(2, Ordering::Release);
                #[cfg(all(target_arch = "aarch64", target_os = "none"))]
                core::arch::asm!("sev", options(nomem, nostack));
            }
            3 => {
                READY.fetch_and(!(1 << index), Ordering::Release);
                return;
            }
            _ => {
                #[cfg(all(target_arch = "aarch64", target_os = "none"))]
                core::arch::asm!("wfe", options(nomem, nostack));
                #[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
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
    unsafe {
        PENDING.2 = true;
    }
}

// ------------------------=
// FUNC: rows
// DESC: Enqueues or collects bounded row batches; None selects the single-core fallback.
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
            if (0..COUNT)
                .any(|i| mask & (1 << i) != 0 && SLOTS[i].state.load(Ordering::Acquire) != 2)
            {
                return Some(false);
            }
            let mut at = start;
            for (i, slot) in SLOTS.iter().enumerate() {
                if mask & (1 << i) == 0 {
                    continue;
                }
                let job = &*slot.job.get();
                if !discarded {
                    output[at..at + job.rows].copy_from_slice(&job.output[..job.rows]);
                }
                at += job.rows;
                slot.state.store(0, Ordering::Relaxed);
            }
            PENDING = (0, 0, false);
            if !discarded {
                *cursor = if at == output.len() { 0 } else { at };
                return Some(at == output.len());
            }
        }
        let ready = READY.load(Ordering::Acquire);
        if ready == 0
            || !matches!(kind, 12 | 14)
            || input.is_empty()
            || input.len() > WIDTH
            || input.len() % 256 != 0
            || *cursor >= output.len()
        {
            return None;
        }
        let stride = input.len() / 256 * if kind == 12 { 144 } else { 210 };
        if data.len() < output.len().checked_mul(stride)? {
            return None;
        }
        let mut at = *cursor;
        let mut mask = 0;
        for (i, slot) in SLOTS.iter().enumerate() {
            if ready & (1 << i) == 0 || at == output.len() {
                continue;
            }
            let job = &mut *slot.job.get();
            job.rows = ROWS.min(output.len() - at);
            job.width = input.len();
            job.kind = kind;
            job.data = data.as_ptr().add(at * stride);
            job.input[..input.len()].copy_from_slice(input);
            at += job.rows;
            mask |= 1 << i;
            slot.state.store(1, Ordering::Release);
        }
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
    // FUNC: concurrent_rows_and_cancellation
    // DESC: Compares real concurrent kernels with direct execution and retires cancelled jobs before reusing buffers.
    // ------------------=
    #[test]
    fn concurrent_rows_and_cancellation() {
        let threads: Vec<_> = (1..=2)
            .map(|id| std::thread::spawn(move || unsafe { worker_entry(id as *mut u8) }))
            .collect();
        let deadline = std::time::Instant::now();
        while online() != 2 {
            assert!(deadline.elapsed().as_secs() < 5);
            std::thread::yield_now();
        }
        for kind in [12, 14] {
            let stride = if kind == 12 { 144 } else { 210 };
            let mut data = vec![0u8; 513 * stride];
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
            let mut expected = vec![0.0; 513];
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
            let mut actual = vec![0.0; 513];
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
            assert_eq!(tiny[0].to_bits(), expected[0].to_bits());
            let deadline = std::time::Instant::now();
            while unsafe { rows(kind, &data, &input, &mut actual, &mut cursor) } != Some(true) {
                assert!(deadline.elapsed().as_secs() < 5);
                std::thread::yield_now();
            }
            assert_eq!(
                actual.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                expected.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
            );
            drain();
        }
        assert!(completed() >= 10);
        for slot in &SLOTS[..2] {
            slot.state.store(3, Ordering::Release);
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(online(), 0);
    }
}
