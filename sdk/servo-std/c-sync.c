/* Bounded handles for the single-owner native executor. Not a multicore pthread ABI. */
#include <pthread.h>
#include <stdint.h>
#include <stdbool.h>
#include <errno.h>
#include <limits.h>
#include <time.h>
#include "include/infinity-error.h"
#include "include/infinity-limits.h"
extern uint64_t infinity_std_thread_id(void);
extern int infinity_std_wait(const uint32_t *, uint32_t, uint64_t, bool);
extern int infinity_std_wake(const uint32_t *, bool);
#define CAPACITY 128
typedef struct { uint32_t generation, sequence, depth, waiters; uint64_t owner; int active, recursive; } Mutex;
typedef struct { uint32_t generation, sequence, waiters; int active; clockid_t clock; } Condition;
static Mutex mutexes[CAPACITY];
static Condition conditions[CAPACITY];
typedef struct { uint64_t id; uint32_t depth; } Reader;
typedef struct { uint32_t generation, sequence, waiters, writers; int active; uint64_t writer; Reader readers[INFINITY_NATIVE_READERS]; } Rwlock;
static Rwlock rwlocks[CAPACITY];

// ------------------------=
// FUNC: pthread_rwlock_init
// DESC: Leases a private bounded reader/writer lock on the native executor.
// ------------------=
int pthread_rwlock_init(pthread_rwlock_t *out,const pthread_rwlockattr_t *a) {
    if (!out || !infinity_std_thread_id()) return EINVAL;
    if (a) return ENOTSUP;
    for (uint32_t i=0;i<CAPACITY;++i) {
        Rwlock *r=&rwlocks[i];
        if (r->active || r->generation >= UINT32_MAX/CAPACITY-1) continue;
        uint32_t generation=r->generation+1;
        *r=(Rwlock){.generation=generation,.active=1}; *out=generation*CAPACITY+i; return 0;
    }
    return EAGAIN;
}
// ------------------------=
// FUNC: rwlock_lookup
// DESC: Resolves generation-tagged handles and the static initializer.
// ------------------=
static Rwlock *rwlock_lookup(pthread_rwlock_t *handle) {
    if (!handle || !infinity_std_thread_id()) return 0;
    if (*handle==PTHREAD_RWLOCK_INITIALIZER && pthread_rwlock_init(handle,0)) return 0;
    Rwlock *r=&rwlocks[*handle % CAPACITY];
    return r->active && r->generation==*handle/CAPACITY ? r : 0;
}
// ------------------------=
// FUNC: rwlock_acquire
// DESC: Tracks reader ownership and parks contention with preference for waiting writers.
// ------------------=
static int rwlock_acquire(pthread_rwlock_t *handle,int write,int try_only) {
    Rwlock *r=rwlock_lookup(handle); if (!r) return EINVAL;
    uint64_t id=infinity_std_thread_id();
    for (;;) {
        uint32_t sequence=__atomic_load_n(&r->sequence,__ATOMIC_ACQUIRE);
        Reader *mine=0,*empty=0; int occupied=0;
        for (unsigned i=0;i<INFINITY_NATIVE_READERS;++i) {
            Reader *reader=&r->readers[i];
            if (reader->depth) { occupied=1; if (reader->id==id) mine=reader; }
            else if (!empty) empty=reader;
        }
        if (r->writer==id || (write && mine)) return try_only ? EBUSY : EDEADLK;
        if (write && !r->writer && !occupied) { r->writer=id; return 0; }
        if (!write && !r->writer && (!r->writers || mine)) {
            Reader *reader=mine ? mine : empty;
            if (!reader || reader->depth==UINT32_MAX) return EAGAIN;
            reader->id=id; ++reader->depth; return 0;
        }
        if (try_only) return EBUSY;
        ++r->waiters; if (write) ++r->writers;
        int status=infinity_native_error(infinity_std_wait(&r->sequence,sequence,0,false));
        --r->waiters; if (write) --r->writers;
        if (status) return status;
    }
}
// ------------------------=
// FUNC: pthread_rwlock_rdlock
// DESC: Acquires a shared reader lease, parking when a writer blocks it.
// ------------------=
int pthread_rwlock_rdlock(pthread_rwlock_t *r) { return rwlock_acquire(r,0,0); }
// ------------------------=
// FUNC: pthread_rwlock_wrlock
// DESC: Acquires exclusive write ownership after all readers release.
// ------------------=
int pthread_rwlock_wrlock(pthread_rwlock_t *r) { return rwlock_acquire(r,1,0); }
// ------------------------=
// FUNC: pthread_rwlock_tryrdlock
// DESC: Attempts shared ownership without parking.
// ------------------=
int pthread_rwlock_tryrdlock(pthread_rwlock_t *r) { return rwlock_acquire(r,0,1); }
// ------------------------=
// FUNC: pthread_rwlock_trywrlock
// DESC: Attempts exclusive ownership without parking.
// ------------------=
int pthread_rwlock_trywrlock(pthread_rwlock_t *r) { return rwlock_acquire(r,1,1); }
// ------------------------=
// FUNC: pthread_rwlock_unlock
// DESC: Releases only the calling thread's ownership and wakes parked contenders.
// ------------------=
int pthread_rwlock_unlock(pthread_rwlock_t *handle) {
    Rwlock *r=rwlock_lookup(handle); if (!r) return EINVAL;
    uint64_t id=infinity_std_thread_id(); int released=0;
    if (r->writer==id) { r->writer=0; released=1; }
    else for (unsigned i=0;i<INFINITY_NATIVE_READERS;++i) if (r->readers[i].id==id && r->readers[i].depth) { --r->readers[i].depth; released=1; break; }
    if (!released) return EPERM;
    __atomic_add_fetch(&r->sequence,1,__ATOMIC_RELEASE); infinity_std_wake(&r->sequence,true); return 0;
}
// ------------------------=
// FUNC: pthread_rwlock_destroy
// DESC: Rejects destruction with owners or parked contenders and invalidates the generation.
// ------------------=
int pthread_rwlock_destroy(pthread_rwlock_t *handle) {
    Rwlock *r=rwlock_lookup(handle); if (!r) return EINVAL;
    if (r->writer || r->waiters) return EBUSY;
    for (unsigned i=0;i<INFINITY_NATIVE_READERS;++i) if (r->readers[i].depth) return EBUSY;
    r->active=0; return 0;
}

