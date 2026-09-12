#include <dlfcn.h>
#include <errno.h>

/* Requires the native application's TLS runtime before production execution.
   Error retrieval consumes only this thread's pending diagnostic. */
static _Thread_local const char *pending_error;

// ------------------------=
// FUNC: reject
// DESC: Records a compatibility error without accessing images, handles or host services.
// ------------------=
static void reject(int code, const char *message) {
    errno = code;
    pending_error = message;
}

// ------------------------=
// FUNC: dlopen
// DESC: Refuses all loads, including current-process lookup, until a native loader exists.
// ------------------=
void *dlopen(const char *path, int flags) {
    (void)path;
    int binding = flags & (RTLD_LAZY | RTLD_NOW);
    if ((flags & ~(RTLD_LAZY | RTLD_NOW | RTLD_GLOBAL)) ||
        (binding != RTLD_LAZY && binding != RTLD_NOW)) {
        reject(EINVAL, "Invalid dynamic loading flags");
    } else {
        reject(ENOSYS, "Native dynamic loading is not available");
    }
    return 0;
}

// ------------------------=
// FUNC: dlsym
// DESC: Rejects symbol lookup without dereferencing untrusted handles or consulting the host.
// ------------------=
void *dlsym(void *handle, const char *name) {
    (void)handle;
    if (!name) reject(EINVAL, "Null symbol name");
    else reject(ENOSYS, "Native dynamic symbol lookup is not available");
    return 0;
}

// ------------------------=
// FUNC: dlclose
// DESC: Fails explicitly because this implementation cannot have issued a valid image handle.
// ------------------=
int dlclose(void *handle) {
    (void)handle;
    reject(ENOSYS, "Native dynamic image handles are not available");
    return -1;
}

// ------------------------=
// FUNC: dlerror
// DESC: Returns and consumes this thread's last diagnostic without clearing errno.
// ------------------=
char *dlerror(void) {
    const char *result = pending_error;
    pending_error = 0;
    return (char *)result;
}

// ------------------------=
// FUNC: dladdr
// DESC: Reports unavailable address symbolization and leaves the caller's result untouched.
// ------------------=
int dladdr(const void *address, Dl_info *out) {
    (void)address;
    if (!out) reject(EINVAL, "Null symbolization output");
    else reject(ENOSYS, "Native address symbolization is not available");
    return 0;
}
