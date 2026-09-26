#include <pthread.h>
#include <stdint.h>
#include <stdbool.h>
#include <limits.h>
#include <errno.h>
#include <string.h>
#include <stdlib.h>
#include "include/infinity-error.h"
#include "include/infinity-limits.h"
extern int infinity_std_thread_create(size_t,void (*)(void *),void *,uint64_t *);
extern int infinity_std_thread_join(uint64_t);
extern void infinity_std_thread_detach(uint64_t);
extern uint64_t infinity_std_thread_id(void);
extern void infinity_std_thread_name(const unsigned char *,size_t);
extern int infinity_std_wait(const uint32_t *,uint32_t,uint64_t,bool);
extern int infinity_std_wake(const uint32_t *,bool);
typedef struct { int used, detached, done, joining; uint64_t id; void *(*start)(void *); void *argument, *result; } Thread;
static Thread threads[INFINITY_NATIVE_THREADS];
// ------------------------=
// FUNC: thread_entry
// DESC: Captures the C return value before the native executor performs TLS teardown.
// ------------------=
static void thread_entry(void *data) {
    Thread *t=data; t->result=t->start ? t->start(t->argument) : 0; t->done=1;
    if (t->detached) t->used=0;
}
// ------------------------=
// FUNC: thread_lookup
// DESC: Rejects stale handles and calls outside the native runtime's owner.
// ------------------=
static Thread *thread_lookup(pthread_t id) {
    if (!infinity_std_thread_id()) return 0;
    for (unsigned i=0;i<INFINITY_NATIVE_THREADS;++i) if (threads[i].used && threads[i].id==id) return &threads[i];
    return 0;
}
// ------------------------=
// FUNC: pthread_attr_init
// DESC: Initializes a bounded default stack on the native scheduler.
// ------------------=
int pthread_attr_init(pthread_attr_t *a) {
    if (!a) return EINVAL;
    *a=(pthread_attr_t){.is_initialized=1,.stacksize=262144,.detachstate=PTHREAD_CREATE_JOINABLE}; return 0;
}
// ------------------------=
// FUNC: pthread_attr_destroy
// DESC: Invalidates thread creation attributes.
// ------------------=
int pthread_attr_destroy(pthread_attr_t *a) { if (!a || !a->is_initialized) return EINVAL; a->is_initialized=0; return 0; }
// ------------------------=
// FUNC: pthread_attr_setstacksize
// DESC: Enforces a bounded stack request without integer narrowing.
// ------------------=
int pthread_attr_setstacksize(pthread_attr_t *a,size_t size) {
    if (!a || !a->is_initialized || size<16384 || size>8*1024*1024) return EINVAL;
    a->stacksize=(int)size; return 0;
}
// ------------------------=
// FUNC: pthread_attr_setdetachstate
// DESC: Accepts only the native joinable or detached lifecycle.
// ------------------=
int pthread_attr_setdetachstate(pthread_attr_t *a,int state) {
    if (!a || !a->is_initialized || (state!=PTHREAD_CREATE_JOINABLE && state!=PTHREAD_CREATE_DETACHED)) return EINVAL;
    a->detachstate=state; return 0;
}
// ------------------------=
// FUNC: pthread_create
// DESC: Starts a real independent native stack with bounded handle and stack resources.
// ------------------=
int pthread_create(pthread_t *out,const pthread_attr_t *a,void *(*start)(void *),void *argument) {
    if (!out || !start || !infinity_std_thread_id()) return EINVAL;
    if (a && (!a->is_initialized || a->stackaddr || a->stacksize<16384 || a->stacksize>8*1024*1024 || (a->detachstate!=PTHREAD_CREATE_JOINABLE && a->detachstate!=PTHREAD_CREATE_DETACHED))) return EINVAL;
    for (unsigned i=0;i<INFINITY_NATIVE_THREADS;++i) {
        Thread *t=&threads[i]; if (t->used) continue;
        *t=(Thread){.used=1,.detached=a && a->detachstate==PTHREAD_CREATE_DETACHED,.start=start,.argument=argument};
        int status=infinity_std_thread_create(a ? (size_t)a->stacksize : 262144,thread_entry,t,&t->id);
        if (status) { t->used=0; return infinity_native_error(status); }
        if (t->id>=UINT32_MAX) { t->start=0; t->detached=1; infinity_std_thread_detach(t->id); return EAGAIN; }
        *out=(pthread_t)t->id;
        if (t->detached) infinity_std_thread_detach(t->id);
        return 0;
    }
    return EAGAIN;
}
// ------------------------=
// FUNC: pthread_self
// DESC: Maps the reserved native root identity without silently truncating worker identities.
// ------------------=
pthread_t pthread_self(void) {
    uint64_t id=infinity_std_thread_id();
    if (!id || (id>=UINT32_MAX && id!=UINT64_MAX)) abort();
    return (pthread_t)id;
}
// ------------------------=
// FUNC: pthread_equal
// DESC: Compares native opaque thread identities.
// ------------------=
int pthread_equal(pthread_t a,pthread_t b) { return a==b; }
// ------------------------=
// FUNC: pthread_join
// DESC: Waits through the native scheduler and transfers the actual C return value.
// ------------------=
int pthread_join(pthread_t id,void **result) {
    Thread *t=thread_lookup(id); if (!t) return ESRCH;
    if (t->detached || t->joining) return EINVAL;
    if (id==pthread_self()) return EDEADLK;
    t->joining=1;
    int status=infinity_std_thread_join(t->id); if (status) { t->joining=0; return infinity_native_error(status); }
    if (result) *result=t->result;
    t->used=0; return 0;
}
// ------------------------=
// FUNC: pthread_detach
// DESC: Releases join authority while preserving an active stack until it finishes.
// ------------------=
int pthread_detach(pthread_t id) {
    Thread *t=thread_lookup(id); if (!t) return ESRCH;
    if (t->detached || t->joining) return EINVAL;
    t->detached=1; infinity_std_thread_detach(t->id);
    if (t->done) t->used=0;
    return 0;
}
// ------------------------=
// FUNC: pthread_setname_np
// DESC: Updates only the calling native thread's bounded diagnostic name.
// ------------------=
int pthread_setname_np(pthread_t id,const char *name) {
    if (!pthread_equal(id,pthread_self())) return ENOTSUP;
    size_t n=0; while (n<32 && name[n]) ++n;
    if (n==32) return ERANGE;
    infinity_std_thread_name((const unsigned char *)name,n); return 0;
}
// ------------------------=
// FUNC: pthread_once
// DESC: Executes one initializer while concurrent native stacks park until publication.
// ------------------=
int pthread_once(pthread_once_t *once,void (*initialize)(void)) {
    if (!once || !initialize || !once->is_initialized || !infinity_std_thread_id()) return EINVAL;
    _Static_assert(sizeof(once->init_executed)==sizeof(uint32_t),"native once wait word");
    for (;;) {
        int state=__atomic_load_n(&once->init_executed,__ATOMIC_ACQUIRE);
        if (state==2) return 0;
        if (state==0 && __atomic_compare_exchange_n(&once->init_executed,&state,1,false,__ATOMIC_ACQ_REL,__ATOMIC_ACQUIRE)) {
            initialize(); __atomic_store_n(&once->init_executed,2,__ATOMIC_RELEASE);
            infinity_std_wake((uint32_t *)&once->init_executed,true); return 0;
        }
        int status=infinity_std_wait((uint32_t *)&once->init_executed,1,0,false);
        if (status) return infinity_native_error(status);
    }
}
