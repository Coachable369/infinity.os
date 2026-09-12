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
#[path = "../kernel/drivers/input/firmware_key.rs"]
mod firmware_key;

// ------------------------=
// FUNC: main
// DESC: Exercises actual firmware translation with extended and legacy key packets.
// ------------------=
fn main() {
    use console::ConsoleKey::*;
    use firmware_key::decode;
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
