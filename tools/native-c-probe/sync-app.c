#include <pthread.h>
#include <errno.h>
#include <infinity/compiler_host.h>
#include <infinity/serial_tls.h>
static InfinityCompilerHost host;
static int error_value, destroyed;
// ------------------------=
// FUNC: __errno
// DESC: Supplies the trusted single-thread fixture's error cell.
// ------------------=
int *__errno(void) { return &error_value; }
// ------------------------=
// FUNC: infinity_compiler_get_host
// DESC: Supplies the actual single-thread QEMU fixture launch guarantee.
// ------------------=
const InfinityCompilerHost *infinity_compiler_get_host(void) { return &host; }
// ------------------------=
// FUNC: memset
// DESC: Implements fixture memory initialization without a host library dependency.
// ------------------=
void *memset(void *out, int value, size_t length) {
    unsigned char *bytes = out;
    for (size_t i = 0; i < length; ++i) bytes[i] = (unsigned char)value;
    return out;
}
// ------------------------=
// FUNC: destroy_value
// DESC: Observes native key destruction through the target ABI.
// ------------------=
static void destroy_value(void *value) { if (value == &error_value) ++destroyed; }
// ------------------------=
// FUNC: infinity_app_entry
// DESC: Exercises the actual Newlib pthread wrappers under the native ELF loader.
// ------------------=
int infinity_app_entry(const void *unused) {
    (void)unused;
    pthread_mutex_t mutex = PTHREAD_MUTEX_INITIALIZER;
    if (pthread_mutex_lock(&mutex) != ENOTSUP) return 1;
    host.serial_execution = 1;
    if (pthread_mutex_lock(&mutex) || pthread_mutex_trylock(&mutex) != EBUSY ||
        pthread_mutex_lock(&mutex) != EDEADLK || pthread_mutex_destroy(&mutex) != EBUSY) return 2;
    if (pthread_mutex_unlock(&mutex) || pthread_mutex_destroy(&mutex)) return 3;
    pthread_mutexattr_t attr;
    if (pthread_mutexattr_init(&attr) || pthread_mutexattr_settype(&attr, PTHREAD_MUTEX_RECURSIVE) ||
        pthread_mutex_init(&mutex, &attr) || pthread_mutex_lock(&mutex) || pthread_mutex_lock(&mutex) ||
        pthread_mutex_unlock(&mutex) || pthread_mutex_unlock(&mutex) || pthread_mutex_destroy(&mutex)) return 4;
    pthread_key_t key;
    if (pthread_key_create(&key, destroy_value) || pthread_setspecific(key, &error_value) ||
        pthread_getspecific(key) != &error_value) return 5;
    infinity_thread_cleanup();
    if (destroyed != 1 || pthread_getspecific(key)) return 6;
    pthread_cond_t cond = PTHREAD_COND_INITIALIZER;
    mutex = PTHREAD_MUTEX_INITIALIZER;
    if (pthread_mutex_lock(&mutex) || pthread_cond_signal(&cond) || pthread_cond_broadcast(&cond) ||
        pthread_cond_wait(&cond, &mutex) != ENOTSUP || pthread_mutex_unlock(&mutex) ||
        pthread_cond_destroy(&cond) || pthread_cond_signal(&cond) != EINVAL) return 7;
    pthread_t thread = 42;
    if (pthread_create(&thread, 0, 0, 0) != ENOTSUP || thread != 42 || pthread_self() != 1 ||
        pthread_join(1, 0) != EDEADLK || pthread_detach(1) != ENOTSUP) return 8;
    return 0;
}
