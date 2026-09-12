#include <infinity/serial_sync.h>
#include <errno.h>
/* One execution thread per compiler image. Lower bits hold recursion depth;
   the high bits distinguish recursive, live and destroyed mutex states. */
#define LIVE 0x40000000u
#define RECURSIVE 0x80000000u
#define DEPTH 0x3fffffffu

// ------------------------=
// FUNC: infinity_mutex_init
// DESC: Initializes a serial mutex with normal or recursive ownership semantics.
// ------------------=
int infinity_mutex_init(uint32_t *mutex, int recursive) {
    if (!mutex || (recursive != 0 && recursive != 1)) return EINVAL;
    *mutex = LIVE | (recursive ? RECURSIVE : 0);
    return 0;
}
// ------------------------=
// FUNC: infinity_mutex_lock
// DESC: Acquires a serial lock, detecting self-deadlock and recursive overflow.
// ------------------=
int infinity_mutex_lock(uint32_t *mutex, int attempt) {
    if (!mutex) return EINVAL;
    if (*mutex == INFINITY_MUTEX_INITIALIZER) infinity_mutex_init(mutex, 0);
    if (!(*mutex & LIVE)) return EINVAL;
    uint32_t depth = *mutex & DEPTH;
    if (depth && !(*mutex & RECURSIVE)) return attempt ? EBUSY : EDEADLK;
    if (depth >= DEPTH - 1) return EAGAIN;
    ++*mutex;
    return 0;
}
// ------------------------=
// FUNC: infinity_mutex_unlock
// DESC: Releases one ownership level and rejects unowned unlocks.
// ------------------=
int infinity_mutex_unlock(uint32_t *mutex) {
    if (!mutex || *mutex == INFINITY_MUTEX_INITIALIZER || !(*mutex & LIVE)) return EINVAL;
    if (!(*mutex & DEPTH)) return EPERM;
    --*mutex;
    return 0;
}
// ------------------------=
// FUNC: infinity_mutex_destroy
// DESC: Rejects destruction while held and invalidates an idle mutex.
// ------------------=
int infinity_mutex_destroy(uint32_t *mutex) {
    if (!mutex) return EINVAL;
    if (*mutex == INFINITY_MUTEX_INITIALIZER) { *mutex = 0; return 0; }
    if (!(*mutex & LIVE)) return EINVAL;
    if (*mutex & DEPTH) return EBUSY;
    *mutex = 0;
    return 0;
}
