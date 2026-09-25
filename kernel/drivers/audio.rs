//! Single-owner hardware adapter. Never grants authority and never starts audio at boot.
use super::hda;
use core::sync::atomic::{AtomicBool, Ordering};
static LOCK: AtomicBool = AtomicBool::new(false);
static mut DMA: hda::Dma = hda::Dma::new();
static mut DEVICE: Option<hda::Hda> = None;
static mut UNTIL: u64 = 0;
static mut LEASE: Option<crate::runtime::audio::AudioStream> = None;
static mut PCM: [i16; hda::SAMPLES] = [0; hda::SAMPLES];
use crate::runtime::audio::{AudioBuffer, CaptureState, CaptureStatus};
static mut INPUT_LEASE: Option<crate::runtime::audio::AudioStream> = None;
static mut INPUT_UNTIL: u64 = 0;
static mut INPUT_LAST_POLL: u64 = 0;
static mut INPUT: AudioBuffer<192000> = AudioBuffer::new();
static mut INPUT_PCM: [i16; hda::SAMPLES] = [0; hda::SAMPLES];
static mut INPUT_STATUS: CaptureStatus = CaptureStatus { state: CaptureState::Idle, sample_rate: 0, frames: 0, peak: 0 };

// ------------------------=
// FUNC: capture_available
// DESC: Reports a real discovered hardware input route without opening the microphone.
// ------------------=
pub fn capture_available() -> bool {
    if LOCK.swap(true, Ordering::Acquire) { return false; }
    let ready = unsafe { (&*(&raw const DEVICE)).as_ref().map(|d| d.capture_available()).unwrap_or(false) };
    LOCK.store(false, Ordering::Release); ready
}
// ------------------------=
// FUNC: capture
// DESC: Starts a three-second explicit microphone sample using a validated AudioInput lease.
// ------------------=
pub fn capture(owner: crate::runtime::execution::SecurityIdentity, capability: u64) -> bool {
    let Some(now) = crate::ui::performance::monotonic_ns() else { return false; };
    let Ok(request) = crate::runtime::iop::IopMessage::request(crate::runtime::iop::OperationId::AudioCaptureStart,
        now, owner, capability, now / 1_000_000_000 + 5, now, &[]) else { return false; };
    let lease = crate::runtime::with_runtime(|runtime| crate::runtime::audio::AudioStream::authorize(
        &request, owner, &runtime.capabilities, now / 1_000_000_000).ok()).flatten();
    if lease.is_none() || LOCK.swap(true, Ordering::Acquire) { return false; }
    let started = unsafe {
        if let Some(device) = (&mut *(&raw mut DEVICE)).as_mut() {
            if device.start_capture().is_ok() {
                let Some(started) = crate::ui::performance::monotonic_ns() else {
                    device.stop_capture(); LOCK.store(false, Ordering::Release); return false;
                };
                (&mut *(&raw mut INPUT)).clear();
                INPUT_STATUS = CaptureStatus { state: CaptureState::Recording, sample_rate: device.sample_rate, frames: 0, peak: 0 };
                INPUT_LAST_POLL = started; INPUT_UNTIL = started.saturating_add(3_000_000_000); INPUT_LEASE = lease; true
            } else {
                if !device.capturing { device.stop_capture(); }
                false
            }
        } else { false }
    };
    LOCK.store(false, Ordering::Release); started
}
// ------------------------=
// FUNC: capture_status
// DESC: Returns bounded microphone diagnostics without exposing recorded samples.
// ------------------=
pub fn capture_status() -> Option<CaptureStatus> {
    if LOCK.swap(true, Ordering::Acquire) { return None; }
    let status = unsafe { INPUT_STATUS }; LOCK.store(false, Ordering::Release); Some(status)
}
// ------------------------=
// FUNC: read_capture
// DESC: Drains live PCM only for the authorized stream owner while its capability remains valid.
// ------------------=
pub fn read_capture(owner: crate::runtime::execution::SecurityIdentity, output: &mut [i16]) -> usize {
    if LOCK.swap(true, Ordering::Acquire) { return 0; }
    let count = unsafe {
        let valid = INPUT_LEASE.filter(|l| l.owner == owner).and_then(|l| crate::ui::performance::monotonic_ns().map(|n| (l, n)))
            .and_then(|(l, n)| crate::runtime::with_runtime(|r| l.valid(&r.capabilities, n / 1_000_000_000))).unwrap_or(false);
        if valid { (&mut *(&raw mut INPUT)).read(output) } else { 0 }
    };
    LOCK.store(false, Ordering::Release); count
}
// ------------------------=
// FUNC: finish_capture
// DESC: Stops hardware and erases private audio before retiring the one-shot microphone lease; lock must be held.
// ------------------=
unsafe fn finish_capture(device: &mut hda::Hda, state: CaptureState) {
    device.stop_capture(); (&mut *(&raw mut INPUT)).clear();
    (&mut *(&raw mut INPUT_PCM)).fill(0); INPUT_STATUS.state = state;
    if let Some(lease) = (&mut *(&raw mut INPUT_LEASE)).take() {
        crate::runtime::with_runtime(|r| { let _ = r.capabilities.retire_leaf(lease.capability, lease.owner); });
    }
}
// ------------------------=
// FUNC: stop_capture
// DESC: Cancels only the caller-owned recording and clears its private sample storage.
// ------------------=
pub fn stop_capture(owner: crate::runtime::execution::SecurityIdentity) -> bool {
    if LOCK.swap(true, Ordering::Acquire) { return false; }
    let stopped = unsafe {
        if INPUT_LEASE.map(|l| l.owner == owner).unwrap_or(false) {
            if let Some(d) = (&mut *(&raw mut DEVICE)).as_mut() { finish_capture(d, CaptureState::Cancelled); }
            true
        } else { false }
    };
    LOCK.store(false, Ordering::Release); stopped
}

