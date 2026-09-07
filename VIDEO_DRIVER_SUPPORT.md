# Native graphics support — implementation status

Firmware GOP compatibility is **not** counted as native GPU support.

| Device family | Native status |
| --- | --- |
| VMware SVGA II / compatible PCI VMSVGA, x86-64 | Initial native FIFO scanout-update backend; kernel PCI discovery and register/MMIO transport implemented. Requires an already-enabled matching 32-bit scanout and mapped 32-bit BARs. |
| VirtualBox VMSVGA on ARM64 | Not implemented: PCI discovery/address mapping/register transport required. |
| VBoxSVGA / VBoxVGA, Bochs / QEMU standard VGA | No native modesetting driver yet. |
| Virtio-GPU | No native driver yet. |
| Intel, AMD, NVIDIA | No native driver yet; support must be tracked per GPU generation and PCI ID. |
| Apple GPU/display controller | No native driver yet; requires Apple-specific display and GPU support. |

The initial SVGA backend is **not a complete graphics driver**: it does not yet independently establish a mode, switch resolution, accelerate rendering, expose hardware cursors, drive multiple monitors, suspend/resume, or recover a reset GPU. It must not be marketed as universal SVGA support.

## Implemented path

- Kernel-owned PCI SVGA II discovery (`15ad:0405`), active scanout verification, FIFO ownership, bounded UPDATE commands, and one host notification per damage batch.
- Application surfaces remain isolated from hardware. Presentation submits only compositor damage. FIFO backpressure retains damage for a later presentation; there is no BUSY spin loop in input/presentation paths.
- Firmware fallback accepts canonical 32-bit RGB/BGR bitmask modes, validates scanout allocation, and prefers resolutions that fit the persistent buffer. Noncanonical/16-bit and BLT-only GOP modes remain unsupported.
- BootInfo ABI remains unchanged. Both live and installed kernels contain the driver; both ESP variants contain the same updated loader.

## Evidence and limits

- `make video-driver-test`: executable mode-policy tests; production SVGA command tests covering clipping, wrap, saturation, malformed pointers and rejection without mutation; the same production FIFO implementation drives QEMU's actual `vmware-svga` device through qtest for more than one full FIFO cycle, asserting host consumption and observed pointer wraparound.
- The qtest fixture establishes the initial mode itself. It validates the native FIFO implementation, **not guest PCI binding, firmware handoff, desktop performance, or VirtualBox hardware compatibility**.
- `make x86_64` and `make aarch64`: kernel, loader and ISO builds.
- `tools/installed-kernel-parity-test.py`: exact installed ELF payload and exact live/installed ESP loader bytes.
- Installed-guest runtime verification remains outstanding. No existing VM disks or graphics settings were changed.

## Remaining driver work

1. Complete SVGA modesetting, scanout allocation/mapping, hardware cursor, reset recovery and guest boot tests; add ARM64 PCI transport.
2. Implement Virtio-GPU 2D resource/scanout/transfer/flush and fences; separately verify each PCI/MMIO transport.
3. Implement Bochs/VBoxSVGA mode control and guest resize integration.
4. Select physical Intel/AMD/NVIDIA/Apple test hardware and implement per-generation modesetting, memory management, interrupts and firmware loading before acceleration.

No unsupported PCI ID is silently assigned a native driver.

Protocol references: [QEMU VMware SVGA device implementation](https://github.com/qemu/qemu/blob/master/hw/display/vmware_vga.c), [UEFI graphics output specification](https://uefi.org/specs/UEFI/2.10/12_Protocols_Console_Support.html), [QEMU standard VGA specification](https://www.qemu.org/docs/master/specs/standard-vga.html).
