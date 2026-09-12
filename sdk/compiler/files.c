#include <infinity/compiler_host.h>
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <unistd.h>
#include <limits.h>
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
};
static struct FileSlot slots[128];

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
    const int supported = O_ACCMODE | O_CREAT | O_TRUNC | O_APPEND | O_EXCL;
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
        return (int)(slot->generation * 128 + i + 3);
    }
    return file_error(EMFILE);
}

// ------------------------=
// FUNC: read
// DESC: Reads a bounded byte request from the descriptor's owning object provider.
// ------------------=
InfinityIoCount read(int fd, void *buffer, size_t size) {
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
