#include "admission.h"
#include <assert.h>
#include <pthread.h>
#include <sched.h>
#include <stddef.h>

static unsigned gate;
static unsigned ready;
static unsigned begin;
static unsigned inside;
static unsigned completions;
static unsigned scratch[64];
enum { THREADS = 16, OPERATIONS = 4000 };

// ------------------------=
// FUNC: contend
// DESC: Races native admission from several callers and verifies exclusive scratch ownership until release.
// ------------------=
static void *contend(void *argument) {
    unsigned value = (unsigned)(size_t)argument;
    __atomic_fetch_add(&ready, 1, __ATOMIC_RELEASE);
    while (!__atomic_load_n(&begin, __ATOMIC_ACQUIRE)) sched_yield();
    for (unsigned operation = 0; operation < OPERATIONS; ++operation) {
        while (!native_engine_try_acquire(&gate)) sched_yield();
        assert(__atomic_fetch_add(&inside, 1, __ATOMIC_SEQ_CST) == 0);
        for (size_t i = 0; i < 64; ++i) scratch[i] = value + operation;
        if (!(operation % 64)) sched_yield();
        for (size_t i = 0; i < 64; ++i) assert(scratch[i] == value + operation);
        ++completions;
        assert(__atomic_fetch_sub(&inside, 1, __ATOMIC_SEQ_CST) == 1);
        native_engine_release(&gate);
    }
    return NULL;
}

// ------------------------=
// FUNC: main
// DESC: Verifies rejection preserves ownership, release permits reuse, and concurrent callers never share native storage.
// ------------------=
int main(void) {
    assert(!native_engine_is_active(&gate));
    assert(native_engine_try_acquire(&gate));
    assert(native_engine_is_active(&gate));
    assert(!native_engine_try_acquire(&gate));
    assert(native_engine_is_active(&gate));
    native_engine_release(&gate);
    pthread_t threads[THREADS];
    for (size_t i = 0; i < THREADS; ++i) assert(!pthread_create(&threads[i], NULL, contend, (void *)(i + 1)));
    while (__atomic_load_n(&ready, __ATOMIC_ACQUIRE) != THREADS) sched_yield();
    __atomic_store_n(&begin, 1, __ATOMIC_RELEASE);
    for (size_t i = 0; i < THREADS; ++i) assert(!pthread_join(threads[i], NULL));
    assert(completions == THREADS * OPERATIONS);
    assert(!native_engine_is_active(&gate));
    assert(inside == 0);
    return 0;
}
