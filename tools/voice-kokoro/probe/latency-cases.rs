#![no_std]
#[path = "../../../kernel/core/boot_info.rs"]
mod boot_info;
#[path = "../../../kernel/runtime/ai/qwen/workers.rs"]
mod workers;
mod dot;
static mut PCM: [i16; 720000] = [0; 720000];
static CANCEL_CALLS: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
static CANCEL_HELPERS_BASELINE: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
unsafe extern "C" {
    // ------------------------=
    // FUNC: native_synthesize
    // DESC: Invokes the production bounded native speech provider.
    // ------------------=
    #[link_name = "infinity_kokoro_native_synthesize"]
    fn native_synthesize(text: *const u8, length: usize, pcm: *mut i16, capacity: usize,
                         frames: *mut usize, cancel: usize, context: usize) -> i32;
    // ------------------------=
    // FUNC: native_diagnostics
    // DESC: Exports structured native heap and failure state after each operation.
    // ------------------=
    #[link_name = "infinity_kokoro_native_diagnostics"]
    fn native_diagnostics(out: *mut usize);
    // ------------------------=
    // FUNC: native_resource
    // DESC: Reads the immutable packaged dependency notice bytes.
    // ------------------=
    #[link_name = "infinity_kokoro_native_resource"]
    fn native_resource(name: *const u8, length: *mut usize) -> *const u8;
    // ------------------------=
    // FUNC: native_profile_read
    // DESC: Exports operation counters to reject instrumented latency measurements.
    // ------------------=
    #[link_name = "infinity_kokoro_native_profile_read"]
    fn native_profile_read(out: *mut u64);
    // ------------------------=
    // FUNC: native_verify_matrix_tiles
    // DESC: Compares optimized native tiles and broadcast/strided dispatch with the reference arithmetic.
    // ------------------=
    #[cfg(not(feature = "baseline-math"))]
    #[link_name = "infinity_kokoro_native_verify_matrix_tiles"]
    fn native_verify_matrix_tiles() -> usize;
    // ------------------------=
    // FUNC: exception_vectors
    // DESC: Names the isolated probe's exception table.
    // ------------------=
    fn exception_vectors();
}
core::arch::global_asm!(".section .text", ".balign 2048", ".global exception_vectors",
    "exception_vectors:", ".rept 16", ".balign 128", "b exception", ".endr");

// ------------------------=
// FUNC: bytes
// DESC: Emits structured measurements and complete PCM through the disposable guest's UART.
// ------------------=
unsafe fn bytes(data: &[u8]) {
    for &byte in data {
        while core::ptr::read_volatile(0x09000018 as *const u32) & 32 != 0 {}
        core::ptr::write_volatile(0x09000000 as *mut u32, byte as u32);
    }
}

// ------------------------=
// FUNC: finish
// DESC: Powers off only the disposable native measurement guest after its binary result.
// ------------------=
fn finish() -> ! {
    unsafe { core::arch::asm!("hvc #0", in("x0") 0x84000008u64); }
    loop { core::hint::spin_loop(); }
}

// ------------------------=
// FUNC: exception
// DESC: Invalidates the result on an architectural fault and preserves fault registers.
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
// DESC: Rejects failed behavioral assertions without mistaking partial output for success.
// ------------------=
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe { bytes(&[0; 32]); }
    finish()
}

// ------------------------=
// FUNC: infinity_kernel_entry
// DESC: Starts real native worker adapters through the ARM loader, without the desktop or installed VM.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_kernel_entry(info: &boot_info::BootInfo) -> ! {
    assert_eq!(info.magic, boot_info::BOOT_MAGIC);
    workers::initialize(info.worker_bridge);
    // This isolated probe owns no IRQ handler or desktop service loop. Keep
    // firmware IRQs masked before installing its fault-only vector table.
    core::arch::asm!("msr daifset, #0xf", "msr vbar_el1, {}", "isb",
                     in(reg) exception_vectors as *const () as u64);
    let expected: usize = env!("INFINITY_PROBE_WORKERS").parse().unwrap();
    let start = workers::clock_ns();
    while workers::online() < expected {
        assert!(workers::clock_ns().saturating_sub(start) < 3_000_000_000);
        core::hint::spin_loop();
    }
    assert!(workers::background(speech_cases));
    loop { core::arch::asm!("wfe", options(nomem, nostack)); }
}

