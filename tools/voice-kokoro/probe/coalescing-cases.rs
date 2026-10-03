#![no_std]

static mut PCM: [i16; 720000] = [0; 720000];
#[repr(C, align(4096))]
struct WorkerStack([u8; 8 * 1024 * 1024]);
static mut WORKER_STACK: WorkerStack = WorkerStack([0; 8 * 1024 * 1024]);
#[repr(C, align(4096))]
struct TranslationTable([u64; 512]);
static mut TRANSLATIONS: TranslationTable = TranslationTable([0; 512]);
core::arch::global_asm!(include_str!("../../../boot/aarch64/handoff.S"));
unsafe extern "C" {
    // ------------------------=
    // FUNC: infinity_ap_callback
    // DESC: Enters the production ARM worker trampoline using the supplied stack, callback, and argument.
    // ------------------=
    fn infinity_ap_callback(context: *const u64);
    #[link_name = "infinity_kokoro_native_synthesize"]
    // ------------------------=
    // FUNC: native_synthesize
    // DESC: Generates bounded PCM using the exact native engine linked into InfinityOS.
    // ------------------=
    fn native_synthesize(text: *const u8, length: usize, pcm: *mut i16, capacity: usize,
                         frames: *mut usize, cancel: usize, context: usize) -> i32;
}
core::arch::global_asm!(
    ".section .text.entry", ".global _start", "_start:",
    "ldr x0, =0xbe000000", "mov sp, x0", "mov x0, #(3 << 20)", "msr cpacr_el1, x0",
    "adr x0, exception_vectors", "msr vbar_el1, x0", "isb", "bl probe", "b .",
    ".balign 2048", "exception_vectors:", ".rept 16", ".balign 128", "b exception", ".endr"
);

// ------------------------=
// FUNC: bytes
// DESC: Emits structured measurements and native PCM through the disposable guest UART.
// ------------------=
unsafe fn bytes(data: &[u8]) {
    for &byte in data {
        while core::ptr::read_volatile(0x09000018 as *const u32) & 32 != 0 {}
        core::ptr::write_volatile(0x09000000 as *mut u32, byte as u32);
    }
}

// ------------------------=
// FUNC: finish
// DESC: Powers down only the isolated measurement guest after emitting its result.
// ------------------=
fn finish() -> ! {
    unsafe { core::arch::asm!("hvc #0", in("x0") 0x84000008u64); }
    loop { core::hint::spin_loop(); }
}

// ------------------------=
// FUNC: exception
// DESC: Reports architectural faults as an invalid measurement before stopping the guest.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn exception() -> ! {
    let (esr, elr, far): (u64, u64, u64);
    core::arch::asm!("mrs {},esr_el1", out(reg) esr);
    core::arch::asm!("mrs {},elr_el1", out(reg) elr);
    core::arch::asm!("mrs {},far_el1", out(reg) far);
    for value in [0, esr, elr, far] { bytes(&value.to_le_bytes()); }
    finish()
}

// ------------------------=
// FUNC: panic
// DESC: Makes any failed native assertion invalidate the binary result.
// ------------------=
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe { bytes(&[0; 32]); }
    finish()
}

// ------------------------=
// FUNC: normal_memory
// DESC: Enables normal cacheable guest RAM required by the native engine's atomic instructions.
// ------------------=
unsafe fn normal_memory() {
    let table = (&raw mut TRANSLATIONS).cast::<u64>();
    core::ptr::write(table, 1 | (1 << 10));
    for index in 1..4 {
        core::ptr::write(table.add(index), ((index as u64) << 30) | 1 | (1 << 2) | (3 << 8) | (1 << 10));
    }
    let tcr: u64 = 25 | (1 << 8) | (1 << 10) | (3 << 12) | (1 << 23) | (2 << 32);
    core::arch::asm!("dsb sy", "msr mair_el1,{mair}", "msr ttbr0_el1,{table}",
        "msr tcr_el1,{tcr}", "isb", "tlbi vmalle1", "dsb sy", "isb",
        mair=in(reg)0xff00u64, table=in(reg)table, tcr=in(reg)tcr);
    let mut control: u64;
    core::arch::asm!("mrs {},sctlr_el1", out(reg)control);
    control |= 1 | (1 << 2) | (1 << 12);
    core::arch::asm!("msr sctlr_el1,{}", "isb", in(reg)control);
}

// ------------------------=
// FUNC: probe
// DESC: Runs paired measurements through the production native worker stack trampoline.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn probe() -> ! {
    normal_memory();
    let stack = (&raw mut WORKER_STACK.0).cast::<u8>() as u64;
    let context = [stack + 8 * 1024 * 1024, speech_cases as *const () as u64, 0];
    infinity_ap_callback(context.as_ptr());
    finish()
}

// ------------------------=
// FUNC: speech_cases
// DESC: Measures the same greeting as two independently synthesized spans and as one coherent span, alternating repeat order in a native guest.
// ------------------=
unsafe extern "C" fn speech_cases(_: u64) {
    let greeting = b"Hello! How can I assist you today?";
    let mut warm_frames = 0;
    assert_eq!(native_synthesize(b"Hi.".as_ptr(), 3, (&raw mut PCM).cast(), 720000,
        &mut warm_frames, 0, 0), 0);
    for (case, split) in [true, false, false, true].into_iter().enumerate() {
        let mut frames = 0usize;
        let mut status = 0;
        let (start, end, frequency): (u64, u64, u64);
        core::arch::asm!("mrs {},cntvct_el0", out(reg) start);
        let spans: &[&[u8]] = if split { &[&greeting[..6], &greeting[7..]] } else { &[greeting] };
        for text in spans {
            let mut produced = 0;
            status = native_synthesize(text.as_ptr(), text.len(), (&raw mut PCM).cast::<i16>().add(frames),
                720000 - frames, &mut produced, 0, 0);
            if status != 0 { break; }
            frames += produced;
        }
        core::arch::asm!("mrs {},cntvct_el0", out(reg) end);
        core::arch::asm!("mrs {},cntfrq_el0", out(reg) frequency);
        for value in [0x494e46434f414c31, case as u64, split as u64, status as u64,
                      frames as u64, end - start, frequency] {
            bytes(&value.to_le_bytes());
        }
        assert!(status == 0 && frames > 0 && frames <= 720000);
        bytes(core::slice::from_raw_parts((&raw const PCM).cast(), frames * 2));
    }
}
