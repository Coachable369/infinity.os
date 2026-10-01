/* Firmware is used only to enumerate/reserve before ExitBootServices.
 * Startup afterwards uses native local APIC INIT/SIPI, never firmware calls. */
typedef void (EFIAPI *X86ApProc)(void *);
typedef struct X86Mp X86Mp;
typedef struct { uint64_t id; uint32_t flags, package, core, thread; } X86Cpu;
struct X86Mp {
    EFI_STATUS (EFIAPI *count)(X86Mp *, size_t *, size_t *);
    EFI_STATUS (EFIAPI *info)(X86Mp *, size_t, X86Cpu *);
};
typedef struct { uint64_t version; uint64_t (EFIAPI *start)(X86ApProc, uint64_t); uint64_t hz; } X86Workers;
extern const uint8_t infinity_ap_page[4096];
static uint64_t x86_ap_page, x86_ap_stacks, x86_ap_cr3, x86_tsc_hz;
static uint32_t x86_ap_ids[8], x86_ap_count, x86_ap_started;
#define INFINITY_X86_WORKER_STACK_PAGES 2048u

// ------------------------=
// FUNC: x86_ticks
// DESC: Reads the ordered native timestamp counter on the bootstrap CPU.
// ------------------=
static uint64_t x86_ticks(void) {
    uint32_t lo, hi;
    __asm__ volatile("lfence; rdtsc" : "=a"(lo), "=d"(hi) :: "memory");
    return ((uint64_t)hi << 32) | lo;
}
// ------------------------=
// FUNC: x86_msr
// DESC: Reads the architectural local APIC mode and base without firmware.
// ------------------=
static uint64_t x86_msr(uint32_t index) {
    uint32_t lo, hi;
    __asm__ volatile("rdmsr" : "=a"(lo), "=d"(hi) : "c"(index));
    return ((uint64_t)hi << 32) | lo;
}
// ------------------------=
// FUNC: x86_delay_us
// DESC: Observes architectural INIT/SIPI timing using the calibrated native counter.
// ------------------=
static void x86_delay_us(uint64_t us) {
    uint64_t start = x86_ticks(), duration = x86_tsc_hz / 1000000 * us;
    while (x86_ticks() - start < duration) __asm__ volatile("pause");
}
// ------------------------=
// FUNC: x86_ipi
// DESC: Sends an IPI in the already enabled APIC mode, with bounded delivery polling.
// ------------------=
static int x86_ipi(uint32_t id, uint32_t command) {
    uint64_t base = x86_msr(0x1b);
    if (!(base & (1u << 11))) return 0;
    if (base & (1u << 10)) {
        __asm__ volatile("wrmsr" :: "c"(0x830), "a"(command), "d"(id) : "memory");
        return 1;
    }
    if (id > 255) return 0;
    volatile uint32_t *apic = (volatile uint32_t *)(uintptr_t)(base & UINT64_C(0xfffff000));
    uint64_t start = x86_ticks();
    while (apic[0x300/4] & (1u << 12))
        if (x86_ticks() - start > x86_tsc_hz / 10) return 0;
    apic[0x310/4] = id << 24;
    apic[0x300/4] = command;
    start = x86_ticks();
    while (apic[0x300/4] & (1u << 12))
        if (x86_ticks() - start > x86_tsc_hz / 10) return 0;
    return 1;
}
// ------------------------=
// FUNC: x86_start_workers
// DESC: Starts independent native AP stacks after firmware exit; never reuses a timed-out mailbox.
// ------------------=
static uint64_t EFIAPI x86_start_workers(X86ApProc entry, uint64_t requested) {
    if (x86_ap_started || !entry || !x86_tsc_hz || !x86_ap_page) return 0;
    x86_ap_started = 1;
    volatile uint64_t *data = (volatile uint64_t *)(uintptr_t)(x86_ap_page + 0x800);
    uint64_t launched = 0;
    for (uint32_t i = 0; i < x86_ap_count && launched < requested; ++i) {
        data[0] = x86_ap_cr3;
        data[1] = x86_ap_stacks + (i + 1) * INFINITY_X86_WORKER_STACK_PAGES * 4096;
        data[2] = (uint64_t)(uintptr_t)entry;
        data[3] = launched + 1;
        data[4] = 0;
        __asm__ volatile("mfence" ::: "memory");
        if (!x86_ipi(x86_ap_ids[i], 0xc500)) break;
        x86_delay_us(10000);
        if (!x86_ipi(x86_ap_ids[i], 0x8500)) break;
        if (!x86_ipi(x86_ap_ids[i], 0x600 | (uint32_t)(x86_ap_page >> 12))) break;
        x86_delay_us(200);
        if (!data[4] && !x86_ipi(x86_ap_ids[i], 0x600 | (uint32_t)(x86_ap_page >> 12))) break;
        uint64_t start = x86_ticks();
        while (!data[4] && x86_ticks() - start < x86_tsc_hz / 10) __asm__ volatile("pause");
        if (!data[4]) break;
        ++launched;
    }
    return launched;
}
// ------------------------=
// FUNC: x86_worker_bridge
// DESC: Reserves native startup resources and copies CPU identities before firmware services end.
// ------------------=
static uint64_t x86_worker_bridge(EFI_SYSTEM_TABLE *system, uint64_t cr3) {
    static X86Workers bridge = {2, x86_start_workers, 0};
    EFI_BOOT_SERVICES *boot = system->boot_services;
    EFI_GUID guid = {0x3fdda605,0xa76e,0x4f46,{0xad,0x29,0x12,0xf4,0x53,0x1b,0x3d,0x08}};
    X86Mp *mp = NULL;
    size_t total = 0, enabled = 0;
    typedef EFI_STATUS (EFIAPI *Stall)(size_t);
    uint64_t begin = x86_ticks();
    if (((Stall)boot->unused_27_28[1])(10000)) return 0;
    x86_tsc_hz = (x86_ticks() - begin) * 100;
    if (x86_tsc_hz < 1000000 || x86_tsc_hz > UINT64_C(10000000000)) return 0;
    bridge.hz = x86_tsc_hz;
    if (boot->locate_protocol(&guid, NULL, (void **)&mp) || mp->count(mp, &total, &enabled))
        return (uint64_t)(uintptr_t)&bridge;
    for (size_t i = 0; i < total && x86_ap_count < 8; ++i) {
        X86Cpu cpu;
        if (!mp->info(mp, i, &cpu) && !(cpu.flags & 1) && (cpu.flags & 6) == 6 && cpu.id <= UINT32_MAX)
            x86_ap_ids[x86_ap_count++] = (uint32_t)cpu.id;
    }
    if (!x86_ap_count) return (uint64_t)(uintptr_t)&bridge;
    x86_ap_page = 0x9ffff;
    if (boot->allocate_pages(EFI_ALLOCATE_MAX_ADDRESS, EFI_LOADER_DATA, 1, &x86_ap_page)) return 0;
    x86_ap_stacks = UINT32_MAX;
    if (boot->allocate_pages(EFI_ALLOCATE_MAX_ADDRESS, EFI_LOADER_DATA,
                             x86_ap_count * INFINITY_X86_WORKER_STACK_PAGES,
                             &x86_ap_stacks)) return 0;
    uint8_t *page = (uint8_t *)(uintptr_t)x86_ap_page;
    for (size_t i = 0; i < 4096; ++i) page[i] = infinity_ap_page[i];
    *(uint32_t *)(page + 0x82a) = (uint32_t)x86_ap_page + 0x850;
    *(uint32_t *)(page + 0x830) = (uint32_t)x86_ap_page + 0x100;
    *(uint32_t *)(page + 0x838) = (uint32_t)x86_ap_page + 0x200;
    x86_ap_cr3 = cr3;
    return (uint64_t)(uintptr_t)&bridge;
}
