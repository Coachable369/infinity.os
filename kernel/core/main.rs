#![no_std]
#![no_main]

mod boot_info;
mod bootstrap;
mod console;
#[path = "../drivers/mod.rs"]
mod drivers;
mod intent;
#[cfg(target_arch = "x86")]
#[path = "../arch/x86/output.rs"]
mod output;
#[cfg(target_arch = "x86_64")]
#[path = "../arch/x86_64/output.rs"]
mod output;
#[cfg(target_arch = "aarch64")]
#[path = "../arch/aarch64/output.rs"]
mod output;
#[path = "../runtime/mod.rs"]
mod runtime;
#[path = "../storage/mod.rs"]
mod storage;
#[path = "../ui/mod.rs"]
mod ui;
mod system;

use boot_info::{BootInfo, BOOT_MAGIC, BOOT_VERSION};
use core::{ffi::c_void, panic::PanicInfo};

#[no_mangle]
// ------------------------=
// FUNC: memcpy
// DESC: Implements the memcpy operation.
// ------------------=
pub unsafe extern "C" fn memcpy(
    destination: *mut c_void,
    source: *const c_void,
    count: usize,
) -> *mut c_void {
    let destination_bytes = destination.cast::<u8>();
    let source_bytes = source.cast::<u8>();
    for index in 0..count {
        core::ptr::write_volatile(
            destination_bytes.add(index),
            core::ptr::read_volatile(source_bytes.add(index)),
        );
    }
    destination
}

#[no_mangle]
// ------------------------=
// FUNC: memmove
// DESC: Copies overlapping freestanding memory ranges in the safe direction.
// ------------------=
pub unsafe extern "C" fn memmove(
    destination: *mut c_void,
    source: *const c_void,
    count: usize,
) -> *mut c_void {
    let destination_bytes = destination.cast::<u8>();
    let source_bytes = source.cast::<u8>();
    if (destination_bytes as usize) <= (source_bytes as usize) {
        for index in 0..count {
            core::ptr::write_volatile(
                destination_bytes.add(index),
                core::ptr::read_volatile(source_bytes.add(index)),
            );
        }
    } else {
        for index in (0..count).rev() {
            core::ptr::write_volatile(
                destination_bytes.add(index),
                core::ptr::read_volatile(source_bytes.add(index)),
            );
        }
    }
    destination
}

#[no_mangle]
// ------------------------=
// FUNC: memset
// DESC: Implements the memset operation.
// ------------------=
pub unsafe extern "C" fn memset(destination: *mut c_void, value: i32, count: usize) -> *mut c_void {
    let destination_bytes = destination.cast::<u8>();
    for index in 0..count {
        core::ptr::write_volatile(destination_bytes.add(index), value as u8);
    }
    destination
}

#[no_mangle]
// ------------------------=
// FUNC: memcmp
// DESC: Implements the memcmp operation.
// ------------------=
pub unsafe extern "C" fn memcmp(left: *const c_void, right: *const c_void, count: usize) -> i32 {
    let left = left.cast::<u8>();
    let right = right.cast::<u8>();
    for index in 0..count {
        let a = core::ptr::read(left.add(index));
        let b = core::ptr::read(right.add(index));
        if a != b {
            return a as i32 - b as i32;
        }
    }
    0
}

#[no_mangle]
// ------------------------=
// FUNC: infinity_kernel_entry
// DESC: Implements the infinity kernel entry operation.
// ------------------=
pub extern "C" fn infinity_kernel_entry(info: *const BootInfo) -> ! {
    let valid = !info.is_null()
        && unsafe {
            (*info).magic == BOOT_MAGIC
                && (*info).version == BOOT_VERSION
                && (*info).architecture == expected_architecture()
                && (*info).memory_descriptor_size != 0
        };

    if !valid {
        unsafe {
            output::initialize(false);
        }
        unsafe {
            output::write(b"InfinityOS\nERROR: invalid BootInfo\n");
        }
        output::idle();
    }

    let info = unsafe { &*info };
    unsafe {
        output::initialize(info.boot_flags & 1 != 0);
    }
    unsafe {
        output::write(b"InfinityOS\nKernel online.\n");
    }
    let devices = drivers::initialize(info);
    runtime::initialize();
    storage::initialize_object_store();
    runtime::storage_initialized();
    runtime::announce_services();
    console::initialize(system::SystemSnapshot::new(info, devices));
    drivers::input::run()
}

// ------------------------=
// FUNC: output_text
// DESC: Implements the output text operation.
// ------------------=
pub fn output_text(bytes: &[u8]) {
    unsafe {
        output::write(bytes);
    }
}

#[cfg(target_arch = "x86_64")]
// ------------------------=
// FUNC: expected_architecture
// DESC: Implements the expected architecture operation.
// ------------------=
const fn expected_architecture() -> u32 {
    boot_info::ARCH_X86_64
}

#[cfg(target_arch = "x86")]
// ------------------------=
// FUNC: expected_architecture
// DESC: Implements the expected architecture operation.
// ------------------=
const fn expected_architecture() -> u32 {
    boot_info::ARCH_X86
}

#[cfg(target_arch = "aarch64")]
// ------------------------=
// FUNC: expected_architecture
// DESC: Implements the expected architecture operation.
// ------------------=
const fn expected_architecture() -> u32 {
    boot_info::ARCH_AARCH64
}

#[panic_handler]
// ------------------------=
// FUNC: panic
// DESC: Implements the panic operation.
// ------------------=
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        output::write(b"InfinityOS\nERROR: kernel panic\n");
    }
    output::idle()
}
