use super::{
    dispatch, dispatch_pointer, pointer, InputAction, InputEvent, InputSource, InputStatus,
    PointerCapabilities,
};
use core::arch::asm;

const DATA: u16 = 0x60;
const STATUS_COMMAND: u16 = 0x64;
const TIMEOUT: usize = 100_000;
static mut MOUSE_DEVICE_ID: u8 = 0;

// ------------------------=
// FUNC: outb
// DESC: Implements the outb operation.
// ------------------=
unsafe fn outb(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack));
}
// ------------------------=
// FUNC: inb
// DESC: Implements the inb operation.
// ------------------=
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    asm!("in al, dx", in("dx") port, out("al") value, options(nomem, nostack));
    value
}
// ------------------------=
// FUNC: wait_write
// DESC: Implements the wait write operation.
// ------------------=
fn wait_write() -> bool {
    for _ in 0..TIMEOUT {
        if unsafe { inb(STATUS_COMMAND) } & 2 == 0 {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}
// ------------------------=
// FUNC: wait_read
// DESC: Implements the wait read operation.
// ------------------=
fn wait_read() -> bool {
    for _ in 0..TIMEOUT {
        if unsafe { inb(STATUS_COMMAND) } & 1 != 0 {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}
// ------------------------=
// FUNC: command
// DESC: Implements the command operation.
// ------------------=
fn command(value: u8) -> bool {
    if !wait_write() {
        return false;
    }
    unsafe {
        outb(STATUS_COMMAND, value);
    }
    true
}
// ------------------------=
// FUNC: data
// DESC: Implements the data operation.
// ------------------=
fn data(value: u8) -> bool {
    if !wait_write() {
        return false;
    }
    unsafe {
        outb(DATA, value);
    }
    true
}
// ------------------------=
// FUNC: response
// DESC: Implements the response operation.
// ------------------=
fn response() -> Option<u8> {
    if wait_read() {
        Some(unsafe { inb(DATA) })
    } else {
        None
    }
}
// ------------------------=
// FUNC: mouse_command
// DESC: Implements the mouse command operation.
// ------------------=
fn mouse_command(value: u8) -> bool {
    command(0xd4) && data(value) && response() == Some(0xfa)
}

// ------------------------=
// FUNC: mouse_set_sample_rate
// DESC: Programs one PS/2 mouse sample rate while preserving bounded controller waits.
// ------------------=
fn mouse_set_sample_rate(rate: u8) -> bool {
    mouse_command(0xf3) && mouse_command(rate)
}

// ------------------------=
// FUNC: mouse_device_id
// DESC: Requests the current PS/2 pointing-device protocol identifier.
// ------------------=
fn mouse_device_id() -> Option<u8> {
    if mouse_command(0xf2) {
        response()
    } else {
        None
    }
}

// ------------------------=
// FUNC: negotiate_mouse_protocol
// DESC: Enables IntelliMouse wheel and Explorer five-button packets when the device supports them.
// ------------------=
fn negotiate_mouse_protocol() -> u8 {
    if !(mouse_set_sample_rate(200) && mouse_set_sample_rate(100) && mouse_set_sample_rate(80)) {
        return 0;
    }
    let wheel_id = mouse_device_id().unwrap_or(0);
    if wheel_id != 3 {
        return 0;
    }
    if mouse_set_sample_rate(200)
        && mouse_set_sample_rate(200)
        && mouse_set_sample_rate(80)
        && mouse_device_id() == Some(4)
    {
        4
    } else {
        3
    }
}
// ------------------------=
// FUNC: key_code
// DESC: Handles key code input or state transitions.
// ------------------=
fn key_code(scan: u8) -> u16 {
    match scan {
        0x1e => 0x04,
        0x30 => 0x05,
        0x2e => 0x06,
        0x20 => 0x07,
        0x12 => 0x08,
        0x21 => 0x09,
        0x22 => 0x0a,
        0x23 => 0x0b,
        0x17 => 0x0c,
        0x24 => 0x0d,
        0x25 => 0x0e,
        0x26 => 0x0f,
        0x32 => 0x10,
        0x31 => 0x11,
        0x18 => 0x12,
        0x19 => 0x13,
        0x10 => 0x14,
        0x13 => 0x15,
        0x1f => 0x16,
        0x14 => 0x17,
        0x16 => 0x18,
        0x2f => 0x19,
        0x11 => 0x1a,
        0x2d => 0x1b,
        0x15 => 0x1c,
        0x2c => 0x1d,
        0x1c => 0x28,
        0x01 => 0x29,
        0x02 => 0x1e,
        0x03 => 0x1f,
        0x04 => 0x20,
        0x05 => 0x21,
        0x06 => 0x22,
        0x07 => 0x23,
        0x08 => 0x24,
        0x09 => 0x25,
        0x0a => 0x26,
        0x0b => 0x27,
        0x0c => 0x2d,
        0x0d => 0x2e,
        0x0e => 0x2a,
        0x3b => 0x3a,
        0x1a => 0x2f,
        0x1b => 0x30,
        0x2b => 0x31,
        0x27 => 0x33,
        0x28 => 0x34,
        0x29 => 0x35,
        0x33 => 0x36,
        0x34 => 0x37,
        0x35 => 0x38,
        0x39 => 0x2c,
        0x0f => 0x2b,
        0x47 => 0x4a,
        0x4f => 0x4d,
        0x53 => 0x4c,
        _ => 0,
    }
}

// ------------------------=
// FUNC: initialize
// DESC: Initializes initialize state.
// ------------------=
pub fn initialize() -> InputStatus {
    for _ in 0..32 {
        if unsafe { inb(STATUS_COMMAND) } & 1 == 0 {
            break;
        }
        let _ = unsafe { inb(DATA) };
    }
    let keyboard = command(0xae) && data(0xf4) && response() == Some(0xfa);
    let mouse_present = command(0xa8) && mouse_command(0xf6);
    let device_id = if mouse_present {
        negotiate_mouse_protocol()
    } else {
        0
    };
    let mouse = mouse_present && mouse_command(0xf4);
    unsafe {
        MOUSE_DEVICE_ID = device_id;
    }
    let mut features = pointer::FEATURE_RELATIVE | pointer::FEATURE_BUTTONS_STANDARD;
    if device_id == 3 || device_id == 4 {
        features |= pointer::FEATURE_WHEEL_VERTICAL;
    }
    if device_id == 4 {
        features |= pointer::FEATURE_BUTTONS_EXTENDED;
    }
    let pointer = if mouse {
        PointerCapabilities {
            transports: pointer::TRANSPORT_PS2,
            features,
            interface_count: 1,
        }
    } else {
        PointerCapabilities::NONE
    };
    InputStatus {
        keyboard,
        mouse,
        pointer,
    }
}

// ------------------------=
// FUNC: run
// DESC: Implements the run operation.
// ------------------=
pub fn run() -> ! {
    let mut mouse_packet = [0u8; 4];
    let mut mouse_index = 0usize;
    let mouse_device_id = unsafe { MOUSE_DEVICE_ID };
    let mouse_packet_size = if mouse_device_id == 3 || mouse_device_id == 4 {
        4
    } else {
        3
    };
    let mut shift = false;
    let mut extended = false;
    loop {
        crate::drivers::network::poll();
        let status = unsafe { inb(STATUS_COMMAND) };
        if status & 1 == 0 {
            crate::bootstrap::animation_tick();
            core::hint::spin_loop();
            continue;
        }
        let value = unsafe { inb(DATA) };
        if status & 0x20 != 0 {
            if mouse_index == 0 && value & 0x08 == 0 {
                continue;
            }
            mouse_packet[mouse_index] = value;
            mouse_index += 1;
            if mouse_index == mouse_packet_size {
                if let Some(event) =
                    pointer::decode_ps2_packet(&mouse_packet[..mouse_packet_size], mouse_device_id)
                {
                    dispatch_pointer(event);
                }
                mouse_index = 0;
            }
        } else {
            crate::console::input_batch(|| {
                dispatch_keyboard(value, &mut shift, &mut extended);
                // Bounded drain preserves every make/break event. Do not consume
                // mouse bytes here: pointer presses retain immediate presentation.
                for _ in 1..64 {
                    let next = unsafe { inb(STATUS_COMMAND) };
                    if next & 1 == 0 || next & 0x20 != 0 { break; }
                    dispatch_keyboard(unsafe { inb(DATA) }, &mut shift, &mut extended);
                }
            });
        }
        // Service queued controller bytes before spending time on the
        // high-resolution bootstrap effects pass.
        crate::bootstrap::animation_tick();
    }
}

// ------------------------=
// FUNC: dispatch_keyboard
// DESC: Decodes each ordered PS/2 make/break byte while retaining modifier and extended-prefix state across bounded drains.
// ------------------=
fn dispatch_keyboard(value: u8, shift: &mut bool, extended: &mut bool) {
    if value == 0xe0 { *extended = true; return; }
    let scan = value & 0x7f;
    if scan == 0x2a || scan == 0x36 { *shift = value & 0x80 == 0; return; }
    let code = if *extended { match scan { 0x4d => 0x4f, 0x4b => 0x50, 0x50 => 0x51, 0x48 => 0x52, _ => 0 } }
        else { key_code(scan) };
    *extended = false;
    dispatch(InputEvent { source: InputSource::Keyboard,
        action: if value & 0x80 == 0 { InputAction::Pressed } else { InputAction::Released },
        code, modifiers: *shift as u8, delta_x: 0, delta_y: 0 });
}
