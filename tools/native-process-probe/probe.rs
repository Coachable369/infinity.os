#[path = "../../kernel/core/boot_info.rs"] mod boot_info;
#[path = "../../kernel/core/memory.rs"] mod memory;
#[path = "../../kernel/runtime/native_c_image.rs"] mod native_c_image;
#[path = "../../kernel/runtime/native_memory.rs"] mod native_memory;
#[path = "../../kernel/runtime/native_task_x86.rs"] mod task;
use task as native_task_x86;
#[path = "../../kernel/runtime/execution.rs"] mod execution;
#[path = "../../kernel/runtime/native_process.rs"] mod native_process;
use native_memory::*;
#[path = "services.rs"] mod services;
#[path = "../../kernel/runtime/native_mappings.rs"] mod native_mappings;
#[path = "compiler.rs"] mod compiler;
#[path = "../../kernel/runtime/native_cpu.rs"] mod cpu;
static IMAGE: &[u8] = include_bytes!(env!("INFINITY_PROCESS_IMAGE"));
static mut STATE: task::ExtendedState = task::ExtendedState([0; 65536]);
static mut SENTINEL: u64 = 0x1020304050607080;

// ------------------------=
// FUNC: finish
// DESC: Reports a structured guest result through the emulator exit device, not through log matching.
// ------------------=
fn finish_code(code: u32) -> ! {
    unsafe { core::arch::asm!("out dx, eax", in("dx") 0xf4u16, in("eax") code); }
    loop { core::hint::spin_loop(); }
}

// ------------------------=
// FUNC: finish
// DESC: Completes a disposable compiler probe with the shared structured result convention.
// ------------------=
fn finish() -> ! { finish_code(16) }

// ------------------------=
// FUNC: emit
// DESC: Emits binary evidence through the disposable x86 debug console.
// ------------------=
fn emit(value: u64) {
    for byte in value.to_le_bytes() { unsafe {
        core::arch::asm!("out dx, al", in("dx") 0xe9u16, in("al") byte);
    } }
}

// ------------------------=
// FUNC: panic
// DESC: Fails the disposable VM on any unexpected kernel assertion.
// ------------------=
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let line = info.location().map_or(0, |location| location.line()) as u64;
    for byte in line.to_le_bytes() {
        unsafe { core::arch::asm!("out dx, al", in("dx") 0xe9u16, in("al") byte); }
    }
    finish_code(1)
}

// ------------------------=
// FUNC: identity
// DESC: Creates a supervisor identity mapping from loader-provided section boundaries.
// ------------------=
fn identity(address: u64, bytes: u64) -> MappedRange {
    MappedRange { virtual_address: address, physical_address: address, bytes }
}

