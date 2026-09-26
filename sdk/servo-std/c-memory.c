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
typedef struct { unsigned char *base; size_t length; uint64_t live[MAX_PAGES/64]; size_t remaining; } Region;
static Region regions[64];
// ------------------------=
// FUNC: rounded_length
// DESC: Rejects zero and overflow before rounding anonymous regions to native page boundaries.
// ------------------=
static size_t rounded_length(size_t length) { return length && length<=SIZE_MAX-(PAGE-1) ? (length+PAGE-1)&~(size_t)(PAGE-1) : 0; }
// ------------------------=
// FUNC: region_range
// DESC: Validates an aligned owned range and rejects already unmapped holes before changing state.
// ------------------=
static Region *region_range(void *address,size_t length,size_t *first,size_t *pages) {
    uintptr_t start=(uintptr_t)address; size_t rounded=rounded_length(length);
    if (!infinity_std_thread_id() || start%PAGE || !rounded || start>UINTPTR_MAX-rounded) return 0;
    for (unsigned i=0;i<64;++i) {
        Region *r=&regions[i]; uintptr_t base=(uintptr_t)r->base;
        if (!r->base || start<base || start-base>r->length || rounded>r->length-(start-base)) continue;
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
    for (unsigned i=0;i<64;++i) {
        Region *r=&regions[i]; if (r->base) continue;
        void *memory=infinity_std_allocate(rounded,PAGE);
        if (!memory) break;
        *r=(Region){.base=memory,.length=rounded,.remaining=rounded/PAGE};
        for (size_t p=0;p<r->remaining;++p) r->live[p/64]|=(uint64_t)1<<(p%64);
        memset(memory,0,rounded); return memory;
    }
    errno=ENOMEM; return MAP_FAILED;
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
    if (!r->remaining) { infinity_std_deallocate(r->base,r->length,PAGE); r->base=0; }
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
