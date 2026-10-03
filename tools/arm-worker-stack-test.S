.text
.global _start
// ------------------------=
// FUNC: _start
// DESC: Enters a disposable EL1 guest with controlled stacks and test exception vectors.
// ------------------=
_start:
    msr daifset, #0xf
    msr spsel, #1
    ldr x0, =0x41000000
    mov sp, x0
    adr x0, test_vectors
    msr vbar_el1, x0
    isb
    bl test_main
    b .

.global run_mp_case
// ------------------------=
// FUNC: run_mp_case
// DESC: Calls the production MP trampoline from each stack mode and records incoming and restored ABI state.
// ------------------=
run_mp_case:
    stp x19, x20, [sp, #-96]!
    stp x21, x22, [sp, #16]
    stp x23, x24, [sp, #32]
    stp x25, x26, [sp, #48]
    stp x27, x28, [sp, #64]
    stp x29, x30, [sp, #80]
    mov x19, sp
    mov x20, x2
    mov x21, x1
    mrs x22, daif
    mrs x23, sp_el0
    msr sp_el0, x3
    msr spsel, x21
    mov x9, #0x340
    msr daif, x9
    mov x9, sp
    str x9, [x20, #120]
    str x3, [x20, #128]
    stp x19, x20, [x20, #136]
    stp x21, x22, [x20, #152]
    str x23, [x20, #168]
    bl infinity_ap_callback
    mov x9, sp
    str x9, [x20, #48]
    mrs x9, spsel
    str x9, [x20, #56]
    mrs x9, daif
    str x9, [x20, #64]
    msr daifset, #0xf
    msr spsel, #1
    mrs x9, sp_el0
    str x9, [x20, #40]
    stp x19, x20, [x20, #80]
    stp x21, x22, [x20, #96]
    str x23, [x20, #112]
    msr sp_el0, x23
    mov sp, x19
    msr daif, x22
    ldp x21, x22, [sp, #16]
    ldp x23, x24, [sp, #32]
    ldp x25, x26, [sp, #48]
    ldp x27, x28, [sp, #64]
    ldp x29, x30, [sp, #80]
    ldp x19, x20, [sp], #96
    ret

.global test_callback
// ------------------------=
// FUNC: test_callback
// DESC: Records the actual worker stack state and triggers a synchronous exception before a normal callback return.
// ------------------=
test_callback:
    sub sp, sp, #16
    str x30, [sp, #8]
    mov x9, #0xc0fe
    str x9, [sp]
    mrs x9, sp_el0
    str x9, [x0]
    mov x9, sp
    str x9, [x0, #8]
    mrs x9, spsel
    str x9, [x0, #16]
    mrs x9, daif
    str x9, [x0, #24]
    str x0, [x0, #32]
.global test_fault_instruction
test_fault_instruction:
    brk #0x123
    ldr x9, [sp]
    str x9, [x0, #72]
    ldr x30, [sp, #8]
    add sp, sp, #16
    mov x9, #0x999
    mov x10, #0xaaa
    mov x11, #0xbbb
    ret

.balign 2048
test_vectors:
.rept 4
    b test_unexpected_exception
    .balign 128
.endr
    b test_sync_exception
    .balign 128
.rept 11
    b test_unexpected_exception
    .balign 128
.endr
// ------------------------=
// FUNC: test_sync_exception
// DESC: Exercises EDK2's exact alternate-stack spill geometry while preserving the original exception for typed assertions.
// ------------------=
test_sync_exception:
    msr spsel, #0
    sub sp, sp, #0x230
    stp x0, x1, [sp, #-0x100]!
    stp x2, x3, [sp, #16]
    adrp x0, fault_record
    add x0, x0, :lo12:fault_record
    mrs x1, elr_el1
    str x1, [x0]
    add x1, x1, #4
    msr elr_el1, x1
    mrs x1, esr_el1
    str x1, [x0, #8]
    mov x1, sp
    str x1, [x0, #16]
    ldr x1, [x0, #24]
    add x1, x1, #1
    str x1, [x0, #24]
    ldp x2, x3, [sp, #16]
    ldp x0, x1, [sp], #0x100
    add sp, sp, #0x230
    msr spsel, #1
    eret
// ------------------------=
// FUNC: test_unexpected_exception
// DESC: Makes an unexpected guest fault fail by withholding the bounded binary success result.
// ------------------=
test_unexpected_exception:
    b .
