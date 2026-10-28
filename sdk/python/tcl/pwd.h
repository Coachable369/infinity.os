#ifndef INFINITY_TCL_PWD_H
#define INFINITY_TCL_PWD_H
#include <sys/types.h>
/* Account database operations are explicitly unsupported in this Tcl port;
 * these layouts exist for Tcl's internal portability contract, not authority. */
struct passwd { char *pw_name, *pw_passwd; uid_t pw_uid; gid_t pw_gid;
    char *pw_gecos, *pw_dir, *pw_shell; };
#endif
