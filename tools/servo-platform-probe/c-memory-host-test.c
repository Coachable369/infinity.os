/* Deterministic owner/yield/failure injection around the production mmap ABI. */
#include <assert.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stddef.h>

/* Never interpose the host process's own virtual-memory implementation. */
#define mmap infinity_test_mmap
#define munmap infinity_test_munmap
#define mprotect infinity_test_mprotect
#define madvise infinity_test_madvise
#include "../../sdk/servo-std/c-memory.c"

typedef struct { void *pointer; size_t size, align; } Allocation;
static Allocation allocations[2048];
static size_t outstanding, allocation_calls, fail_at, yield_at;
static uint64_t owner=1;
static unsigned char *nested;

// ------------------------=
// FUNC: infinity_std_thread_id
// DESC: Models the production allocator's exclusive owner-thread authorization.
// ------------------=
uint64_t infinity_std_thread_id(void) { return owner; }

// ------------------------=
// FUNC: infinity_std_allocate
// DESC: Injects allocation failure and a cooperative mapping call before returning an owned allocation.
// ------------------=
void *infinity_std_allocate(size_t size,size_t align) {
    ++allocation_calls;
    if (allocation_calls==fail_at) return 0;
    void *pointer=0;
    assert(!posix_memalign(&pointer,align<sizeof(void*)?sizeof(void*):align,size));
    memset(pointer,0xa5,size);
    size_t slot=0;
    while (slot<2048 && allocations[slot].pointer) ++slot;
    assert(slot<2048);
    allocations[slot]=(Allocation){pointer,size,align};
    ++outstanding;
    if (allocation_calls==yield_at) {
        yield_at=0;
        nested=mmap(0,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);
        assert(nested!=MAP_FAILED);
        nested[0]=0x6e;
    }
    return pointer;
}

// ------------------------=
// FUNC: infinity_std_deallocate
// DESC: Requires exact allocator ownership and layout while checking all failed mappings reclaim their backing.
// ------------------=
void infinity_std_deallocate(void *pointer,size_t size,size_t align) {
    size_t slot=0;
    while (slot<2048 && allocations[slot].pointer!=pointer) ++slot;
    assert(slot<2048 && allocations[slot].size==size && allocations[slot].align==align);
    allocations[slot]=(Allocation){0};
    --outstanding;
    free(pointer);
}

// ------------------------=
// FUNC: concurrent_mappings
// DESC: Requires 128 live independent mappings, mixed-order release, zeroed reuse and complete reclamation.
// ------------------=
static void concurrent_mappings(void) {
    unsigned char *live[128];
    for (unsigned pass=0;pass<3;++pass) {
        for (unsigned i=0;i<128;++i) {
            live[i]=mmap(0,8192,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);
            assert(live[i]!=MAP_FAILED && (uintptr_t)live[i]%4096==0);
            for (size_t byte=0;byte<8192;++byte) assert(live[i][byte]==0);
            live[i][0]=(unsigned char)i; live[i][8191]=(unsigned char)(255-i);
        }
        for (unsigned parity=0;parity<2;++parity) {
            for (unsigned i=parity;i<128;i+=2) {
                assert(live[i][0]==i && live[i][8191]==255-i);
                assert(munmap(live[i]+4096,4096)==0);
                assert(mprotect(live[i]+4096,4096,PROT_READ|PROT_WRITE)==-1 && errno==EINVAL);
                assert(munmap(live[i],4096)==0);
            }
        }
        assert(outstanding==0 && regions==0);
    }
}

// ------------------------=
// FUNC: interleaved_mapping
// DESC: Simulates another native fiber creating a mapping at both allocation safepoints and preserves both owners.
// ------------------=
static void interleaved_mapping(void) {
    for (size_t point=1;point<=2;++point) {
        yield_at=allocation_calls+point;
        nested=0;
        unsigned char *outer=mmap(0,8192,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);
        assert(outer!=MAP_FAILED && nested && outer!=nested && nested[0]==0x6e);
        assert(mprotect(nested,4096,PROT_READ|PROT_WRITE)==0);
        assert(munmap(outer,8192)==0 && munmap(nested,4096)==0);
        assert(outstanding==0 && regions==0);
    }
}

// ------------------------=
// FUNC: failure_reclaims
// DESC: Rejects backing or metadata allocation failure, invalid ownership and executable requests without leaks.
// ------------------=
static void failure_reclaims(void) {
    for (size_t point=1;point<=2;++point) {
        fail_at=allocation_calls+point;
        assert(mmap(0,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0)==MAP_FAILED);
        assert(errno==ENOMEM && outstanding==0 && regions==0);
        fail_at=0;
    }
    owner=0;
    assert(mmap(0,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0)==MAP_FAILED && errno==EINVAL);
    owner=1;
    assert(mmap(0,4096,PROT_EXEC,MAP_PRIVATE|MAP_ANONYMOUS,-1,0)==MAP_FAILED && errno==ENOTSUP);
    assert(mmap(0,SIZE_MAX,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0)==MAP_FAILED && errno==EINVAL);
    assert(outstanding==0 && regions==0);
}

// ------------------------=
// FUNC: main
// DESC: Exercises production memory-adapter behavior without claiming installed-browser or CAPTCHA acceptance.
// ------------------=
int main(void) {
    concurrent_mappings();
    interleaved_mapping();
    failure_reclaims();
    return 0;
}
