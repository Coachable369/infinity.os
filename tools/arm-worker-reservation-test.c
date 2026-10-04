#include <assert.h>
#include <stdint.h>
#include <stddef.h>
#include <string.h>
#define INFINITY_PSCI_TEST
#define EFIAPI
#define EFI_ALLOCATE_MAX_ADDRESS 1
#define EFI_LOADER_DATA 2
#define PAGE_SIZE 4096
typedef uint64_t EFI_STATUS;
typedef void (*INFINITY_AP_PROC)(void *);
typedef struct { uint32_t a; uint16_t b,c; uint8_t d[8]; } EFI_GUID;
typedef struct {
    void *unused_07_15[9];
    EFI_STATUS (*allocate_pages)(uint32_t,uint32_t,size_t,uint64_t *);
    EFI_STATUS (*locate_protocol)(EFI_GUID *,void *,void **);
} EFI_BOOT_SERVICES;
typedef struct { EFI_BOOT_SERVICES *boot_services; } EFI_SYSTEM_TABLE;
static size_t fallback_calls;
// ------------------------=
// FUNC: start_psci_workers
// DESC: Records MP fallback without executing privileged CPU startup in the host behavioral harness.
// ------------------=
static uint64_t start_psci_workers(INFINITY_AP_PROC procedure,uint64_t requested) {
    (void)procedure; (void)requested; ++fallback_calls; return 0;
}
// ------------------------=
// FUNC: discover_psci_workers
// DESC: Leaves caller-supplied topology intact while exercising the real MP launch implementation.
// ------------------=
static void discover_psci_workers(EFI_SYSTEM_TABLE *system) { (void)system; }
#include "../boot/common/worker_bridge.h"
static InfinityProcessor processors[8];
static size_t processor_count, launched_count;
static uint64_t launched_affinities[8], launched_arguments[8];
// ------------------------=
// FUNC: infinity_ap_callback
// DESC: Supplies the native trampoline symbol without invoking AP code during firmware mock startup.
// ------------------=
void infinity_ap_callback(void *context) { (void)context; }
// ------------------------=
// FUNC: mock_count
// DESC: Reports fixture processor enablement through the production firmware MP protocol.
// ------------------=
static EFI_STATUS mock_count(InfinityMp *mp,size_t *total,size_t *enabled) {
    (void)mp; *total=processor_count; *enabled=0;
    for (size_t i=0;i<processor_count;++i) *enabled+=(processors[i].flags&2)!=0;
    return 0;
}
// ------------------------=
// FUNC: mock_info
// DESC: Returns exact unsorted sparse processor IDs and flags to the actual launch loop.
// ------------------=
static EFI_STATUS mock_info(InfinityMp *mp,size_t index,InfinityProcessor *info) {
    (void)mp; assert(index<processor_count); *info=processors[index]; return 0;
}
// ------------------------=
// FUNC: mock_start
// DESC: Records observable AP selection and context numbering produced by the native launcher.
// ------------------=
static EFI_STATUS mock_start(InfinityMp *mp,INFINITY_AP_PROC procedure,size_t cpu,
                             void *event,size_t timeout,void *argument,uint8_t *finished) {
    (void)mp; (void)event; (void)timeout; (void)finished;
    assert(procedure==infinity_ap_callback && launched_count<8);
    InfinityApContext *context=argument;
    launched_affinities[launched_count]=processors[cpu].id;
    launched_arguments[launched_count++]=context->argument;
    return 0;
}
// ------------------------=
// FUNC: mock_create
// DESC: Supplies an event handle for each accepted AP without asynchronous host work.
// ------------------=
static EFI_STATUS mock_create(uint32_t type,size_t tpl,void *callback,void *context,void **event) {
    (void)type; (void)tpl; (void)callback; (void)context; *event=(void *)(uintptr_t)1; return 0;
}
// ------------------------=
// FUNC: mock_close
// DESC: Accepts event cleanup without changing processor selection.
// ------------------=
static EFI_STATUS mock_close(void *event) { (void)event; return 0; }
// ------------------------=
// FUNC: work
// DESC: Provides a non-null native callback identity for the production startup API.
// ------------------=
static void work(void *argument) { (void)argument; }
// ------------------------=
// FUNC: reset
// DESC: Initializes the real launcher's firmware dependencies and bounded native context storage.
// ------------------=
static void reset(void) {
    static InfinityMp mp={.count=mock_count,.info=mock_info,.start=mock_start};
    static EFI_BOOT_SERVICES boot;
    memset(&boot,0,sizeof(boot));
    boot.unused_07_15[0]=(void *)mock_create; boot.unused_07_15[4]=(void *)mock_close;
    worker_mp=&mp; worker_boot=&boot; worker_started=0; launched_count=0; fallback_calls=0;
    for (size_t i=0;i<8;++i) {
        psci_contexts[i].stack=UINT64_C(0x100000)+i*UINT64_C(0x10000);
        psci_contexts[i].exception_stack=psci_contexts[i].stack+4096;
    }
}
// ------------------------=
// FUNC: main
// DESC: Exercises actual MP startup across reserved IDs, requests, disabled CPUs, and the one-worker fallback contract.
// ------------------=
int main(void) {
    processors[0]=(InfinityProcessor){.id=7,.flags=6};
    processors[1]=(InfinityProcessor){.id=0,.flags=7};
    processors[2]=(InfinityProcessor){.id=2,.flags=6};
    processors[3]=(InfinityProcessor){.id=19,.flags=0};
    processors[4]=(InfinityProcessor){.id=3,.flags=6};
    processor_count=5; worker_service_cpu_reserved=1; worker_service_cpu=7;
    reset(); assert(infinity_start_workers(work,64)==2);
    assert(launched_count==2 && launched_affinities[0]==2 && launched_affinities[1]==3);
    assert(launched_arguments[0]==1 && launched_arguments[1]==2 && fallback_calls==0);
    reset(); assert(infinity_start_workers(work,1)==1 && launched_affinities[0]==2);
    reset(); assert(infinity_start_workers(work,0)==0 && launched_count==0);
    worker_service_cpu_reserved=0;
    reset(); assert(infinity_start_workers(work,64)==3 && launched_affinities[0]==7);
    worker_service_cpu_reserved=1; processor_count=2;
    reset(); assert(infinity_start_workers(work,64)==1 && launched_affinities[0]==7);
    processor_count=5; processors[2].flags=2; processors[4].flags=0;
    reset(); assert(infinity_start_workers(work,64)==1 && launched_affinities[0]==7);
    return 0;
}
