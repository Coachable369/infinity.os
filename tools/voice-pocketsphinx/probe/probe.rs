#![no_std]
static INPUT: &[u8] = include_bytes!(env!("INFINITY_STT_FIXTURE"));
static mut PCM: [i16; 160000] = [0; 160000];
unsafe extern "C" {
    fn infinity_stt_native_recognize(
        pcm: *const i16,
        samples: usize,
        text: *mut u8,
        capacity: usize,
        length: *mut usize,
        memory: *mut usize,
        cancel: usize,
    ) -> i32;
    fn infinity_stt_native_memory_state(live: *mut usize, erased: *mut usize);
}
static mut CANCEL_CALLS: usize = 0;
// ------------------------=
// FUNC: cancel
// DESC: Cancels after initialization and bounded utterance processing, before transcript publication.
// ------------------=
unsafe extern "C" fn cancel() -> i32 {
    CANCEL_CALLS += 1;
    (CANCEL_CALLS >= 3) as i32
}
core::arch::global_asm!(
    ".section .text.entry",
    ".global _start",
    "_start:",
    "ldr x0, =0x5f000000",
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
// DESC: Emits binary guest evidence through PL011 without host inference.
// ------------------=
unsafe fn bytes(data: &[u8]) {
    for &b in data {
        while core::ptr::read_volatile(0x09000018 as *const u32) & 32 != 0 {}
        core::ptr::write_volatile(0x09000000 as *mut u32, b as u32);
    }
}
// ------------------------=
// FUNC: finish
// DESC: Stops only the test machine via PSCI.
// ------------------=
fn finish() -> ! {
    unsafe {
        core::arch::asm!("hvc #0",in("x0")0x84000008u64);
    }
    loop {
        core::hint::spin_loop();
    }
}
// ------------------------=
// FUNC: exception
// DESC: Reports native architectural failures as structured binary fields.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn exception() -> ! {
    let esr: u64;
    let elr: u64;
    let far: u64;
    core::arch::asm!("mrs {},esr_el1",out(reg)esr);
    core::arch::asm!("mrs {},elr_el1",out(reg)elr);
    core::arch::asm!("mrs {},far_el1",out(reg)far);
    for v in [0, esr, elr, far] {
        bytes(&v.to_le_bytes());
    }
    finish()
}
// ------------------------=
// FUNC: panic
// DESC: Reports a failed native probe assertion without relying on log text.
// ------------------=
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe {
        bytes(&[0; 32]);
    }
    finish()
}
// ------------------------=
// FUNC: probe
// DESC: Feeds real recorded English PCM into the native recognizer and emits its actual transcript and timing.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn probe() -> ! {
    for (i, p) in INPUT.chunks_exact(2).enumerate() {
        PCM[i] = i16::from_le_bytes([p[0], p[1]]);
    }
    let mut baseline = 0;
    for case in 0..6 {
        if case == 4 {
            (&mut *(&raw mut PCM)).fill(0);
        }
        let mut text = [0u8; 512];
        let mut length = 0;
        let mut memory = 0;
        let mut live = 0;
        let mut erased = 0;
        let start: u64;
        let end: u64;
        let frequency: u64;
        core::arch::asm!("mrs {},cntvct_el0",out(reg)start);
        let result = infinity_stt_native_recognize(
            (&raw const PCM).cast(),
            INPUT.len() / 2,
            text.as_mut_ptr(),
            if case == 3 { 2 } else { text.len() },
            &mut length,
            &mut memory,
            if case == 2 || case == 5 {
                cancel as *const () as usize
            } else {
                0
            },
        );
        core::arch::asm!("mrs {},cntvct_el0",out(reg)end);
        core::arch::asm!("mrs {},cntfrq_el0",out(reg)frequency);
        infinity_stt_native_memory_state(&mut live, &mut erased);
        for v in [2, case, result as u64, length as u64, memory as u64,
            end - start, frequency, live as u64, erased as u64] {
            bytes(&v.to_le_bytes());
        }
        bytes(&text[..length]);
        if case == 0 {
            baseline = live;
        } else {
            assert_eq!(live, baseline);
        }
        assert_eq!(
            result,
            match case {
                2 | 5 => 2,
                3 => 6,
                4 => 5,
                _ => 0,
            }
        );
        if case >= 2 {
            assert_eq!(length, 0);
            assert_eq!(text[0], 0);
        }
        assert!(erased > 0);
    }
    finish()
}
