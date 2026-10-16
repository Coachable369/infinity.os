#ifndef INFINITY_NATIVE_CONTROLLER_H
#define INFINITY_NATIVE_CONTROLLER_H
#include <stdint.h>
#include <stddef.h>

/* Discovery is deliberately read-only. Unsupported topology leaves kind zero. */
typedef struct { uint32_t kind, reserved; uint64_t base, auxiliary; } InfinityNativeController;
typedef int (*InfinityControllerRead)(void *, uint64_t, uint64_t *);

#define INFINITY_ARM_CONTROLLER_KIND_MASK UINT32_C(0xff)
#define INFINITY_ARM_CONTROLLER_TIMER_SHIFT 8
#define INFINITY_ARM_CONTROLLER_VIRTUAL_TIMER UINT32_C(0x10000)

// ------------------------=
// FUNC: nc_failure
// DESC: Returns an unavailable controller descriptor with a stable structured discovery reason.
// ------------------=
static InfinityNativeController nc_failure(uint32_t reason) {
    InfinityNativeController result={0}; result.reserved=reason; return result;
}

// ------------------------=
// FUNC: nc_u32
// DESC: Reads an unaligned little-endian ACPI field.
// ------------------=
static uint32_t nc_u32(const uint8_t *p) {
    return (uint32_t)p[0] | (uint32_t)p[1]<<8 | (uint32_t)p[2]<<16 | (uint32_t)p[3]<<24;
}
// ------------------------=
// FUNC: nc_u64
// DESC: Reads an unaligned little-endian physical address.
// ------------------=
static uint64_t nc_u64(const uint8_t *p) { return nc_u32(p) | (uint64_t)nc_u32(p+4)<<32; }
// ------------------------=
// FUNC: nc_table
// DESC: Validates bounded table bytes before consuming any variable-length records.
// ------------------=
static int nc_table(const uint8_t *p, size_t bytes, uint32_t signature, size_t minimum) {
    if (!p || bytes < minimum || bytes > 1024*1024 || nc_u32(p) != signature || nc_u32(p+4) != bytes) return 0;
    uint8_t sum=0; for (size_t i=0;i<bytes;++i) sum+=p[i];
    return sum==0;
}
// ------------------------=
// FUNC: nc_address
// DESC: Accepts aligned nonzero MMIO ranges without wrapping the address space.
// ------------------=
static int nc_address(uint64_t address, uint64_t size, uint64_t alignment) {
    return address && !(address & (alignment-1)) && size && address <= UINT64_MAX-size;
}
// ------------------------=
// FUNC: nc_arm_controller_kind
// DESC: Encodes a validated firmware timer source while preserving legacy physical-PPI30 handoffs.
// ------------------=
static uint32_t nc_arm_controller_kind(uint32_t controller, uint32_t physical, uint32_t virtual_timer) {
    if (physical==30) return controller;
    if (virtual_timer>=16 && virtual_timer<32)
        return controller | (virtual_timer<<INFINITY_ARM_CONTROLLER_TIMER_SHIFT) | INFINITY_ARM_CONTROLLER_VIRTUAL_TIMER;
    return 0;
}
// ------------------------=
// FUNC: infinity_arm_controller
// DESC: Selects the enabled BSP GIC and a supported physical or virtual timer from checked ACPI tables, with bounded redistributor discovery.
// ------------------=
static inline InfinityNativeController infinity_arm_controller(const uint8_t *madt, size_t madt_bytes,
    const uint8_t *gtdt, size_t gtdt_bytes, uint64_t mpidr, InfinityControllerRead read, void *context) {
    InfinityNativeController result={0};
    if (!nc_table(madt,madt_bytes,0x43495041,44)) return nc_failure(1);
    if (!nc_table(gtdt,gtdt_bytes,0x54445447,80)) return nc_failure(2);
    uint32_t timer_kind=nc_arm_controller_kind(0,nc_u32(gtdt+56),nc_u32(gtdt+64));
    if (!timer_kind && nc_u32(gtdt+56)!=30) return nc_failure(28);
    if (mpidr & ~UINT64_C(0xff00ffffff)) return nc_failure(3);
    uint64_t distributor=0,cpu=0,redistributor=0;
    unsigned version=0,controllers=0,processors=0,ranges=0;
    size_t at=44;
    for (;at+2<=madt_bytes;) {
        const uint8_t *p=madt+at; size_t length=p[1];
        if (length<2 || length>madt_bytes-at) return nc_failure(4);
        if (p[0]==11) {
            if (length<76) return nc_failure(5);
            if ((nc_u32(p+12)&1) && nc_u64(p+68)==mpidr) {
                ++processors; cpu=nc_u64(p+32); redistributor=nc_u64(p+60);
            }
        } else if (p[0]==12) {
            if (length<24) return nc_failure(6);
            ++controllers; distributor=nc_u64(p+8); version=p[20];
        } else if (p[0]==14) {
            if (length<16 || ++ranges>32) return nc_failure(7);
            uint64_t base=nc_u64(p+4), size=nc_u32(p+12);
            if (!nc_address(base,size,65536) || size<0x20000 || size>256*1024*1024) return nc_failure(8);
        }
        at+=length;
    }
    if (at!=madt_bytes) return nc_failure(9);
    if (processors!=1) return nc_failure(10);
    if (controllers!=1 || !nc_address(distributor,65536,4096)) return nc_failure(11);
    if (version==2 && nc_address(cpu,8192,4096)) {
        result.kind=nc_arm_controller_kind(2,nc_u32(gtdt+56),nc_u32(gtdt+64));
        result.base=distributor; result.auxiliary=cpu; return result;
    }
    if (version!=3 || !read) return nc_failure(12);
    uint32_t affinity=(uint32_t)mpidr | (uint32_t)(mpidr>>8 & 0xff000000);
    if (redistributor) {
        uint64_t typer=0;
        if (!nc_address(redistributor,0x20000,65536) || !read(context,redistributor+8,&typer) ||
            (uint32_t)(typer>>32)!=affinity) return nc_failure(13);
    } else {
        unsigned matches=0,frames=0;
        for (at=44;at<madt_bytes;at+=madt[at+1]) {
            const uint8_t *p=madt+at; if (p[0]!=14) continue;
            uint64_t base=nc_u64(p+4),size=nc_u32(p+12),offset=0;
            while (offset<size) {
                if (size-offset<0x20000 || ++frames>128) return nc_failure(14);
                uint64_t typer=0;
                if (!read(context,base+offset+8,&typer)) return nc_failure(14);
                if ((uint32_t)(typer>>32)==affinity) { ++matches; redistributor=base+offset; }
                uint64_t stride=(typer&2)?0x40000:0x20000;
                if (stride>size-offset) return nc_failure(15);
                offset+=stride;
                if (typer&16) break;
                if (offset==size) return nc_failure(16); /* Region must end at a Last frame. */
            }
        }
        if (matches!=1) return nc_failure(17);
    }
    result.kind=nc_arm_controller_kind(3,nc_u32(gtdt+56),nc_u32(gtdt+64));
    result.base=redistributor; result.auxiliary=distributor; return result;
}
#endif
