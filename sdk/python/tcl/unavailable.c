#include "tclInt.h"
#include <infinity/compiler_host.h>

// ------------------------=
// FUNC: unsupported
// DESC: Reports a structured unsupported operation without creating a process, pipe or network authority.
// ------------------=
static int unsupported(Tcl_Interp *interp) {
    errno = ENOTSUP;
    if (interp) {
        Tcl_SetObjResult(interp, Tcl_NewStringObj("This operation has no authorized native Tcl provider", -1));
        Tcl_SetErrorCode(interp, "INFINITY", "UNSUPPORTED", NULL);
    }
    return TCL_ERROR;
}
// ------------------------=
// FUNC: getsockname
// DESC: Rejects socket introspection because native Tcl owns no raw socket descriptors.
// ------------------=
int getsockname(int descriptor, struct sockaddr *address, socklen_t *length) {
    (void)descriptor; (void)address; (void)length;
    errno = ENOTSOCK; return -1;
}
// ------------------------=
// FUNC: mknod
// DESC: Refuses device-node creation rather than bypassing native device capability ownership.
// ------------------=
int mknod(const char *path, mode_t mode, dev_t device) {
    (void)path; (void)mode; (void)device;
    errno = ENOTSUP; return -1;
}
// ------------------------=
// FUNC: geteuid
// DESC: Reports unavailable Unix numeric identity; native authorization remains capability based.
// ------------------=
uid_t geteuid(void) { errno = ENOTSUP; return (uid_t)-1; }
// ------------------------=
// FUNC: chown
// DESC: Rejects Unix owner mutation instead of silently changing native object ownership.
// ------------------=
int chown(const char *path, uid_t owner, gid_t group) {
    (void)path; (void)owner; (void)group; errno = ENOTSUP; return -1;
}
// ------------------------=
// FUNC: mkfifo
// DESC: Rejects FIFO creation until a scoped native pipe service exists.
// ------------------=
int mkfifo(const char *path, mode_t mode) { (void)path; (void)mode; errno = ENOTSUP; return -1; }
// ------------------------=
// FUNC: TclWinCPUID
// DESC: Executes the upstream CPU observation contract on x86; other architectures report unsupported without fabricated registers.
// ------------------=
int TclWinCPUID(unsigned int index, unsigned int *registers) {
#if defined(__x86_64__)
    unsigned int a, b, c, d;
    __asm__ volatile("cpuid" : "=a"(a), "=b"(b), "=c"(c), "=d"(d) : "a"(index), "c"(0));
    registers[0] = a; registers[1] = b; registers[2] = c; registers[3] = d;
    return TCL_OK;
#else
    (void)index; (void)registers; return TCL_ERROR;
#endif
}
// ------------------------=
// FUNC: TclpHasSockets
// DESC: Rejects general Tcl sockets; the separate scoped pip HTTPS service is not ambient socket authority.
// ------------------=
int TclpHasSockets(Tcl_Interp *interp) { return unsupported(interp); }
// ------------------------=
// FUNC: TclpFinalizeSockets
// DESC: Has no socket resources to finalize because unsupported creation fails atomically.
// ------------------=
void TclpFinalizeSockets(void) { }
// ------------------------=
// FUNC: Tcl_GetHostName
// DESC: Returns only the native launcher's observed node name, never the build host's name.
// ------------------=
const char *Tcl_GetHostName(void) {
    const InfinityCompilerHost *host = infinity_compiler_get_host();
    return host && host->node_name ? host->node_name : "";
}
// ------------------------=
// FUNC: Tcl_OpenTcpClient
// DESC: Rejects ungranted raw client networking with a Tcl error.
// ------------------=
Tcl_Channel Tcl_OpenTcpClient(Tcl_Interp *interp, int port, const char *host,
    const char *local, int local_port, int async) {
    (void)port; (void)host; (void)local; (void)local_port; (void)async;
    unsupported(interp); return NULL;
}
// ------------------------=
// FUNC: Tcl_OpenTcpServer
// DESC: Rejects ungranted listening sockets instead of publishing an unusable channel.
// ------------------=
Tcl_Channel Tcl_OpenTcpServer(Tcl_Interp *interp, int port, const char *host,
    Tcl_TcpAcceptProc *proc, ClientData data) {
    (void)port; (void)host; (void)proc; (void)data; unsupported(interp); return NULL;
}
// ------------------------=
// FUNC: Tcl_MakeTcpClientChannel
// DESC: Rejects import of a raw socket into an interpreter without a socket provider.
// ------------------=
Tcl_Channel Tcl_MakeTcpClientChannel(ClientData socket) { (void)socket; unsupported(NULL); return NULL; }
// ------------------------=
// FUNC: TclpMakeTcpClientChannelMode
// DESC: Preserves fail-closed behavior for Tcl's internal socket-channel entrypoint.
// ------------------=
void *TclpMakeTcpClientChannelMode(void *socket, int mode) { (void)mode; return Tcl_MakeTcpClientChannel(socket); }
// ------------------------=
// FUNC: TclSockGetPort
// DESC: Parses numeric ports without performing an unsupported service-database lookup.
// ------------------=
int TclSockGetPort(Tcl_Interp *interp, const char *text, const char *protocol, int *port) {
    (void)protocol;
    if (Tcl_GetInt(interp, text, port) != TCL_OK || *port < 0 || *port > 65535) return TCL_ERROR;
    return TCL_OK;
}
// ------------------------=
// FUNC: TclSockMinimumBuffers
// DESC: Rejects socket options without a native socket handle.
// ------------------=
int TclSockMinimumBuffers(void *socket, int size) { (void)socket; (void)size; unsupported(NULL); return TCL_ERROR; }
// ------------------------=
// FUNC: TclpGetHostByName
// DESC: Rejects ambient DNS outside the native network service authority.
// ------------------=
struct hostent *TclpGetHostByName(const char *name) { (void)name; unsupported(NULL); return NULL; }
// ------------------------=
// FUNC: TclpGetHostByAddr
// DESC: Rejects reverse DNS outside the native network service authority.
// ------------------=
struct hostent *TclpGetHostByAddr(const char *addr, int length, int type) {
    (void)addr; (void)length; (void)type; unsupported(NULL); return NULL;
}
// ------------------------=
// FUNC: inet_ntoa
// DESC: Formats an address value without using a socket or host service.
// ------------------=
char *inet_ntoa(struct in_addr address) {
    static char buffer[16];
    const unsigned char *b = (const unsigned char *)&address.s_addr;
    snprintf(buffer, sizeof(buffer), "%u.%u.%u.%u", b[0], b[1], b[2], b[3]); return buffer;
}
// ------------------------=
// FUNC: TclpCreatePipe
// DESC: Rejects Unix pipelines until a native pipe provider is authorized.
// ------------------=
int TclpCreatePipe(TclFile *read_file, TclFile *write_file) {
    *read_file = NULL; *write_file = NULL; unsupported(NULL); return 0;
}
// ------------------------=
// FUNC: Tcl_CreatePipe
// DESC: Reports that the Tcl channel-pipe operation has no native implementation yet.
// ------------------=
int Tcl_CreatePipe(Tcl_Interp *interp, Tcl_Channel *read_channel, Tcl_Channel *write_channel, int flags) {
    (void)flags; *read_channel = NULL; *write_channel = NULL; return unsupported(interp);
}
// ------------------------=
// FUNC: TclpCreateProcess
// DESC: Rejects ambient fork and exec rather than bypassing InfinityOS application lifecycle authorization.
// ------------------=
int TclpCreateProcess(Tcl_Interp *interp, int argc, const char **argv, TclFile input,
    TclFile output, TclFile error, Tcl_Pid *pid) {
    (void)argc; (void)argv; (void)input; (void)output; (void)error; *pid = NULL; return unsupported(interp);
}
// ------------------------=
// FUNC: TclpCreateCommandChannel
// DESC: Returns no channel when no native subprocess was created.
// ------------------=
Tcl_Channel TclpCreateCommandChannel(TclFile input, TclFile output, TclFile error, int count, Tcl_Pid *pids) {
    (void)input; (void)output; (void)error; (void)count; (void)pids; unsupported(NULL); return NULL;
}
// ------------------------=
// FUNC: TclpMakeFile
// DESC: Obtains an existing authorized descriptor without duplicating its authority.
// ------------------=
TclFile TclpMakeFile(Tcl_Channel channel, int direction) {
    ClientData handle;
    if (Tcl_GetChannelHandle(channel, direction, &handle) != TCL_OK) return NULL;
    return (TclFile)((intptr_t)handle + 1);
}
// ------------------------=
// FUNC: TclpOpenFile
// DESC: Opens only a file accepted by the current native namespace provider.
// ------------------=
TclFile TclpOpenFile(const char *name, int mode) {
    int descriptor = open(name, mode, 0600);
    return descriptor < 0 ? NULL : (TclFile)(intptr_t)(descriptor + 1);
}
// ------------------------=
// FUNC: TclpCloseFile
// DESC: Closes an owned native descriptor with Tcl's biased-handle convention.
// ------------------=
int TclpCloseFile(TclFile file) { return close((int)(intptr_t)file - 1); }
// ------------------------=
// FUNC: TclpCreateTempFile
// DESC: Rejects implicit temporary-file creation until an explicit work directory is supplied.
// ------------------=
TclFile TclpCreateTempFile(const char *contents) { (void)contents; unsupported(NULL); return NULL; }
// ------------------------=
// FUNC: TclpTempFileName
// DESC: Does not invent writable paths or fall back to a host temporary directory.
// ------------------=
Tcl_Obj *TclpTempFileName(void) { unsupported(NULL); return NULL; }
// ------------------------=
// FUNC: TclpTempFileNameForLibrary
// DESC: Rejects extraction of native libraries into ungranted temporary paths.
// ------------------=
Tcl_Obj *TclpTempFileNameForLibrary(Tcl_Interp *interp, Tcl_Obj *path) {
    (void)path; unsupported(interp); return NULL;
}
// ------------------------=
// FUNC: TclGetAndDetachPids
// DESC: Reports unsupported process detachment instead of losing native lifecycle ownership.
// ------------------=
void TclGetAndDetachPids(Tcl_Interp *interp, Tcl_Channel channel) { (void)channel; unsupported(interp); }
// ------------------------=
// FUNC: Tcl_WaitPid
// DESC: Rejects waiting on Unix child identities that this interpreter cannot create.
// ------------------=
Tcl_Pid Tcl_WaitPid(Tcl_Pid pid, int *status, int options) {
    (void)pid; (void)status; (void)options; errno = ECHILD; return (Tcl_Pid)(intptr_t)-1;
}
// ------------------------=
// FUNC: Tcl_PidObjCmd
// DESC: Reports the real native context identity for pid and rejects Unix pipe process queries.
// ------------------=
int Tcl_PidObjCmd(ClientData unused, Tcl_Interp *interp, int count, Tcl_Obj *const *args) {
    (void)unused;
    if (count != 1) { Tcl_WrongNumArgs(interp, 1, args, ""); return TCL_ERROR; }
    pid_t pid = getpid();
    if (pid < 0) return unsupported(interp);
    Tcl_SetObjResult(interp, Tcl_NewLongObj(pid)); return TCL_OK;
}
// ------------------------=
// FUNC: TclpFinalizePipes
// DESC: Has no native pipe handles to release because creation fails atomically.
// ------------------=
void TclpFinalizePipes(void) { }
