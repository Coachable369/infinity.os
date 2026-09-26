#![no_std]
#[path = "../../../kernel/core/boot_info.rs"]
mod boot_info;
#[path = "../../../kernel/runtime/ai/qwen/workers.rs"]
mod workers;
use core::sync::atomic::{AtomicUsize, Ordering};
static DONE: AtomicUsize = AtomicUsize::new(0);
static mut PCM: [i16; 720000] = [0; 720000];
static mut RESULT: [u64; 7] = [0; 7];
static mut START: u64 = 0;
#[cfg(feature = "correctness")]
static mut CHECKPOINTS: u64 = 0;
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct Gate { low: u16, selector: u16, flags: u32, high: u32, zero: u32 }
static mut IDT: [Gate; 256] = [Gate { low: 0, selector: 24, flags: 0, high: 0, zero: 0 }; 256];
core::arch::global_asm!(
    ".global probe_fault", "probe_fault:",
    "mov rax, [rsp + 8]", "mov dx, 0xe9", "mov ecx, 8",
    "2:", "out dx, al", "shr rax, 8", "loop 2b",
    "mov dx, 0xf4", "mov eax, 9", "out dx, eax", "cli", "3:", "hlt", "jmp 3b"
);
unsafe extern "C" {
    static infinity_kokoro_native_init_start: u8;
    static infinity_kokoro_native_init_end: u8;
    fn infinity_kokoro_ggml_cpu_init();
    fn infinity_kokoro_ggml_vec_dot_f16(n: i32, out: *mut f32, bs: usize,
        x: *const u16, bx: usize, y: *const u16, by: usize, rows: i32);
    fn infinity_kokoro_ggml_vec_dot_f32(n: i32, out: *mut f32, bs: usize,
        x: *const f32, bx: usize, y: *const f32, by: usize, rows: i32);
    fn infinity_kokoro_native_synthesize(text: *const u8, length: usize, pcm: *mut i16,
        capacity: usize, frames: *mut usize, cancel: usize, context: usize) -> i32;
    fn infinity_kokoro_native_diagnostics(out: *mut usize);
    fn infinity_kokoro_native_profile_read(out: *mut u64);
    fn infinity_kokoro_native_f16c_available() -> i32;
    fn infinity_kokoro_native_dot4(kind: i32, n: i32, out: *mut f32,
        x: *const u8, stride: usize, y: *const u8) -> i32;
}
// ------------------------=
// FUNC: finish
// DESC: Returns a binary status from the isolated guest without using a host runtime.
// ------------------=
fn finish(code: u32) -> ! {
    unsafe { core::arch::asm!("out dx, eax", in("dx") 0xf4u16, in("eax") code); }
    loop { core::hint::spin_loop(); }
}
// ------------------------=
// FUNC: panic
// DESC: Makes any failed guest assertion a non-success exit status.
// ------------------=
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { finish(1) }
// ------------------------=
// FUNC: cancel
// DESC: Exercises immediate cancellation through the real native provider ABI.
// ------------------=
extern "C" fn cancel(_: usize) -> i32 { 1 }
// ------------------------=
// FUNC: deadline
// DESC: Enforces the production deadline or a separately selected, bounded emulation-correctness work budget.
// ------------------=
extern "C" fn deadline(_: usize) -> i32 {
    // Cross-ISA correctness is deliberately separate from deadline acceptance.
    // Bound work and expose binary checkpoints; the host also bounds wall time.
    #[cfg(feature = "correctness")]
    unsafe {
        CHECKPOINTS += 1;
        if CHECKPOINTS % 64 == 0 {
            for byte in CHECKPOINTS.to_le_bytes() {
                core::arch::asm!("out dx, al", in("dx") 0x505u16, in("al") byte);
            }
        }
        return (CHECKPOINTS >= 8192) as i32;
    }
    #[cfg(not(feature = "correctness"))]
    unsafe { (workers::clock_ns() - START >= 90_000_000_000) as i32 }
}
// ------------------------=
// FUNC: verify_dots
// DESC: Checks exact vector sums across SIMD boundaries and unaligned rows in the real guest.
// ------------------=
unsafe fn verify_dots() {
    let mut half = [0u16; 272];
    let mut float = [0f32; 272];
    for i in 0..272 {
        half[i] = if i % 2 == 0 { 0x3c00 } else { 0xc000 };
        float[i] = if i % 2 == 0 { 1.0 } else { -2.0 };
    }
    #[repr(align(16))]
    struct Aligned<T>(T);
    let mut tiled_half = Aligned([0u16; 5 * 512]);
    let mut tiled_float = Aligned([0f32; 5 * 512]);
    for i in 0..5 * 512 {
        tiled_half.0[i] = 0x3000 + ((i * 173) % 0x1800) as u16;
        tiled_float.0[i] = ((i * 173 % 1024) as f32 - 512.0) / 73.0;
    }
    for n in [32usize, 64, 128, 256, 512] {
        for kind in [0, 1] {
            let width = if kind == 1 { 2 } else { 4 };
            let x = if kind == 1 { tiled_half.0.as_ptr().cast::<u8>() } else { tiled_float.0.as_ptr().cast::<u8>() };
            let y = x.add(4 * 512 * width);
            let mut actual = [0f32; 4];
            assert_eq!(infinity_kokoro_native_dot4(kind, n as i32, actual.as_mut_ptr(), x, 512 * width, y), 1);
            for row in 0..4 {
                let mut expected = 0.0;
                if kind == 1 {
                    infinity_kokoro_ggml_vec_dot_f16(n as i32, &mut expected, 0,
                        x.add(row * 512 * width).cast(), 0, y.cast(), 0, 1);
                } else {
                    infinity_kokoro_ggml_vec_dot_f32(n as i32, &mut expected, 0,
                        x.add(row * 512 * width).cast(), 0, y.cast(), 0, 1);
                }
                assert_eq!(actual[row].to_bits(), expected.to_bits());
            }
        }
    }
    let x = tiled_half.0.as_ptr().cast::<u8>();
    let mut untouched = [123.0f32; 4];
    for (kind, length, offset, stride) in [(1, 16, 0, 1024), (1, 31, 0, 1024),
        (2, 32, 0, 1024), (1, 32, 2, 1024), (1, 32, 0, 1026)] {
        assert_eq!(infinity_kokoro_native_dot4(kind, length, untouched.as_mut_ptr(),
            x.add(offset), stride, x), 0);
        assert_eq!(untouched, [123.0; 4]);
    }
    for length in [1usize, 7, 31, 32, 33, 63, 64, 65, 127, 256] {
        for offset in 0..8 {
            let mut expected = 0.0;
            for i in offset..offset + length { expected += float[i] * float[i + 1]; }
            let mut actual = 0.0;
            infinity_kokoro_ggml_vec_dot_f16(length as i32, &mut actual, 0,
                half.as_ptr().add(offset), 0, half.as_ptr().add(offset + 1), 0, 1);
            assert_eq!(actual, expected);
            infinity_kokoro_ggml_vec_dot_f32(length as i32, &mut actual, 0,
                float.as_ptr().add(offset), 0, float.as_ptr().add(offset + 1), 0, 1);
            assert_eq!(actual, expected);
        }
    }
}
// ------------------------=
// FUNC: speech_job
// DESC: Verifies cancellation then synthesizes real PCM on a production background worker.
// ------------------=
unsafe fn speech_job() {
    unsafe extern "C" { fn probe_fault(); }
    let address = probe_fault as *const () as u64;
    for i in 0..256 {
        IDT[i] = Gate { low: address as u16, selector: 24,
            flags: 0x8e00 | ((address as u32 >> 16) << 16), high: (address >> 32) as u32, zero: 0 };
    }
    #[repr(C, packed)]
    struct Descriptor { size: u16, address: u64 }
    let descriptor = Descriptor { size: 4095, address: (&raw const IDT) as u64 };
    core::arch::asm!("lidt [{}]", in(reg) &descriptor, options(readonly, nostack));
    #[cfg(feature = "dot-only")]
    {
        let mut cursor = (&raw const infinity_kokoro_native_init_start).cast::<unsafe extern "C" fn()>();
        let end = (&raw const infinity_kokoro_native_init_end).cast::<unsafe extern "C" fn()>();
        while cursor != end { (*cursor)(); cursor = cursor.add(1); }
        infinity_kokoro_ggml_cpu_init();
        verify_dots();
        for byte in 205u64.to_le_bytes() {
            core::arch::asm!("out dx, al", in("dx") 0xe9u16, in("al") byte);
        }
        for byte in (infinity_kokoro_native_f16c_available() as u64).to_le_bytes() {
            core::arch::asm!("out dx, al", in("dx") 0xe9u16, in("al") byte);
        }
        finish(16);
    }
    let mut frames = 0;
    let pcm = (&raw mut PCM).cast();
    assert_eq!(infinity_kokoro_native_synthesize(b"Hi.".as_ptr(), 3, pcm, 720000,
        &mut frames, cancel as *const () as usize, 0), 2);
    assert_eq!(frames, 0);
    let begin = workers::clock_ns();
    START = begin;
    let status = infinity_kokoro_native_synthesize(b"Hi.".as_ptr(), 3, pcm, 720000,
        &mut frames, deadline as *const () as usize, 0);
    if status == 0 { verify_dots(); }
    let mut diagnostics = [0usize; 12];
    infinity_kokoro_native_diagnostics(diagnostics.as_mut_ptr());
    RESULT = [1, status as u64, frames as u64, workers::clock_ns() - begin,
              diagnostics[1] as u64, diagnostics[2] as u64, diagnostics[0] as u64];
    DONE.store(1, Ordering::Release);
}
// ------------------------=
// FUNC: infinity_kernel_entry
// DESC: Uses the production loader and worker scheduler while the BSP remains independently responsive.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_kernel_entry(info: &boot_info::BootInfo) -> ! {
    assert_eq!(info.magic, boot_info::BOOT_MAGIC);
    assert_eq!(info.version, boot_info::BOOT_VERSION);
    workers::initialize(info.worker_bridge);
    let begin = workers::clock_ns();
    assert!(begin > 0);
    while workers::online() == 0 && workers::clock_ns() - begin < 1_000_000_000 {
        core::hint::spin_loop();
    }
    assert!(workers::background(speech_job));
    let mut heartbeat = 0u64;
    while DONE.load(Ordering::Acquire) == 0 {
        heartbeat += 1;
        #[cfg(not(feature = "correctness"))]
        assert!(workers::clock_ns() - begin < 180_000_000_000);
        core::hint::spin_loop();
    }
    let result = RESULT;
    for value in result.into_iter().chain([heartbeat]) {
        for byte in value.to_le_bytes() {
            core::arch::asm!("out dx, al", in("dx") 0xe9u16, in("al") byte);
        }
    }
    let mut profile = [0u64; 256];
    infinity_kokoro_native_profile_read(profile.as_mut_ptr());
    for value in profile {
        for byte in value.to_le_bytes() {
            core::arch::asm!("out dx, al", in("dx") 0xe9u16, in("al") byte);
        }
    }
    assert_eq!(result[1], 0);
    assert!((2400..=720000).contains(&result[2]));
    for sample in core::slice::from_raw_parts((&raw const PCM).cast::<i16>(), result[2] as usize) {
        for byte in sample.to_le_bytes() {
            core::arch::asm!("out dx, al", in("dx") 0xe9u16, in("al") byte);
        }
    }
    finish(16)
}
