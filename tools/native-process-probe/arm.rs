#[path = "../../kernel/core/memory.rs"] mod memory;
#[path = "../../kernel/core/boot_info.rs"] mod boot_info;
#[path = "../../kernel/runtime/native_c_image.rs"] mod native_c_image;
#[path = "../../kernel/runtime/native_memory.rs"] mod native_memory;
#[path = "../../kernel/runtime/native_memory_arm.rs"] mod native_memory_arm;
#[path = "../../kernel/runtime/native_mappings.rs"] mod native_mappings;
#[path = "../../kernel/runtime/native_stdio.rs"] mod native_stdio;
#[path = "../../kernel/runtime/native_compiler_launch.rs"] mod native_compiler_launch;
#[path = "../../kernel/runtime/native_job_files.rs"] mod native_job_files;
#[path = "../../kernel/runtime/native_compiler_task.rs"] mod native_compiler_task;
#[path = "../../kernel/runtime/native_app_task.rs"] mod native_app_task;
use cpu as native_cpu;
#[path = "../../kernel/runtime/native_task_arm.rs"] mod task;
use task as native_task_arm;
#[path = "../../kernel/runtime/execution.rs"] mod execution;
#[path = "../../kernel/runtime/native_process.rs"] mod native_process;
use native_memory::*;
#[path = "services.rs"] mod services;
use native_memory_arm::ArmAddressSpace;
#[path = "compiler.rs"] mod compiler;
#[path = "../../kernel/runtime/native_cpu.rs"] mod cpu;

const COMPILER: bool = option_env!("INFINITY_COMPILER_PROBE").is_some();
const POOL_BYTES: usize = if COMPILER { 256 * 1024 * 1024 } else { 8 * 1024 * 1024 };
const STACK_TOP: u64 = if COMPILER { 0x5ff00000 } else { 0x42000000 };

static IMAGE: &[u8] = include_bytes!(env!("INFINITY_PROCESS_IMAGE"));
static mut SENTINEL: u64 = 0x1020304050607080;
#[repr(C, align(4096))]
struct Arena([u8; POOL_BYTES]);
static mut POOL: Arena = Arena([0; POOL_BYTES]);
#[repr(C, align(4096))]
struct Table([u64; 512]);
static mut BOOT_TABLE: Table = Table([0; 512]);

core::arch::global_asm!(
    ".section .text.entry", ".global _start", "_start:", "msr daifset, #0xf",
    "msr spsel, #1", "ldr x0, ={stack}", "mov sp, x0",
    "bl process_probe", "b .", stack = const STACK_TOP
);
unsafe extern "C" {
    static text_start: u8; static text_end: u8;
    static rodata_start: u8; static rodata_end: u8;
    static data_start: u8; static data_end: u8;
}

