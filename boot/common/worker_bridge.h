/* PI MP Services bootstrap bridge. AP callbacks are native kernel code and
 * cannot use firmware services. Boot services remain live on the ARM path. */
typedef void (EFIAPI *INFINITY_AP_PROC)(void *);
#include "psci_workers.h"
typedef struct InfinityMp InfinityMp;
typedef struct { uint64_t id; uint32_t flags; uint32_t package, core, thread; } InfinityProcessor;
struct InfinityMp {
    EFI_STATUS (EFIAPI *count)(InfinityMp *, size_t *, size_t *);
    EFI_STATUS (EFIAPI *info)(InfinityMp *, size_t, InfinityProcessor *);
    void *all;
    EFI_STATUS (EFIAPI *start)(InfinityMp *, INFINITY_AP_PROC, size_t, void *, size_t, void *, uint8_t *);
    void *switch_bsp, *enable, *whoami;
};
typedef struct { uint64_t version; uint64_t (EFIAPI *start)(INFINITY_AP_PROC, uint64_t); } InfinityWorkers;
static InfinityMp *worker_mp;
static EFI_BOOT_SERVICES *worker_boot;
static void *worker_events[64];
static uint8_t worker_started;
extern void EFIAPI infinity_ap_callback(void *context);
#define INFINITY_WORKER_STACK_PAGES 2048u
#define INFINITY_WORKER_EXCEPTION_STACK_PAGES 4u

// ------------------------=
// FUNC: infinity_start_workers
// DESC: Starts available APs asynchronously, retaining only the BSP for input and services.
// ------------------=
static uint64_t EFIAPI infinity_start_workers(INFINITY_AP_PROC procedure, uint64_t requested) {
    if (worker_started || !procedure) return 0;
    worker_started = 1;
    if (!worker_mp) return start_psci_workers(procedure,requested);
    size_t total = 0, enabled = 0;
    if (worker_mp->count(worker_mp, &total, &enabled) || enabled < 2) return start_psci_workers(procedure,requested);
    size_t limit = enabled - 1;
    if (limit > requested) limit = requested;
    if (limit > 64) limit = 64;
    typedef EFI_STATUS (EFIAPI *CreateEvent)(uint32_t, size_t, void *, void *, void **);
    typedef EFI_STATUS (EFIAPI *CloseEvent)(void *);
    CreateEvent create = (CreateEvent)worker_boot->unused_07_15[0];
    CloseEvent close = (CloseEvent)worker_boot->unused_07_15[4];
    size_t launched = 0;
    for (size_t cpu = 0; cpu < total && launched < limit; ++cpu) {
        InfinityProcessor info;
        if (worker_mp->info(worker_mp, cpu, &info) || (info.flags & 1) || (info.flags & 6) != 6) continue;
        InfinityApContext *context = &psci_contexts[launched];
        // MP firmware stacks are not sized for speech decoding. Every worker
        // must enter its reserved native stack before calling kernel code.
        if (!context->stack || !context->exception_stack) break;
        context->procedure = (uint64_t)(uintptr_t)procedure;
        context->argument = launched + 1;
        if (create(0, 0, NULL, NULL, &worker_events[launched])) continue;
        EFI_STATUS status = worker_mp->start(worker_mp, infinity_ap_callback, cpu, worker_events[launched], 0,
                                            context, NULL);
        if (status) { close(worker_events[launched]); continue; }
        ++launched;
    }
    return launched ? launched : start_psci_workers(procedure,requested);
}

// ------------------------=
// FUNC: infinity_worker_bridge
// DESC: Discovers standardized MP services without changing CPU state when the protocol is absent.
// ------------------=
static uint64_t infinity_worker_bridge(EFI_SYSTEM_TABLE *system) {
    static InfinityWorkers bridge = { 1, infinity_start_workers };
    EFI_GUID guid = {0x3fdda605,0xa76e,0x4f46,{0xad,0x29,0x12,0xf4,0x53,0x1b,0x3d,0x08}};
    worker_boot = system->boot_services;
    discover_psci_workers(system);
    if (worker_boot->locate_protocol(&guid, NULL, (void **)&worker_mp)) worker_mp=NULL;
    if (!worker_mp && !psci_count) return 0;
    size_t count = psci_count, total = 0, enabled = 0;
    if (worker_mp && !worker_mp->count(worker_mp, &total, &enabled) && enabled > 1 && enabled - 1 > count)
        count = enabled - 1;
    // Reserve before the boot memory map is captured. Native Whisper and
    // Kokoro execute only on these isolated stacks; keep the installed
    // contract identical to the behavioral speech probes.
    for (size_t i = 0; i < count && i < 64; ++i) {
        uint64_t stack = UINT64_C(0xffffffff);
        if (worker_boot->allocate_pages(EFI_ALLOCATE_MAX_ADDRESS, EFI_LOADER_DATA,
                                        INFINITY_WORKER_STACK_PAGES + INFINITY_WORKER_EXCEPTION_STACK_PAGES,
                                        &stack)) break;
        psci_contexts[i].stack = stack + INFINITY_WORKER_STACK_PAGES * PAGE_SIZE;
        // EDK2's synchronous exception vector switches to SP_EL0. PSCI
        // does not initialize it, and MP callbacks cannot share the BSP's
        // emergency area. Reserve a distinct stack outside native frames.
        psci_contexts[i].exception_stack = psci_contexts[i].stack +
                                         INFINITY_WORKER_EXCEPTION_STACK_PAGES * PAGE_SIZE;
    }
    return (uint64_t)(uintptr_t)&bridge;
}
