.arch armv8-a+simd
.text
.global infinity_arm_code_start
infinity_arm_code_start:
.p2align 4
.global infinity_arm_enter
// ------------------------=
// FUNC: infinity_arm_enter
// DESC: Saves kernel state, installs private translations, and enters EL0 with a private exception stack.
// ------------------=
infinity_arm_enter:
    adrp x9, infinity_arm_registers
    str x0, [x9, :lo12:infinity_arm_registers]
    adrp x9, infinity_arm_kernel
    add x9, x9, :lo12:infinity_arm_kernel
    stp x19, x20, [x9, #0]
    stp x21, x22, [x9, #16]
    stp x23, x24, [x9, #32]
    stp x25, x26, [x9, #48]
    stp x27, x28, [x9, #64]
    stp x29, x30, [x9, #80]
    mov x10, sp
    mrs x11, spsel
    stp x10, x11, [x9, #96]
    msr spsel, #1
    mov x10, sp
    mrs x11, sp_el0
    stp x10, x11, [x9, #112]
    mov sp, x2
    mrs x10, ttbr0_el1
    mrs x11, ttbr1_el1
    stp x10, x11, [x9, #128]
    mrs x10, tcr_el1
    mrs x11, mair_el1
    stp x10, x11, [x9, #144]
    mrs x10, sctlr_el1
    mrs x11, vbar_el1
    stp x10, x11, [x9, #160]
    mrs x10, tpidr_el0
    mrs x11, cpacr_el1
    stp x10, x11, [x9, #176]
    orr x12, x11, #(3 << 20)
    bic x12, x12, #(3 << 16)
    bic x12, x12, #(3 << 24)
    msr cpacr_el1, x12
    isb
    mrs x10, fpcr
    mrs x11, fpsr
    stp x10, x11, [x9, #192]
    stp q0, q1, [x9, #208]
    stp q2, q3, [x9, #240]
    stp q4, q5, [x9, #272]
    stp q6, q7, [x9, #304]
    stp q8, q9, [x9, #336]
    stp q10, q11, [x9, #368]
    stp q12, q13, [x9, #400]
    stp q14, q15, [x9, #432]
    stp q16, q17, [x9, #464]
    stp q18, q19, [x9, #496]
    stp q20, q21, [x9, #528]
    stp q22, q23, [x9, #560]
    stp q24, q25, [x9, #592]
    stp q26, q27, [x9, #624]
    stp q28, q29, [x9, #656]
    stp q30, q31, [x9, #688]
    mrs x10, tpidrro_el0
    str x10, [x9, #720]
    msr tpidrro_el0, xzr
    // Both old and new kernel mappings are identity mappings. Disable translation
    // for the format transition; never let old descriptors use the new TCR.
    dsb sy
    mrs x10, sctlr_el1
    bic x10, x10, #1
    msr sctlr_el1, x10
    isb
    msr ttbr0_el1, x1
    mov x11, #0xff
    msr mair_el1, x11
    ldr x11, =0x0000000280803510
    msr tcr_el1, x11
    adr x11, infinity_arm_vectors
    msr vbar_el1, x11
    tlbi vmalle1
    dsb sy
    isb
    orr x10, x10, #1
    msr sctlr_el1, x10
    isb
    mov x30, x0
    ldr x10, [x30, #248]
    msr sp_el0, x10
    ldr x10, [x30, #256]
    msr elr_el1, x10
    ldr x10, [x30, #264]
    msr spsr_el1, x10
    ldr x10, [x30, #296]
    msr tpidr_el0, x10
    ldp q0, q1, [x30, #304]
    ldp q2, q3, [x30, #336]
    ldp q4, q5, [x30, #368]
    ldp q6, q7, [x30, #400]
    ldp q8, q9, [x30, #432]
    ldp q10, q11, [x30, #464]
    ldp q12, q13, [x30, #496]
    ldp q14, q15, [x30, #528]
    ldp q16, q17, [x30, #560]
    ldp q18, q19, [x30, #592]
    ldp q20, q21, [x30, #624]
    ldp q22, q23, [x30, #656]
    ldp q24, q25, [x30, #688]
    ldp q26, q27, [x30, #720]
    ldp q28, q29, [x30, #752]
    ldp q30, q31, [x30, #784]
    ldr x10, [x30, #816]
    ldr x11, [x30, #824]
    msr fpcr, x10
    msr fpsr, x11
    ldp x0, x1, [x30]
    ldp x2, x3, [x30, #16]
    ldp x4, x5, [x30, #32]
    ldp x6, x7, [x30, #48]
    ldp x8, x9, [x30, #64]
    ldp x10, x11, [x30, #80]
    ldp x12, x13, [x30, #96]
    ldp x14, x15, [x30, #112]
    ldp x16, x17, [x30, #128]
    ldp x18, x19, [x30, #144]
    ldp x20, x21, [x30, #160]
    ldp x22, x23, [x30, #176]
    ldp x24, x25, [x30, #192]
    ldp x26, x27, [x30, #208]
    ldp x28, x29, [x30, #224]
    ldr x30, [x30, #240]
    eret

.balign 2048
infinity_arm_vectors:
.rept 8
    b infinity_arm_kernel_fault

    .balign 128
.endr
    b infinity_arm_sync
    .balign 128
    b infinity_arm_irq
    .balign 128
.rept 6
    b infinity_arm_kernel_fault
    .balign 128
.endr

// ------------------------=
// FUNC: infinity_arm_sync
// DESC: Captures a lower-level syscall or fault without treating it as a kernel failure.
// ------------------=
infinity_arm_sync:
    stp x0, x1, [sp, #-16]!
    mov x1, #0
    b infinity_arm_trap
// ------------------------=
// FUNC: infinity_arm_irq
// DESC: Returns a timer/interrupt boundary to the kernel scheduler.
// ------------------=
infinity_arm_irq:
    stp x0, x1, [sp, #-16]!
    mov x1, #1
infinity_arm_trap:
    msr daifset, #0xf
    adrp x0, infinity_arm_registers
    ldr x0, [x0, :lo12:infinity_arm_registers]
    str x1, [x0, #288]
    stp x2, x3, [x0, #16]
    stp x4, x5, [x0, #32]
    stp x6, x7, [x0, #48]
    stp x8, x9, [x0, #64]
    stp x10, x11, [x0, #80]
    stp x12, x13, [x0, #96]
    stp x14, x15, [x0, #112]
    stp x16, x17, [x0, #128]
    stp x18, x19, [x0, #144]
    stp x20, x21, [x0, #160]
    stp x22, x23, [x0, #176]
    stp x24, x25, [x0, #192]
    stp x26, x27, [x0, #208]
    stp x28, x29, [x0, #224]
    str x30, [x0, #240]
    ldp x1, x2, [sp], #16
    stp x1, x2, [x0]
    mrs x1, sp_el0
    mrs x2, elr_el1
    stp x1, x2, [x0, #248]
    mrs x1, spsr_el1
    mrs x2, esr_el1
    stp x1, x2, [x0, #264]
    mrs x1, far_el1
    str x1, [x0, #280]
    mrs x1, tpidr_el0
    str x1, [x0, #296]
    stp q0, q1, [x0, #304]
    stp q2, q3, [x0, #336]
    stp q4, q5, [x0, #368]
    stp q6, q7, [x0, #400]
    stp q8, q9, [x0, #432]
    stp q10, q11, [x0, #464]
    stp q12, q13, [x0, #496]
    stp q14, q15, [x0, #528]
    stp q16, q17, [x0, #560]
    stp q18, q19, [x0, #592]
    stp q20, q21, [x0, #624]
    stp q22, q23, [x0, #656]
    stp q24, q25, [x0, #688]
    stp q26, q27, [x0, #720]
    stp q28, q29, [x0, #752]
    stp q30, q31, [x0, #784]
    mrs x1, fpcr
    mrs x2, fpsr
    str x1, [x0, #816]
    str x2, [x0, #824]
    adrp x9, infinity_arm_kernel
    add x9, x9, :lo12:infinity_arm_kernel
    dsb sy
    mrs x10, sctlr_el1
    bic x10, x10, #1
    msr sctlr_el1, x10
    isb
    ldp x10, x11, [x9, #128]
    msr ttbr0_el1, x10
    msr ttbr1_el1, x11
    ldp x10, x11, [x9, #144]
    msr tcr_el1, x10
    msr mair_el1, x11
    ldp x10, x11, [x9, #160]
    msr vbar_el1, x11
    tlbi vmalle1
    dsb sy
    isb
    msr sctlr_el1, x10
    isb
    ldp q0, q1, [x9, #208]
    ldp q2, q3, [x9, #240]
    ldp q4, q5, [x9, #272]
    ldp q6, q7, [x9, #304]
    ldp q8, q9, [x9, #336]
    ldp q10, q11, [x9, #368]
    ldp q12, q13, [x9, #400]
    ldp q14, q15, [x9, #432]
    ldp q16, q17, [x9, #464]
    ldp q18, q19, [x9, #496]
    ldp q20, q21, [x9, #528]
    ldp q22, q23, [x9, #560]
    ldp q24, q25, [x9, #592]
    ldp q26, q27, [x9, #624]
    ldp q28, q29, [x9, #656]
    ldp q30, q31, [x9, #688]
    ldr x10, [x9, #720]
    msr tpidrro_el0, x10
    ldp x10, x11, [x9, #192]
    msr fpcr, x10
    msr fpsr, x11
    ldp x10, x11, [x9, #176]
    msr tpidr_el0, x10
    msr cpacr_el1, x11
    ldp x10, x11, [x9, #112]
    mov sp, x10
    msr sp_el0, x11
    ldp x10, x11, [x9, #96]
    msr spsel, x11
    mov sp, x10
    ldp x19, x20, [x9]
    ldp x21, x22, [x9, #16]
    ldp x23, x24, [x9, #32]
    ldp x25, x26, [x9, #48]
    ldp x27, x28, [x9, #64]
    ldp x29, x30, [x9, #80]
    ret

infinity_arm_kernel_fault:
    msr daifset, #0xf
    wfi
    b infinity_arm_kernel_fault
.ltorg
.global infinity_arm_code_end
infinity_arm_code_end:
