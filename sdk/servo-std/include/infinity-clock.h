#ifndef INFINITY_SERVO_CLOCK_H
#define INFINITY_SERVO_CLOCK_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
// ------------------------=
// FUNC: infinity_std_clock
// DESC: Reads a normalized clock from the governed native runtime provider.
// ------------------=
int infinity_std_clock(uint32_t, uint64_t *, uint32_t *);

// ------------------------=
// FUNC: infinity_monotonic_ns
// DESC: Converts the native monotonic clock without accepting invalid fields or overflow.
// ------------------=
static inline int infinity_monotonic_ns(uint64_t *out) {
    if (!out) return 22;
    uint64_t seconds = 0;
    uint32_t nanos = 0;
    int status = infinity_std_clock(0, &seconds, &nanos);
    if (status) return status;
    if (nanos >= 1000000000u || seconds > (UINT64_MAX - nanos) / 1000000000u) return 75;
    *out = seconds * 1000000000u + nanos;
    return 0;
}
#ifdef __cplusplus
}
#endif
#endif
