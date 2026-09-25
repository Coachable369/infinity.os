//! Single-owner hardware adapter. Never grants authority and never starts audio at boot.
use super::hda;
use core::sync::atomic::{AtomicBool, Ordering};
static LOCK: AtomicBool = AtomicBool::new(false);
static mut DMA: hda::Dma = hda::Dma::new();
static mut DEVICE: Option<hda::Hda> = None;
static mut UNTIL: u64 = 0;
static mut LEASE: Option<crate::runtime::audio::AudioStream> = None;
static mut PCM: [i16; hda::SAMPLES] = [0; hda::SAMPLES];

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
        now, owner, capability, now / 1_000_000_000 + 3, now, &[]) else { return false; };
    let lease = crate::runtime::with_runtime(|runtime| crate::runtime::audio::AudioStream::authorize(
        &request, owner, &runtime.capabilities, now / 1_000_000_000).ok()).flatten();
    if lease.is_none() || LOCK.swap(true, Ordering::Acquire) { return false; }
    let started = unsafe {
        if let Some(device) = (&mut *(&raw mut DEVICE)).as_mut() {
            let count = device.sample_rate as usize / 10 * 2;
            let pcm = &mut (&mut *(&raw mut PCM))[..count];
            let _ = hda::tone_at_rate(pcm, device.sample_rate);
            if device.start(pcm).is_ok() { UNTIL = now + 2_000_000_000; LEASE = lease; true } else { false }
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
            if device.playing {
                let authorized = LEASE.and_then(|lease| now.map(|n| (lease, n)))
                    .and_then(|(lease, n)| crate::runtime::with_runtime(|runtime| lease.valid(&runtime.capabilities, n / 1_000_000_000))).unwrap_or(false);
                if !authorized || now.map(|n| n >= UNTIL).unwrap_or(true) || device.position().is_err() {
                    device.stop();
                    if let Some(lease) = LEASE.take() {
                        crate::runtime::with_runtime(|runtime| { let _ = runtime.capabilities.retire_leaf(lease.capability, lease.owner); });
                    }
                }
            }
        }
    }
    LOCK.store(false, Ordering::Release);
}
