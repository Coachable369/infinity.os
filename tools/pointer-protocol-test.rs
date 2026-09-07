#![allow(dead_code)]

#[path = "../kernel/drivers/input/pointer.rs"]
mod pointer;

use pointer::{
    decode_ps2_packet, decode_usb_absolute_pointer, decode_usb_boot_mouse, PointerButtonArbiter,
    PointerCapabilities, BUTTON_BACK, BUTTON_FORWARD, BUTTON_LEFT, BUTTON_MIDDLE, BUTTON_RIGHT,
    FEATURE_ABSOLUTE, FEATURE_BUTTONS_EXTENDED, FEATURE_RELATIVE, TRANSPORT_PS2,
    TRANSPORT_USB_HID_ABSOLUTE, TRANSPORT_USB_HID_RELATIVE,
};

// ------------------------=
// FUNC: tablet_edges_survive_burst
// DESC: Verifies a drained tablet burst preserves down/up coordinates and wheel events without replaying stale motion.
// ------------------=
fn tablet_edges_survive_burst() {
    use pointer::{AbsolutePointerBatch, AbsolutePointerEvent};
    let mut batch = AbsolutePointerBatch::new(0);
    let mut delivered = Vec::new();
    for (x, buttons, wheel_y) in [
        (1, 0, 0),
        (2, 0, 0),
        (3, 1, 0),
        (4, 1, 0),
        (5, 1, 0),
        (6, 0, 0),
        (7, 0, 1),
        (8, 0, 1),
        (9, 0, 0),
        (10, 0, 0),
    ] {
        for event in batch
            .push(AbsolutePointerEvent {
                x,
                y: 100,
                buttons,
                wheel_x: 0,
                wheel_y,
            })
            .into_iter()
            .flatten()
        {
            delivered.push((event.x, event.buttons, event.wheel_y));
        }
    }
    if let Some(event) = batch.finish() {
        delivered.push((event.x, event.buttons, event.wheel_y));
    }
    assert_eq!(
        delivered,
        vec![
            (2, 0, 0),
            (3, 1, 0),
            (5, 1, 0),
            (6, 0, 0),
            (7, 0, 1),
            (8, 0, 1),
            (10, 0, 0)
        ]
    );
    assert_eq!(batch.finish(), None);
}

// ------------------------=
// FUNC: composite_button_capture
// DESC: Verifies motion-only reports cannot release a drag owned by another pointer transport.
// ------------------=
fn composite_button_capture() {
    let mut buttons = PointerButtonArbiter::new();

    buttons.set_usb(0, BUTTON_LEFT);
    assert_eq!(buttons.combined(), BUTTON_LEFT);

    // VirtualBox commonly sends absolute movement with no button bits while
    // raw USB HID owns the actual press. The move must preserve capture.
    buttons.set_firmware_absolute(0);
    assert_eq!(buttons.combined(), BUTTON_LEFT);

    // An idle interrupt endpoint emits no report at all. Leaving the source
    // untouched must preserve the held state until a real release arrives.
    assert_eq!(buttons.combined(), BUTTON_LEFT);

    buttons.set_usb(0, 0);
    assert_eq!(buttons.combined(), 0);

    // Releasing one of two held sources must not release the composite device.
    buttons.set_firmware_relative(BUTTON_LEFT);
    buttons.set_asynchronous_usb(BUTTON_LEFT);
    buttons.set_firmware_relative(0);
    assert_eq!(buttons.combined(), BUTTON_LEFT);
    buttons.set_asynchronous_usb(0);
    assert_eq!(buttons.combined(), 0);
}

// ------------------------=
// FUNC: absolute_pointer_edges
// DESC: Verifies stationary buttonless firmware notifications cannot fabricate wizard activations.
// ------------------=
fn absolute_pointer_edges() {
    assert!(!pointer::absolute_pointer_state_changed(
        420, 830, false, 420, 830, false
    ));
    assert!(pointer::absolute_pointer_state_changed(
        420, 830, false, 421, 830, false
    ));
    assert!(pointer::absolute_pointer_state_changed(
        420, 830, false, 420, 830, true
    ));
    assert!(pointer::absolute_pointer_state_changed(
        420, 830, true, 420, 830, false
    ));
}

// ------------------------=
// FUNC: ps2_protocols
// DESC: Verifies standard, wheel, and five-button PS/2 packet decoding.
// ------------------=
fn ps2_protocols() {
    let standard = decode_ps2_packet(&[0x0b, 5, 0xfd], 0).expect("standard PS/2 packet");
    assert_eq!(standard.delta_x, 5);
    assert_eq!(standard.delta_y, 3);
    assert_eq!(standard.buttons, BUTTON_LEFT | BUTTON_RIGHT);
    assert!(standard.left_button());
    assert!(standard.right_button());

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
    assert_eq!(
        relative.buttons,
        BUTTON_LEFT | BUTTON_MIDDLE | BUTTON_FORWARD
    );
    assert_eq!(relative.wheel_y, 1);
    assert_eq!(relative.wheel_x, 2);

    let absolute =
        decode_usb_absolute_pointer(&[0x03, 0xff, 1, 0, 0xff, 0x7f, 0, 0x40]).expect("USB tablet");
    assert_eq!(absolute.x, 1000);
    assert!((499..=501).contains(&absolute.y));
    assert_eq!(absolute.buttons, BUTTON_LEFT | BUTTON_RIGHT);
    assert!(absolute.left_button());
    assert!(absolute.right_button());
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
    tablet_edges_survive_burst();
    ps2_protocols();
    usb_hid_protocols();
    discovery_metadata();
    composite_button_capture();
    absolute_pointer_edges();
    println!(
        "PASS pointer protocols: PS/2, wheel, five-button, USB HID relative, USB HID absolute"
    );
}
