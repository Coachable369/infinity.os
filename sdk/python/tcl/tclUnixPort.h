#ifndef INFINITY_TCL_PORT_H
#define INFINITY_TCL_PORT_H
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <sys/wait.h>
#include <time.h>
#include <stdint.h>
#include <inttypes.h>
#include <unistd.h>
#include <dirent.h>
#include <utime.h>
#include <limits.h>
#include <float.h>
#include <pwd.h>
#include <grp.h>

typedef struct dirent Tcl_DirEntry;
typedef DIR TclDIR;
typedef off_t Tcl_SeekOffset;
#define TclOSreaddir readdir
#define TclOSopendir opendir
#define TclOSrewinddir rewinddir
#define TclOSclosedir closedir
#define TclOSseek lseek
#define TclOSopen open
#define TclOSstat stat
#define TclOSlstat lstat
#define WAIT_STATUS_TYPE int
#define MAXPATHLEN 256
#define MAXNAMLEN 255
#define TCL_PLATFORM_TRANSLATION TCL_TRANSLATE_LF
#define TclpReleaseFile(file)
#define TclpSysAlloc(size, isBin) malloc((size_t)(size))
#define TclpSysFree(ptr) free((char *)(ptr))
#define TclpSysRealloc(ptr, size) realloc((char *)(ptr), (size_t)(size))
#define TclpExit exit

/* Tcl's exported internal-stub ABI includes these opaque socket types even
 * when the platform intentionally does not provide general socket authority. */
struct in_addr { uint32_t s_addr; };
typedef uint32_t socklen_t;
struct sockaddr { uint16_t sa_family; unsigned char sa_data[14]; };
#define AF_UNSPEC 0
#define AF_INET 2
#define AF_INET6 10
extern int getsockname(int descriptor, struct sockaddr *address, socklen_t *length);
extern int mknod(const char *path, mode_t mode, dev_t device);
struct hostent;
struct addrinfo;
extern char **environ;
extern int TclUnixSetBlockingMode(int fd, int mode);
extern struct passwd *TclpGetPwNam(const char *name);
extern struct passwd *TclpGetPwUid(uid_t uid);
extern struct group *TclpGetGrNam(const char *name);
extern struct group *TclpGetGrGid(gid_t gid);
extern struct hostent *TclpGetHostByName(const char *name);
extern struct hostent *TclpGetHostByAddr(const char *addr, int length, int type);
extern void *TclpMakeTcpClientChannelMode(void *socket, int mode);
extern char *inet_ntoa(struct in_addr address);
#endif
