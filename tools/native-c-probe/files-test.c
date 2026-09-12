#include <infinity/compiler_host.h>
#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <unistd.h>
#include <string.h>
static const InfinityCompilerHost *host;
static int denied, closes;
static uint32_t observed_mode;
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
// FUNC: main
// DESC: Tests mode translation, revocation, ownership, stale descriptors, reuse and bounded capacity.
// ------------------=
int main(void) {
    InfinityCompilerFiles files = {object_open, object_read, object_write, object_seek, object_close};
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
    denied = EACCES;
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
