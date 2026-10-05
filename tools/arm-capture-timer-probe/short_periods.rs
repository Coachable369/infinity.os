//! Diagnostic-only fixed short BDL periods; not a replacement production driver.
use core::ptr::{read_volatile, write_volatile};
use crate::hda::ResidentProgress;
pub struct Tracker { bytes: u32, cyclic: u32, position: u32 }
#[repr(C, align(128))]
struct Descriptors([[u64; 2]; 256]);
#[repr(C, align(128))]
struct Silence([i16; 48_000 * 4]);
static mut DESCRIPTORS: Descriptors = Descriptors([[0; 2]; 256]);
static SILENCE: Silence = Silence([0; 48_000 * 4]);

// ------------------------=
// FUNC: output
// DESC: Finds the first output stream from the same HDA capability register as the production driver.
// ------------------=
unsafe fn output(base: usize) -> usize {
    base + 0x80 + ((read_volatile(base as *const u32) >> 8) & 15) as usize * 0x20
}

// ------------------------=
// FUNC: reset
// DESC: Bounds the stream-reset acknowledgement without sleeps or firmware services.
// ------------------=
unsafe fn reset(address: usize, value: u8) {
    write_volatile(address as *mut u8, value);
    for _ in 0..100_000 {
        if read_volatile(address as *const u32) & 1 == value as u32 { return; }
    }
    panic!();
}

// ------------------------=
// FUNC: start
// DESC: Splits immutable PCM and a two-second silent tail into aligned <=20ms IOC periods for A/B diagnosis.
// ------------------=
pub unsafe fn start(base: usize, samples: &'static [i16], rate: u32) -> Tracker {
    let bytes = samples.len() as u32 * 2;
    let silent_bytes = rate * 4 * 2;
    let period = rate * 4 / 50 / 128 * 128;
    let mut count = 0;
    for (address, total) in [(samples.as_ptr() as u64, bytes), (SILENCE.0.as_ptr() as u64, silent_bytes)] {
        let mut offset = 0;
        while offset < total {
            assert!(count < 256);
            let length = period.min(total - offset);
            DESCRIPTORS.0[count] = [address + offset as u64, length as u64 | (1 << 32)];
            count += 1;
            offset += length;
        }
    }
    let r = output(base);
    write_volatile(r as *mut u8, 0);
    reset(r, 1);
    reset(r, 0);
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    core::arch::asm!("dsb sy", options(nostack));
    let bdl = core::ptr::addr_of!(DESCRIPTORS.0) as u64;
    write_volatile((r + 8) as *mut u32, bytes + silent_bytes);
    write_volatile((r + 0xc) as *mut u16, (count - 1) as u16);
    write_volatile((r + 0x12) as *mut u16, if rate == 48_000 { 0x11 } else { 0x4011 });
    write_volatile((r + 0x18) as *mut u32, bdl as u32);
    write_volatile((r + 0x1c) as *mut u32, (bdl >> 32) as u32);
    write_volatile((r + 3) as *mut u8, 0x1c);
    write_volatile(r as *mut u32, (1 << 20) | 2);
    Tracker { bytes, cyclic: bytes + silent_bytes, position: 0 }
}

// ------------------------=
// FUNC: progress
// DESC: Uses the production silence-drain tracker while acknowledging nonterminal period IOC completions.
// ------------------=
pub unsafe fn progress(base: usize, tracker: &mut Tracker) -> ResidentProgress {
    let r = output(base);
    let position = read_volatile((r + 4) as *const u32);
    let status = (read_volatile(r as *const u32) >> 24) as u8;
    if status & 4 != 0 { write_volatile((r + 3) as *mut u8, 4); }
    assert!(status & 0x18 == 0 && position <= tracker.cyclic && position >= tracker.position);
    tracker.position = position;
    let complete = position >= tracker.bytes + 65_536;
    ResidentProgress { played_bytes: position.min(tracker.bytes), total_bytes: tracker.bytes, complete }
}
