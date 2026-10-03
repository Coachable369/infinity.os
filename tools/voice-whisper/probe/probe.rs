#![no_std]
static INPUT: &[u8] = include_bytes!(env!("INFINITY_STT_FIXTURE"));
static mut PCM: [i16; 160000] = [0; 160000];
static mut SYNTH: [i16; 160000] = [0; 160000];
#[repr(C, align(4096))]
struct WorkerStack([u8; 8 * 1024 * 1024]);
static mut WORKER_STACK: WorkerStack = WorkerStack([0; 8 * 1024 * 1024]);
#[repr(C, align(4096))]
struct ExceptionStack([u8; 16 * 1024]);
static mut EXCEPTION_STACK: ExceptionStack = ExceptionStack([0; 16 * 1024]);
#[repr(C, align(4096))]
struct TranslationTable([u64; 512]);
static mut TRANSLATIONS: TranslationTable = TranslationTable([0; 512]);
core::arch::global_asm!(include_str!("../../../boot/aarch64/handoff.S"));

unsafe extern "C" {
    fn infinity_ap_callback(context: *const u64);
    fn infinity_kokoro_native_recognize(
        pcm: *const i16,
        samples: usize,
        text: *mut u8,
        capacity: usize,
        length: *mut usize,
        memory: *mut usize,
        cancel: usize,
        context: usize,
    ) -> i32;
    fn infinity_kokoro_native_prepare_recognition(
        memory: *mut usize,
        cancel: usize,
        context: usize,
    ) -> i32;
    fn infinity_kokoro_native_diagnostics(output: *mut usize);
    fn infinity_kokoro_native_synthesize(
        text: *const u8,
        length: usize,
        pcm: *mut i16,
        capacity: usize,
        frames: *mut usize,
        cancel: usize,
        context: usize,
    ) -> i32;
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
// FUNC: cancel
// DESC: Exercises immediate caller cancellation before inference publication.
// ------------------=
unsafe extern "C" fn cancel(_: usize) -> i32 {
    1
}

// ------------------------=
// FUNC: normal_memory
// DESC: Maps guest RAM as normal cacheable memory for native atomics and model execution.
// ------------------=
unsafe fn normal_memory() {
    let table = (&raw mut TRANSLATIONS).cast::<u64>();
    core::ptr::write(table, 1 | (1 << 10));
    for index in 1..4 {
        core::ptr::write(table.add(index), ((index as u64) << 30) | 1 | (1 << 2) | (3 << 8) | (1 << 10));
    }
    let control = 25 | (1 << 8) | (1 << 10) | (3 << 12) | (1 << 23) | (2u64 << 32);
    core::arch::asm!("dsb sy", "msr mair_el1,{mair}", "msr ttbr0_el1,{table}",
        "msr tcr_el1,{control}", "isb", "tlbi vmalle1", "dsb sy", "isb",
        mair=in(reg)0xff00u64, table=in(reg)table, control=in(reg)control);
    let mut system: u64;
    core::arch::asm!("mrs {},sctlr_el1",out(reg)system);
    system |= 1 | (1 << 2) | (1 << 12);
    core::arch::asm!("msr sctlr_el1,{}", "isb",in(reg)system);
}

// ------------------------=
// FUNC: bytes
// DESC: Emits only structured guest evidence and recognizer API output over PL011.
// ------------------=
unsafe fn bytes(data: &[u8]) {
    for &byte in data {
        while core::ptr::read_volatile(0x09000018 as *const u32) & 32 != 0 {}
        core::ptr::write_volatile(0x09000000 as *mut u32, byte as u32);
    }
}

// ------------------------=
// FUNC: finish
// DESC: Stops only the disposable native test machine through PSCI.
// ------------------=
fn finish() -> ! {
    unsafe { core::arch::asm!("hvc #0", in("x0") 0x84000008u64); }
    loop { core::hint::spin_loop(); }
}

// ------------------------=
// FUNC: exception
// DESC: Reports architecture faults as numeric fields distinct from recognition results.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn exception() -> ! {
    let (esr, elr, far): (u64, u64, u64);
    core::arch::asm!("mrs {},esr_el1",out(reg)esr);
    core::arch::asm!("mrs {},elr_el1",out(reg)elr);
    core::arch::asm!("mrs {},far_el1",out(reg)far);
    for value in [0, esr, elr, far] { bytes(&value.to_le_bytes()); }
    finish()
}

