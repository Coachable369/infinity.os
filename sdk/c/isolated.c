#include <infinity/app.h>
#include <infinity/syscall.h>

extern int infinity_app_entry(const InfinityAppApi *);

// ------------------------=
// FUNC: console_write
// DESC: Sends bounded output through a native syscall; this callback and its table live entirely in user memory.
// ------------------=
static int console_write(void *context, const unsigned char *bytes, size_t length) {
    (void)context;
    size_t written = 0;
    while (written < length) {
        size_t count = length - written;
        if (count > 4096) count = 4096;
        int64_t result = infinity_system_call(INFINITY_SYS_CONSOLE_WRITE,
            (uintptr_t)(bytes + written), count, 0, 0, 0, 0);
        if (result <= 0 || (uint64_t)result > count) return -1;
        written += (size_t)result;
    }
    return (int)written;
}

// ------------------------=
// FUNC: infinity_app_start
// DESC: Starts ordinary main inside hardware-isolated user mode and exits via the supervisor, never through a kernel callback.
// ------------------=
__attribute__((noreturn)) void infinity_app_start(void) {
    const InfinityAppApi api = {INFINITY_APP_ABI, sizeof(InfinityAppApi), 0, console_write, 0};
    int status = infinity_app_entry(&api);
    infinity_system_call(INFINITY_SYS_EXIT, (uint64_t)(int64_t)status, 0, 0, 0, 0, 0);
    for (;;) infinity_system_call(INFINITY_SYS_YIELD, 0, 0, 0, 0, 0, 0);
}
