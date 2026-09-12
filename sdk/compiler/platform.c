#include <infinity/compiler_host.h>
#include <sys/mman.h>
#include <time.h>
#include <errno.h>
#include <sys/random.h>
#include <dirent.h>
#include <sys/statvfs.h>
#include <sys/resource.h>
#include <sys/wait.h>
#include <sys/stat.h>

static const InfinityCompilerHost *host;
static InfinityCompilerHost crash_owner;
static int crash_registered;

/* Handles never recycle, so a closed pointer cannot become another directory.
   This initial bridge bounds each compiler process to 64 directory opens. */
struct InfinityDirectory {
    InfinityCompilerHost owner;
    uint64_t cursor;
    int active;
    struct dirent entry;
};
static struct InfinityDirectory directories[64];
static size_t directory_count;

// ------------------------=
// FUNC: infinity_compiler_set_host
// DESC: Installs the serial compiler invocation's explicit OS service table.
// ------------------=
void infinity_compiler_set_host(const InfinityCompilerHost *services) { host = services; }

// ------------------------=
// FUNC: infinity_compiler_get_host
// DESC: Returns this invocation's explicit service table without ambient host fallback.
// ------------------=
const InfinityCompilerHost *infinity_compiler_get_host(void) { return host; }

// ------------------------=
// FUNC: infinity_compiler_executable_path
// DESC: Returns the launcher's actual executable object reference without requiring procfs or a host path.
// ------------------=
const char *infinity_compiler_executable_path(void) {
    return host && host->executable_path ? host->executable_path : "";
}

// ------------------------=
// FUNC: infinity_compiler_home_path
// DESC: Returns only the launcher's authorized native home namespace.
// ------------------=
const char *infinity_compiler_home_path(void) { return host ? host->home_path : 0; }

// ------------------------=
// FUNC: fail
// DESC: Preserves an explicit service failure in errno without pretending the operation succeeded.
// ------------------=
static int fail(int error) { errno = error; return -1; }

// ------------------------=
// FUNC: infinity_compiler_register_crash_handler
// DESC: Requests real context fault notification; missing registration support fails explicitly.
// ------------------=
int infinity_compiler_register_crash_handler(void (*callback)(void *), void *context) {
    if (!callback) return fail(EINVAL);
    if (crash_registered) return fail(EBUSY);
    if (!host || !host->register_crash_handler || !host->unregister_crash_handler)
        return fail(ENOSYS);
    InfinityCompilerHost owner = *host;
    int error = owner.register_crash_handler(owner.context, callback, context);
    if (!error) { crash_owner = owner; crash_registered = 1; }
    return error ? fail(error) : 0;
}

// ------------------------=
// FUNC: infinity_compiler_unregister_crash_handler
// DESC: Releases through the registering provider; failed release retains ownership for retry.
// ------------------=
int infinity_compiler_unregister_crash_handler(void) {
    if (!crash_registered) return fail(EINVAL);
    int error = crash_owner.unregister_crash_handler(crash_owner.context);
    if (!error) { crash_registered = 0; crash_owner = (InfinityCompilerHost){0}; }
    return error ? fail(error) : 0;
}

// ------------------------=
// FUNC: getrlimit
// DESC: Rejects unavailable resource measurements without inventing limits.
// ------------------=
int getrlimit(int resource, struct rlimit *out) {
    (void)resource; return fail(out ? ENOSYS : EFAULT);
}
// ------------------------=
// FUNC: setrlimit
// DESC: Rejects changes until a capability-governed resource service is bound.
// ------------------=
int setrlimit(int resource, const struct rlimit *limit) {
    (void)resource; return fail(limit ? ENOSYS : EFAULT);
}
// ------------------------=
// FUNC: getrusage
// DESC: Reports unavailable process observations without publishing fabricated counters.
// ------------------=
int getrusage(int who, struct rusage *out) {
    (void)who; return fail(out ? ENOSYS : EFAULT);
}
// ------------------------=
// FUNC: wait4
// DESC: Rejects Unix child waits without changing status or usage outputs.
// ------------------=
pid_t wait4(pid_t pid, int *status, int options, struct rusage *usage) {
    (void)pid; (void)status; (void)options; (void)usage; return fail(ENOSYS);
}
// ------------------------=
// FUNC: lstat
// DESC: Rejects unbound link metadata queries rather than falling through to host storage.
// ------------------=
int lstat(const char *path, struct stat *out) {
    return fail(path && out ? ENOSYS : EFAULT);
}
// ------------------------=
// FUNC: madvise
// DESC: Reports unavailable memory advice without promising page eviction or prefetch.
// ------------------=
int madvise(void *address, size_t size, int advice) {
    (void)address; (void)advice; return fail(size ? ENOSYS : EINVAL);
}