// ------------------------=
// FUNC: panic
// DESC: Marks a failed guest assertion without treating diagnostic text as evidence.
// ------------------=
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe { bytes(&[0; 32]); }
    finish()
}

// ------------------------=
// FUNC: probe
// DESC: Runs recognition on the production secondary-processor stack boundary.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn probe() -> ! {
    normal_memory();
    (&mut *(&raw mut WORKER_STACK.0)).fill(0xa5);
    let stack = (&raw mut WORKER_STACK.0).cast::<u8>() as u64;
    let emergency = (&raw mut EXCEPTION_STACK.0).cast::<u8>() as u64 + 16 * 1024;
    let context = [stack + 8 * 1024 * 1024, recognition_cases as *const () as u64, 0,
                   0, 0, 0, 0, 0, 0, 0, emergency];
    infinity_ap_callback(context.as_ptr());
    finish()
}

// ------------------------=
// FUNC: recognition_cases
// DESC: Verifies real decoding, repeat use, output bounds, silence rejection, and cancellation.
// ------------------=
unsafe extern "C" fn recognition_cases(_: *mut u8) {
    for (index, pair) in INPUT.chunks_exact(2).enumerate() {
        PCM[index] = i16::from_le_bytes([pair[0], pair[1]]);
    }
    let mut prepared_memory = 0usize;
    assert_eq!(
        infinity_kokoro_native_prepare_recognition(&mut prepared_memory, 0, 0),
        0,
    );
    assert!(prepared_memory > 0);
    for case in 0..5usize {
        let mut text = [0u8; 512];
        let mut length = 0usize;
        let mut memory = 0usize;
        let start: u64;
        let end: u64;
        let frequency: u64;
        if case == 1 {
            let mut frames = 0usize;
            assert_eq!(infinity_kokoro_native_synthesize(b"Hi.".as_ptr(), 3,
                (&raw mut SYNTH).cast(), 160000, &mut frames, 0, 0), 0);
            assert!(frames > 0);
        }
        if case == 3 { (&mut *(&raw mut PCM)).fill(0); }
        if case == 4 {
            for (index, pair) in INPUT.chunks_exact(2).enumerate() {
                PCM[index] = i16::from_le_bytes([pair[0], pair[1]]);
            }
        }
        core::arch::asm!("mrs {},cntvct_el0",out(reg)start);
        let result = infinity_kokoro_native_recognize(
            (&raw const PCM).cast(), INPUT.len() / 2, text.as_mut_ptr(),
            if case == 2 { 2 } else { text.len() }, &mut length, &mut memory,
            if case == 4 { cancel as *const () as usize } else { 0 }, 0,
        );
        let mut diagnostics = [0usize; 12];
        infinity_kokoro_native_diagnostics(diagnostics.as_mut_ptr());
        core::arch::asm!("mrs {},cntvct_el0",out(reg)end);
        core::arch::asm!("mrs {},cntfrq_el0",out(reg)frequency);
        let stack = &*(&raw const WORKER_STACK.0);
        let first = stack.iter().position(|byte| *byte != 0xa5).unwrap_or(stack.len());
        for value in [1, case as u64, result as u64, length as u64, memory as u64,
                      end - start, frequency, (stack.len() - first) as u64] {
            bytes(&value.to_le_bytes());
        }
        for value in diagnostics { bytes(&(value as u64).to_le_bytes()); }
        bytes(&text[..length]);
        assert_eq!(result, [0, 0, 6, 5, 2][case]);
        if case >= 2 { assert_eq!(length, 0); }
    }
}
