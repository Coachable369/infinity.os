/* SQLite's native VFS for the launch-bound, process-private Python namespace.
 * File authority and persistence are exactly those of that namespace. This
 * does not expose a Unix filesystem, shared-memory WAL, or ambient processes. */
#include "sqlite3.h"
#include <infinity/compiler_host.h>
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>

typedef struct NativeSqlFile {
    sqlite3_file base;
    int fd, lock, remove;
    char path[256];
    struct NativeSqlFile *next;
} NativeSqlFile;
static NativeSqlFile *files;
static uint64_t temporary_sequence;

// ------------------------=
// FUNC: native_close
// DESC: Releases a real descriptor and removes only its explicitly temporary namespace entry.
// ------------------=
static int native_close(sqlite3_file *file) {
    NativeSqlFile *f = (NativeSqlFile *)file;
    NativeSqlFile **entry = &files;
    while (*entry && *entry != f) entry = &(*entry)->next;
    if (*entry) *entry = f->next;
    int result = close(f->fd) ? SQLITE_IOERR_CLOSE : SQLITE_OK;
    if (f->remove && unlink(f->path) && errno != ENOENT) result = SQLITE_IOERR_DELETE;
    f->base.pMethods = 0;
    return result;
}

// ------------------------=
// FUNC: native_read
// DESC: Reads the requested offset and zero-fills short reads as required by SQLite's pager contract.
// ------------------=
static int native_read(sqlite3_file *file, void *buffer, int count, sqlite3_int64 offset) {
    NativeSqlFile *f = (NativeSqlFile *)file;
    if (count < 0 || offset < 0 || (sqlite3_int64)(off_t)offset != offset) return SQLITE_IOERR_READ;
    int done = 0;
    while (done < count) {
        ssize_t n = pread(f->fd, (char *)buffer + done, count - done, (off_t)offset + done);
        if (n < 0) { if (errno == EINTR) continue; return SQLITE_IOERR_READ; }
        if (!n) { memset((char *)buffer + done, 0, count - done); return SQLITE_IOERR_SHORT_READ; }
        done += (int)n;
    }
    return SQLITE_OK;
}

// ------------------------=
// FUNC: native_write
// DESC: Writes through the granted provider without claiming success for partial or denied writes.
// ------------------=
static int native_write(sqlite3_file *file, const void *buffer, int count, sqlite3_int64 offset) {
    NativeSqlFile *f = (NativeSqlFile *)file;
    if (count < 0 || offset < 0 || (sqlite3_int64)(off_t)offset != offset
        || lseek(f->fd, (off_t)offset, SEEK_SET) != offset) return SQLITE_IOERR_WRITE;
    int done = 0;
    while (done < count) {
        ssize_t n = write(f->fd, (const char *)buffer + done, count - done);
        if (n < 0 && errno == EINTR) continue;
        if (n <= 0) return errno == ENOSPC || errno == EFBIG ? SQLITE_FULL : SQLITE_IOERR_WRITE;
        done += (int)n;
    }
    return SQLITE_OK;
}

// ------------------------=
// FUNC: native_truncate
// DESC: Truncates only a writable process-bound file, preserving provider errors.
// ------------------=
static int native_truncate(sqlite3_file *file, sqlite3_int64 size) {
    if (size < 0 || (sqlite3_int64)(off_t)size != size) return SQLITE_IOERR_TRUNCATE;
    return ftruncate(((NativeSqlFile *)file)->fd, (off_t)size) ? SQLITE_IOERR_TRUNCATE : SQLITE_OK;
}

// ------------------------=
// FUNC: native_sync
// DESC: Uses the namespace's real transaction flush; a RAM namespace is not advertised as reboot durable.
// ------------------=
static int native_sync(sqlite3_file *file, int flags) {
    (void)flags;
    return fsync(((NativeSqlFile *)file)->fd) ? SQLITE_IOERR_FSYNC : SQLITE_OK;
}

// ------------------------=
// FUNC: native_size
// DESC: Reports the current file provider's exact length.
// ------------------=
static int native_size(sqlite3_file *file, sqlite3_int64 *out) {
    struct stat value;
    if (fstat(((NativeSqlFile *)file)->fd, &value)) return SQLITE_IOERR_FSTAT;
    *out = value.st_size;
    return SQLITE_OK;
}

// ------------------------=
// FUNC: native_lock
// DESC: Enforces pager locks between connections sharing this process-private namespace.
// ------------------=
static int native_lock(sqlite3_file *file, int requested) {
    NativeSqlFile *f = (NativeSqlFile *)file;
    if (requested <= f->lock) return SQLITE_OK;
    for (NativeSqlFile *other = files; other; other = other->next) {
        if (other == f || strcmp(other->path, f->path)) continue;
        if ((requested == SQLITE_LOCK_SHARED && other->lock >= SQLITE_LOCK_PENDING)
            || (requested == SQLITE_LOCK_RESERVED && other->lock >= SQLITE_LOCK_RESERVED)
            || (requested == SQLITE_LOCK_PENDING && other->lock >= SQLITE_LOCK_PENDING)
            || (requested == SQLITE_LOCK_EXCLUSIVE && other->lock != SQLITE_LOCK_NONE)) return SQLITE_BUSY;
    }
    f->lock = requested;
    return SQLITE_OK;
}

