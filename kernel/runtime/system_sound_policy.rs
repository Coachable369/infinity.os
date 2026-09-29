//! Deterministic policy for trusted operating-system sound cues.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemSoundCue {
    Boot,
    Login,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootOrigin {
    Iso,
    Installed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootPhase {
    FirmwareEntry,
    ReadyScreenAudioAvailable,
    UserInterfaceReady,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemSoundEvent {
    BootTransition {
        origin: BootOrigin,
        phase: BootPhase,
        audio_available: bool,
    },
    Authentication { accepted: bool },
}

// ------------------------=
// FUNC: cue_for_event
// DESC: Maps observable boot and authentication transitions to their sound cue without emitting false-success audio.
// ------------------=
pub const fn cue_for_event(
    event: SystemSoundEvent,
    boot_already_played: bool,
) -> Option<SystemSoundCue> {
    match event {
        SystemSoundEvent::BootTransition {
            phase: BootPhase::ReadyScreenAudioAvailable,
            audio_available: true,
            ..
        } if !boot_already_played => Some(SystemSoundCue::Boot),
        SystemSoundEvent::Authentication { accepted: true } => Some(SystemSoundCue::Login),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{BootOrigin, BootPhase, SystemSoundCue, SystemSoundEvent, cue_for_event};

    // ------------------------=
    // FUNC: emits_boot_once_only_on_the_ready_screen
    // DESC: Verifies both boot origins emit only when the completed progress screen and native audio are ready.
    // ------------------=
    #[test]
    fn emits_boot_once_only_on_the_ready_screen() {
        for origin in [BootOrigin::Iso, BootOrigin::Installed] {
            assert_eq!(
                cue_for_event(
                    SystemSoundEvent::BootTransition {
                        origin,
                        phase: BootPhase::FirmwareEntry,
                        audio_available: false,
                    },
                    false,
                ),
                None
            );
            assert_eq!(
                cue_for_event(
                    SystemSoundEvent::BootTransition {
                        origin,
                        phase: BootPhase::ReadyScreenAudioAvailable,
                        audio_available: true,
                    },
                    false,
                ),
                Some(SystemSoundCue::Boot)
            );
            assert_eq!(
                cue_for_event(
                    SystemSoundEvent::BootTransition {
                        origin,
                        phase: BootPhase::UserInterfaceReady,
                        audio_available: true,
                    },
                    false,
                ),
                None
            );
            assert_eq!(
                cue_for_event(
                    SystemSoundEvent::BootTransition {
                        origin,
                        phase: BootPhase::ReadyScreenAudioAvailable,
                        audio_available: true,
                    },
                    true,
                ),
                None
            );
        }
    }

    // ------------------------=
    // FUNC: emits_login_only_for_accepted_authentication
    // DESC: Verifies failed credential checks remain silent while every accepted login produces its cue.
    // ------------------=
    #[test]
    fn emits_login_only_for_accepted_authentication() {
        assert_eq!(
            cue_for_event(SystemSoundEvent::Authentication { accepted: false }, false),
            None
        );
        assert_eq!(
            cue_for_event(SystemSoundEvent::Authentication { accepted: true }, false),
            Some(SystemSoundCue::Login)
        );
    }
}
