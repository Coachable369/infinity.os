/* Session-only SQLite boundary. Persistent object storage is not yet exposed. */
#include <sqlite3.h>
#include <pthread.h>
#include <stdint.h>
#include <stdlib.h>
#include <time.h>
extern int infinity_std_entropy(unsigned char *,size_t);
extern uint64_t infinity_std_thread_id(void);
typedef struct { int used, permanent; pthread_mutex_t handle; } NativeMutex;
static NativeMutex locks[128];
static int configured;
// ------------------------=
// FUNC: mutex_init
// DESC: Accepts initialization only after the native provider is bound.
// ------------------=
static int mutex_init(void) { return infinity_std_thread_id() ? SQLITE_OK : SQLITE_MISUSE; }
// ------------------------=
// FUNC: mutex_end
// DESC: Releases static mutex resources only after SQLite shuts down.
// ------------------=
static int mutex_end(void) {
    for (unsigned i=0;i<128;++i) if (locks[i].used) {
        if (!locks[i].permanent || pthread_mutex_destroy(&locks[i].handle)) return SQLITE_BUSY;
        locks[i].used=0;
    }
    return SQLITE_OK;
}
// ------------------------=
// FUNC: mutex_alloc
// DESC: Allocates native recursive/private mutexes without recursing through SQLite allocation.
// ------------------=
static sqlite3_mutex *mutex_alloc(int type) {
    if (type<0 || type>13) return 0;
    unsigned begin=type>=2 ? (unsigned)type-2 : 12;
    unsigned end=type>=2 ? begin+1 : 128;
    for (unsigned i=begin;i<end;++i) {
        NativeMutex *m=&locks[i];
        if (m->used) { if (type>=2) return (sqlite3_mutex *)m; continue; }
        pthread_mutexattr_t a;
        if (pthread_mutexattr_init(&a)) return 0;
        if (pthread_mutexattr_settype(&a,PTHREAD_MUTEX_RECURSIVE)) { pthread_mutexattr_destroy(&a); return 0; }
        int status=pthread_mutex_init(&m->handle,&a); pthread_mutexattr_destroy(&a);
        if (status) return 0;
        m->used=1; m->permanent=type>=2; return (sqlite3_mutex *)m;
    }
    return 0;
}
// ------------------------=
// FUNC: mutex_free
// DESC: Returns only dynamic mutexes after SQLite releases ownership.
// ------------------=
static void mutex_free(sqlite3_mutex *value) {
    NativeMutex *m=(NativeMutex *)value;
    if (!m || !m->used || m->permanent || pthread_mutex_destroy(&m->handle)) abort();
    m->used=0;
}
// ------------------------=
// FUNC: mutex_enter
// DESC: Parks native contention and fails closed on invalid mutex ownership.
// ------------------=
static void mutex_enter(sqlite3_mutex *value) { if (pthread_mutex_lock(&((NativeMutex *)value)->handle)) abort(); }
// ------------------------=
// FUNC: mutex_try
// DESC: Reports contention without pretending a failed lock succeeded.
// ------------------=
static int mutex_try(sqlite3_mutex *value) { return pthread_mutex_trylock(&((NativeMutex *)value)->handle) ? SQLITE_BUSY : SQLITE_OK; }
// ------------------------=
// FUNC: mutex_leave
// DESC: Releases a real native lock, enforcing its owner.
// ------------------=
static void mutex_leave(sqlite3_mutex *value) { if (pthread_mutex_unlock(&((NativeMutex *)value)->handle)) abort(); }
// ------------------------=
// FUNC: vfs_open
// DESC: Denies file-backed databases; SQLite's own in-memory pager remains functional.
// ------------------=
static int vfs_open(sqlite3_vfs *v,const char *name,sqlite3_file *f,int flags,int *out) {
    (void)v;(void)name;(void)flags; f->pMethods=0; if(out)*out=0; return SQLITE_CANTOPEN;
}
// ------------------------=
// FUNC: vfs_delete
// DESC: Denies deletion outside native object authority.
// ------------------=
static int vfs_delete(sqlite3_vfs *v,const char *name,int sync) { (void)v;(void)name;(void)sync;return SQLITE_IOERR_DELETE; }
// ------------------------=
// FUNC: vfs_access
// DESC: Reports no file namespace in the memory-only profile.
// ------------------=
static int vfs_access(sqlite3_vfs *v,const char *name,int flags,int *out) { (void)v;(void)name;(void)flags;*out=0;return SQLITE_OK; }
// ------------------------=
// FUNC: vfs_path
// DESC: Rejects path conversion rather than manufacturing host paths.
// ------------------=
static int vfs_path(sqlite3_vfs *v,const char *name,int length,char *out) { (void)v;(void)name;if(length>0)*out=0;return SQLITE_CANTOPEN; }
// ------------------------=
// FUNC: vfs_random
// DESC: Uses granted native entropy, never a deterministic fallback seed.
// ------------------=
static int vfs_random(sqlite3_vfs *v,int bytes,char *out) {
    (void)v;if(bytes<0 || infinity_std_entropy((unsigned char *)out,(size_t)bytes)) abort();return bytes;
}
// ------------------------=
// FUNC: vfs_sleep
// DESC: Sleeps through the native scheduler without blocking its owner CPU.
// ------------------=
static int vfs_sleep(sqlite3_vfs *v,int micros) {
    (void)v; if(micros<=0)return 0;
    struct timespec duration={micros/1000000,(micros%1000000)*1000};
    return nanosleep(&duration,0) ? 0 : micros;
}
// ------------------------=
// FUNC: vfs_time
// DESC: Converts actual UTC into SQLite Julian time or reports unavailable clock state.
// ------------------=
static int vfs_time(sqlite3_vfs *v,double *out) {
    (void)v; struct timespec now;if(clock_gettime(CLOCK_REALTIME,&now))return SQLITE_ERROR;
    *out=2440587.5+(double)now.tv_sec/86400.0+(double)now.tv_nsec/86400000000000.0;return SQLITE_OK;
}
static sqlite3_vfs memory_vfs={.iVersion=1,.szOsFile=sizeof(sqlite3_file),.mxPathname=256,
    .zName="infinity-session",.xOpen=vfs_open,.xDelete=vfs_delete,.xAccess=vfs_access,
    .xFullPathname=vfs_path,.xRandomness=vfs_random,.xSleep=vfs_sleep,.xCurrentTime=vfs_time};
// ------------------------=
// FUNC: sqlite3_os_init
// DESC: Registers only the explicit session backend after real synchronization is configured.
// ------------------=
int sqlite3_os_init(void) { return configured ? sqlite3_vfs_register(&memory_vfs,1) : SQLITE_MISUSE; }
// ------------------------=
// FUNC: sqlite3_os_end
// DESC: Removes the session backend on SQLite shutdown.
// ------------------=
int sqlite3_os_end(void) { return sqlite3_vfs_unregister(&memory_vfs); }
// ------------------------=
// FUNC: infinity_sqlite_initialize
// DESC: Installs native mutexes before SQLite may initialize; call before Servo startup.
// ------------------=
int infinity_sqlite_initialize(void) {
    if(!infinity_std_thread_id())return SQLITE_MISUSE;
    if(configured)return sqlite3_initialize();
    sqlite3_mutex_methods methods={mutex_init,mutex_end,mutex_alloc,mutex_free,mutex_enter,mutex_try,mutex_leave,0,0};
    int result=sqlite3_config(SQLITE_CONFIG_MUTEX,&methods);if(result)return result;
    configured=1;return sqlite3_initialize();
}
