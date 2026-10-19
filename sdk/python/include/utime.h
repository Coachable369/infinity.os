#ifndef INFINITY_PYTHON_UTIME_H
#define INFINITY_PYTHON_UTIME_H
#include <sys/types.h>
#include_next <utime.h>
int utime(const char *, const struct utimbuf *);
#endif
