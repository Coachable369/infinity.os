use core::arch::asm;
use core::ptr::{read_volatile, write_volatile};

#[path = "output_platform.rs"]
mod output_platform;
#[path = "../output_lock.rs"]
mod output_lock;

const TX_FIFO_FULL: u32 = 1 << 5;
static mut SERIAL_BASE: Option<usize> = None;

// ------------------------=
// FUNC: initialize
// DESC: Initializes initialize state.
// ------------------=
pub unsafe fn initialize(boot_flags: u64) {
    // EDK leaves the selected PL011 configured. VMware remains framebuffer-only.
    SERIAL_BASE = output_platform::serial_base(boot_flags);
}

// ------------------------=
// FUNC: write
// DESC: Implements the write operation.
// ------------------=
pub unsafe fn write(bytes: &[u8]) {
    let Some(base) = SERIAL_BASE else { return; };
    output_lock::serialized(|| unsafe {
        let data = base as *mut u32;
        let flags = (base + 0x18) as *const u32;
        for &byte in bytes {
            while read_volatile(flags) & TX_FIFO_FULL != 0 {}
            write_volatile(data, byte as u32);
        }
    });
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
