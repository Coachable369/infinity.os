#include <infinity/compiler_host.h>
#include <stdint.h>
#include <errno.h>
#ifdef __INFINITY__
#include <unistd.h>
#endif

static unsigned char *base;
static size_t capacity, used;

// ------------------------=
// FUNC: sbrk
// DESC: Adjusts Newlib's break within an explicitly granted private memory region.
// ------------------=
void *sbrk(ptrdiff_t increment) {
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->heap_base || !host->heap_size) { errno = ENOSYS; return (void *)-1; }
    if (!base) {
        uintptr_t address = (uintptr_t)host->heap_base;
        if (address % 16 || host->heap_size > UINTPTR_MAX - address) { errno = EINVAL; return (void *)-1; }
        base = host->heap_base; capacity = host->heap_size;
    }
    if (host->heap_base != base || host->heap_size != capacity) { errno = EBUSY; return (void *)-1; }
    size_t next;
    if (increment >= 0) {
        size_t growth = (size_t)increment;
        if (growth > capacity - used) { errno = ENOMEM; return (void *)-1; }
        next = used + growth;
    } else {
        size_t shrink = (size_t)(-(increment + 1)) + 1;
        if (shrink > used) { errno = EINVAL; return (void *)-1; }
        next = used - shrink;
    }
    void *previous = base + used;
    used = next;
    return previous;
}
