//! BSP-owned, bounded ring-3 execution slices. App faults return to the caller;
//! no app callback is ever invoked at CPL0. Requires the process mapping to
//! retain supervisor access to this module, its stacks, and the local APIC page.
use core::sync::atomic::{AtomicBool, Ordering};

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Registers {
    pub r15: u64, pub r14: u64, pub r13: u64, pub r12: u64,
    pub r11: u64, pub r10: u64, pub r9: u64, pub r8: u64,
    pub rdi: u64, pub rsi: u64, pub rbp: u64, pub rdx: u64,
    pub rcx: u64, pub rbx: u64, pub rax: u64,
    pub vector: u64, pub error: u64,
    pub rip: u64, pub cs: u64, pub flags: u64, pub rsp: u64, pub ss: u64,
    pub fs_base: u64, pub gs_base: u64,
}

#[repr(C, align(64))]
pub struct ExtendedState(pub [u8; 65536]);

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct Descriptor { limit: u16, base: u64 }

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct Gate { low: u16, selector: u16, ist: u8, flags: u8, middle: u16, high: u32, zero: u32 }
const EMPTY_GATE: Gate = Gate { low: 0, selector: 8, ist: 0, flags: 0, middle: 0, high: 0, zero: 0 };

#[repr(C, align(4096))]
struct Stack([u8; 32768]);
static mut TRAP_STACK: Stack = Stack([0; 32768]);
static mut TSS: [u8; 104] = [0; 104];
static mut GDT: [u64; 7] = [0; 7];
static mut IDT: [Gate; 256] = [EMPTY_GATE; 256];
static BUSY: AtomicBool = AtomicBool::new(false);
static mut KERNEL_STATE: ExtendedState = ExtendedState([0; 65536]);
static mut USER_FRAME: core::mem::MaybeUninit<Registers> = core::mem::MaybeUninit::uninit();

#[no_mangle]
static mut infinity_task_kernel_rsp: u64 = 0;
#[no_mangle]
static mut infinity_task_kernel_cr3: u64 = 0;
#[no_mangle]
static mut infinity_task_registers: u64 = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunError { Busy, UnsupportedCpu, InvalidContext }

core::arch::global_asm!(include_str!("native_task_x86.S"));
unsafe extern "C" {
    fn infinity_task_enter(registers: *mut Registers, root: u64, trap_stack: u64);
    fn infinity_task_restore_segments(code: u64, stack: u64);
    static infinity_task_vectors: [u64; 256];
    static infinity_task_code_start:u8;
    static infinity_task_code_end:u8;
}

// ------------------------=
// FUNC: map_trampoline
// DESC: Maps only the ring-transition code, descriptor tables, private trap stack and stable register frame into user process roots as supervisor-only.
// ------------------=
pub unsafe fn map_trampoline<T:super::native_memory::PageTable>(tables:&mut T,memory:&mut super::native_memory::NativeMemory)->Result<(),super::native_memory::MemoryError> {
    use super::native_memory::PagePermissions as P;
    let start=(&raw const infinity_task_code_start) as u64;
    let end=(&raw const infinity_task_code_end) as u64;
    super::native_cpu::map_privileged(tables,memory,start,(end-start) as usize,P::KERNEL_RX)?;
    super::native_cpu::map_privileged(tables,memory,(&raw const infinity_task_vectors) as u64,2048,P::KERNEL_R)?;
    for (address,bytes) in [((&raw const TRAP_STACK) as u64,32768),
        ((&raw const TSS) as u64,104),((&raw const GDT) as u64,56),((&raw const IDT) as u64,4096),
        ((&raw const infinity_task_kernel_rsp) as u64,8),((&raw const infinity_task_kernel_cr3) as u64,8),
        ((&raw const infinity_task_registers) as u64,8),((&raw const USER_FRAME) as u64,core::mem::size_of::<Registers>())] {
        super::native_cpu::map_privileged(tables,memory,address,bytes,P::KERNEL_RW)?;
    }
    Ok(())
}

// ------------------------=
// FUNC: extended_state
// DESC: Saves or restores all enabled architectural extended state, rejecting CPUs whose state exceeds the fixed bound.
// ------------------=
unsafe fn extended_state(state: *mut ExtendedState, restore: bool, mask: u64) {
    if mask != 0 {
        if restore { core::arch::asm!("xrstor64 [{}]", in(reg) state,
            in("eax") mask as u32, in("edx") (mask >> 32) as u32); }
        else { core::arch::asm!("xsave64 [{}]", in(reg) state,
            in("eax") mask as u32, in("edx") (mask >> 32) as u32); }
    } else if restore { core::arch::asm!("fxrstor64 [{}]", in(reg) state); }
    else { core::arch::asm!("fxsave64 [{}]", in(reg) state); }
}

