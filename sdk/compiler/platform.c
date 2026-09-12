#include <infinity/compiler_host.h>
#include <sys/mman.h>
#include <time.h>
#include <errno.h>

static const InfinityCompilerHost *host;

// ------------------------=
// FUNC: infinity_compiler_set_host
// DESC: Installs the serial compiler invocation's explicit OS service table.
// ------------------=
void infinity_compiler_set_host(const InfinityCompilerHost *services) { host = services; }

// ------------------------=
// FUNC: infinity_compiler_executable_path
// DESC: Returns the launcher's actual executable object reference without requiring procfs or a host path.
// ------------------=
const char *infinity_compiler_executable_path(void) {
    return host && host->executable_path ? host->executable_path : "";
}

// ------------------------=
// FUNC: fail
// DESC: Preserves an explicit service failure in errno without pretending the operation succeeded.
// ------------------=
static int fail(int error) { errno = error; return -1; }

// ------------------------=
// FUNC: clock_gettime
// DESC: Converts the OS clock callback's actual nanoseconds to a normalized timespec; unavailable clocks remain errors.
// ------------------=
int clock_gettime(clockid_t clock, struct timespec *out) {
    if (!out) return fail(EFAULT);
    if (clock != CLOCK_REALTIME && clock != CLOCK_MONOTONIC) return fail(EINVAL);
    if (!host || !host->clock_ns) return fail(ENOSYS);
    uint64_t ns;
    int error = host->clock_ns(host->context, clock, &ns);
    if (error) return fail(error);
    time_t seconds = (time_t)(ns / 1000000000u);
    if (seconds < 0 || (uint64_t)seconds != ns / 1000000000u) return fail(EOVERFLOW);
    out->tv_sec = seconds;
    out->tv_nsec = (long)(ns % 1000000000u);
    return 0;
}

// ------------------------=
// FUNC: mmap
// DESC: Requests native memory or object mapping from the OS; this layer never emulates executable protection with malloc.
// ------------------=
void *mmap(void *address, size_t size, int protection, int flags, int fd, off_t offset) {
    if (!size || offset < 0 || (protection & ~(PROT_READ | PROT_WRITE | PROT_EXEC))) { fail(EINVAL); return MAP_FAILED; }
    if (!host || !host->map) { fail(ENOSYS); return MAP_FAILED; }
    void *result = address;
    int error = host->map(host->context, &result, size, protection, flags, fd, offset);
    if (error) { fail(error); return MAP_FAILED; }
    return result;
}

// ------------------------=
// FUNC: munmap
// DESC: Releases an OS-owned mapping and propagates invalid-range failures.
// ------------------=
int munmap(void *address, size_t size) {
    if (!size) return fail(EINVAL);
    if (!host || !host->unmap) return fail(ENOSYS);
    int error = host->unmap(host->context, address, size);
    return error ? fail(error) : 0;
}

// ------------------------=
// FUNC: mprotect
// DESC: Delegates real protection changes rather than reporting an unimplemented MMU operation as successful.
// ------------------=
int mprotect(void *address, size_t size, int protection) {
    if (!size || (protection & ~(PROT_READ | PROT_WRITE | PROT_EXEC))) return fail(EINVAL);
    if (!host || !host->protect) return fail(ENOSYS);
    int error = host->protect(host->context, address, size, protection);
    return error ? fail(error) : 0;
}

// ------------------------=
// FUNC: msync
// DESC: Publishes writable object mappings through the owning OS service.
// ------------------=
int msync(void *address, size_t size, int flags) {
    if (!size) return fail(EINVAL);
    if (!host || !host->sync) return fail(ENOSYS);
    int error = host->sync(host->context, address, size, flags);
    return error ? fail(error) : 0;
}
