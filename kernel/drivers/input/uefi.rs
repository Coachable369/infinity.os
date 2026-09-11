use super::{
    dispatch, dispatch_pointer, dispatch_pointer_absolute, pointer, AbsolutePointerEvent,
    InputAction, InputEvent, InputSource, InputStatus, PointerCapabilities, PointerEvent,
};
use crate::boot_info::BootInfo;

#[repr(C)]
struct InputKey {
    scan_code: u16,
    unicode_character: u16,
}

#[repr(C)]
struct SimpleTextInput {
    reset: usize,
    read_key_stroke: unsafe extern "C" fn(*mut SimpleTextInput, *mut InputKey) -> u64,
    wait_for_key: usize,
}

#[repr(C)]
struct PointerState {
    relative_x: i32,
    relative_y: i32,
    relative_z: i32,
    left_button: u8,
    right_button: u8,
}

#[repr(C)]
struct SimplePointer {
    reset: unsafe extern "C" fn(*mut SimplePointer, u8) -> u64,
    get_state: unsafe extern "C" fn(*mut SimplePointer, *mut PointerState) -> u64,
    wait_for_input: usize,
    mode: usize,
}

#[repr(C)]
struct AbsolutePointerMode {
    min_x: u64,
    min_y: u64,
    min_z: u64,
    max_x: u64,
    max_y: u64,
    max_z: u64,
    attributes: u32,
}

#[repr(C)]
struct AbsolutePointerState {
    current_x: u64,
    current_y: u64,
    current_z: u64,
    active_buttons: u32,
}

#[repr(C)]
struct AbsolutePointer {
    reset: unsafe extern "C" fn(*mut AbsolutePointer, u8) -> u64,
    get_state: unsafe extern "C" fn(*mut AbsolutePointer, *mut AbsolutePointerState) -> u64,
    wait_for_input: usize,
    mode: *const AbsolutePointerMode,
}

const MAX_POINTER_PROTOCOLS: usize = 8;

#[repr(C)]
struct FirmwarePointers {
    check_event: usize,
    absolute_count: u32,
    relative_count: u32,
    usb_mouse_count: u32,
    reserved: u32,
    absolute: [*mut AbsolutePointer; MAX_POINTER_PROTOCOLS],
    relative: [*mut SimplePointer; MAX_POINTER_PROTOCOLS],
    usb_mouse: [*mut UsbIo; MAX_POINTER_PROTOCOLS],
    usb_endpoint: [u8; MAX_POINTER_PROTOCOLS],
    usb_mouse_absolute: [u8; MAX_POINTER_PROTOCOLS],
    usb_keyboard_count: u32,
    reserved2: u32,
    usb_keyboard: [*mut UsbIo; MAX_POINTER_PROTOCOLS],
    usb_keyboard_endpoint: [u8; MAX_POINTER_PROTOCOLS],
    usb_mouse_async: u32,
    usb_mouse_sequence: u32,
    usb_mouse_length: u32,
    usb_mouse_report: [u8; 8],
    block_count: u32,
    reserved3: u32,
    blocks: [usize; MAX_POINTER_PROTOCOLS],
}

#[repr(C)]
struct UsbIo {
    control_transfer: usize,
    bulk_transfer: usize,
    async_interrupt_transfer: usize,
    sync_interrupt_transfer: usize,
    isochronous_transfer: usize,
    async_isochronous_transfer: usize,
    get_device_descriptor: usize,
    get_config_descriptor: usize,
    get_interface_descriptor: usize,
    get_endpoint_descriptor: usize,
    port_reset: usize,
}

