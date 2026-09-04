# InfinityOS boot contract through Milestone 6

The x86_64 live installer profile requires at least 512 MiB of RAM. The live
kernel carries the verified installed System Generation so installation can
complete without a network or host dependency. Installed-disk boot remains
valid in the 256 MiB automated profile.

## x86_64 — implemented and QEMU-tested

Build with `make x86_64`; boot-test with `make test-x86_64`. The resulting
`build/infinity-x86_64.iso` is also the VMware UEFI installation/boot medium.

1. UEFI discovers `EFI/BOOT/BOOTX64.EFI` in the ISO's El Torito FAT image.
2. The InfinityOS loader initializes COM1 and scans bounded GPT entries for a
   valid Infinity Container. A completed version-1 container directly locates
   its Pool, Space table, Boot Catalog, ACTIVE System Generation, System Manifest,
   and bounded CORE component table. The manifest supplies the kernel reference,
   length, architecture, and CRC-32. If no installed generation exists, installation
   media falls back to `EFI/INFINITY/KERNEL.ELF` through UEFI filesystem access.
3. The loader verifies ELF magic, class, byte order, machine, program-header
   bounds, loadable-segment bounds, and entry containment.
4. All `PT_LOAD` segments are placed at their requested physical addresses. The
   milestone kernel is linked at physical address `0x04000000`, leaving enough
   low-memory space for the generated high-resolution boot assets and firmware allocations.
5. The loader allocates a 256 KiB initial stack, sized for bounded native-object
   generation verification, and creates InfinityOS-owned
   four-level page tables. Two-megabyte pages identity-map physical addresses
   `0x00000000` through `0xffffffff`; no virtual-memory policy is implied.
6. The final UEFI memory map is captured in `BootInfo`. No allocation occurs
   between that capture and `ExitBootServices` (a stale map key is retried once).
7. The architecture handoff disables maskable interrupts, installs a minimal
   flat GDT, reloads data selectors and `CS`, installs `CR3`, selects the new
   stack, and invokes the ELF entry using the System V x86_64 ABI.
8. The Rust kernel validates `BootInfo`, initializes COM1, renders the splash,
   binds framebuffer and PS/2 input devices, then enters the input polling loop.

An installed x86_64 disk contains `EFI/BOOT/BOOTX64.EFI` and
`EFI/InfinityOS/infinity.efi`. System Space points to the Boot Catalog rather than
to the kernel. The loader never scans the object store: it verifies every boot
record, every required CORE declaration, architecture compatibility, and exact
kernel bytes before handoff. Invalid or incomplete active generations emit a
recovery-media diagnostic and are never executed.

The installed profile reports `InfinityOS Native Boot`, mounts native System
objects, starts the runtime, and enters Infinity Console directly. The Recovery
Node menu exists only in the live recovery/installer profile.

### x86_64 kernel-entry state

- Execution mode: 64-bit long mode, ring 0.
- Paging: enabled; InfinityOS-owned 4 GiB identity map using 2 MiB pages.
- Interrupt state: `IF=0`; no IDT is promised or used.
- Direction flag: inherited from compliant UEFI application state (clear).
- Floating point/SIMD: unused; the Rust target disables SIMD and red-zone use.
- Stack: 64 KiB, downward-growing, 16-byte aligned before the handoff call.
- Register contract: `RDI = BootInfo*`; all other general registers unspecified.
- Return contract: kernel entry is divergent and must never return.

## x86 32-bit — implemented and QEMU-tested

The InfinityOS-owned boot sector is packaged as an El Torito floppy-emulation
image for broad BIOS compatibility. It loads the bounded second-stage payload,
enables A20, installs a flat GDT, enters 32-bit protected mode, validates and
loads an embedded ELF32 kernel at `0x00100000`, creates `BootInfo`, establishes
a stack at `0x00090000`, and invokes the Rust kernel using the 32-bit C ABI.
Build with `make x86`; test with `make test-x86`.

## AArch64 — implemented and QEMU-tested

Build with `make aarch64`; boot-test with `make test-aarch64`. The resulting
`build/infinity-aarch64.iso` is the native VirtualBox/VMware Fusion medium for
Apple Silicon. AArch64 UEFI loads `EFI/BOOT/BOOTAA64.EFI`; the common bounded loader
then validates and places the AArch64 ELF kernel at `0x10000000`, within
VirtualBox's ARMv8 RAM window. The automated QEMU profile packages the same
kernel source at `0x48000000`, because QEMU `virt` and VirtualBox expose
non-overlapping physical RAM windows in this milestone.

The architecture handoff validates the known
UEFI entry levels (EL1 under QEMU and EL2 under VirtualBox), installs a
16-byte-aligned 64 KiB stack, and branches to the ELF entry with
`x0 = BootInfo*`. Milestone 0 retains the active firmware translation regime
rather than introducing a premature MMU policy. The kernel writes to the UEFI framebuffer;
under EDK/QEMU it also uses the `virt` PL011 UART at `0x09000000` for automated
testing. Non-EDK firmware never accesses that QEMU-specific UART.

BootInfo also preserves the UEFI Runtime Services pointer. The AArch64 installer
uses only its standard `ResetSystem` entry after the animated completion
countdown. On restart, the bounded loader probes attached fixed disks before the
recovery payload and selects a checksum-valid ACTIVE installed generation. Thus
a valid installed disk boots normally even when recovery media is still mounted;
detached-media boot remains the definitive independence test.

Milestone 1 temporarily retains AArch64 boot services and leaves firmware
interrupt delivery enabled so typed UEFI and direct USB-I/O HID bridges can service
USB input before the native xHCI driver exists. The loader enumerates every pointer
protocol plus boot-keyboard and boot-mouse interrupt endpoints; the kernel accepts
Simple Pointer deltas, normalized Absolute Pointer coordinates, and HID reports. x86_64 still exits boot services
before handoff. The bridge is an explicit transition boundary, not a permanent
kernel architecture.

## Failure behavior

Before firmware exit, loader failures are emitted to both UEFI text output and
COM1. After the successful handoff, serial output is kernel-owned. Bootstrap
success never returns to firmware.

x86_64 installed boot is TESTED. AArch64 UEFI recovery/install media and
target-disk generation discovery are TESTED in VirtualBox on Apple Silicon,
including a disk-only restart with the ISO detached. x86 BIOS has no
installed-generation path.
