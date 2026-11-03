.text
.p2align 2
.globl infinity_handoff
.globl infinity_sync_code

// ------------------------=
// FUNC: infinity_sync_code
// DESC: Publishes ELF text copied through the data cache before any CPU fetches it as instructions.
// ------------------=
infinity_sync_code:
    mrs x4, ctr_el0
    ubfx x5, x4, #16, #4
    mov x6, #4
    lsl x5, x6, x5
    sub x7, x5, #1
    bic x2, x0, x7
.Lclean_code:
    dc cvau, x2
    add x2, x2, x5
    cmp x2, x1
    b.lo .Lclean_code
    dsb ish
    and x4, x4, #15
    lsl x5, x6, x4
    sub x7, x5, #1
    bic x2, x0, x7
.Linvalidate_code:
    ic ivau, x2
    add x2, x2, x5
    cmp x2, x1
    b.lo .Linvalidate_code
    dsb ish
    isb
    ret

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
    mrs x9, daif
    msr daifset, #0xf
    mrs x10, spsel
    stp x19, x20, [sp, #-48]!
    stp x21, x22, [sp, #16]
    stp x23, x30, [sp, #32]
    mov x19, sp
    mov x20, x9
    mov x21, x10
    // The firmware synchronous vector selects SP_EL0 before spilling. Keep
    // that emergency stack separate from the live native SP_ELx frames.
    msr spsel, #1
    mrs x22, sp_el0
    mov x23, sp
    ldp x9, x10, [x0]
    ldr x11, [x0, #80]
    ldr x0, [x0, #16]
    msr sp_el0, x11
    mov sp, x9
    blr x10
    msr daifset, #0xf
    mov sp, x23
    msr sp_el0, x22
    msr spsel, x21
    mov sp, x19
    mov x9, x20
    ldp x21, x22, [sp, #16]
    ldp x23, x30, [sp, #32]
    ldp x19, x20, [sp], #48
    msr daif, x9
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
    ldr x13, [x0, #80]      // private emergency exception stack
    msr sp_el0, x13
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
