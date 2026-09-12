#ifndef INFINITY_COMPILER_RESOURCE_H
#define INFINITY_COMPILER_RESOURCE_H
#include <sys/time.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef uint64_t rlim_t;
struct rlimit { rlim_t rlim_cur, rlim_max; };
struct rusage { struct timeval ru_utime, ru_stime; };
#define RUSAGE_SELF 0
#define RUSAGE_CHILDREN -1
#define RUSAGE_THREAD 1
#define RLIMIT_CORE 1
#define RLIMIT_DATA 2
#define RLIMIT_STACK 3
int getrlimit(int, struct rlimit *);
int setrlimit(int, const struct rlimit *);
int getrusage(int, struct rusage *);
#ifdef __cplusplus
}
#endif
#endif
