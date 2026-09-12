#ifndef INFINITY_SYS_MMAN_H
#define INFINITY_SYS_MMAN_H
#include <stddef.h>
#include <sys/types.h>
#define PROT_NONE 0
#define PROT_READ 1
#define PROT_WRITE 2
#define PROT_EXEC 4
#define MAP_SHARED 1
#define MAP_PRIVATE 2
#define MAP_FIXED 16
#define MAP_ANONYMOUS 32
#define MAP_ANON MAP_ANONYMOUS
#define MAP_FAILED ((void *)-1)
#define MS_ASYNC 1
#define MS_INVALIDATE 2
#define MS_SYNC 4
#define MADV_DONTNEED 1
#define MADV_WILLNEED 2
#define MADV_RANDOM 3
#ifdef __cplusplus
extern "C" {
#endif
void *mmap(void *, size_t, int, int, int, off_t);
int munmap(void *, size_t);
int mprotect(void *, size_t, int);
int msync(void *, size_t, int);
int madvise(void *, size_t, int);
#ifdef __cplusplus
}
#endif
#endif
