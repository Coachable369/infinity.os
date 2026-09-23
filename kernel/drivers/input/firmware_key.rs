//! UEFI key translation shared by the legacy and extended console bridges.
use crate::console::ConsoleKey;

// ------------------------=
// FUNC: decode
// DESC: Preserves valid firmware modifiers, including Ctrl letters and reverse Tab.
// ------------------=
pub fn decode(scan: u16, character: u16, shift_state: u32) -> Option<ConsoleKey> {
    let valid = shift_state & 0x8000_0000 != 0;
    let shift = valid && shift_state & 3 != 0;
    let control = valid && shift_state & 12 != 0;
    if let Some(code) = super::desktop_shortcuts::spatial_chord(
        scan == 0 && character == 9,
        valid && shift_state & 0xc0 != 0,
        shift,
    ) {
        return Some(ConsoleKey::Shortcut(code));
    }
    if control && scan == 0 {
        let letter = match character {
            1..=26 => b'a' + character as u8 - 1,
            65..=90 | 97..=122 => (character as u8).to_ascii_lowercase(),
            _ => 0,
        };
        if letter != 0 {
            return Some(ConsoleKey::Shortcut(
                super::desktop_shortcuts::shortcut_code(letter, shift),
            ));
        }
    }
    if shift {
        let direction = match scan {
            1 => -2,
            2 => 2,
            3 => 1,
            4 => -1,
            5 => -3,
            6 => 3,
            _ => 0,
        };
        if direction != 0 {
            return Some(ConsoleKey::SelectMove(direction));
        }
    }
    if scan != 0 {
        return Some(match scan {
            1 => ConsoleKey::Up,
            2 => ConsoleKey::Down,
            3 => ConsoleKey::Right,
            4 => ConsoleKey::Left,
            5 => ConsoleKey::Home,
            6 => ConsoleKey::End,
            8 => ConsoleKey::Delete,
            0x0b => ConsoleKey::Help,
            0x17 => ConsoleKey::Escape,
            _ => return None,
        });
    }
    Some(match character {
        8 => ConsoleKey::Backspace,
        9 => ConsoleKey::Tab(shift),
        13 => ConsoleKey::Enter,
        1..=26 => ConsoleKey::Shortcut(b'a' + character as u8 - 1),
        32..=126 => ConsoleKey::Character(character as u8),
        _ => return None,
    })
}
