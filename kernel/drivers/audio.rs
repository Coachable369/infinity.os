//! Single-owner hardware adapter. Never grants authority and never starts audio at boot.
use super::hda;
#[path = "speech_pcm.rs"] mod speech_pcm;
use core::sync::atomic::{AtomicBool, Ordering};
static LOCK: AtomicBool = AtomicBool::new(false);
static mut DMA: hda::Dma = hda::Dma::new();
static mut DEVICE: Option<hda::Hda> = None;
static mut UNTIL: u64 = 0;
static mut RESIDENT_BYTES: usize = 0;
static mut LEASE: Option<crate::runtime::audio::AudioStream> = None;
static mut PCM: [i16; hda::SAMPLES] = [0; hda::SAMPLES];
use crate::runtime::audio::{AudioBuffer, CaptureState, CaptureStatus, InfinityAudio, InfinityAudioStatus};
use crate::runtime::audio::PlaybackState;
const INFINITY_AUDIO_SAMPLES: usize = 48_000 * 2 * 32;
static mut INFINITY_AUDIO: InfinityAudio<INFINITY_AUDIO_SAMPLES, 128> = InfinityAudio::new();
static mut INFINITY_OWNER: crate::runtime::execution::SecurityIdentity = crate::runtime::execution::SecurityIdentity([0; 16]);
static mut INFINITY_GENERATION: u64 = 0;
static mut INFINITY_ACTIVE: bool = false;
static mut INFINITY_STARTED: bool = false;
static mut INFINITY_PLAYED_FRAMES: u64 = 0;
const SYSTEM_CUE_MAX_SAMPLES: usize = 48_000 * 2 * 12;
#[repr(align(128))]
struct ResidentSystemCue([i16; SYSTEM_CUE_MAX_SAMPLES]);
static mut SYSTEM_CUE_PCM: ResidentSystemCue = ResidentSystemCue([0; SYSTEM_CUE_MAX_SAMPLES]);
static mut SYSTEM_CUE_LENGTH: usize = 0;
static mut PLAY_STATE: PlaybackState = PlaybackState::Idle;
static mut INPUT_LEASE: Option<crate::runtime::audio::AudioStream> = None;
static mut INPUT_UNTIL: u64 = 0;
static mut INPUT_LAST_POLL: u64 = 0;
static mut INPUT: AudioBuffer<192000> = AudioBuffer::new();
static mut INPUT_PCM: [i16; hda::CAPTURE_SAMPLES] = [0; hda::CAPTURE_SAMPLES];
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
// FUNC: renew_capture
// DESC: Extends a caller-owned live stream through another validated short IOP lease without restarting DMA.
// ------------------=
pub fn renew_capture(owner: crate::runtime::execution::SecurityIdentity, capability: u64) -> bool {
    let Some(now) = crate::ui::performance::monotonic_ns() else { return false; };
    if LOCK.swap(true, Ordering::Acquire) { return false; }
    let renewed = unsafe {
        if let Some(lease) = INPUT_LEASE.filter(|lease| lease.owner == owner) {
            let request = crate::runtime::iop::IopMessage::request(crate::runtime::iop::OperationId::AudioCaptureStart,
                now, owner, capability, now / 1_000_000_000 + 5, now, &[]);
            let next = request.ok().and_then(|request| crate::runtime::with_runtime(|r|
                lease.renew(&request, &r.capabilities, now / 1_000_000_000).ok()).flatten());
            if INPUT_STATUS.state == CaptureState::Recording && next.is_some() {
                INPUT_LEASE = next; INPUT_UNTIL = now.saturating_add(3_000_000_000);
                if capability != lease.capability { crate::runtime::with_runtime(|r| { let _ = r.capabilities.retire_leaf(lease.capability, owner); }); }
                true
            } else { false }
        } else { false }
    };
    LOCK.store(false, Ordering::Release); renewed
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
// FUNC: playback_rate
// DESC: Returns the negotiated hardware rate before an AP prepares bounded speech PCM.
// ------------------=
pub fn playback_rate() -> Option<u32> {
    if LOCK.swap(true, Ordering::Acquire) { return None; }
    let rate = unsafe { (&*(&raw const DEVICE)).as_ref().map(|d| d.sample_rate) };
    LOCK.store(false, Ordering::Release); rate
}
// ------------------------=
// FUNC: playback_frames
// DESC: Returns the owning resident stream's real DMA cursor instead of estimating progress from wall time.
// ------------------=
pub fn playback_frames(owner: crate::runtime::execution::SecurityIdentity) -> Option<usize> {
    if LOCK.swap(true, Ordering::Acquire) { return None; }
    let frames = unsafe {
        if RESIDENT_BYTES > 0 && LEASE.is_some_and(|lease| lease.owner == owner) {
            (&*(&raw const DEVICE)).as_ref().filter(|device| device.playing)
                .and_then(|device| device.position().ok()).map(|bytes| bytes as usize / 4)
        } else { None }
    };
    LOCK.store(false, Ordering::Release); frames
}
// ------------------------=
// FUNC: infinity_audio_open
// DESC: Opens a reusable generation-bound PCM queue under typed output authority without starting hardware.
// ------------------=
pub fn infinity_audio_open(owner: crate::runtime::execution::SecurityIdentity, capability: u64, generation: u64) -> bool {
    if generation == 0 { return false; }
    let Some(now) = crate::ui::performance::monotonic_ns() else { return false; };
    let Ok(request) = crate::runtime::iop::IopMessage::request(crate::runtime::iop::OperationId::AudioPlaybackStart,
        now, owner, capability, now / 1_000_000_000 + 35, now, &[]) else { return false; };
    let lease = crate::runtime::with_runtime(|runtime| crate::runtime::audio::AudioStream::authorize(
        &request, owner, &runtime.capabilities, now / 1_000_000_000).ok()).flatten();
    if lease.is_none() || LOCK.swap(true, Ordering::Acquire) { return false; }
    let opened = unsafe {
        let idle = (&*(&raw const DEVICE)).as_ref().is_some_and(|device| !device.playing);
        if idle {
            (&mut *(&raw mut INFINITY_AUDIO)).reset(generation);
            INFINITY_OWNER = owner; INFINITY_GENERATION = generation; INFINITY_ACTIVE = true; INFINITY_STARTED = false;
            INFINITY_PLAYED_FRAMES = 0;
            LEASE = lease; PLAY_STATE = PlaybackState::Idle; true
        } else { false }
    };
    LOCK.store(false, Ordering::Release); opened
}
// ------------------------=
// FUNC: infinity_audio_append
// DESC: Renews stream authority and appends processed stereo PCM with its content range to the active queue.
// ------------------=
pub fn infinity_audio_append(owner: crate::runtime::execution::SecurityIdentity, capability: u64, generation: u64,
    samples: &[i16], content_start: usize, content_end: usize) -> bool {
    let Some(now) = crate::ui::performance::monotonic_ns() else { return false; };
    if LOCK.swap(true, Ordering::Acquire) { return false; }
    let appended = unsafe {
        if !INFINITY_ACTIVE || INFINITY_OWNER != owner || INFINITY_GENERATION != generation { false }
        else if let Some(current) = LEASE {
            let request = crate::runtime::iop::IopMessage::request(crate::runtime::iop::OperationId::AudioPlaybackStart,
                now, owner, capability, now / 1_000_000_000 + 35, now, &[]);
            let next = request.ok().and_then(|request| crate::runtime::with_runtime(|runtime|
                current.renew(&request, &runtime.capabilities, now / 1_000_000_000).ok()).flatten());
            if let Some(next) = next {
                if (&mut *(&raw mut INFINITY_AUDIO)).append(generation, samples, content_start, content_end).is_ok() {
                    LEASE = Some(next);
                    if capability != current.capability { crate::runtime::with_runtime(|runtime| { let _ = runtime.capabilities.retire_leaf(current.capability, owner); }); }
                    true
                } else { false }
            } else { false }
        } else { false }
    };
    LOCK.store(false, Ordering::Release); appended
}
// ------------------------=
// FUNC: infinity_audio_can_append
// DESC: Checks whether an active generation can accept a complete processed span without partially publishing it.
// ------------------=
pub fn infinity_audio_can_append(owner: crate::runtime::execution::SecurityIdentity, generation: u64,
    sample_count: usize, content_start: usize, content_end: usize) -> Option<bool> {
    if LOCK.swap(true, Ordering::Acquire) { return None; }
    let accepted = unsafe {
        INFINITY_ACTIVE && INFINITY_OWNER == owner && INFINITY_GENERATION == generation
            && (&*(&raw const INFINITY_AUDIO)).can_append(generation, sample_count, content_start, content_end)
    };
    LOCK.store(false, Ordering::Release); Some(accepted)
}
// ------------------------=
// FUNC: infinity_audio_seal
// DESC: Seals a prepared stream and starts one continuous resident HDA DMA session without scheduler-timed refills.
// ------------------=
pub fn infinity_audio_seal(owner: crate::runtime::execution::SecurityIdentity, generation: u64) -> bool {
    if LOCK.swap(true, Ordering::Acquire) { return false; }
    let started = unsafe {
        if !INFINITY_ACTIVE || INFINITY_STARTED || INFINITY_OWNER != owner || INFINITY_GENERATION != generation { false }
        else if (&mut *(&raw mut INFINITY_AUDIO)).seal(generation).is_err() { false }
        else if let Some(device) = (&mut *(&raw mut DEVICE)).as_mut().filter(|device| !device.playing) {
            let resident = (&*(&raw const INFINITY_AUDIO)).sealed_resident(generation);
            if let Some(samples) = resident {
                if device.start_resident(samples).is_ok() {
                    RESIDENT_BYTES = samples.len() * 2; INFINITY_STARTED = true; INFINITY_PLAYED_FRAMES = 0;
                    UNTIL = crate::ui::performance::monotonic_ns().unwrap_or(0).saturating_add(35_000_000_000);
                    PLAY_STATE = PlaybackState::Playing; true
                } else { false }
            } else { false }
        } else { false }
    };
    LOCK.store(false, Ordering::Release); started
}
// ------------------------=
// FUNC: infinity_audio_progress
// DESC: Maps the real HDA cursor onto the active queue timeline for reusable media and synchronized UI clients.
// ------------------=
pub fn infinity_audio_progress(owner: crate::runtime::execution::SecurityIdentity, generation: u64) -> Option<InfinityAudioStatus> {
    if LOCK.swap(true, Ordering::Acquire) { return None; }
    let progress = unsafe {
        if !INFINITY_ACTIVE || INFINITY_OWNER != owner || INFINITY_GENERATION != generation { None }
        else {
            let played = if INFINITY_STARTED { (&*(&raw const DEVICE)).as_ref().and_then(|device| device.position().ok())
                .map(|bytes| (bytes as u64) / 4).unwrap_or(INFINITY_PLAYED_FRAMES) }
                else { INFINITY_PLAYED_FRAMES };
            Some((&*(&raw const INFINITY_AUDIO)).status(played))
        }
    };
    LOCK.store(false, Ordering::Release); progress
}
// ------------------------=
// FUNC: infinity_audio_echo_reference
// DESC: Copies only already-played queued PCM into a bounded mono reference for full-duplex echo reduction.
// ------------------=
pub fn infinity_audio_echo_reference(owner: crate::runtime::execution::SecurityIdentity, generation: u64, output: &mut [i16]) -> bool {
    if LOCK.swap(true, Ordering::Acquire) { return false; }
    let copied = unsafe {
        if !INFINITY_ACTIVE || INFINITY_OWNER != owner || INFINITY_GENERATION != generation { false }
        else if let Some(device) = (&*(&raw const DEVICE)).as_ref() {
            let played = device.position().ok().map(|bytes| (bytes as u64) / 4).unwrap_or(INFINITY_PLAYED_FRAMES);
            (&*(&raw const INFINITY_AUDIO)).reference_mono(played, device.sample_rate, 16_000, output)
        } else { false }
    };
    LOCK.store(false, Ordering::Release); copied
}
// ------------------------=
// FUNC: play_resident_speech
// DESC: Starts prefilled kernel-owned speech under typed output authority; no per-frame copies or conversions.
// ------------------=
/// Caller owns immutable resident PCM until playback stops; trailing PCM must be silence.
pub unsafe fn play_resident_speech(owner: crate::runtime::execution::SecurityIdentity, capability: u64,
    samples: &'static [i16], spoken_samples: usize, rate: u32) -> bool {
    use crate::runtime::iop::{IopMessage, OperationId};
    if spoken_samples == 0 || spoken_samples > samples.len() || spoken_samples % 2 != 0
        || samples.len() != rate as usize * 2 * 32 { return false; }
    let Some(now) = crate::ui::performance::monotonic_ns() else { return false; };
    let Ok(request) = IopMessage::request(OperationId::AudioPlaybackStart, now, owner, capability,
        now / 1_000_000_000 + 35, now, &[]) else { return false; };
    let lease = crate::runtime::with_runtime(|r| crate::runtime::audio::AudioStream::authorize(
        &request, owner, &r.capabilities, now / 1_000_000_000).ok()).flatten();
    if lease.is_none() || LOCK.swap(true, Ordering::Acquire) { return false; }
    let started = if let Some(d) = (&mut *(&raw mut DEVICE)).as_mut().filter(|d| !d.playing && d.sample_rate == rate) {
        if d.start_resident(samples).is_ok() {
            RESIDENT_BYTES = spoken_samples * 2; INFINITY_ACTIVE = false; INFINITY_STARTED = false;
            UNTIL = crate::ui::performance::monotonic_ns().unwrap_or(now).saturating_add(31_000_000_000);
            LEASE = lease; PLAY_STATE = PlaybackState::Playing; true
        } else { false }
    } else { false };
    LOCK.store(false, Ordering::Release); started
}
// ------------------------=
// FUNC: play_resident_system_cue
// DESC: Converts a bounded 16-kHz mono system cue once and gives its complete stereo waveform to HDA so boot playback cannot underrun before the service loop starts.
// ------------------=
pub fn play_resident_system_cue(
    owner: crate::runtime::execution::SecurityIdentity,
    capability: u64,
    samples: &'static [i16],
) -> bool {
    if samples.is_empty() || samples.len() > 16_000 * 12 {
        return false;
    }
    let Some(now) = crate::ui::performance::monotonic_ns() else { return false; };
    let Ok(request) = crate::runtime::iop::IopMessage::request(
        crate::runtime::iop::OperationId::AudioPlaybackStart,
        now,
        owner,
        capability,
        now / 1_000_000_000 + 15,
        now,
        &[],
    ) else { return false; };
    let lease = crate::runtime::with_runtime(|runtime| {
        crate::runtime::audio::AudioStream::authorize(
            &request,
            owner,
            &runtime.capabilities,
            now / 1_000_000_000,
        )
        .ok()
    })
    .flatten();
    if lease.is_none() || LOCK.swap(true, Ordering::Acquire) {
        return false;
    }
    let started = unsafe {
        if let Some(device) = (&mut *(&raw mut DEVICE)).as_mut().filter(|device| !device.playing) {
            let frames = samples.len().saturating_mul(device.sample_rate as usize) / 16_000;
            let output_samples = frames.saturating_mul(2);
            if output_samples == 0 || output_samples > SYSTEM_CUE_MAX_SAMPLES {
                false
            } else {
                let output = &mut (&mut *(&raw mut SYSTEM_CUE_PCM.0))[..output_samples];
                output.fill(0);
                let converted = speech_pcm::fill(samples, output, 0, device.sample_rate);
                // SAFETY: the single audio lock prevents mutation of this aligned static
                // buffer until finish_playback stops DMA and clears the resident cue.
                let resident: &'static [i16] = &(&*(&raw const SYSTEM_CUE_PCM.0))[..output_samples];
                if converted && device.start_resident(resident).is_ok() {
                    let duration_ns = frames as u64 * 1_000_000_000 / device.sample_rate as u64;
                    SYSTEM_CUE_LENGTH = output_samples;
                    RESIDENT_BYTES = output_samples * 2;
                    INFINITY_ACTIVE = false;
                    INFINITY_STARTED = false;
                    UNTIL = crate::ui::performance::monotonic_ns()
                        .unwrap_or(now)
                        .saturating_add(duration_ns);
                    LEASE = lease;
                    PLAY_STATE = PlaybackState::Playing;
                    true
                } else {
                    false
                }
            }
        } else {
            false
        }
    };
    LOCK.store(false, Ordering::Release);
    started
}
// ------------------------=
// FUNC: playback_state
// DESC: Exposes the authoritative native output state without exposing speech samples.
// ------------------=
pub fn playback_state() -> Option<PlaybackState> {
    if LOCK.swap(true, Ordering::Acquire) { return None; }
    let state = unsafe { PLAY_STATE }; LOCK.store(false, Ordering::Release); Some(state)
}
// ------------------------=
// FUNC: play_speech
// DESC: Copies bounded 16-kHz mono speech into an authorized native output stream; no application supplies DMA pointers.
// ------------------=
pub fn play_speech(owner: crate::runtime::execution::SecurityIdentity, capability: u64, samples: &[i16]) -> bool {
    if samples.is_empty() || samples.len() > 480000 { return false; }
    let generation = crate::ui::performance::monotonic_ns().unwrap_or(1).max(1);
    if !infinity_audio_open(owner, capability, generation) { return false; }
    let Some(rate) = playback_rate() else { stop_playback(owner); return false; };
    let mut source = 0;
    while source < samples.len() {
        let count = (samples.len() - source).min(1600);
        let frames = (count * rate as usize + 15_999) / 16_000;
        let output_samples = frames * 2;
        let converted = unsafe {
            let output = &mut (&mut *(&raw mut PCM))[..output_samples];
            output.fill(0);
            speech_pcm::fill_rate(&samples[source..source + count], output, 0, 16_000, rate)
                && infinity_audio_append(owner, capability, generation, output, 0, 0)
        };
        if !converted { stop_playback(owner); return false; }
        source += count;
    }
    infinity_audio_seal(owner, generation)
}
// ------------------------=
// FUNC: finish_playback
// DESC: Stops DMA and erases private speech PCM before retiring output authority; audio lock must be held.
// ------------------=
unsafe fn finish_playback(device: &mut hda::Hda, state: PlaybackState) {
    device.stop(); PLAY_STATE = state; RESIDENT_BYTES = 0; INFINITY_STARTED = false;
    if INFINITY_ACTIVE { (&mut *(&raw mut INFINITY_AUDIO)).erase_pcm(); }
    (&mut *(&raw mut PCM)).fill(0);
    (&mut *(&raw mut SYSTEM_CUE_PCM.0))[..SYSTEM_CUE_LENGTH].fill(0);
    SYSTEM_CUE_LENGTH = 0;
    if let Some(lease) = (&mut *(&raw mut LEASE)).take() {
        crate::runtime::with_runtime(|r| { let _ = r.capabilities.retire_leaf(lease.capability, lease.owner); });
    }
}
// ------------------------=
// FUNC: stop_playback
// DESC: Cancels only caller-owned playback for stop or barge-in without affecting another session.
// ------------------=
pub fn stop_playback(owner: crate::runtime::execution::SecurityIdentity) -> bool {
    if LOCK.swap(true, Ordering::Acquire) { return false; }
    let stopped = unsafe {
        if LEASE.map(|l| l.owner == owner).unwrap_or(false) {
            if let Some(d) = (&mut *(&raw mut DEVICE)).as_mut() { finish_playback(d, PlaybackState::Cancelled); }
            true
        } else { false }
    };
    LOCK.store(false, Ordering::Release); stopped
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
                    UNTIL = started.saturating_add(2_000_000_000); LEASE = lease;
                    INFINITY_ACTIVE = false; INFINITY_STARTED = false; PLAY_STATE = PlaybackState::Playing; true
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
                else if n.saturating_sub(INPUT_LAST_POLL) >= hda::CAPTURE_FRAMES as u64 * 1_000_000_000 / device.sample_rate as u64 {
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
                        Err(error) => finish_capture(device, if hda::capture_fault_is_recoverable(error) {
                            CaptureState::Overrun
                        } else {
                            CaptureState::DeviceLost
                        }),
                    }
                }
            }
            if device.playing {
                let authorized = LEASE.and_then(|lease| now.map(|n| (lease, n)))
                    .and_then(|(lease, n)| crate::runtime::with_runtime(|runtime| lease.valid(&runtime.capabilities, n / 1_000_000_000))).unwrap_or(false);
                if !authorized { finish_playback(device, PlaybackState::Denied); }
                else if device.position().is_err() { finish_playback(device, PlaybackState::DeviceLost); }
                else if now.map(|n| n >= UNTIL).unwrap_or(true) {
                    finish_playback(device, if INFINITY_ACTIVE { PlaybackState::Underrun } else { PlaybackState::Complete });
                }
                else if RESIDENT_BYTES > 0 {
                    let position = device.position().unwrap_or(0) as usize;
                    if INFINITY_ACTIVE { INFINITY_PLAYED_FRAMES = (position / 4) as u64; }
                    if position >= RESIDENT_BYTES { finish_playback(device, PlaybackState::Complete); }
                }
            }
        }
    }
    LOCK.store(false, Ordering::Release);
}
