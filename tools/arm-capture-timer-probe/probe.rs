#![no_std]
#![allow(dead_code)]
use core::sync::atomic::{AtomicUsize, Ordering};
#[path = "../../kernel/core/boot_info.rs"]
mod boot_info;
#[path = "../../kernel/drivers/hda.rs"]
mod hda;
const MAGIC: u64 = 0x494e46434150544d;
static PHASE: AtomicUsize = AtomicUsize::new(0);
static ACK: AtomicUsize = AtomicUsize::new(usize::MAX);
static READY: AtomicUsize = AtomicUsize::new(0);
static mut RECORDS: [[u64; 14]; 8] = [[0; 14]; 8];
static mut TARGET_STATES: [[u64; 14]; 6] = [[0; 14]; 6];
static mut DMA: hda::Dma = hda::Dma::new();
static mut PCM: [i16; 8192] = [0; 8192];

// ------------------------=
// FUNC: ticks
// DESC: Reads the native counter independently of VirtualBox's device timer queue.
// ------------------=
fn ticks() -> u64 { let v; unsafe { core::arch::asm!("mrs {}, cntvct_el0", out(reg) v); } v }

// ------------------------=
// FUNC: registers
// DESC: Records actual per-core timer ownership and expiry before applying any experiment.
// ------------------=
unsafe fn registers() -> [u64; 14] {
    let (cpu, level, mask, pctl, pcval, vctl, vcval, physical, virtual_time, freq);
    core::arch::asm!("mrs {}, mpidr_el1", out(reg) cpu);
    core::arch::asm!("mrs {}, CurrentEL", out(reg) level);
    core::arch::asm!("mrs {}, daif", out(reg) mask);
    core::arch::asm!("mrs {}, cntp_ctl_el0", out(reg) pctl);
    core::arch::asm!("mrs {}, cntp_cval_el0", out(reg) pcval);
    core::arch::asm!("mrs {}, cntv_ctl_el0", out(reg) vctl);
    core::arch::asm!("mrs {}, cntv_cval_el0", out(reg) vcval);
    core::arch::asm!("mrs {}, cntpct_el0", out(reg) physical);
    core::arch::asm!("mrs {}, cntvct_el0", out(reg) virtual_time);
    core::arch::asm!("mrs {}, cntfrq_el0", out(reg) freq);
    let (isr, pending, group, priority);
    core::arch::asm!("mrs {}, isr_el1", out(reg) isr);
    core::arch::asm!("mrs {}, ICC_HPPIR1_EL1", out(reg) pending);
    core::arch::asm!("mrs {}, ICC_IGRPEN1_EL1", out(reg) group);
    core::arch::asm!("mrs {}, ICC_PMR_EL1", out(reg) priority);
    [cpu, level, mask, pctl, pcval, vctl, vcval, physical, virtual_time, freq, isr, pending, group, priority]
}

// ------------------------=
// FUNC: worker
// DESC: Uses the production AP bridge with controlled timer phases on CPU7 and unchanged WFE mailbox behavior.
// ------------------=
unsafe extern "efiapi" fn worker(argument: *mut u8) {
    let slot = argument as usize;
    if slot >= 8 { return; }
    RECORDS[slot] = registers();
    READY.fetch_or(1 << slot, Ordering::Release);
    let target = RECORDS[slot][0] & 255 == 7;
    let mut previous = usize::MAX;
    loop {
        let phase = PHASE.load(Ordering::Acquire);
        if target && phase != previous {
            if phase == 1 || phase >= 3 {
                // VirtualBox 7.2.16 reads CNTP but rejects even a zero write.
                core::arch::asm!("msr cntv_ctl_el0, xzr", "isb");
            } else if phase == 2 {
                core::arch::asm!("msr cntv_cval_el0, xzr", "msr cntv_ctl_el0, {}", "isb", in(reg) 1u64);
            }
            TARGET_STATES[phase] = registers();
            previous = phase;
            ACK.store(phase, Ordering::Release);
        }
        if target && phase == 4 { core::hint::spin_loop(); }
        else { core::arch::asm!("wfe", options(nomem, nostack)); }
    }
}

// ------------------------=
// FUNC: bytes
// DESC: Emits bounded binary evidence only outside measured capture phases on the VirtualBox PL011.
// ------------------=
unsafe fn bytes(data: &[u8]) {
    const HEX: &[u8] = b"0123456789abcdef";
    for &value in data {
        for character in [HEX[(value >> 4) as usize], HEX[(value & 15) as usize]] {
            while core::ptr::read_volatile(0xffdd_f018usize as *const u32) & 32 != 0 {}
            core::ptr::write_volatile(0xffdd_f000usize as *mut u32, character as u32);
        }
    }
    while core::ptr::read_volatile(0xffdd_f018usize as *const u32) & 32 != 0 {}
    core::ptr::write_volatile(0xffdd_f000usize as *mut u32, 10);
    while core::ptr::read_volatile(0xffdd_f018usize as *const u32) & 8 != 0 {}
}

