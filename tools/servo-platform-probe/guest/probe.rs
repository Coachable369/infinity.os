#![no_std]
use core::{alloc::Layout, ptr::NonNull};
use infinity_servo_runtime_primitives::arena::Arena;

#[repr(align(4096))]
struct Memory([u8; 4096]);
static mut MEMORY: Memory = Memory([0; 4096]);

core::arch::global_asm!(
    ".section .text.entry", ".global _start", "_start:",
    "ldr x0, =0x41f00000", "mov sp, x0", "mov x0, #(3 << 20)",
    "msr cpacr_el1, x0", "isb", "bl probe", "b ."
);

// ------------------------=
// FUNC: finish
// DESC: Emits a structured result and powers down only this disposable test guest.
// ------------------=
fn finish(status: u64, allocations: u64, capacity: u64) -> ! {
    unsafe {
        for value in [1u64, status, allocations, capacity] {
            for byte in value.to_le_bytes() {
                while core::ptr::read_volatile(0x09000018 as *const u32) & 32 != 0 {}
                core::ptr::write_volatile(0x09000000 as *mut u32, byte as u32);
            }
        }
        core::arch::asm!("hvc #0", in("x0") 0x84000008u64);
    }
    loop { core::hint::spin_loop(); }
}

// ------------------------=
// FUNC: probe
// DESC: Executes native allocations, writes, exhaustion and coalescing without any host allocator.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn probe() -> ! {
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
    finish(0, 65, arena.capacity() as u64)
}

// ------------------------=
// FUNC: panic
// DESC: Makes a failed runtime assertion observable as a non-success binary result.
// ------------------=
#[panic_handler]
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
