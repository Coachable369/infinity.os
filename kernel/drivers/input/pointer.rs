pub const BUTTON_LEFT: u8 = 1 << 0;
pub const BUTTON_RIGHT: u8 = 1 << 1;
pub const BUTTON_MIDDLE: u8 = 1 << 2;
pub const BUTTON_BACK: u8 = 1 << 3;
pub const BUTTON_FORWARD: u8 = 1 << 4;

// ------------------------=
// FUNC: absolute_pointer_state_changed
// DESC: Accepts only observable absolute motion or a real button transition as actionable pointer input.
// ------------------=
pub const fn absolute_pointer_state_changed(
    previous_x: i32,
    previous_y: i32,
    previous_pressed: bool,
    next_x: i32,
    next_y: i32,
    next_pressed: bool,
) -> bool {
    previous_x != next_x || previous_y != next_y || previous_pressed != next_pressed
}

pub const TRANSPORT_PS2: u32 = 1 << 0;
pub const TRANSPORT_UEFI_RELATIVE: u32 = 1 << 1;
pub const TRANSPORT_UEFI_ABSOLUTE: u32 = 1 << 2;
pub const TRANSPORT_USB_HID_RELATIVE: u32 = 1 << 3;
pub const TRANSPORT_USB_HID_ABSOLUTE: u32 = 1 << 4;

pub const FEATURE_RELATIVE: u32 = 1 << 0;
pub const FEATURE_ABSOLUTE: u32 = 1 << 1;
pub const FEATURE_WHEEL_VERTICAL: u32 = 1 << 2;
pub const FEATURE_WHEEL_HORIZONTAL: u32 = 1 << 3;
pub const FEATURE_BUTTONS_STANDARD: u32 = 1 << 4;
pub const FEATURE_BUTTONS_EXTENDED: u32 = 1 << 5;

const MAX_BUTTON_SOURCES: usize = 8;

/// Preserves button ownership while one physical pointer is exposed through
/// multiple firmware and raw-HID transports. Motion reports from one source
/// must not release a button that remains held on another source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PointerButtonArbiter {
    firmware_absolute: u8,
    absolute_mirrored_by_usb: bool,
    firmware_relative: u8,
    asynchronous_usb: u8,
    usb: [u8; MAX_BUTTON_SOURCES],
}

