#include <infinity/compiler_host.h>
#include <time.h>
#include <sys/time.h>
#include <unistd.h>
#include <errno.h>
#include <limits.h>

// ------------------------=
// FUNC: time_error
// DESC: Preserves a native timing error through the C API.
// ------------------=
static int time_error(int error) { errno = error; return -1; }

// ------------------------=
// FUNC: gettimeofday
// DESC: Converts observed wall-clock time without inventing a Unix timezone.
// ------------------=
int gettimeofday(struct timeval *out, void *zone) {
    if (!out) return time_error(EFAULT);
    if (zone) return time_error(ENOTSUP);
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->clock_ns) return time_error(ENOSYS);
    uint64_t ns = 0;
    int error = host->clock_ns(host->context, CLOCK_REALTIME, &ns);
    if (error) return time_error(error);
    struct timeval result = {0};
    result.tv_sec = (time_t)(ns / 1000000000u);
    result.tv_usec = (suseconds_t)((ns % 1000000000u) / 1000);
    if ((uint64_t)result.tv_sec != ns / 1000000000u) return time_error(EOVERFLOW);
    *out = result;
    return 0;
}

// ------------------------=
// FUNC: nanosleep
// DESC: Delegates bounded waits and reports only measured interruption remainders.
// ------------------=
int nanosleep(const struct timespec *request, struct timespec *remaining) {
    if (!request) return time_error(EFAULT);
    if (request->tv_sec < 0 || request->tv_nsec < 0 || request->tv_nsec >= 1000000000)
        return time_error(EINVAL);
    if ((uint64_t)request->tv_sec > (UINT64_MAX - (uint64_t)request->tv_nsec) / 1000000000u)
        return time_error(EOVERFLOW);
    uint64_t ns = (uint64_t)request->tv_sec * 1000000000u + (uint64_t)request->tv_nsec;
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->sleep_ns) return time_error(ENOSYS);
    uint64_t left = 0;
    int error = host->sleep_ns(host->context, ns, &left);
    if (error == EINTR && remaining) {
        if (left > ns) return time_error(EIO);
        struct timespec result = {(time_t)(left / 1000000000u), (long)(left % 1000000000u)};
        *remaining = result;
    }
    return error ? time_error(error) : 0;
}

// ------------------------=
// FUNC: usleep
// DESC: Converts microseconds to the native wait boundary.
// ------------------=
int usleep(useconds_t delay) {
    struct timespec request = {(time_t)(delay / 1000000u), (long)(delay % 1000000u) * 1000};
    return nanosleep(&request, 0);
}

// ------------------------=
// FUNC: getpagesize
// DESC: Returns the launcher's actual memory granularity, rejecting invalid measurements.
// ------------------=
int getpagesize(void) {
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    if (!host || !host->page_size) return time_error(ENOSYS);
    uint32_t size = host->page_size;
    if (size > INT_MAX || (size & (size - 1))) return time_error(EINVAL);
    return (int)size;
}
