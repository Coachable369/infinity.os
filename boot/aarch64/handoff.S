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
