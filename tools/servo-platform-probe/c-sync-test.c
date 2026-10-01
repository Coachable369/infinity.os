#include <pthread.h>
#include <time.h>
#include <errno.h>
#include "../../sdk/servo-std/include/infinity-limits.h"
static pthread_mutex_t mutex = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t condition;
static int ready;
static int consumed;
// ------------------------=
// FUNC: mutex_capacity
// DESC: Exercises script-scale live locks, bounded exhaustion, stale handles and complete pool reuse.
// ------------------=
static int mutex_capacity(void) {
    static pthread_mutex_t locks[INFINITY_NATIVE_MUTEXES];
    unsigned count=0;
    int status=0;
    while (count<INFINITY_NATIVE_MUTEXES && !(status=pthread_mutex_init(&locks[count],0))) ++count;
    pthread_mutex_t extra;
    if (count<1024 || (count<INFINITY_NATIVE_MUTEXES && status!=EAGAIN) || pthread_mutex_init(&extra,0)!=EAGAIN) return 1;
    for (unsigned i=0;i<count;++i)
        if (pthread_mutex_lock(&locks[i]) || pthread_mutex_unlock(&locks[i])) return 2;
    pthread_mutex_t stale=locks[count/2];
    if (pthread_mutex_destroy(&locks[count/2]) || pthread_mutex_init(&extra,0)) return 3;
    if (pthread_mutex_trylock(&stale)!=EINVAL || pthread_mutex_destroy(&extra)) return 4;
    for (unsigned i=0;i<count;++i)
        if (i!=count/2 && pthread_mutex_destroy(&locks[i])) return 5;
    for (unsigned i=0;i<count;++i) if (pthread_mutex_init(&locks[i],0)) return 6;
    for (unsigned i=0;i<count;++i) if (pthread_mutex_destroy(&locks[i])) return 7;
    return 0;
}
// ------------------------=
// FUNC: infinity_c_sync_begin
// DESC: Creates a monotonic condition and tests recursive ownership and locked destruction.
// ------------------=
int infinity_c_sync_begin(void) {
    if (mutex_capacity()) return 6;
    pthread_condattr_t a;
    if (pthread_condattr_init(&a) || pthread_condattr_setclock(&a,CLOCK_MONOTONIC) || pthread_cond_init(&condition,&a) || pthread_condattr_destroy(&a)) return 1;
    pthread_mutexattr_t ma; pthread_mutex_t recursive;
    if (pthread_mutexattr_init(&ma) || pthread_mutexattr_settype(&ma,PTHREAD_MUTEX_RECURSIVE) || pthread_mutex_init(&recursive,&ma)) return 2;
    if (pthread_mutex_lock(&recursive) || pthread_mutex_lock(&recursive) || pthread_mutex_destroy(&recursive) != EBUSY) return 3;
    if (pthread_mutex_unlock(&recursive) || pthread_mutex_unlock(&recursive) || pthread_mutex_destroy(&recursive)) return 4;
    if (pthread_mutex_lock(&recursive) != EINVAL || pthread_mutexattr_destroy(&ma)) return 5;
    ready=0; consumed=0; return 0;
}
// ------------------------=
// FUNC: infinity_c_sync_consumer
// DESC: Parks until the producer publishes state, then observes it while owning the mutex.
// ------------------=
int infinity_c_sync_consumer(void) {
    if (pthread_mutex_lock(&mutex)) return 1;
    while (!ready) if (pthread_cond_wait(&condition,&mutex)) return 2;
    ++consumed;
    return pthread_mutex_unlock(&mutex);
}
// ------------------------=
// FUNC: infinity_c_sync_producer
// DESC: Signals both waiting native stacks without holding the mutex during a scheduler sleep.
// ------------------=
int infinity_c_sync_producer(void) {
    struct timespec pause={0,1000000};
    if (nanosleep(&pause,0) || pthread_mutex_lock(&mutex)) return 1;
    if (pthread_cond_destroy(&condition) != EBUSY) return 2;
    ready=1;
    if (pthread_cond_broadcast(&condition)) return 3;
    return pthread_mutex_unlock(&mutex);
}
// ------------------------=
// FUNC: infinity_c_sync_end
// DESC: Checks wake delivery and timed-wait mutex reacquisition before destroying handles.
// ------------------=
int infinity_c_sync_end(void) {
    if (consumed != 2 || pthread_mutex_lock(&mutex)) return 1;
    struct timespec past={0,0};
    if (pthread_cond_timedwait(&condition,&mutex,&past) != ETIMEDOUT) return 2;
    if (pthread_mutex_trylock(&mutex) != EBUSY) return 3;
    if (pthread_mutex_unlock(&mutex) || pthread_cond_destroy(&condition) || pthread_mutex_destroy(&mutex)) return 4;
    return 0;
}
