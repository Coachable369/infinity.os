#include "native_controller.h"
#if defined(INFINITY_AARCH64)
// ------------------------=
// FUNC: nc_firmware_read
// DESC: Reads an aligned redistributor register only when EL1 translation is identity mapped and accessible.
// ------------------=
static int nc_firmware_read(void *context, uint64_t address, uint64_t *value) {
    (void)context;
    if (address & 7) return 0;
    uint64_t par;
    __asm__ volatile("at s1e1r, %1; isb; mrs %0, par_el1" : "=r"(par) : "r"(address) : "memory");
    if ((par&1) || (par & UINT64_C(0x0000fffffffff000)) != (address & ~UINT64_C(4095))) return 0;
    *value=*(volatile uint64_t *)(uintptr_t)address;
    return 1;
}
#endif
// ------------------------=
// FUNC: discover_native_controller
// DESC: Discovers native slice interrupt hardware without modifying firmware routing or enabling execution.
// ------------------=
static InfinityNativeController discover_native_controller(EFI_SYSTEM_TABLE *system) {
    InfinityNativeController none={0};
#if defined(INFINITY_AARCH64)
    uint64_t level,mpidr;
    __asm__ volatile("mrs %0, CurrentEL; mrs %1, mpidr_el1" : "=r"(level),"=r"(mpidr));
    if (level!=4 || !system || !system->configuration_table || system->number_of_table_entries>4096) return none;
    for (size_t i=0;i<system->number_of_table_entries;++i) {
        EFI_CONFIGURATION_TABLE *entry=&system->configuration_table[i];
        if (!equal_bytes((const uint8_t *)&entry->vendor_guid,(const uint8_t *)&acpi20_table_guid,sizeof(EFI_GUID)) || !entry->vendor_table) continue;
        const uint8_t *rsdp=entry->vendor_table;
        if (nc_u64(rsdp)!=UINT64_C(0x2052545020445352) || rsdp[15]<2) return none;
        uint8_t sum=0; for (size_t j=0;j<20;++j) sum+=rsdp[j];
        size_t length=nc_u32(rsdp+20);
        if (sum || length<36 || length>4096) return none;
        for (size_t j=0;j<length;++j) sum+=rsdp[j];
        if (sum) return none;
        const uint8_t *xsdt=(const uint8_t *)(uintptr_t)nc_u64(rsdp+24);
        if (!xsdt || !nc_table(xsdt,nc_u32(xsdt+4),0x54445358,36) || (nc_u32(xsdt+4)-36)%8) return none;
        const uint8_t *madt=NULL,*gtdt=NULL;
        for (size_t j=36;j<nc_u32(xsdt+4);j+=8) {
            const uint8_t *table=(const uint8_t *)(uintptr_t)nc_u64(xsdt+j);
            if (!table) continue;
            if (nc_u32(table)==0x43495041) { if (madt) return none; madt=table; }
            if (nc_u32(table)==0x54445447) { if (gtdt) return none; gtdt=table; }
        }
        if (!madt || !gtdt) return none;
        return infinity_arm_controller(madt,nc_u32(madt+4),gtdt,nc_u32(gtdt+4),
            mpidr & UINT64_C(0xff00ffffff),nc_firmware_read,NULL);
    }
#else
    (void)system;
    uint32_t a,b,c,d;
    __asm__ volatile("cpuid" : "=a"(a),"=b"(b),"=c"(c),"=d"(d) : "a"(1),"c"(0));
    if (!(d & (1u<<9))) return none;
    __asm__ volatile("rdmsr" : "=a"(a),"=d"(d) : "c"(0x1bu));
    uint64_t msr=(uint64_t)d<<32|a,base=msr & UINT64_C(0x0000000ffffff000);
    if (!(msr & (1u<<11)) || (msr & (1u<<10)) || !base || base>=UINT64_C(64)*1024*1024*1024) return none;
    none.kind=1; none.base=base;
#endif
    return none;
}
