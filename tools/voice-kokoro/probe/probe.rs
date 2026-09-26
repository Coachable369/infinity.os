#![no_std]
static mut PCM: [i16; 720000] = [0; 720000];
#[repr(C, align(4096))]
struct TranslationTable([u64; 512]);
static mut TRANSLATIONS: TranslationTable = TranslationTable([0; 512]);
unsafe extern "C" {
    fn native_synthesize(
        text: *const u8,
        length: usize,
        pcm: *mut i16,
        capacity: usize,
        frames: *mut usize,
        cancel: usize,
        context: usize,
    ) -> i32;
    static native_init_start: usize;
    static native_init_end: usize;
}
core::arch::global_asm!(
    ".section .text.entry",
    ".global _start",
    "_start:",
    "ldr x0, =0xbe000000",
    "mov sp, x0",
    "mov x0, #(3 << 20)",
    "msr cpacr_el1, x0",
    "adr x0, exception_vectors",
    "msr vbar_el1, x0",
    "isb",
    "bl probe",
    "b .",
    ".balign 2048",
    "exception_vectors:",
    ".rept 16",
    ".balign 128",
    "b exception",
    ".endr"
);
// ------------------------=
// FUNC: bytes
// DESC: Emits actual guest PCM and structured results through the test UART.
// ------------------=
unsafe fn bytes(data: &[u8]) {
    for &b in data {
        while core::ptr::read_volatile(0x09000018 as *const u32) & 32 != 0 {}
        core::ptr::write_volatile(0x09000000 as *mut u32, b as u32);
    }
}
// ------------------------=
// FUNC: finish
// DESC: Powers down only the disposable native test guest.
// ------------------=
fn finish() -> ! {
    unsafe {
        core::arch::asm!("hvc #0", in("x0") 0x84000008u64);
    }
    loop {
        core::hint::spin_loop();
    }
}
// ------------------------=
// FUNC: exception
// DESC: Reports CPU faults numerically, distinct from successful synthesis.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn exception() -> ! {
    let (esr, elr, far): (u64, u64, u64);
    core::arch::asm!("mrs {},esr_el1",out(reg)esr);
    core::arch::asm!("mrs {},elr_el1",out(reg)elr);
    core::arch::asm!("mrs {},far_el1",out(reg)far);
    for value in [0, esr, elr, far] {
        bytes(&value.to_le_bytes());
    }
    finish()
}
// ------------------------=
// FUNC: panic
// DESC: Marks a failed probe assertion as a non-success binary result.
// ------------------=
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe {
        bytes(&[0; 32]);
    }
    finish()
}
// ------------------------=
// FUNC: cancel
// DESC: Exercises cancellation without a host process or timer.
// ------------------=
unsafe extern "C" fn cancel(_: usize) -> i32 {
    1
}
// ------------------------=
// FUNC: normal_memory
// DESC: Maps guest RAM as normal cacheable memory so native CPU atomics have architectural support.
// ------------------=
unsafe fn normal_memory() {
    let table = (&raw mut TRANSLATIONS).cast::<u64>();
    core::ptr::write(table, 1 | (1 << 10));
    for i in 1..4 {
        core::ptr::write(
            table.add(i),
            ((i as u64) << 30) | 1 | (1 << 2) | (3 << 8) | (1 << 10),
        );
    }
    let tcr: u64 = 25 | (1 << 8) | (1 << 10) | (3 << 12) | (1 << 23) | (2 << 32);
    core::arch::asm!("dsb sy", "msr mair_el1,{mair}", "msr ttbr0_el1,{table}",
        "msr tcr_el1,{tcr}", "isb", "tlbi vmalle1", "dsb sy", "isb",
        mair=in(reg)0xff00u64,table=in(reg)table,tcr=in(reg)tcr);
    let mut control: u64;
    core::arch::asm!("mrs {},sctlr_el1",out(reg)control);
    control |= 1 | (1 << 2) | (1 << 12);
    core::arch::asm!("msr sctlr_el1,{}", "isb",in(reg)control);
}
// ------------------------=
// FUNC: probe
// DESC: Executes genuine Kokoro twice plus cancellation and input bounds in a freestanding guest.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn probe() -> ! {
    normal_memory();
    let mut init = &raw const native_init_start;
    while init < &raw const native_init_end {
        let function: unsafe extern "C" fn() = core::mem::transmute(*init);
        function();
        init = init.add(1);
    }
    for case in 0..4 {
        let mut frames = 0;
        let (start, end, frequency): (u64, u64, u64);
        core::arch::asm!("mrs {},cntvct_el0",out(reg)start);
        let result = native_synthesize(
            b"Hi.".as_ptr(),
            if case == 3 { 0 } else { 3 },
            (&raw mut PCM).cast(),
            720000,
            &mut frames,
            if case == 2 {
                cancel as *const () as usize
            } else {
                0
            },
            0,
        );
        core::arch::asm!("mrs {},cntvct_el0",out(reg)end);
        core::arch::asm!("mrs {},cntfrq_el0",out(reg)frequency);
        for value in [
            1,
            case,
            result as u64,
            frames as u64,
            end - start,
            frequency,
        ] {
            bytes(&value.to_le_bytes());
        }
        assert!(frames <= 720000);
        bytes(core::slice::from_raw_parts(
            (&raw const PCM).cast(),
            frames * 2,
        ));
        if result != [0, 0, 2, 1][case as usize] {
            finish();
        }
    }
    finish()
}
