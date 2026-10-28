#ifndef INFINITY_TCL_GRP_H
#define INFINITY_TCL_GRP_H
#include <sys/types.h>
struct group { char *gr_name, *gr_passwd; gid_t gr_gid; char **gr_mem; };
#endif
