//! Small hardware adapters; compiler invocation and syscall behavior are shared.
use super::{native_memory::*, task};
#[cfg(target_arch = "aarch64")]
pub type Tables = super::native_memory_arm::ArmAddressSpace;
#[cfg(target_arch = "x86_64")]
pub type Tables = X86AddressSpace;
#[cfg(target_arch = "aarch64")]
pub type Controller = task::InterruptController;
#[cfg(target_arch = "x86_64")]
pub type Controller = ();
#[cfg(target_arch = "aarch64")]
pub const MACHINE: u16 = 183;
#[cfg(target_arch = "x86_64")]
pub const MACHINE: u16 = 62;
#[cfg(target_arch = "aarch64")]
pub const TRIPLE: &[u8] = b"aarch64-none-elf";
#[cfg(target_arch = "x86_64")]
pub const TRIPLE: &[u8] = b"x86_64-none-elf";

pub enum Slice { Preempted, Call(u64, [u64; 6]), Fault(u64, u64, u64) }
pub struct Context {
    registers: task::Registers,
    #[cfg(target_arch = "x86_64")]
    state: task::ExtendedState,
    #[cfg(target_arch = "x86_64")]
    initialized: bool,
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
    pub unsafe fn run(&mut self, root: u64, controller: Controller) -> Slice {
        #[cfg(target_arch = "aarch64")]
        {
            task::run(&mut self.registers, root, controller, 2000).unwrap();
            let r = &self.registers;
            if r.reason == 1 { Slice::Preempted }
            else if r.syndrome >> 26 == 0x15 { Slice::Call(r.x[8], r.x[..6].try_into().unwrap()) }
            else { Slice::Fault(r.syndrome, r.fault_address, r.pc) }
        }
        #[cfg(target_arch = "x86_64")]
        {
            let _ = controller;
            task::run(&mut self.registers, &mut self.state, &mut self.initialized, root, 0xfee00000).unwrap();
            let r = &self.registers;
            if r.vector == 32 { Slice::Preempted }
            else if r.vector == 128 { Slice::Call(r.rax, [r.rdi, r.rsi, r.rdx, r.rcx, r.r8, r.r9]) }
            else { Slice::Fault(r.vector, r.error, r.rip) }
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
pub unsafe fn prepare(process: &mut ProcessAddressSpace<Tables>, memory: &mut NativeMemory) {
    #[cfg(target_arch = "aarch64")]
    {
        let _ = memory;
        for address in (process.image.physical_address..process.image.physical_address + process.image.page_count as u64 * 4096).step_by(64) {
            core::arch::asm!("dc cvau, {}", in(reg)address);
        }
        core::arch::asm!("dsb ish", "ic iallu", "dsb ish", "isb");
    }
    #[cfg(target_arch = "x86_64")]
    { process.page_tables.map_range(memory, 0xfee00000, 0xfee00000, 4096, PagePermissions::KERNEL_RW).unwrap(); }
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
        Some(((ticks as u128 * 1_000_000_000) / frequency as u128) as u64)
    }
    #[cfg(target_arch = "x86_64")]
    { None }
}
