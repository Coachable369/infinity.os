#ifndef INFINITY_NATIVE_POOL_H
#define INFINITY_NATIVE_POOL_H
#include <stdint.h>
typedef uint64_t (*InfinityPoolAllocate)(void *,uint64_t,uint64_t);
// ------------------------=
// FUNC: infinity_reserve_process_pool
// DESC: Reserves the largest bounded arena that fits; optional native processes must not prevent a low-memory OS from booting.
// ------------------=
static inline uint64_t infinity_reserve_process_pool(InfinityPoolAllocate allocate,void *context,
    uint64_t maximum,uint64_t *bytes) {
    *bytes=0;
    for(uint64_t size=UINT64_C(256)*1024*1024;size>=UINT64_C(64)*1024*1024;size/=2) {
        uint64_t base=allocate(context,size,maximum);
        if(base) { *bytes=size; return base; }
    }
    return 0;
}
#endif
