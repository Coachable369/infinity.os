#include <infinity/compiler_host.h>
#include <errno.h>
#include <signal.h>
#include <stdint.h>
#include <string.h>
#include <unistd.h>

// ------------------------=
// FUNC: process_error
// DESC: Publishes a native process-boundary error without fabricating Unix process state.
// ------------------=
static int process_error(int error) { errno = error; return -1; }

// ------------------------=
// FUNC: getpid
// DESC: Returns the stable identity assigned to this compiler Execution Context.
// ------------------=
pid_t getpid(void) {
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->process_identity || (pid_t)host->process_identity <= 0 ||
        (uint32_t)(pid_t)host->process_identity != host->process_identity)
        return (pid_t)process_error(ENOSYS);
    return (pid_t)host->process_identity;
}

// ------------------------=
// FUNC: getsid
// DESC: Returns this compiler context's native session identity and rejects unrelated contexts.
// ------------------=
pid_t getsid(pid_t process) {
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->process_identity) return (pid_t)process_error(ENOSYS);
    if (process != 0 && (uint32_t)process != host->process_identity) return (pid_t)process_error(ESRCH);
    uint32_t session = host->session_identity ? host->session_identity : host->process_identity;
    if ((pid_t)session <= 0 || (uint32_t)(pid_t)session != session) return (pid_t)process_error(EOVERFLOW);
    return (pid_t)session;
}

// ------------------------=
// FUNC: gethostname
// DESC: Copies the launcher's authenticated node name without truncation or host fallback.
// ------------------=
int gethostname(char *out, size_t size) {
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!out || !size) return process_error(EFAULT);
    if (!host || !host->node_name || !*host->node_name) return process_error(ENOSYS);
    size_t length = 0;
    while (length < size && host->node_name[length]) ++length;
    if (length == size) return process_error(ENAMETOOLONG);
    memcpy(out, host->node_name, length + 1);
    return 0;
}

// ------------------------=
// FUNC: isatty
// DESC: Queries only the launch-bound standard console streams.
// ------------------=
int isatty(int descriptor) {
    if (descriptor < 0 || descriptor > 2) { errno = ENOTTY; return 0; }
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->console || !host->console->is_terminal) { errno = ENOTTY; return 0; }
    int terminal = 0;
    int error = host->console->is_terminal(host->context, (uint32_t)descriptor, &terminal);
    if (error) { errno = error; return 0; }
    if (!terminal) { errno = ENOTTY; return 0; }
    return 1;
}

// ------------------------=
// FUNC: dup2
// DESC: Accepts identity duplication of a bound standard stream and rejects unsupported descriptor aliasing.
// ------------------=
int dup2(int source, int destination) {
    if (source < 0 || destination < 0) return process_error(EBADF);
    if (source != destination) return process_error(ENOTSUP);
    if (source <= 2) {
        const InfinityCompilerHost *host = infinity_compiler_get_host();
        return host && host->console ? destination : process_error(EBADF);
    }
    return process_error(ENOTSUP);
}

// ------------------------=
// FUNC: kill
// DESC: Supports a non-mutating liveness query and rejects Unix signal delivery.
// ------------------=
int kill(pid_t process, int signal_number) {
    if (process <= 0) return process_error(EINVAL);
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->process_identity) return process_error(ENOSYS);
    if (signal_number != 0) return process_error(ENOTSUP);
    if ((uint32_t)process == host->process_identity) return 0;
    if (!host->process_alive) return process_error(ESRCH);
    int alive = 0;
    int error = host->process_alive(host->context, (uint32_t)process, &alive);
    if (error) return process_error(error);
    return alive ? 0 : process_error(ESRCH);
}

// ------------------------=
// FUNC: sigaction
// DESC: Rejects ambient POSIX signal handlers because compiler faults belong to the native crash service.
// ------------------=
int sigaction(int signal_number, const struct sigaction *action, struct sigaction *previous) {
    (void)signal_number; (void)action; (void)previous;
    return process_error(ENOTSUP);
}

// ------------------------=
// FUNC: sigprocmask
// DESC: Rejects POSIX signal masks rather than implying protection from native context faults.
// ------------------=
int sigprocmask(int operation, const sigset_t *set, sigset_t *previous) {
    (void)operation; (void)set; (void)previous;
    return process_error(ENOTSUP);
}

// ------------------------=
// FUNC: _exit
// DESC: Requests irreversible native context termination and traps if a provider incorrectly returns.
// ------------------=
_Noreturn void _exit(int status) {
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (host && host->terminate) (void)host->terminate(host->context, status);
    __builtin_trap();
    for (;;) { }
}
