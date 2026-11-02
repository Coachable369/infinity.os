//! Native HID keyboard translation shared by USB, PS/2 and focused behavioral tests.
use super::desktop_shortcuts;

// ------------------------=
// FUNC: modified_console_key
// DESC: Preserves control commands and selection navigation across native keyboard transports.
// ------------------=
pub fn modified_console_key(code:u16, modifiers:u8)->Option<crate::console::ConsoleKey> {
    use crate::console::ConsoleKey;
    if let Some(code) = desktop_shortcuts::application_chord(code, modifiers) {
        return Some(ConsoleKey::Shortcut(code));
    }
    let key=console_key(code,modifiers&1!=0)?;
    if modifiers&6!=0 {if let ConsoleKey::Character(c)=key {return Some(ConsoleKey::Shortcut(desktop_shortcuts::shortcut_code(c, modifiers&1!=0)));}}
    if modifiers&1!=0 {let direction=match key {ConsoleKey::Left=>-1,ConsoleKey::Right=>1,ConsoleKey::Up=>-2,ConsoleKey::Down=>2,ConsoleKey::Home=>-3,ConsoleKey::End=>3,_=>0};if direction!=0{return Some(ConsoleKey::SelectMove(direction));}}
    Some(key)
}

// ------------------------=
// FUNC: console_key
// DESC: Implements the console key operation.
// ------------------=
fn console_key(usage: u16, shift: bool) -> Option<crate::console::ConsoleKey> {
    use crate::console::ConsoleKey;
    let character = match usage {
        0x57 => b'+',
        0x56 => b'-',
        0x04..=0x1d => b'a' + (usage - 0x04) as u8,
        0x1e..=0x26 => {
            if shift {
                b"!@#$%^&*("[(usage - 0x1e) as usize]
            } else {
                b'1' + (usage - 0x1e) as u8
            }
        }
        0x27 => {
            if shift {
                b')'
            } else {
                b'0'
            }
        }
        0x2c => b' ',
        0x2d => {
            if shift {
                b'_'
            } else {
                b'-'
            }
        }
        0x2e => {
            if shift {
                b'+'
            } else {
                b'='
            }
        }
        0x2f => {
            if shift {
                b'{'
            } else {
                b'['
            }
        }
        0x30 => {
            if shift {
                b'}'
            } else {
                b']'
            }
        }
        0x31 => {
            if shift {
                b'|'
            } else {
                b'\\'
            }
        }
        0x33 => {
            if shift {
                b':'
            } else {
                b';'
            }
        }
        0x34 => {
            if shift {
                b'"'
            } else {
                b'\''
            }
        }
        0x35 => {
            if shift {
                b'~'
            } else {
                b'`'
            }
        }
        0x36 => {
            if shift {
                b'<'
            } else {
                b','
            }
        }
        0x37 => {
            if shift {
                b'>'
            } else {
                b'.'
            }
        }
        0x38 => {
            if shift {
                b'?'
            } else {
                b'/'
            }
        }
        0x28 => return Some(ConsoleKey::Enter),
        0x29 => return Some(ConsoleKey::Escape),
        0x2a => return Some(ConsoleKey::Backspace),
        0x2b => return Some(ConsoleKey::Tab(shift)),
        0x4f => return Some(ConsoleKey::Right),
        0x50 => return Some(ConsoleKey::Left),
        0x51 => return Some(ConsoleKey::Down),
        0x52 => return Some(ConsoleKey::Up),
        0x4a => return Some(ConsoleKey::Home),
        0x4d => return Some(ConsoleKey::End),
        0x4c => return Some(ConsoleKey::Delete),
        0x3a => return Some(ConsoleKey::Help),
        _ => return None,
    };
    Some(ConsoleKey::Character(
        if shift && character.is_ascii_lowercase() {
            character.to_ascii_uppercase()
        } else {
            character
        },
    ))
}
