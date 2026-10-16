//! EL0 execution with a physical-timer slice. Platform discovery supplies a
//! GICv2 CPU interface or the current CPU's GICv3 redistributor; no app sees MMIO.
use core::sync::atomic::{AtomicBool, Ordering};

#[repr(C, align(16))]
#[derive(Clone,Copy)]
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
pub enum SliceTimer {
    Physical { interrupt: u8 },
    Virtual { interrupt: u8 },
}

#[derive(Clone, Copy)]
pub enum InterruptController {
    V2 { distributor: u64, cpu: u64, timer: SliceTimer },
    V3 { redistributor: u64, timer: SliceTimer },
}

impl InterruptController {
    // ------------------------=
    // FUNC: timer
    // DESC: Returns the firmware-validated private timer used for native process slices.
    // ------------------=
    const fn timer(self) -> SliceTimer {
        match self { Self::V2 { timer, .. } | Self::V3 { timer, .. } => timer }
    }
    // ------------------------=
    // FUNC: timer_interrupt
    // DESC: Exposes the validated timer interrupt for structured platform verification.
    // ------------------=
    pub const fn timer_interrupt(self) -> u8 { self.timer().interrupt() }
}

impl SliceTimer {
    // ------------------------=
    // FUNC: interrupt
    // DESC: Returns the firmware-advertised private interrupt number for this timer.
    // ------------------=
    pub const fn interrupt(self) -> u8 {
        match self { Self::Physical { interrupt } | Self::Virtual { interrupt } => interrupt }
    }
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
static mut USER_FRAME: core::mem::MaybeUninit<Registers> = core::mem::MaybeUninit::uninit();
static BUSY: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunError { Busy, UnsupportedLevel, InvalidContext }

core::arch::global_asm!(include_str!("native_task_arm.S"));
unsafe extern "C" { fn infinity_arm_enter(frame: *mut Registers, root: u64, stack: u64); }
unsafe extern "C" { static infinity_arm_code_start:u8; static infinity_arm_code_end:u8; }

// ------------------------=
// FUNC: map_trampoline
// DESC: Maps only native exception code, private trap stack and fixed register storage with supervisor permissions.
// ------------------=
pub unsafe fn map_trampoline<T:super::native_memory::PageTable>(tables:&mut T,memory:&mut super::native_memory::NativeMemory)->Result<(),super::native_memory::MemoryError> {
    use super::native_memory::PagePermissions as P;
    let start=(&raw const infinity_arm_code_start) as u64;
    let end=(&raw const infinity_arm_code_end) as u64;
    super::native_cpu::map_privileged(tables,memory,start,(end-start) as usize,P::KERNEL_RX)?;
    for (address,bytes) in [((&raw const TRAP_STACK) as u64,32768),
        ((&raw const infinity_arm_kernel) as u64,core::mem::size_of::<Saved>()),
        ((&raw const infinity_arm_registers) as u64,8),
        ((&raw const USER_FRAME) as u64,core::mem::size_of::<Registers>())] {
        super::native_cpu::map_privileged(tables,memory,address,bytes,P::KERNEL_RW)?;
    }
    Ok(())
}

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
    let timer=controller.timer();
    let timer_interrupt=timer.interrupt();
    let frequency: u64;
    core::arch::asm!("mrs {}, cntfrq_el0", out(reg) frequency);
    let (old_timer,old_compare,old_other_timer)=match timer {
        SliceTimer::Physical { .. } => {
            let (control,compare,virtual_control):(u64,u64,u64);
            core::arch::asm!("mrs {}, cntp_ctl_el0", "mrs {}, cntp_cval_el0", "mrs {}, cntv_ctl_el0",
                out(reg) control,out(reg) compare,out(reg) virtual_control);
            // Firmware may keep its virtual timer armed. Preserve and mask it so
            // only the selected physical slice timer can preempt this context.
            core::arch::asm!("msr cntv_ctl_el0, {}", "isb", in(reg) virtual_control|2);
            (control,compare,virtual_control)
        }
        SliceTimer::Virtual { .. } => {
            let (control,compare):(u64,u64);
            // Do not touch CNTP on virtualized platforms which reserve or trap it.
            core::arch::asm!("mrs {}, cntv_ctl_el0", "mrs {}, cntv_cval_el0",
                out(reg) control,out(reg) compare);
            (control,compare,0)
        }
    };
    let timer_bit=1u32<<timer_interrupt;
    let (bank, old_control, old_priority, old_group, enabled, old_sre) = match controller {
        InterruptController::V2 { distributor, cpu, .. } => {
            let control = read(cpu);
            let priority = read(cpu + 4);
            let group = read(distributor + 0x80);
            let enabled = read(distributor + 0x100);
            // Keep the firmware-selected security group. Moving the slice timer to
            // Group 1 silently disables slices when firmware enables only
            // distributor Group 0 (QEMU GICv2 without Security Extensions).
            write(distributor + 0x100, timer_bit);
            write(cpu + 4, 1); // only highest-priority private slice timer
            // AckCtl permits Group 1 acknowledgement through IAR when the
            // platform exposes the secure view (QEMU's non-security model).
            write(cpu, control | 7);
            (distributor, control as u64, priority as u64, group, enabled, 0)
        }
        InterruptController::V3 { redistributor, .. } => {
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
            write(bank + 0x80, group | timer_bit);
            write(bank + 0x100, timer_bit);
            core::arch::asm!("msr ICC_PMR_EL1, {}", "msr ICC_IGRPEN1_EL1, {}", "isb",
                in(reg) 1u64, in(reg) 1u64);
            (bank, control, priority, group, enabled, sre)
        }
    };
    let priority_address = (bank + 0x400 + timer_interrupt as u64) as *mut u8;
    let old_irq_priority = core::ptr::read_volatile(priority_address);
    core::ptr::write_volatile(priority_address, 0);
    let ticks = (frequency * microseconds as u64 / 1_000_000).max(1);
    match timer {
        SliceTimer::Physical { .. } => core::arch::asm!("msr cntp_tval_el0, {}", "msr cntp_ctl_el0, {}", "isb",
            in(reg) ticks,in(reg) 1u64),
        SliceTimer::Virtual { .. } => core::arch::asm!("msr cntv_tval_el0, {}", "msr cntv_ctl_el0, {}", "isb",
            in(reg) ticks,in(reg) 1u64),
    }
    frame.pstate &= 0xf0000000; // EL0t, IRQ enabled, no inherited privilege flags.
    let transition=(&raw mut USER_FRAME).cast::<Registers>();
    transition.write(*frame);
    infinity_arm_enter(transition, root, (&raw mut TRAP_STACK).cast::<u8>().add(32768) as u64);
    *frame=transition.read();
    if frame.reason == 1 {
        let interrupt = match controller {
            InterruptController::V2 { cpu, .. } => read(cpu + 12) as u64,
            InterruptController::V3 { .. } => {
                let id: u64;
                core::arch::asm!("mrs {}, ICC_IAR1_EL1", out(reg) id); id
            }
        };
        match timer {
            SliceTimer::Physical { .. } => core::arch::asm!("msr cntp_ctl_el0, {}", "isb", in(reg) 2u64),
            SliceTimer::Virtual { .. } => core::arch::asm!("msr cntv_ctl_el0, {}", "isb", in(reg) 2u64),
        }
        frame.syndrome = interrupt;
        if interrupt & 1023 < 1020 {
            match controller {
                InterruptController::V2 { cpu, .. } => write(cpu + 16, interrupt as u32),
                InterruptController::V3 { .. } => {
                    core::arch::asm!("msr ICC_EOIR1_EL1, {}", "isb", in(reg) interrupt);
                    let control: u64;
                    core::arch::asm!("mrs {}, ICC_CTLR_EL1", out(reg) control);
                    if control & 2 != 0 {
                        // Split EOI mode drops priority separately from deactivation.
                        // Leaving the acknowledged timer active prevents the next slice from firing.
                        core::arch::asm!("msr ICC_DIR_EL1, {}", "isb", in(reg) interrupt);
                    }
                }
            }
        }
    }
    match timer {
        SliceTimer::Physical { .. } => core::arch::asm!("msr cntp_ctl_el0, {}", "isb", in(reg) 2u64),
        SliceTimer::Virtual { .. } => core::arch::asm!("msr cntv_ctl_el0, {}", "isb", in(reg) 2u64),
    }
    if enabled & timer_bit == 0 { write(bank + 0x180, timer_bit); }
    write(bank + 0x80, old_group);
    core::ptr::write_volatile(priority_address, old_irq_priority);
    match controller {
        InterruptController::V2 { cpu, .. } => { write(cpu + 4, old_priority as u32); write(cpu, old_control as u32); }
        InterruptController::V3 { .. } => {
            core::arch::asm!("msr ICC_PMR_EL1, {}", "msr ICC_IGRPEN1_EL1, {}", "isb",
                "msr ICC_SRE_EL1, {}", "isb", in(reg) old_priority, in(reg) old_control, in(reg) old_sre);
        }
    }
    match timer {
        SliceTimer::Physical { .. } => {
            core::arch::asm!("msr cntp_cval_el0, {}", "msr cntp_ctl_el0, {}", "isb",
                in(reg) old_compare,in(reg) old_timer);
            core::arch::asm!("msr cntv_ctl_el0, {}", "isb", in(reg) old_other_timer);
        }
        SliceTimer::Virtual { .. } => core::arch::asm!("msr cntv_cval_el0, {}", "msr cntv_ctl_el0, {}", "isb",
            in(reg) old_compare,in(reg) old_timer),
    }
    BUSY.store(false, Ordering::Release);
    core::arch::asm!("msr daif, {}", in(reg) daif);
    Ok(())
}

// ------------------------=
// FUNC: read
// DESC: Reads a kernel-only interrupt-controller register.
// ------------------=
unsafe fn read(address: u64) -> u32 {
    // A pre/post-indexed MMIO load has no valid instruction syndrome on a
    // stage-two trap. Keep the plain scalar form required by HVF MMIO emulation.
    let value: u32;
    core::arch::asm!("ldr {value:w}, [{address}]", value = out(reg) value,
        address = in(reg) address, options(nostack, preserves_flags));
    value
}
// ------------------------=
// FUNC: write
// DESC: Publishes a kernel-only interrupt-controller register update before enabling the timer.
// ------------------=
unsafe fn write(address: u64, value: u32) {
    core::arch::asm!("str {value:w}, [{address}]", "dsb sy",
        value = in(reg) value, address = in(reg) address,
        options(nostack, preserves_flags));
}
