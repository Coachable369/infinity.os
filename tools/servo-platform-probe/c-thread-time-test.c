#include <pthread.h>
#include <stdint.h>
#include <errno.h>
#include <time.h>
static pthread_key_t key;
static unsigned drops;
// ------------------------=
// FUNC: destroy_value
// DESC: Records actual native thread-exit cleanup of C TLS values.
// ------------------=
static void destroy_value(void *value) { if ((uintptr_t)value == 17 || (uintptr_t)value == 23) ++drops; }
// ------------------------=
// FUNC: infinity_c_tls_begin
// DESC: Allocates the shared test key from the real native provider.
// ------------------=
int infinity_c_tls_begin(void) { drops = 0; return pthread_key_create(&key, destroy_value); }
// ------------------------=
// FUNC: infinity_c_tls_worker
// DESC: Verifies isolated C TLS and errno across native scheduler sleeps, plus clock errors.
// ------------------=
int infinity_c_tls_worker(uintptr_t value) {
    if (pthread_getspecific(key) || pthread_setspecific(key, (void *)value)) return 1;
    errno = (int)value;
    struct timespec before, after, pause = {0, 1000000}, remaining = {4, 5};
    if (clock_gettime(CLOCK_MONOTONIC, &before) || nanosleep(&pause, &remaining)) return 2;
    if (errno != (int)value || (uintptr_t)pthread_getspecific(key) != value) return 3;
    if (remaining.tv_sec || remaining.tv_nsec || clock_gettime(CLOCK_MONOTONIC, &after)) return 4;
    if (after.tv_sec < before.tv_sec || (after.tv_sec == before.tv_sec && after.tv_nsec < before.tv_nsec)) return 5;
    if (clock_gettime((clockid_t)-1, &after) != -1 || errno != EINVAL) return 6;
    pause.tv_nsec = 1000000000;
    if (nanosleep(&pause, 0) != -1 || errno != EINVAL) return 7;
    return 0;
}
// ------------------------=
// FUNC: infinity_c_tls_end
// DESC: Checks destructor count, stale-key rejection and generation-safe reuse.
// ------------------=
int infinity_c_tls_end(void) {
    if (drops != 2 || pthread_key_delete(key)) return 1;
    if (pthread_key_delete(key) != EINVAL) return 2;
    pthread_key_t replacement;
    if (pthread_key_create(&replacement, 0) || replacement == key) return 3;
    return pthread_key_delete(replacement);
}
