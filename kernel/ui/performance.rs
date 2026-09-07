//! Bounded measured renderer telemetry. Unavailable measurements remain optional.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub const VERSION: u16 = 1;
pub const FRAME_HISTORY: usize = 3600;
pub const AGGREGATE_HISTORY: usize = 600;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum FallbackReason {
    #[default]
    None,
    InitialRender,
    DisplayChange,
    AppearanceChange,
    Recovery,
    Fragmentation,
    Debug,
    Unclassified,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FramePerformanceSample {
    pub version: u16,
    pub sequence: u64,
    pub timestamp_ns: Option<u64>,
    pub frame_ns: Option<u64>,
    pub compose_ns: Option<u64>,
    pub present_ns: Option<u64>,
    pub submitted_regions: u32,
    pub merged_regions: u32,
    pub damaged_pixels: u64,
    pub screen_pixels: u64,
    pub fallback: FallbackReason,
    pub visible_surfaces: Option<u16>,
    pub composed_surfaces: Option<u16>,
    pub reused_surfaces: Option<u16>,
    pub budget_ns: u64,
    pub budget_missed: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Aggregate {
    pub second: u64,
    pub samples: u32,
    pub timed_samples: u32,
    pub total_frame_ns: u64,
    pub worst_frame_ns: u64,
    pub missed: u32,
    pub fallbacks: u32,
    pub damaged_pixels: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub latest: FramePerformanceSample,
    pub samples: usize,
    pub timed_samples: usize,
    pub average_ns: Option<u64>,
    pub p95_ns: Option<u64>,
    pub worst_ns: Option<u64>,
    pub fallbacks: u64,
    pub overwritten_samples: u64,
    pub dropped_records: u64,
}

pub struct PerformanceHistory {
    frames: [FramePerformanceSample; FRAME_HISTORY],
    aggregates: [Aggregate; AGGREGATE_HISTORY],
    next: usize,
    len: usize,
    sequence: u64,
    fallbacks: u64,
}

impl PerformanceHistory {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates fixed frame and one-second aggregate rings without heap growth.
    // ------------------=
    pub const fn new() -> Self {
        const EMPTY: FramePerformanceSample = FramePerformanceSample {
            version: VERSION,
            sequence: 0,
            timestamp_ns: None,
            frame_ns: None,
            compose_ns: None,
            present_ns: None,
            submitted_regions: 0,
            merged_regions: 0,
            damaged_pixels: 0,
            screen_pixels: 0,
            fallback: FallbackReason::None,
            visible_surfaces: None,
            composed_surfaces: None,
            reused_surfaces: None,
            budget_ns: 16_666_667,
            budget_missed: false,
        };
        const AGGREGATE: Aggregate = Aggregate {
            second: 0,
            samples: 0,
            timed_samples: 0,
            total_frame_ns: 0,
            worst_frame_ns: 0,
            missed: 0,
            fallbacks: 0,
            damaged_pixels: 0,
        };
        Self {
            frames: [EMPTY; FRAME_HISTORY],
            aggregates: [AGGREGATE; AGGREGATE_HISTORY],
            next: 0,
            len: 0,
            sequence: 0,
            fallbacks: 0,
        }
    }

    // ------------------------=
    // FUNC: record
    // DESC: Records a completed frame and aggregates only supplied measured durations.
    // ------------------=
    pub fn record(&mut self, mut sample: FramePerformanceSample) {
        self.sequence = self.sequence.saturating_add(1);
        sample.version = VERSION;
        sample.sequence = self.sequence;
        sample.budget_missed = sample
            .frame_ns
            .map(|ns| ns > sample.budget_ns)
            .unwrap_or(false);
        self.fallbacks = self
            .fallbacks
            .saturating_add(u64::from(sample.fallback != FallbackReason::None));
        self.frames[self.next] = sample;
        self.next = (self.next + 1) % FRAME_HISTORY;
        self.len = (self.len + 1).min(FRAME_HISTORY);
        if let Some(timestamp) = sample.timestamp_ns {
            let second = timestamp / 1_000_000_000;
            let aggregate = &mut self.aggregates[second as usize % AGGREGATE_HISTORY];
            if aggregate.second != second {
                *aggregate = Aggregate {
                    second,
                    ..Aggregate::default()
                };
            }
            aggregate.samples = aggregate.samples.saturating_add(1);
            if let Some(duration) = sample.frame_ns {
                aggregate.timed_samples = aggregate.timed_samples.saturating_add(1);
                aggregate.total_frame_ns = aggregate.total_frame_ns.saturating_add(duration);
                aggregate.worst_frame_ns = aggregate.worst_frame_ns.max(duration);
            }
            aggregate.missed = aggregate
                .missed
                .saturating_add(u32::from(sample.budget_missed));
            aggregate.fallbacks = aggregate
                .fallbacks
                .saturating_add(u32::from(sample.fallback != FallbackReason::None));
            aggregate.damaged_pixels = aggregate
                .damaged_pixels
                .saturating_add(sample.damaged_pixels);
        }
    }

    // ------------------------=
    // FUNC: recent
    // DESC: Returns a frame by age, zero denoting the latest completed presentation.
    // ------------------=
    pub fn recent(&self, age: usize) -> Option<FramePerformanceSample> {
        if age >= self.len {
            return None;
        }
        let value = self.frames[(self.next + FRAME_HISTORY - 1 - age) % FRAME_HISTORY];
        let latest = self.frames[(self.next + FRAME_HISTORY - 1) % FRAME_HISTORY];
        if let (Some(now), Some(then)) = (latest.timestamp_ns, value.timestamp_ns) {
            if now.saturating_sub(then) >= 60_000_000_000 {
                return None;
            }
        }
        Some(value)
    }

    // ------------------------=
    // FUNC: aggregate
    // DESC: Returns the exact retained second or absence when that bucket has expired.
    // ------------------=
    pub fn aggregate(&self, second: u64) -> Option<Aggregate> {
        if let Some(now) = self.recent(0).and_then(|frame| frame.timestamp_ns) {
            if now / 1_000_000_000 >= second.saturating_add(AGGREGATE_HISTORY as u64) {
                return None;
            }
        }
        let value = self.aggregates[second as usize % AGGREGATE_HISTORY];
        (value.second == second && value.samples != 0).then_some(value)
    }

    // ------------------------=
    // FUNC: summary
    // DESC: Computes measured rolling statistics only when a consumer requests a snapshot.
    // ------------------=
    pub fn summary(&self) -> Summary {
        let mut durations = [0u64; FRAME_HISTORY];
        let mut count = 0;
        let mut total = 0u128;
        let mut retained = 0;
        for age in 0..self.len {
            let Some(frame) = self.recent(age) else {
                continue;
            };
            retained += 1;
            if let Some(ns) = frame.frame_ns {
                durations[count] = ns;
                count += 1;
                total += u128::from(ns);
            }
        }
        durations[..count].sort_unstable();
        Summary {
            latest: self.recent(0).unwrap_or_default(),
            samples: retained,
            timed_samples: count,
            average_ns: (count != 0).then(|| (total / count as u128) as u64),
            p95_ns: (count != 0).then(|| durations[(count * 95).div_ceil(100).saturating_sub(1)]),
            worst_ns: (count != 0).then(|| durations[count - 1]),
            fallbacks: self.fallbacks,
            overwritten_samples: self.sequence.saturating_sub(FRAME_HISTORY as u64),
            dropped_records: 0,
        }
    }
}

struct Telemetry {
    held: AtomicBool,
    dropped: AtomicU64,
    history: UnsafeCell<PerformanceHistory>,
}
// All access is guarded by the acquire/release try-lock; contention never spins.
unsafe impl Sync for Telemetry {}
static TELEMETRY: Telemetry = Telemetry {
    held: AtomicBool::new(false),
    dropped: AtomicU64::new(0),
    history: UnsafeCell::new(PerformanceHistory::new()),
};

// ------------------------=
// FUNC: publish
// DESC: Records renderer telemetry without waiting for a reader or allocating memory.
// ------------------=
pub fn publish(sample: FramePerformanceSample) {
    if TELEMETRY
        .held
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        TELEMETRY.dropped.fetch_add(1, Ordering::Relaxed);
        return;
    }
    unsafe {
        (*TELEMETRY.history.get()).record(sample);
    }
    TELEMETRY.held.store(false, Ordering::Release);
}

// ------------------------=
// FUNC: snapshot
// DESC: Returns bounded public performance metadata or busy without exposing any pixels.
// ------------------=
pub fn snapshot() -> Option<Summary> {
    if TELEMETRY
        .held
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        return None;
    }
    let mut summary = unsafe { (*TELEMETRY.history.get()).summary() };
    summary.dropped_records = TELEMETRY.dropped.load(Ordering::Relaxed);
    TELEMETRY.held.store(false, Ordering::Release);
    Some(summary)
}