static mut INPUT: *mut SimpleTextInput = core::ptr::null_mut();
static mut POINTER: *mut SimplePointer = core::ptr::null_mut();
static mut ABSOLUTE_POINTER: *mut AbsolutePointer = core::ptr::null_mut();
static mut POINTERS: *mut FirmwarePointers = core::ptr::null_mut();
static mut USB_KEYS: [u8; 6] = [0; 6];
static mut USB_MOUSE_BUTTONS: [u8; MAX_POINTER_PROTOCOLS] = [0; MAX_POINTER_PROTOCOLS];
static mut USB_MOUSE_ABSOLUTE_X: [u16; MAX_POINTER_PROTOCOLS] = [0; MAX_POINTER_PROTOCOLS];
static mut USB_MOUSE_ABSOLUTE_Y: [u16; MAX_POINTER_PROTOCOLS] = [0; MAX_POINTER_PROTOCOLS];
static mut POINTER_BUTTONS: pointer::PointerButtonArbiter = pointer::PointerButtonArbiter::new();
static mut USB_MOUSE_SEQUENCE: u32 = 0;
static mut USB_MOUSE_ASYNC_IDLE: u32 = 0;
static mut USE_DIRECT_USB_MOUSE: bool = false;
static mut USE_DIRECT_USB_KEYBOARD: bool = false;
static mut USE_ABSOLUTE_MOVEMENT: bool = false;
static mut RELATIVE_UPDATED_THIS_POLL: bool = false;
// True only during a poll cycle in which an Absolute Pointer protocol
// actually returned a state. Some VirtualBox ARM firmware advertises the
// protocol but stops producing states after ExitBootServices; raw USB HID
// motion must remain available as the live fallback in that case.
static mut ABSOLUTE_UPDATED_THIS_POLL: bool = false;

// ------------------------=
// FUNC: firmware_absolute_buttons
// DESC: Updates the absolute-pointer source and returns the composite held-button state.
// ------------------=
fn firmware_absolute_buttons(buttons: u8) -> u8 {
    unsafe {
        let state = &raw mut POINTER_BUTTONS;
        (*state).set_firmware_absolute(buttons);
        (*state).combined()
    }
}

// ------------------------=
// FUNC: firmware_relative_buttons
// DESC: Updates the relative-pointer source and returns the composite held-button state.
// ------------------=
fn firmware_relative_buttons(buttons: u8) -> u8 {
    unsafe {
        let state = &raw mut POINTER_BUTTONS;
        (*state).set_firmware_relative(buttons);
        (*state).combined()
    }
}

// ------------------------=
// FUNC: asynchronous_usb_buttons
// DESC: Updates the asynchronous USB source and returns the composite held-button state.
// ------------------=
fn asynchronous_usb_buttons(buttons: u8) -> u8 {
    unsafe {
        let state = &raw mut POINTER_BUTTONS;
        (*state).set_asynchronous_usb(buttons);
        (*state).combined()
    }
}

// ------------------------=
// FUNC: synchronous_usb_buttons
// DESC: Updates one synchronous USB source and returns the composite held-button state.
// ------------------=
fn synchronous_usb_buttons(index: usize, buttons: u8) -> u8 {
    unsafe {
        let state = &raw mut POINTER_BUTTONS;
        (*state).set_usb(index, buttons);
        (*state).combined()
    }
}

// ------------------------=
// FUNC: block_io
// DESC: Implements the block io operation.
// ------------------=
pub(crate) fn block_io(index: usize) -> Option<usize> {
    let pointers = unsafe { POINTERS.as_ref() }?;
    if index >= (pointers.block_count as usize).min(MAX_POINTER_PROTOCOLS) {
        return None;
    }
    let address = pointers.blocks[index];
    if address == 0 {
        None
    } else {
        Some(address)
    }
}

type CheckEvent = unsafe extern "efiapi" fn(usize) -> u64;
type AsyncInterruptTransfer =
    unsafe extern "efiapi" fn(*mut UsbIo, u8, u8, usize, usize, usize, usize) -> u64;
type SyncInterruptTransfer =
    unsafe extern "efiapi" fn(*mut UsbIo, u8, *mut u8, *mut usize, u32, *mut u32) -> u64;

