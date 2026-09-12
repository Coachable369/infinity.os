#include <infinity/compiler_host.h>
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <unistd.h>
#include <limits.h>
#include <sys/stat.h>
#include <stdarg.h>
#ifdef __INFINITY__
typedef _READ_WRITE_RETURN_TYPE InfinityIoCount;
#else
typedef ssize_t InfinityIoCount;
#endif

/* Descriptor generations prevent a stale descriptor from reaching a reused
   slot. Exhausted generations retire rather than wrap. 0..2 are not ambient. */
struct FileSlot {
    InfinityCompilerFiles api;
    void *context;
    uint64_t object;
    unsigned generation;
    uint32_t mode;
    int active;
    int open_flags, descriptor_flags;
};
static struct FileSlot slots[128];

// ------------------------=
// FUNC: standard_stream
// DESC: Returns the explicitly launch-bound console service for descriptors zero through two.
// ------------------=
static const InfinityCompilerConsole *standard_stream(int fd, const InfinityCompilerHost **owner) {
    if (fd < 0 || fd > 2) return 0;
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->console) { errno = EBADF; return 0; }
    if (owner) *owner = host;
    return host->console;
}

// ------------------------=
// FUNC: file_error
// DESC: Preserves a native provider error at the compatibility boundary.
// ------------------=
static int file_error(int error) { errno = error; return -1; }

// ------------------------=
// FUNC: lookup
// DESC: Rejects stale, closed and non-object descriptors.
// ------------------=
static struct FileSlot *lookup(int fd) {
    if (fd < 3) { errno = EBADF; return 0; }
    unsigned key = (unsigned)(fd - 3);
    struct FileSlot *slot = &slots[key % 128];
    if (!slot->active || slot->generation != key / 128) { errno = EBADF; return 0; }
    return slot;
}

// ------------------------=
// FUNC: open
// DESC: Translates supported file modes to an authorized native object handle.
// ------------------=
int open(const char *path, int flags, ...) {
    if (!path || !*path) return file_error(EINVAL);
    const int supported = O_ACCMODE | O_CREAT | O_TRUNC | O_APPEND | O_EXCL | O_CLOEXEC;
    if (flags & ~supported) return file_error(ENOTSUP);
    uint32_t mode;
    switch (flags & O_ACCMODE) {
    case O_RDONLY: mode = 1; break;
    case O_WRONLY: mode = 2; break;
    case O_RDWR: mode = 3; break;
    default: return file_error(EINVAL);
    }
    if ((flags & (O_TRUNC | O_APPEND)) && !(mode & 2)) return file_error(EINVAL);
    if ((flags & O_EXCL) && !(flags & O_CREAT)) return file_error(EINVAL);
    if (flags & O_CREAT) mode |= 4;
    if (flags & O_TRUNC) mode |= 8;
    if (flags & O_APPEND) mode |= 16;
    if (flags & O_EXCL) mode |= 32;
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->files || !host->files->open || !host->files->close)
        return file_error(ENOSYS);
    for (unsigned i = 0; i < 128; ++i) {
        struct FileSlot *slot = &slots[i];
        if (slot->active || slot->generation > ((unsigned)INT_MAX - 130) / 128) continue;
        InfinityCompilerFiles api = *host->files;
        void *context = host->context;
        uint64_t object = 0;
        int error = api.open(context, path, mode, &object);
        if (error) return file_error(error);
        slot->api = api; slot->context = context; slot->object = object;
        slot->mode = mode; slot->active = 1;
        slot->open_flags = flags & (O_ACCMODE | O_APPEND);
        slot->descriptor_flags = (flags & O_CLOEXEC) ? FD_CLOEXEC : 0;
        return (int)(slot->generation * 128 + i + 3);
    }
    return file_error(EMFILE);
}

// ------------------------=
// FUNC: read
// DESC: Reads a bounded byte request from the descriptor's owning object provider.
// ------------------=
InfinityIoCount read(int fd, void *buffer, size_t size) {
    if (fd >= 0 && fd <= 2) {
        if (fd != 0) return file_error(EBADF);
        if ((!buffer && size) || size > (size_t)INT_MAX) return file_error(EINVAL);
        const InfinityCompilerHost *owner = 0;
        const InfinityCompilerConsole *console = standard_stream(fd, &owner);
        if (!console || !console->read) return file_error(console ? ENOSYS : errno);
        size_t count = 0;
        int error = console->read(owner->context, (uint32_t)fd, buffer, size, &count);
        if (error) return file_error(error);
        return count > size ? file_error(EIO) : (InfinityIoCount)count;
    }
    struct FileSlot *slot = lookup(fd);
    if (!slot) return -1;
    if (!(slot->mode & 1)) return file_error(EBADF);
    if ((!buffer && size) || size > (size_t)INT_MAX) return file_error(EINVAL);
    if (!slot->api.read) return file_error(ENOSYS);
    size_t count = 0;
    int error = slot->api.read(slot->context, slot->object, buffer, size, &count);
    if (error) return file_error(error);
    if (count > size) return file_error(EIO);
    return (InfinityIoCount)count;
}

