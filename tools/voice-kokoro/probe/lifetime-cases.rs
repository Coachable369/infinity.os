unsafe extern "C" {
    // ------------------------=
    // FUNC: infinity_kokoro_native_diagnostics
    // DESC: Reads bounded native allocator and phase diagnostics without exposing user content.
    // ------------------=
    fn infinity_kokoro_native_diagnostics(out: *mut usize);
    // ------------------------=
    // FUNC: infinity_kokoro_native_memory_stats
    // DESC: Reads private allocator and graph metadata high-water values between exclusive native jobs.
    // ------------------=
    fn infinity_kokoro_native_memory_stats(out: *mut usize);
    // ------------------------=
    // FUNC: infinity_kokoro_ggml_backend_buffer_is_meta
    // DESC: Exercises the exact native backend operation that faulted on the installed system.
    // ------------------=
    fn infinity_kokoro_ggml_backend_buffer_is_meta(buffer: *const u8) -> bool;
    // ------------------------=
    // FUNC: infinity_kokoro_native_prepare_recognition
    // DESC: Initializes the shared resident speech engines before varied synthesis jobs.
    // ------------------=
    fn infinity_kokoro_native_prepare_recognition(memory: *mut usize, cancel: usize, context: usize) -> i32;
    // ------------------------=
    // FUNC: infinity_kokoro_native_recognize
    // DESC: Exercises the actual recognizer before synthesis to reproduce shared arena allocation order.
    // ------------------=
    fn infinity_kokoro_native_recognize(pcm: *const i16, samples: usize, text: *mut u8,
        capacity: usize, length: *mut usize, memory: *mut usize, cancel: usize, context: usize) -> i32;
}
static INPUT: &[u8] = include_bytes!(env!("INFINITY_STT_FIXTURE"));
static mut INPUT_PCM: [i16; 160000] = [0; 160000];
static mut TRANSCRIPT: [u8; 512] = [0; 512];

core::arch::global_asm!(
    ".text", ".global lifetime_vector", "lifetime_vector:",
    "mov x4,x0", "mov x3,x30", "mov x5,x29",
    "mrs x0,esr_el1", "mrs x1,elr_el1", "mrs x2,far_el1", "b lifetime_fault"
);

// ------------------------=
// FUNC: lifetime_fault
// DESC: Captures the original native fault, bad pointer and bounded frame chain before stopping the isolated guest.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn lifetime_fault(esr: u64, pc: u64, far: u64, lr: u64, argument: u64, mut frame: u64) -> ! {
    let mut diagnostics = [0usize; 12];
    infinity_kokoro_native_diagnostics(diagnostics.as_mut_ptr());
    for value in [0x494e464c49464546, esr, pc, far, lr, argument] { bytes(&value.to_le_bytes()); }
    for value in diagnostics { bytes(&(value as u64).to_le_bytes()); }
    let low = (&raw const WORKER_STACK.0).cast::<u8>() as u64;
    for _ in 0..8 {
        if frame < low || frame > low + 8 * 1024 * 1024 - 16 || frame % 16 != 0 {
            bytes(&0u64.to_le_bytes());
            frame = 0;
        } else {
            bytes(&core::ptr::read((frame + 8) as *const u64).to_le_bytes());
            frame = core::ptr::read(frame as *const u64);
        }
    }
    finish()
}

// ------------------------=
// FUNC: report
// DESC: Emits one fixed-width native state transition and allocator snapshot for behavioral validation.
// ------------------=
unsafe fn report(case: usize, kind: u64, status: i32, frames: usize, ticks: u64, hash: u64) {
    let frequency: u64;
    core::arch::asm!("mrs {},cntfrq_el0", out(reg) frequency);
    let mut diagnostics = [0usize; 12];
    infinity_kokoro_native_diagnostics(diagnostics.as_mut_ptr());
    for value in [0x494e464c49464531, case as u64, kind, status as u64, frames as u64, ticks, frequency] {
        bytes(&value.to_le_bytes());
    }
    for value in diagnostics { bytes(&(value as u64).to_le_bytes()); }
    bytes(&hash.to_le_bytes());
    let mut memory_stats = [0usize; 7];
    infinity_kokoro_native_memory_stats(memory_stats.as_mut_ptr());
    for value in memory_stats { bytes(&(value as u64).to_le_bytes()); }
}

