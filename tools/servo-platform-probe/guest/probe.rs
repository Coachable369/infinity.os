#![no_std]
#[cfg(feature = "std-probe")]
extern crate std;
use core::{alloc::Layout, ptr::NonNull};
use infinity_servo_runtime_primitives::arena::Arena;
#[cfg(feature = "context-probe")]
mod context_probe;
#[cfg(feature = "executor-probe")]
mod executor_probe;
#[cfg(feature = "std-probe")]
mod std_probe;
#[cfg(feature = "std-probe")]
mod dns_probe;
#[cfg(feature = "socket-probe")]
mod socket_probe;
#[cfg(feature = "async-probe")]
mod entropy_probe;

#[repr(align(4096))]
struct Memory([u8; 4096]);
static mut MEMORY: Memory = Memory([0; 4096]);

#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(
    ".section .text.entry", ".global _start", "_start:",
    "ldr x0, =0x47f00000", "mov sp, x0", "mov x0, #(3 << 20)",
    "msr cpacr_el1, x0", "isb", "bl probe_memory", "bl probe", "b ."
);
#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(include_str!("aarch64-memory.S"));

// ------------------------=
// FUNC: infinity_kernel_entry
// DESC: Runs the shared probe on the native x86 UEFI handoff stack with floating-point state enabled.
// ------------------=
#[cfg(target_arch = "x86_64")]
#[no_mangle]
pub unsafe extern "C" fn infinity_kernel_entry(_: *const u8) -> ! {
    core::arch::asm!("mov rax, cr0", "and rax, -13", "or rax, 2", "mov cr0, rax",
        "mov rax, cr4", "or rax, 1536", "mov cr4, rax", "fninit", out("rax") _);
    probe()
}

// ------------------------=
// FUNC: finish
// DESC: Emits a structured result and powers down only this disposable test guest.
// ------------------=
fn finish(status: u64, allocations: u64, capacity: u64) -> ! {
    unsafe {
        #[cfg(target_arch = "aarch64")]
        {
        let version = if cfg!(feature = "async-probe") { 7 } else if cfg!(feature = "socket-probe") { 6 } else if cfg!(feature = "mio-probe") { 5 } else if cfg!(feature = "std-probe") { 4 } else if cfg!(feature = "executor-probe") { 3 } else if cfg!(feature = "context-probe") { 2 } else { 1 };
        for value in [version, status, allocations, capacity] {
            for byte in value.to_le_bytes() {
                while core::ptr::read_volatile(0x09000018 as *const u32) & 32 != 0 {}
                core::ptr::write_volatile(0x09000000 as *mut u32, byte as u32);
            }
        }
        core::arch::asm!("hvc #0", in("x0") 0x84000008u64);
        }
        #[cfg(target_arch = "x86_64")]
        {
            let expected = if cfg!(feature = "context-probe") { 2002 } else { 65 };
            let code = if status == 0 && allocations == expected && capacity == 4096 { 0x10u32 } else { 0x12 };
            core::arch::asm!("out dx, eax", in("dx") 0xf4u16, in("eax") code);
        }
    }
    loop { core::hint::spin_loop(); }
}

// ------------------------=
// FUNC: probe
// DESC: Executes native allocations, writes, exhaustion and coalescing without any host allocator.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn probe() -> ! {
    #[cfg(feature = "std-probe")]
    std_probe::initialize();
    let bytes = core::slice::from_raw_parts_mut(core::ptr::addr_of_mut!(MEMORY.0).cast::<u8>(), 4096);
    let mut arena = Arena::new(bytes).unwrap();
    let layout = Layout::from_size_align(23, 64).unwrap();
    let mut pointers = [NonNull::dangling(); 64];
    for (index, slot) in pointers.iter_mut().enumerate() {
        *slot = arena.allocate(layout).unwrap();
        assert_eq!(slot.as_ptr() as usize % 64, 0);
        for offset in 0..23 { slot.as_ptr().add(offset).write_volatile(index as u8); }
    }
    assert!(arena.allocate(layout).is_none());
    assert_eq!(arena.allocated(), 4096);
    for (index, pointer) in pointers.iter().enumerate() {
        for offset in 0..23 { assert_eq!(pointer.as_ptr().add(offset).read_volatile(), index as u8); }
    }
    for step in 0..64 { arena.release(pointers[(step * 17) % 64], layout); }
    assert_eq!(arena.allocated(), 0);
    let whole = Layout::from_size_align(4096, 4096).unwrap();
    let pointer = arena.allocate(whole).unwrap();
    assert_eq!(pointer.as_ptr() as usize % 4096, 0);
    arena.release(pointer, whole);
    assert_eq!(arena.allocated(), 0);
    #[cfg(feature = "context-probe")]
    let operations = context_probe::run();
    #[cfg(feature = "executor-probe")]
    executor_probe::run();
    #[cfg(feature = "std-probe")]
    std_probe::run();
    #[cfg(not(feature = "context-probe"))]
    let operations = 65;
    finish(0, operations, arena.capacity() as u64)
}

// ------------------------=
// FUNC: panic
// DESC: Makes a failed runtime assertion observable as a non-success binary result.
// ------------------=
#[panic_handler]
#[cfg(not(feature = "std-probe"))]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { finish(1, 0, 0) }

// ------------------------=
// FUNC: memset
// DESC: Supplies freestanding byte initialization without a host C runtime.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn memset(output: *mut core::ffi::c_void, value: i32, size: usize) -> *mut core::ffi::c_void {
    for index in 0..size { output.cast::<u8>().add(index).write_volatile(value as u8); }
    output
}
// ------------------------=
// FUNC: memcpy
// DESC: Supplies nonoverlapping byte copies without a host C runtime.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn memcpy(output: *mut core::ffi::c_void, input: *const core::ffi::c_void, size: usize) -> *mut core::ffi::c_void {
    for index in 0..size { output.cast::<u8>().add(index).write_volatile(input.cast::<u8>().add(index).read_volatile()); }
    output
}
