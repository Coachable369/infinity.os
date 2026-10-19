#include <dlfcn.h>
#include <errno.h>
#include <stdint.h>
#include <stddef.h>
#include <infinity/syscall.h>

/* Native module operations carry only private user pointers and scalar values. */
enum { MODULE_OPEN = 0x1230, MODULE_SYMBOL = 0x1231 };
static _Thread_local const char *pending_error;

// ------------------------=
// FUNC: failure
// DESC: Preserves native loader errors as a thread-local dlerror diagnostic and errno.
// ------------------=
static void *failure(int error) {
    errno = error;
    pending_error = error == ENOENT ? "Native extension or symbol not found" :
        error == ENOEXEC ? "Invalid or unsupported InfinityOS native extension" :
        error == ENOMEM ? "Native extension memory budget exhausted" :
        error == EACCES ? "Native extension access denied" : "Native extension loader failure";
    return NULL;
}

// ------------------------=
// FUNC: dlopen
// DESC: Requests a checked native extension load from the process's explicit Object Store grants.
// ------------------=
void *dlopen(const char *path, int flags) {
    if (!path) return failure(ENOTSUP);
    uint64_t handle = 0;
    int64_t result = infinity_system_call(MODULE_OPEN, (uintptr_t)path, flags, (uintptr_t)&handle, 0, 0, 0);
    return result < 0 ? failure((int)-result) : (void *)(uintptr_t)handle;
}

// ------------------------=
// FUNC: dlsym
// DESC: Resolves an export only through a live library handle owned by this isolated process.
// ------------------=
void *dlsym(void *handle, const char *name) {
    if (!name) return failure(EINVAL);
    uint64_t address = 0;
    int64_t result = infinity_system_call(MODULE_SYMBOL, (uintptr_t)handle, (uintptr_t)name, (uintptr_t)&address, 0, 0, 0);
    return result < 0 ? failure((int)-result) : (void *)(uintptr_t)address;
}

// ------------------------=
// FUNC: dlclose
// DESC: Rejects early unloading because CPython may retain pointers; process teardown reclaims every module.
// ------------------=
int dlclose(void *handle) { (void)handle; failure(ENOTSUP); return -1; }

// ------------------------=
// FUNC: dlerror
// DESC: Consumes only the current thread's pending dynamic-loader diagnostic.
// ------------------=
char *dlerror(void) { const char *error = pending_error; pending_error = NULL; return (char *)error; }

// ------------------------=
// FUNC: dladdr
// DESC: Reports unsupported address symbolization without fabricating a module identity.
// ------------------=
int dladdr(const void *address, Dl_info *info) { (void)address; (void)info; failure(ENOTSUP); return 0; }