// ------------------------=
// FUNC: initialize
// DESC: Initializes initialize state.
// ------------------=
pub fn initialize(info: &BootInfo) -> InputStatus {
    unsafe {
        POINTER_BUTTONS = pointer::PointerButtonArbiter::new();
        USB_MOUSE_BUTTONS = [0; MAX_POINTER_PROTOCOLS];
    }
    if info.boot_flags & 16 != 0 && info.firmware_pointer != 0 {
        unsafe {
            POINTERS = info.firmware_pointer as usize as *mut FirmwarePointers;
        }
        if let Some(pointers) = unsafe { POINTERS.as_ref() } {
            // Keep both firmware and raw HID paths available. VirtualBox ARM
            // delivers through Simple/Absolute Pointer while QEMU's UEFI can
            // advertise a protocol yet deliver only through the boot endpoint.
            // Per-poll update flags below prevent applying the same motion twice.
            unsafe {
                USE_DIRECT_USB_MOUSE = pointers.usb_mouse_count != 0;
            }
            // A VirtualBox USB tablet and its raw HID endpoint describe the
            // same physical pointer. AbsolutePointer provides smooth motion;
            // raw HID remains active only to supply button edges that the
            // firmware protocol sometimes drops. Never apply both motions.
            unsafe {
                USE_ABSOLUTE_MOVEMENT = pointers.absolute_count != 0;
            }
            unsafe {
                USE_DIRECT_USB_KEYBOARD =
                    info.boot_flags & 1 == 0 && pointers.usb_keyboard_count != 0;
            }
            for &pointer in pointers
                .absolute
                .iter()
                .take(pointers.absolute_count as usize)
            {
                if !pointer.is_null() {
                    unsafe {
                        ((*pointer).reset)(pointer, 0);
                    }
                }
            }
            for &pointer in pointers
                .relative
                .iter()
                .take(pointers.relative_count as usize)
            {
                if !pointer.is_null() {
                    unsafe {
                        ((*pointer).reset)(pointer, 0);
                    }
                }
            }
        }
    } else if info.boot_flags & 8 != 0 && info.firmware_pointer != 0 {
        unsafe {
            ABSOLUTE_POINTER = info.firmware_pointer as usize as *mut AbsolutePointer;
        }
        unsafe {
            ((*ABSOLUTE_POINTER).reset)(ABSOLUTE_POINTER, 0);
        }
    } else if info.boot_flags & 4 != 0 && info.firmware_pointer != 0 {
        unsafe {
            POINTER = info.firmware_pointer as usize as *mut SimplePointer;
        }
        unsafe {
            ((*POINTER).reset)(POINTER, 0);
        }
    }
    let pointer = discovered_pointer_capabilities();
    if info.boot_flags & 2 != 0 && info.firmware_input != 0 {
        unsafe {
            INPUT = info.firmware_input as usize as *mut SimpleTextInput;
        }
        InputStatus {
            keyboard: true,
            mouse: pointer.available(),
            pointer,
        }
    } else {
        InputStatus {
            keyboard: false,
            mouse: pointer.available(),
            pointer,
        }
    }
}

