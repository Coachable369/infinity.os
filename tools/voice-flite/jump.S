/* Private error recovery: never unwinds across Rust or another worker's stack. */
#ifdef __APPLE__
#define ENTRY(x) _##x
#else
#define ENTRY(x) x
#endif
.text
.p2align 2
.global ENTRY(infinity_flite_setjmp)
// ------------------------=
// FUNC: infinity_flite_setjmp
// DESC: Saves the ABI callee-preserved context at the C-only synthesis boundary.
// ------------------=
ENTRY(infinity_flite_setjmp):
#if defined(__aarch64__)
    stp x19, x20, [x0, #0]
    stp x21, x22, [x0, #16]
    stp x23, x24, [x0, #32]
    stp x25, x26, [x0, #48]
    stp x27, x28, [x0, #64]
    stp x29, x30, [x0, #80]
    mov x9, sp
    str x9, [x0, #96]
    stp d8, d9, [x0, #104]
    stp d10, d11, [x0, #120]
    stp d12, d13, [x0, #136]
    stp d14, d15, [x0, #152]
    mov w0, #0
    ret
#elif defined(__x86_64__)
    movq %rbx, 0(%rdi)
    movq %rbp, 8(%rdi)
    movq %r12, 16(%rdi)
    movq %r13, 24(%rdi)
    movq %r14, 32(%rdi)
    movq %r15, 40(%rdi)
    leaq 8(%rsp), %rax
    movq %rax, 48(%rdi)
    movq (%rsp), %rax
    movq %rax, 56(%rdi)
    xorl %eax, %eax
    ret
#endif
.global ENTRY(infinity_flite_longjmp)
// ------------------------=
// FUNC: infinity_flite_longjmp
// DESC: Returns a nonzero error to the owning synthesis call with its preserved registers intact.
// ------------------=
ENTRY(infinity_flite_longjmp):
#if defined(__aarch64__)
    ldp x19, x20, [x0, #0]
    ldp x21, x22, [x0, #16]
    ldp x23, x24, [x0, #32]
    ldp x25, x26, [x0, #48]
    ldp x27, x28, [x0, #64]
    ldp x29, x30, [x0, #80]
    ldr x9, [x0, #96]
    mov sp, x9
    ldp d8, d9, [x0, #104]
    ldp d10, d11, [x0, #120]
    ldp d12, d13, [x0, #136]
    ldp d14, d15, [x0, #152]
    mov w0, #1
    ret
#elif defined(__x86_64__)
    movq 0(%rdi), %rbx
    movq 8(%rdi), %rbp
    movq 16(%rdi), %r12
    movq 24(%rdi), %r13
    movq 32(%rdi), %r14
    movq 40(%rdi), %r15
    movq 48(%rdi), %rsp
    movl $1, %eax
    jmp *56(%rdi)
#endif
