#include "infinity-clock.h"
#include <assert.h>
#include <stddef.h>
static uint64_t seconds;
static uint32_t nanos;
static int failure;

// ------------------------=
// FUNC: infinity_std_clock
// DESC: Supplies deterministic clock/error vectors to test the actual C/C++ adapter.
// ------------------=
int infinity_std_clock(uint32_t kind, uint64_t *s, uint32_t *n) {
    assert(kind == 0);
    *s = seconds;
    *n = nanos;
    return failure;
}

// ------------------------=
// FUNC: main
// DESC: Checks normalized conversion, largest valid value, refusal, malformed time and overflow.
// ------------------=
int main(void) {
    uint64_t output = 77;
    seconds = 3; nanos = 123;
    assert(infinity_monotonic_ns(&output) == 0 && output == 3000000123ull);
    seconds = UINT64_MAX / 1000000000u; nanos = UINT64_MAX % 1000000000u;
    assert(infinity_monotonic_ns(&output) == 0 && output == UINT64_MAX);
    output = 77; ++nanos;
    assert(infinity_monotonic_ns(&output) == 75 && output == 77);
    seconds = 1; nanos = 1000000000u;
    assert(infinity_monotonic_ns(&output) == 75 && output == 77);
    nanos = 0; failure = 95;
    assert(infinity_monotonic_ns(&output) == 95 && output == 77);
    assert(infinity_monotonic_ns(NULL) == 22);
    return 0;
}