// ------------------------=
// FUNC: valid_directory
// DESC: Checks handle identity before dereferencing caller-provided pointers.
// ------------------=
static int valid_directory(DIR *directory) {
    for (size_t i = 0; i < directory_count; ++i)
        if (directory == &directories[i]) return directory->active;
    return 0;
}

// ------------------------=
// FUNC: opendir
// DESC: Opens a bounded, explicitly authorized provider cursor without host I/O.
// ------------------=
DIR *opendir(const char *path) {
    if (!path || !*path) { fail(EINVAL); return 0; }
    if (!host || !host->directory_open || !host->directory_next ||
        !host->directory_close) { fail(ENOSYS); return 0; }
    if (directory_count == 64) { fail(EMFILE); return 0; }
    InfinityCompilerHost owner = *host;
    uint64_t cursor = 0;
    int error = owner.directory_open(owner.context, path, &cursor);
    if (error) { fail(error); return 0; }
    DIR *directory = &directories[directory_count++];
    directory->owner = owner;
    directory->cursor = cursor;
    directory->active = 1;
    return directory;
}

// ------------------------=
// FUNC: readdir
// DESC: Publishes only complete direct-child names and distinguishes EOF from failure.
// ------------------=
struct dirent *readdir(DIR *directory) {
    if (!valid_directory(directory)) { fail(EBADF); return 0; }
    char name[96] = {0};
    int error = directory->owner.directory_next(directory->owner.context,
        directory->cursor, name, sizeof name);
    if (error) { fail(error); return 0; }
    size_t length = 0;
    while (length < sizeof name && name[length]) ++length;
    if (length == sizeof name) { fail(EOVERFLOW); return 0; }
    if (!length) { errno = 0; return 0; }
    for (size_t i = 0; i < length; ++i)
        if (name[i] == '/') { fail(EIO); return 0; }
    if ((length == 1 && name[0] == '.') ||
        (length == 2 && name[0] == '.' && name[1] == '.')) {
        fail(EIO); return 0;
    }
    for (size_t i = 0; i <= length; ++i) directory->entry.d_name[i] = name[i];
    return &directory->entry;
}

// ------------------------=
// FUNC: closedir
// DESC: Closes through the opening provider and permanently invalidates the handle.
// ------------------=
int closedir(DIR *directory) {
    if (!valid_directory(directory)) return fail(EBADF);
    directory->active = 0;
    int error = directory->owner.directory_close(directory->owner.context,
        directory->cursor);
    return error ? fail(error) : 0;
}

// ------------------------=
// FUNC: statvfs
// DESC: Reports unavailable capacity observations without fabricating Pool statistics.
// ------------------=
int statvfs(const char *path, struct statvfs *out) {
    return fail(!path || !out ? EFAULT : ENOSYS);
}

// ------------------------=
// FUNC: fstatvfs
// DESC: Rejects unsupported descriptor-capacity queries without modifying output.
// ------------------=
int fstatvfs(int fd, struct statvfs *out) {
    return fail(!out ? EFAULT : fd < 0 ? EBADF : ENOSYS);
}

// ------------------------=
// FUNC: getentropy
// DESC: Obtains up to 256 bytes from the native entropy service, publishing no partial output on failure.
// ------------------=
int getentropy(void *out, size_t size) {
    if (size > 256) return fail(EIO);
    if (!size) return 0;
    if (!out) return fail(EFAULT);
    if (!host || !host->entropy) return fail(ENOSYS);
    unsigned char temporary[256] = {0};
    int error = host->entropy(host->context, temporary, size);
    if (!error) {
        unsigned char *bytes = out;
        for (size_t i = 0; i < size; ++i) bytes[i] = temporary[i];
    }
    volatile unsigned char *wipe = temporary;
    for (size_t i = 0; i < sizeof temporary; ++i) wipe[i] = 0;
    return error ? fail(error) : 0;
}

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
