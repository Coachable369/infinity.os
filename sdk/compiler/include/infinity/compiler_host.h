#ifndef INFINITY_COMPILER_HOST_H
#define INFINITY_COMPILER_HOST_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef struct InfinityCompilerMetadata {
    uint64_t identity;
    uint64_t size;
    int64_t modified_seconds;
    uint32_t modified_nanoseconds;
    uint32_t kind; /* 1=content object, 2=namespace */
    uint32_t access; /* effective capability rights: read=1, write=2, execute=4 */
} InfinityCompilerMetadata;
typedef struct InfinityCompilerFiles {
    /* Native mode bits: read=1, write=2, create=4, truncate=8, append=16,
       exclusive=32. Provider must authorize each operation, including revocation.
       Reads use an opened ObjectStore snapshot; writes commit native versions. */
    int (*open)(void *, const char *, uint32_t, uint64_t *);
    int (*read)(void *, uint64_t, void *, size_t, size_t *);
    int (*write)(void *, uint64_t, const void *, size_t, size_t *);
    int (*seek)(void *, uint64_t, int64_t, int, int64_t *);
    int (*close)(void *, uint64_t);
    /* Positional reads must not alter the opened cursor, including on failure. */
    int (*read_at)(void *, uint64_t, uint64_t, void *, size_t, size_t *);
    int (*inspect)(void *, uint64_t, InfinityCompilerMetadata *);
    int (*inspect_path)(void *, const char *, InfinityCompilerMetadata *);
    int (*truncate)(void *, uint64_t, uint64_t);
} InfinityCompilerFiles;
typedef struct InfinityCompilerNamespace {
    /* Output paths are canonical native references, at most 95 bytes, without
       a NUL terminator. Every request is capability checked by the provider. */
    int (*current)(void *, char *, size_t, size_t *);
    int (*resolve)(void *, const char *, char *, size_t, size_t *);
    int (*change)(void *, const char *);
    int (*create)(void *, const char *);
    int (*remove)(void *, const char *);
    /* Native references are aliases to ObjectStore identities. Symbolic
       aliases may be rejected when the namespace does not support them. */
    int (*link)(void *, const char *, const char *, uint32_t);
    int (*read_link)(void *, const char *, char *, size_t, size_t *);
} InfinityCompilerNamespace;
typedef struct InfinityCompilerConsole {
    /* Stream numbers are the launch-bound standard streams 0, 1 and 2. */
    int (*read)(void *, uint32_t, void *, size_t, size_t *);
    int (*write)(void *, uint32_t, const void *, size_t, size_t *);
    int (*is_terminal)(void *, uint32_t, int *);
} InfinityCompilerConsole;
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
    const InfinityCompilerFiles *files;
    /* Launcher-owned, writable, committed private region; never an address to
       allocate or map speculatively. Stable for the lifetime of this image. */
    void *heap_base;
    size_t heap_size;
    uint32_t page_size;
    /* Relative monotonic wait; EINTR returns measured remaining nanoseconds. */
    int (*sleep_ns)(void *, uint64_t, uint64_t *);
    /* Must be explicitly guaranteed by the launcher; serial synchronization
       is not safe for multiple threads entering the same compiler image. */
    uint32_t serial_execution;
    const char *home_path;
    const InfinityCompilerNamespace *namespaces;
    const InfinityCompilerConsole *console;
    /* Stable, nonzero identifiers scoped to this native Execution Context. */
    uint32_t process_identity;
    uint32_t session_identity;
    const char *node_name;
    /* Query another context without granting control over it. */
    int (*process_alive)(void *, uint32_t, int *);
    /* A successful implementation does not return. */
    int (*terminate)(void *, int);
} InfinityCompilerHost;
void infinity_compiler_set_host(const InfinityCompilerHost *);
const InfinityCompilerHost *infinity_compiler_get_host(void);
const char *infinity_compiler_executable_path(void);
const char *infinity_compiler_home_path(void);
int infinity_compiler_register_crash_handler(void (*)(void *), void *);
int infinity_compiler_unregister_crash_handler(void);
#ifdef __cplusplus
}
#endif
#endif
