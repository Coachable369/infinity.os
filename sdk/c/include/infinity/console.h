#ifndef INFINITY_CONSOLE_H
#define INFINITY_CONSOLE_H
#include <stddef.h>
#include <infinity/syscall.h>

// ------------------------=
// FUNC: infinity_read_line
// DESC: Reads an interactive terminal line using checked native input; EOF and errors return negative results.
// ------------------=
static inline int infinity_read_line(char *buffer, size_t capacity) {
    if (!buffer || capacity < 2) return -22;
    size_t length = 0;
    int overflow = 0;
    for (;;) {
        unsigned char byte;
        int64_t result = infinity_system_call(INFINITY_SYS_CONSOLE_READ,
            (uintptr_t)&byte, 1, 0, 0, 0, 0);
        if (result == -11) continue; /* The supervisor parks this task until input arrives. */
        if (result <= 0) { buffer[length] = 0; return overflow ? -75 : result < 0 ? (int)result : length ? (int)length : -1; }
        if (byte == '\n') { buffer[length] = 0; return overflow ? -75 : (int)length; }
        if (length + 1 < capacity) buffer[length++] = (char)byte;
        else overflow = 1; /* Drain the rest of an oversized line instead of splitting commands. */
    }
}
#endif
