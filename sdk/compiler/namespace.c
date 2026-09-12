#include <infinity/compiler_host.h>
#include <unistd.h>
#include <sys/stat.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>

// ------------------------=
// FUNC: namespace_error
// DESC: Returns a native provider error without changing namespace state.
// ------------------=
static int namespace_error(int error) { errno = error; return -1; }

// ------------------------=
// FUNC: namespace_path
// DESC: Publishes a bounded canonical reference only after successful resolution.
// ------------------=
static char *namespace_path(const char *path, char *out, size_t size) {
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->namespaces) { namespace_error(ENOSYS); return 0; }
    char buffer[96] = {0}; size_t length = 0;
    int error;
    if (path) {
        if (!*path) { namespace_error(EINVAL); return 0; }
        if (!host->namespaces->resolve) { namespace_error(ENOSYS); return 0; }
        error = host->namespaces->resolve(host->context, path, buffer, 95, &length);
    } else {
        if (!host->namespaces->current) { namespace_error(ENOSYS); return 0; }
        error = host->namespaces->current(host->context, buffer, 95, &length);
    }
    if (error) { namespace_error(error); return 0; }
    if (!length || length > 95 || buffer[0] != '/' || memchr(buffer, 0, length)) {
        namespace_error(EIO); return 0;
    }
    if (out && size <= length) { namespace_error(ERANGE); return 0; }
    if (!out) {
        if (size && size <= length) { namespace_error(ERANGE); return 0; }
        out = malloc(size ? size : length + 1);
        if (!out) { namespace_error(ENOMEM); return 0; }
    }
    memcpy(out, buffer, length); out[length] = 0;
    return out;
}
// ------------------------=
// FUNC: getcwd
// DESC: Reads the current native working namespace with failure-atomic output.
// ------------------=
char *getcwd(char *out, size_t size) { return namespace_path(0, out, size); }
// ------------------------=
// FUNC: realpath
// DESC: Resolves an existing native namespace reference, not a host path.
// ------------------=
char *realpath(const char *path, char *out) {
    if (!path) { namespace_error(EINVAL); return 0; }
    return namespace_path(path, out, out ? 96 : 0);
}
// ------------------------=
// FUNC: chdir
// DESC: Commits a provider-validated working namespace.
// ------------------=
int chdir(const char *path) {
    if (!path || !*path) return namespace_error(EINVAL);
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->namespaces || !host->namespaces->change) return namespace_error(ENOSYS);
    int error = host->namespaces->change(host->context, path);
    return error ? namespace_error(error) : 0;
}
// ------------------------=
// FUNC: mkdir
// DESC: Creates a capability-governed namespace, rejecting unsupported Unix permission requests.
// ------------------=
int mkdir(const char *path, mode_t mode) {
    if (!path || !*path) return namespace_error(EINVAL);
    if (mode != 0700 && mode != 0755 && mode != 0777) return namespace_error(ENOTSUP);
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->namespaces || !host->namespaces->create) return namespace_error(ENOSYS);
    int error = host->namespaces->create(host->context, path);
    return error ? namespace_error(error) : 0;
}
// ------------------------=
// FUNC: unlink
// DESC: Removes an authorized object reference with native reclamation semantics.
// ------------------=
int unlink(const char *path) {
    if (!path || !*path) return namespace_error(EINVAL);
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->namespaces || !host->namespaces->remove) return namespace_error(ENOSYS);
    int error = host->namespaces->remove(host->context, path);
    return error ? namespace_error(error) : 0;
}
