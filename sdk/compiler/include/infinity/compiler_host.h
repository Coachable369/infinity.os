#ifndef INFINITY_COMPILER_HOST_H
#define INFINITY_COMPILER_HOST_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef struct InfinityCompilerHost {
    void *context;
    const char *executable_path;
    /* Return zero on success or a positive errno on failure. */
    int (*clock_ns)(void *, int, uint64_t *);
    int (*map)(void *, void **, size_t, int, int, int, int64_t);
    int (*unmap)(void *, void *, size_t);
    int (*protect)(void *, void *, size_t, int);
    int (*sync)(void *, void *, size_t, int);
    /* Fill the entire request with native entropy or return a positive errno. */
    int (*entropy)(void *, void *, size_t);
} InfinityCompilerHost;
void infinity_compiler_set_host(const InfinityCompilerHost *);
const char *infinity_compiler_executable_path(void);
#ifdef __cplusplus
}
#endif
#endif
