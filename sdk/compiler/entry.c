#include <infinity/compiler_start.h>
#include <infinity/compiler_syscall.h>
#include <infinity/syscall.h>
extern int main(int, char **);
extern InfinityInitializer __init_array_start[], __init_array_end[];
extern InfinityInitializer __fini_array_start[], __fini_array_end[];

// ------------------------=
// FUNC: infinity_compiler_entry
// DESC: Enters isolated user mode with local syscall adapters and exits through the native dispatcher.
// ------------------=
__attribute__((noreturn)) void infinity_compiler_entry(const InfinityIsolatedCompilerLaunch *launch) {
    InfinityCompilerHost host;
    int result = 126;
    if (infinity_compiler_isolated_host(launch, &host) == 0) {
        InfinityCompilerLaunch invocation = { INFINITY_COMPILER_LAUNCH_ABI,
            sizeof(invocation), &host, launch->argc, launch->argv };
        result = infinity_compiler_run(&invocation, main, __init_array_start,
        __init_array_end - __init_array_start, __fini_array_start,
        __fini_array_end - __fini_array_start);
    }
    infinity_system_call(INFINITY_SYS_EXIT, (uint64_t)(int64_t)result, 0, 0, 0, 0, 0);
    for (;;) infinity_system_call(INFINITY_SYS_YIELD, 0, 0, 0, 0, 0, 0);
}
