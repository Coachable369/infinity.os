#include <infinity/compiler_host.h>
#include <assert.h>
#include <errno.h>
#include <string.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/stat.h>
static const InfinityCompilerHost *host;
static int failure, changes, malformed, links;
// ------------------------=
// FUNC: infinity_compiler_get_host
// DESC: Supplies a namespace fixture without host filesystem access.
// ------------------=
const InfinityCompilerHost *infinity_compiler_get_host(void) { return host; }
// ------------------------=
// FUNC: current
// DESC: Returns a controlled native reference.
// ------------------=
static int current(void *context, char *out, size_t size, size_t *length) {
    (void)context; assert(size >= 5); memcpy(out, "/work", 5); *length = 5;
    if (malformed == 1) *length = 96;
    if (malformed == 2) out[0] = 'x';
    if (malformed == 3) out[2] = 0;
    return failure;
}
// ------------------------=
// FUNC: resolve
// DESC: Resolves a fixture reference without parsing shell output.
// ------------------=
static int resolve(void *context, const char *path, char *out, size_t size, size_t *length) {
    assert(path && *path); return current(context, out, size, length);
}
// ------------------------=
// FUNC: change
// DESC: Records only successful namespace mutations.
// ------------------=
static int change(void *context, const char *path) {
    (void)context; assert(path && *path); if (!failure) ++changes; return failure;
}
// ------------------------=
// FUNC: alias
// DESC: Records exact hard and symbolic reference requests from the compatibility bridge.
// ------------------=
static int alias(void *context, const char *existing, const char *created, uint32_t symbolic) {
    (void)context; assert(!strcmp(existing, "source") && !strcmp(created, "created"));
    if (!failure) links += symbolic ? 2 : 1;
    return failure;
}
// ------------------------=
// FUNC: read_alias
// DESC: Returns a bounded native alias target without a compatibility-layer terminator.
// ------------------=
static int read_alias(void *context, const char *path, char *out, size_t size, size_t *length) {
    (void)context; assert(!strcmp(path, "created") && size >= 6);
    memcpy(out, "source", 6); *length = malformed == 4 ? size + 1 : 6;
    return failure;
}
// ------------------------=
// FUNC: main
// DESC: Tests atomic path output, buffer bounds, allocation and mutation denial.
// ------------------=
int main(void) {
    char out[96] = {42};
    assert(!getcwd(out, sizeof out) && errno == ENOSYS && out[0] == 42);
    InfinityCompilerNamespace api = {current, resolve, change, change, change, alias, read_alias};
    InfinityCompilerHost service = {.namespaces = &api}; host = &service;
    assert(!getcwd(out, 5) && errno == ERANGE && out[0] == 42);
    assert(getcwd(out, sizeof out) == out && memcmp(out, "/work", 6) == 0);
    char *allocated = realpath(".", 0); assert(allocated && memcmp(allocated, out, 6) == 0); free(allocated);
    assert(chdir("work") == 0 && mkdir("work", 0700) == 0 && unlink("work") == 0 && changes == 3);
    assert(link("source", "created") == 0 && symlink("source", "created") == 0 && links == 3);
    memset(out, 42, sizeof out);
    assert(readlink("created", out, sizeof out) == 6 && !memcmp(out, "source", 6) && out[6] == 42);
    failure = EACCES; out[0] = 42;
    assert(!realpath(".", out) && errno == EACCES && out[0] == 42);
    assert(unlink("work") == -1 && errno == EACCES && changes == 3);
    assert(mkdir("work", 04777) == -1 && errno == ENOTSUP && changes == 3);
    failure = 0;
    for (malformed = 1; malformed <= 3; ++malformed) {
        out[0] = 42;
        assert(!getcwd(out, sizeof out) && errno == EIO && out[0] == 42);
    }
    malformed = 4;
    assert(readlink("created", out, sizeof out) == -1 && errno == EIO);
    assert(!realpath(0, out) && errno == EINVAL);
    assert(link(0, "created") == -1 && errno == EINVAL);
    assert(readlink("", out, sizeof out) == -1 && errno == EINVAL);
    assert(chdir("") == -1 && errno == EINVAL && changes == 3);
    return 0;
}