// ------------------------=
// FUNC: run
// DESC: Implements the run operation.
// ------------------=
pub fn run() -> ! {
    loop {
        crate::drivers::network::poll();
        let input = unsafe { INPUT };
        let direct_keyboard = unsafe { USE_DIRECT_USB_KEYBOARD }
            && unsafe { POINTERS.as_ref() }
                .map(|pointers| pointers.usb_keyboard_count != 0)
                .unwrap_or(false);
        if !direct_keyboard && !input.is_null() {
            let mut key = InputKey {
                scan_code: 0,
                unicode_character: 0,
            };
            let status = unsafe { ((*input).read_key_stroke)(input, &mut key) };
            if status == 0 {
                let character = key.unicode_character;
                let console_key = match character {
                    8 => Some(crate::console::ConsoleKey::Backspace),
                    9 => Some(crate::console::ConsoleKey::Tab(false)),
                    13 => Some(crate::console::ConsoleKey::Enter),
                    1..=26 => Some(crate::console::ConsoleKey::Shortcut(b'a' + character as u8 - 1)),
                    32..=126 => Some(crate::console::ConsoleKey::Character(character as u8)),
                    _ if key.scan_code == 0x01 => Some(crate::console::ConsoleKey::Up),
                    _ if key.scan_code == 0x02 => Some(crate::console::ConsoleKey::Down),
                    _ if key.scan_code == 0x03 => Some(crate::console::ConsoleKey::Right),
                    _ if key.scan_code == 0x04 => Some(crate::console::ConsoleKey::Left),
                    _ if key.scan_code == 0x05 => Some(crate::console::ConsoleKey::Home),
                    _ if key.scan_code == 0x06 => Some(crate::console::ConsoleKey::End),
                    _ if key.scan_code == 0x08 => Some(crate::console::ConsoleKey::Delete),
                    _ if key.scan_code == 0x0b => Some(crate::console::ConsoleKey::Help),
                    _ if key.scan_code == 0x17 => Some(crate::console::ConsoleKey::Escape),
                    _ => None,
                };
                if let Some(key) = console_key {
                    crate::console::input(key);
                }
            }
        }
        let pointers = unsafe { POINTERS };
        if let Some(pointers) = unsafe { pointers.as_mut() } {
            unsafe {
                ABSOLUTE_UPDATED_THIS_POLL = false;
                RELATIVE_UPDATED_THIS_POLL = false;
            }
            for &pointer in pointers
                .absolute
                .iter()
                .take(pointers.absolute_count as usize)
            {
                poll_absolute(pointer);
            }
            for &pointer in pointers
                .relative
                .iter()
                .take(pointers.relative_count as usize)
            {
                poll_relative(pointer);
            }
            if unsafe { USE_DIRECT_USB_MOUSE } {
                if pointers.usb_mouse_async != 0 {
                    poll_async_usb_mouse(pointers);
                    // VirtualBox's USB-tablet mode exposes both a relative
                    // boot mouse and a protocol-zero absolute tablet. The
                    // relative endpoint uses the shared asynchronous report,
                    // but the absolute endpoint must still be polled
                    // synchronously only when the firmware protocol did not
                    // produce a current state. Polling both live paths on each
                    // gesture queued redundant xHCI work behind the bootstrap
                    // effects renderer and made the cursor trail the host.
                    for index in 0..(pointers.usb_mouse_count as usize).min(MAX_POINTER_PROTOCOLS) {
                        if pointers.usb_mouse_absolute[index] != 0
                            && !unsafe { USE_ABSOLUTE_MOVEMENT && ABSOLUTE_UPDATED_THIS_POLL }
                        {
                            poll_usb_mouse(
                                index,
                                pointers.usb_mouse[index],
                                pointers.usb_endpoint[index],
                                true,
                            );
                        }
                    }
                } else {
                    for index in 0..(pointers.usb_mouse_count as usize).min(MAX_POINTER_PROTOCOLS) {
                        poll_usb_mouse(
                            index,
                            pointers.usb_mouse[index],
                            pointers.usb_endpoint[index],
                            pointers.usb_mouse_absolute[index] != 0,
                        );
                    }
                }
            }
            if unsafe { USE_DIRECT_USB_KEYBOARD } {
                for index in 0..(pointers.usb_keyboard_count as usize).min(MAX_POINTER_PROTOCOLS) {
                    poll_usb_keyboard(
                        pointers.usb_keyboard[index],
                        pointers.usb_keyboard_endpoint[index],
                    );
                }
            }
        } else {
            let absolute_pointer = unsafe { ABSOLUTE_POINTER };
            if !absolute_pointer.is_null() {
                poll_absolute(absolute_pointer);
            } else {
                poll_relative(unsafe { POINTER });
            }
        }
        // Input always wins the current loop iteration. The bootstrap particle
        // pass can be comparatively expensive under an emulated ARM GPU; doing
        // it first let HID reports queue behind visual effects and made only
        // the animated startup screen feel delayed.
        crate::bootstrap::animation_tick();
    }
}

