# InfinityOS reference hardware

| Capability | x86_64 QEMU | AArch64 QEMU | AArch64 VirtualBox | x86 QEMU |
|---|---|---|---|---|
| Serial diagnostics | TESTED | TESTED | UNSUPPORTED | TESTED |
| Linear framebuffer | TESTED | TESTED | TESTED | UNSUPPORTED |
| Splash animation | TESTED | TESTED | TESTED | UNSUPPORTED |
| Keyboard | TESTED, PS/2 | TESTED, UEFI/direct USB HID | TESTED, direct USB HID | TESTED, PS/2 |
| Mouse | TESTED, PS/2 menu clicks | BRIDGE READY, UEFI/USB HID | TESTED, USB HID button activation | TESTED, PS/2 |
| Interrupt controller | NOT IMPLEMENTED | NOT IMPLEMENTED | NOT IMPLEMENTED | NOT IMPLEMENTED |
| Timer driver | NOT IMPLEMENTED | NOT IMPLEMENTED | NOT IMPLEMENTED | NOT IMPLEMENTED |
| Storage driver | TESTED, ATA PIO whole-disk installer | NOT IMPLEMENTED | TESTED, retained UEFI Block I/O to NVMe VDI | NOT IMPLEMENTED |
| Network driver | NOT IMPLEMENTED | NOT IMPLEMENTED | NOT IMPLEMENTED | NOT IMPLEMENTED |

The x86_64 ATA PIO capability is intentionally scoped to the disposable QEMU
`pc` test profile. The AArch64 VirtualBox path uses a retained firmware Block I/O
bridge and requires the VDI on its own NVMe controller; it is not yet a native
NVMe driver. AHCI, native NVMe, virtio-block, USB storage, and real-hardware safety
classification remain future work.
The splash delay uses bounded processor spinning and is not presented as a timer.
The AArch64 bridge deliberately retains boot services for firmware-managed USB
keyboard and pointer input; it is an implemented transition path, not the future
native xHCI driver. QEMU confirms ARM pointer discovery but does not forward its
synthetic tablet events through EDK2 after handoff, so that column does not claim
end-to-end click proof.

## Pointer discovery detail

| Pointer class | Implementation state | Verification state |
|---|---|---|
| Standard relative PS/2 | IMPLEMENTED | TESTED by x86/x86_64 QEMU menu activation |
| PS/2 IntelliMouse wheel | IMPLEMENTED | TESTED by host packet suite; hardware/VM extension negotiation untested |
| PS/2 Explorer five-button | IMPLEMENTED | TESTED by host packet suite; hardware/VM extension negotiation untested |
| UEFI Simple Pointer | IMPLEMENTED | TESTED for discovery; firmware delivery varies after handoff |
| UEFI Absolute Pointer | IMPLEMENTED | TESTED on AArch64 VirtualBox |
| USB HID boot mouse | IMPLEMENTED | TESTED by AArch64 QEMU menu activation |
| USB HID absolute virtual tablet | IMPLEMENTED | TESTED on AArch64 VirtualBox |
| Wheel and buttons in supported USB reports | IMPLEMENTED | TESTED by host packet suite |
| Multitouch gestures and pen pressure/tilt | PLANNED | NOT TESTED |
| Native xHCI/EHCI controller ownership | PLANNED | NOT TESTED |

Trackballs and 2.4 GHz receiver mice normally present as relative USB HID mice;
firmware-paired Bluetooth pointing devices can present through the UEFI pointer
protocols. This does not claim that InfinityOS can pair a Bluetooth device itself.