// ------------------------=
// FUNC: infinity_kernel_entry
// DESC: Runs compiler-produced C at CPL3 and verifies faults, preemption, and kernel survival across repeated launches.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_kernel_entry(info: *const boot_info::BootInfo) -> ! {
    let boot = &*info;
    let mut memory = NativeMemory::empty();
    memory.initialize(boot.native_pool_address, boot.native_pool_bytes).unwrap();
    let initial = memory.available_pages();
    let kernel = KernelAccessMap {
        text: identity(boot.kernel_text_address, boot.kernel_text_bytes),
        rodata: identity(boot.kernel_rodata_address, boot.kernel_rodata_bytes),
        data: identity(boot.kernel_data_address, boot.kernel_data_bytes),
        stack: identity(boot.kernel_stack_address, boot.kernel_stack_bytes),
        gateway: identity(0, 0),
    };
    if option_env!("INFINITY_COMPILER_PROBE").is_some() { compiler::run(&mut memory, kernel, 0xfee00000); }
    let image = native_c_image::Image::parse(IMAGE, 62).unwrap();
    let layout = CompilerVirtualLayout { image_base: 0x100000000, heap_base: 0x200000000,
        heap_pages: 2, stack_guard_base: 0x300000000, stack_pages: 8 };
    for kind in [0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 6, 7] {
        let mut process = CompilerAddressSpace::build(&mut memory, 1, &image, layout, kernel).unwrap();
        native_c_image::Image::load_at(IMAGE, 62, process.image_bytes_mut(), layout.image_base as usize).unwrap();
        process.page_tables.map_range(&mut memory, 0xfee00000, 0xfee00000, 4096,
            PagePermissions::KERNEL_RW).unwrap();
        let mut registers = task::Registers { rip: process.entry, rsp: process.stack_top - 8,
            rdi: kind, rsi: (&raw mut SENTINEL) as u64, fs_base: layout.heap_base, ..Default::default() };
        let mut initialized = false;
        let mut yields = 0;
        for _ in 0..100 {
            task::run(&mut registers, &mut *(&raw mut STATE), &mut initialized,
                process.page_tables.root_physical_address(), 0xfee00000).unwrap();
            if kind == 7 && registers.vector == 128 {
                if let Some(result) = services::client_contract(&mut process.page_tables, registers.rax,
                    [registers.rdi, registers.rsi, registers.rdx, registers.rcx, registers.r8, registers.r9], &mut yields) {
                    registers.rax = result;
                    continue;
                }
            }
            if kind == 6 && registers.vector == 128 && registers.rax == 0x1001 {
                assert_eq!([registers.rdi, registers.rsi, registers.rdx,
                    registers.rcx, registers.r8, registers.r9], [yields, 11, 22, 33, 44, 55]);
                registers.rax = yields + 100;
                yields += 1;
                continue;
            }
            if kind == 5 && registers.vector == 128 && registers.rax == 5 {
                assert_eq!(registers.rbx, yields);
                yields += 1;
                continue;
            }
            if registers.vector != 32 || kind == 3 { break; }
        }
        if kind == 5 || kind == 6 { assert_eq!(yields, 16); }
        if kind == 7 { assert_eq!(yields, 3); }
        let expected = match kind { 1 | 2 => 14, 3 => 32, 4 => 13, _ => 128 };
        assert_eq!(registers.vector, expected);
        assert_eq!(registers.cs & 3, 3);
        assert_eq!(SENTINEL, 0x1020304050607080);
        if kind == 0 || kind >= 5 { assert_eq!(registers.rax, 3); assert_eq!(registers.rbx, 42); }
        if kind == 1 { assert_eq!(registers.error & 7, 7); }
        if kind == 2 { assert_ne!(registers.error & 16, 0); }
        for value in [kind, registers.vector, registers.error, registers.rax] {
            for byte in value.to_le_bytes() {
                core::arch::asm!("out dx, al", in("dx") 0xe9u16, in("al") byte);
            }
        }
        process.release(&mut memory).unwrap();
        assert_eq!(memory.available_pages(), initial);
    }
    finish()
}

// ------------------------=
// FUNC: memcpy
// DESC: Provides freestanding memory copying for the native probe.
// ------------------=
#[no_mangle]
unsafe extern "C" fn memcpy(dst: *mut core::ffi::c_void, src: *const core::ffi::c_void, count: usize) -> *mut core::ffi::c_void {
    memory::copy(dst.cast(), src.cast(), count); dst
}
// ------------------------=
// FUNC: memset
// DESC: Provides freestanding initialization for page-table and register structures.
// ------------------=
#[no_mangle]
unsafe extern "C" fn memset(dst: *mut core::ffi::c_void, value: i32, count: usize) -> *mut core::ffi::c_void {
    memory::fill(dst.cast(), value as u8, count); dst
}
// ------------------------=
// FUNC: memcmp
// DESC: Compares bounded probe data without a host C library.
// ------------------=
#[no_mangle]
unsafe extern "C" fn memcmp(a: *const core::ffi::c_void, b: *const core::ffi::c_void, count: usize) -> i32 {
    for i in 0..count { let difference = *a.cast::<u8>().add(i) as i32 - *b.cast::<u8>().add(i) as i32;
        if difference != 0 { return difference; } }
    0
}
