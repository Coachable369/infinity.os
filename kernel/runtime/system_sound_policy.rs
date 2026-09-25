//! Deterministic policy for trusted operating-system sound cues.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemSoundCue {
    Boot,
    Login,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemSoundEvent {
    BootReady { audio_available: bool },
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
        SystemSoundEvent::BootReady {
            audio_available: true,
        } if !boot_already_played => Some(SystemSoundCue::Boot),
        SystemSoundEvent::Authentication { accepted: true } => Some(SystemSoundCue::Login),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{cue_for_event, SystemSoundCue, SystemSoundEvent};

    // ------------------------=
    // FUNC: emits_boot_once_only_after_audio_is_ready
    // DESC: Verifies the boot transition cannot emit before hardware readiness or more than once.
    // ------------------=
    #[test]
    fn emits_boot_once_only_after_audio_is_ready() {
        assert_eq!(
            cue_for_event(
                SystemSoundEvent::BootReady {
                    audio_available: false,
                },
                false,
            ),
            None
        );
        assert_eq!(
            cue_for_event(
                SystemSoundEvent::BootReady {
                    audio_available: true,
                },
                false,
            ),
            Some(SystemSoundCue::Boot)
        );
        assert_eq!(
            cue_for_event(
                SystemSoundEvent::BootReady {
                    audio_available: true,
                },
                true,
            ),
            None
        );
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