// ------------------------=
// FUNC: write
// DESC: Writes through the original provider without bypassing native version checks.
// ------------------=
InfinityIoCount write(int fd, const void *buffer, size_t size) {
    if (fd >= 0 && fd <= 2) {
        if (fd == 0) return file_error(EBADF);
        if ((!buffer && size) || size > (size_t)INT_MAX) return file_error(EINVAL);
        const InfinityCompilerHost *owner = 0;
        const InfinityCompilerConsole *console = standard_stream(fd, &owner);
        if (!console || !console->write) return file_error(console ? ENOSYS : errno);
        size_t count = 0;
        int error = console->write(owner->context, (uint32_t)fd, buffer, size, &count);
        if (error) return file_error(error);
        return count > size ? file_error(EIO) : (InfinityIoCount)count;
    }
    struct FileSlot *slot = lookup(fd);
    if (!slot) return -1;
    if (!(slot->mode & 2)) return file_error(EBADF);
    if ((!buffer && size) || size > (size_t)INT_MAX) return file_error(EINVAL);
    if (!slot->api.write) return file_error(ENOSYS);
    size_t count = 0;
    int error = slot->api.write(slot->context, slot->object, buffer, size, &count);
    if (error) return file_error(error);
    if (count > size) return file_error(EIO);
    return (InfinityIoCount)count;
}

// ------------------------=
// FUNC: lseek
// DESC: Forwards validated seek origin and rejects unrepresentable offsets.
// ------------------=
off_t lseek(int fd, off_t offset, int origin) {
    struct FileSlot *slot = lookup(fd);
    if (!slot) return -1;
    if (origin != SEEK_SET && origin != SEEK_CUR && origin != SEEK_END) return file_error(EINVAL);
    if (!slot->api.seek) return file_error(ENOSYS);
    int64_t result = 0;
    int error = slot->api.seek(slot->context, slot->object, offset, origin, &result);
    if (error) return file_error(error);
    if (result < 0 || (int64_t)(off_t)result != result) return file_error(EOVERFLOW);
    return (off_t)result;
}

// ------------------------=
// FUNC: close
// DESC: Consumes the handle even on commit failure and invalidates stale descriptors.
// ------------------=
int close(int fd) {
    struct FileSlot *slot = lookup(fd);
    if (!slot) return -1;
    slot->active = 0;
    ++slot->generation;
    int error = slot->api.close(slot->context, slot->object);
    return error ? file_error(error) : 0;
}

// ------------------------=
// FUNC: pread
// DESC: Reads an explicit object offset without changing the sequential cursor.
// ------------------=
ssize_t pread(int fd, void *buffer, size_t size, off_t offset) {
    struct FileSlot *slot = lookup(fd);
    if (!slot) return -1;
    if (!(slot->mode & 1)) return file_error(EBADF);
    if ((!buffer && size) || size > (size_t)INT_MAX || offset < 0) return file_error(EINVAL);
    if (!slot->api.read_at) return file_error(ENOSYS);
    size_t count = 0;
    int error = slot->api.read_at(slot->context, slot->object, (uint64_t)offset, buffer, size, &count);
    if (error) return file_error(error);
    return count > size ? file_error(EIO) : (ssize_t)count;
}

