#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod ps2;
#[cfg(target_arch = "aarch64")]
pub(crate) mod uefi;
pub mod pointer;

use crate::boot_info::BootInfo;
pub use pointer::{AbsolutePointerEvent, PointerCapabilities, PointerEvent};

#[derive(Clone, Copy)]
pub enum InputSource {
    Keyboard,
    Mouse,
}
#[derive(Clone, Copy)]
pub enum InputAction {
    Pressed,
    Released,
    Motion,
}
#[derive(Clone, Copy)]
pub struct InputEvent {
    pub source: InputSource,
    pub action: InputAction,
    pub code: u16,
    pub modifiers: u8,
    pub delta_x: i16,
    pub delta_y: i16,
}
pub struct InputStatus {
    pub keyboard: bool,
    pub mouse: bool,
    pub pointer: PointerCapabilities,
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
// ------------------------=
// FUNC: initialize
// DESC: Initializes initialize state.
// ------------------=
pub fn initialize(_info: &BootInfo) -> InputStatus {
    ps2::initialize()
}
#[cfg(target_arch = "aarch64")]
// ------------------------=
// FUNC: initialize
// DESC: Initializes initialize state.
// ------------------=
pub fn initialize(info: &BootInfo) -> InputStatus {
    uefi::initialize(info)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
// ------------------------=
// FUNC: run
// DESC: Implements the run operation.
// ------------------=
pub fn run() -> ! {
    ps2::run()
}
#[cfg(target_arch = "aarch64")]
// ------------------------=
// FUNC: run
// DESC: Implements the run operation.
// ------------------=
pub fn run() -> ! {
    uefi::run()
}

// ------------------------=
// FUNC: dispatch
// DESC: Implements the dispatch operation.
// ------------------=
pub fn dispatch(event: InputEvent) {
    if matches!(
        (event.source, event.action),
        (InputSource::Keyboard, InputAction::Pressed)
    ) {
        if let Some(key) = console_key(event.code, event.modifiers & 1 != 0) {
            crate::console::input(key);
        }
    }
    let _ = (event.code, event.delta_x, event.delta_y);
}

// ------------------------=
// FUNC: dispatch_pointer
// DESC: Handles dispatch pointer input or state transitions.
// ------------------=
pub fn dispatch_pointer(event: PointerEvent) {
    crate::console::pointer(event.delta_x, event.delta_y, event.left_button());
    dispatch_pointer_wheel(event.wheel_x, event.wheel_y);
}

// ------------------------=
// FUNC: dispatch_pointer_absolute
// DESC: Handles dispatch pointer absolute input or state transitions.
// ------------------=
pub fn dispatch_pointer_absolute(event: AbsolutePointerEvent) {
    crate::console::pointer_absolute(event.x, event.y, event.left_button());
    dispatch_pointer_wheel(event.wheel_x, event.wheel_y);
}

// ------------------------=
// FUNC: dispatch_pointer_wheel
// DESC: Routes bounded wheel movement through the same accessible navigation actions as arrow keys.
// ------------------=
fn dispatch_pointer_wheel(horizontal: i8, vertical: i8) {
    let vertical_steps = (vertical as i16).unsigned_abs().min(8);
    let vertical_key = if vertical < 0 {
        crate::console::ConsoleKey::Up
    } else {
        crate::console::ConsoleKey::Down
    };
    for _ in 0..vertical_steps {
        crate::console::input(vertical_key);
    }
    let horizontal_steps = (horizontal as i16).unsigned_abs().min(8);
    let horizontal_key = if horizontal < 0 {
        crate::console::ConsoleKey::Left
    } else {
        crate::console::ConsoleKey::Right
    };
    for _ in 0..horizontal_steps {
        crate::console::input(horizontal_key);
    }
}

// ------------------------=
// FUNC: report_pointer_discovery
// DESC: Emits the pointer transports and features found during initial driver discovery.
// ------------------=
pub fn report_pointer_discovery(capabilities: PointerCapabilities) {
    if !capabilities.available() {
        crate::output_text(b"[input] no pointer interface discovered\n");
        return;
    }
    crate::output_text(b"[input] pointer driver ");
    crate::output_text(capabilities.driver_name().as_bytes());
    crate::output_text(b"; modes:");
    if capabilities.features & pointer::FEATURE_RELATIVE != 0 {
        crate::output_text(b" relative");
    }
    if capabilities.features & pointer::FEATURE_ABSOLUTE != 0 {
        crate::output_text(b" absolute");
    }
    if capabilities.features & pointer::FEATURE_WHEEL_VERTICAL != 0 {
        crate::output_text(b" wheel");
    }
    if capabilities.features & pointer::FEATURE_WHEEL_HORIZONTAL != 0 {
        crate::output_text(b" horizontal-wheel");
    }
    if capabilities.features & pointer::FEATURE_BUTTONS_EXTENDED != 0 {
        crate::output_text(b" five-button");
    } else if capabilities.features & pointer::FEATURE_BUTTONS_STANDARD != 0 {
        crate::output_text(b" buttons");
    }
    crate::output_text(b"\n");
}

// ------------------------=
// FUNC: console_key
// DESC: Implements the console key operation.
// ------------------=
fn console_key(usage: u16, shift: bool) -> Option<crate::console::ConsoleKey> {
    use crate::console::ConsoleKey;
    let character = match usage {
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
