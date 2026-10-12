.text
.global infinity_task_code_start
infinity_task_code_start:
.global infinity_task_restore_segments
// ------------------------=
// FUNC: infinity_task_restore_segments
// DESC: Restores the caller's code and stack selectors after reinstalling its descriptor table.
// ------------------=
infinity_task_restore_segments:
    mov ss, si
    push rdi
    lea rax, [rip + 1f]
    push rax
    retfq
1:
    ret

.global infinity_task_enter
// ------------------------=
// FUNC: infinity_task_enter
// DESC: Switches to the checked process root and returns through CPL3, never by a kernel call to app code.
// ------------------=
infinity_task_enter:
    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15
    mov [rip + infinity_task_kernel_rsp], rsp
    mov rax, cr3
    mov [rip + infinity_task_kernel_cr3], rax
    mov [rip + infinity_task_registers], rdi
    mov rsp, rdx
    mov cr3, rsi
    push qword ptr [rdi + 168]
    push qword ptr [rdi + 160]
    push qword ptr [rdi + 152]
    push qword ptr [rdi + 144]
    push qword ptr [rdi + 136]
    mov r15, [rdi]
    mov r14, [rdi + 8]
    mov r13, [rdi + 16]
    mov r12, [rdi + 24]
    mov r11, [rdi + 32]
    mov r10, [rdi + 40]
    mov r9, [rdi + 48]
    mov r8, [rdi + 56]
    mov rsi, [rdi + 72]
    mov rbp, [rdi + 80]
    mov rdx, [rdi + 88]
    mov rcx, [rdi + 96]
    mov rbx, [rdi + 104]
    mov rax, [rdi + 112]
    mov rdi, [rdi + 64]
    iretq

// ------------------------=
// FUNC: infinity_task_trap
// DESC: Captures app registers and restores the kernel root/stack before returning the slice result.
// ------------------=
infinity_task_trap:
    cld
    push rax
    push rbx
    push rcx
    push rdx
    push rbp
    push rsi
    push rdi
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15
    // A CPL0 exception is a kernel bug, not an app fault. Never disguise it.
    test byte ptr [rsp + 144], 3
    jz infinity_task_kernel_fault
    mov rsi, rsp
    mov rdi, [rip + infinity_task_registers]
    mov ecx, 22
    rep movsq
    mov rax, [rip + infinity_task_kernel_cr3]
    mov cr3, rax
    mov rsp, [rip + infinity_task_kernel_rsp]
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp
    ret
infinity_task_kernel_fault:
    cli
    hlt
    jmp infinity_task_kernel_fault

.altmacro
.macro TASK_VECTOR index
.p2align 4
infinity_task_vector_\index:
    .if (\index != 8) && (\index != 10) && (\index != 11) && (\index != 12) && (\index != 13) && (\index != 14) && (\index != 17) && (\index != 21) && (\index != 29) && (\index != 30)
    push 0
    .endif
    push \index
    jmp infinity_task_trap
.endm
.set vector_number, 0
.rept 256
TASK_VECTOR %vector_number
.set vector_number, vector_number + 1
.endr

.global infinity_task_code_end
infinity_task_code_end:
.section .rodata
.global infinity_task_vectors
.p2align 3
infinity_task_vectors:
.macro TASK_ADDRESS index
    .quad infinity_task_vector_\index
.endm
.set vector_number, 0
.rept 256
TASK_ADDRESS %vector_number
.set vector_number, vector_number + 1
.endr
