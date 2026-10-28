//! Small hardware adapters; compiler invocation and syscall behavior are shared.
use super::native_memory::*;
#[cfg(target_arch = "aarch64")]
use super::native_task_arm as task;
#[cfg(target_arch = "x86_64")]
use super::native_task_x86 as task;
#[cfg(target_arch = "aarch64")]
pub type Tables = super::native_memory_arm::ArmAddressSpace;
#[cfg(target_arch = "x86_64")]
pub type Tables = X86AddressSpace;
#[cfg(target_arch = "aarch64")]
pub type Controller = task::InterruptController;
#[cfg(target_arch = "x86_64")]
pub type Controller = u64;
pub use task::RunError;
#[cfg(target_arch = "aarch64")]
pub const MACHINE: u16 = 183;
#[cfg(target_arch = "x86_64")]
pub const MACHINE: u16 = 62;
#[cfg(target_arch = "aarch64")]
pub const TRIPLE: &[u8] = b"aarch64-none-elf";
#[cfg(target_arch = "x86_64")]
pub const TRIPLE: &[u8] = b"x86_64-none-elf";

pub use super::native_process::Trap as Slice;
// ------------------------=
// FUNC: controller_from_handoff
// DESC: Accepts only the current architecture's aligned, firmware-discovered controller contract; no guessed defaults.
// ------------------=
pub fn controller_from_handoff(
    kind: u32,
    reserved: u32,
    base: u64,
    auxiliary: u64,
) -> Option<Controller> {
    if reserved != 0 || base == 0 || base & 4095 != 0 || base.checked_add(0x20000).is_none() {
        return None;
    }
    #[cfg(target_arch = "x86_64")]
    {
        if kind == 1 && auxiliary == 0 && base < 64 * 1024 * 1024 * 1024 {
            Some(base)
        } else {
            None
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        const KIND_MASK: u32 = 0xff;
        const TIMER_SHIFT: u32 = 8;
        const VIRTUAL_TIMER: u32 = 0x10000;
        if auxiliary == 0 || auxiliary & 4095 != 0 || auxiliary.checked_add(65536).is_none() {
            return None;
        }
        let controller_kind = kind & KIND_MASK;
        let timer = if kind == controller_kind {
            task::SliceTimer::Physical { interrupt: 30 }
        } else if kind & !0x1ffff == 0 && kind & VIRTUAL_TIMER != 0 {
            let interrupt = ((kind >> TIMER_SHIFT) & 255) as u8;
            if !(16..32).contains(&interrupt) {
                return None;
            }
            task::SliceTimer::Virtual { interrupt }
        } else {
            return None;
        };
        match controller_kind {
            2 => Some(Controller::V2 {
                distributor: base,
                cpu: auxiliary,
                timer,
            }),
            3 if base & 65535 == 0 => Some(Controller::V3 {
                redistributor: base,
                timer,
            }),
            _ => None,
        }
    }
}
pub struct Context {
    registers: task::Registers,
    #[cfg(target_arch = "x86_64")]
    state: task::ExtendedState,
    #[cfg(target_arch = "x86_64")]
    initialized: bool,
}

pub struct BoundContext {
    context: Context,
    root: u64,
    controller: Controller,
}
impl BoundContext {
    // ------------------------=
    // FUNC: new
    // DESC: Binds a live private mapping and native controller; caller must retain mappings until execution terminates.
    // ------------------=
    pub unsafe fn new(context: Context, root: u64, controller: Controller) -> Self {
        Self {
            context,
            root,
            controller,
        }
    }
}
impl super::native_process::Executor for BoundContext {
    // ------------------------=
    // FUNC: enter
    // DESC: Enters the bound private user context once; platform failures terminate supervision rather than panicking the OS.
    // ------------------=
    fn enter(&mut self) -> Result<Slice, ()> {
        unsafe { self.context.run(self.root, self.controller).map_err(|_| ()) }
    }
    // ------------------------=
    // FUNC: result
    // DESC: Writes the completed native service result to the user ABI register.
    // ------------------=
    fn result(&mut self, value: i64) {
        self.context.result(value as u64);
    }
}
impl Context {
    // ------------------------=
    // FUNC: new
    // DESC: Constructs the platform calling convention and isolated thread-local register state.
    // ------------------=
    pub fn new(entry: u64, stack: u64, argument: u64, tls: u64) -> Self {
        #[cfg(target_arch = "aarch64")]
        {
            let mut registers = task::Registers::new(entry, stack);
            registers.x[0] = argument;
            registers.thread_pointer = tls;
            Self { registers }
        }
        #[cfg(target_arch = "x86_64")]
        {
            Self {
                registers: task::Registers {
                    rip: entry,
                    rsp: stack - 8,
                    rdi: argument,
                    fs_base: tls,
                    ..Default::default()
                },
                state: task::ExtendedState([0; 65536]),
                initialized: false,
            }
        }
    }
    // ------------------------=
    // FUNC: run
    // DESC: Runs one preemptible native user slice and normalizes only the hardware trap encoding.
    // ------------------=
    pub unsafe fn run(&mut self, root: u64, controller: Controller) -> Result<Slice, RunError> {
        #[cfg(target_arch = "aarch64")]
        {
            task::run(&mut self.registers, root, controller, 2000)?;
            let r = &self.registers;
            Ok(if r.reason == 1 {
                Slice::Preempted
            } else if r.syndrome >> 26 == 0x15 {
                Slice::Call(r.x[8], [r.x[0], r.x[1], r.x[2], r.x[3], r.x[4], r.x[5]])
            } else {
                Slice::Fault(r.syndrome, r.fault_address, r.pc)
            })
        }
        #[cfg(target_arch = "x86_64")]
        {
            task::run(
                &mut self.registers,
                &mut self.state,
                &mut self.initialized,
                root,
                controller,
            )?;
            let r = &self.registers;
            Ok(if r.vector == 32 {
                Slice::Preempted
            } else if r.vector == 128 {
                Slice::Call(r.rax, [r.rdi, r.rsi, r.rdx, r.rcx, r.r8, r.r9])
            } else {
                Slice::Fault(r.vector, r.error, r.rip)
            })
        }
    }
    // ------------------------=
    // FUNC: result
    // DESC: Places a service return value in the architecture's user ABI return register.
    // ------------------=
    pub fn result(&mut self, value: u64) {
        #[cfg(target_arch = "aarch64")]
        {
            self.registers.x[0] = value;
        }
        #[cfg(target_arch = "x86_64")]
        {
            self.registers.rax = value;
        }
    }
}

// ------------------------=
// FUNC: prepare
// DESC: Prepares instruction visibility and the supervisor-only interrupt device mapping for a new process.
// ------------------=
pub unsafe fn prepare(
    process: &mut ProcessAddressSpace<Tables>,
    memory: &mut NativeMemory,
    controller: Controller,
) -> Result<(), MemoryError> {
    clean_instruction_range(
        process.image.physical_address,
        process.image.page_count as usize * 4096,
    );
    finish_preparation(process, memory, controller)
}

// ------------------------=
// FUNC: clean_instruction_range
// DESC: Publishes a caller-bounded image chunk to the instruction point of unification on the native CPU.
// ------------------=
pub unsafe fn clean_instruction_range(address: u64, bytes: usize) {
    #[cfg(target_arch = "aarch64")]
    for at in (address..address + bytes as u64).step_by(64) {
        core::arch::asm!("dc cvau, {}", in(reg)at);
    }
    #[cfg(target_arch = "x86_64")]
    let _ = (address, bytes);
}

// ------------------------=
// FUNC: finish_preparation
// DESC: Installs the small supervisor trampoline and orders previously cleaned chunks before first execution.
// ------------------=
pub unsafe fn finish_preparation(
    process: &mut ProcessAddressSpace<Tables>,
    memory: &mut NativeMemory,
    controller: Controller,
) -> Result<(), MemoryError> {
    task::map_trampoline(&mut process.page_tables, memory)?;
    #[cfg(target_arch = "aarch64")]
    {
        let _ = (memory, controller);
        core::arch::asm!("dsb ish", "ic iallu", "dsb ish", "isb");
        Ok(())
    }
    #[cfg(target_arch = "x86_64")]
    {
        process.page_tables.map_range(
            memory,
            controller,
            controller,
            4096,
            PagePermissions::KERNEL_RW,
        )
    }
}

// ------------------------=
// FUNC: map_privileged
// DESC: Identity maps only the supervisor trampoline pages, accepting identical overlaps but never replacing user mappings or permissions.
// ------------------=
pub fn map_privileged<T: PageTable>(
    tables: &mut T,
    memory: &mut NativeMemory,
    address: u64,
    bytes: usize,
    permissions: PagePermissions,
) -> Result<(), MemoryError> {
    if permissions.user || bytes == 0 {
        return Err(MemoryError::InvalidPermissions);
    }
    let end = address
        .checked_add(bytes as u64)
        .and_then(|v| v.checked_add(4095))
        .ok_or(MemoryError::AddressOverflow)?
        & !4095;
    let mut page = address & !4095;
    while page < end {
        if let Some(mapping) = tables.mapping(page) {
            if mapping.physical_address != page || mapping.permissions != permissions {
                return Err(MemoryError::MappingConflict);
            }
        } else {
            tables.map_range(memory, page, page, 4096, permissions)?;
        }
        page += 4096;
    }
    Ok(())
}

// ------------------------=
// FUNC: private_kernel_map
// DESC: Starts with no ambient kernel mappings; prepare installs the small architecture-specific privileged transition boundary.
// ------------------=
pub const fn private_kernel_map() -> KernelAccessMap {
    let empty = MappedRange {
        virtual_address: 0,
        physical_address: 0,
        bytes: 0,
    };
    KernelAccessMap {
        text: empty,
        rodata: empty,
        data: empty,
        stack: empty,
        gateway: empty,
    }
}

// ------------------------=
// FUNC: clock_ns
// DESC: Returns measured architectural timer nanoseconds, or unavailable when no calibrated clock is supplied.
// ------------------=
pub unsafe fn clock_ns() -> Option<u64> {
    #[cfg(target_arch = "aarch64")]
    {
        let ticks: u64;
        let frequency: u64;
        core::arch::asm!("mrs {}, cntpct_el0", out(reg)ticks);
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg)frequency);
        if frequency == 0 {
            return None;
        }
        Some(((ticks as u128 * 1_000_000_000) / frequency as u128).min(u64::MAX as u128) as u64)
    }
    #[cfg(target_arch = "x86_64")]
    {
        None
    }
}
