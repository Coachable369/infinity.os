#include <stdint.h>
#include <stddef.h>

static uint64_t ticks[128];
static uint64_t calls[128];

// ------------------------=
// FUNC: native_profile_clock
// DESC: Reads the guest architectural timer without a host timing service.
// ------------------=
uint64_t native_profile_clock(void) {
#if defined(__aarch64__)
    uint64_t value;
    __asm__ volatile("mrs %0,cntvct_el0" : "=r"(value));
    return value;
#else
    unsigned low,high;
    __asm__ volatile("lfence; rdtsc" : "=a"(low), "=d"(high) :: "memory");
    return ((uint64_t)high<<32)|low;
#endif
}

// ------------------------=
// FUNC: native_profile_record
// DESC: Accumulates operation timing only on the single owning inference worker in diagnostic builds.
// ------------------=
void native_profile_record(unsigned operation, uint64_t start) {
    if (operation >= 128) return;
    ticks[operation] += native_profile_clock() - start;
    calls[operation]++;
}

// ------------------------=
// FUNC: native_profile_read
// DESC: Exports numeric operation counters, with zero values when diagnostic instrumentation is disabled.
// ------------------=
void native_profile_read(uint64_t *out) {
    for (size_t i = 0; i < 128; ++i) { out[i * 2] = ticks[i]; out[i * 2 + 1] = calls[i]; }
}