// ------------------------=
// FUNC: state_mask
// DESC: Computes the enabled XSAVE mask without executing optional instructions on unsupported CPUs.
// ------------------=
unsafe fn state_mask() -> Result<u64, RunError> {
    let features = core::arch::x86_64::__cpuid(1);
    if features.edx & (1 << 24) == 0 { return Err(RunError::UnsupportedCpu); }
    if features.ecx & (1 << 27) == 0 { return Ok(0); }
    if core::arch::x86_64::__cpuid_count(13, 0).ebx > 65536 {
        return Err(RunError::UnsupportedCpu);
    }
    Ok(core::arch::x86_64::_xgetbv(0))
}

// ------------------------=
// FUNC: run
// DESC: Runs one timer-bounded user slice, preserving kernel state and returning a syscall, interrupt, or app fault.
// ------------------=
pub unsafe fn run(registers: &mut Registers, state: &mut ExtendedState, initialized: &mut bool,
    root: u64, apic: u64) -> Result<(), RunError> {
    if root == 0 || root & 4095 != 0 || apic == 0 || apic & 4095 != 0 ||
        registers.rip >= (1 << 47) || registers.rsp >= (1 << 47) ||
        registers.fs_base >= (1 << 47) || registers.gs_base >= (1 << 47) {
        return Err(RunError::InvalidContext);
    }
    let mask = state_mask()?;
    let mut apic_low: u32;
    let mut apic_high: u32;
    core::arch::asm!("rdmsr", in("ecx") 0x1bu32, out("eax") apic_low, out("edx") apic_high);
    if apic_low & (1 << 11) == 0 || apic_low & (1 << 10) != 0 ||
        (((apic_high as u64) << 32) | (apic_low as u64 & !4095)) != apic {
        return Err(RunError::UnsupportedCpu);
    }
    if BUSY.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        return Err(RunError::Busy);
    }
    let flags: u64;
    core::arch::asm!("pushfq", "pop {}", "cli", out(reg) flags);
    let code: u16; let stack_selector: u16; let task_selector: u16;
    core::arch::asm!("mov {0:x}, cs", "mov {1:x}, ss", "str {2:x}",
        out(reg) code, out(reg) stack_selector, out(reg) task_selector);
    let low: u32;
    let high: u32;
    core::arch::asm!("rdmsr", in("ecx") 0xc0000080u32, out("eax") low, out("edx") high);
    core::arch::asm!("wrmsr", in("ecx") 0xc0000080u32, in("eax") low | (1 << 11), in("edx") high);
    let cr0: u64;
    core::arch::asm!("mov {}, cr0", out(reg) cr0);
    core::arch::asm!("mov cr0, {}", in(reg) cr0 | (1 << 16));
    let mut old_gdt = Descriptor { limit: 0, base: 0 };
    let mut old_idt = old_gdt;
    core::arch::asm!("sgdt [{}]", "sidt [{}]", in(reg) &mut old_gdt, in(reg) &mut old_idt);
    let stack = (&raw mut TRAP_STACK).cast::<u8>().add(32768) as u64;
    core::ptr::write_unaligned((&raw mut TSS).cast::<u8>().add(4).cast::<u64>(), stack);
    core::ptr::write_unaligned((&raw mut TSS).cast::<u8>().add(102).cast::<u16>(), 104);
    let tss = (&raw mut TSS) as u64;
    GDT = [0, 0x00af9a000000ffff, 0x00cf92000000ffff,
        0x00cff2000000ffff, 0x00affa000000ffff,
        103 | ((tss & 0xffffff) << 16) | (0x89 << 40) | (((tss >> 24) & 255) << 56), tss >> 32];
    for index in 0..256 {
        let entry = infinity_task_vectors[index];
        IDT[index] = Gate { low: entry as u16, selector: 8, ist: 0,
            flags: if index == 128 { 0xee } else { 0x8e }, middle: (entry >> 16) as u16,
            high: (entry >> 32) as u32, zero: 0 };
    }
    let gdt = Descriptor { limit: 55, base: (&raw const GDT) as u64 };
    let idt = Descriptor { limit: 4095, base: (&raw const IDT) as u64 };
    let register = |offset| (apic + offset) as *mut u32;
    let old_svr = core::ptr::read_volatile(register(0xf0));
    let old_tpr = core::ptr::read_volatile(register(0x80));
    let old_lvt = core::ptr::read_volatile(register(0x320));
    let old_divide = core::ptr::read_volatile(register(0x3e0));
    let old_count = core::ptr::read_volatile(register(0x390));
    let pic_master: u8;
    let pic_slave: u8;
    core::arch::asm!("in al, dx", in("dx") 0x21u16, out("al") pic_master);
    core::arch::asm!("in al, dx", in("dx") 0xa1u16, out("al") pic_slave);
    core::arch::asm!("out dx, al", in("dx") 0x21u16, in("al") 255u8);
    core::arch::asm!("out dx, al", in("dx") 0xa1u16, in("al") 255u8);
    extended_state(&raw mut KERNEL_STATE, false, mask);
    if !*initialized {
        state.0.fill(0);
        // Architectural initial x87/MXCSR state; absent XSAVE components initialize on restore.
        state.0[0..2].copy_from_slice(&0x37fu16.to_le_bytes());
        state.0[24..28].copy_from_slice(&0x1f80u32.to_le_bytes());
        *initialized = true;
    }
    registers.cs = 0x23;
    registers.ss = 0x1b;
    registers.flags = (registers.flags & 0xcd5) | 0x202;
    core::arch::asm!("lgdt [{}]", "lidt [{}]", "ltr ax",
        in(reg) &gdt, in(reg) &idt, in("ax") 0x28u16);
    core::ptr::write_volatile(register(0xf0), old_svr | 0x100);
    core::ptr::write_volatile(register(0x80), 0);
    core::ptr::write_volatile(register(0x3e0), 3); // divide by 16
    core::ptr::write_volatile(register(0x320), 32);
    core::ptr::write_volatile(register(0x380), 100_000); // finite one-shot; caller counts actual elapsed time
    let kernel_fs = read_msr(0xc0000100);
    let kernel_gs = read_msr(0xc0000101);
    write_msr(0xc0000100, registers.fs_base);
    write_msr(0xc0000101, registers.gs_base);
    extended_state(state, true, mask);
    let transition=(&raw mut USER_FRAME).cast::<Registers>();
    transition.write(*registers);
    infinity_task_enter(transition, root, stack);
    *registers=transition.read();
    extended_state(state, false, mask);
    extended_state(&raw mut KERNEL_STATE, true, mask);
    registers.fs_base = read_msr(0xc0000100);
    registers.gs_base = read_msr(0xc0000101);
    write_msr(0xc0000100, kernel_fs);
    write_msr(0xc0000101, kernel_gs);
    core::ptr::write_volatile(register(0x320), 1 << 16);
    if registers.vector >= 32 && registers.vector != 128 && registers.vector != 255 {
        core::ptr::write_volatile(register(0xb0), 0);
    }
    core::ptr::write_volatile(register(0x3e0), old_divide);
    core::ptr::write_volatile(register(0x320), old_lvt);
    core::ptr::write_volatile(register(0x380), old_count);
    core::ptr::write_volatile(register(0x80), old_tpr);
    core::ptr::write_volatile(register(0xf0), old_svr);
    core::arch::asm!("lgdt [{}]", "lidt [{}]", in(reg) &old_gdt, in(reg) &old_idt);
    infinity_task_restore_segments(code as u64, stack_selector as u64);
    if task_selector != 0 && task_selector & 4 == 0 && (task_selector as usize & !7) + 15 <= old_gdt.limit as usize {
        // LTR requires an available TSS descriptor. Restore its busy state through LTR itself.
        let access = (old_gdt.base + (task_selector as u64 & !7) + 5) as *mut u8;
        let value = core::ptr::read_volatile(access);
        if value & 15 == 11 {
            core::ptr::write_volatile(access, value & !2);
            core::arch::asm!("ltr ax", in("ax") task_selector);
        }
    }
    core::arch::asm!("mov cr0, {}", in(reg) cr0);
    core::arch::asm!("wrmsr", in("ecx") 0xc0000080u32, in("eax") low, in("edx") high);
    core::arch::asm!("out dx, al", in("dx") 0x21u16, in("al") pic_master);
    core::arch::asm!("out dx, al", in("dx") 0xa1u16, in("al") pic_slave);
    BUSY.store(false, Ordering::Release);
    core::arch::asm!("push {}", "popfq", in(reg) flags);
    Ok(())
}

// ------------------------=
// FUNC: read_msr
// DESC: Reads kernel-managed thread-pointer state across a user execution slice.
// ------------------=
unsafe fn read_msr(index: u32) -> u64 {
    let low: u32; let high: u32;
    core::arch::asm!("rdmsr", in("ecx") index, out("eax") low, out("edx") high);
    (high as u64) << 32 | low as u64
}
// ------------------------=
// FUNC: write_msr
// DESC: Installs a validated user thread pointer or restores the original kernel value.
// ------------------=
unsafe fn write_msr(index: u32, value: u64) {
    core::arch::asm!("wrmsr", in("ecx") index, in("eax") value as u32, in("edx") (value >> 32) as u32);
}
