//! EL0 execution with a physical-timer slice. Platform discovery supplies a
//! GICv2 CPU interface or the current CPU's GICv3 redistributor; no app sees MMIO.
use core::sync::atomic::{AtomicBool, Ordering};

#[repr(C, align(16))]
pub struct Registers {
    pub x: [u64; 31],
    pub sp: u64,
    pub pc: u64,
    pub pstate: u64,
    pub syndrome: u64,
    pub fault_address: u64,
    pub reason: u64,
    pub thread_pointer: u64,
    pub vectors: [u64; 64],
    pub fpcr: u64,
    pub fpsr: u64,
}

impl Registers {
    // ------------------------=
    // FUNC: new
    // DESC: Initializes an EL0 context with no inherited kernel registers or floating-point state.
    // ------------------=
    pub const fn new(pc: u64, sp: u64) -> Self {
        Self { x: [0; 31], sp, pc, pstate: 0, syndrome: 0, fault_address: 0,
            reason: 0, thread_pointer: 0, vectors: [0; 64], fpcr: 0, fpsr: 0 }
    }
}

#[derive(Clone, Copy)]
pub enum InterruptController {
    V2 { distributor: u64, cpu: u64 },
    V3 { redistributor: u64 },
}

#[repr(C, align(4096))]
struct Stack([u8; 32768]);
static mut TRAP_STACK: Stack = Stack([0; 32768]);
#[repr(C, align(16))]
struct Saved([u64; 128]);
#[no_mangle]
static mut infinity_arm_kernel: Saved = Saved([0; 128]);
#[no_mangle]
static mut infinity_arm_registers: u64 = 0;
static BUSY: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunError { Busy, UnsupportedLevel, InvalidContext }

core::arch::global_asm!(include_str!("native_task_arm.S"));
unsafe extern "C" { fn infinity_arm_enter(frame: *mut Registers, root: u64, stack: u64); }

