#![allow(dead_code)]

#[path = "../kernel/drivers/input/pointer.rs"]
mod pointer;

use pointer::{
    decode_ps2_packet, decode_usb_absolute_pointer, decode_usb_boot_mouse, PointerCapabilities,
    BUTTON_BACK, BUTTON_FORWARD, BUTTON_LEFT, BUTTON_MIDDLE, BUTTON_RIGHT,
    FEATURE_ABSOLUTE, FEATURE_BUTTONS_EXTENDED, FEATURE_RELATIVE, TRANSPORT_PS2,
    TRANSPORT_USB_HID_ABSOLUTE, TRANSPORT_USB_HID_RELATIVE,
};

// ------------------------=
// FUNC: ps2_protocols
// DESC: Verifies standard, wheel, and five-button PS/2 packet decoding.
// ------------------=
fn ps2_protocols() {
    let standard = decode_ps2_packet(&[0x0b, 5, 0xfd], 0).expect("standard PS/2 packet");
    assert_eq!(standard.delta_x, 5);
    assert_eq!(standard.delta_y, 3);
    assert_eq!(standard.buttons, BUTTON_LEFT | BUTTON_RIGHT);

    let wheel = decode_ps2_packet(&[0x0c, 0xff, 2, 0x01], 3).expect("wheel packet");
    assert_eq!(wheel.delta_x, -1);
    assert_eq!(wheel.delta_y, -2);
    assert_eq!(wheel.buttons, BUTTON_MIDDLE);
    assert_eq!(wheel.wheel_y, -1);

    let explorer = decode_ps2_packet(&[0x08, 0, 0, 0x3f], 4).expect("Explorer packet");
    assert_eq!(explorer.buttons, BUTTON_BACK | BUTTON_FORWARD);
    assert_eq!(explorer.wheel_y, 1);

    assert!(decode_ps2_packet(&[0x48, 1, 1], 0).is_none());
}

// ------------------------=
// FUNC: usb_hid_protocols
// DESC: Verifies common relative USB mouse and absolute virtual-tablet report decoding.
// ------------------=
fn usb_hid_protocols() {
    let relative = decode_usb_boot_mouse(&[0x15, 0xf8, 7, 0xff, 2]).expect("USB mouse");
    assert_eq!(relative.delta_x, -8);
    assert_eq!(relative.delta_y, 7);
    assert_eq!(relative.buttons, BUTTON_LEFT | BUTTON_MIDDLE | BUTTON_FORWARD);
    assert_eq!(relative.wheel_y, 1);
    assert_eq!(relative.wheel_x, 2);

    let absolute = decode_usb_absolute_pointer(&[0x03, 0xff, 1, 0, 0xff, 0x7f, 0, 0x40])
        .expect("USB tablet");
    assert_eq!(absolute.x, 1000);
    assert!((499..=501).contains(&absolute.y));
    assert_eq!(absolute.buttons, BUTTON_LEFT | BUTTON_RIGHT);
    assert_eq!(absolute.wheel_y, 1);
    assert_eq!(absolute.wheel_x, 1);
}

// ------------------------=
// FUNC: discovery_metadata
// DESC: Verifies transport capability names remain deterministic for the device registry.
// ------------------=
fn discovery_metadata() {
    let ps2 = PointerCapabilities {
        transports: TRANSPORT_PS2,
        features: FEATURE_RELATIVE | FEATURE_BUTTONS_EXTENDED,
        interface_count: 1,
    };
    assert!(ps2.available());
    assert_eq!(ps2.driver_name(), "ps2-explorer-mouse");

    let tablet = PointerCapabilities {
        transports: TRANSPORT_USB_HID_ABSOLUTE,
        features: FEATURE_ABSOLUTE,
        interface_count: 1,
    };
    assert_eq!(tablet.driver_name(), "usb-hid-absolute-pointer");

    let composite = PointerCapabilities {
        transports: TRANSPORT_USB_HID_RELATIVE | TRANSPORT_USB_HID_ABSOLUTE,
        features: FEATURE_RELATIVE | FEATURE_ABSOLUTE,
        interface_count: 2,
    };
    assert_eq!(composite.driver_name(), "composite-pointer");
}

// ------------------------=
// FUNC: main
// DESC: Runs the host-side pointer protocol and discovery regression suite.
// ------------------=
fn main() {
    ps2_protocols();
    usb_hid_protocols();
    discovery_metadata();
    println!("PASS pointer protocols: PS/2, wheel, five-button, USB HID relative, USB HID absolute");
}
