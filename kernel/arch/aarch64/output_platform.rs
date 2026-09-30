pub const SERIAL_ENABLED: u64 = 1;
pub const VIRTUALBOX_UART: u64 = 1 << 6;
pub const QEMU_PL011_BASE: usize = 0x0900_0000;
pub const VIRTUALBOX_PL011_BASE: usize = 0xffdd_f000;

// ------------------------=
// FUNC: serial_base
// DESC: Selects the self-contained AArch64 UART from loader-observed platform metadata.
// ------------------=
pub const fn serial_base(boot_flags: u64) -> Option<usize> {
    if boot_flags & SERIAL_ENABLED == 0 {
        None
    } else if boot_flags & VIRTUALBOX_UART != 0 {
        Some(VIRTUALBOX_PL011_BASE)
    } else {
        Some(QEMU_PL011_BASE)
    }
}
