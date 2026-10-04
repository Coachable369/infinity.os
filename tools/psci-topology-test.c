#include <stdint.h>
#include <stddef.h>
#include <assert.h>
#include <string.h>
#define INFINITY_PSCI_TEST
typedef void EFI_SYSTEM_TABLE;
#include "../boot/common/psci_workers.h"
// ------------------------=
// FUNC: checksum
// DESC: Creates a valid ACPI checksum at the specified checksum byte.
// ------------------=
static void checksum(uint8_t *bytes, size_t size, size_t offset) {
    bytes[offset]=0; uint8_t sum=0;
    for (size_t i=0;i<size;++i) sum+=bytes[i];
    bytes[offset]=(uint8_t)-sum;
}
// ------------------------=
// FUNC: topology
// DESC: Encodes native ACPI fixtures with caller-specified affinity order and enabled flags.
// ------------------=
static void topology(uint8_t *rsdp, uint8_t *madt, const uint64_t *ids,
                     const uint8_t *flags, size_t count) {
    memset(rsdp,0,36); memset(madt,0,44+count*80);
    memcpy(rsdp,"RSD PTR ",8); memcpy(rsdp+9,"ORCLVB",6); rsdp[15]=2;
    uint32_t length=36; memcpy(rsdp+20,&length,4);
    checksum(rsdp,20,8); checksum(rsdp,36,32);
    memcpy(madt,"APIC",4); memcpy(madt+10,"ORCLVB",6);
    length=(uint32_t)(44+count*80); memcpy(madt+4,&length,4);
    for (size_t i=0;i<count;++i) {
        uint8_t *cpu=madt+44+i*80;
        cpu[0]=11; cpu[1]=80; cpu[12]=flags[i]; memcpy(cpu+68,&ids[i],8);
    }
    checksum(madt,length,9);
}
// ------------------------=
// FUNC: select_workers
// DESC: Exercises the shared native launch admission policy against bounded requested worker counts.
// ------------------=
static size_t select_workers(size_t requested, uint64_t *selected) {
    size_t count=0;
    for (size_t i=0;i<psci_count && count<requested;++i)
        if (worker_cpu_allowed(psci_cpus[i],psci_count)) selected[count++]=psci_cpus[i];
    return count;
}
// ------------------------=
// FUNC: main
// DESC: Verifies validated platform identity, unsorted sparse reservation, small systems, requests, and malformed ACPI rejection.
// ------------------=
int main(void) {
    uint8_t rsdp[36], table[44+5*80]; uint64_t selected[5];
    const uint64_t ids[5]={7,0,2,19,3}; const uint8_t flags[5]={1,1,1,0,1};
    topology(rsdp,table,ids,flags,5);
    parse_psci_cpus(table,0); configure_worker_service_cpu(rsdp,table);
    assert(psci_count==3 && psci_cpus[0]==7 && psci_cpus[1]==2 && psci_cpus[2]==3);
    assert(worker_service_cpu_reserved && worker_service_cpu==7);
    assert(select_workers(0,selected)==0);
    assert(select_workers(1,selected)==1 && selected[0]==2);
    assert(select_workers(64,selected)==2 && selected[0]==2 && selected[1]==3);
    assert(!worker_cpu_allowed(UINT64_C(0x80000007),3));
    assert(worker_cpu_allowed(7,1));
    rsdp[9]='X'; checksum(rsdp,20,8); checksum(rsdp,36,32);
    configure_worker_service_cpu(rsdp,table);
    assert(!worker_service_cpu_reserved && select_workers(64,selected)==3);
    topology(rsdp,table,ids,flags,5); table[10]='X'; checksum(table,sizeof(table),9);
    configure_worker_service_cpu(rsdp,table); assert(!worker_service_cpu_reserved);
    topology(rsdp,table,ids,flags,5); rsdp[8]^=1;
    configure_worker_service_cpu(rsdp,table); assert(!worker_service_cpu_reserved);
    topology(rsdp,table,ids,flags,5); rsdp[32]^=1;
    configure_worker_service_cpu(rsdp,table); assert(!worker_service_cpu_reserved);
    topology(rsdp,table,ids,flags,5); table[9]^=1;
    configure_worker_service_cpu(rsdp,table); assert(!worker_service_cpu_reserved);
    parse_psci_cpus(table,0); assert(psci_count==0);
    const uint64_t duplicates[5]={0,1,2,3,3}; const uint8_t disabled[5]={1,1,0,1,1};
    topology(rsdp,table,duplicates,disabled,5); parse_psci_cpus(table,0);
    assert(psci_count==2 && psci_cpus[0]==1 && psci_cpus[1]==3);
    table[45]=1; checksum(table,sizeof(table),9);
    parse_psci_cpus(table,0); assert(psci_count==0);
    configure_worker_service_cpu(rsdp,table); assert(!worker_service_cpu_reserved);
    const uint64_t small[3]={1,0,2}; const uint8_t enabled[3]={1,1,1};
    topology(rsdp,table,small,enabled,2); parse_psci_cpus(table,0);
    configure_worker_service_cpu(rsdp,table);
    assert(psci_count==1 && !worker_service_cpu_reserved && select_workers(64,selected)==1);
    topology(rsdp,table,small,enabled,3); parse_psci_cpus(table,0);
    configure_worker_service_cpu(rsdp,table);
    assert(worker_service_cpu_reserved && worker_service_cpu==2 && select_workers(64,selected)==1);
    topology(rsdp,table,small+1,enabled,1); parse_psci_cpus(table,0);
    configure_worker_service_cpu(rsdp,table);
    assert(psci_count==0 && !worker_service_cpu_reserved && select_workers(64,selected)==0);
    return 0;
}
