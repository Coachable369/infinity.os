#include <pthread.h>
#include <stdint.h>
#include <time.h>
#include <errno.h>
#include <stdlib.h>
#include "include/infinity-error.h"
extern int infinity_std_tls_create(void (*)(void *), size_t *);
extern int infinity_c_tls_destroy(size_t);
extern void *infinity_std_tls_get(size_t);
extern int infinity_std_tls_set(size_t, void *);
extern int *infinity_c_errno_location(void);
extern int infinity_std_clock(uint32_t, uint64_t *, uint32_t *);
extern int infinity_std_sleep(uint64_t);

// ------------------------=
// FUNC: __errno
// DESC: Uses per-thread C errno storage separate from Rust's stable error encoding; initialization is mandatory.
// ------------------=
int *__errno(void) {
    int *value = infinity_c_errno_location();
    if (!value) abort();
    return value;
}
// ------------------------=
// FUNC: pthread_key_create
// DESC: Creates a real native TLS key without narrowing a generation silently.
// ------------------=
int pthread_key_create(pthread_key_t *key, void (*destructor)(void *)) {
    if (!key) return EINVAL;
    size_t native;
    int status = infinity_std_tls_create(destructor, &native);
    if (status) return infinity_native_error(status);
    if ((size_t)(pthread_key_t)native != native) { infinity_c_tls_destroy(native); return EAGAIN; }
    *key = (pthread_key_t)native;
    return 0;
}
// ------------------------=
// FUNC: pthread_key_delete
// DESC: Revokes a generation-tagged key without calling destructors on other threads.
// ------------------=
int pthread_key_delete(pthread_key_t key) { return infinity_native_error(infinity_c_tls_destroy(key)); }
// ------------------------=
// FUNC: pthread_getspecific
// DESC: Reads the current native stack's TLS value.
// ------------------=
void *pthread_getspecific(pthread_key_t key) { return infinity_std_tls_get(key); }
// ------------------------=
// FUNC: pthread_setspecific
// DESC: Associates data with only the current stack and a valid key generation.
// ------------------=
int pthread_setspecific(pthread_key_t key, const void *value) { return infinity_native_error(infinity_std_tls_set(key, (void *)value)); }
// ------------------------=
// FUNC: clock_gettime
// DESC: Converts actual native monotonic/UTC clocks and rejects unsupported clocks or time overflow.
// ------------------=
int clock_gettime(clockid_t clock, struct timespec *out) {
    if (!out || (clock != CLOCK_MONOTONIC && clock != CLOCK_REALTIME)) { errno = EINVAL; return -1; }
    uint64_t seconds; uint32_t nanos;
    int status = infinity_std_clock(clock == CLOCK_MONOTONIC ? 0 : 1, &seconds, &nanos);
    if (status) { errno = infinity_native_error(status); return -1; }
    time_t converted = (time_t)seconds;
    if (converted < 0 || (uint64_t)converted != seconds) { errno = EOVERFLOW; return -1; }
    out->tv_sec = converted; out->tv_nsec = nanos;
    return 0;
}
// ------------------------=
// FUNC: nanosleep
// DESC: Sleeps through the native scheduler rather than blocking the desktop CPU in a host call.
// ------------------=
int nanosleep(const struct timespec *duration, struct timespec *remaining) {
    if (!duration || duration->tv_sec < 0 || duration->tv_nsec < 0 || duration->tv_nsec >= 1000000000L) {
        errno = EINVAL; return -1;
    }
    uint64_t seconds = (uint64_t)duration->tv_sec;
    if (seconds > (UINT64_MAX - duration->tv_nsec) / 1000000000ull) { errno = EOVERFLOW; return -1; }
    int status = infinity_std_sleep(seconds * 1000000000ull + duration->tv_nsec);
    if (status) { errno = infinity_native_error(status); return -1; }
    if (remaining) { remaining->tv_sec = 0; remaining->tv_nsec = 0; }
    return 0;
}
