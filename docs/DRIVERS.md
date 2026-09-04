# InfinityOS driver foundation

Milestone 1 begins with three typed identities: `display0`, `keyboard0`, and
`mouse0`. They are registry identities, not paths or file-like interfaces.

## Lifecycle

The current fixed registry follows `discover -> match -> initialize -> ready`.
It uses no heap allocation. Each `DeviceIdentity` records a typed device kind,
driver name, and operational state. This intentionally small representation can
later become the kernel-side capability boundary for isolated driver services;
kernel residency is a temporary implementation fact, not part of the contract.

## Display capability

The UEFI loader queries every directly writable GOP mode and selects the mode
with the largest pixel area before it captures the final memory map. It then
passes a firmware-neutral linear framebuffer description. The
kernel validates address, size, dimensions, stride, and format before creating
a `DisplayDevice`. Pixel writes are bounded and volatile. The device supports
background blitting, alpha compositing, particles, pixels, and a small diagnostic
font—only what the boot experience requires. The splash scales and center-crops
its raster background, reveals a generated transparent luminous emblem in 120
center-out steps, and moves an interpolated energy pulse independently of loading
milestones. After loading, three phase-offset comet streams continue tracing the
exact infinity spline through both loops and the center crossover at a timer-capped
60 Hz while the seven-row terminal remains interactive. Each moving particle has
a cool-blue halo, a white-hot core, and a short luminous star trail; the heads add
cinematic axial and diagonal rays. The particles continue after loading and their
positions are restored and redrawn independently of progress. There is no GPU driver,
compositor, or UI toolkit.

## Input capability

`InputEvent` separates source, action, and HID-compatible key code.
`PointerEvent` is the transport-neutral pointer contract: signed relative motion,
a five-button bitmap, vertical/horizontal wheel deltas, and a companion absolute
coordinate event. Initial discovery records transport and feature bits rather
than reducing every device to a Boolean `mouse present` result. The device registry
therefore reports the actual binding (`ps2-mouse`, `ps2-wheel-mouse`,
`ps2-explorer-mouse`, `uefi-relative-pointer`, `uefi-absolute-pointer`,
`usb-hid-mouse`, `usb-hid-absolute-pointer`, or `composite-pointer`).

The x86 PS/2 binding translates scan codes into keyboard press/release and
pointer events. During discovery it performs the standard sample-rate negotiation
for IntelliMouse wheel packets and Explorer five-button packets, falling back to
the standard three-byte protocol if either extension is absent. It decodes
extended arrow scan codes and preserves Shift for reverse Tab traversal. Those
events drive the two-card startup menu
and the installer's shared hover, focus, and activation model. All controller waits are bounded. The driver is polled
because interrupt infrastructure is outside this requested slice.

AArch64 VirtualBox exposes USB input rather than PS/2. Until InfinityOS owns an
xHCI stack, `keyboard0` binds through the UEFI simple-input protocol and keeps
boot services active. `mouse0` can bind through UEFI pointer protocols and also
releases the firmware HID transfer before directly polling boot-mouse reports.
The keyboard
bridge translates UEFI navigation scan codes for installer accessibility and is
interaction-tested; VirtualBox verifies USB button activation, while QEMU verifies
pointer discovery and device readiness. The loader enumerates every pointer handle
so an inactive legacy protocol cannot mask VirtualBox's active USB device. QEMU's
injected tablet events are not delivered through EDK2 after
handoff. Both retain typed identities and will later bind native USB capabilities without
exposing USB packets to higher layers.

Common device forms map onto those normalized transports as follows:

| Physical or virtual form | Initial binding | Normalized behavior |
|---|---|---|
| PS/2 mouse or emulated legacy mouse | PS/2 | Relative, 3 buttons |
| IntelliMouse-compatible wheel mouse | PS/2 extended | Relative, 3 buttons, wheel |
| Explorer-compatible mouse | PS/2 extended | Relative, 5 buttons, wheel |
| USB wired/wireless mouse or trackball | USB HID boot / UEFI Simple Pointer | Relative motion and buttons; optional report wheel/buttons are decoded |
| USB tablet, virtual tablet, or firmware touchpad | USB HID absolute / UEFI Absolute Pointer | Normalized absolute coordinates and buttons |
| Firmware-exposed Bluetooth pointer | UEFI relative/absolute pointer | Same normalized contract; pairing remains firmware-owned |

This is single-pointer support. Multitouch contacts, pressure/tilt, touch gestures,
Bluetooth pairing, and vendor-specific programmable controls are not claimed.
They require dedicated HID report parsing or later native controller services.
The current AArch64 USB bridge is transitional; it is not falsely presented as a
native xHCI implementation.

The same driver modules are compiled into the recovery ISO kernel and the installed
System Generation kernel. `make input-regression-test` executes packet-decoding,
discovery-metadata, and bounded-redraw state scenarios. `build.sh` runs those
behavioral scenarios against the same source used for the completed images.

VirtualBox's new-VM default of `PS/2 Mouse` is not a functional pointer source
on the ARM virtual machine. `builds/configure-virtualbox-arm64.sh <VM>` applies
and verifies the supported `USB Tablet` + `USB Keyboard` + xHCI-only hardware
profile. Legacy OHCI remains disabled so VirtualBox cannot route the tablet
away from the xHCI-backed UEFI HID path. This VM-level requirement cannot be
changed by software inside an ISO.
The build-time guard applies that profile to attached powered-off VMs, while the
packaged `start-virtualbox-arm64.sh` launcher enforces it before every start.

## Unsafe boundaries

Unsafe operations are contained in architecture output, the PS/2 port-I/O
driver, and framebuffer volatile access. Architecture-independent registry and
event code contain no raw port or MMIO operations. DMA is not required yet.
