#ifndef INFINITY_STT_MMAN_H
#define INFINITY_STT_MMAN_H
#include <stddef.h>
#include <sys/types.h>
#define PROT_READ 1
#define MAP_SHARED 1
#define MAP_PRIVATE 2
#define MAP_FAILED ((void *)-1)
void *mmap(void *, size_t, int, int, int, off_t);
int munmap(void *, size_t);
#endif