// ------------------------=
// FUNC: convert_metadata
// DESC: Projects native object metadata into representable compatibility fields atomically.
// ------------------=
static int convert_metadata(const InfinityCompilerMetadata *value, struct stat *out) {
    if (!out) return file_error(EFAULT);
    if ((value->kind != 1 && value->kind != 2) || value->access > 7 ||
        value->modified_nanoseconds >= 1000000000) return file_error(EIO);
    struct stat result = {0};
    result.st_ino = (ino_t)value->identity;
    result.st_size = (off_t)value->size;
    result.st_mtime = (time_t)value->modified_seconds;
    if ((uint64_t)result.st_ino != value->identity || result.st_size < 0 ||
        (uint64_t)result.st_size != value->size ||
        (int64_t)result.st_mtime != value->modified_seconds) return file_error(EOVERFLOW);
#ifdef __APPLE__
    result.st_mtimespec.tv_nsec = value->modified_nanoseconds;
#else
    result.st_mtim.tv_nsec = value->modified_nanoseconds;
#endif
    result.st_mode = value->kind == 1 ? S_IFREG : S_IFDIR;
    if (value->access & 1) result.st_mode |= S_IRUSR;
    if (value->access & 2) result.st_mode |= S_IWUSR;
    if (value->access & 4) result.st_mode |= S_IXUSR;
    *out = result;
    return 0;
}

// ------------------------=
// FUNC: fstat
// DESC: Inspects the opened snapshot through its owning provider.
// ------------------=
int fstat(int fd, struct stat *out) {
    if (fd >= 0 && fd <= 2) {
        if (!out) return file_error(EFAULT);
        if (!standard_stream(fd, 0)) return -1;
        struct stat result = {0};
        result.st_mode = S_IFCHR | (fd == 0 ? S_IRUSR : S_IWUSR);
        *out = result;
        return 0;
    }
    struct FileSlot *slot = lookup(fd);
    if (!slot) return -1;
    if (!out) return file_error(EFAULT);
    if (!slot->api.inspect) return file_error(ENOSYS);
    InfinityCompilerMetadata value = {0};
    int error = slot->api.inspect(slot->context, slot->object, &value);
    return error ? file_error(error) : convert_metadata(&value, out);
}

// ------------------------=
// FUNC: path_metadata
// DESC: Requests authorized native namespace metadata, never consulting a host filesystem.
// ------------------=
static int path_metadata(const char *path, InfinityCompilerMetadata *value) {
    if (!path || !*path) return file_error(EINVAL);
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->files || !host->files->inspect_path) return file_error(ENOSYS);
    int error = host->files->inspect_path(host->context, path, value);
    return error ? file_error(error) : 0;
}

// ------------------------=
// FUNC: stat
// DESC: Returns current path metadata while preserving output on failure.
// ------------------=
int stat(const char *path, struct stat *out) {
    if (!out) return file_error(EFAULT);
    InfinityCompilerMetadata value = {0};
    if (path_metadata(path, &value)) return -1;
    return convert_metadata(&value, out);
}

// ------------------------=
// FUNC: access
// DESC: Checks effective native rights without granting subsequent operations authority.
// ------------------=
int access(const char *path, int mode) {
    if (mode & ~(R_OK | W_OK | X_OK)) return file_error(EINVAL);
    InfinityCompilerMetadata value = {0};
    if (path_metadata(path, &value)) return -1;
    if (value.access > 7 || (value.kind != 1 && value.kind != 2)) return file_error(EIO);
    uint32_t requested = ((mode & R_OK) ? 1u : 0u) |
                         ((mode & W_OK) ? 2u : 0u) | ((mode & X_OK) ? 4u : 0u);
    return (value.access & requested) == requested ? 0 : file_error(EACCES);
}

// ------------------------=
// FUNC: fcntl
// DESC: Maintains descriptor flags; unsupported locking and duplication requests remain explicit errors.
// ------------------=
int fcntl(int fd, int command, ...) {
    struct FileSlot *slot = lookup(fd);
    if (!slot) return -1;
    if (command == F_GETFD) return slot->descriptor_flags;
    if (command == F_GETFL) return slot->open_flags;
    if (command == F_SETFD) {
        va_list args; va_start(args, command);
        int flags = va_arg(args, int); va_end(args);
        if (flags & ~FD_CLOEXEC) return file_error(EINVAL);
        slot->descriptor_flags = flags;
        return 0;
    }
    return file_error(ENOTSUP);
}

// ------------------------=
// FUNC: ftruncate
// DESC: Changes an authorized object's snapshot length through the owning versioned provider.
// ------------------=
int ftruncate(int fd, off_t length) {
    struct FileSlot *slot = lookup(fd);
    if (!slot) return -1;
    if (!(slot->mode & 2)) return file_error(EBADF);
    if (length < 0) return file_error(EINVAL);
    if (!slot->api.truncate) return file_error(ENOSYS);
    int error = slot->api.truncate(slot->context, slot->object, (uint64_t)length);
    return error ? file_error(error) : 0;
}