// ------------------------=
// FUNC: cancel
// DESC: Tests cancellation before entering inference.
// ------------------=
unsafe extern "C" fn cancel(_: usize) -> i32 { 1 }

// ------------------------=
// FUNC: cancel_inference
// DESC: Cancels only after this inference has joined native helper work when helpers are explicitly enabled, or after eight serial inference checks.
// ------------------=
unsafe extern "C" fn cancel_inference(_: usize) -> i32 {
    let checked = CANCEL_CALLS.fetch_add(1, core::sync::atomic::Ordering::Relaxed) + 1 >= 8;
    let helpers_complete = env!("INFINITY_PROBE_HELPERS") != "1"
        || workers::parallel_completed() > CANCEL_HELPERS_BASELINE.load(core::sync::atomic::Ordering::Relaxed);
    i32::from(checked && helpers_complete)
}

// ------------------------=
// FUNC: speech_cases
// DESC: Measures identical synthesis and cancellation cases with real AP helpers and returns complete samples plus helper completion counts.
// ------------------=
unsafe fn speech_cases() {
    core::arch::asm!("msr vbar_el1, {}", "isb", in(reg) exception_vectors as *const () as u64);
    bytes(&0x494e464c41544331u64.to_le_bytes());
    bytes(&(workers::online() as u64).to_le_bytes());
    for case in 0..10usize {
        let text: &[u8] = match case {
            6 => b"Hello, I am Infinity. How can I help you today?",
            7 | 9 => b"Welcome to Infinity. Your assistant runs entirely on this computer. You can ask questions, work on your projects, and explore your ideas with a natural voice.",
            _ => b"Hi.",
        };
        let (start, end, frequency): (u64, u64, u64);
        let mut frames = 0;
        if case == 4 {
            CANCEL_CALLS.store(0, core::sync::atomic::Ordering::Relaxed);
            CANCEL_HELPERS_BASELINE.store(workers::parallel_completed(), core::sync::atomic::Ordering::Relaxed);
        }
        core::arch::asm!("mrs {},cntvct_el0", out(reg) start);
        let result = native_synthesize(text.as_ptr(), if case == 3 { 0 } else { text.len() },
            (&raw mut PCM).cast(), 720000, &mut frames,
            if case == 2 { cancel as *const () as usize }
            else if case == 4 { cancel_inference as *const () as usize } else { 0 }, 0);
        core::arch::asm!("mrs {},cntvct_el0", out(reg) end);
        core::arch::asm!("mrs {},cntfrq_el0", out(reg) frequency);
        let mut diagnostics = [0usize; 12];
        native_diagnostics(diagnostics.as_mut_ptr());
        for value in [2, case as u64, result as u64, frames as u64, end - start,
                      frequency, diagnostics[0] as u64, diagnostics[1] as u64,
                      diagnostics[2] as u64, diagnostics[3] as u64] {
            bytes(&value.to_le_bytes());
        }
        for value in &diagnostics[4..] { bytes(&(*value as u64).to_le_bytes()); }
        assert!(frames <= 720000);
        bytes(core::slice::from_raw_parts((&raw const PCM).cast(), frames * 2));
        assert_eq!(result, [0, 0, 2, 1, 2, 0, 0, 0, 0, 0][case]);
        if case == 4 && env!("INFINITY_PROBE_HELPERS") == "1" {
            assert!(workers::parallel_completed() > CANCEL_HELPERS_BASELINE.load(core::sync::atomic::Ordering::Relaxed));
        }
    }
    let mut notice_length = 0;
    let notice = native_resource(b"/licenses/kokoro-dependencies.txt\0".as_ptr(), &mut notice_length);
    assert!(!notice.is_null() && notice_length > 0 && notice_length <= 1048576);
    bytes(&(notice_length as u64).to_le_bytes());
    bytes(core::slice::from_raw_parts(notice, notice_length));
    let mut profile = [0u64; 256];
    native_profile_read(profile.as_mut_ptr());
    for value in profile { bytes(&value.to_le_bytes()); }
    bytes(&dot::verify().to_le_bytes());
    bytes(&dot::verify_tiles().to_le_bytes());
    #[cfg(not(feature = "baseline-math"))]
    assert_eq!(native_verify_matrix_tiles(), 392);
    bytes(&0u64.to_le_bytes()); // Firmware-owned stack: use standalone canary probe for its bound.
    bytes(&(workers::parallel_completed() as u64).to_le_bytes());
    finish()
}
