/* No ambient Unix filesystem, descriptors, processes or terminal authority. */
#include <unistd.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <time.h>
#include <errno.h>
#include "include/infinity-error.h"
/* Newlib's x86 syscall connectors use public spellings; ARM's connectors use
 * underscore spellings. Both enter the identical native denial/time policy. */
#if defined(__x86_64__)
#define _gettimeofday gettimeofday
#define _open open
#define _close close
#define _read read
#define _write write
#define _lseek lseek
#define _fstat fstat
#define _isatty isatty
#define _unlink unlink
#define _getpid getpid
#define _kill kill
#endif
extern int infinity_std_entropy(unsigned char *, size_t);
// ------------------------=
// FUNC: getentropy
// DESC: Serves bounded C entropy requests from the granted native source, failing closed on denial.
// ------------------=
int getentropy(void *output,size_t length) {
    if (length>256 || (!output && length)) { errno=EIO; return -1; }
    if (!length) return 0;
    int result=infinity_std_entropy(output,length);
    if (result) { errno=infinity_native_error(result); return -1; }
    return 0;
}
// ------------------------=
// FUNC: sysconf
// DESC: Reports the native page geometry and single-owner executor width; rejects other host queries.
// ------------------=
long sysconf(int name) {
    if (name==_SC_PAGESIZE) return 4096;
#ifdef _SC_NPROCESSORS_ONLN
    if (name==_SC_NPROCESSORS_ONLN) return 1;
#endif
    errno=EINVAL; return -1;
}
// ------------------------=
// FUNC: _gettimeofday
// DESC: Converts only a supplied native UTC clock and propagates unavailable time without fabrication.
// ------------------=
int _gettimeofday(struct timeval *out,void *zone) {
    if (!out || zone) { errno=EINVAL; return -1; }
    struct timespec now; if (clock_gettime(CLOCK_REALTIME,&now)) return -1;
    out->tv_sec=now.tv_sec; out->tv_usec=now.tv_nsec/1000; return 0;
}
// ------------------------=
// FUNC: readlink
// DESC: Denies Unix symlink discovery; native timezone and object adapters must not inspect host paths.
// ------------------=
ssize_t readlink(const char *path,char *out,size_t size) { (void)path;(void)out;(void)size;errno=ENOTSUP;return -1; }
// ------------------------=
// FUNC: _open
// DESC: Denies ambient filesystem access; browser site data must use the native object adapter.
// ------------------=
int _open(const char *path,int flags,...) { (void)path;(void)flags;errno=ENOTSUP;return -1; }
// ------------------------=
// FUNC: _close
// DESC: Rejects unissued Unix descriptor identities.
// ------------------=
int _close(int fd) { (void)fd;errno=EBADF;return -1; }
// ------------------------=
// FUNC: _read
// DESC: Rejects reads from unissued Unix descriptor identities without changing buffers.
// ------------------=
_READ_WRITE_RETURN_TYPE _read(int fd,void *buffer,size_t size) { (void)fd;(void)buffer;(void)size;errno=EBADF;return -1; }
// ------------------------=
// FUNC: _write
// DESC: Rejects writes to unissued descriptors rather than silently discarding data as success.
// ------------------=
_READ_WRITE_RETURN_TYPE _write(int fd,const void *buffer,size_t size) { (void)fd;(void)buffer;(void)size;errno=EBADF;return -1; }
// ------------------------=
// FUNC: _lseek
// DESC: Rejects seeking an unissued native descriptor.
// ------------------=
off_t _lseek(int fd,off_t offset,int whence) { (void)fd;(void)offset;(void)whence;errno=EBADF;return -1; }
// ------------------------=
// FUNC: _fstat
// DESC: Rejects metadata queries on unissued descriptors without inventing filesystem metadata.
// ------------------=
int _fstat(int fd,struct stat *out) { (void)fd;(void)out;errno=EBADF;return -1; }
// ------------------------=
// FUNC: _isatty
// DESC: Reports no Unix terminal for any descriptor.
// ------------------=
int _isatty(int fd) { (void)fd;errno=EBADF;return 0; }
// ------------------------=
// FUNC: _unlink
// DESC: Denies mutation of Unix paths; object deletion requires native authority.
// ------------------=
int _unlink(const char *path) { (void)path;errno=ENOTSUP;return -1; }
// ------------------------=
// FUNC: _getpid
// DESC: Rejects host process identity because the native runtime is not a Unix process.
// ------------------=
int _getpid(void) { errno=ENOTSUP;return -1; }
// ------------------------=
// FUNC: _kill
// DESC: Denies Unix signal delivery; cancellation uses the native service boundary.
// ------------------=
int _kill(int pid,int signal) { (void)pid;(void)signal;errno=ENOTSUP;return -1; }
// ------------------------=
// FUNC: _exit
// DESC: Traps an unrecoverable runtime termination instead of returning or calling a host process.
// ------------------=
__attribute__((noreturn)) void _exit(int status) { (void)status; __builtin_trap(); }
