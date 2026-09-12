#include <pthread.h>
#include <infinity/compiler_host.h>
#include <infinity/serial_sync.h>
#include <infinity/serial_tls.h>
#include <errno.h>

// ------------------------=
// FUNC: serial
// DESC: Rejects use unless the launcher guarantees a single execution thread.
// ------------------=
static int serial(void) {
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    return host && host->serial_execution == 1;
}
// ------------------------=
// FUNC: pthread_mutexattr_init
// DESC: Initializes supported private mutex attributes.
// ------------------=
int pthread_mutexattr_init(pthread_mutexattr_t *attr) {
    if (!attr) return EINVAL;
    *attr = (pthread_mutexattr_t){0}; attr->is_initialized = 1;
    return 0;
}
// ------------------------=
// FUNC: pthread_mutexattr_destroy
// DESC: Invalidates a mutex attribute record.
// ------------------=
int pthread_mutexattr_destroy(pthread_mutexattr_t *attr) {
    if (!attr || !attr->is_initialized) return EINVAL;
    attr->is_initialized = 0; return 0;
}
// ------------------------=
// FUNC: pthread_mutexattr_settype
// DESC: Selects supported serial mutex recursion semantics.
// ------------------=
int pthread_mutexattr_settype(pthread_mutexattr_t *attr, int type) {
    if (!attr || !attr->is_initialized || type < PTHREAD_MUTEX_NORMAL || type > PTHREAD_MUTEX_DEFAULT) return EINVAL;
    attr->type = type; attr->recursive = type == PTHREAD_MUTEX_RECURSIVE; return 0;
}
// ------------------------=
// FUNC: pthread_mutex_init
// DESC: Creates a serial mutex after validating launcher authority and attributes.
// ------------------=
int pthread_mutex_init(pthread_mutex_t *mutex, const pthread_mutexattr_t *attr) {
    if (!serial()) return ENOTSUP;
    if (attr && !attr->is_initialized) return EINVAL;
    return infinity_mutex_init(mutex, attr ? attr->recursive : 0);
}
// ------------------------=
// FUNC: pthread_mutex_lock
// DESC: Acquires the native serial lock without silently accepting self-deadlock.
// ------------------=
int pthread_mutex_lock(pthread_mutex_t *mutex) {
    return serial() ? infinity_mutex_lock(mutex, 0) : ENOTSUP;
}
// ------------------------=
// FUNC: pthread_mutex_trylock
// DESC: Reports busy ownership without waiting.
// ------------------=
int pthread_mutex_trylock(pthread_mutex_t *mutex) {
    return serial() ? infinity_mutex_lock(mutex, 1) : ENOTSUP;
}
// ------------------------=
// FUNC: pthread_mutex_unlock
// DESC: Releases serial ownership with state validation.
// ------------------=
int pthread_mutex_unlock(pthread_mutex_t *mutex) {
    return serial() ? infinity_mutex_unlock(mutex) : ENOTSUP;
}
// ------------------------=
// FUNC: pthread_mutex_destroy
// DESC: Destroys only an idle serial mutex.
// ------------------=
int pthread_mutex_destroy(pthread_mutex_t *mutex) {
    return serial() ? infinity_mutex_destroy(mutex) : ENOTSUP;
}
// ------------------------=
// FUNC: pthread_create
// DESC: Rejects additional threads in the explicit single-thread compiler runtime.
// ------------------=
int pthread_create(pthread_t *thread, const pthread_attr_t *attr, void *(*entry)(void *), void *argument) {
    (void)thread; (void)attr; (void)entry; (void)argument; return ENOTSUP;
}
// ------------------------=
// FUNC: pthread_self
// DESC: Identifies the sole invocation thread only in an authorized serial image.
// ------------------=
pthread_t pthread_self(void) { return serial() ? 1 : 0; }
// ------------------------=
// FUNC: pthread_join
// DESC: Rejects joining self or a nonexistent additional thread.
// ------------------=
int pthread_join(pthread_t thread, void **result) {
    (void)result; return serial() && thread == 1 ? EDEADLK : ESRCH;
}
// ------------------------=
// FUNC: pthread_detach
// DESC: Rejects detach since this runtime owns no independently created threads.
// ------------------=
int pthread_detach(pthread_t thread) { (void)thread; return ENOTSUP; }
// ------------------------=
// FUNC: pthread_key_create
// DESC: Allocates a thread-local key only under the serial execution guarantee.
// ------------------=
int pthread_key_create(pthread_key_t *key, void (*destroy)(void *)) {
    return serial() ? infinity_key_create(key, destroy) : ENOTSUP;
}
// ------------------------=
// FUNC: pthread_setspecific
// DESC: Stores an invocation-local value under a validated key.
// ------------------=
int pthread_setspecific(pthread_key_t key, const void *value) {
    return serial() ? infinity_key_set(key, value) : ENOTSUP;
}
// ------------------------=
// FUNC: pthread_getspecific
// DESC: Returns only this serial invocation's key value.
// ------------------=
void *pthread_getspecific(pthread_key_t key) { return serial() ? infinity_key_get(key) : 0; }
// ------------------------=
// FUNC: __cxa_thread_atexit
// DESC: Registers cleanup for a static compiler image; dynamic module unload is unsupported.
// ------------------=
int __cxa_thread_atexit(void (*destroy)(void *), void *value, void *dso) {
    (void)dso;
    int error = serial() ? infinity_thread_destructor(destroy, value) : ENOTSUP;
    if (error) { errno = error; return -1; }
    return 0;
}
// ------------------------=
// FUNC: condition
// DESC: Initializes a static serial condition or rejects a destroyed condition.
// ------------------=
static int condition(pthread_cond_t *cond) {
    if (!serial()) return ENOTSUP;
    if (!cond) return EINVAL;
    if (*cond == PTHREAD_COND_INITIALIZER) *cond = 1;
    return *cond ? 0 : EINVAL;
}
// ------------------------=
// FUNC: pthread_cond_destroy
// DESC: Invalidates a condition with no possible concurrent waiter in serial mode.
// ------------------=
int pthread_cond_destroy(pthread_cond_t *cond) {
    int error = condition(cond);
    if (!error) *cond = 0;
    return error;
}
// ------------------------=
// FUNC: pthread_cond_signal
// DESC: Records notification of a valid condition; serial mode has no other waiter.
// ------------------=
int pthread_cond_signal(pthread_cond_t *cond) {
    int error = condition(cond);
    if (!error) *cond = *cond >= UINT32_MAX - 2 ? 1 : *cond + 1;
    return error;
}
// ------------------------=
// FUNC: pthread_cond_broadcast
// DESC: Notifies the serial condition's empty concurrent waiter set.
// ------------------=
int pthread_cond_broadcast(pthread_cond_t *cond) { return pthread_cond_signal(cond); }
// ------------------------=
// FUNC: pthread_cond_wait
// DESC: Rejects a wait requiring another thread without releasing or corrupting the caller's mutex.
// ------------------=
int pthread_cond_wait(pthread_cond_t *cond, pthread_mutex_t *mutex) {
    int error = condition(cond);
    if (error) return error;
    if (!mutex) return EINVAL;
    return ENOTSUP;
}
// ------------------------=
// FUNC: pthread_cond_timedwait
// DESC: Rejects unsupported concurrent waits rather than manufacturing a timeout.
// ------------------=
int pthread_cond_timedwait(pthread_cond_t *cond, pthread_mutex_t *mutex, const struct timespec *deadline) {
    if (!deadline || deadline->tv_nsec < 0 || deadline->tv_nsec >= 1000000000) return EINVAL;
    return pthread_cond_wait(cond, mutex);
}