// ------------------------=
// FUNC: emit
// DESC: Emits fixed-size numeric guest evidence on PL011.
// ------------------=
fn emit(value: u64) {
    for byte in value.to_le_bytes() { unsafe {
        while core::ptr::read_volatile(0x09000018 as *const u32) & 32 != 0 {}
        core::ptr::write_volatile(0x09000000 as *mut u32, byte as u32);
    } }
}
// ------------------------=
// FUNC: finish
// DESC: Powers off the disposable ARM VM using its PSCI conduit after evidence is emitted.
// ------------------=
fn finish() -> ! {
    unsafe { core::arch::asm!("hvc #0", in("x0") 0x84000008u64); }
    loop { core::hint::spin_loop(); }
}
// ------------------------=
// FUNC: panic
// DESC: Emits a numeric failure location without confusing it with passing case records.
// ------------------=
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    emit(u64::MAX); emit(info.location().map_or(0, |location| location.line()) as u64); finish()
}
// ------------------------=
// FUNC: region
// DESC: Builds a page-aligned identity mapping from linker-verified boundaries.
// ------------------=
fn region(start: *const u8, end: *const u8) -> MappedRange {
    MappedRange { virtual_address: start as u64, physical_address: start as u64,
        bytes: end as u64 - start as u64 }
}
// ------------------------=
// FUNC: process_probe
// DESC: Verifies real EL0 syscalls, permission faults, privileged-instruction rejection and timer recovery.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn process_probe() -> ! {
    let table = (&raw mut BOOT_TABLE).cast::<u64>();
    *table = 1 | (1 << 10);
    *table.add(1) = 0x40000000 | 1 | (1 << 2) | (3 << 8) | (1 << 10);
    let control = 25 | (1 << 8) | (1 << 10) | (3 << 12) | (1 << 23) | (2u64 << 32);
    core::arch::asm!("msr mair_el1, {mair}", "msr tcr_el1, {control}", "msr ttbr0_el1, {table}",
        "dsb sy", "isb", "tlbi vmalle1", "dsb sy", "isb",
        mair=in(reg)0xff00u64, control=in(reg)control, table=in(reg)table);
    let mut sctlr: u64;
    core::arch::asm!("mrs {}, sctlr_el1", out(reg) sctlr);
    sctlr |= 1 | (1 << 2) | (1 << 12);
    core::arch::asm!("msr sctlr_el1, {}", "isb", in(reg) sctlr);
    // QEMU's reset controller is disabled; production discovery supplies the
    // existing enabled controller instead of relying on these test addresses.
    let controller = if env!("INFINITY_PROCESS_GIC") == "3" {
        core::ptr::write_volatile(0x08000000 as *mut u32, (1 << 4) | 2);
        core::ptr::write_volatile(0x080a0014 as *mut u32, 0);
        let mut awake = false;
        for _ in 0..100000 {
            if core::ptr::read_volatile(0x080a0014 as *const u32) & 4 == 0 { awake = true; break; }
        }
        assert!(awake);
        if option_env!("INFINITY_PROCESS_SPLIT_EOI").is_some() {
            core::arch::asm!("msr ICC_SRE_EL1, {}", "isb", in(reg) 1u64);
            let control: u64;
            core::arch::asm!("mrs {}, ICC_CTLR_EL1", out(reg) control);
            core::arch::asm!("msr ICC_CTLR_EL1, {}", "isb", in(reg) control | 2);
            let configured: u64;
            core::arch::asm!("mrs {}, ICC_CTLR_EL1", out(reg) configured);
            assert_ne!(configured & 2, 0);
        }
        task::InterruptController::V3 { redistributor: 0x080a0000 }
    } else {
        core::ptr::write_volatile(0x08000000 as *mut u32, 3);
        task::InterruptController::V2 { distributor: 0x08000000, cpu: 0x08010000 }
    };
    let mut memory = NativeMemory::empty();
    memory.initialize((&raw mut POOL) as u64, POOL_BYTES as u64).unwrap();
    let kernel = KernelAccessMap {
        text: region(&raw const text_start, &raw const text_end),
        rodata: region(&raw const rodata_start, &raw const rodata_end),
        data: region(&raw const data_start, &raw const data_end),
        stack: MappedRange { virtual_address: STACK_TOP - 0x100000, physical_address: STACK_TOP - 0x100000, bytes: 0x100000 },
        gateway: MappedRange { virtual_address: 0, physical_address: 0, bytes: 0 },
    };
    run_cases(memory,kernel,controller)
}
// ------------------------=
// FUNC: infinity_kernel_entry
// DESC: Exercises the production firmware handoff without guessed GIC addresses or a replacement firmware memory map.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_kernel_entry(info: *const boot_info::BootInfo) -> ! {
    let boot=&*info;
    assert_eq!(boot.magic,boot_info::BOOT_MAGIC);
    assert_eq!(boot.version,boot_info::BOOT_VERSION);
    assert!(cpu::controller_from_handoff(0,0,0,0).is_none());
    assert!(cpu::controller_from_handoff(1,0,0xfee00000,0).is_none());
    assert!(cpu::controller_from_handoff(3,1,0x080a0000,0x08000000).is_none());
    assert!(cpu::controller_from_handoff(3,0,0x080a1000,0x08000000).is_none());
    let controller=cpu::controller_from_handoff(boot.native_controller_kind,
        boot.native_controller_reserved,boot.native_controller_base,boot.native_controller_auxiliary).unwrap();
    let mut memory=NativeMemory::empty();
    memory.initialize(boot.native_pool_address,boot.native_pool_bytes).unwrap();
    let kernel=KernelAccessMap {
        text: mapped(boot.kernel_text_address,boot.kernel_text_bytes),
        rodata: mapped(boot.kernel_rodata_address,boot.kernel_rodata_bytes),
        data: mapped(boot.kernel_data_address,boot.kernel_data_bytes),
        stack: mapped(boot.kernel_stack_address,boot.kernel_stack_bytes), gateway: mapped(0,0),
    };
    run_cases(memory,kernel,controller)
}
// ------------------------=
// FUNC: mapped
// DESC: Converts the loader's checked section bounds into supervisor mappings.
// ------------------=
fn mapped(address:u64,bytes:u64)->MappedRange {
    MappedRange {virtual_address:address,physical_address:address,bytes}
}
// ------------------------=
// FUNC: run_cases
// DESC: Runs identical isolation and recovery cases for firmware and bare-metal entry paths.
// ------------------=
unsafe fn run_cases(mut memory:NativeMemory,kernel:KernelAccessMap,controller:cpu::Controller)->! {
    let initial=memory.available_pages();
    if COMPILER { compiler::run(&mut memory, kernel, controller); }
    let image = native_c_image::Image::parse(IMAGE, 183).unwrap();
    let layout = CompilerVirtualLayout { image_base: 0x100000000, heap_base: 0x200000000,
        heap_pages: 2, stack_guard_base: 0x300000000, stack_pages: 8 };
    for kind in [0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 6, 7] {
        let mut process = ProcessAddressSpace::<ArmAddressSpace>::build(&mut memory, 1, &image, layout, kernel).unwrap();
        native_c_image::Image::load_at(IMAGE, 183, process.image_bytes_mut(), layout.image_base as usize).unwrap();
        // Make copied instructions visible before the first user fetch.
        for address in (process.image.physical_address..process.image.physical_address +
            process.image.page_count as u64 * 4096).step_by(64) {
            core::arch::asm!("dc cvau, {}", in(reg) address);
        }
        core::arch::asm!("dsb ish", "ic iallu", "dsb ish", "isb");
        let mut registers = task::Registers::new(process.entry, process.stack_top);
        registers.x[0] = kind;
        registers.x[1] = (&raw mut SENTINEL) as u64;
        registers.thread_pointer = layout.heap_base;
        let mut yields = 0;
        for _ in 0..100 {
            let virtual_control:u64; let virtual_deadline:u64;
            core::arch::asm!("mrs {}, cntv_ctl_el0", "mrs {}, cntv_cval_el0",
                out(reg) virtual_control,out(reg) virtual_deadline);
            task::run(&mut registers, process.page_tables.root_physical_address(),
                controller, 2000).unwrap();
            // Control must survive a slice. Firmware is free to advance an expired
            // deadline immediately on IRQ restoration, so compare only masked timers.
            let restored_control:u64; let restored_deadline:u64;
            core::arch::asm!("mrs {}, cntv_ctl_el0", "mrs {}, cntv_cval_el0",
                out(reg) restored_control,out(reg) restored_deadline);
            assert_eq!(restored_control & 3,virtual_control & 3);
            if virtual_control & 3 != 1 { assert_eq!(restored_deadline,virtual_deadline); }
            if kind == 7 && registers.reason == 0 && registers.syndrome >> 26 == 0x15 {
                if let Some(result) = services::client_contract(&mut process.page_tables, registers.x[8],
                    registers.x[..6].try_into().unwrap(), &mut yields) {
                    registers.x[0] = result;
                    continue;
                }
            }
            if kind == 6 && registers.reason == 0 && registers.syndrome >> 26 == 0x15 && registers.x[8] == 0x1001 {
                assert_eq!(&registers.x[..6], &[yields, 11, 22, 33, 44, 55]);
                registers.x[0] = yields + 100;
                yields += 1;
                continue;
            }
            if kind == 5 && registers.reason == 0 && registers.syndrome >> 26 == 0x15 && registers.x[8] == 5 {
                assert_eq!(registers.x[0], yields);
                yields += 1;
                continue;
            }
            if registers.reason != 1 || kind == 3 { break; }
        }
        if kind == 5 || kind == 6 { assert_eq!(yields, 16); }
        if kind == 7 { assert_eq!(yields, 3); }
        for value in [kind, registers.reason, registers.syndrome, registers.x[0]] { emit(value); }
        assert_eq!(core::ptr::read_volatile(&raw const SENTINEL), 0x1020304050607080);
        assert_eq!(registers.pstate & 15, 0);
        if kind == 3 {
            assert_eq!(registers.reason, 1); assert_eq!(registers.syndrome & 1023, 30);
            // A second busy-loop slice must preempt even under split EOI mode.
            task::run(&mut registers, process.page_tables.root_physical_address(), controller, 2000).unwrap();
            assert_eq!(registers.reason, 1); assert_eq!(registers.syndrome & 1023, 30);
        }
        else {
            assert_eq!(registers.reason, 0);
            let expected = match kind { 1 => 0x24, 2 => 0x20, 4 => 0, _ => 0x15 };
            assert_eq!(registers.syndrome >> 26, expected);
            if kind == 0 || kind >= 5 { assert_eq!(registers.x[0], 42); }
        }
        process.release(&mut memory).unwrap();
        assert_eq!(memory.available_pages(), initial);
    }
    emit(0x49534f4c41544544);
    finish()
}
// ------------------------=
// FUNC: memcpy
// DESC: Supplies the probe's freestanding C copy primitive.
// ------------------=
#[no_mangle]
unsafe extern "C" fn memcpy(dst: *mut core::ffi::c_void, src: *const core::ffi::c_void, count: usize) -> *mut core::ffi::c_void {
    memory::copy(dst.cast(), src.cast(), count); dst
}
// ------------------------=
// FUNC: memset
// DESC: Supplies the probe's freestanding zero-initialization primitive.
// ------------------=
#[no_mangle]
unsafe extern "C" fn memset(dst: *mut core::ffi::c_void, value: i32, count: usize) -> *mut core::ffi::c_void {
    memory::fill(dst.cast(), value as u8, count); dst
}
// ------------------------=
// FUNC: memcmp
// DESC: Supplies bounded byte comparison for the ELF validator.
// ------------------=
#[no_mangle]
unsafe extern "C" fn memcmp(a: *const core::ffi::c_void, b: *const core::ffi::c_void, count: usize) -> i32 {
    for i in 0..count { let difference = *a.cast::<u8>().add(i) as i32 - *b.cast::<u8>().add(i) as i32;
        if difference != 0 { return difference; } }
    0
}
