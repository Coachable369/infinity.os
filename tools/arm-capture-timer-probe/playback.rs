//! Native production-HDA multi-utterance playback diagnosis; synthetic PCM only.
#![no_std]
#![allow(dead_code)]
use core::sync::atomic::{AtomicUsize, Ordering};
#[path = "../../kernel/core/boot_info.rs"]
mod boot_info;
#[path = "../../kernel/drivers/hda.rs"]
mod hda;
#[cfg(feature = "short-periods")]
mod short_periods;

const MAGIC: u64 = 0x494e46504c415942;
const LENGTHS: [usize; 4] = [79_906, 98_615, 119_473, 116_285];
const PHASES: usize = if cfg!(feature = "production-long") { 9 } else { 8 };
static READY: AtomicUsize = AtomicUsize::new(0);
static mut CPUS: [u64; 7] = [u64::MAX; 7];
static mut DMA: hda::Dma = hda::Dma::new();
static mut CAPTURE: [i16; 8192] = [0; 8192];
#[repr(C, align(128))]
struct Samples([i16; 48_000 * 32 * 2]);
static mut PCM: Samples = Samples([0; 48_000 * 32 * 2]);

// ------------------------=
// FUNC: ticks
// DESC: Reads the native counter independently of host audio scheduling.
// ------------------=
fn ticks() -> u64 { let value; unsafe { core::arch::asm!("mrs {}, cntvct_el0", out(reg) value); } value }

// ------------------------=
// FUNC: native_clock
// DESC: Supplies real monotonic nanoseconds to the production refill deadline guard without overflowing counter scaling.
// ------------------=
fn native_clock() -> Option<u64> {
    let frequency: u64;
    unsafe { core::arch::asm!("mrs {}, cntfrq_el0", out(reg) frequency); }
    if frequency == 0 { return None; }
    let counter = ticks();
    Some(counter / frequency * 1_000_000_000 + counter % frequency * 1_000_000_000 / frequency)
}

// ------------------------=
// FUNC: cpu
// DESC: Identifies the physical affinity claimed by the production worker bridge.
// ------------------=
fn cpu() -> u64 { let value: u64; unsafe { core::arch::asm!("mrs {}, mpidr_el1", out(reg) value); } value & 255 }

// ------------------------=
// FUNC: worker
// DESC: Parks native workers using the production mailbox idle instruction without taking timer ownership.
// ------------------=
unsafe extern "efiapi" fn worker(argument: *mut u8) {
    let slot = argument as usize;
    assert!((1..7).contains(&slot));
    CPUS[slot] = cpu();
    READY.fetch_or(1 << slot, Ordering::Release);
    loop { core::arch::asm!("wfe", options(nomem, nostack)); }
}

// ------------------------=
// FUNC: words
// DESC: Emits framed numeric evidence without recording microphone samples.
// ------------------=
unsafe fn words(values: &[u64]) {
    const HEX: &[u8] = b"0123456789abcdef";
    let frequency: u64;
    core::arch::asm!("mrs {}, cntfrq_el0", out(reg) frequency);
    let mut next = ticks();
    for value in values {
        for value in value.to_le_bytes() {
            for character in [HEX[(value >> 4) as usize], HEX[(value & 15) as usize]] {
                // The virtual PL011 FIFO-ready bit does not guarantee that its
                // asynchronous file backend retained an unlimited burst. Match
                // the configured115200baud wire rate for diagnostic bytes only.
                while ticks() < next { core::hint::spin_loop(); }
                while core::ptr::read_volatile(0xffdd_f018usize as *const u32) & 32 != 0 {}
                core::ptr::write_volatile(0xffdd_f000usize as *mut u32, character as u32);
                next = ticks() + frequency / 11520 + 1;
            }
        }
        while ticks() < next { core::hint::spin_loop(); }
        while core::ptr::read_volatile(0xffdd_f018usize as *const u32) & 32 != 0 {}
        core::ptr::write_volatile(0xffdd_f000usize as *mut u32, 10);
        next = ticks() + frequency / 11520 + 1;
    }
    while core::ptr::read_volatile(0xffdd_f018usize as *const u32) & 8 != 0 {}
}

// ------------------------=
// FUNC: finish
// DESC: Leaves the disposable guest available for a final host counter snapshot after bounded diagnosis.
// ------------------=
fn finish(status: u64) -> ! {
    unsafe { words(&[MAGIC, status]); }
    loop { core::hint::spin_loop(); }
}

// ------------------------=
// FUNC: panic
// DESC: Turns a native invariant failure into a structured nonzero result.
// ------------------=
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { finish(99) }

// ------------------------=
// FUNC: sample
// DESC: Generates exact low-level synthetic PCM with a unique period and final marker for each utterance.
// ------------------=
fn sample(index: usize, frame: usize, frames: usize, rate: usize) -> i16 {
    let period = if frame + rate / 5 >= frames { 32 + index * 2 } else { 96 + index * 8 };
    let phase = frame % period;
    let value = if phase < period / 2 { -4000 + phase as i32 * 16000 / period as i32 }
        else { 12000 - phase as i32 * 16000 / period as i32 };
    #[cfg(feature = "production-long")]
    {
        // A nonperiodic quiet watermark makes stale ring replay fail exact PCM
        // comparison even when tone and DMA-ring periods happen to align.
        let mut x = frame as u32 ^ (index as u32 + 1).wrapping_mul(0x9e37_79b9);
        x = (x ^ (x >> 16)).wrapping_mul(0x7feb_352d);
        x = (x ^ (x >> 15)).wrapping_mul(0x846c_a68b);
        return (value + ((x ^ (x >> 16)) & 15) as i32 - 8) as i16;
    }
    #[cfg(not(feature = "production-long"))]
    { value as i16 }
}

