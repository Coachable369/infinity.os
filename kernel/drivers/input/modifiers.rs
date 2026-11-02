//! Current physical and validated firmware modifier state for native mouse chords.
use core::sync::atomic::{AtomicU8, Ordering};

pub const USB_SOURCES: usize = 8;
static USB: [AtomicU8; USB_SOURCES] = [const { AtomicU8::new(0) }; USB_SOURCES];
static PS2: AtomicU8 = AtomicU8::new(0);
static FIRMWARE: AtomicU8 = AtomicU8::new(0);

// ------------------------=
// FUNC: firmware_report
// DESC: Replaces firmware state from each validated current-state read, including releases and unavailable state.
// ------------------=
pub fn firmware_report(modifiers: u8) { FIRMWARE.store(modifiers,Ordering::Release); }

// ------------------------=
// FUNC: usb_report
// DESC: Records every boot report, including modifier-only releases and simultaneous left/right keys.
// ------------------=
pub fn usb_report(source: usize, bits: u8) {
    if let Some(state)=USB.get(source) {state.store(bits, Ordering::Release);}
}

// ------------------------=
// FUNC: usb_reset
// DESC: Clears only a removed or reset keyboard, retaining modifiers held on other devices.
// ------------------=
pub fn usb_reset(source: usize) { usb_report(source,0); }

// ------------------------=
// FUNC: usb_fault
// DESC: Distinguishes idle interrupt timeouts from transport faults that invalidate held keys.
// ------------------=
pub fn usb_fault(status: u64, usb_status: u32) -> bool {
    let code=status & 0x7fff_ffff_ffff_ffff;
    if matches!(code,6|18) && usb_status & !0x40 == 0 {return false;}
    status!=0 || usb_status!=0
}

// ------------------------=
// FUNC: reset
// DESC: Resets physical source state when native input is initialized again.
// ------------------=
pub fn reset() {
    for state in &USB {state.store(0,Ordering::Release);}
    PS2.store(0,Ordering::Release);
    FIRMWARE.store(0,Ordering::Release);
}

// ------------------------=
// FUNC: ps2_transition
// DESC: Updates independent physical modifier bits from a PS/2 make or break byte.
// ------------------=
pub fn ps2_transition(value: u8, extended: bool) {
    let bit = match (value & 0x7f, extended) {
        (0x1d, false) => 0x01, (0x1d, true) => 0x10,
        (0x2a, false) => 0x02, (0x36, false) => 0x20,
        (0x5b, true) => 0x08, (0x5c, true) => 0x80,
        (0x38, false) => 0x04, (0x38, true) => 0x40,
        _ => return,
    };
    if value & 0x80 == 0 { PS2.fetch_or(bit, Ordering::AcqRel); }
    else { PS2.fetch_and(!bit, Ordering::AcqRel); }
}

// ------------------------=
// FUNC: held
// DESC: Returns the native Shift/Control/Command mask shared by physical mouse input.
// ------------------=
pub fn held() -> u8 {
    let bits=USB.iter().fold(PS2.load(Ordering::Acquire),|bits,state|bits|state.load(Ordering::Acquire));
    super::desktop_shortcuts::hid_modifiers(bits) | FIRMWARE.load(Ordering::Acquire)
}
