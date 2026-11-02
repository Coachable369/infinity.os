pub mod pointer;
pub(crate) mod desktop_shortcuts;
pub(crate) mod modifiers;
mod buffer;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod ps2;
#[cfg(target_arch = "aarch64")]
pub(crate) mod uefi;
#[cfg(target_arch = "aarch64")]
mod firmware_key;
#[cfg(target_arch = "aarch64")]
mod firmware_keyboard;

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
static mut KEY_REPEAT: crate::ui::input_preferences::Repeat =
    crate::ui::input_preferences::Repeat::new();

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
// ------------------------=
// FUNC: initialize
// DESC: Initializes initialize state.
// ------------------=
pub fn initialize(_info: &BootInfo) -> InputStatus {
    modifiers::reset();
    ps2::initialize()
}
#[cfg(target_arch = "aarch64")]
// ------------------------=
// FUNC: initialize
// DESC: Initializes initialize state.
// ------------------=
pub fn initialize(info: &BootInfo) -> InputStatus {
    modifiers::reset();
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
        (InputSource::Keyboard, InputAction::Released)
    ) {
        unsafe {
            (&mut *(&raw mut KEY_REPEAT)).release(event.code);
        }
    }
    if matches!(
        (event.source, event.action),
        (InputSource::Keyboard, InputAction::Pressed)
    ) {
        if let Some(key) = modified_console_key(event.code, event.modifiers) {
            if let Some(now) = crate::ui::performance::monotonic_ns() {
                if !unsafe {
                    (&mut *(&raw mut KEY_REPEAT)).press(
                        event.code,
                        event.modifiers,
                        now / 1_000_000,
                        crate::ui::input_preferences::current(),
                    )
                } {
                    return;
                }
            }
            crate::console::input(key);
        }
    }
    let _ = (event.code, event.delta_x, event.delta_y);
}

// ------------------------=
// FUNC: keyboard_tick
// DESC: Services configured repeats on transports that report physical key-down and key-up events.
// ------------------=
pub fn keyboard_tick() {
    if let Some(now) = crate::ui::performance::monotonic_ns() {
        let next = unsafe {
            (&mut *(&raw mut KEY_REPEAT))
                .poll(now / 1_000_000, crate::ui::input_preferences::current())
        };
        if let Some((code, modifiers)) = next {
            if let Some(key) = modified_console_key(code, modifiers) {
                crate::console::input(key);
            }
        }
    }
}

// ------------------------=
// FUNC: dispatch_pointer
// DESC: Handles dispatch pointer input or state transitions.
// ------------------=
pub fn dispatch_pointer(event: PointerEvent) {
    crate::console::pointer_buttons(event.delta_x, event.delta_y, event.buttons);
    dispatch_pointer_wheel(event.wheel_x, event.wheel_y);
}

// ------------------------=
// FUNC: dispatch_pointer_absolute
// DESC: Handles dispatch pointer absolute input or state transitions.
// ------------------=
pub fn dispatch_pointer_absolute(event: AbsolutePointerEvent) {
    crate::console::pointer_absolute_buttons(event.x, event.y, event.buttons);
    dispatch_pointer_wheel(event.wheel_x, event.wheel_y);
}

// ------------------------=
// FUNC: dispatch_pointer_wheel
// DESC: Routes bounded wheel movement through the same accessible navigation actions as arrow keys.
// ------------------=
fn dispatch_pointer_wheel(horizontal: i8, vertical: i8) {
    if !crate::console::pointer_scroll(vertical) {
        let vertical_steps = (vertical as i16).unsigned_abs().min(8);
        let vertical_key = if vertical < 0 {
            crate::console::ConsoleKey::Up
        } else {
            crate::console::ConsoleKey::Down
        };
        for _ in 0..vertical_steps {
            crate::console::input(vertical_key);
        }
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

mod keyboard;
pub use keyboard::modified_console_key;