// ------------------------=
// FUNC: native_unlock
// DESC: Lowers an existing connection lock without changing another connection's authority.
// ------------------=
static int native_unlock(sqlite3_file *file, int requested) {
    NativeSqlFile *f = (NativeSqlFile *)file;
    if (requested > f->lock || requested < SQLITE_LOCK_NONE) return SQLITE_IOERR_UNLOCK;
    f->lock = requested;
    return SQLITE_OK;
}

// ------------------------=
// FUNC: native_reserved
// DESC: Inspects actual live reservations for the same canonical database name.
// ------------------=
static int native_reserved(sqlite3_file *file, int *out) {
    NativeSqlFile *f = (NativeSqlFile *)file;
    *out = 0;
    for (NativeSqlFile *other = files; other; other = other->next)
        if (!strcmp(other->path, f->path) && other->lock >= SQLITE_LOCK_RESERVED) *out = 1;
    return SQLITE_OK;
}

// ------------------------=
// FUNC: native_control
// DESC: Exposes meaningful file state while declining unsupported native file controls.
// ------------------=
static int native_control(sqlite3_file *file, int operation, void *argument) {
    if (operation == SQLITE_FCNTL_LOCKSTATE) { *(int *)argument = ((NativeSqlFile *)file)->lock; return SQLITE_OK; }
    if (operation == SQLITE_FCNTL_VFSNAME) { *(char **)argument = sqlite3_mprintf("infinity-private"); return *(char **)argument ? SQLITE_OK : SQLITE_NOMEM; }
    return SQLITE_NOTFOUND;
}

// ------------------------=
// FUNC: native_sector
// DESC: Uses the conservative pager sector size rather than inventing atomic device writes.
// ------------------=
static int native_sector(sqlite3_file *file) { (void)file; return 4096; }

// ------------------------=
// FUNC: native_characteristics
// DESC: Makes no atomic-write or power-loss guarantees beyond the actual file provider.
// ------------------=
static int native_characteristics(sqlite3_file *file) { (void)file; return 0; }

static const sqlite3_io_methods methods = {
    .iVersion = 1, .xClose = native_close, .xRead = native_read, .xWrite = native_write,
    .xTruncate = native_truncate, .xSync = native_sync, .xFileSize = native_size,
    .xLock = native_lock, .xUnlock = native_unlock, .xCheckReservedLock = native_reserved,
    .xFileControl = native_control, .xSectorSize = native_sector, .xDeviceCharacteristics = native_characteristics
};

// ------------------------=
// FUNC: native_open
// DESC: Opens only launch-bound file paths, with exclusive creation for unnamed temporary files.
// ------------------=
static int native_open(sqlite3_vfs *vfs, const char *path, sqlite3_file *file, int flags, int *actual) {
    (void)vfs;
    NativeSqlFile *f = (NativeSqlFile *)file;
    memset(f, 0, sizeof(*f));
    if (!path) {
        if (temporary_sequence == UINT64_MAX) return SQLITE_CANTOPEN;
        snprintf(f->path, sizeof(f->path), "/tmp/sqlite-%llu", (unsigned long long)++temporary_sequence);
        flags |= SQLITE_OPEN_CREATE | SQLITE_OPEN_EXCLUSIVE | SQLITE_OPEN_DELETEONCLOSE;
    } else {
        if (strlen(path) >= sizeof(f->path)) return SQLITE_CANTOPEN;
        strcpy(f->path, path);
    }
    int mode = flags & SQLITE_OPEN_READONLY ? O_RDONLY : O_RDWR;
    if (flags & SQLITE_OPEN_CREATE) mode |= O_CREAT;
    if (flags & SQLITE_OPEN_EXCLUSIVE) mode |= O_EXCL;
    f->fd = open(f->path, mode, 0600);
    if (f->fd < 0) return SQLITE_CANTOPEN;
    f->remove = !!(flags & SQLITE_OPEN_DELETEONCLOSE);
    f->next = files; files = f; f->base.pMethods = &methods;
    if (actual) *actual = flags;
    return SQLITE_OK;
}

// ------------------------=
// FUNC: native_delete
// DESC: Removes a scoped file and propagates failure without claiming directory durability.
// ------------------=
static int native_delete(sqlite3_vfs *vfs, const char *path, int sync_directory) {
    (void)vfs; (void)sync_directory;
    return unlink(path) && errno != ENOENT ? SQLITE_IOERR_DELETE : SQLITE_OK;
}

