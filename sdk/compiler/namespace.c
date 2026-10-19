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
    char buffer[256] = {0}; size_t length = 0;
    int error;
    if (path) {
        if (!*path) { namespace_error(EINVAL); return 0; }
        if (!host->namespaces->resolve) { namespace_error(ENOSYS); return 0; }
        error = host->namespaces->resolve(host->context, path, buffer, 255, &length);
    } else {
        if (!host->namespaces->current) { namespace_error(ENOSYS); return 0; }
        error = host->namespaces->current(host->context, buffer, 255, &length);
    }
    if (error) { namespace_error(error); return 0; }
    if (!length || length > 255 || buffer[0] != '/' || memchr(buffer, 0, length)) {
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
    return namespace_path(path, out, out ? 256 : 0);
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

// ------------------------=
// FUNC: create_link
// DESC: Creates a capability-checked native ObjectStore reference without copying content.
// ------------------=
static int create_link(const char *existing, const char *created, uint32_t symbolic) {
    if (!existing || !*existing || !created || !*created) return namespace_error(EINVAL);
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->namespaces || !host->namespaces->link) return namespace_error(ENOSYS);
    int error = host->namespaces->link(host->context, existing, created, symbolic);
    return error ? namespace_error(error) : 0;
}

// ------------------------=
// FUNC: link
// DESC: Creates an additional authorized name for an existing ObjectStore identity.
// ------------------=
int link(const char *existing, const char *created) { return create_link(existing, created, 0); }

// ------------------------=
// FUNC: symlink
// DESC: Requests an explicitly supported symbolic native reference from the namespace provider.
// ------------------=
int symlink(const char *existing, const char *created) { return create_link(existing, created, 1); }

// ------------------------=
// FUNC: readlink
// DESC: Reads a bounded native symbolic reference without adding a terminating byte.
// ------------------=
ssize_t readlink(const char *path, char *out, size_t size) {
    if (!path || !*path || (!out && size)) return namespace_error(EINVAL);
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->namespaces || !host->namespaces->read_link) return namespace_error(ENOSYS);
    char temporary[256] = {0};
    size_t capacity = size < sizeof temporary ? size : sizeof temporary;
    size_t length = 0;
    int error = host->namespaces->read_link(host->context, path, temporary, capacity, &length);
    if (error) return namespace_error(error);
    if (length > capacity || (ssize_t)length < 0) return namespace_error(EIO);
    if (length) memcpy(out, temporary, length);
    return (ssize_t)length;
}

// ------------------------=
// FUNC: rename
// DESC: Atomically moves an authorized reference through its owning namespace provider.
// ------------------=
int rename(const char *source, const char *target) {
    if (!source || !*source || !target || !*target) return namespace_error(EINVAL);
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->namespaces || !host->namespaces->rename) return namespace_error(ENOSYS);
    int error = host->namespaces->rename(host->context, source, target);
    return error ? namespace_error(error) : 0;
}
// ------------------------=
// FUNC: rmdir
// DESC: Removes an empty authorized directory without conflating it with file removal.
// ------------------=
int rmdir(const char *path) {
    if (!path || !*path) return namespace_error(EINVAL);
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->namespaces || !host->namespaces->remove_directory) return namespace_error(ENOSYS);
    int error = host->namespaces->remove_directory(host->context, path);
    return error ? namespace_error(error) : 0;
}
// ------------------------=
// FUNC: chmod
// DESC: Requests scoped permissions without creating ambient authority or accepting special mode bits.
// ------------------=
int chmod(const char *path, mode_t mode) {
    if (!path || !*path) return namespace_error(EINVAL);
    if (mode & ~0777) return namespace_error(ENOTSUP);
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->namespaces || !host->namespaces->permissions) return namespace_error(ENOSYS);
    int error = host->namespaces->permissions(host->context, path, mode);
    return error ? namespace_error(error) : 0;
}

static mode_t creation_mask = 0022;
// ------------------------=
// FUNC: umask
// DESC: Maintains the process-local creation preference; native capability grants remain authoritative.
// ------------------=
mode_t umask(mode_t mask) {
    mode_t previous = creation_mask;
    creation_mask = mask & 0777;
    return previous;
}
