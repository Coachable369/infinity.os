#include <infinity/serial_sync.h>
#include <infinity/serial_tls.h>
#include <assert.h>
#include <errno.h>
static unsigned destroyed;
// ------------------------=
// FUNC: destroy_value
// DESC: Verifies reverse thread-destructor order.
// ------------------=
static void destroy_value(void *value) { assert((uintptr_t)value == 2 - destroyed); ++destroyed; }
// ------------------------=
// FUNC: main
// DESC: Verifies lock ownership, recursion, busy/deadlock errors and destroyed-state rejection.
// ------------------=
int main(void) {
    uint32_t mutex = INFINITY_MUTEX_INITIALIZER;
    assert(infinity_mutex_lock(&mutex, 0) == 0);
    assert(infinity_mutex_lock(&mutex, 1) == EBUSY);
    assert(infinity_mutex_lock(&mutex, 0) == EDEADLK);
    assert(infinity_mutex_destroy(&mutex) == EBUSY);
    assert(infinity_mutex_unlock(&mutex) == 0);
    assert(infinity_mutex_unlock(&mutex) == EPERM);
    assert(infinity_mutex_destroy(&mutex) == 0);
    assert(infinity_mutex_lock(&mutex, 0) == EINVAL);
    assert(infinity_mutex_init(&mutex, 1) == 0);
    assert(infinity_mutex_lock(&mutex, 0) == 0);
    assert(infinity_mutex_lock(&mutex, 0) == 0);
    assert(infinity_mutex_unlock(&mutex) == 0);
    assert(infinity_mutex_destroy(&mutex) == EBUSY);
    assert(infinity_mutex_unlock(&mutex) == 0);
    assert(infinity_mutex_destroy(&mutex) == 0);
    assert(infinity_mutex_init(0, 0) == EINVAL);
    uint32_t key;
    assert(infinity_key_create(&key, 0) == 0);
    assert(infinity_key_set(key, &mutex) == 0 && infinity_key_get(key) == &mutex);
    assert(infinity_key_set(0, &mutex) == EINVAL && !infinity_key_get(0));
    assert(infinity_thread_destructor(destroy_value, (void *)1) == 0);
    assert(infinity_thread_destructor(destroy_value, (void *)2) == 0);
    infinity_thread_cleanup();
    assert(destroyed == 2 && !infinity_key_get(key));
    for (unsigned i = 1; i < 128; ++i) assert(infinity_key_create(&key, 0) == 0);
    assert(infinity_key_create(&key, 0) == EAGAIN);
    return 0;
}
