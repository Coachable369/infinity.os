mod console {
    #[derive(Debug, PartialEq)]
    pub enum ConsoleKey {
        Shortcut(u8),
        SelectMove(i8),
        Character(u8),
        Backspace,
        Enter,
        Escape,
        Tab(bool),
        Up,
        Down,
        Left,
        Right,
        Home,
        End,
        Delete,
        Help,
    }
}
#[path = "../kernel/drivers/input/desktop_shortcuts.rs"]
mod desktop_shortcuts;
#[path = "../kernel/drivers/input/firmware_key.rs"]
mod firmware_key;

// ------------------------=
// FUNC: main
// DESC: Exercises actual firmware translation with extended and legacy key packets.
// ------------------=
fn main() {
    use console::ConsoleKey::*;
    use firmware_key::decode;
    for command in [0x40, 0x80, 0xc0] {
        assert_eq!(
            decode(0, 9, 0x8000_0000 | command),
            Some(Shortcut(desktop_shortcuts::SPATIAL_NEXT))
        );
        assert_eq!(
            decode(0, 9, 0x8000_0001 | command),
            Some(Shortcut(desktop_shortcuts::SPATIAL_PREVIOUS))
        );
        assert_eq!(decode(0, 9, command), Some(Tab(false)));
    }
    for bits in 0..=255u8 {
        let modifiers = desktop_shortcuts::hid_modifiers(bits);
        assert_eq!(modifiers & 1 != 0, bits & 0x22 != 0);
        assert_eq!(modifiers & 2 != 0, bits & 0x11 != 0);
        assert_eq!(modifiers & 4 != 0, bits & 0x88 != 0);
        assert_eq!(
            desktop_shortcuts::spatial_chord(true, modifiers & 4 != 0, modifiers & 1 != 0),
            if bits & 0x88 == 0 {
                None
            } else {
                Some(if bits & 0x22 == 0 {
                    desktop_shortcuts::SPATIAL_NEXT
                } else {
                    desktop_shortcuts::SPATIAL_PREVIOUS
                })
            }
        );
        assert_eq!(
            desktop_shortcuts::spatial_chord(false, modifiers & 4 != 0, modifiers & 1 != 0),
            None
        );
    }
    for letter in [b'n', b'e', b't', b'p', b'l'] {
        assert_eq!(
            decode(0, letter as u16, 0x8000_0005),
            Some(Shortcut(letter.to_ascii_uppercase()))
        );
    }
    for ctrl in [4, 8] {
        for value in [10, b'j' as u16, b'J' as u16] {
            assert_eq!(decode(0, value, 0x8000_0000 | ctrl), Some(Shortcut(b'j')));
        }
        assert_eq!(decode(0, 13, 0x8000_0000 | ctrl), Some(Shortcut(b'm')));
        assert_eq!(
            decode(0, b'c' as u16, 0x8000_0000 | ctrl),
            Some(Shortcut(b'c'))
        );
    }
    assert_eq!(decode(0, b'j' as u16, 8), Some(Character(b'j')));
    assert_eq!(decode(0, 10, 0), Some(Shortcut(b'j')));
    assert_eq!(decode(0, 9, 0x8000_0001), Some(Tab(true)));
    assert_eq!(decode(0, 9, 0), Some(Tab(false)));
    assert_eq!(decode(0, 13, 0), Some(Enter));
    assert_eq!(decode(0, 8, 0), Some(Backspace));
    assert_eq!(decode(4, 0, 0x8000_0002), Some(SelectMove(-1)));
    assert_eq!(decode(3, 0, 0), Some(Right));
    assert_eq!(decode(0, 0, 0x8000_0008), None);
}
