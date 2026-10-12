#include <stdint.h>
#include <infinity/syscall.h>

// ------------------------=
// FUNC: infinity_app_entry
// DESC: Exercises real user privileges, supervisor protection, NX, illegal I/O, and timer preemption.
// ------------------=
__attribute__((noreturn)) void infinity_app_entry(uint64_t kind, volatile uint64_t *kernel) {
    if (kind == 1) *kernel = 0;
    if (kind == 2) ((void (*)(void))(uintptr_t)0x200000000ull)();
    if (kind == 3) for (;;) __asm__ volatile("nop");
    if (kind == 5) {
        for (uint64_t index = 0; index < 16; ++index) {
            uint64_t vector, tls;
#if defined(__x86_64__)
            uint64_t call = 5;
            __asm__ volatile("movq %[index], %%xmm15; movq %[index], %%fs:0; int $0x80;"
                "movq %%xmm15, %[vector]; movq %%fs:0, %[tls]"
                : "+a"(call), [vector] "=r"(vector), [tls] "=r"(tls)
                : [index] "b"(index) : "xmm15", "memory");
#else
            register uint64_t value __asm__("x0") = index;
            register uint64_t call __asm__("x8") = 5;
            __asm__ volatile("fmov d15, %[index]; mrs x9, tpidr_el0; str %[index], [x9]; svc #0;"
                "fmov %[vector], d15; mrs x9, tpidr_el0; ldr %[tls], [x9]"
                : "+r"(value), [vector] "=r"(vector), [tls] "=r"(tls)
                : [index] "r"(index), "r"(call) : "x9", "v15", "memory");
#endif
            if (vector != index || tls != index) __builtin_trap();
        }
    }
    if (kind == 6) {
        for (uint64_t index = 0; index < 16; ++index) {
            int64_t result = infinity_system_call(INFINITY_SYS_YIELD,
                index, 11, 22, 33, 44, 55);
            if (result != (int64_t)(index + 100)) __builtin_trap();
        }
    }
#if defined(__x86_64__)
    if (kind == 4) __asm__ volatile("outb %%al, %%dx" :: "a"(0), "d"(0x80));
    uint16_t cs;
    __asm__ volatile("mov %%cs, %0" : "=r"(cs));
    __asm__ volatile("int $0x80" :: "a"((uint64_t)(cs & 3)), "b"(42ull) : "memory");
#else
    if (kind == 4) __asm__ volatile("msr sctlr_el1, xzr");
    register uint64_t result __asm__("x0") = 42;
    register uint64_t syscall __asm__("x8") = 1;
    __asm__ volatile("svc #0" : "+r"(result) : "r"(syscall) : "memory");
#endif
    for (;;) __asm__ volatile("nop");
}
