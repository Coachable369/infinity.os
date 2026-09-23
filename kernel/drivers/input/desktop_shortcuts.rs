//! Reserved Ctrl+Shift desktop workflows shared by all keyboard transports.
pub const SPATIAL_NEXT: u8 = 0x80;
pub const SPATIAL_PREVIOUS: u8 = 0x81;

// ------------------------=
// FUNC: switcher_request
// DESC: Reserves Shift-Tab and Command-Tab only within an authenticated desktop session.
// ------------------=
pub fn switcher_request(
    shift_tab: bool,
    shortcut: Option<u8>,
    authenticated: bool,
    desktop: bool,
) -> Option<bool> {
    if !authenticated || !desktop {
        return None;
    }
    if shift_tab || shortcut == Some(SPATIAL_PREVIOUS) {
        Some(true)
    } else if shortcut == Some(SPATIAL_NEXT) {
        Some(false)
    } else {
        None
    }
}

// ------------------------=
// FUNC: hid_modifiers
// DESC: Preserves Shift, Control and either Command/Super key from USB HID reports.
// ------------------=
pub fn hid_modifiers(bits: u8) -> u8 {
    u8::from(bits & 0x22 != 0)
        | (u8::from(bits & 0x11 != 0) << 1)
        | (u8::from(bits & 0x88 != 0) << 2)
}

// ------------------------=
// FUNC: spatial_chord
// DESC: Reserves Command/Super Tab for carousel navigation without consuming ordinary Tab.
// ------------------=
pub fn spatial_chord(tab: bool, command: bool, shift: bool) -> Option<u8> {
    (tab && command).then_some(if shift {
        SPATIAL_PREVIOUS
    } else {
        SPATIAL_NEXT
    })
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopAction {
    Notes,
    Home,
    Command,
    Tasks,
    Lock,
    SnapLeft,
    SnapRight,
    Center,
    Cycle,
    Grow,
    Shrink,
}

// ------------------------=
// FUNC: shortcut_code
// DESC: Keeps unshifted application commands while reserving shifted desktop workflow chords.
// ------------------=
pub fn shortcut_code(letter: u8, shift: bool) -> u8 {
    let letter = letter.to_ascii_lowercase();
    if shift
        && matches!(
            letter,
            b'n' | b'e' | b't' | b'p' | b'l' | b'h' | b'b' | b'g' | b'w' | b'u' | b'i' | b'k'
        )
    {
        letter.to_ascii_uppercase()
    } else if shift && letter == b'z' {
        b'y'
    } else {
        letter
    }
}

// ------------------------=
// FUNC: desktop_action
// DESC: Resolves reserved chords only in an authenticated, unlocked desktop surface.
// ------------------=
pub fn desktop_action(code: u8, authenticated: bool, desktop: bool) -> Option<DesktopAction> {
    if !authenticated || !desktop {
        return None;
    }
    Some(match code {
        b'N' => DesktopAction::Notes,
        b'E' => DesktopAction::Home,
        b'T' => DesktopAction::Command,
        b'P' => DesktopAction::Tasks,
        b'L' => DesktopAction::Lock,
        b'H' => DesktopAction::SnapLeft,
        b'B' => DesktopAction::SnapRight,
        b'G' => DesktopAction::Center,
        b'W' => DesktopAction::Cycle,
        b'U' => DesktopAction::Grow,
        b'I' => DesktopAction::Shrink,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: switcher_keys_preserve_login_and_plain_tab
    // DESC: Verifies both shortcut directions and the authenticated desktop boundary.
    // ------------------=
    fn switcher_keys_preserve_login_and_plain_tab() {
        for authenticated in [false, true] {
            for desktop in [false, true] {
                let enabled = authenticated && desktop;
                assert_eq!(
                    switcher_request(true, None, authenticated, desktop),
                    enabled.then_some(true)
                );
                assert_eq!(
                    switcher_request(false, Some(SPATIAL_NEXT), authenticated, desktop),
                    enabled.then_some(false)
                );
                assert_eq!(
                    switcher_request(false, Some(SPATIAL_PREVIOUS), authenticated, desktop),
                    enabled.then_some(true)
                );
                assert_eq!(switcher_request(false, None, authenticated, desktop), None);
                assert_eq!(
                    switcher_request(false, Some(b'c'), authenticated, desktop),
                    None
                );
            }
        }
    }
    // ------------------------=
    // FUNC: desktop_chords_preserve_app_commands_and_security_boundaries
    // DESC: Tests all workflows, unshifted collisions, locked surfaces, and redo compatibility.
    // ------------------=
    #[test]
    fn desktop_chords_preserve_app_commands_and_security_boundaries() {
        for (key, action) in [
            (b'n', DesktopAction::Notes),
            (b'e', DesktopAction::Home),
            (b't', DesktopAction::Command),
            (b'p', DesktopAction::Tasks),
            (b'l', DesktopAction::Lock),
            (b'h', DesktopAction::SnapLeft),
            (b'b', DesktopAction::SnapRight),
            (b'g', DesktopAction::Center),
            (b'w', DesktopAction::Cycle),
            (b'u', DesktopAction::Grow),
            (b'i', DesktopAction::Shrink),
        ] {
            let code = shortcut_code(key, true);
            assert_eq!(desktop_action(code, true, true), Some(action));
            assert_eq!(desktop_action(code, false, true), None);
            assert_eq!(desktop_action(code, true, false), None);
            assert_eq!(desktop_action(shortcut_code(key, false), true, true), None);
        }
        assert_eq!(shortcut_code(b'z', true), b'y');
        assert_eq!(shortcut_code(b'c', true), b'c');
        assert_eq!(desktop_action(b'J', true, true), None);
    }
}