// ------------------------=
// FUNC: poll_async_usb_mouse
// DESC: Implements the poll async usb mouse operation.
// ------------------=
fn poll_async_usb_mouse(pointers: &mut FirmwarePointers) {
    let sequence = unsafe { core::ptr::read_volatile(&raw const pointers.usb_mouse_sequence) };
    if sequence == unsafe { USB_MOUSE_SEQUENCE } {
        let idle = unsafe { USB_MOUSE_ASYNC_IDLE }.saturating_add(1);
        unsafe {
            USB_MOUSE_ASYNC_IDLE = idle;
        }
        // VirtualBox ARM accepts the async transfer but may stop scheduling
        // callbacks after control enters the kernel. Fall back to bounded
        // synchronous polling before the user's first gesture can be lost.
        if idle == 64 {
            for index in 0..(pointers.usb_mouse_count as usize).min(MAX_POINTER_PROTOCOLS) {
                let usb = pointers.usb_mouse[index];
                let endpoint = pointers.usb_endpoint[index];
                if usb.is_null() || endpoint == 0 {
                    continue;
                }
                let address = unsafe { (*usb).async_interrupt_transfer };
                if address != 0 {
                    let cancel: AsyncInterruptTransfer = unsafe { core::mem::transmute(address) };
                    let _ = unsafe { cancel(usb, endpoint, 0, 0, 0, 0, 0) };
                }
            }
            unsafe {
                core::ptr::write_volatile(&raw mut pointers.usb_mouse_async, 0);
            }
            asynchronous_usb_buttons(0);
        }
        return;
    }
    unsafe {
        USB_MOUSE_ASYNC_IDLE = 0;
    }
    let length = unsafe { core::ptr::read_volatile(&raw const pointers.usb_mouse_length) }
        .min(pointers.usb_mouse_report.len() as u32) as usize;
    let mut report = [0u8; 8];
    for (index, byte) in report.iter_mut().enumerate().take(length) {
        *byte = unsafe {
            core::ptr::read_volatile(
                (&raw const pointers.usb_mouse_report)
                    .cast::<u8>()
                    .add(index),
            )
        };
    }
    unsafe {
        USB_MOUSE_SEQUENCE = sequence;
    }
    if length >= 3 {
        let firmware_motion = unsafe {
            (USE_ABSOLUTE_MOVEMENT && ABSOLUTE_UPDATED_THIS_POLL) || RELATIVE_UPDATED_THIS_POLL
        };
        if let Some(mut event) = pointer::decode_usb_boot_mouse(&report[..length]) {
            event.buttons = asynchronous_usb_buttons(event.buttons);
            if firmware_motion {
                event.delta_x = 0;
                event.delta_y = 0;
            }
            dispatch_pointer(event);
        }
    }
}

// ------------------------=
// FUNC: poll_absolute
// DESC: Implements the poll absolute operation.
// ------------------=
fn poll_absolute(absolute_pointer: *mut AbsolutePointer) {
    if absolute_pointer.is_null() {
        return;
    }
    pump_event(unsafe { (*absolute_pointer).wait_for_input });
    let mut state = AbsolutePointerState {
        current_x: 0,
        current_y: 0,
        current_z: 0,
        active_buttons: 0,
    };
    if unsafe { ((*absolute_pointer).get_state)(absolute_pointer, &mut state) } == 0 {
        let mode = unsafe { (*absolute_pointer).mode.as_ref() };
        if let Some(mode) = mode {
            unsafe {
                ABSOLUTE_UPDATED_THIS_POLL = true;
            }
            let source_buttons = if state.active_buttons != 0 {
                pointer::BUTTON_LEFT
            } else {
                0
            };
            dispatch_pointer_absolute(AbsolutePointerEvent {
                x: normalize_absolute(state.current_x, mode.min_x, mode.max_x),
                y: normalize_absolute(state.current_y, mode.min_y, mode.max_y),
                // UEFI Absolute Pointer defines touch and alternate-active
                // bits rather than USB button numbers. Either active contact
                // is the primary activation in InfinityOS.
                buttons: firmware_absolute_buttons(source_buttons),
                wheel_x: 0,
                wheel_y: 0,
            });
        }
    }
}

// ------------------------=
// FUNC: poll_relative
// DESC: Implements the poll relative operation.
// ------------------=
fn poll_relative(pointer: *mut SimplePointer) {
    if pointer.is_null() {
        return;
    }
    pump_event(unsafe { (*pointer).wait_for_input });
    let mut state = PointerState {
        relative_x: 0,
        relative_y: 0,
        relative_z: 0,
        left_button: 0,
        right_button: 0,
    };
    if unsafe { ((*pointer).get_state)(pointer, &mut state) } == 0 {
        unsafe {
            RELATIVE_UPDATED_THIS_POLL = true;
        }
        let source_buttons = (state.left_button != 0) as u8 * pointer::BUTTON_LEFT
            | (state.right_button != 0) as u8 * pointer::BUTTON_RIGHT;
        dispatch_pointer(PointerEvent {
            delta_x: pointer_delta(state.relative_x),
            delta_y: -pointer_delta(state.relative_y),
            buttons: firmware_relative_buttons(source_buttons),
            wheel_x: 0,
            wheel_y: pointer_delta(state.relative_z).clamp(i8::MIN as i16, i8::MAX as i16) as i8,
        });
    }
}