// ------------------------=
// FUNC: run
// DESC: Executes one EL0 slice and restores the kernel's memory, register, and interrupt state on every app trap.
// ------------------=
pub unsafe fn run(frame: &mut Registers, root: u64, controller: InterruptController,
    microseconds: u32) -> Result<(), RunError> {
    let level: u64;
    core::arch::asm!("mrs {}, CurrentEL", out(reg) level);
    if level != 4 { return Err(RunError::UnsupportedLevel); }
    if root == 0 || root & 4095 != 0 || root >> 48 != 0 || frame.pc >> 48 != 0 ||
        frame.sp >> 48 != 0 || microseconds == 0 || microseconds > 10_000 {
        return Err(RunError::InvalidContext);
    }
    if BUSY.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        return Err(RunError::Busy);
    }
    let daif: u64;
    core::arch::asm!("mrs {}, daif", "msr daifset, #0xf", out(reg) daif);
    let old_timer: u64;
    let old_compare: u64;
    let frequency: u64;
    core::arch::asm!("mrs {}, cntp_ctl_el0", "mrs {}, cntp_cval_el0", "mrs {}, cntfrq_el0",
        out(reg) old_timer, out(reg) old_compare, out(reg) frequency);
    let (bank, old_control, old_priority, old_group, enabled, old_sre) = match controller {
        InterruptController::V2 { distributor, cpu } => {
            let control = read(cpu);
            let priority = read(cpu + 4);
            let group = read(distributor + 0x80);
            let enabled = read(distributor + 0x100);
            // Non-secure Group 1 physical timer PPI30, current CPU bank.
            write(distributor + 0x80, group | (1 << 30));
            write(distributor + 0x100, 1 << 30);
            write(cpu + 4, 255);
            // AckCtl permits Group 1 acknowledgement through IAR when the
            // platform exposes the secure view (QEMU's non-security model).
            write(cpu, control | 7);
            (distributor, control as u64, priority as u64, group, enabled, 0)
        }
        InterruptController::V3 { redistributor } => {
            let sre: u64;
            core::arch::asm!("mrs {}, ICC_SRE_EL1", out(reg) sre);
            core::arch::asm!("msr ICC_SRE_EL1, {}", "isb", in(reg) sre | 1);
            let control: u64;
            let priority: u64;
            core::arch::asm!("mrs {}, ICC_IGRPEN1_EL1", "mrs {}, ICC_PMR_EL1",
                out(reg) control, out(reg) priority);
            let bank = redistributor + 0x10000;
            let group = read(bank + 0x80);
            let enabled = read(bank + 0x100);
            write(bank + 0x80, group | (1 << 30));
            write(bank + 0x100, 1 << 30);
            core::arch::asm!("msr ICC_PMR_EL1, {}", "msr ICC_IGRPEN1_EL1, {}", "isb",
                in(reg) 255u64, in(reg) 1u64);
            (bank, control, priority, group, enabled, sre)
        }
    };
    let priority_address = (bank + 0x400 + 30) as *mut u8;
    let old_irq_priority = core::ptr::read_volatile(priority_address);
    core::ptr::write_volatile(priority_address, 0x80);
    let ticks = (frequency * microseconds as u64 / 1_000_000).max(1);
    core::arch::asm!("msr cntp_tval_el0, {}", "msr cntp_ctl_el0, {}", "isb",
        in(reg) ticks, in(reg) 1u64);
    frame.pstate &= 0xf0000000; // EL0t, IRQ enabled, no inherited privilege flags.
    infinity_arm_enter(frame, root, (&raw mut TRAP_STACK).cast::<u8>().add(32768) as u64);
    if frame.reason == 1 {
        let interrupt = match controller {
            InterruptController::V2 { cpu, .. } => read(cpu + 12) as u64,
            InterruptController::V3 { .. } => {
                let id: u64;
                core::arch::asm!("mrs {}, ICC_IAR1_EL1", out(reg) id); id
            }
        };
        core::arch::asm!("msr cntp_ctl_el0, {}", "isb", in(reg) 2u64);
        frame.syndrome = interrupt;
        if interrupt & 1023 < 1020 {
            match controller {
                InterruptController::V2 { cpu, .. } => write(cpu + 16, interrupt as u32),
                InterruptController::V3 { .. } => {
                    core::arch::asm!("msr ICC_EOIR1_EL1, {}", "isb", in(reg) interrupt);
                }
            }
        }
    }
    core::arch::asm!("msr cntp_ctl_el0, {}", "isb", in(reg) 2u64);
    if enabled & (1 << 30) == 0 { write(bank + 0x180, 1 << 30); }
    write(bank + 0x80, old_group);
    core::ptr::write_volatile(priority_address, old_irq_priority);
    match controller {
        InterruptController::V2 { cpu, .. } => { write(cpu + 4, old_priority as u32); write(cpu, old_control as u32); }
        InterruptController::V3 { .. } => {
            core::arch::asm!("msr ICC_PMR_EL1, {}", "msr ICC_IGRPEN1_EL1, {}", "isb",
                "msr ICC_SRE_EL1, {}", "isb", in(reg) old_priority, in(reg) old_control, in(reg) old_sre);
        }
    }
    core::arch::asm!("msr cntp_cval_el0, {}", "msr cntp_ctl_el0, {}", "isb",
        in(reg) old_compare, in(reg) old_timer);
    BUSY.store(false, Ordering::Release);
    core::arch::asm!("msr daif, {}", in(reg) daif);
    Ok(())
}

// ------------------------=
// FUNC: read
// DESC: Reads a kernel-only interrupt-controller register.
// ------------------=
unsafe fn read(address: u64) -> u32 { core::ptr::read_volatile(address as *const u32) }
// ------------------------=
// FUNC: write
// DESC: Publishes a kernel-only interrupt-controller register update before enabling the timer.
// ------------------=
unsafe fn write(address: u64, value: u32) {
    core::ptr::write_volatile(address as *mut u32, value);
    core::arch::asm!("dsb sy");
}
