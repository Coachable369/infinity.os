/* Disposable single-CPU fixture: identity-map RAM as normal cacheable memory,
 * MMIO below 1 GiB as device memory. This is not process isolation. */
.section .data
.balign 4096
probe_translation_table:
.quad 0x0060000000000401
.quad 0x0000000040000705
.quad 0x0000000080000705
.quad 0x00000000c0000705
.zero 4064
.section .text
// ------------------------=
// FUNC: probe_memory
// DESC: Enables identity translation so real C libraries can use unaligned accesses to normal RAM.
// ------------------=
.global probe_memory
probe_memory:
    ldr x0, =probe_translation_table
    msr ttbr0_el1, x0
    mov x0, #0xff04
    msr mair_el1, x0
    ldr x0, =0x200003520
    msr tcr_el1, x0
    dsb sy
    isb
    tlbi vmalle1
    dsb sy
    isb
    mrs x0, sctlr_el1
    orr x0, x0, #1
    orr x0, x0, #4
    orr x0, x0, #4096
    bic x0, x0, #2
    msr sctlr_el1, x0
    isb
    ret
