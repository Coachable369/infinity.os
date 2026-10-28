/* Focused provider-path regression, not proof of installed SQLite durability. */
#include <assert.h>
#include <limits.h>
#define sqlite3_os_init infinity_sqlite_os_init
#define sqlite3_os_end infinity_sqlite_os_end
#include "../../sdk/python/sqlite.c"

// ------------------------=
// FUNC: infinity_compiler_get_host
// DESC: Keeps this path-only fixture from obtaining runtime clock or entropy authority.
// ------------------=
const InfinityCompilerHost *infinity_compiler_get_host(void) { return NULL; }

// ------------------------=
// FUNC: main
// DESC: Exercises actual existing-parent resolution, absent leaves, aliases, failure atomicity and bounded output under repository-local scratch.
// ------------------=
int main(void) {
    char scratch[] = "build/tmp/sqlite-path-XXXXXX";
    assert(mkdtemp(scratch));
    assert(chdir(scratch) == 0);
    char parent[PATH_MAX];
    assert(getcwd(parent, sizeof(parent)));
    assert(strlen(parent) + 20 < 256);
    char path[256], alias[256], rejected[256];
    assert(native_path(NULL, "new.db", sizeof(path), path) == SQLITE_OK);
    assert(strncmp(path, parent, strlen(parent)) == 0);
    assert(access("new.db", F_OK) == -1 && errno == ENOENT);
    assert(native_path(NULL, "./new.db", sizeof(alias), alias) == SQLITE_OK);
    assert(strcmp(path, alias) == 0);
    assert(mkdir("directory", 0700) == 0);
    assert(native_path(NULL, "directory/../new.db", sizeof(alias), alias) == SQLITE_OK);
    assert(strcmp(path, alias) == 0);
    memset(rejected, 0x5a, sizeof(rejected));
    assert(native_path(NULL, "missing/new.db", sizeof(rejected), rejected) == SQLITE_CANTOPEN);
    assert(native_path(NULL, "directory/", sizeof(rejected), rejected) == SQLITE_OK);
    memset(rejected, 0x5a, sizeof(rejected));
    assert(native_path(NULL, "new.db", 3, rejected) == SQLITE_CANTOPEN);
    assert(native_path(NULL, "new.db", 0, rejected) == SQLITE_CANTOPEN);
    assert(native_path(NULL, "", sizeof(rejected), rejected) == SQLITE_CANTOPEN);
    for (size_t index = 0; index < sizeof(rejected); ++index) assert(rejected[index] == 0x5a);
    int fd = open("new.db", O_WRONLY | O_CREAT | O_EXCL, 0600);
    assert(fd >= 0 && close(fd) == 0);
    assert(native_path(NULL, "new.db", sizeof(alias), alias) == SQLITE_OK);
    assert(strcmp(path, alias) == 0);
    assert(native_path(NULL, "new.db/child", sizeof(rejected), rejected) == SQLITE_CANTOPEN);
    assert(unlink("new.db") == 0 && rmdir("directory") == 0);
    assert(chdir("..") == 0);
    assert(rmdir(strrchr(scratch, '/') + 1) == 0);
    return 0;
}
