#![no_std]
#![no_main]

// Shared no-allocator TCP/HTTP mechanisms for the native service adapter.
pub use infinity_http as http_transport;

mod boot_info;
mod bootstrap;
mod console;
mod crash;
#[path = "../drivers/mod.rs"]
mod drivers;
mod intent;
mod install_boot;
mod memory;
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
mod system;
#[path = "../ui/mod.rs"]
mod ui;

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
    memory::copy(destination.cast(), source.cast(), count);
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
    memory::move_bytes(destination.cast(), source.cast(), count);
    destination
}

#[no_mangle]
// ------------------------=
// FUNC: memset
// DESC: Implements the memset operation.
// ------------------=
pub unsafe extern "C" fn memset(destination: *mut c_void, value: i32, count: usize) -> *mut c_void {
    memory::fill(destination.cast(), value as u8, count);
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
    crash::register_boot_info(info);
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
        crash::fatal(
            crash::CrashReason::InvalidBootInformation,
            b"BOOTINFO CONTRACT VALIDATION FAILED",
        );
    }

    let info = unsafe { &*info };
    storage::payload::initialize(info.payload_bridge);
    unsafe {
        output::initialize(info.boot_flags & 1 != 0);
    }
    unsafe {
        output::write(b"InfinityOS\nKernel online.\n");
    }
    crash::set_phase(crash::CrashPhase::Drivers);
    let devices = drivers::initialize(info);
    crash::set_phase(crash::CrashPhase::Runtime);
    if info.native_pool_address != 0 {
        let _ = runtime::native_memory::initialize(info.native_pool_address, info.native_pool_bytes);
    }
    runtime::initialize(cfg!(target_arch = "x86") || info.boot_flags & 32 == 0);
    storage::initialize_installation_identity(
        &info.firmware_entropy,
        info.firmware_entropy_valid == 1,
    );
    let _ =
        runtime::initialize_node_identity(&info.firmware_entropy, info.firmware_entropy_valid == 1);
    if info.network_device_count > 0 {
        let mut hardware_address = [0u8; 6];
        hardware_address.copy_from_slice(&info.network_mac[..6]);
        let _ = runtime::register_firmware_network_device(
            runtime::network::types::FirmwareNetworkDevice {
                firmware_handle: info.firmware_network,
                device_id: if hardware_address == [0; 6] {
                    0x5545_4649_4e45_5430
                } else {
                    u64::from_le_bytes([
                        hardware_address[0],
                        hardware_address[1],
                        hardware_address[2],
                        hardware_address[3],
                        hardware_address[4],
                        hardware_address[5],
                        0,
                        0,
                    ])
                },
                hardware_address: if info.network_mac_length >= 6 {
                    Some(hardware_address)
                } else {
                    None
                },
                link_state: match info.network_link_state {
                    2 => runtime::network::types::LinkState::Up,
                    1 => runtime::network::types::LinkState::Down,
                    _ => runtime::network::types::LinkState::Unknown,
                },
                maximum_frame_size: info.network_mtu.min(u16::MAX as u32) as u16,
                can_receive: info.network_capabilities & 1 != 0,
                can_transmit: info.network_capabilities & 2 != 0,
            },
        );
    }
    crash::set_phase(crash::CrashPhase::Storage);
    storage::initialize_object_store();
    runtime::storage_initialized();
    drivers::network::initialize(info);
    crash::set_phase(crash::CrashPhase::Services);
    runtime::announce_services();
    crash::set_phase(crash::CrashPhase::UserInterface);
    if info.model_address != 0
        && matches!(info.model_bytes, 5_027_783_488 | 7_174_806_496)
        && info.model_work_address != 0
        && info.model_work_bytes >= 1280 * 1024 * 1024
    {
        // SAFETY: the boot ABI owns these dedicated non-overlapping allocations;
        // firmware reserves them and they are handed to the AI service once.
        let model = unsafe {
            core::slice::from_raw_parts(info.model_address as *const u8, info.model_bytes as usize)
        };
        let arena = unsafe {
            core::slice::from_raw_parts_mut(
                info.model_work_address as *mut u8,
                info.model_work_bytes as usize,
            )
        };
        let ready = runtime::ai::with_ai_runtime(|ai| {
            let (qwen, second) = model.split_at(5_027_783_488);
            let (qwen_arena, second_arena) = arena.split_at_mut(1280 * 1024 * 1024);
            let ready = ai.load_qwen(qwen, qwen_arena);
            if !second.is_empty() { ai.load_ministral(second, second_arena); }
            ready
        });
        if ready {
            output_text(if info.worker_bridge == 0 {
                b"[AI] MP startup bridge unavailable; single-core fallback\n"
            } else { b"[AI] MP startup bridge discovered\n" });
            unsafe { runtime::ai::qwen::workers::initialize(info.worker_bridge); }
        }
        output_text(if ready {
            b"[AI] native Qwen3-8B verified and ready\n"
        } else {
            b"[AI] native Qwen3-8B verification failed\n"
        });
    }
    console::initialize(system::SystemSnapshot::new(info, devices));
    crash::set_phase(crash::CrashPhase::Input);
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
fn panic(info: &PanicInfo) -> ! {
    crash::fatal_panic(info)
}
