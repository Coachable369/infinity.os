#include "../../sdk/servo-std/include/sys/mman.h"
#include <stdint.h>
#include <errno.h>
#include <unistd.h>
extern int _open(const char *,int,...);
extern ssize_t _read(int,void *,size_t);
extern ssize_t _write(int,const void *,size_t);
// ------------------------=
// FUNC: infinity_c_memory_test
// DESC: Exercises real zeroed backing, split release, holes, protection denial and reclaim cycles.
// ------------------=
int infinity_c_memory_test(void) {
    for (unsigned cycle=0;cycle<70;++cycle) {
        unsigned char *p=mmap(0,12288,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);
        if (p==MAP_FAILED || (uintptr_t)p%4096) return 1;
        for (unsigned i=0;i<12288;++i) { if (p[i]) return 2; p[i]=93; }
        if (madvise(p,4096,MADV_DONTNEED) || p[0] || p[4095] || p[4096]!=93) return 3;
        if (mprotect(p,4096,PROT_EXEC)!=-1 || errno!=ENOTSUP) return 4;
        if (munmap(p+4096,4096)) return 5;
        if (madvise(p,12288,MADV_DONTNEED)!=-1 || errno!=EINVAL) return 6;
        if (munmap(p,4096) || munmap(p+8192,4096)) return 7;
        if (munmap(p,4096)!=-1) return 8;
    }
    if (mmap(0,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE,4,0)!=MAP_FAILED || errno!=ENOTSUP) return 9;
    if (mmap(0,SIZE_MAX,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0)!=MAP_FAILED || errno!=EINVAL) return 10;
    if (sysconf(_SC_PAGESIZE)!=4096) return 11;
    if (_open("/etc/passwd",0)!=-1 || errno!=ENOTSUP) return 12;
    char buffer=42;
    if (_read(0,&buffer,1)!=-1 || errno!=EBADF || buffer!=42) return 13;
    if (_write(1,&buffer,1)!=-1 || errno!=EBADF) return 14;
    return 0;
}
