#include <infinity/compiler_host.h>
#include <assert.h>
#include <errno.h>
#include <signal.h>
#include <string.h>
#include <unistd.h>

static int terminal_value;
static int alive_value;

// ------------------------=
// FUNC: terminal
// DESC: Returns a controlled terminal state for a launch-bound standard descriptor.
// ------------------=
static int terminal(void *context, uint32_t stream, int *out) {
    assert(context == &terminal_value && stream <= 2); *out = terminal_value; return 0;
}

// ------------------------=
// FUNC: alive
// DESC: Returns controlled native Execution Context liveness without granting signal authority.
// ------------------=
static int alive(void *context, uint32_t process, int *out) {
    assert(context == &terminal_value && process == 19); *out = alive_value; return 0;
}

// ------------------------=
// FUNC: main
// DESC: Exercises native identity, node, console and unsupported signal behavior.
// ------------------=
int main(void) {
    infinity_compiler_set_host(0);
    char node[16] = {42};
    assert(getpid() == -1 && errno == ENOSYS);
    assert(gethostname(node, sizeof node) == -1 && errno == ENOSYS && node[0] == 42);
    assert(isatty(1) == 0 && errno == ENOTTY);
    InfinityCompilerConsole console = {.is_terminal = terminal};
    InfinityCompilerHost host = {.context = &terminal_value, .console = &console,
        .process_identity = 17, .session_identity = 9, .node_name = "node1", .process_alive = alive};
    infinity_compiler_set_host(&host);
    assert(getpid() == 17 && getsid(0) == 9 && getsid(17) == 9);
    assert(getsid(19) == -1 && errno == ESRCH);
    assert(gethostname(node, sizeof node) == 0 && !strcmp(node, "node1"));
    node[0] = 42;
    assert(gethostname(node, 4) == -1 && errno == ENAMETOOLONG && node[0] == 42);
    assert(isatty(1) == 0 && errno == ENOTTY);
    terminal_value = 1;
    assert(isatty(1) == 1);
    assert(dup2(1, 1) == 1 && dup2(1, 2) == -1 && errno == ENOTSUP);
    assert(kill(17, 0) == 0);
    alive_value = 1;
    assert(kill(19, 0) == 0);
    alive_value = 0;
    assert(kill(19, 0) == -1 && errno == ESRCH);
    assert(kill(17, SIGTERM) == -1 && errno == ENOTSUP);
    struct sigaction action = {0};
    assert(sigaction(SIGTERM, &action, 0) == -1 && errno == ENOTSUP);
    assert(sigprocmask(SIG_BLOCK, 0, 0) == -1 && errno == ENOTSUP);
    return 0;
}
