//! Native cooperative stack switching. This is not a scheduler or a std thread provider.
//! Caller owns stack lifetime, exclusion, TLS activation and scheduling policy.

#[repr(C, align(16))]
pub struct Context { words: [u64; 24] }

impl Context {
    // ------------------------=
    // FUNC: empty
    // DESC: Reserves a context which becomes resumable only after initialization or its first save.
    // ------------------=
    pub const fn empty() -> Self { Self { words: [0; 24] } }

    // ------------------------=
    // FUNC: initialize
    // DESC: Prepares an exclusive native stack to enter a nonreturning thread trampoline.
    // ------------------=
    /// # Safety
    /// The stack must remain live, exclusively owned and stationary until execution
    /// terminates. The caller must not access it while that context runs. Entry must
    /// never unwind across the switch. Thread-local machine state is the scheduler's
    /// responsibility; switching a stack does not create a resource/capability owner.
    pub unsafe fn initialize(&mut self, stack: &mut [u8], entry: extern "C" fn() -> !) -> bool {
        if stack.len() < 4096 { return false; }
        let top = (stack.as_mut_ptr() as usize + stack.len()) & !15;
        self.words = [0; 24];
        #[cfg(target_arch = "aarch64")]
        {
            self.words[11] = entry as usize as u64;
            self.words[12] = top as u64;
        }
        #[cfg(target_arch = "x86_64")]
        {
            let sp = top - 16;
            (sp as *mut u64).write(entry as usize as u64);
            ((sp + 8) as *mut u64).write(0);
            self.words[0] = sp as u64;
            // MXCSR and x87 control word occupy offsets 56 and 60.
            self.words[7] = 0x037f_0000_1f80;
        }
        true
    }
}

unsafe extern "C" { fn infinity_context_swap(old: *mut Context, new: *const Context); }

// ------------------------=
// FUNC: switch
// DESC: Saves the current ABI-preserved registers and resumes an initialized or previously suspended stack.
// ------------------=
/// # Safety
/// Both pointers must be aligned, nonoverlapping and valid; new must be resumable
/// with an exclusive live stack. Neither context may execute on another CPU.
/// Interrupt/trap state and machine TLS must be managed by the caller. Contexts
/// must not unwind across this boundary. This function returns when old is resumed.
pub unsafe fn switch(old: *mut Context, new: *const Context) {
    infinity_context_swap(old, new);
}

#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(r#"
.arch_extension fp
.section .text.infinity_context_swap,"ax"
.global infinity_context_swap
infinity_context_swap:
    stp x19, x20, [x0, #0]
    stp x21, x22, [x0, #16]
    stp x23, x24, [x0, #32]
    stp x25, x26, [x0, #48]
    stp x27, x28, [x0, #64]
    stp x29, x30, [x0, #80]
    mov x2, sp
    str x2, [x0, #96]
    stp d8, d9, [x0, #112]
    stp d10, d11, [x0, #128]
    stp d12, d13, [x0, #144]
    stp d14, d15, [x0, #160]
    mrs x2, fpcr
    mrs x3, fpsr
    stp x2, x3, [x0, #176]
    ldp x19, x20, [x1, #0]
    ldp x21, x22, [x1, #16]
    ldp x23, x24, [x1, #32]
    ldp x25, x26, [x1, #48]
    ldp x27, x28, [x1, #64]
    ldp x29, x30, [x1, #80]
    ldr x2, [x1, #96]
    mov sp, x2
    ldp d8, d9, [x1, #112]
    ldp d10, d11, [x1, #128]
    ldp d12, d13, [x1, #144]
    ldp d14, d15, [x1, #160]
    ldp x2, x3, [x1, #176]
    msr fpcr, x2
    msr fpsr, x3
    ret
"#);

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(r#"
.section .text.infinity_context_swap,"ax"
.global infinity_context_swap
infinity_context_swap:
    mov [rdi], rsp
    mov [rdi + 8], rbx
    mov [rdi + 16], rbp
    mov [rdi + 24], r12
    mov [rdi + 32], r13
    mov [rdi + 40], r14
    mov [rdi + 48], r15
    stmxcsr [rdi + 56]
    fnstcw [rdi + 60]
    mov rsp, [rsi]
    mov rbx, [rsi + 8]
    mov rbp, [rsi + 16]
    mov r12, [rsi + 24]
    mov r13, [rsi + 32]
    mov r14, [rsi + 40]
    mov r15, [rsi + 48]
    ldmxcsr [rsi + 56]
    fldcw [rsi + 60]
    ret
"#);