impl PointerButtonArbiter {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty multi-transport pointer button state.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            firmware_absolute: 0,
            absolute_mirrored_by_usb: false,
            firmware_relative: 0,
            asynchronous_usb: 0,
            usb: [0; MAX_BUTTON_SOURCES],
        }
    }

    // ------------------------=
    // FUNC: set_firmware_absolute
    // DESC: Records the latest explicit button state from the firmware absolute pointer.
    // ------------------=
    pub fn set_firmware_absolute(&mut self, buttons: u8) {
        if !self.absolute_mirrored_by_usb {
            self.firmware_absolute = buttons;
        }
    }

    // ------------------------=
    // FUNC: set_usb_absolute
    // DESC: Gives a reporting raw tablet ownership of its firmware mirror, preventing a missed mirrored release from latching a button.
    // ------------------=
    pub fn set_usb_absolute(&mut self, source: usize, buttons: u8) {
        if source >= MAX_BUTTON_SOURCES {
            return;
        }
        self.absolute_mirrored_by_usb = true;
        self.firmware_absolute = 0;
        self.set_usb(source, buttons);
    }

    // ------------------------=
    // FUNC: set_firmware_relative
    // DESC: Records the latest explicit button state from the firmware relative pointer.
    // ------------------=
    pub fn set_firmware_relative(&mut self, buttons: u8) {
        self.firmware_relative = buttons;
    }

    // ------------------------=
    // FUNC: set_asynchronous_usb
    // DESC: Records the latest explicit button state from the asynchronous USB HID path.
    // ------------------=
    pub fn set_asynchronous_usb(&mut self, buttons: u8) {
        self.asynchronous_usb = buttons;
    }

    // ------------------------=
    // FUNC: set_usb
    // DESC: Records the latest explicit button state for one synchronously polled USB interface.
    // ------------------=
    pub fn set_usb(&mut self, source: usize, buttons: u8) {
        if let Some(state) = self.usb.get_mut(source) {
            *state = buttons;
        }
    }

    // ------------------------=
    // FUNC: combined
    // DESC: Returns buttons held by any active transport without fabricating release edges.
    // ------------------=
    pub fn combined(&self) -> u8 {
        self.usb.iter().fold(
            self.firmware_absolute | self.firmware_relative | self.asynchronous_usb,
            |buttons, source| buttons | *source,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PointerEvent {
    pub delta_x: i16,
    pub delta_y: i16,
    pub buttons: u8,
    pub wheel_x: i8,
    pub wheel_y: i8,
}

impl PointerEvent {
    // ------------------------=
    // FUNC: left_button
    // DESC: Reports whether the primary pointer button is currently pressed.
    // ------------------=
    pub const fn left_button(self) -> bool {
        self.buttons & BUTTON_LEFT != 0
    }

    // ------------------------=
    // FUNC: right_button
    // DESC: Reports whether the secondary pointer button is currently pressed.
    // ------------------=
    pub const fn right_button(self) -> bool {
        self.buttons & BUTTON_RIGHT != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AbsolutePointerEvent {
    pub x: i32,
    pub y: i32,
    pub buttons: u8,
    pub wheel_x: i8,
    pub wheel_y: i8,
}

pub struct AbsolutePointerBatch {
    buttons: u8,
    pending: Option<AbsolutePointerEvent>,
}
impl AbsolutePointerBatch {
    // ------------------------=
    // FUNC: new
    // DESC: Starts a tablet burst using the last dispatched button state.
    // ------------------=
    pub const fn new(buttons: u8) -> Self {
        Self {
            buttons,
            pending: None,
        }
    }
    // ------------------------=
    // FUNC: push
    // DESC: Coalesces motion only; preserves every button edge and wheel report immediately.
    // ------------------=
    pub fn push(&mut self, event: AbsolutePointerEvent) -> [Option<AbsolutePointerEvent>; 2] {
        if event.buttons != self.buttons || event.wheel_x != 0 || event.wheel_y != 0 {
            self.buttons = event.buttons;
            [self.pending.take(), Some(event)]
        } else {
            self.pending = Some(event);
            [None, None]
        }
    }
    // ------------------------=
    // FUNC: finish
    // DESC: Returns the newest motion after the final preserved edge.
    // ------------------=
    pub fn finish(&mut self) -> Option<AbsolutePointerEvent> {
        self.pending.take()
    }
}

impl AbsolutePointerEvent {
    // ------------------------=
    // FUNC: left_button
    // DESC: Reports whether the primary button on an absolute pointer is pressed.
    // ------------------=
    pub const fn left_button(self) -> bool {
        self.buttons & BUTTON_LEFT != 0
    }

    // ------------------------=
    // FUNC: right_button
    // DESC: Reports whether the secondary button on an absolute pointer is pressed.
    // ------------------=
    pub const fn right_button(self) -> bool {
        self.buttons & BUTTON_RIGHT != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PointerCapabilities {
    pub transports: u32,
    pub features: u32,
    pub interface_count: u8,
}

impl PointerCapabilities {
    pub const NONE: Self = Self {
        transports: 0,
        features: 0,
        interface_count: 0,
    };

    // ------------------------=
    // FUNC: available
    // DESC: Reports whether initial hardware discovery found a usable pointer interface.
    // ------------------=
    pub const fn available(self) -> bool {
        self.transports != 0 && self.interface_count != 0
    }

    // ------------------------=
    // FUNC: driver_name
    // DESC: Selects a stable registry name for the discovered pointer transport set.
    // ------------------=
    pub const fn driver_name(self) -> &'static str {
        if self.transports.count_ones() > 1 {
            "composite-pointer"
        } else if self.transports & TRANSPORT_USB_HID_ABSOLUTE != 0 {
            "usb-hid-absolute-pointer"
        } else if self.transports & TRANSPORT_USB_HID_RELATIVE != 0 {
            "usb-hid-mouse"
        } else if self.transports & TRANSPORT_UEFI_ABSOLUTE != 0 {
            "uefi-absolute-pointer"
        } else if self.transports & TRANSPORT_UEFI_RELATIVE != 0 {
            "uefi-relative-pointer"
        } else if self.transports & TRANSPORT_PS2 != 0 {
            if self.features & FEATURE_BUTTONS_EXTENDED != 0 {
                "ps2-explorer-mouse"
            } else if self.features & FEATURE_WHEEL_VERTICAL != 0 {
                "ps2-wheel-mouse"
            } else {
                "ps2-mouse"
            }
        } else {
            "pointer-unavailable"
        }
    }
}

// ------------------------=
// FUNC: decode_ps2_packet
// DESC: Decodes standard, IntelliMouse wheel, and Explorer five-button PS/2 packets.
// ------------------=
pub fn decode_ps2_packet(packet: &[u8], device_id: u8) -> Option<PointerEvent> {
    let required = if device_id == 3 || device_id == 4 {
        4
    } else {
        3
    };
    if packet.len() < required || packet[0] & 0x08 == 0 || packet[0] & 0xc0 != 0 {
        return None;
    }
    let mut buttons = packet[0] & 0x07;
    let mut wheel_y = 0;
    if required == 4 {
        wheel_y = -sign_extend_nibble(packet[3]);
        if device_id == 4 {
            if packet[3] & 0x10 != 0 {
                buttons |= BUTTON_BACK;
            }
            if packet[3] & 0x20 != 0 {
                buttons |= BUTTON_FORWARD;
            }
        }
    }
    Some(PointerEvent {
        delta_x: packet[1] as i8 as i16,
        delta_y: -(packet[2] as i8 as i16),
        buttons,
        wheel_x: 0,
        wheel_y,
    })
}

// ------------------------=
// FUNC: decode_usb_boot_mouse
// DESC: Decodes the standard USB HID boot-mouse prefix with optional wheel and extra buttons.
// ------------------=
pub fn decode_usb_boot_mouse(report: &[u8]) -> Option<PointerEvent> {
    if report.len() < 3 {
        return None;
    }
    Some(PointerEvent {
        delta_x: report[1] as i8 as i16,
        delta_y: report[2] as i8 as i16,
        buttons: report[0] & 0x1f,
        wheel_x: report.get(4).copied().unwrap_or(0) as i8,
        wheel_y: -(report.get(3).copied().unwrap_or(0) as i8),
    })
}

// ------------------------=
// FUNC: decode_usb_absolute_pointer
// DESC: Decodes the standards-compatible 8-byte absolute HID report used by VM tablets.
// ------------------=
pub fn decode_usb_absolute_pointer(report: &[u8]) -> Option<AbsolutePointerEvent> {
    if report.len() < 8 {
        return None;
    }
    let raw_x = u16::from_le_bytes([report[4], report[5]]);
    let raw_y = u16::from_le_bytes([report[6], report[7]]);
    Some(AbsolutePointerEvent {
        x: normalize_usb_absolute_axis(raw_x),
        y: normalize_usb_absolute_axis(raw_y),
        buttons: report[0] & 0x1f,
        wheel_x: report[2] as i8,
        wheel_y: -(report[1] as i8),
    })
}

// ------------------------=
// FUNC: normalize_usb_absolute_axis
// DESC: Converts the HID logical range used by common virtual USB tablets into screen space.
// ------------------=
pub fn normalize_usb_absolute_axis(value: u16) -> i32 {
    ((value.min(0x7fff) as u32 * 1000) / 0x7fffu32) as i32
}

// ------------------------=
// FUNC: sign_extend_nibble
// DESC: Converts a four-bit two's-complement wheel value into a signed byte.
// ------------------=
fn sign_extend_nibble(value: u8) -> i8 {
    ((value & 0x0f) as i8) << 4 >> 4
}