// ------------------------=
// FUNC: pthread_mutexattr_init
// DESC: Initializes private, nonrecursive native mutex attributes.
// ------------------=
int pthread_mutexattr_init(pthread_mutexattr_t *a) { if (!a) return EINVAL; *a = (pthread_mutexattr_t){0}; a->is_initialized = 1; return 0; }
// ------------------------=
// FUNC: pthread_mutexattr_destroy
// DESC: Invalidates initialized attributes.
// ------------------=
int pthread_mutexattr_destroy(pthread_mutexattr_t *a) { if (!a || !a->is_initialized) return EINVAL; a->is_initialized = 0; return 0; }
// ------------------------=
// FUNC: pthread_mutexattr_settype
// DESC: Selects supported mutex recursion behavior without enabling cross-process sharing.
// ------------------=
int pthread_mutexattr_settype(pthread_mutexattr_t *a, int type) {
    if (!a || !a->is_initialized || (type != PTHREAD_MUTEX_NORMAL && type != PTHREAD_MUTEX_RECURSIVE && type != PTHREAD_MUTEX_ERRORCHECK && type != PTHREAD_MUTEX_DEFAULT)) return EINVAL;
    a->type = type; a->recursive = type == PTHREAD_MUTEX_RECURSIVE; return 0;
}
// ------------------------=
// FUNC: pthread_mutex_init
// DESC: Leases a bounded generation-tagged native mutex handle.
// ------------------=
int pthread_mutex_init(pthread_mutex_t *out, const pthread_mutexattr_t *a) {
    if (!out || !infinity_std_thread_id() || (a && !a->is_initialized)) return EINVAL;
    for (uint32_t i = 0; i < CAPACITY; ++i) {
        Mutex *m = &mutexes[i];
        if (m->active || m->generation >= (UINT32_MAX / CAPACITY) - 1) continue;
        uint32_t generation = m->generation + 1;
        *m = (Mutex){.generation = generation, .active = 1, .recursive = a && a->recursive};
        *out = generation * CAPACITY + i; return 0;
    }
    return EAGAIN;
}
// ------------------------=
// FUNC: mutex_lookup
// DESC: Resolves a live generation; lazily creates a standard static initializer.
// ------------------=
static Mutex *mutex_lookup(pthread_mutex_t *handle) {
    if (!handle || !infinity_std_thread_id()) return 0;
    if (*handle == PTHREAD_MUTEX_INITIALIZER && pthread_mutex_init(handle, 0)) return 0;
    Mutex *m = &mutexes[*handle % CAPACITY];
    return m->active && m->generation == *handle / CAPACITY ? m : 0;
}
// ------------------------=
// FUNC: pthread_mutex_trylock
// DESC: Acquires unowned or recursively-owned mutexes without blocking.
// ------------------=
int pthread_mutex_trylock(pthread_mutex_t *handle) {
    Mutex *m = mutex_lookup(handle); if (!m) return EINVAL;
    uint64_t id = infinity_std_thread_id();
    if (m->owner && !(m->owner == id && m->recursive)) return EBUSY;
    if (m->depth == UINT32_MAX) return EAGAIN;
    m->owner = id; ++m->depth; return 0;
}
// ------------------------=
// FUNC: pthread_mutex_lock
// DESC: Parks contending native stacks on the mutex sequence instead of spinning on the UI CPU.
// ------------------=
int pthread_mutex_lock(pthread_mutex_t *handle) {
    Mutex *m = mutex_lookup(handle); if (!m) return EINVAL;
    for (;;) {
        uint32_t sequence = __atomic_load_n(&m->sequence, __ATOMIC_ACQUIRE);
        int status = pthread_mutex_trylock(handle);
        if (status != EBUSY) return status;
        if (m->owner == infinity_std_thread_id()) return EDEADLK;
        ++m->waiters;
        status = infinity_native_error(infinity_std_wait(&m->sequence, sequence, 0, false));
        --m->waiters;
        if (status) return status;
    }
}
// ------------------------=
// FUNC: pthread_mutex_unlock
// DESC: Enforces ownership and wakes native contenders only after final recursive release.
// ------------------=
int pthread_mutex_unlock(pthread_mutex_t *handle) {
    Mutex *m = mutex_lookup(handle); if (!m) return EINVAL;
    if (m->owner != infinity_std_thread_id()) return EPERM;
    if (--m->depth == 0) {
        m->owner = 0; __atomic_add_fetch(&m->sequence, 1, __ATOMIC_RELEASE);
        infinity_std_wake(&m->sequence, false);
    }
    return 0;
}
// ------------------------=
// FUNC: pthread_mutex_destroy
// DESC: Refuses a held mutex and invalidates its generation on release.
// ------------------=
int pthread_mutex_destroy(pthread_mutex_t *handle) {
    Mutex *m = mutex_lookup(handle); if (!m) return EINVAL;
    if (m->owner || m->waiters) return EBUSY;
    m->active = 0; *handle = 0; return 0;
}
// ------------------------=
// FUNC: pthread_condattr_init
// DESC: Uses POSIX realtime by default; native callers may explicitly choose monotonic time.
// ------------------=
int pthread_condattr_init(pthread_condattr_t *a) { if (!a) return EINVAL; *a = (pthread_condattr_t){.is_initialized=1, .clock=CLOCK_REALTIME}; return 0; }
// ------------------------=
// FUNC: pthread_condattr_setclock
// DESC: Accepts only clocks implemented by the native clock adapter.
// ------------------=
int pthread_condattr_setclock(pthread_condattr_t *a, clockid_t clock) {
    if (!a || !a->is_initialized || (clock != CLOCK_REALTIME && clock != CLOCK_MONOTONIC)) return EINVAL;
    a->clock = clock; return 0;
}
// ------------------------=
// FUNC: pthread_condattr_destroy
// DESC: Invalidates condition attributes.
// ------------------=
int pthread_condattr_destroy(pthread_condattr_t *a) { if (!a || !a->is_initialized) return EINVAL; a->is_initialized=0; return 0; }
// ------------------------=
// FUNC: pthread_cond_init
// DESC: Leases a bounded native condition handle and retains its deadline clock.
// ------------------=
int pthread_cond_init(pthread_cond_t *out, const pthread_condattr_t *a) {
    if (!out || !infinity_std_thread_id() || (a && !a->is_initialized)) return EINVAL;
    for (uint32_t i=0; i<CAPACITY; ++i) {
        Condition *c=&conditions[i];
        if (c->active || c->generation >= (UINT32_MAX / CAPACITY)-1) continue;
        uint32_t generation=c->generation+1;
        *c=(Condition){.generation=generation,.active=1,.clock=a ? a->clock : CLOCK_REALTIME};
        *out=generation*CAPACITY+i; return 0;
    }
    return EAGAIN;
}
// ------------------------=
// FUNC: condition_lookup
// DESC: Resolves only live condition generations, including static initializers.
// ------------------=
static Condition *condition_lookup(pthread_cond_t *handle) {
    if (!handle || !infinity_std_thread_id()) return 0;
    if (*handle == PTHREAD_COND_INITIALIZER && pthread_cond_init(handle,0)) return 0;
    Condition *c=&conditions[*handle % CAPACITY];
    return c->active && c->generation == *handle / CAPACITY ? c : 0;
}
// ------------------------=
// FUNC: condition_wait
// DESC: Releases the mutex, performs compare-and-park, then reacquires it even after timeout.
// ------------------=
static int condition_wait(pthread_cond_t *handle, pthread_mutex_t *mutex, const struct timespec *deadline) {
    Condition *c=condition_lookup(handle); Mutex *m=mutex_lookup(mutex);
    if (!c || !m) return EINVAL;
    if (m->owner != infinity_std_thread_id() || m->depth != 1) return EPERM;
    uint64_t delay=0;
    if (deadline) {
        if (deadline->tv_sec < 0 || deadline->tv_nsec < 0 || deadline->tv_nsec >= 1000000000L) return EINVAL;
        struct timespec now;
        if (clock_gettime(c->clock,&now)) return errno;
        if (deadline->tv_sec > now.tv_sec || (deadline->tv_sec == now.tv_sec && deadline->tv_nsec > now.tv_nsec)) {
            uint64_t seconds=(uint64_t)(deadline->tv_sec-now.tv_sec);
            int64_t nanos=deadline->tv_nsec-now.tv_nsec;
            if (nanos < 0) { --seconds; nanos+=1000000000; }
            if (seconds > (UINT64_MAX-(uint64_t)nanos)/1000000000) return EOVERFLOW;
            delay=seconds*1000000000+(uint64_t)nanos;
        }
    }
    uint32_t sequence=__atomic_load_n(&c->sequence,__ATOMIC_ACQUIRE);
    ++c->waiters;
    int status=pthread_mutex_unlock(mutex);
    if (!status) {
        status=infinity_native_error(infinity_std_wait(&c->sequence,sequence,delay,deadline != 0));
        int locked=pthread_mutex_lock(mutex);
        if (locked) status=locked;
    }
    --c->waiters; return status;
}
// ------------------------=
// FUNC: pthread_cond_wait
// DESC: Waits indefinitely using the native event scheduler.
// ------------------=
int pthread_cond_wait(pthread_cond_t *c,pthread_mutex_t *m) { return condition_wait(c,m,0); }
// ------------------------=
// FUNC: pthread_cond_timedwait
// DESC: Interprets the deadline in the condition's configured clock domain.
// ------------------=
int pthread_cond_timedwait(pthread_cond_t *c,pthread_mutex_t *m,const struct timespec *t) { return t ? condition_wait(c,m,t) : EINVAL; }
// ------------------------=
// FUNC: pthread_cond_signal
// DESC: Wakes one native waiter after advancing the observed condition sequence.
// ------------------=
int pthread_cond_signal(pthread_cond_t *handle) {
    Condition *c=condition_lookup(handle); if (!c) return EINVAL;
    __atomic_add_fetch(&c->sequence,1,__ATOMIC_RELEASE); infinity_std_wake(&c->sequence,false); return 0;
}
// ------------------------=
// FUNC: pthread_cond_broadcast
// DESC: Wakes all native waiters without allowing future waits to consume stale signals.
// ------------------=
int pthread_cond_broadcast(pthread_cond_t *handle) {
    Condition *c=condition_lookup(handle); if (!c) return EINVAL;
    __atomic_add_fetch(&c->sequence,1,__ATOMIC_RELEASE); infinity_std_wake(&c->sequence,true); return 0;
}
// ------------------------=
// FUNC: pthread_cond_destroy
// DESC: Refuses to recycle a condition while native stacks are waiting on it.
// ------------------=
int pthread_cond_destroy(pthread_cond_t *handle) {
    Condition *c=condition_lookup(handle); if (!c) return EINVAL;
    if (c->waiters) return EBUSY;
    c->active=0; *handle=0; return 0;
}
