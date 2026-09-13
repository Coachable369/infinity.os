#![no_std]
#[path = "../../kernel/runtime/native_c_image.rs"]
mod image;
#[path = "../../kernel/core/memory.rs"]
mod memory;
#[repr(align(4096))]
struct Arena([u8; image::MAX_IMAGE]);
static mut ARENA: Arena = Arena([0; image::MAX_IMAGE]);
static mut TLS_A: [u8; 1024] = [0; 1024];
static mut TLS_B: [u8; 1024] = [0; 1024];

// ------------------------=
// FUNC: finish
// DESC: Returns structured success or failure through the emulator debug-exit device.
// ------------------=
fn finish(code: u32) -> ! {
    unsafe {
        core::arch::asm!("out dx, eax", in("dx") 0xf4u16, in("eax") code);
    }
    loop {
        core::hint::spin_loop();
    }
}
#[panic_handler]
// ------------------------=
// FUNC: panic
// DESC: Converts any failed assertion into a failing guest exit status.
// ------------------=
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    finish(0x12)
}

#[no_mangle]
// ------------------------=
// FUNC: memcpy
// DESC: Supplies the production freestanding memory copy for the isolated test kernel.
// ------------------=
pub unsafe extern "C" fn memcpy(
    d: *mut core::ffi::c_void,
    s: *const core::ffi::c_void,
    n: usize,
) -> *mut core::ffi::c_void {
    memory::copy(d.cast(), s.cast(), n);
    d
}
#[no_mangle]
// ------------------------=
// FUNC: memmove
// DESC: Supplies the production overlapping copy for the isolated test kernel.
// ------------------=
pub unsafe extern "C" fn memmove(
    d: *mut core::ffi::c_void,
    s: *const core::ffi::c_void,
    n: usize,
) -> *mut core::ffi::c_void {
    memory::move_bytes(d.cast(), s.cast(), n);
    d
}
#[no_mangle]
// ------------------------=
// FUNC: memset
// DESC: Supplies the production fill operation for the isolated test kernel.
// ------------------=
pub unsafe extern "C" fn memset(
    d: *mut core::ffi::c_void,
    value: i32,
    n: usize,
) -> *mut core::ffi::c_void {
    memory::fill(d.cast(), value as u8, n);
    d
}
#[no_mangle]
// ------------------------=
// FUNC: memcmp
// DESC: Compares bounded memory ranges used by the checked loader.
// ------------------=
pub unsafe extern "C" fn memcmp(
    a: *const core::ffi::c_void,
    b: *const core::ffi::c_void,
    n: usize,
) -> i32 {
    for i in 0..n {
        let delta = a.cast::<u8>().add(i).read() as i32 - b.cast::<u8>().add(i).read() as i32;
        if delta != 0 {
            return delta;
        }
    }
    0
}
#[no_mangle]
// ------------------------=
// FUNC: infinity_kernel_entry
// DESC: Executes real Clang local-exec TLS under alternating hardware FS bases and verifies restored per-thread values.
// ------------------=
pub extern "C" fn infinity_kernel_entry(_: *const u8) -> ! {
    unsafe {
        core::arch::asm!("cli", options(nostack));
        let arena = &mut (*(&raw mut ARENA)).0;
        let program = include_bytes!("../../build/native-c/tls-x86_64.elf");
        let loaded = image::Image::load(program, 62, arena).unwrap();
        let template = loaded.tls.unwrap();
        let first = template
            .initialize(program, 62, &mut *(&raw mut TLS_A))
            .unwrap();
        let second = template
            .initialize(program, 62, &mut *(&raw mut TLS_B))
            .unwrap();
        let entry: extern "C" fn(u32) -> i32 =
            core::mem::transmute(arena.as_ptr().add(loaded.entry));
        let previous = image::tls::replace_thread_pointer(first.thread_pointer());
        let mut failed = false;
        for turn in 0..64 {
            image::tls::replace_thread_pointer(first.thread_pointer());
            failed |= entry(turn) != 0;
            image::tls::replace_thread_pointer(second.thread_pointer());
            failed |= entry(turn) != 0;
        }
        image::tls::replace_thread_pointer(previous);
        let observed = image::tls::replace_thread_pointer(previous);
        assert_eq!(observed, previous);
        assert!(!failed);
    }
    finish(0x10)
}
