/* Private anonymous RW backing only. No executable/protected/file mappings. */
#include "include/sys/mman.h"
#include <stdint.h>
#include <errno.h>
#include <string.h>
extern void *infinity_std_allocate(size_t,size_t);
extern void infinity_std_deallocate(void *,size_t,size_t);
extern uint64_t infinity_std_thread_id(void);
#define PAGE 4096u
#define MAX_PAGES 4096u
typedef struct Region {
    struct Region *next;
    unsigned char *base;
    size_t length, remaining;
    uint64_t live[];
} Region;
/* The serialized native executor owns this list. Allocation/deallocation may
 * yield, so only fully initialized regions are published and retired regions
 * are unlinked before either callback. Metadata shares the governed heap. */
static Region *regions;
// ------------------------=
// FUNC: rounded_length
// DESC: Rejects zero and overflow before rounding anonymous regions to native page boundaries.
// ------------------=
static size_t rounded_length(size_t length) { return length && length<=SIZE_MAX-(PAGE-1) ? (length+PAGE-1)&~(size_t)(PAGE-1) : 0; }
// ------------------------=
// FUNC: region_bytes
// DESC: Sizes page ownership metadata for the actual bounded mapping rather than reserving a fixed descriptor table.
// ------------------=
static size_t region_bytes(size_t length) { return sizeof(Region)+((length/PAGE+63)/64)*sizeof(uint64_t); }
// ------------------------=
// FUNC: region_range
// DESC: Validates an aligned owned range and rejects already unmapped holes before changing state.
// ------------------=
static Region *region_range(void *address,size_t length,size_t *first,size_t *pages) {
    uintptr_t start=(uintptr_t)address; size_t rounded=rounded_length(length);
    if (!infinity_std_thread_id() || start%PAGE || !rounded || start>UINTPTR_MAX-rounded) return 0;
    for (Region *r=regions;r;r=r->next) {
        uintptr_t base=(uintptr_t)r->base;
        if (start<base || start-base>r->length || rounded>r->length-(start-base)) continue;
        *first=(start-base)/PAGE; *pages=rounded/PAGE;
        for (size_t p=*first;p<*first+*pages;++p) if (!(r->live[p/64] & ((uint64_t)1<<(p%64)))) return 0;
        return r;
    }
    return 0;
}
// ------------------------=
// FUNC: mmap
// DESC: Allocates zeroed, page-aligned private RW memory from the governed heap; unsupported authority fails closed.
// ------------------=
void *mmap(void *hint,size_t length,int protection,int flags,int fd,off_t offset) {
    (void)hint;
    size_t rounded=rounded_length(length);
    if (!rounded || !infinity_std_thread_id()) { errno=EINVAL; return MAP_FAILED; }
    if (protection!=(PROT_READ|PROT_WRITE) || flags!=(MAP_PRIVATE|MAP_ANONYMOUS) || fd!=-1 || offset) { errno=ENOTSUP; return MAP_FAILED; }
    if (rounded>MAX_PAGES*PAGE) { errno=ENOMEM; return MAP_FAILED; }
    void *memory=infinity_std_allocate(rounded,PAGE);
    if (!memory) { errno=ENOMEM; return MAP_FAILED; }
    size_t metadata=region_bytes(rounded);
    Region *r=infinity_std_allocate(metadata,_Alignof(Region));
    if (!r) { infinity_std_deallocate(memory,rounded,PAGE); errno=ENOMEM; return MAP_FAILED; }
    memset(r,0,metadata);
    r->base=memory; r->length=rounded; r->remaining=rounded/PAGE;
    for (size_t p=0;p<r->remaining;++p) r->live[p/64]|=(uint64_t)1<<(p%64);
    memset(memory,0,rounded);
    r->next=regions; regions=r;
    return memory;
}
// ------------------------=
// FUNC: munmap
// DESC: Revokes owned pages; returns the backing allocation when its last live subrange is released.
// ------------------=
int munmap(void *address,size_t length) {
    size_t first,pages; Region *r=region_range(address,length,&first,&pages);
    if (!r) { errno=EINVAL; return -1; }
    for (size_t p=first;p<first+pages;++p) r->live[p/64]&=~((uint64_t)1<<(p%64));
    r->remaining-=pages;
    if (!r->remaining) {
        Region **link=&regions;
        while (*link!=r) link=&(*link)->next;
        *link=r->next;
        infinity_std_deallocate(r->base,r->length,PAGE);
        infinity_std_deallocate(r,region_bytes(r->length),_Alignof(Region));
    }
    return 0;
}
// ------------------------=
// FUNC: mprotect
// DESC: Accepts unchanged RW permissions only; never claims absent hardware protection or executable authority.
// ------------------=
int mprotect(void *address,size_t length,int protection) {
    size_t first,pages;
    if (!region_range(address,length,&first,&pages)) { errno=EINVAL; return -1; }
    if (protection!=(PROT_READ|PROT_WRITE)) { errno=ENOTSUP; return -1; }
    return 0;
}
// ------------------------=
// FUNC: madvise
// DESC: Implements anonymous discard as deterministic zeroing without claiming physical decommit.
// ------------------=
int madvise(void *address,size_t length,int advice) {
    size_t first,pages;
    if (!region_range(address,length,&first,&pages)) { errno=EINVAL; return -1; }
    if (advice==MADV_DONTNEED) { memset(address,0,pages*PAGE); return 0; }
    if (advice==MADV_NORMAL || advice==MADV_WILLNEED || advice==MADV_RANDOM) return 0;
    errno=ENOTSUP; return -1;
}
