#ifndef INFINITY_SYSCALL_H
#define INFINITY_SYSCALL_H
#include <stdint.h>

enum InfinitySystemCall {
    INFINITY_SYS_EXIT = 0x1000,
    INFINITY_SYS_YIELD = 0x1001,
    INFINITY_SYS_CONSOLE_WRITE = 0x1002,
    INFINITY_SYS_CONSOLE_READ = 0x1003,
    INFINITY_SYS_WINDOW_CREATE = 0x1100,
    INFINITY_SYS_WINDOW_CLOSE = 0x1101,
    INFINITY_SYS_WINDOW_TITLE = 0x1102,
    INFINITY_SYS_WINDOW_MOVE = 0x1103,
    INFINITY_SYS_WINDOW_RESIZE = 0x1104,
    INFINITY_SYS_WINDOW_STATE = 0x1105,
    INFINITY_SYS_WINDOW_FOCUS = 0x1106,
    INFINITY_SYS_WINDOW_CAPTURE = 0x1107,
    INFINITY_SYS_WINDOW_RELEASE = 0x1108,
    INFINITY_SYS_EVENT_SUBSCRIBE = 0x1110,
    INFINITY_SYS_EVENT_POLL = 0x1111,
    INFINITY_SYS_EVENT_UNSUBSCRIBE = 0x1112,
    INFINITY_SYS_SURFACE_CREATE = 0x1120,
    INFINITY_SYS_SURFACE_UPLOAD = 0x1121,
    INFINITY_SYS_SURFACE_COMMIT = 0x1122,
    INFINITY_SYS_SURFACE_RESIZE = 0x1123,
    INFINITY_SYS_SURFACE_DESTROY = 0x1124,
    INFINITY_SYS_UI_OPEN = 0x1140,
    INFINITY_SYS_UI_UPSERT = 0x1141,
    INFINITY_SYS_UI_POLL = 0x1142,
    INFINITY_SYS_FONT_GLYPH = 0x1250
};

/* CONSOLE_WRITE arguments are bytes, length (at most4096), channel (0/1
 * stdout,2 stderr). CONSOLE_READ arguments are buffer and capacity (at most1024).
 * Negative results are ABI errors. Authority is always supplied by the
 * current execution context; there is no caller identity or kernel callback. */
// ------------------------=
// FUNC: infinity_system_call
// DESC: Enters the native service dispatcher from hardware-isolated user mode on either supported CPU.
// ------------------=
static inline int64_t infinity_system_call(uint64_t operation, uint64_t a0,
    uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5) {
#if defined(__x86_64__)
    register uint64_t fifth __asm__("r8") = a4;
    register uint64_t sixth __asm__("r9") = a5;
    __asm__ volatile("int $0x80" : "+a"(operation)
        : "D"(a0), "S"(a1), "d"(a2), "c"(a3), "r"(fifth), "r"(sixth)
        : "memory", "cc");
    return (int64_t)operation;
#elif defined(__aarch64__)
    register uint64_t code __asm__("x8") = operation;
    register uint64_t first __asm__("x0") = a0;
    register uint64_t second __asm__("x1") = a1;
    register uint64_t third __asm__("x2") = a2;
    register uint64_t fourth __asm__("x3") = a3;
    register uint64_t fifth __asm__("x4") = a4;
    register uint64_t sixth __asm__("x5") = a5;
    __asm__ volatile("svc #0" : "+r"(first)
        : "r"(code), "r"(second), "r"(third), "r"(fourth), "r"(fifth), "r"(sixth)
        : "memory", "cc");
    return (int64_t)first;
#else
#error Native user execution requires x86_64 or AArch64
#endif
}
#endif
