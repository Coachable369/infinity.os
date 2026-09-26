/* Native SpiderMonkey time, backed by the same provider as Rust std. */
#include "mozilla/TimeStamp.h"
#include "infinity-clock.h"

namespace mozilla {
static uint64_t engineStart = 0;
static bool initialized = false;

// ------------------------=
// FUNC: ClockNow
// DESC: Requires the installed runtime clock before running engine static initializers.
// ------------------=
static uint64_t ClockNow() {
    uint64_t now = 0;
    MOZ_RELEASE_ASSERT(infinity_monotonic_ns(&now) == 0);
    return now;
}

// ------------------------=
// FUNC: ToSeconds
// DESC: Converts signed native nanosecond durations to seconds.
// ------------------=
double BaseTimeDurationPlatformUtils::ToSeconds(int64_t ticks) {
    return double(ticks) / 1000000000.0;
}

// ------------------------=
// FUNC: TicksFromMilliseconds
// DESC: Saturates out-of-range duration conversions just like the upstream backend.
// ------------------=
int64_t BaseTimeDurationPlatformUtils::TicksFromMilliseconds(double milliseconds) {
    double ticks = milliseconds * 1000000.0;
    if (ticks >= double(INT64_MAX)) return INT64_MAX;
    if (ticks <= double(INT64_MIN)) return INT64_MIN;
    return int64_t(ticks);
}

// ------------------------=
// FUNC: Startup
// DESC: Captures the engine lifetime epoch after the native provider has been installed.
// ------------------=
void TimeStamp::Startup() {
    if (!initialized) { engineStart = ClockNow(); initialized = true; }
}

// ------------------------=
// FUNC: Shutdown
// DESC: Releases no resources because clock ownership remains with the OS provider.
// ------------------=
void TimeStamp::Shutdown() {}

// ------------------------=
// FUNC: Now
// DESC: Returns native monotonic nanoseconds for either requested resolution.
// ------------------=
TimeStamp TimeStamp::Now(bool) { return TimeStamp(ClockNow()); }

// ------------------------=
// FUNC: ComputeProcessUptime
// DESC: Reports microseconds since engine initialization, not since OS boot or a host process.
// ------------------=
uint64_t TimeStamp::ComputeProcessUptime() {
    uint64_t now = ClockNow();
    MOZ_RELEASE_ASSERT(initialized && now >= engineStart);
    return (now - engineStart) / 1000;
}
} // namespace mozilla
