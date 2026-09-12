#include <infinity/compiler_host.h>
#include <dirent.h>
#include <sys/statvfs.h>
#include <sys/resource.h>
#include <sys/wait.h>
#include <sys/stat.h>
#include <sys/mman.h>
#include <assert.h>
#include <errno.h>
#include <string.h>
static int opens, closes, next_error;
static const char *next_name = "hello.c";
// ------------------------=
// FUNC: open_fixture
// DESC: Models an explicit path grant and independent cursor identities, not real storage.
// ------------------=
static int open_fixture(void *context, const char *path, uint64_t *cursor) {
    assert(context == &opens);
    if (strcmp(path, "/allowed")) return EACCES;
    *cursor = ++opens; return 0;
}
// ------------------------=
// FUNC: next_fixture
// DESC: Supplies bounded entries, malformed entries and failures for adapter tests.
// ------------------=
static int next_fixture(void *context, uint64_t cursor, char *out, size_t size) {
    assert(context == &opens && cursor > 0 && size == 96);
    if (!next_name) memset(out, 'x', size);
    else strcpy(out, next_name);
    return next_error;
}
// ------------------------=
// FUNC: close_fixture
// DESC: Verifies close reaches the original provider exactly once.
// ------------------=
static int close_fixture(void *context, uint64_t cursor) {
    assert(context == &opens && cursor > 0); ++closes; return 0;
}
// ------------------------=
// FUNC: main
// DESC: Exercises authority failures, cursor lifetime, EOF, malformed names and resource bounds.
// ------------------=
int main(void) {
    infinity_compiler_set_host(0);
    assert(!opendir("/allowed") && errno == ENOSYS);
    InfinityCompilerHost services = {.context=&opens, .directory_open=open_fixture,
        .directory_next=next_fixture, .directory_close=close_fixture};
    infinity_compiler_set_host(&services);
    assert(!opendir("/denied") && errno == EACCES && opens == 0);
    DIR *first = opendir("/allowed");
    DIR *second = opendir("/allowed");
    assert(first && second && first != second);
    struct dirent *entry = readdir(first);
    assert(entry && !strcmp(entry->d_name, "hello.c"));
    next_error = EACCES;
    assert(!readdir(first) && errno == EACCES);
    assert(!strcmp(entry->d_name, "hello.c"));
    next_error = 0; next_name = "";
    errno = EIO; assert(!readdir(first) && errno == 0);
    next_name = "../escape"; assert(!readdir(first) && errno == EIO);
    next_name = 0; assert(!readdir(first) && errno == EOVERFLOW);
    next_name = "next.c";
    infinity_compiler_set_host(0);
    assert(readdir(second)); // Existing cursor stays with its opening provider.
    assert(closedir(first) == 0 && closes == 1);
    assert(!readdir(first) && errno == EBADF);
    assert(closedir(first) == -1 && errno == EBADF && closes == 1);
    assert(closedir(second) == 0);
    infinity_compiler_set_host(&services);
    for (int i = 2; i < 64; ++i) { DIR *d = opendir("/allowed"); assert(d); assert(!closedir(d)); }
    assert(!opendir("/allowed") && errno == EMFILE && opens == 64);
    struct statvfs stats = {7, 8, 9, 10, 11};
    assert(statvfs("/allowed", &stats) == -1 && errno == ENOSYS);
    assert(stats.f_blocks == 8 && stats.f_flags == 11);
    struct rlimit limit = {17, 19};
    assert(getrlimit(RLIMIT_STACK, &limit) == -1 && errno == ENOSYS);
    assert(limit.rlim_cur == 17 && limit.rlim_max == 19);
    assert(setrlimit(RLIMIT_DATA, &limit) == -1 && errno == ENOSYS);
    int status = 37;
    assert(wait4(1, &status, 0, 0) == -1 && errno == ENOSYS && status == 37);
    assert(madvise(0, 10, MADV_DONTNEED) == -1 && errno == ENOSYS);
    return 0;
}