// ------------------------=
// FUNC: initialize
// DESC: Records boot-discovered HDA hardware without enabling microphone capture.
// ------------------=
pub fn initialize(info: &crate::boot_info::BootInfo) {
    if info.boot_reserved == 0 || LOCK.swap(true, Ordering::Acquire) { return; }
    unsafe { DEVICE = hda::Hda::initialize(info.boot_reserved as usize, &raw mut DMA).ok(); }
    LOCK.store(false, Ordering::Release);
}
// ------------------------=
// FUNC: available
// DESC: Returns actual successful controller and output-route initialization.
// ------------------=
pub fn available() -> bool {
    if LOCK.swap(true, Ordering::Acquire) { return false; }
    let ready = unsafe { (&*(&raw const DEVICE)).is_some() };
    LOCK.store(false, Ordering::Release); ready
}
// ------------------------=
// FUNC: tone
// DESC: Validates AudioOutput authority before starting a bounded two-second diagnostic tone.
// ------------------=
pub fn tone(owner: crate::runtime::execution::SecurityIdentity, capability: u64) -> bool {
    let Some(now) = crate::ui::performance::monotonic_ns() else { return false; };
    let Ok(request) = crate::runtime::iop::IopMessage::request(crate::runtime::iop::OperationId::AudioTone,
        now, owner, capability, now / 1_000_000_000 + 5, now, &[]) else { return false; };
    let lease = crate::runtime::with_runtime(|runtime| crate::runtime::audio::AudioStream::authorize(
        &request, owner, &runtime.capabilities, now / 1_000_000_000).ok()).flatten();
    if lease.is_none() || LOCK.swap(true, Ordering::Acquire) { return false; }
    let started = unsafe {
        if let Some(device) = (&mut *(&raw mut DEVICE)).as_mut() {
            let count = device.sample_rate as usize / 10 * 2;
            let pcm = &mut (&mut *(&raw mut PCM))[..count];
            let _ = hda::tone_at_rate(pcm, device.sample_rate);
            if device.start(pcm).is_ok() {
                // First-use host device initialization can consume part of the lease.
                // Count audible duration from DMA start, without extending authority.
                if let Some(started) = crate::ui::performance::monotonic_ns() {
                    UNTIL = started.saturating_add(2_000_000_000); LEASE = lease; true
                } else { device.stop(); false }
            } else { false }
        } else { false }
    };
    LOCK.store(false, Ordering::Release); started
}
// ------------------------=
// FUNC: poll
// DESC: Stops the finite tone at deadline or DMA fault without waiting on hardware or repainting.
// ------------------=
pub fn poll() {
    if LOCK.swap(true, Ordering::Acquire) { return; }
    unsafe {
        if let Some(device) = (&mut *(&raw mut DEVICE)).as_mut() {
            let now = crate::ui::performance::monotonic_ns();
            if device.capturing {
                let authorized = INPUT_LEASE.and_then(|l| now.map(|n| (l, n)))
                    .and_then(|(l, n)| crate::runtime::with_runtime(|r| l.valid(&r.capabilities, n / 1_000_000_000))).unwrap_or(false);
                let n = now.unwrap_or(0);
                if !authorized { finish_capture(device, CaptureState::Denied); }
                else if n.saturating_sub(INPUT_LAST_POLL) >= hda::FRAMES as u64 * 1_000_000_000 / device.sample_rate as u64 {
                    finish_capture(device, CaptureState::Overrun);
                } else {
                    INPUT_LAST_POLL = n;
                    match device.read_capture(&mut *(&raw mut INPUT_PCM)) {
                        Ok(count) => {
                            let samples = &(&*(&raw const INPUT_PCM))[..count];
                            INPUT_STATUS.frames += (count / 2) as u64;
                            for &s in samples { INPUT_STATUS.peak = INPUT_STATUS.peak.max(s.unsigned_abs()); }
                            if (&mut *(&raw mut INPUT)).push_stereo(samples).is_err() { finish_capture(device, CaptureState::Overrun); }
                            else if n >= INPUT_UNTIL { finish_capture(device, CaptureState::Complete); }
                        }
                        Err(_) => finish_capture(device, CaptureState::DeviceLost),
                    }
                }
            }
            if device.playing {
                let authorized = LEASE.and_then(|lease| now.map(|n| (lease, n)))
                    .and_then(|(lease, n)| crate::runtime::with_runtime(|runtime| lease.valid(&runtime.capabilities, n / 1_000_000_000))).unwrap_or(false);
                if !authorized || now.map(|n| n >= UNTIL).unwrap_or(true) || device.position().is_err() {
                    device.stop();
                    if let Some(lease) = (&mut *(&raw mut LEASE)).take() {
                        crate::runtime::with_runtime(|runtime| { let _ = runtime.capabilities.retire_leaf(lease.capability, lease.owner); });
                    }
                }
            }
        }
    }
    LOCK.store(false, Ordering::Release);
}
