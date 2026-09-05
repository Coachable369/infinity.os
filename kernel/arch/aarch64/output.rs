use core::arch::asm;
use core::ptr::{read_volatile, write_volatile};

const PL011_DATA: *mut u32 = 0x0900_0000 as *mut u32;
const PL011_FLAGS: *const u32 = 0x0900_0018 as *const u32;
const TX_FIFO_FULL: u32 = 1 << 5;
static mut SERIAL_ENABLED: bool = false;

// ------------------------=
// FUNC: initialize
// DESC: Initializes initialize state.
// ------------------=
pub unsafe fn initialize(serial_enabled: bool) {
    // EDK/QEMU leaves PL011 configured. VMware is framebuffer-only.
    SERIAL_ENABLED = serial_enabled;
}

// ------------------------=
// FUNC: write
// DESC: Implements the write operation.
// ------------------=
pub unsafe fn write(bytes: &[u8]) {
    if !SERIAL_ENABLED {
        return;
    }
    for &byte in bytes {
        while read_volatile(PL011_FLAGS) & TX_FIFO_FULL != 0 {}
        write_volatile(PL011_DATA, byte as u32);
    }
}

// ------------------------=
// FUNC: quiesce
// DESC: Masks asynchronous exceptions before the processor enters an unrecoverable stop state.
// ------------------=
pub fn quiesce() {
    unsafe {
        asm!("msr daifset, #0xf", options(nomem, nostack));
    }
}

// ------------------------=
// FUNC: idle
// DESC: Implements the idle operation.
// ------------------=
pub fn idle() -> ! {
    loop {
        unsafe {
            asm!("wfe", options(nomem, nostack));
        }
    }
}
