#ifndef INFINITY_COMPILER_SYSCALL_H
#define INFINITY_COMPILER_SYSCALL_H
#include <infinity/compiler_host.h>
#define INFINITY_ISOLATED_COMPILER_ABI 1u
/* This launch record contains user addresses and values only. No privileged
 * callback pointer is accepted from the loader. Service grants belong to the
 * Execution Context and are checked again by the native dispatcher. */
typedef struct InfinityIsolatedCompilerLaunch {
    uint32_t version, size;
    int32_t argc;
    uint32_t process_identity, session_identity, reserved;
    char **argv;
    void *heap_base;
    uint64_t heap_size;
    const char *executable_path, *home_path, *node_name;
} InfinityIsolatedCompilerLaunch;
enum InfinityCompilerOperation {
    INFINITY_COMPILER_CLOCK_NS = 0x1200,
    INFINITY_COMPILER_MAP = 0x1201,
    INFINITY_COMPILER_UNMAP = 0x1202,
    INFINITY_COMPILER_PROTECT = 0x1203,
    INFINITY_COMPILER_SYNC = 0x1204,
    INFINITY_COMPILER_ENTROPY = 0x1205,
    INFINITY_COMPILER_DIRECTORY_OPEN = 0x1206,
    INFINITY_COMPILER_DIRECTORY_NEXT = 0x1207,
    INFINITY_COMPILER_DIRECTORY_CLOSE = 0x1208,
    INFINITY_COMPILER_SLEEP_NS = 0x1209,
    INFINITY_COMPILER_PROCESS_ALIVE = 0x120a,
    INFINITY_COMPILER_FILE_OPEN = 0x120b,
    INFINITY_COMPILER_FILE_READ = 0x120c,
    INFINITY_COMPILER_FILE_WRITE = 0x120d,
    INFINITY_COMPILER_FILE_SEEK = 0x120e,
    INFINITY_COMPILER_FILE_CLOSE = 0x120f,
    INFINITY_COMPILER_FILE_READ_AT = 0x1210,
    INFINITY_COMPILER_FILE_INSPECT = 0x1211,
    INFINITY_COMPILER_FILE_INSPECT_PATH = 0x1212,
    INFINITY_COMPILER_FILE_TRUNCATE = 0x1213,
    INFINITY_COMPILER_NAMESPACE_CURRENT = 0x1214,
    INFINITY_COMPILER_NAMESPACE_RESOLVE = 0x1215,
    INFINITY_COMPILER_NAMESPACE_CHANGE = 0x1216,
    INFINITY_COMPILER_NAMESPACE_CREATE = 0x1217,
    INFINITY_COMPILER_NAMESPACE_REMOVE = 0x1218,
    INFINITY_COMPILER_NAMESPACE_LINK = 0x1219,
    INFINITY_COMPILER_NAMESPACE_READ_LINK = 0x121a,
    INFINITY_COMPILER_CONSOLE_READ = 0x121b,
    INFINITY_COMPILER_CONSOLE_WRITE = 0x121c,
    INFINITY_COMPILER_CONSOLE_IS_TERMINAL = 0x121d,
};
int infinity_compiler_isolated_host(const InfinityIsolatedCompilerLaunch *, InfinityCompilerHost *);
#endif

