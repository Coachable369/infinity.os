//! Trusted, capability-checked operating-system sound playback.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::runtime::{
    capability::CapabilityType,
    execution::SecurityIdentity,
    system_sound_policy::{
        BootOrigin, BootPhase, SystemSoundCue, SystemSoundEvent, cue_for_event,
    },
};

const SYSTEM_SOUND_OWNER: SecurityIdentity = SecurityIdentity([0xfe; 16]);
const BOOT_PCM_BYTES: usize = include_bytes!("../../assets/sounds/boot.pcm").len();
const LOGIN_PCM_BYTES: usize = include_bytes!("../../assets/sounds/login.pcm").len();

#[repr(align(2))]
struct AlignedPcm<const N: usize>([u8; N]);

static BOOT_PCM: AlignedPcm<BOOT_PCM_BYTES> =
    AlignedPcm(*include_bytes!("../../assets/sounds/boot.pcm"));
static LOGIN_PCM: AlignedPcm<LOGIN_PCM_BYTES> =
    AlignedPcm(*include_bytes!("../../assets/sounds/login.pcm"));
static BOOT_PLAYED: AtomicBool = AtomicBool::new(false);

#[repr(C, align(128))]
struct LoginResident([i16; 48_000 * 2 * 32]);
static mut LOGIN_RESIDENT: LoginResident = LoginResident([0; 48_000 * 2 * 32]);
#[path = "../drivers/speech_pcm.rs"]
mod login_pcm;

// ------------------------=
// FUNC: play_login_resident
// DESC: Prepares the entire login cue before animation so rendering stalls cannot starve a 100-ms refill stream.
// ------------------=
pub fn play_login_resident() -> bool {
    let Some(rate) = crate::drivers::audio::playback_rate() else { return false; };
    let Some(now_ns) = crate::ui::performance::monotonic_ns() else { return false; };
    // Only this kernel-owned cue may reuse the resident buffer. Stop its prior
    // DMA lease before touching the samples, never another user's playback.
    let _ = crate::drivers::audio::stop_playback(SYSTEM_SOUND_OWNER);
    if matches!(crate::drivers::audio::playback_state(), Some(crate::runtime::audio::PlaybackState::Playing) | None) {
        return false;
    }
    let samples = LOGIN_PCM.samples();
    let length = samples.len().saturating_mul(rate as usize).div_ceil(16_000) * 2;
    // HDA's DMA position leads audible codec/host output. Drain into resident
    // silence before stopping the stream, preserving the cue's reverb tail.
    let drained_length = length.saturating_add(rate as usize / 4 * 2);
    let capacity = rate as usize * 2 * 32;
    if length == 0 || drained_length > capacity || capacity > 48_000 * 2 * 32 { return false; }
    let now = now_ns / 1_000_000_000;
    let capability = crate::runtime::with_runtime(|runtime| runtime.capabilities.grant(
        CapabilityType::AudioOutput, 0, 1, 0, SYSTEM_SOUND_OWNER,
        SYSTEM_SOUND_OWNER, Some(now + 35), 0)).and_then(Result::ok);
    let Some(capability) = capability else { return false; };
    let started = unsafe {
        let output = &mut (&mut *(&raw mut LOGIN_RESIDENT.0))[..capacity];
        output.fill(0);
        login_pcm::fill(samples, &mut output[..length], 0, rate)
            && crate::drivers::audio::play_resident_speech(SYSTEM_SOUND_OWNER, capability,
                &(&*(&raw const LOGIN_RESIDENT.0))[..capacity], drained_length, rate)
    };
    if !started {
        crate::runtime::with_runtime(|runtime| {
            let _ = runtime.capabilities.retire_leaf(capability, SYSTEM_SOUND_OWNER);
        });
    }
    started
}

impl<const N: usize> AlignedPcm<N> {
    // ------------------------=
    // FUNC: samples
    // DESC: Exposes the build-verified little-endian 16-bit mono cue as aligned immutable samples.
    // ------------------=
    fn samples(&'static self) -> &'static [i16] {
        debug_assert_eq!(N % core::mem::size_of::<i16>(), 0);
        // SAFETY: AlignedPcm has i16 alignment, the generated payload has an even
        // byte count, and both storage and returned slice have static lifetime.
        unsafe { core::slice::from_raw_parts(self.0.as_ptr().cast::<i16>(), N / 2) }
    }
}

// ------------------------=
// FUNC: play
// DESC: Grants a short-lived kernel sound capability and submits bounded PCM through the native audio service.
// ------------------=
fn play(cue: SystemSoundCue) -> bool {
    let samples = match cue {
        SystemSoundCue::Boot => BOOT_PCM.samples(),
        SystemSoundCue::Login => LOGIN_PCM.samples(),
    };
    let Some(now_ns) = crate::ui::performance::monotonic_ns() else {
        return false;
    };
    let now = now_ns / 1_000_000_000;
    let _ = crate::drivers::audio::stop_playback(SYSTEM_SOUND_OWNER);
    let capability = crate::runtime::with_runtime(|runtime| {
        runtime.capabilities.grant(
            CapabilityType::AudioOutput,
            0,
            1,
            0,
            SYSTEM_SOUND_OWNER,
            SYSTEM_SOUND_OWNER,
            Some(now + 35),
            0,
        )
    })
    .and_then(Result::ok);
    let Some(capability) = capability else {
        return false;
    };
    if crate::drivers::audio::play_speech(SYSTEM_SOUND_OWNER, capability, samples) {
        true
    } else {
        crate::runtime::with_runtime(|runtime| {
            let _ = runtime
                .capabilities
                .retire_leaf(capability, SYSTEM_SOUND_OWNER);
        });
        false
    }
}

// ------------------------=
// FUNC: play_boot_once
// DESC: Plays boot.mp3's runtime PCM once, after native output hardware becomes available.
// ------------------=
pub fn play_boot_once() -> bool {
    let available = crate::drivers::audio::available();
    if cue_for_event(
        SystemSoundEvent::BootTransition {
            origin: if cfg!(feature = "installer") {
                BootOrigin::Iso
            } else {
                BootOrigin::Installed
            },
            phase: BootPhase::NativeAudioReady,
            audio_available: available,
        },
        BOOT_PLAYED.load(Ordering::Acquire),
    ) != Some(SystemSoundCue::Boot)
        || BOOT_PLAYED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
    {
        return false;
    }
    if play(SystemSoundCue::Boot) {
        true
    } else {
        BOOT_PLAYED.store(false, Ordering::Release);
        false
    }
}

// ------------------------=
// FUNC: play_login
// DESC: Plays login.mp3's runtime PCM only for an accepted authentication transition.
// ------------------=
pub fn play_login() -> bool {
    if cue_for_event(
        SystemSoundEvent::Authentication { accepted: true },
        BOOT_PLAYED.load(Ordering::Acquire),
    ) == Some(SystemSoundCue::Login)
    {
        play(SystemSoundCue::Login)
    } else {
        false
    }
}
