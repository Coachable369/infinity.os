#include <infinity/compiler_host.h>
#include <errno.h>
#include <assert.h>
#include <stdint.h>
#include <stddef.h>
extern void *infinity_test_sbrk(ptrdiff_t);
static const InfinityCompilerHost *host;
// ------------------------=
// FUNC: infinity_compiler_get_host
// DESC: Supplies only the fixture's granted region.
// ------------------=
const InfinityCompilerHost *infinity_compiler_get_host(void) { return host; }
// ------------------------=
// FUNC: main
// DESC: Tests exact break movement, capacity, shrink underflow and provider replacement rejection.
// ------------------=
int main(void) {
    _Alignas(16) unsigned char region[128];
    InfinityCompilerHost service = {.heap_base = region, .heap_size = sizeof region};
    assert(infinity_test_sbrk(0) == (void *)-1 && errno == ENOSYS);
    host = &service;
    assert(infinity_test_sbrk(32) == region);
    assert(infinity_test_sbrk(0) == region + 32);
    assert(infinity_test_sbrk(97) == (void *)-1 && errno == ENOMEM);
    assert(infinity_test_sbrk(0) == region + 32);
    assert(infinity_test_sbrk(PTRDIFF_MIN) == (void *)-1 && errno == EINVAL);
    assert(infinity_test_sbrk(-16) == region + 32);
    assert(infinity_test_sbrk(112) == region + 16);
    assert(infinity_test_sbrk(1) == (void *)-1 && errno == ENOMEM);
    service.heap_size--;
    assert(infinity_test_sbrk(0) == (void *)-1 && errno == EBUSY);
    return 0;
}
