#include <infinity/compiler_syscall.h>
#include <infinity/syscall.h>
#include <errno.h>
#include <limits.h>

// ------------------------=
// FUNC: service_result
// DESC: Converts native negative errno results to the existing compiler adapter contract.
// ------------------=
static int service_result(int64_t result) {
    return result == 0 ? 0 : (result < 0 && result >= -4095 ? (int)-result : EIO);
}

// ------------------------=
// FUNC: isolated_clock_ns
// DESC: Marshals clock ns across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_clock_ns(void *context, int clock, uint64_t *out) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_CLOCK_NS, clock, (uintptr_t)out, 0, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_map
// DESC: Marshals map across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_map(void *context, void **address, size_t bytes, int protection, int flags, int descriptor, int64_t offset) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_MAP, (uintptr_t)address, bytes, protection, flags, descriptor, offset));
}

// ------------------------=
// FUNC: isolated_unmap
// DESC: Marshals unmap across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_unmap(void *context, void *address, size_t bytes) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_UNMAP, (uintptr_t)address, bytes, 0, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_protect
// DESC: Marshals protect across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_protect(void *context, void *address, size_t bytes, int protection) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_PROTECT, (uintptr_t)address, bytes, protection, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_sync
// DESC: Marshals sync across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_sync(void *context, void *address, size_t bytes, int flags) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_SYNC, (uintptr_t)address, bytes, flags, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_entropy
// DESC: Marshals entropy across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_entropy(void *context, void *out, size_t bytes) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_ENTROPY, (uintptr_t)out, bytes, 0, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_directory_open
// DESC: Marshals directory open across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_directory_open(void *context, const char *path, uint64_t *out) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_DIRECTORY_OPEN, (uintptr_t)path, (uintptr_t)out, 0, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_directory_next
// DESC: Marshals directory next across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_directory_next(void *context, uint64_t handle, char *out, size_t bytes) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_DIRECTORY_NEXT, handle, (uintptr_t)out, bytes, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_directory_close
// DESC: Marshals directory close across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_directory_close(void *context, uint64_t handle) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_DIRECTORY_CLOSE, handle, 0, 0, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_sleep_ns
// DESC: Marshals sleep ns across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_sleep_ns(void *context, uint64_t duration, uint64_t *remaining) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_SLEEP_NS, duration, (uintptr_t)remaining, 0, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_process_alive
// DESC: Marshals process alive across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_process_alive(void *context, uint32_t identity, int *out) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_PROCESS_ALIVE, identity, (uintptr_t)out, 0, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_file_open
// DESC: Marshals file open across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_file_open(void *context, const char *path, uint32_t mode, uint64_t *out) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_FILE_OPEN, (uintptr_t)path, mode, (uintptr_t)out, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_file_read
// DESC: Marshals file read across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_file_read(void *context, uint64_t handle, void *out, size_t bytes, size_t *done) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_FILE_READ, handle, (uintptr_t)out, bytes, (uintptr_t)done, 0, 0));
}

// ------------------------=
// FUNC: isolated_file_write
// DESC: Marshals file write across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_file_write(void *context, uint64_t handle, const void *data, size_t bytes, size_t *done) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_FILE_WRITE, handle, (uintptr_t)data, bytes, (uintptr_t)done, 0, 0));
}

// ------------------------=
// FUNC: isolated_file_seek
// DESC: Marshals file seek across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_file_seek(void *context, uint64_t handle, int64_t offset, int origin, int64_t *out) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_FILE_SEEK, handle, offset, origin, (uintptr_t)out, 0, 0));
}

// ------------------------=
// FUNC: isolated_file_close
// DESC: Marshals file close across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_file_close(void *context, uint64_t handle) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_FILE_CLOSE, handle, 0, 0, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_file_read_at
// DESC: Marshals file read at across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_file_read_at(void *context, uint64_t handle, uint64_t offset, void *out, size_t bytes, size_t *done) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_FILE_READ_AT, handle, offset, (uintptr_t)out, bytes, (uintptr_t)done, 0));
}

// ------------------------=
// FUNC: isolated_file_inspect
// DESC: Marshals file inspect across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_file_inspect(void *context, uint64_t handle, InfinityCompilerMetadata *out) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_FILE_INSPECT, handle, (uintptr_t)out, 0, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_file_inspect_path
// DESC: Marshals file inspect path across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_file_inspect_path(void *context, const char *path, InfinityCompilerMetadata *out) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_FILE_INSPECT_PATH, (uintptr_t)path, (uintptr_t)out, 0, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_file_truncate
// DESC: Marshals file truncate across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_file_truncate(void *context, uint64_t handle, uint64_t bytes) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_FILE_TRUNCATE, handle, bytes, 0, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_namespace_current
// DESC: Marshals namespace current across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_namespace_current(void *context, char *out, size_t bytes, size_t *done) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_NAMESPACE_CURRENT, (uintptr_t)out, bytes, (uintptr_t)done, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_namespace_resolve
// DESC: Marshals namespace resolve across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_namespace_resolve(void *context, const char *path, char *out, size_t bytes, size_t *done) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_NAMESPACE_RESOLVE, (uintptr_t)path, (uintptr_t)out, bytes, (uintptr_t)done, 0, 0));
}

