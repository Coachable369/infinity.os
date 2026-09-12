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
    /* Provider must authorize namespace access and bind the cursor to that grant.
       next returns a direct child basename or an empty name at end; errors must
       not advance the cursor. close consumes the cursor even on error. */
    int (*directory_open)(void *, const char *, uint64_t *);
    int (*directory_next)(void *, uint64_t, char *, size_t);
    int (*directory_close)(void *, uint64_t);
    /* Bind to the owning Execution Context's fatal-fault notification. Provider
       must not resume a faulted context or treat this as Unix signal authority. */
    int (*register_crash_handler)(void *, void (*)(void *), void *);
    int (*unregister_crash_handler)(void *);
} InfinityCompilerHost;
void infinity_compiler_set_host(const InfinityCompilerHost *);
const char *infinity_compiler_executable_path(void);
int infinity_compiler_register_crash_handler(void (*)(void *), void *);
int infinity_compiler_unregister_crash_handler(void);
#ifdef __cplusplus
}
#endif
#endif