// ------------------------=
// FUNC: poll_usb_mouse
// DESC: Implements the poll usb mouse operation.
// ------------------=
fn poll_usb_mouse(index: usize, usb: *mut UsbIo, endpoint: u8, absolute: bool) {
    if usb.is_null() || endpoint == 0 {
        return;
    }
    let address = unsafe { (*usb).sync_interrupt_transfer };
    if address == 0 {
        return;
    }
    let transfer: SyncInterruptTransfer = unsafe { core::mem::transmute(address) };
    let mut total_x = 0i32;
    let mut total_y = 0i32;
    let mut total_wheel_x = 0i16;
    let mut total_wheel_y = 0i16;
    let mut received = false;
    let mut buttons = unsafe { USB_MOUSE_BUTTONS[index] };
    let mut latest_absolute = None;
    let mut absolute_batch =
        pointer::AbsolutePointerBatch::new(synchronous_usb_buttons(index, buttons));
    let mut absolute_changed = false;

    // Relative reports are accumulated in a short burst. Absolute devices are
    // allowed a larger drain budget because only their newest coordinate is
    // useful; presenting each queued tablet report separately made the cursor
    // replay old positions behind the host pointer on the animated boot menu.
    let report_budget = if absolute { 16 } else { 4 };
    for _ in 0..report_budget {
        let mut report = [0u8; 8];
        let mut length = report.len();
        let mut usb_status = 0u32;
        if unsafe {
            transfer(
                usb,
                endpoint,
                report.as_mut_ptr(),
                &mut length,
                1,
                &mut usb_status,
            )
        } != 0
            || length < 3
        {
            break;
        }
        received = true;
        if absolute {
            if length < 8 {
                continue;
            }
            // VirtualBox's protocol-zero USB tablet report is an eight-byte
            // packed record: buttons, vertical wheel, horizontal wheel,
            // padding, X(u16 LE), Y(u16 LE). X and Y have a declared logical
            // range of 0..0x7fff. Reading bytes 1..4 as axes mixed wheel and
            // padding into the coordinates, pinning the cursor near an edge.
            let Some(mut event) = pointer::decode_usb_absolute_pointer(&report[..length]) else {
                continue;
            };
            let x = u16::from_le_bytes([report[4], report[5]]);
            let y = u16::from_le_bytes([report[6], report[7]]);
            absolute_changed |= x != unsafe { USB_MOUSE_ABSOLUTE_X[index] }
                || y != unsafe { USB_MOUSE_ABSOLUTE_Y[index] }
                || event.buttons != buttons
                || event.wheel_x != 0
                || event.wheel_y != 0;
            unsafe {
                USB_MOUSE_ABSOLUTE_X[index] = x;
                USB_MOUSE_ABSOLUTE_Y[index] = y;
                USB_MOUSE_BUTTONS[index] = event.buttons;
            }
            buttons = event.buttons;
            event.buttons = synchronous_usb_buttons(index, event.buttons);
            for edge in absolute_batch.push(event).into_iter().flatten() {
                // Button coordinates belong to the edge, not the latest
                // firmware motion. Losing a down/up pair here drops clicks
                // and title-bar drag capture even when the cursor tracks.
                dispatch_pointer_absolute(edge);
            }
            continue;
        }
        let Some(mut event) = pointer::decode_usb_boot_mouse(&report[..length]) else {
            continue;
        };
        total_x += event.delta_x as i32;
        total_y += event.delta_y as i32;
        total_wheel_x += event.wheel_x as i16;
        total_wheel_y += event.wheel_y as i16;
        if event.buttons != buttons {
            let firmware_motion = unsafe {
                (USE_ABSOLUTE_MOVEMENT && ABSOLUTE_UPDATED_THIS_POLL) || RELATIVE_UPDATED_THIS_POLL
            };
            event.delta_x = if firmware_motion {
                0
            } else {
                total_x.clamp(i16::MIN as i32, i16::MAX as i32) as i16
            };
            event.delta_y = if firmware_motion {
                0
            } else {
                total_y.clamp(i16::MIN as i32, i16::MAX as i32) as i16
            };
            event.wheel_x = total_wheel_x.clamp(i8::MIN as i16, i8::MAX as i16) as i8;
            event.wheel_y = total_wheel_y.clamp(i8::MIN as i16, i8::MAX as i16) as i8;
            let source_buttons = event.buttons;
            event.buttons = synchronous_usb_buttons(index, source_buttons);
            dispatch_pointer(event);
            total_x = 0;
            total_y = 0;
            total_wheel_x = 0;
            total_wheel_y = 0;
            buttons = source_buttons;
            unsafe {
                USB_MOUSE_BUTTONS[index] = buttons;
            }
        }
    }
    if received {
        latest_absolute = absolute_batch.finish();
        if let Some(event) = latest_absolute {
            if absolute_changed {
                // VirtualBox exposes one physical tablet through both the
                // UEFI Absolute Pointer protocol and raw USB HID. When the
                // firmware path already supplied this poll's position, retain
                // raw-HID button/wheel edges but never replay its older axis
                // sample over the newer coordinate.
                if unsafe { USE_ABSOLUTE_MOVEMENT && ABSOLUTE_UPDATED_THIS_POLL } {
                    dispatch_pointer(PointerEvent {
                        delta_x: 0,
                        delta_y: 0,
                        buttons: event.buttons,
                        wheel_x: event.wheel_x,
                        wheel_y: event.wheel_y,
                    });
                } else {
                    dispatch_pointer_absolute(event);
                }
            }
        } else if !absolute
            && !unsafe {
                (USE_ABSOLUTE_MOVEMENT && ABSOLUTE_UPDATED_THIS_POLL) || RELATIVE_UPDATED_THIS_POLL
            }
            && (total_x != 0 || total_y != 0 || total_wheel_x != 0 || total_wheel_y != 0)
        {
            dispatch_pointer(PointerEvent {
                delta_x: total_x.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                delta_y: total_y.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                buttons: synchronous_usb_buttons(index, buttons),
                wheel_x: total_wheel_x.clamp(i8::MIN as i16, i8::MAX as i16) as i8,
                wheel_y: total_wheel_y.clamp(i8::MIN as i16, i8::MAX as i16) as i8,
            });
        }
    }
}

