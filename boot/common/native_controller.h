#ifndef INFINITY_NATIVE_CONTROLLER_H
#define INFINITY_NATIVE_CONTROLLER_H
#include <stdint.h>
#include <stddef.h>

/* Discovery is deliberately read-only. Unsupported topology leaves kind zero. */
typedef struct { uint32_t kind, reserved; uint64_t base, auxiliary; } InfinityNativeController;
typedef int (*InfinityControllerRead)(void *, uint64_t, uint64_t *);

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
// FUNC: infinity_arm_controller
// DESC: Selects the enabled BSP GIC and physical timer from checked ACPI tables, with bounded redistributor discovery.
// ------------------=
static inline InfinityNativeController infinity_arm_controller(const uint8_t *madt, size_t madt_bytes,
    const uint8_t *gtdt, size_t gtdt_bytes, uint64_t mpidr, InfinityControllerRead read, void *context) {
    InfinityNativeController none={0}, result={0};
    if (!nc_table(madt,madt_bytes,0x43495041,44) || !nc_table(gtdt,gtdt_bytes,0x54445447,80) ||
        nc_u32(gtdt+56)!=30 || (mpidr & ~UINT64_C(0xff00ffffff))) return none;
    uint64_t distributor=0,cpu=0,redistributor=0;
    unsigned version=0,controllers=0,processors=0,ranges=0;
    size_t at=44;
    for (;at+2<=madt_bytes;) {
        const uint8_t *p=madt+at; size_t length=p[1];
        if (length<2 || length>madt_bytes-at) return none;
        if (p[0]==11) {
            if (length<76) return none;
            if ((nc_u32(p+12)&1) && nc_u64(p+68)==mpidr) {
                ++processors; cpu=nc_u64(p+32); redistributor=nc_u64(p+60);
            }
        } else if (p[0]==12) {
            if (length<24) return none;
            ++controllers; distributor=nc_u64(p+8); version=p[20];
        } else if (p[0]==14) {
            if (length<16 || ++ranges>32) return none;
            uint64_t base=nc_u64(p+4), size=nc_u32(p+12);
            if (!nc_address(base,size,65536) || size<0x20000 || size>16*1024*1024) return none;
        }
        at+=length;
    }
    if (at!=madt_bytes || processors!=1 || controllers!=1 || !nc_address(distributor,65536,4096)) return none;
    if (version==2 && nc_address(cpu,8192,4096)) {
        result.kind=2; result.base=distributor; result.auxiliary=cpu; return result;
    }
    if (version!=3 || !read) return none;
    uint32_t affinity=(uint32_t)mpidr | (uint32_t)(mpidr>>8 & 0xff000000);
    if (redistributor) {
        uint64_t typer=0;
        if (!nc_address(redistributor,0x20000,65536) || !read(context,redistributor+8,&typer) ||
            (uint32_t)(typer>>32)!=affinity) return none;
    } else {
        unsigned matches=0,frames=0;
        for (at=44;at<madt_bytes;at+=madt[at+1]) {
            const uint8_t *p=madt+at; if (p[0]!=14) continue;
            uint64_t base=nc_u64(p+4),size=nc_u32(p+12),offset=0;
            while (offset<size) {
                if (size-offset<0x20000 || ++frames>128) return none;
                uint64_t typer=0;
                if (!read(context,base+offset+8,&typer)) return none;
                if ((uint32_t)(typer>>32)==affinity) { ++matches; redistributor=base+offset; }
                uint64_t stride=(typer&2)?0x40000:0x20000;
                if (stride>size-offset) return none;
                offset+=stride;
                if (typer&16) break;
                if (offset==size) return none; /* Region must end at a Last frame. */
            }
        }
        if (matches!=1) return none;
    }
    result.kind=3; result.base=redistributor; result.auxiliary=distributor; return result;
}
#endif
