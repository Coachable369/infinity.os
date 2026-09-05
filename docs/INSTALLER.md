# InfinityOS installer

## InfinityUI forward feature parity

The installer packages InfinityUI, the Skin Registry, Window Server, Clipboard,
official dark skin, alternate diagnostic skin, SafeSkin, semantic icons,
wallpaper, and fonts into every fresh System Generation. x86_64 and AArch64 ESP
images carry the versioned skin packages; the installed kernel carries the same
runtime code and default framebuffer asset. Installation verification reports
the InfinityUI and trusted-window policy checks explicitly. The live ISO is not
an installed-system dependency.

## Mandatory live and installed parity

Fresh installation is part of each feature's acceptance boundary. Any new
runtime service, UI asset, font, policy, schema, or registry added to the live
ISO must be packaged into and verified from the installed System Generation.
The default 45-family font library is stored in both live and installed EFI
System bootstrap images, whose registered component checksum covers that exact
payload. The desktop artwork and smooth early-UI atlas are compiled into the
same installed kernel/runtime image used after installation media is removed.

## Milestone 5.x installed generation

The deterministic write path is GPT and EFI, Container/Pool/Spaces, Generation 1
INSTALLING metadata, central CORE component table, kernel and native System
objects, byte-for-byte/CRC verification, READY manifest, EFI verification,
ACTIVE manifest/catalog publication, then the complete container marker. Any
error stops before activation. The completion page reports only checks which the
provisioner actually performed.

The graphical progress screen is driven by those same successful provisioner
checkpoints; it never advances on a failed write or verification. A generated
activation scene, traced infinity pulse, verified-stage markers, neutral progress
rail, and current operation label remain visible until the complete marker is
verified. The completion action starts a three-second animated reboot transition
and only then requests firmware reset.

The System Space boot reference is a compact catalog, not a direct kernel sector
and not a full namespace query. The installed ESP contains both the firmware
fallback path and the InfinityOS vendor path. The installer payload is explicit:
the build embeds the separately built installed kernel and ESP, while the central
component registry declares all nine required CORE roles and checksums them. The
ninth role is the native AI/local-ML/voice/agent runtime suite.

Future components must register with an install class (`CORE`, `SYSTEM OPTIONAL`,
or `POST-INSTALL`) and pass this clean-install/detached-media test.

## Milestone 4 service integration

The installer is now the optional `Installer Service` in the shared Infinity Runtime, not a separate installer OS. On the live ISO its Execution Context receives time-limited Storage.Discover, Storage.Provision, Boot.Install, and System.Install capabilities. Exact destructive confirmation and completion are security Record events. Successful installation revokes the lease immediately.

Provisioning installs and verifies native System objects in addition to the
bootloader and kernel: `/system/runtime`, `/system/service-registry`,
`/system/capability-policy`, `/system/models/local-intent-v1`,
`/system/ai/bootstrap`, `/system/voice/runtime`, and
`/system/agents/policy`, and `/system/organization/schema`. They contain versioned compact binary records, never
host files or JSON. Model Object ID/checksum linkage is validated at boot. The
policy object includes the initial durable `System.ServiceInstalled` audit
record. The installed kernel contains the same runtime/service components and
boots without the ISO; its profile does not start Installer Service.

The verified dependency chain is firmware, bootloader, kernel, runtime core,
device/event, storage, object, namespace, console, Local ML, Infinity AI, Voice,
and Agent services. Full namespace state is not needed to find the bootstrap
objects, and model availability never blocks deterministic console use.

Milestone 3A replaces the navigation-only placeholder with a deterministic
text-graphical wizard. It requires no model or network service.

## Flow

The implemented path is Welcome, storage comparison, hierarchy explanation,
disk discovery, device details, date/time configuration, provisioning preview,
destructive confirmation popup, provisioning, verification, and completion.
Escape returns to the startup selector until a commit is running. Storage uses
one truthful policy: the selected disk becomes an Infinity Pool whose System,
Personal, Applications, and Recovery Spaces share capacity dynamically.

The date/time page starts from the UEFI real-time clock when firmware exposes a
valid value, otherwise it uses a bounded UTC fallback. Date, 24-hour local time,
and one of the initial typed time-zone choices are keyboard and mouse editable.
The immutable provisioning plan carries the selection, and installation writes
it as the versioned native System metadata object
`/system/settings/date-time`. Verification remounts the Object Store and checks
that the exact selected value survived before Generation 1 may become ACTIVE.
System services can read the validated value through the storage service
boundary; it is not a decorative wizard-only choice.

