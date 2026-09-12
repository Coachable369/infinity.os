#include <infinity/compiler_start.h>
extern int main(int, char **);
extern InfinityInitializer __init_array_start[], __init_array_end[];
extern InfinityInitializer __fini_array_start[], __fini_array_end[];

// ------------------------=
// FUNC: infinity_compiler_entry
// DESC: Enters a fresh compiler image using native launch arguments, not a Unix process stack.
// ------------------=
int infinity_compiler_entry(const InfinityCompilerLaunch *launch) {
    return infinity_compiler_run(launch, main, __init_array_start,
        __init_array_end - __init_array_start, __fini_array_start,
        __fini_array_end - __fini_array_start);
}
