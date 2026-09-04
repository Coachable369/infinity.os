# x86 bootstrap

The 32-bit path deliberately differs from UEFI targets. `boot_sector.asm` uses
BIOS floppy geometry to load a bounded second stage; `bootstrap.asm` establishes
A20 and protected mode, loads the embedded ELF32 kernel, constructs `BootInfo`,
and transfers ownership to Rust.