The time-zone field and interactive world map are two projections of the same
typed selection. A map click resolves its longitude to the nearest supported
zone; keyboard Left/Right selection moves the highlighted longitude window and
representative latitude marker. Ordinary pointer motion updates only the cursor
damage region, so the map, glass panel, and photographic background do not
repaint or flash as the pointer travels across the screen.

Every interactive page exposes a visible keyboard focus ring and pointer hover
state over its Back and primary action controls. Tab moves between controls
(Shift+Tab reverses on PS/2); Left/Up and Right/Down provide equivalent navigation;
Enter activates the focused control; Escape cancels back to the startup selector.
Mouse movement tracks across the full installer surface and clicking a control
uses the same activation path as Enter.
Button activation requires an explicit UEFI contact bit or raw USB HID press
edge; stationary buttonless firmware notifications are ignored.

The reviewed plan remains visible when a focused confirmation popup warns that
all selected-disk data will be permanently erased. Cancel is available through
Escape, mouse, or keyboard focus; Erase & Install requires explicitly activating
that popup action. The automated test cancels once and proves no provisioning
marker appears before reopening and accepting the prompt.

## Layering and safety

The UI can only create a `StorageProvisioningPlan` and pass that immutable plan
to `StorageManager::provision`. ATA port operations exist only in the block-device
driver. GPT and Infinity-format writes exist only in the format provisioner.

The destructive implementation is compiled only into the x86_64 installation
kernel. The installed System kernel is built separately without the installer
payload. Current policy recognizes only the primary legacy ATA disk exposed by
the controlled QEMU `pc` test profile. No host disk path is visible to the guest.

## Verification and incomplete installs

The container state advances through `provisioning`, `installing`, `verifying`,
and finally `complete`. The bootloader accepts only `complete`. Before that final
state is committed, the installer re-reads and validates both GPT headers, the
partition-entry CRC, every byte of the EFI image, all native metadata checksums,
the four Space records, and every installed-kernel byte.

Critical errors stop the operation, emit a specific serial diagnostic, and never
write the complete marker.

## Reproducible test

```sh
make reset-test-disk
make install-boot-test
make boot-installed
```

`make install-boot-test` performs all three actions itself: creates a zeroed 128 MiB
known test artifact, drives the wizard, and then boots the resulting disk in a
second QEMU process with no CD-ROM or ISO attached.
The completion screen's Enter action, countdown markers, and final firmware-reset
request are tested against QEMU's ACPI reset control with an 8042 CPU-reset
fallback before the independent installed-disk boot begins. AArch64 receives the
UEFI Runtime Services table through BootInfo and invokes `ResetSystem`, replacing
the former idle-loop placeholder.
It then corrupts copies of the known test disk to prove that an INSTALLING
manifest and a checksum-invalid kernel are rejected before kernel handoff.

## Tested VM configuration and support

| Item | Tested value |
|---|---|
| Architecture | x86_64 |
| Firmware | QEMU EDK2/OVMF UEFI |
| Machine | QEMU `pc` (i440FX/PIIX legacy ATA) |
| Memory | 256 MiB |
| Target | 128 MiB raw image as primary-master ATA |
| Installer media | InfinityOS x86_64 ISO as secondary-master CD-ROM |
| Installed boot | target disk only; no CD-ROM device attached |

AArch64 VirtualBox discovers a writable NVMe-attached virtual disk through the
retained UEFI Block I/O bridge; the installer ISO should remain on VirtioSCSI.
The tested ARM path is whole-disk planning on a disposable 2 GiB VDI. 32-bit x86
still builds and boots without a supported storage device. Native AHCI, NVMe,
virtio-block, multiple-disk selection, unallocated-space installation,
coexistence, and advanced planning are not yet implemented.

## Milestone 8 payload

The installed kernel contains Network, Network Policy, Network Transport, and
Network Discovery service implementations and manifests; network capability,
IOP, and IEF schema IDs; the Settings inspector; and native network bootstrap
and profile-state objects. These are CORE generation content, not live-media
helpers. Host artifact validation is **IMPLEMENTED**; full clean-install NIC
acceptance is **UNSUPPORTED** until a wire driver exists.
# Milestone 7 payload

The System Space bootstrap now includes the versioned identity-state object and
the Identity, Authentication, Session, Settings, Onboarding, and Shell service
IDs in the compact installed component/service registries. The live installer does not run
first-boot identity creation; the independently booted installed generation
does so after its object store is online.
