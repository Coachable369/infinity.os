.text
.p2align 2
.globl infinity_handoff

// UEFI AArch64 and the Rust AAPCS64 kernel ABI both receive arguments in x0-x3:
//   x0 = BootInfo*, x1 = ELF entry, x2 = stack top, x3 = reserved
// ------------------------=
// FUNC: infinity_handoff
// DESC: Establishes the kernel stack and transfers UEFI control to the Rust entry point.
// ------------------=
infinity_handoff:
    msr daifset, #0xf
    mrs x4, CurrentEL
    cmp x4, #4
    b.eq .Lready
    cmp x4, #8
    b.ne .Lpark
.Lready:
    mov sp, x2
    mov x29, xzr
    mov x30, xzr
    // Boot services remain active for the temporary firmware input bridge.
    msr daifclr, #0xf
    br x1
.Lpark:
    wfe
    b .Lpark

.p2align 6
.globl infinity_ap_callback
// ------------------------=
// FUNC: infinity_ap_callback
// DESC: Switches firmware MP callbacks to a reserved native stack, preserving the firmware return context.
// ------------------=
infinity_ap_callback:
    stp x19, x30, [sp, #-16]!
    mov x19, sp
    ldp x9, x10, [x0]
    ldr x0, [x0, #16]
    mov sp, x9
    blr x10
    mov sp, x19
    ldp x19, x30, [sp], #16
    ret

.p2align 6
.globl infinity_ap_entry
// ------------------------=
// FUNC: infinity_ap_entry
// DESC: Restores validated identity-mapped EL1 translation state on a PSCI-started AP before entering its native worker.
// ------------------=
infinity_ap_entry:
    msr daifset, #0xf
    mrs x4, CurrentEL
    cmp x4, #4
    b.ne .Lpark
    msr spsel, #1
    ldp x9, x10, [x0]       // stack, callback
    ldr x11, [x0, #16]      // callback argument
    ldr x12, [x0, #32]      // SCTLR
    ldr x4, [x0, #40]
    msr tcr_el1, x4
    ldr x4, [x0, #48]
    msr ttbr0_el1, x4
    ldr x4, [x0, #56]
    msr ttbr1_el1, x4
    ldr x4, [x0, #64]
    msr mair_el1, x4
    ldr x4, [x0, #72]
    msr vbar_el1, x4
    dsb sy
    isb
    tlbi vmalle1
    ic iallu
    dsb sy
    isb
    msr sctlr_el1, x12
    isb
    mov sp, x9
    mov x0, x11
    mov x29, xzr
    blr x10
    b .Lpark
