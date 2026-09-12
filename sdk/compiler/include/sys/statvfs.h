#ifndef INFINITY_COMPILER_STATVFS_H
#define INFINITY_COMPILER_STATVFS_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Compatibility projection only; no mount table or Unix filesystem exists. */
struct statvfs { uint64_t f_frsize, f_blocks, f_bfree, f_bavail, f_flags; };
#define MNT_LOCAL 1
int statvfs(const char *, struct statvfs *);
int fstatvfs(int, struct statvfs *);
#ifdef __cplusplus
}
#endif
#endif