// ------------------------=
// FUNC: invalid_backend_callback
// DESC: Injects the captured impossible buffer pointer after the real native admission and fatal boundary are active.
// ------------------=
unsafe extern "C" fn invalid_backend_callback(_: usize) -> i32 {
    infinity_kokoro_ggml_backend_buffer_is_meta(0xffffffff14000400u64 as *const u8) as i32
}

// ------------------------=
// FUNC: speech_cases
// DESC: Alternates real recognition with repeated varied synthesis shapes, reporting the first failing transition without retries.
// ------------------=
unsafe extern "C" fn speech_cases(_: u64) {
    let mut memory = 0;
    let prepared = infinity_kokoro_native_prepare_recognition(&mut memory, 0, 0);
    report(0, 10, prepared, memory, 0, 0);
    if prepared != 0 { return; }
    for (i, pair) in INPUT.chunks_exact(2).enumerate() {
        core::ptr::write((&raw mut INPUT_PCM).cast::<i16>().add(i), i16::from_le_bytes([pair[0], pair[1]]));
    }
    let texts: [&[u8]; 6] = [
        b"Hi.",
        b"Hello! How can I assist you today?",
        b"I am doing well, thank you for asking. What would you like to talk about today?",
        b"Yes, I can.",
        b"I can explain the system, manage your files, or help you learn something new.",
        b"Please tell me more about that.",
    ];
    for round in 0..2 {
        let mut length = 0;
        let result = infinity_kokoro_native_recognize((&raw const INPUT_PCM).cast(), INPUT.len() / 2,
            (&raw mut TRANSCRIPT).cast(), 512, &mut length, &mut memory, 0, 0);
        report(round, 11, result, length, 0, 0);
        if result != 0 { return; }
        for (index, text) in texts.iter().enumerate() {
            let case = round * texts.len() + index;
            report(case, 0, 0, 0, 0, 0);
            let (start, end): (u64, u64);
            core::arch::asm!("mrs {},cntvct_el0", out(reg) start);
            let mut frames = 0;
            let result = native_synthesize(text.as_ptr(), text.len(), (&raw mut PCM).cast(), 720000, &mut frames, 0, 0);
            core::arch::asm!("mrs {},cntvct_el0", out(reg) end);
            let mut hash = 0xcbf29ce484222325u64;
            if result == 0 && frames <= 720000 {
                for byte in core::slice::from_raw_parts((&raw const PCM).cast::<u8>(), frames * 2) {
                    hash = (hash ^ *byte as u64).wrapping_mul(0x100000001b3);
                }
            }
            report(case, 1, result, frames, end - start, hash);
            if result != 0 { return; }
        }
    }
    assert!(!infinity_kokoro_ggml_backend_buffer_is_meta(core::ptr::null()));
    let mut frames = 123;
    core::ptr::write_bytes((&raw mut PCM).cast::<u8>(), 0x7f, 720000 * 2);
    let invalid = native_synthesize(b"Hello".as_ptr(), 5, (&raw mut PCM).cast(), 720000,
        &mut frames, invalid_backend_callback as *const () as usize, 0);
    let mut nonzero = 0u64;
    for sample in core::slice::from_raw_parts((&raw const PCM).cast::<i16>(), 720000) {
        nonzero |= *sample as u16 as u64;
    }
    report(12, 12, invalid, frames, 0, nonzero);
    frames = 123;
    let quarantined = native_synthesize(b"Hello".as_ptr(), 5, (&raw mut PCM).cast(), 720000,
        &mut frames, 0, 0);
    report(13, 13, quarantined, frames, 0, 0);
}