// ------------------------=
// FUNC: native_access
// DESC: Queries current provider permissions rather than granting future access.
// ------------------=
static int native_access(sqlite3_vfs *vfs, const char *path, int mode, int *out) {
    (void)vfs;
    int permissions = mode == SQLITE_ACCESS_READWRITE ? R_OK | W_OK : mode == SQLITE_ACCESS_READ ? R_OK : F_OK;
    *out = access(path, permissions) == 0;
    return *out || errno == ENOENT || errno == EACCES ? SQLITE_OK : SQLITE_IOERR_ACCESS;
}

// ------------------------=
// FUNC: native_path
// DESC: Resolves existing entries or a missing leaf beneath a real namespace directory; creation still requires xOpen authority.
// ------------------=
static int native_path(sqlite3_vfs *vfs, const char *path, int capacity, char *out) {
    (void)vfs;
    char canonical[256];
    if (!path || !*path || capacity <= 0 || strlen(path) >= sizeof(canonical)) return SQLITE_CANTOPEN;
    if (!realpath(path, canonical)) {
        if (errno != ENOENT) return SQLITE_CANTOPEN;
        char parent[256];
        const char *slash = strrchr(path, '/');
        const char *leaf = slash ? slash + 1 : path;
        if (!*leaf || !strcmp(leaf, ".") || !strcmp(leaf, "..")) return SQLITE_CANTOPEN;
        size_t parent_length = slash ? (size_t)(slash - path) : 0;
        if (parent_length) {
            memcpy(parent, path, parent_length);
            parent[parent_length] = 0;
        } else {
            strcpy(parent, slash ? "/" : ".");
        }
        struct stat directory;
        if (!realpath(parent, canonical) || stat(canonical, &directory) || !S_ISDIR(directory.st_mode))
            return SQLITE_CANTOPEN;
        size_t length = strlen(canonical);
        size_t separator = length > 1 ? 1 : 0;
        if (length + separator + strlen(leaf) >= sizeof(canonical)) return SQLITE_CANTOPEN;
        if (separator) canonical[length++] = '/';
        strcpy(canonical + length, leaf);
    }
    if (strlen(canonical) >= (size_t)capacity) return SQLITE_CANTOPEN;
    strcpy(out, canonical);
    return SQLITE_OK;
}

// ------------------------=
// FUNC: native_random
// DESC: Fills randomness exclusively from the launch's native entropy service.
// ------------------=
static int native_random(sqlite3_vfs *vfs, int count, char *out) {
    (void)vfs;
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    return count > 0 && host && host->entropy && !host->entropy(host->context, out, count) ? count : 0;
}

// ------------------------=
// FUNC: native_sleep
// DESC: Yields through the native relative wait and reports only a completed duration.
// ------------------=
static int native_sleep(sqlite3_vfs *vfs, int microseconds) {
    (void)vfs;
    return microseconds > 0 && !usleep(microseconds) ? microseconds : 0;
}

// ------------------------=
// FUNC: native_time
// DESC: Converts the actual realtime clock to SQLite's Julian millisecond epoch.
// ------------------=
static int native_time(sqlite3_vfs *vfs, sqlite3_int64 *out) {
    (void)vfs;
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    uint64_t ns = 0;
    if (!host || !host->clock_ns || host->clock_ns(host->context, CLOCK_REALTIME, &ns)) return SQLITE_ERROR;
    *out = (sqlite3_int64)(ns / 1000000) + INT64_C(210866760000000);
    return SQLITE_OK;
}

// ------------------------=
// FUNC: native_time_double
// DESC: Provides the legacy Julian-day query from the same measured clock.
// ------------------=
static int native_time_double(sqlite3_vfs *vfs, double *out) {
    sqlite3_int64 milliseconds;
    int result = native_time(vfs, &milliseconds);
    if (result == SQLITE_OK) *out = (double)milliseconds / 86400000.0;
    return result;
}

static sqlite3_vfs native_vfs = {
    .iVersion = 2, .szOsFile = sizeof(NativeSqlFile), .mxPathname = 255,
    .zName = "infinity-private", .xOpen = native_open, .xDelete = native_delete,
    .xAccess = native_access, .xFullPathname = native_path, .xRandomness = native_random,
    .xSleep = native_sleep, .xCurrentTime = native_time_double, .xCurrentTimeInt64 = native_time
};

// ------------------------=
// FUNC: sqlite3_os_init
// DESC: Registers the native VFS as the only default, with no Unix or host fallback.
// ------------------=
int sqlite3_os_init(void) { return sqlite3_vfs_register(&native_vfs, 1); }

// ------------------------=
// FUNC: sqlite3_os_end
// DESC: Unregisters the VFS after SQLite has released its connections.
// ------------------=
int sqlite3_os_end(void) { return sqlite3_vfs_unregister(&native_vfs); }
