#include <errno.h>
#include <utime.h>
#include <time.h>
#include <unistd.h>
#include <sys/times.h>

// ------------------------=
// FUNC: clock_getres
// DESC: Reports unsupported resolution queries instead of inventing clock accuracy.
// ------------------=
int clock_getres(clockid_t clock, struct timespec *out) {
    (void)clock; (void)out;
    errno = ENOTSUP; return -1;
}

// ------------------------=
// FUNC: pause
// DESC: Rejects signal waits in the isolated process without asynchronous signals.
// ------------------=
int pause(void) { errno = ENOTSUP; return -1; }

// ------------------------=
// FUNC: rmdir
// DESC: Prevents directory mutation through read-only Object Store snapshots.
// ------------------=
int rmdir(const char *path) { errno = path ? EROFS : EFAULT; return -1; }

// ------------------------=
// FUNC: _times
// DESC: Rejects unavailable process CPU accounting rather than reporting wall time as CPU time.
// ------------------=
clock_t _times(struct tms *out) { (void)out; errno = ENOTSUP; return (clock_t)-1; }

// ------------------------=
// FUNC: times
// DESC: Uses the same unsupported accounting result for Newlib ports without underscored syscalls.
// ------------------=
clock_t times(struct tms *out) { return _times(out); }

// ------------------------=
// FUNC: utime
// DESC: Rejects Unix timestamp mutation; Object Store version metadata is not a writable POSIX timestamp.
// ------------------=
int utime(const char *path, const struct utimbuf *times) {
    (void)times;
    errno = path ? ENOTSUP : EFAULT;
    return -1;
}
