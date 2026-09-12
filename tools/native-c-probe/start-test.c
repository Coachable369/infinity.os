#include <infinity/compiler_start.h>
#include <assert.h>
static int order;
static const InfinityCompilerHost *active;
// ------------------------=
// FUNC: infinity_compiler_set_host
// DESC: Records service binding in the startup-only fixture.
// ------------------=
void infinity_compiler_set_host(const InfinityCompilerHost *host) { active = host; }
// ------------------------=
// FUNC: initialize
// DESC: Verifies constructors run after binding and before main.
// ------------------=
static void initialize(void) { assert(active && order++ == 0); }
// ------------------------=
// FUNC: first_finalizer
// DESC: Verifies the first finalizer runs last.
// ------------------=
static void first_finalizer(void) { assert(active && order++ == 3); }
// ------------------------=
// FUNC: last_finalizer
// DESC: Verifies reverse finalizer ordering.
// ------------------=
static void last_finalizer(void) { assert(active && order++ == 2); }
// ------------------------=
// FUNC: program
// DESC: Checks exact argument identity and propagates an arbitrary exit status.
// ------------------=
static int program(int argc, char **argv) {
    assert(active && order++ == 1 && argc == 1 && argv[0][0] == 'c');
    return 37;
}
// ------------------------=
// FUNC: main
// DESC: Exercises rejected launches, constructor ordering, result propagation and one-shot lifecycle.
// ------------------=
int main(void) {
    InfinityCompilerHost host = {0};
    char name[] = "cc";
    char *argv[] = {name, 0};
    InfinityCompilerLaunch launch = {INFINITY_COMPILER_LAUNCH_ABI, sizeof launch, &host, 1, argv};
    InfinityInitializer init[] = {initialize};
    InfinityInitializer fini[] = {first_finalizer, last_finalizer};
    assert(infinity_compiler_run(0, program, init, 1, fini, 2) == 126);
    launch.version++;
    assert(infinity_compiler_run(&launch, program, init, 1, fini, 2) == 126);
    launch.version--;
    argv[1] = name;
    assert(infinity_compiler_run(&launch, program, init, 1, fini, 2) == 126);
    argv[1] = 0;
    assert(!active && order == 0);
    assert(infinity_compiler_run(&launch, program, init, 1, fini, 2) == 37);
    assert(!active && order == 4);
    assert(infinity_compiler_run(&launch, program, init, 1, fini, 2) == 126);
    return 0;
}