// ------------------------=
// FUNC: monotonic_ns
// DESC: Reads a calibrated architectural clock; unsupported frequencies return absence.
// ------------------=
pub fn monotonic_ns() -> Option<u64> {
    #[cfg(all(target_os = "none", target_arch = "aarch64"))]
    {
        let counter: u64;
        let frequency: u64;
        unsafe {
            core::arch::asm!("isb", "mrs {}, cntvct_el0", out(reg) counter, options(nomem, nostack));
            core::arch::asm!("mrs {}, cntfrq_el0", out(reg) frequency, options(nomem, nostack));
        }
        return ticks_to_ns(counter, frequency);
    }
    #[cfg(all(target_os = "none", target_arch = "x86_64"))]
    {
        use core::arch::x86_64::{__cpuid, _mm_lfence, _rdtsc};
        // Cache the architectural calibration; CPUID serializes execution and must not run per frame.
        static FREQUENCY: AtomicU64 = AtomicU64::new(0);
        let mut frequency = FREQUENCY.load(Ordering::Relaxed);
        if frequency == 0 {
            frequency = if unsafe { __cpuid(0).eax } >= 0x15 {
                let clock = unsafe { __cpuid(0x15) };
                if clock.eax != 0 && clock.ebx != 0 && clock.ecx != 0 {
                    u64::from(clock.ecx) * u64::from(clock.ebx) / u64::from(clock.eax)
                } else {
                    1
                }
            } else {
                1
            };
            FREQUENCY.store(frequency, Ordering::Relaxed);
        }
        if frequency <= 1 {
            return None;
        }
        let counter = unsafe {
            _mm_lfence();
            _rdtsc()
        };
        return ticks_to_ns(counter, frequency);
    }
    #[cfg(not(any(
        all(target_os = "none", target_arch = "aarch64"),
        all(target_os = "none", target_arch = "x86_64")
    )))]
    {
        None
    }
}

// ------------------------=
// FUNC: ticks_to_ns
// DESC: Converts measured counter ticks without overflowing intermediate multiplication.
// ------------------=
pub fn ticks_to_ns(ticks: u64, frequency: u64) -> Option<u64> {
    if frequency == 0 {
        return None;
    }
    Some(
        ((u128::from(ticks) * 1_000_000_000) / u128::from(frequency)).min(u128::from(u64::MAX))
            as u64,
    )
}

// ------------------------=
// FUNC: elapsed
// DESC: Returns a duration only when both monotonic endpoints are available and ordered.
// ------------------=
pub fn elapsed(start: Option<u64>, end: Option<u64>) -> Option<u64> {
    end?.checked_sub(start?)
}
