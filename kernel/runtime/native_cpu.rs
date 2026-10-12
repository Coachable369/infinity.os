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
pub struct Context {
    registers: task::Registers,
    #[cfg(target_arch = "x86_64")]
    state: task::ExtendedState,
    #[cfg(target_arch = "x86_64")]
    initialized: bool,
}

pub struct BoundContext { context: Context, root: u64, controller: Controller }
impl BoundContext {
    // ------------------------=
    // FUNC: new
    // DESC: Binds a live private mapping and native controller; caller must retain mappings until execution terminates.
    // ------------------=
    pub unsafe fn new(context: Context, root: u64, controller: Controller) -> Self {
        Self { context, root, controller }
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
    fn result(&mut self, value: i64) { self.context.result(value as u64); }
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
        { Self { registers: task::Registers { rip: entry, rsp: stack - 8, rdi: argument,
                fs_base: tls, ..Default::default() }, state: task::ExtendedState([0; 65536]), initialized: false } }
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
            Ok(if r.reason == 1 { Slice::Preempted }
            else if r.syndrome >> 26 == 0x15 { Slice::Call(r.x[8], [r.x[0], r.x[1], r.x[2], r.x[3], r.x[4], r.x[5]]) }
            else { Slice::Fault(r.syndrome, r.fault_address, r.pc) })
        }
        #[cfg(target_arch = "x86_64")]
        {
            task::run(&mut self.registers, &mut self.state, &mut self.initialized, root, controller)?;
            let r = &self.registers;
            Ok(if r.vector == 32 { Slice::Preempted }
            else if r.vector == 128 { Slice::Call(r.rax, [r.rdi, r.rsi, r.rdx, r.rcx, r.r8, r.r9]) }
            else { Slice::Fault(r.vector, r.error, r.rip) })
        }
    }
    // ------------------------=
    // FUNC: result
    // DESC: Places a service return value in the architecture's user ABI return register.
    // ------------------=
    pub fn result(&mut self, value: u64) {
        #[cfg(target_arch = "aarch64")]
        { self.registers.x[0] = value; }
        #[cfg(target_arch = "x86_64")]
        { self.registers.rax = value; }
    }
}

// ------------------------=
// FUNC: prepare
// DESC: Prepares instruction visibility and the supervisor-only interrupt device mapping for a new process.
// ------------------=
pub unsafe fn prepare(process: &mut ProcessAddressSpace<Tables>, memory: &mut NativeMemory,
    controller: Controller) -> Result<(), MemoryError> {
    #[cfg(target_arch = "aarch64")]
    {
        let _ = (memory, controller);
        for address in (process.image.physical_address..process.image.physical_address + process.image.page_count as u64 * 4096).step_by(64) {
            core::arch::asm!("dc cvau, {}", in(reg)address);
        }
        core::arch::asm!("dsb ish", "ic iallu", "dsb ish", "isb");
        Ok(())
    }
    #[cfg(target_arch = "x86_64")]
    { process.page_tables.map_range(memory, controller, controller, 4096, PagePermissions::KERNEL_RW) }
}

// ------------------------=
// FUNC: clock_ns
// DESC: Returns measured architectural timer nanoseconds, or unavailable when no calibrated clock is supplied.
// ------------------=
pub unsafe fn clock_ns() -> Option<u64> {
    #[cfg(target_arch = "aarch64")]
    {
        let ticks: u64; let frequency: u64;
        core::arch::asm!("mrs {}, cntpct_el0", out(reg)ticks);
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg)frequency);
        if frequency == 0 { return None; }
        Some(((ticks as u128 * 1_000_000_000) / frequency as u128).min(u64::MAX as u128) as u64)
    }
    #[cfg(target_arch = "x86_64")]
    { None }
}
