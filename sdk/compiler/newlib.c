/* Newlib's machine ports differ only in their low-level symbol spelling.
 * Both spellings reach the same capability-governed native service bridge. */
#include <unistd.h>
#include <signal.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <stddef.h>
#include <fcntl.h>

// ------------------------=
// FUNC: _open
// DESC: Routes Newlib opens through the same launch-bound Object Store grants.
// ------------------=
int _open(const char *path, int flags, int mode) { return open(path, flags, mode); }

// ------------------------=
// FUNC: _close
// DESC: Adapts Newlib's descriptor close to the native compiler file bridge.
// ------------------=
int _close(int fd) { return close(fd); }
// ------------------------=
// FUNC: _lseek
// DESC: Preserves native seek results and errno for Newlib's reentrant wrapper.
// ------------------=
off_t _lseek(int fd, off_t offset, int origin) { return lseek(fd, offset, origin); }
// ------------------------=
// FUNC: _read
// DESC: Reads only through the launch-bound native file or console grant.
// ------------------=
ssize_t _read(int fd, void *out, size_t size) { return read(fd, out, size); }
// ------------------------=
// FUNC: _write
// DESC: Writes only through the launch-bound native file or console grant.
// ------------------=
ssize_t _write(int fd, const void *data, size_t size) { return write(fd, data, size); }
// ------------------------=
// FUNC: _sbrk
// DESC: Keeps the ARM Newlib allocator inside the same granted private heap.
// ------------------=
void *_sbrk(ptrdiff_t increment) { return sbrk(increment); }
// ------------------------=
// FUNC: _fstat
// DESC: Retrieves native object metadata without introducing host filesystem access.
// ------------------=
int _fstat(int fd, struct stat *out) { return fstat(fd, out); }
// ------------------------=
// FUNC: _isatty
// DESC: Reports the existing native console endpoint terminal status.
// ------------------=
int _isatty(int fd) { return isatty(fd); }
// ------------------------=
// FUNC: _kill
// DESC: Preserves the native process bridge's restricted signal semantics.
// ------------------=
int _kill(pid_t process, int signal_number) { return kill(process, signal_number); }
// ------------------------=
// FUNC: _getpid
// DESC: Returns the launch-assigned native execution identity.
// ------------------=
pid_t _getpid(void) { return getpid(); }
// ------------------------=
// FUNC: _gettimeofday
// DESC: Delegates wall-clock reads to the native clock bridge.
// ------------------=
int _gettimeofday(struct timeval *out, void *zone) { return gettimeofday(out, zone); }
// ------------------------=
// FUNC: _unlink
// DESC: Removes a native namespace entry using the existing authorized implementation.
// ------------------=
int _unlink(const char *path) { return unlink(path); }
// ------------------------=
// FUNC: _link
// DESC: Creates a native alias using the existing authorized implementation.
// ------------------=
int _link(const char *source, const char *destination) { return link(source, destination); }
