use core::arch::asm;

const COM1: u16 = 0x3f8;

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
// FUNC: initialize
// DESC: Initializes initialize state.
// ------------------=
pub unsafe fn initialize(_serial_enabled: bool) {
    outb(COM1 + 1, 0x00);
    outb(COM1 + 3, 0x80);
    outb(COM1, 0x01);
    outb(COM1 + 1, 0x00);
    outb(COM1 + 3, 0x03);
    outb(COM1 + 2, 0xc7);
    outb(COM1 + 4, 0x0b);
}

// ------------------------=
// FUNC: write
// DESC: Implements the write operation.
// ------------------=
pub unsafe fn write(bytes: &[u8]) {
    for &byte in bytes {
        while inb(COM1 + 5) & 0x20 == 0 {}
        outb(COM1, byte);
    }
}

// ------------------------=
// FUNC: quiesce
// DESC: Masks interrupts before the processor enters an unrecoverable stop state.
// ------------------=
pub fn quiesce() {
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
}

// ------------------------=
// FUNC: idle
// DESC: Implements the idle operation.
// ------------------=
pub fn idle() -> ! {
    loop {
        unsafe {
            asm!("hlt", options(nomem, nostack));
        }
    }
}