// ------------------------=
// FUNC: poll_usb_keyboard
// DESC: Implements the poll usb keyboard operation.
// ------------------=
fn poll_usb_keyboard(usb: *mut UsbIo, endpoint: u8) {
    if usb.is_null() || endpoint == 0 {
        return;
    }
    let address = unsafe { (*usb).sync_interrupt_transfer };
    if address == 0 {
        return;
    }
    let transfer: SyncInterruptTransfer = unsafe { core::mem::transmute(address) };
    let mut report = [0u8; 8];
    let mut length = report.len();
    let mut usb_status = 0u32;
    if unsafe {
        transfer(
            usb,
            endpoint,
            report.as_mut_ptr(),
            &mut length,
            1,
            &mut usb_status,
        )
    } != 0
        || length < 8
    {
        return;
    }
    let current = [
        report[2], report[3], report[4], report[5], report[6], report[7],
    ];
    let previous = unsafe { USB_KEYS };
    let modifiers = (if report[0] & 0x22 != 0 { 1 } else { 0 })
        | (if report[0] & 0x11 != 0 { 2 } else { 0 });
    for &usage in &current {
        if usage != 0 && !previous.contains(&usage) {
            dispatch(InputEvent {
                source: InputSource::Keyboard,
                action: InputAction::Pressed,
                code: usage as u16,
                modifiers,
                delta_x: 0,
                delta_y: 0,
            });
        }
    }
    for &usage in &previous {
        if usage != 0 && !current.contains(&usage) {
            dispatch(InputEvent {
                source: InputSource::Keyboard,
                action: InputAction::Released,
                code: usage as u16,
                modifiers,
                delta_x: 0,
                delta_y: 0,
            });
        }
    }
    unsafe {
        USB_KEYS = current;
    }
}

