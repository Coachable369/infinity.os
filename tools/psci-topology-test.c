#include <stdint.h>
#include <stddef.h>
#include <assert.h>
#include <string.h>
#define INFINITY_PSCI_TEST
typedef void EFI_SYSTEM_TABLE;
#include "../boot/common/psci_workers.h"
// ------------------------=
// FUNC: checksum
// DESC: Creates a valid ACPI checksum for a binary fixture.
// ------------------=
static void checksum(uint8_t *bytes, size_t size) {
    bytes[9]=0; uint8_t sum=0;
    for (size_t i=0;i<size;++i) sum+=bytes[i];
    bytes[9]=(uint8_t)-sum;
}
// ------------------------=
// FUNC: main
// DESC: Verifies BSP exclusion, disabled CPUs, duplicate affinity rejection, and malformed MADT handling.
// ------------------=
int main(void) {
    uint8_t table[44+5*80]={0};
    memcpy(table,"APIC",4);
    uint32_t size=sizeof(table); memcpy(table+4,&size,4);
    for (size_t i=0;i<5;++i) {
        uint8_t *cpu=table+44+i*80; cpu[0]=11; cpu[1]=80; cpu[12]=1; cpu[68]=(uint8_t)i;
    }
    table[44+2*80+12]=0; table[44+4*80+68]=3;
    checksum(table,sizeof(table)); parse_psci_cpus(table,0);
    assert(psci_count==2 && psci_cpus[0]==1 && psci_cpus[1]==3);
    table[10]^=1; parse_psci_cpus(table,0); assert(psci_count==0);
    checksum(table,sizeof(table)); table[45]=1; checksum(table,sizeof(table));
    parse_psci_cpus(table,0); assert(psci_count==0);
    return 0;
}
