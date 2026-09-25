#![no_std]
#![allow(dead_code)]
#[path = "../../kernel/drivers/hda.rs"]
mod hda;
static mut DMA: hda::Dma = hda::Dma::new();
static mut PCM: [i16; hda::SAMPLES] = [0; hda::SAMPLES];
core::arch::global_asm!(".section .text.entry", ".global _start", "_start:", "ldr x0, =0x48000000", "mov sp, x0", "bl probe", "b .");
// ------------------------=
// FUNC: ticks
// DESC: Reads the architectural timer for the freestanding hardware test.
// ------------------=
fn ticks() -> u64 { let value; unsafe { core::arch::asm!("mrs {}, cntvct_el0",out(reg)value); } value }
// ------------------------=
// FUNC: finish
// DESC: Exits QEMU with a machine-checkable semihosting result.
// ------------------=
fn finish(code: u64) -> ! {
    if code == 0 { unsafe { core::arch::asm!("hvc #0", in("x0") 0x84000008u64); } }
    let status = [0x20026u64, code];
    unsafe { core::arch::asm!("hlt #0xf000", in("x0") 0x20u64, in("x1") status.as_ptr()); }
    loop { core::hint::spin_loop(); }
}
// ------------------------=
// FUNC: panic
// DESC: Converts assertions into a failing VM exit rather than a diagnostic-string oracle.
// ------------------=
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { finish(99) }
// ------------------------=
// FUNC: probe
// DESC: Exercises the actual native driver against QEMU HDA DMA and emits a two-second PCM tone.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn probe() -> ! {
    use core::ptr::{read_volatile, write_volatile};
    let mut base = 0;
    for slot in 0..32usize {
        let cfg = (0x4010000000usize + (slot << 15)) as *mut u32;
        if read_volatile(cfg) == 0x26688086 {
            write_volatile(cfg.add(4), 0x10000000);
            write_volatile((cfg as usize + 4) as *mut u16, 6);
            base = 0x10000000;
        }
    }
    assert_ne!(base, 0);
    let mut driver = match hda::Hda::initialize(base, &raw mut DMA) {
        Ok(d) => d,
        Err(hda::Error::Timeout) => finish(2),
        Err(_) => finish(3),
    };
    hda::tone(&mut *(&raw mut PCM));
    assert!(driver.start(&*(&raw const PCM)).is_ok());
    let frequency: u64; core::arch::asm!("mrs {}, cntfrq_el0",out(reg)frequency);
    let start = ticks();
    let mut moved = false;
    while ticks() - start < frequency * 2 {
        let position = driver.position().unwrap();
        assert!(position < (hda::SAMPLES * 2) as u32);
        moved |= position > 0;
    }
    driver.stop();
    assert!(moved);
    finish(0)
}
// ------------------------=
// FUNC: memset
// DESC: Supplies freestanding compiler memory initialization.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn memset(p:*mut core::ffi::c_void,v:i32,n:usize)->*mut core::ffi::c_void {
    for i in 0..n { core::ptr::write_volatile(p.cast::<u8>().add(i),v as u8); } p
}
// ------------------------=
// FUNC: memcpy
// DESC: Supplies freestanding compiler memory copies.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn memcpy(p:*mut core::ffi::c_void,s:*const core::ffi::c_void,n:usize)->*mut core::ffi::c_void {
    for i in 0..n { core::ptr::write_volatile(p.cast::<u8>().add(i),core::ptr::read_volatile(s.cast::<u8>().add(i))); } p
}