// ------------------------=
// FUNC: isolated_namespace_change
// DESC: Marshals namespace change across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_namespace_change(void *context, const char *path) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_NAMESPACE_CHANGE, (uintptr_t)path, 0, 0, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_namespace_create
// DESC: Marshals namespace create across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_namespace_create(void *context, const char *path) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_NAMESPACE_CREATE, (uintptr_t)path, 0, 0, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_namespace_remove
// DESC: Marshals namespace remove across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_namespace_remove(void *context, const char *path) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_NAMESPACE_REMOVE, (uintptr_t)path, 0, 0, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_namespace_link
// DESC: Marshals namespace link across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_namespace_link(void *context, const char *source, const char *target, uint32_t flags) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_NAMESPACE_LINK, (uintptr_t)source, (uintptr_t)target, flags, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_namespace_read_link
// DESC: Marshals namespace read link across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_namespace_read_link(void *context, const char *path, char *out, size_t bytes, size_t *done) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_NAMESPACE_READ_LINK, (uintptr_t)path, (uintptr_t)out, bytes, (uintptr_t)done, 0, 0));
}

// ------------------------=
// FUNC: isolated_console_read
// DESC: Marshals console read across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_console_read(void *context, uint32_t stream, void *out, size_t bytes, size_t *done) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_CONSOLE_READ, stream, (uintptr_t)out, bytes, (uintptr_t)done, 0, 0));
}

// ------------------------=
// FUNC: isolated_console_write
// DESC: Marshals console write across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_console_write(void *context, uint32_t stream, const void *data, size_t bytes, size_t *done) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_CONSOLE_WRITE, stream, (uintptr_t)data, bytes, (uintptr_t)done, 0, 0));
}

// ------------------------=
// FUNC: isolated_console_is_terminal
// DESC: Marshals console is terminal across the user privilege boundary; never invokes a kernel callback.
// ------------------=
static int isolated_console_is_terminal(void *context, uint32_t stream, int *out) {
    (void)context;
    return service_result(infinity_system_call(INFINITY_COMPILER_CONSOLE_IS_TERMINAL, stream, (uintptr_t)out, 0, 0, 0, 0));
}

// ------------------------=
// FUNC: isolated_terminate
// DESC: Exits the owning execution context without returning to a privileged entrypoint.
// ------------------=
static int isolated_terminate(void *context, int status) {
    (void)context;
    infinity_system_call(INFINITY_SYS_EXIT, (uint64_t)(int64_t)status, 0, 0, 0, 0, 0);
    for (;;) infinity_system_call(INFINITY_SYS_YIELD, 0, 0, 0, 0, 0, 0);
}

static const InfinityCompilerFiles files = {
    isolated_file_open, isolated_file_read, isolated_file_write, isolated_file_seek,
    isolated_file_close, isolated_file_read_at, isolated_file_inspect,
    isolated_file_inspect_path, isolated_file_truncate
};
static const InfinityCompilerNamespace namespaces = {
    isolated_namespace_current, isolated_namespace_resolve, isolated_namespace_change,
    isolated_namespace_create, isolated_namespace_remove, isolated_namespace_link,
    isolated_namespace_read_link
};
static const InfinityCompilerConsole console = {
    isolated_console_read, isolated_console_write, isolated_console_is_terminal
};

// ------------------------=
// FUNC: infinity_compiler_isolated_host
// DESC: Builds a wholly user-owned service table from a bounded launch record without trusted code pointers.
// ------------------=
int infinity_compiler_isolated_host(const InfinityIsolatedCompilerLaunch *launch,
                                   InfinityCompilerHost *host) {
    if (!launch || !host || launch->version != INFINITY_ISOLATED_COMPILER_ABI ||
        launch->size != sizeof(*launch) || launch->reserved || launch->argc < 1 ||
        launch->argc > 4096 || !launch->argv || launch->argv[launch->argc] ||
        !launch->heap_base || (uintptr_t)launch->heap_base % 16 ||
        !launch->heap_size || launch->heap_size > UINTPTR_MAX - (uintptr_t)launch->heap_base ||
        !launch->process_identity || !launch->session_identity ||
        !launch->executable_path || !launch->home_path || !launch->node_name) return EINVAL;
    for (int i = 0; i < launch->argc; ++i) if (!launch->argv[i]) return EINVAL;
    *host = (InfinityCompilerHost){
        .executable_path = launch->executable_path, .clock_ns = isolated_clock_ns,
        .map = isolated_map, .unmap = isolated_unmap, .protect = isolated_protect,
        .sync = isolated_sync, .entropy = isolated_entropy,
        .directory_open = isolated_directory_open, .directory_next = isolated_directory_next,
        .directory_close = isolated_directory_close, .files = &files,
        .heap_base = launch->heap_base, .heap_size = launch->heap_size, .page_size = 4096,
        .sleep_ns = isolated_sleep_ns, .serial_execution = 1, .home_path = launch->home_path,
        .namespaces = &namespaces, .console = &console, .process_identity = launch->process_identity,
        .session_identity = launch->session_identity, .node_name = launch->node_name,
        .process_alive = isolated_process_alive, .terminate = isolated_terminate
    };
    return 0;
}

