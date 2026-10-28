#include "tclInt.h"
#include <locale.h>
#include <infinity/compiler_host.h>

// ------------------------=
// FUNC: TclpInitPlatform
// DESC: Selects slash-path Tcl semantics without fabricating a Unix kernel or changing native signals.
// ------------------=
void TclpInitPlatform(void) {
    tclPlatform = TCL_PLATFORM_UNIX;
    setlocale(LC_NUMERIC, "C");
}
// ------------------------=
// FUNC: TclpInitLibraryPath
// DESC: Uses only the bundled immutable system Tcl library as the initial search path.
// ------------------=
void TclpInitLibraryPath(char **value, int *length, Tcl_Encoding *encoding) {
    const char *path = TCL_LIBRARY;
    *length = (int)strlen(path);
    *value = ckalloc((unsigned)*length + 1);
    memcpy(*value, path, (size_t)*length + 1);
    *encoding = Tcl_GetEncoding(NULL, "utf-8");
}
// ------------------------=
// FUNC: TclpSetInitialEncodings
// DESC: Matches the native object namespace's UTF-8 encoding without reading host locale state.
// ------------------=
void TclpSetInitialEncodings(void) { Tcl_SetSystemEncoding(NULL, "utf-8"); }
// ------------------------=
// FUNC: Tcl_GetEncodingNameFromEnvironment
// DESC: Reports the native UTF-8 contract rather than an unavailable host environment locale.
// ------------------=
const char *Tcl_GetEncodingNameFromEnvironment(Tcl_DString *buffer) {
    Tcl_DStringInit(buffer);
    return Tcl_DStringAppend(buffer, "utf-8", -1);
}
// ------------------------=
// FUNC: TclpSetVariables
// DESC: Publishes truthful InfinityOS runtime identity and the bundled Tcl library location.
// ------------------=
void TclpSetVariables(Tcl_Interp *interp) {
    Tcl_SetVar2(interp, "tclDefaultLibrary", NULL, TCL_LIBRARY, TCL_GLOBAL_ONLY);
    Tcl_SetVar2(interp, "tcl_pkgPath", NULL, TCL_PACKAGE_PATH, TCL_GLOBAL_ONLY);
    Tcl_SetVar2(interp, "tcl_platform", "platform", "unix", TCL_GLOBAL_ONLY);
    Tcl_SetVar2(interp, "tcl_platform", "os", "InfinityOS", TCL_GLOBAL_ONLY);
    Tcl_SetVar2(interp, "tcl_platform", "osVersion", "1", TCL_GLOBAL_ONLY);
    Tcl_SetVar2(interp, "tcl_platform", "threaded", "0", TCL_GLOBAL_ONLY);
    Tcl_SetVar2(interp, "tcl_platform", "wordSize", "8", TCL_GLOBAL_ONLY);
    Tcl_SetVar2(interp, "tcl_platform", "pointerSize", "8", TCL_GLOBAL_ONLY);
    Tcl_SetVar2(interp, "tcl_platform", "byteOrder", "littleEndian", TCL_GLOBAL_ONLY);
#if defined(__aarch64__)
    Tcl_SetVar2(interp, "tcl_platform", "machine", "aarch64", TCL_GLOBAL_ONLY);
#else
    Tcl_SetVar2(interp, "tcl_platform", "machine", "x86_64", TCL_GLOBAL_ONLY);
#endif
}
// ------------------------=
// FUNC: TclpFindVariable
// DESC: Locates only the execution context's local environment entry using exact variable boundaries.
// ------------------=
int TclpFindVariable(const char *name, int *length) {
    size_t size = strlen(name);
    int index = 0;
    if (environ) for (; environ[index]; ++index) {
        if (!strncmp(environ[index], name, size) && environ[index][size] == '=') {
            *length = (int)size;
            return index;
        }
    }
    *length = index;
    return -1;
}
// ------------------------=
// FUNC: TclUnixSetBlockingMode
// DESC: Changes native descriptor flags only through the existing capability-scoped file provider.
// ------------------=
int TclUnixSetBlockingMode(int descriptor, int mode) {
    int flags = fcntl(descriptor, F_GETFL);
    if (flags < 0) return -1;
    return fcntl(descriptor, F_SETFL,
        mode == TCL_MODE_BLOCKING ? flags & ~O_NONBLOCK : flags | O_NONBLOCK);
}
// ------------------------=
// FUNC: TclpGetPwNam
// DESC: Rejects ambient Unix account lookup; native identity remains outside Tcl's account database ABI.
// ------------------=
struct passwd *TclpGetPwNam(const char *name) { (void)name; errno = ENOTSUP; return NULL; }
// ------------------------=
// FUNC: TclpGetPwUid
// DESC: Rejects Unix UID lookup without inventing user identities.
// ------------------=
struct passwd *TclpGetPwUid(uid_t uid) { (void)uid; errno = ENOTSUP; return NULL; }
// ------------------------=
// FUNC: TclpGetGrNam
// DESC: Rejects Unix group lookup instead of granting authority through a fabricated group.
// ------------------=
struct group *TclpGetGrNam(const char *name) { (void)name; errno = ENOTSUP; return NULL; }
// ------------------------=
// FUNC: TclpGetGrGid
// DESC: Rejects numeric Unix group lookup in the capability-based native runtime.
// ------------------=
struct group *TclpGetGrGid(gid_t gid) { (void)gid; errno = ENOTSUP; return NULL; }
