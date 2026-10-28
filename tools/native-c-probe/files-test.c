#include <infinity/compiler_host.h>
#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <unistd.h>
#include <string.h>
#include <sys/stat.h>
static const InfinityCompilerHost *host;
static int denied, closes;
static unsigned char written[12347];
static size_t write_cursor, write_limit = sizeof written, write_chunk = 4096;
static int write_fixture, write_fault;
static uint32_t observed_mode;
static InfinityCompilerMetadata metadata = {41, 3, 123, 456, 1, 3};
// ------------------------=
// FUNC: infinity_compiler_get_host
// DESC: Supplies explicit fixture services without host file access.
// ------------------=
const InfinityCompilerHost *infinity_compiler_get_host(void) { return host; }
// ------------------------=
// FUNC: object_open
// DESC: Captures translated mode bits and returns a controlled object identity.
// ------------------=
static int object_open(void *context, const char *path, uint32_t mode, uint64_t *out) {
    assert(context == &denied && path && *path);
    observed_mode = mode; *out = 41; return denied;
}
// ------------------------=
// FUNC: object_read
// DESC: Models revocation and a short object read.
// ------------------=
static int object_read(void *context, uint64_t id, void *out, size_t size, size_t *count) {
    assert(context == &denied && id == 41);
    if (denied) return denied;
    *count = size < 3 ? size : 3;
    memcpy(out, "abc", *count); return 0;
}
// ------------------------=
// FUNC: object_write
// DESC: Verifies provider delivery of exact payload bytes.
// ------------------=
static int object_write(void *context, uint64_t id, const void *data, size_t size, size_t *count) {
    if (write_fixture) {
        assert(context == &denied && id == 41);
        if (denied) return denied;
        if (write_cursor >= write_limit) return ENOSPC;
        if (write_fault == 1) { *count = 0; return 0; }
        if (write_fault == 2) { *count = size + 1; return 0; }
        *count = size < write_chunk ? size : write_chunk;
        if (*count > write_limit - write_cursor) *count = write_limit - write_cursor;
        assert(*count <= sizeof written - write_cursor);
        if (*count) memcpy(written + write_cursor, data, *count);
        write_cursor += *count;
        return 0;
    }
    assert(context == &denied && id == 41 && size == 3 && memcmp(data, "xyz", 3) == 0);
    *count = size; return denied;
}
// ------------------------=
// FUNC: object_seek
// DESC: Returns a controlled absolute offset.
// ------------------=
static int object_seek(void *context, uint64_t id, int64_t offset, int origin, int64_t *out) {
    assert(context == &denied && id == 41 && origin == SEEK_SET);
    *out = offset; return denied;
}
// ------------------------=
// FUNC: object_close
// DESC: Counts consumed cursors, including failed commits.
// ------------------=
static int object_close(void *context, uint64_t id) {
    assert(context == &denied && id == 41); ++closes; return denied;
}
// ------------------------=
// FUNC: object_read_at
// DESC: Supplies a positional snapshot read independently of cursor operations.
// ------------------=
static int object_read_at(void *context, uint64_t id, uint64_t offset, void *out, size_t size, size_t *count) {
    assert(offset == 2);
    return object_read(context, id, out, size, count);
}
// ------------------------=
// FUNC: object_inspect
// DESC: Supplies controlled metadata or a revocation failure.
// ------------------=
static int object_inspect(void *context, uint64_t id, InfinityCompilerMetadata *out) {
    assert(context == &denied && id == 41);
    *out = metadata; return denied;
}
// ------------------------=
// FUNC: path_inspect
// DESC: Resolves a fixture path without accessing the host filesystem.
// ------------------=
static int path_inspect(void *context, const char *path, InfinityCompilerMetadata *out) {
    assert(path && *path); return object_inspect(context, 41, out);
}
// ------------------------=
// FUNC: console_read
// DESC: Returns bounded input only through standard stream zero.
// ------------------=
static int console_read(void *context, uint32_t stream, void *out, size_t size, size_t *count) {
    assert(context == &denied && stream == 0 && size == 4);
    if (denied) return denied;
    memcpy(out, "in", 2); *count = 2; return 0;
}
// ------------------------=
// FUNC: console_write
// DESC: Verifies diagnostic bytes are forwarded to a launch-bound output stream.
// ------------------=
static int console_write(void *context, uint32_t stream, const void *bytes, size_t size, size_t *count) {
    assert(context == &denied && stream == 2 && size == 3 && !memcmp(bytes, "err", 3));
    if (denied) return denied;
    *count = size; return 0;
}
// ------------------------=
// FUNC: object_truncate
// DESC: Verifies exact length forwarding and provider failure propagation.
// ------------------=
static int object_truncate(void *context, uint64_t id, uint64_t length) {
    assert(context == &denied && id == 41 && length == 7); return denied;
}
// ------------------------=
// FUNC: object_flush
// DESC: Checks descriptor ownership and exact transaction flush failure propagation.
// ------------------=
static int object_flush(void *context, uint64_t id) {
    assert(context == &denied && id == 41); return denied;
}
// ------------------------=
// FUNC: object_permissions
// DESC: Verifies descriptor permission forwarding without adding namespace authority.
// ------------------=
static int object_permissions(void *context, uint64_t id, uint32_t mode) {
    assert(mode == 0644); return object_flush(context, id);
}
// ------------------------=
// FUNC: main
// DESC: Tests mode translation, revocation, ownership, stale descriptors, reuse and bounded capacity.
// ------------------=
int main(void) {
    InfinityCompilerFiles files = {object_open, object_read, object_write, object_seek, object_close,
                                  object_read_at, object_inspect, path_inspect, object_truncate,
                                  object_flush, object_permissions};
    InfinityCompilerConsole console = {.read = console_read, .write = console_write};
    InfinityCompilerHost services = {.context = &denied, .files = &files, .console = &console};
    char console_buffer[4] = {0};
    denied = 0;
    host = &services;
    assert(read(0, console_buffer, sizeof console_buffer) == 2 && !memcmp(console_buffer, "in", 2));
    assert(write(2, "err", 3) == 3);
    struct stat console_info = {0};
    assert(fstat(2, &console_info) == 0 && S_ISCHR(console_info.st_mode));
    assert(read(1, console_buffer, 1) == -1 && errno == EBADF);
    assert(write(0, "x", 1) == -1 && errno == EBADF);
    host = 0;
    assert(open("object", O_RDONLY) == -1 && errno == ENOSYS);
    host = &services;
    assert(open("object", O_RDONLY | O_TRUNC) == -1 && errno == EINVAL);
    int fd = open("object", O_RDWR | O_CREAT | O_EXCL, 0600);
    assert(fd >= 3 && observed_mode == 39);
    assert(fcntl(fd, F_GETFD) == 0);
    assert(fcntl(fd, F_SETFD, FD_CLOEXEC) == 0 && fcntl(fd, F_GETFD) == FD_CLOEXEC);
    assert(fcntl(fd, F_GETFL) == O_RDWR);
    assert(fcntl(fd, F_SETFL, O_APPEND) == -1 && errno == ENOTSUP);
    assert(ftruncate(fd, 7) == 0);
    assert(fsync(fd) == 0 && fchmod(fd, 0644) == 0);
    assert(fchmod(fd, 04755) == -1 && errno == ENOTSUP);
    char buffer[4] = {0};
    host = 0;
    assert(read(fd, buffer, sizeof buffer) == 3 && memcmp(buffer, "abc", 3) == 0);
    assert(write(fd, "xyz", 3) == 3 && lseek(fd, 17, SEEK_SET) == 17);
    assert(pread(fd, buffer, 3, 2) == 3);
    assert(pread(fd, buffer, 3, -1) == -1 && errno == EINVAL);
    struct stat info = {0};
    assert(fstat(fd, &info) == 0 && info.st_size == 3 && info.st_ino == 41 && S_ISREG(info.st_mode));
    host = &services;
    assert(stat("object", &info) == 0 && info.st_mtime == 123);
    assert(lstat("object", &info) == 0 && info.st_ino == 41);
    assert(access("object", R_OK | W_OK) == 0);
    assert(access("object", X_OK) == -1 && errno == EACCES);
    metadata.size = UINT64_MAX;
    assert(stat("object", &info) == -1 && errno == EOVERFLOW && info.st_size == 3);
    metadata.size = 3;
    metadata.modified_nanoseconds = 1000000000;
    assert(fstat(fd, &info) == -1 && errno == EIO && info.st_mtime == 123);
    metadata.modified_nanoseconds = 456;
    denied = EACCES;
    assert(fsync(fd) == -1 && errno == EACCES);
    assert(fchmod(fd, 0644) == -1 && errno == EACCES);
    assert(ftruncate(fd, 7) == -1 && errno == EACCES);
    assert(fstat(fd, &info) == -1 && errno == EACCES && info.st_size == 3);
    assert(read(fd, buffer, 3) == -1 && errno == EACCES);
    assert(close(fd) == -1 && errno == EACCES && closes == 1);
    assert(read(fd, buffer, 3) == -1 && errno == EBADF);
    assert(fsync(fd) == -1 && errno == EBADF);
    denied = 0; host = &services;
#ifdef O_NOFOLLOW
    int nofollow = open("object", O_RDWR | O_CREAT | O_EXCL | O_NOFOLLOW, 0600);
    assert(nofollow >= 3 && observed_mode == (39 | 128));
    assert(close(nofollow) == 0);
    denied = ENOTSUP;
    assert(open("object", O_RDONLY | O_NOFOLLOW) == -1 && errno == ENOTSUP);
    denied = 0;
#endif
    int next = open("object", O_RDONLY);
    assert(next != fd && next >= 3);
    assert(write(next, "xyz", 3) == -1 && errno == EBADF);
    assert(close(next) == 0);
    int stream = open("object", O_RDWR);
    assert(stream >= 3);
    unsigned char payload[sizeof written];
    for (size_t i = 0; i < sizeof payload; ++i) payload[i] = (unsigned char)(i * 31 + 7);
    write_fixture = 1;
    assert(write(stream, payload, sizeof payload) == (ssize_t)sizeof payload);
    assert(write_cursor == sizeof payload && memcmp(payload, written, sizeof payload) == 0);
    write_cursor = 0; write_limit = 4097;
    assert(write(stream, payload, sizeof payload) == 4097 && errno == ENOSPC);
    assert(write_cursor == 4097 && memcmp(payload, written, 4097) == 0);
    write_cursor = 0; write_limit = sizeof written; write_fault = 1;
    assert(write(stream, payload, sizeof payload) == -1 && errno == EIO && !write_cursor);
    write_fault = 2;
    assert(write(stream, payload, sizeof payload) == -1 && errno == EIO && !write_cursor);
    write_fault = 0; denied = EACCES;
    assert(write(stream, payload, sizeof payload) == -1 && errno == EACCES && !write_cursor);
    denied = 0;
    assert(write(stream, 0, 0) == 0 && !write_cursor);
    assert(close(stream) == 0);
    write_fixture = 0;
    int descriptors[128];
    for (int i = 0; i < 128; ++i) { descriptors[i] = open("object", O_RDONLY); assert(descriptors[i] >= 3); }
    assert(open("object", O_RDONLY) == -1 && errno == EMFILE);
    for (int i = 0; i < 128; ++i) assert(close(descriptors[i]) == 0);
    for (int i = 0; i < 256; ++i) { int current = open("object", O_RDONLY); assert(current >= 3); assert(close(current) == 0); }
    return 0;
}
