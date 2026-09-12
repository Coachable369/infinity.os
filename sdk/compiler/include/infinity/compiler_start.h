#ifndef INFINITY_COMPILER_START_H
#define INFINITY_COMPILER_START_H
#include <infinity/compiler_host.h>
#define INFINITY_COMPILER_LAUNCH_ABI 1u
typedef struct InfinityCompilerLaunch {
    uint32_t version;
    uint32_t size;
    const InfinityCompilerHost *host;
    int argc;
    char **argv;
} InfinityCompilerLaunch;
typedef void (*InfinityInitializer)(void);
int infinity_compiler_run(const InfinityCompilerLaunch *, int (*)(int, char **),
                         const InfinityInitializer *, size_t,
                         const InfinityInitializer *, size_t);
#endif
