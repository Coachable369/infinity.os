#ifndef INFINITY_COMPILER_WAIT_H
#define INFINITY_COMPILER_WAIT_H
#include_next <sys/wait.h>
#include <sys/resource.h>
#ifdef __cplusplus
extern "C" {
#endif
pid_t wait4(pid_t, int *, int, struct rusage *);
#ifdef __cplusplus
}
#endif
#endif
