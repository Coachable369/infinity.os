#ifndef INFINITY_COMPILER_STAT_H
#define INFINITY_COMPILER_STAT_H
#include_next <sys/stat.h>
#ifdef __cplusplus
extern "C" {
#endif
int lstat(const char *, struct stat *);
#ifdef __cplusplus
}
#endif
#endif