// ------------------------=
// FUNC: finish
// DESC: Powers down only this disposable native diagnostic guest after reporting numeric status.
// ------------------=
fn finish(status: u64) -> ! {
    unsafe { bytes(&MAGIC.to_le_bytes()); bytes(&status.to_le_bytes());
        core::arch::asm!("hvc #0", in("x0") 0x84000008u64); }
    loop { core::hint::spin_loop(); }
}

// ------------------------=
// FUNC: panic
// DESC: Reports failed invariants through a structured nonzero result.
// ------------------=
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { finish(99) }

// ------------------------=
// FUNC: infinity_kernel_entry
// DESC: Measures native microphone DMA without keyboard, pointer, clock-register kicks, or host recognition services.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_kernel_entry(info: &boot_info::BootInfo) -> ! {
    assert_eq!(info.magic, boot_info::BOOT_MAGIC);
    assert!(info.worker_bridge != 0 && info.boot_reserved != 0);
    RECORDS[0] = registers();
    assert_ne!(RECORDS[0][0] & 255, 7);
    let frequency = RECORDS[0][9];
    let mut device = hda::Hda::initialize(info.boot_reserved as usize, &raw mut DMA).unwrap();
    device.start_capture().unwrap();
    #[repr(C)]
    struct Bridge { version: u64, start: unsafe extern "efiapi" fn(unsafe extern "efiapi" fn(*mut u8), u64) -> u64 }
    let bridge = &*(info.worker_bridge as *const Bridge);
    assert_eq!(bridge.version, 1);
    let count = (bridge.start)(worker, 7);
    assert_eq!(count, 7);
    let started = ticks();
    while READY.load(Ordering::Acquire) != 254 {
        if ticks() - started > frequency * 2 { finish(2); }
    }
    let header = [MAGIC, 2, count, device.sample_rate as u64, frequency];
    for value in header { bytes(&value.to_le_bytes()); }
    for record in &*(&raw const RECORDS) { for value in record { bytes(&value.to_le_bytes()); } }
    let mut phases = [[0u64; 8]; 6];
    for phase in 0..6 {
        PHASE.store(phase, Ordering::Release);
        core::arch::asm!("sev", options(nomem, nostack));
        let begin = ticks();
        while ACK.load(Ordering::Acquire) != phase {
            if ticks() - begin > frequency * 2 { finish(3); }
        }
        let begin = ticks();
        let mut last_data = begin;
        let mut frames = 0;
        let mut max_gap = 0;
        let mut reads = 0;
        let start_lpib = core::ptr::read_volatile((info.boot_reserved as usize + 0x84) as *const u32);
        while ticks() - begin < frequency * 8 {
            let now = ticks();
            let samples = device.read_capture(&mut *(&raw mut PCM)).unwrap();
            reads += 1;
            if samples > 0 {
                max_gap = max_gap.max(now - last_data);
                last_data = now;
                frames += samples as u64 / 2;
            }
        }
        max_gap = max_gap.max(ticks() - last_data);
        let end_lpib = core::ptr::read_volatile((info.boot_reserved as usize + 0x84) as *const u32);
        phases[phase] = [phase as u64, begin, ticks(), frames, max_gap, reads, start_lpib as u64, end_lpib as u64];
        for value in TARGET_STATES[phase] { bytes(&value.to_le_bytes()); }
        for value in phases[phase] { bytes(&value.to_le_bytes()); }
    }
    device.stop_capture();
    finish(0)
}

// ------------------------=
// FUNC: memset
// DESC: Supplies freestanding compiler memory initialization.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn memset(p: *mut core::ffi::c_void, value: i32, count: usize) -> *mut core::ffi::c_void {
    for i in 0..count { core::ptr::write_volatile(p.cast::<u8>().add(i), value as u8); } p
}

// ------------------------=
// FUNC: memcpy
// DESC: Supplies freestanding compiler memory copying.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn memcpy(p: *mut core::ffi::c_void, source: *const core::ffi::c_void, count: usize) -> *mut core::ffi::c_void {
    for i in 0..count { core::ptr::write_volatile(p.cast::<u8>().add(i), core::ptr::read_volatile(source.cast::<u8>().add(i))); } p
}
