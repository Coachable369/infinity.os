#include <pthread.h>
#include <stdint.h>
#include <time.h>
#include <errno.h>
#include "../../sdk/servo-std/include/infinity-limits.h"
static pthread_once_t once=PTHREAD_ONCE_INIT;
static int count;
static pthread_rwlock_t rw=PTHREAD_RWLOCK_INITIALIZER;
static int written;
// ------------------------=
// FUNC: rw_writer
// DESC: Verifies exclusion while the root retains a reader lease, then publishes under write ownership.
// ------------------=
static void *rw_writer(void *arg) {
    if (pthread_rwlock_trywrlock(&rw)!=EBUSY || pthread_rwlock_unlock(&rw)!=EPERM) return 0;
    if (pthread_rwlock_wrlock(&rw)) return 0;
    written=1;
    if (pthread_rwlock_destroy(&rw)!=EBUSY || pthread_rwlock_unlock(&rw)) return 0;
    return arg;
}
// ------------------------=
// FUNC: initialize_once
// DESC: Yields during initialization to exercise concurrent once callers.
// ------------------=
static void initialize_once(void) { struct timespec pause={0,1000000}; ++count; nanosleep(&pause,0); }
// ------------------------=
// FUNC: c_worker
// DESC: Executes through pthread_create and returns a checked opaque result.
// ------------------=
static void *c_worker(void *arg) {
    if (!pthread_equal(pthread_self(),pthread_self())) return 0;
    if (pthread_once(&once,initialize_once)) return 0;
    return arg;
}
// ------------------------=
// FUNC: infinity_c_thread_test
// DESC: Proves C creation, distinct identities, joining, return values, once behavior and stale-handle rejection.
// ------------------=
int infinity_c_thread_test(void) {
    pthread_attr_t a; pthread_t first,second;
    if (pthread_attr_init(&a) || pthread_attr_setstacksize(&a,65536)) return 1;
    if (pthread_attr_setstacksize(&a,(size_t)-1) != EINVAL) return 2;
    if (pthread_create(&first,&a,c_worker,(void *)(uintptr_t)17) || pthread_create(&second,&a,c_worker,(void *)(uintptr_t)23)) return 3;
    if (pthread_equal(first,second) || pthread_equal(first,pthread_self())) return 4;
    void *one=0,*two=0;
    if (pthread_join(first,&one) || pthread_join(second,&two)) return 5;
    if ((uintptr_t)one != 17 || (uintptr_t)two != 23 || count != 1) return 6;
    if (pthread_join(first,0) != ESRCH || pthread_detach(second) != ESRCH) return 7;
    if (pthread_rwlock_rdlock(&rw) || pthread_rwlock_rdlock(&rw)) return 8;
    if (pthread_rwlock_wrlock(&rw)!=EDEADLK || pthread_rwlock_destroy(&rw)!=EBUSY) return 9;
    if (pthread_create(&first,&a,rw_writer,(void *)(uintptr_t)31)) return 10;
    struct timespec pause={0,2000000};
    if (nanosleep(&pause,0) || written) return 11;
    if (pthread_rwlock_unlock(&rw) || pthread_rwlock_unlock(&rw) || pthread_join(first,&one)) return 12;
    if ((uintptr_t)one!=31 || written!=1 || pthread_rwlock_destroy(&rw)) return 13;
    if (pthread_rwlock_rdlock(&rw)!=EINVAL) return 14;
    pthread_t handles[INFINITY_NATIVE_THREADS], rejected;
    for (unsigned i=0;i<INFINITY_NATIVE_THREADS;++i)
        if (pthread_create(&handles[i],&a,c_worker,(void *)(uintptr_t)(i+1))) return 15;
    if (pthread_create(&rejected,&a,c_worker,0)!=EAGAIN) return 16;
    for (unsigned i=0;i<INFINITY_NATIVE_THREADS;++i) {
        void *value=0;
        if (pthread_join(handles[i],&value) || (uintptr_t)value!=i+1) return 17;
    }
    if (pthread_create(&first,&a,c_worker,(void *)(uintptr_t)41) ||
        pthread_join(first,&one) || (uintptr_t)one!=41) return 18;
    return pthread_attr_destroy(&a);
}
