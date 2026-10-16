#include <assert.h>
#include <string.h>
#include "../boot/common/native_controller.h"
#include "../boot/common/boot_info.h"
#include "../boot/common/native_pool.h"
static uint8_t madt[44+80+24+16],gtdt[80];
static unsigned reads;
static unsigned allocations;
// ------------------------=
// FUNC: limited_pool
// DESC: Models contiguous firmware availability while checking requested physical limits.
// ------------------=
static uint64_t limited_pool(void *context,uint64_t bytes,uint64_t maximum) {
    ++allocations; assert(maximum==UINT64_C(0xffffffff));
    return bytes<=*(uint64_t *)context ? UINT64_C(0x50000000) : 0;
}
// ------------------------=
// FUNC: put32
// DESC: Encodes fixture fields independently of parser reads.
// ------------------=
static void put32(uint8_t *p,uint32_t value) { for(unsigned i=0;i<4;++i) p[i]=(uint8_t)(value>>(8*i)); }
// ------------------------=
// FUNC: put64
// DESC: Encodes fixture physical addresses.
// ------------------=
static void put64(uint8_t *p,uint64_t value) { for(unsigned i=0;i<8;++i) p[i]=(uint8_t)(value>>(8*i)); }
// ------------------------=
// FUNC: checksum
// DESC: Produces a valid ACPI checksum after a fixture mutation.
// ------------------=
static void checksum(uint8_t *p,size_t n) { p[9]=0; uint8_t sum=0; for(size_t i=0;i<n;++i) sum+=p[i]; p[9]=(uint8_t)-sum; }
// ------------------------=
// FUNC: setup
// DESC: Creates valid controller/timer tables for a nonzero BSP affinity.
// ------------------=
static void setup(unsigned version) {
    memset(madt,0,sizeof(madt)); memset(gtdt,0,sizeof(gtdt)); reads=0;
    put32(madt,0x43495041); put32(madt+4,sizeof(madt));
    uint8_t *cpu=madt+44,*dist=cpu+80,*range=dist+24;
    cpu[0]=11; cpu[1]=80; put32(cpu+12,1); put64(cpu+32,0x08010000); put64(cpu+68,0x100000002);
    dist[0]=12; dist[1]=24; put64(dist+8,0x08000000); dist[20]=(uint8_t)version;
    range[0]=14; range[1]=16; put64(range+4,0x080a0000); put32(range+12,0x40000);
    put32(gtdt,0x54445447); put32(gtdt+4,sizeof(gtdt)); put32(gtdt+56,30);
    checksum(madt,sizeof(madt)); checksum(gtdt,sizeof(gtdt));
}
// ------------------------=
// FUNC: read_typer
// DESC: Models two redistributors and rejects any MMIO read outside those registers.
// ------------------=
static int read_typer(void *context,uint64_t address,uint64_t *value) {
    (void)context; ++reads;
    if(address==0x080a0008) { *value=0; return 1; }
    if(address==0x080c0008) { *value=UINT64_C(0x01000002)<<32|16; return 1; }
    return 0;
}
// ------------------------=
// FUNC: discover
// DESC: Exercises the same pure parser used by both installed and live firmware loaders.
// ------------------=
static InfinityNativeController discover(void) {
    return infinity_arm_controller(madt,sizeof(madt),gtdt,sizeof(gtdt),0x100000002,read_typer,0);
}
// ------------------------=
// FUNC: main
// DESC: Verifies controller addresses, fail-closed malformed topology and bounded MMIO access through structured results.
// ------------------=
int main(void) {
    for(unsigned mib=0;mib<=256;mib+=64) {
        uint64_t available=(uint64_t)mib*1024*1024,bytes=999; allocations=0;
        uint64_t base=infinity_reserve_process_pool(limited_pool,&available,UINT64_C(0xffffffff),&bytes);
        unsigned expected=mib>=256?256:mib>=128?128:mib>=64?64:0;
        assert(bytes==(uint64_t)expected*1024*1024);
        assert(base==(expected?UINT64_C(0x50000000):0));
        assert(allocations==(mib>=256?1:mib>=128?2:3));
    }
    assert(sizeof(InfinityBootInfo)==392 && offsetof(InfinityBootInfo,native_controller_kind)==368);
    setup(2); InfinityNativeController c=discover();
    assert(c.kind==2 && c.base==0x08000000 && c.auxiliary==0x08010000 && reads==0);
    setup(3); c=discover(); assert(c.kind==3 && c.base==0x080c0000 && c.auxiliary==0x08000000 && reads==2);
    setup(3); put64(madt+44+60,0x080c0000); checksum(madt,sizeof(madt)); c=discover(); assert(c.kind==3 && reads==1);
    setup(3); put64(madt+44+60,0x080a0000); checksum(madt,sizeof(madt)); assert(discover().reserved==13);
    setup(3); madt[9]++; assert(discover().reserved==1 && reads==0);
    setup(3); gtdt[9]++; assert(discover().reserved==2 && reads==0);
    setup(3); put32(gtdt+56,29); checksum(gtdt,sizeof(gtdt)); assert(discover().reserved==28 && reads==0);
    setup(3); put32(gtdt+56,26); put32(gtdt+64,27); checksum(gtdt,sizeof(gtdt));
    c=discover(); assert((c.kind&INFINITY_ARM_CONTROLLER_KIND_MASK)==3);
    assert((c.kind&INFINITY_ARM_CONTROLLER_VIRTUAL_TIMER)!=0);
    assert(((c.kind>>INFINITY_ARM_CONTROLLER_TIMER_SHIFT)&255)==27 && c.reserved==0);
    setup(3); madt[45]=0; checksum(madt,sizeof(madt)); assert(discover().kind==0 && reads==0);
    setup(3); madt[45]=255; checksum(madt,sizeof(madt)); assert(discover().kind==0 && reads==0);
    setup(3); put32(madt+44+12,0); checksum(madt,sizeof(madt)); assert(discover().kind==0 && reads==0);
    setup(3); put32(madt+44+80+24+12,UINT32_MAX); checksum(madt,sizeof(madt)); assert(discover().kind==0 && reads==0);
    setup(3); put32(madt+44+80+24+12,0x20000); checksum(madt,sizeof(madt)); assert(discover().kind==0 && reads==1);
    setup(4); assert(discover().kind==0 && reads==0);
    setup(2); put64(madt+44+32,0); checksum(madt,sizeof(madt)); assert(discover().kind==0);
    setup(2); assert(infinity_arm_controller(madt,43,gtdt,sizeof(gtdt),2,read_typer,0).kind==0);
    return 0;
}
