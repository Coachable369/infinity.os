/* ACPI-advertised PSCI CPU_ON fallback for identity-mapped EL1 firmware. */
typedef struct {
    uint64_t stack, procedure, argument, level, sctlr, tcr, ttbr0, ttbr1, mair, vbar;
} InfinityApContext;
static InfinityApContext psci_contexts[64] __attribute__((aligned(64)));
static uint64_t psci_cpus[64];
static size_t psci_count;
static uint8_t psci_hvc;
extern void infinity_ap_entry(void);

// ------------------------=
// FUNC: worker_u32
// DESC: Reads an unaligned ACPI little-endian integer.
// ------------------=
static uint32_t worker_u32(const uint8_t *p) {
    return (uint32_t)p[0] | (uint32_t)p[1]<<8 | (uint32_t)p[2]<<16 | (uint32_t)p[3]<<24;
}
// ------------------------=
// FUNC: worker_u64
// DESC: Reads an unaligned ACPI address.
// ------------------=
static uint64_t worker_u64(const uint8_t *p) { return worker_u32(p) | (uint64_t)worker_u32(p+4)<<32; }
// ------------------------=
// FUNC: worker_checksum
// DESC: Rejects corrupt or unbounded firmware tables.
// ------------------=
static int worker_checksum(const uint8_t *p, size_t n) {
    if (!p || n < 20 || n > 1024*1024) return 0;
    uint8_t sum = 0; for (size_t i=0; i<n; ++i) sum += p[i]; return sum == 0;
}
// ------------------------=
// FUNC: worker_table
// DESC: Checks table length, signature and checksum before field access.
// ------------------=
static int worker_table(const uint8_t *p, uint32_t signature, size_t minimum) {
    return p && worker_u32(p)==signature && worker_u32(p+4)>=minimum && worker_checksum(p,worker_u32(p+4));
}
// ------------------------=
// FUNC: parse_psci_cpus
// DESC: Validates enabled unique GICC affinities, excluding the BSP and malformed records.
// ------------------=
static void parse_psci_cpus(const uint8_t *madt, uint64_t self) {
    psci_count=0;
    if (!worker_table(madt,0x43495041,44)) return;
    size_t length=worker_u32(madt+4), at=44;
    for (; at+2<=length;) {
        const uint8_t *cpu=madt+at;
        if (cpu[1]<2 || cpu[1]>length-at) { psci_count=0; return; }
        if (cpu[0]==11 && cpu[1]>=76 && (worker_u32(cpu+12)&1)) {
            uint64_t id=worker_u64(cpu+68);
            int duplicate=id==self || (id & ~UINT64_C(0xff00ffffff));
            for (size_t j=0;j<psci_count;++j) duplicate |= psci_cpus[j]==id;
            if (!duplicate && psci_count<64) psci_cpus[psci_count++]=id;
        }
        at+=cpu[1];
    }
    if (at!=length) psci_count=0;
}
#ifndef INFINITY_PSCI_TEST
// ------------------------=
// FUNC: discover_psci_workers
// DESC: Reads the PSCI conduit and enabled CPU affinities from validated FADT and MADT tables.
// ------------------=
static void discover_psci_workers(EFI_SYSTEM_TABLE *system) {
    const uint8_t *fadt = NULL, *madt = NULL;
    if (!system->configuration_table || system->number_of_table_entries > 4096) return;
    for (size_t i=0; i<system->number_of_table_entries; ++i) {
        EFI_CONFIGURATION_TABLE *entry = &system->configuration_table[i];
        if (entry->vendor_guid.data1 != 0x8868e871 || !entry->vendor_table) continue;
        const uint8_t *rsdp = entry->vendor_table;
        if (worker_u64(rsdp)!=UINT64_C(0x2052545020445352) || rsdp[15]<2 || !worker_checksum(rsdp,20) || worker_u32(rsdp+20)<36 || !worker_checksum(rsdp,worker_u32(rsdp+20))) continue;
        const uint8_t *xsdt = (const uint8_t *)(uintptr_t)worker_u64(rsdp+24);
        if (!worker_table(xsdt,0x54445358,36) || (worker_u32(xsdt+4)-36)%8) continue;
        for (size_t at=36; at<worker_u32(xsdt+4); at+=8) {
            const uint8_t *table=(const uint8_t *)(uintptr_t)worker_u64(xsdt+at);
            if (worker_table(table,0x50434146,132)) fadt=table;
            if (worker_table(table,0x43495041,44)) madt=table;
        }
    }
    if (!fadt || !madt || !(fadt[129]&1)) return;
    psci_hvc = !!(fadt[129]&2);
    uint64_t self;
    __asm__ volatile("mrs %0, mpidr_el1" : "=r"(self));
    self &= UINT64_C(0xff00ffffff);
    parse_psci_cpus(madt,self);
}
// ------------------------=
// FUNC: worker_identity
// DESC: Requires identity mappings before sharing EL1 translation tables with a cold AP.
// ------------------=
static int worker_identity(uint64_t address) {
    uint64_t translated;
    __asm__ volatile("at s1e1r, %1; isb; mrs %0, par_el1" : "=r"(translated) : "r"(address) : "memory");
    return !(translated&1) && (translated&UINT64_C(0x000ffffffffff000))==(address&UINT64_C(0x000ffffffffff000));
}
// ------------------------=
// FUNC: worker_cpu_on
// DESC: Invokes CPU_ON through only the platform-advertised PSCI conduit.
// ------------------=
static int64_t worker_cpu_on(uint64_t cpu, InfinityApContext *context) {
    register uint64_t x0 __asm__("x0")=UINT64_C(0xc4000003);
    register uint64_t x1 __asm__("x1")=cpu;
    register uint64_t x2 __asm__("x2")=(uint64_t)(uintptr_t)infinity_ap_entry;
    register uint64_t x3 __asm__("x3")=(uint64_t)(uintptr_t)context;
    if (psci_hvc) __asm__ volatile("hvc #0" : "+r"(x0), "+r"(x1), "+r"(x2), "+r"(x3) :: "x4","x5","x6","x7","x8","x9","x10","x11","x12","x13","x14","x15","x16","x17","memory","cc");
    else __asm__ volatile("smc #0" : "+r"(x0), "+r"(x1), "+r"(x2), "+r"(x3) :: "x4","x5","x6","x7","x8","x9","x10","x11","x12","x13","x14","x15","x16","x17","memory","cc");
    return (int64_t)x0;
}
// ------------------------=
// FUNC: start_psci_workers
// DESC: Uses reserved private AP stacks to start cold cores without taking over CPUs already owned by firmware.
// ------------------=
static uint64_t start_psci_workers(INFINITY_AP_PROC procedure, uint64_t requested) {
    uint64_t level;
    __asm__ volatile("mrs %0, CurrentEL" : "=r"(level));
    if (level!=4 || !psci_count || !worker_identity((uint64_t)(uintptr_t)infinity_ap_entry) || !worker_identity((uint64_t)(uintptr_t)procedure)) return 0;
    // Discovery already excludes the BSP. Do not strand a second core.
    size_t limit=psci_count;
    if (limit>64) limit=64;
    if (limit>requested) limit=requested;
    size_t launched=0;
    for (size_t i=0;i<psci_count && launched<limit;++i) {
        InfinityApContext *c=&psci_contexts[launched];
        if (!c->stack) break;
        c->procedure=(uint64_t)(uintptr_t)procedure; c->argument=launched+1; c->level=level;
        __asm__ volatile("mrs %0, sctlr_el1" : "=r"(c->sctlr));
        __asm__ volatile("mrs %0, tcr_el1" : "=r"(c->tcr));
        __asm__ volatile("mrs %0, ttbr0_el1" : "=r"(c->ttbr0));
        __asm__ volatile("mrs %0, ttbr1_el1" : "=r"(c->ttbr1));
        __asm__ volatile("mrs %0, mair_el1" : "=r"(c->mair));
        __asm__ volatile("mrs %0, vbar_el1" : "=r"(c->vbar));
        if (!worker_identity((uint64_t)(uintptr_t)c) || !worker_identity(c->stack-16)) break;
        uint64_t ctr;
        __asm__ volatile("mrs %0, ctr_el0" : "=r"(ctr));
        uintptr_t line=(uintptr_t)4<<((ctr>>16)&15);
        for (uintptr_t p=(uintptr_t)c&~(line-1); p<(uintptr_t)(c+1);p+=line) __asm__ volatile("dc cvac, %0" :: "r"(p) : "memory");
        __asm__ volatile("dsb sy" ::: "memory");
        if (worker_cpu_on(psci_cpus[i],c)==0) ++launched;
    }
    return launched;
}
#endif