// ------------------------=
// FUNC: pump_event
// DESC: Implements the pump event operation.
// ------------------=
fn pump_event(event: usize) {
    if event == 0 {
        return;
    }
    let pointers = unsafe { POINTERS };
    let Some(pointers) = (unsafe { pointers.as_ref() }) else {
        return;
    };
    if pointers.check_event == 0 {
        return;
    }
    let check: CheckEvent = unsafe { core::mem::transmute(pointers.check_event) };
    let _ = unsafe { check(event) };
}

// ------------------------=
// FUNC: discovered_pointer_capabilities
// DESC: Classifies every pointer transport exposed during initial firmware hardware discovery.
// ------------------=
fn discovered_pointer_capabilities() -> PointerCapabilities {
    let mut capabilities = PointerCapabilities::NONE;
    if let Some(pointers) = unsafe { POINTERS.as_ref() } {
        if pointers.relative_count != 0 {
            capabilities.transports |= pointer::TRANSPORT_UEFI_RELATIVE;
            capabilities.features |= pointer::FEATURE_RELATIVE
                | pointer::FEATURE_BUTTONS_STANDARD
                | pointer::FEATURE_WHEEL_VERTICAL;
            capabilities.interface_count = capabilities
                .interface_count
                .saturating_add(pointers.relative_count.min(MAX_POINTER_PROTOCOLS as u32) as u8);
        }
        if pointers.absolute_count != 0 {
            capabilities.transports |= pointer::TRANSPORT_UEFI_ABSOLUTE;
            capabilities.features |= pointer::FEATURE_ABSOLUTE | pointer::FEATURE_BUTTONS_STANDARD;
            capabilities.interface_count = capabilities
                .interface_count
                .saturating_add(pointers.absolute_count.min(MAX_POINTER_PROTOCOLS as u32) as u8);
        }
        for index in 0..(pointers.usb_mouse_count as usize).min(MAX_POINTER_PROTOCOLS) {
            if pointers.usb_mouse_absolute[index] != 0 {
                capabilities.transports |= pointer::TRANSPORT_USB_HID_ABSOLUTE;
                capabilities.features |= pointer::FEATURE_ABSOLUTE
                    | pointer::FEATURE_BUTTONS_STANDARD
                    | pointer::FEATURE_BUTTONS_EXTENDED
                    | pointer::FEATURE_WHEEL_VERTICAL
                    | pointer::FEATURE_WHEEL_HORIZONTAL;
            } else {
                capabilities.transports |= pointer::TRANSPORT_USB_HID_RELATIVE;
                capabilities.features |= pointer::FEATURE_RELATIVE
                    | pointer::FEATURE_BUTTONS_STANDARD
                    | pointer::FEATURE_BUTTONS_EXTENDED
                    | pointer::FEATURE_WHEEL_VERTICAL;
            }
            capabilities.interface_count = capabilities.interface_count.saturating_add(1);
        }
    } else if !unsafe { ABSOLUTE_POINTER }.is_null() {
        capabilities.transports = pointer::TRANSPORT_UEFI_ABSOLUTE;
        capabilities.features = pointer::FEATURE_ABSOLUTE | pointer::FEATURE_BUTTONS_STANDARD;
        capabilities.interface_count = 1;
    } else if !unsafe { POINTER }.is_null() {
        capabilities.transports = pointer::TRANSPORT_UEFI_RELATIVE;
        capabilities.features = pointer::FEATURE_RELATIVE
            | pointer::FEATURE_BUTTONS_STANDARD
            | pointer::FEATURE_WHEEL_VERTICAL;
        capabilities.interface_count = 1;
    }
    capabilities
}

// ------------------------=
// FUNC: normalize_absolute
// DESC: Implements the normalize absolute operation.
// ------------------=
fn normalize_absolute(value: u64, minimum: u64, maximum: u64) -> i32 {
    if maximum <= minimum {
        return 0;
    }
    (((value.saturating_sub(minimum)) as u128 * 1000) / (maximum - minimum) as u128) as i32
}

// ------------------------=
// FUNC: pointer_delta
// DESC: Handles pointer delta input or state transitions.
// ------------------=
fn pointer_delta(value: i32) -> i16 {
    value.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}