// ------------------------=
// FUNC: capture_poll
// DESC: Services the active ADC using discarded host-disabled input, never retaining private microphone audio.
// ------------------=
unsafe fn capture_poll(device: &mut hda::Hda) -> usize {
    device.read_capture(&mut *(&raw mut CAPTURE)).unwrap() / 2
}

// ------------------------=
// FUNC: infinity_kernel_entry
// DESC: Plays four variable-length utterances twice, with bounded natural-gap and immediate-next-start cases.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_kernel_entry(info: &boot_info::BootInfo) -> ! {
    assert_eq!(info.magic, boot_info::BOOT_MAGIC);
    assert!(info.worker_bridge != 0 && info.boot_reserved != 0);
    let frequency: u64;
    core::arch::asm!("mrs {}, cntfrq_el0", out(reg) frequency);
    CPUS[0] = cpu();
    #[repr(C)]
    struct Bridge { version: u64, start: unsafe extern "efiapi" fn(unsafe extern "efiapi" fn(*mut u8), u64) -> u64 }
    let bridge = &*(info.worker_bridge as *const Bridge);
    assert_eq!(bridge.version, 1);
    assert_eq!((bridge.start)(worker, 7), 6);
    let started = ticks();
    while READY.load(Ordering::Acquire) != 126 {
        if ticks() - started > frequency * 2 { finish(2); }
    }
    assert_eq!(*(&raw const CPUS), [0, 1, 2, 3, 4, 5, 6]);
    let mut device = hda::Hda::initialize(info.boot_reserved as usize, &raw mut DMA).unwrap();
    device.start_capture().unwrap();
    #[cfg(feature = "short-periods")]
    {
        // Configure the codec through production verbs using only zero samples;
        // all measured speech then uses the diagnostic short-period BDL.
        device.start_resident(core::slice::from_raw_parts(core::ptr::addr_of!(PCM.0).cast::<i16>(), 256), native_clock).unwrap();
        device.stop();
    }
    let header = [MAGIC, if cfg!(feature = "production-long") { 3 } else if cfg!(feature = "short-periods") { 2 } else { 1 },
        device.sample_rate as u64, frequency, 6];
    words(&header);
    words(&*(&raw const CPUS));
    let mut result = [[0u64; 9]; PHASES];
    for index in 0..PHASES {
        let frames = if index == 8 { device.sample_rate as usize * 32 } else { LENGTHS[index % 4] };
        for frame in 0..frames {
            let value = sample(index, frame, frames, device.sample_rate as usize);
            PCM.0[frame * 2] = value;
            PCM.0[frame * 2 + 1] = value;
        }
        let samples = core::slice::from_raw_parts(core::ptr::addr_of!(PCM.0).cast::<i16>(), frames * 2);
        #[cfg(not(feature = "short-periods"))]
        device.start_resident(samples, native_clock).unwrap();
        #[cfg(feature = "short-periods")]
        let mut tracker = short_periods::start(info.boot_reserved as usize, samples, device.sample_rate);
        let begin = ticks();
        let mut capture_frames = 0u64;
        let mut last_progress = begin;
        let mut max_gap = 0;
        let mut previous = 0;
        let mut polls = 0;
        let mut next_poll = begin;
        loop {
            let now = ticks();
            if now < next_poll { core::hint::spin_loop(); continue; }
            next_poll = now + frequency / 1000;
            capture_frames += capture_poll(&mut device) as u64;
            #[cfg(not(feature = "short-periods"))]
            let progress = device.resident_progress().unwrap();
            #[cfg(feature = "short-periods")]
            let progress = short_periods::progress(info.boot_reserved as usize, &mut tracker);
            polls += 1;
            if progress.played_bytes != previous {
                max_gap = max_gap.max(now - last_progress);
                last_progress = now;
                previous = progress.played_bytes;
            }
            if progress.complete {
                result[index] = [index as u64, frames as u64, begin, now, previous as u64,
                    max_gap, polls, capture_frames, device.position().unwrap() as u64];
                device.stop();
                break;
            }
            if now - begin > frequency * (frames as u64 / device.sample_rate as u64 + 5) { finish(10 + index as u64); }
        }
        // Test spacing, not a production drain workaround: compare naturally
        // separated turns against immediate RUN/reset transitions. The last
        // interval leaves the host asynchronous sink time to write its evidence.
        if index < 4 || index == 7 || index + 1 == PHASES {
            let gap = ticks();
            let mut next_poll = gap;
            while ticks() - gap < frequency * 2 {
                let now = ticks();
                if now >= next_poll { capture_poll(&mut device); next_poll = now + frequency / 1000; }
            }
        }
    }
    device.stop_capture();
    // Repeat the complete packet after audio has stopped; UART telemetry is
    // auxiliary to exact raw-DMA and post-ring WAV evidence, not the PCM oracle.
    words(&header);
    words(&*(&raw const CPUS));
    for row in result { words(&row); }
    finish(0)
}

// ------------------------=
// FUNC: memset
// DESC: Supplies freestanding compiler memory initialization.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn memset(p: *mut core::ffi::c_void, value: i32, count: usize) -> *mut core::ffi::c_void {
    for i in 0..count { core::ptr::write_volatile(p.cast::<u8>().add(i), value as u8); } p
}

// ------------------------=
// FUNC: memcpy
// DESC: Supplies freestanding compiler memory copying.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn memcpy(p: *mut core::ffi::c_void, source: *const core::ffi::c_void, count: usize) -> *mut core::ffi::c_void {
    for i in 0..count { core::ptr::write_volatile(p.cast::<u8>().add(i), core::ptr::read_volatile(source.cast::<u8>().add(i))); } p
}
