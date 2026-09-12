#include <infinity/compiler_host.h>
#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <unistd.h>
#include <string.h>
#include <sys/stat.h>
static const InfinityCompilerHost *host;
static int denied, closes;
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
// FUNC: main
// DESC: Tests mode translation, revocation, ownership, stale descriptors, reuse and bounded capacity.
// ------------------=
int main(void) {
    InfinityCompilerFiles files = {object_open, object_read, object_write, object_seek, object_close,
                                  object_read_at, object_inspect, path_inspect};
    InfinityCompilerHost services = {.context = &denied, .files = &files};
    assert(open("object", O_RDONLY) == -1 && errno == ENOSYS);
    host = &services;
    assert(open("object", O_RDONLY | O_TRUNC) == -1 && errno == EINVAL);
    int fd = open("object", O_RDWR | O_CREAT | O_EXCL, 0600);
    assert(fd >= 3 && observed_mode == 39);
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
    assert(access("object", R_OK | W_OK) == 0);
    assert(access("object", X_OK) == -1 && errno == EACCES);
    metadata.size = UINT64_MAX;
    assert(stat("object", &info) == -1 && errno == EOVERFLOW && info.st_size == 3);
    metadata.size = 3;
    metadata.modified_nanoseconds = 1000000000;
    assert(fstat(fd, &info) == -1 && errno == EIO && info.st_mtime == 123);
    metadata.modified_nanoseconds = 456;
    denied = EACCES;
    assert(fstat(fd, &info) == -1 && errno == EACCES && info.st_size == 3);
    assert(read(fd, buffer, 3) == -1 && errno == EACCES);
    assert(close(fd) == -1 && errno == EACCES && closes == 1);
    assert(read(fd, buffer, 3) == -1 && errno == EBADF);
    denied = 0; host = &services;
    int next = open("object", O_RDONLY);
    assert(next != fd && next >= 3);
    assert(write(next, "xyz", 3) == -1 && errno == EBADF);
    assert(close(next) == 0);
    int descriptors[128];
    for (int i = 0; i < 128; ++i) { descriptors[i] = open("object", O_RDONLY); assert(descriptors[i] >= 3); }
    assert(open("object", O_RDONLY) == -1 && errno == EMFILE);
    for (int i = 0; i < 128; ++i) assert(close(descriptors[i]) == 0);
    for (int i = 0; i < 256; ++i) { int current = open("object", O_RDONLY); assert(current >= 3); assert(close(current) == 0); }
    return 0;
}
