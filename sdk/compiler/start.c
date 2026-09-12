#include <infinity/compiler_start.h>
#include <infinity/serial_tls.h>

static int launched;

// ------------------------=
// FUNC: infinity_compiler_run
// DESC: Runs one validated native invocation with ordered constructors and reverse-order destructors.
// ------------------=
int infinity_compiler_run(const InfinityCompilerLaunch *launch,
                         int (*program)(int, char **),
                         const InfinityInitializer *init, size_t init_count,
                         const InfinityInitializer *fini, size_t fini_count) {
    if (launched || !launch || launch->version != INFINITY_COMPILER_LAUNCH_ABI ||
        launch->size < sizeof(*launch) || !launch->host || launch->host->serial_execution != 1 || !program ||
        launch->argc < 1 || launch->argc > 4096 || !launch->argv ||
        launch->argv[launch->argc] || (init_count && !init) || (fini_count && !fini))
        return 126;
    for (int i = 0; i < launch->argc; ++i)
        if (!launch->argv[i]) return 126;
    launched = 1;
    infinity_compiler_set_host(launch->host);
    for (size_t i = 0; i < init_count; ++i) if (init[i]) init[i]();
    int result = program(launch->argc, launch->argv);
    infinity_thread_cleanup();
    while (fini_count) { --fini_count; if (fini[fini_count]) fini[fini_count](); }
    infinity_compiler_set_host(0);
    return result;
}
