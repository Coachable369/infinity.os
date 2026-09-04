# InfinityOS architecture through Milestone 7.x

## Installability invariant

Every completed architecture milestone must also produce an equivalently updated installable InfinityOS system. A feature is not integrated until it is declared in the component registry, installed on a clean target, survives reboot, and is verified while booting with installation media physically absent.

This applies to every forward implementation: runtime services, interface
assets, font resources, policies, schemas, registries, and user-visible
behavior. Live-media-only payloads are prohibited unless explicitly classified
as installer-only. Architecture-specific boot mechanisms may differ, but a
freshly installed System Generation must preserve feature parity after the ISO
is detached.

## Milestone 4 runtime boundary

Drivers initialize privileged mechanisms, then the Infinity Runtime constructs Execution Contexts and starts services by dependency. Live and installed environments share the same modules. IOP provides bounded typed request/response; IEF provides bounded typed notifications; capabilities are checked at use and delivery time; services own authoritative state.

Kernel mechanisms are context creation/destruction, scheduling hooks, endpoint queues, basic capability validation, interrupt boundaries, memory-region ownership, and accounting. Object/namespace logic, service policy, and routing policy remain above those mechanisms, although this bring-up image is statically linked.

Architecture-neutral interfaces and logical region isolation are TESTED. Page-table isolation and hardware context switching are SCAFFOLDED. Core code compiles for x86, x86_64, and AArch64. Installed-disk end-to-end acceptance targets x86_64 UEFI/QEMU first.

System Space stores versioned runtime, registry, policy, kernel, local-model,
AI-bootstrap, voice-framework, and agent-policy objects so a future generation
can bind them atomically. These are typed native objects, not traditional files.
Rolling/side-by-side upgrades are PLANNED; operation and manifest versions
already avoid simultaneous-upgrade assumptions.

## Milestone 6 AI boundary

Infinity AI, Local ML, Voice, and Agent services sit above the Runtime Core.
The kernel provides only Execution Context, scheduling, IPC, capability, memory,
and accounting mechanisms. Provider selection, model registry, context policy,
intent planning, tool policy, and agent lifecycle remain modular runtime/service
logic. Deterministic console operations execute without a model; unresolved
human language may request a local typed plan which must pass ordinary policy.

CPU inference and host security/resource tests are TESTED. Remote providers,
speech models, native audio, accelerator inference, and MMU-enforced service
separation are explicitly not claimed. See `docs/AI.md`.

Editable diagrams live in `assets/architecture/`.

## Implemented now

InfinityOS currently has one deliberately narrow vertical slice:

- InfinityOS-owned x86_64 and AArch64 UEFI applications;
- a bounded, architecture-parameterized ELF64 loader isolated in the bootstrap;
- a versioned, firmware-neutral `BootInfo` ABI;
- architecture handoff assembly that owns GDT, page tables, stack, and register
  translation;
- an architecture-independent Rust kernel entry, framebuffer output, and
  architecture-specific serial/idle code;
- bootable BIOS/EFI ISOs and automated serial-output smoke tests for all targets;
- a fixed typed device registry, a bounded framebuffer capability, and PS/2
  keyboard/mouse bindings that emit structured input events;
- a bounded startup selector and architecture-independent Infinity Console;
- exact and built-in intent resolvers producing a closed typed-operation enum;
- an explicit operation-policy seam and dispatcher;
- a native local CPU intent model, provider router, Model Registry, Context
  Broker, Tool Broker, voice boundary, and bounded Agent Manager;
- a deterministic installer with typed storage discovery, planning,
  provisioning, and verification boundaries;
- standards-compatible protective MBR/GPT and EFI boot infrastructure beneath
  versioned Infinity Container, Pool, and Space metadata;
- installed-disk boot discovery that resolves the System kernel through
  completed container metadata.

ELF is a bootstrap container choice, not an operating-system object or
application model. UEFI structures stop at the loader boundary. The kernel sees
only this 112-byte, C-layout contract:

| Offset | Field | Meaning |
|---:|---|---|
| 0 | `magic: u64` | `0x494e46424f4f5430` |
| 8 | `version: u32` | Contract version `4` |
| 12 | `architecture: u32` | `1` x86, `2` x86_64, `3` AArch64 |
| 16 | `memory_map_address: u64` | Physical address of captured descriptors |
| 24 | `memory_map_size: u64` | Valid bytes at that address |
| 32 | `memory_descriptor_size: u64` | Firmware descriptor stride |
| 40 | `firmware_revision: u64` | UEFI system-table revision |
| 48 | `boot_flags: u64` | Bit 0 QEMU UART, bit 1 firmware key input, bit 2 simple pointer, bit 3 absolute pointer, bit 4 enumerated pointer set |
| 56 | `framebuffer_address: u64` | Physical address of linear framebuffer |
| 64 | `framebuffer_size: u64` | Framebuffer extent in bytes |
| 72 | `framebuffer_width: u32` | Visible width in pixels |
| 76 | `framebuffer_height: u32` | Visible height in pixels |
| 80 | `framebuffer_stride: u32` | Pixels between adjacent scan lines |
| 84 | `framebuffer_format: u32` | UEFI RGB/BGR format identifier |
| 88 | `firmware_input: u64` | Temporary AArch64 UEFI input bridge pointer |
| 96 | `firmware_pointer: u64` | Temporary AArch64 UEFI pointer bridge pointer |
| 104 | `firmware_runtime_services: u64` | UEFI Runtime Services table used for standard firmware reset |

The loader owns firmware interaction, validation, allocation, initial mappings,
and the irreversible handoff. The kernel owns all machine state afterward.

## Future InfinityOS direction — not implemented

InfinityOS still has no Unix process/filesystem substrate, networking stack,
graphical compositor, third-party application runtime, remote model integration,
speech model, or GPU/NPU inference backend. Storage drivers beyond the scoped
x86_64 ATA PIO installer capability are not implemented. No Unix or other legacy
general-purpose OS semantics have been introduced. The Infinity Container,
native object store, System objects, and human namespace are not a filesystem.

Milestone 6.5 adds a relational organization service and a schema-driven,
typed Console language. See `OBJECT_ORGANIZATION.md`, `CONSOLE.md`, and
`MILESTONE_6_5_COMPLIANCE.md`.

## Milestone 7.x InfinityUI boundary

InfinityUI adds architecture-neutral retained elements, logical layout, typed
input/focus, versioned pluggable skins, bounded rendering damage, trusted UI,
and capability-owned windows above kernel display/input mechanisms. The Skin
Registry, Window Server, InfinityUI, and Clipboard remain separate modular
services. The official default skin follows the supplied authentication gold
standard; alternate and SafeSkin packages prove replacement and recovery.

The fresh System Generation includes the same onboarding, authentication, and
desktop wallpapers, regular and semibold UI atlases, full redistributable font
catalog, compiled semantic/app icon pack, and Default Dark skin used by live
media. No installed UI asset is loaded from the host, network, or detached ISO.

See `INFINITYUI.md`, `SKINS.md`, `WINDOW_SERVER.md`, `UI_SECURITY.md`, and
`MILESTONE_7X_COMPLIANCE.md`.

## Known limitations and temporary mechanisms

- x86, x86_64, and AArch64 are implemented and QEMU-tested. AArch64 is also
  framebuffer-verified in VirtualBox on the Apple Silicon development host.
- The x86_64 map is a temporary low-4-GiB identity map using large pages. It is
  sufficient for the milestone and is not the future memory manager.
- AArch64 temporarily retains the UEFI-established identity mappings after
  firmware exit; replacing them is a later memory-management milestone.
- The AArch64 kernel source is packaged at different fixed physical addresses
  for QEMU and VirtualBox because their Milestone 0 RAM windows do not overlap.
  A future relocatable kernel loader will remove this packaging distinction.
- x86_64 is linked at `0x04000000`; AArch64 is packaged at `0x10000000` for
  VirtualBox and `0x48000000` for QEMU. Loaders obtain placement from ELF headers.
- The x86 BIOS loader advances real-mode destination segments and enforces a
  1024-sector bound below its protected-mode stack.
- `BootInfo` retains the firmware memory-map bytes because that information is
  expensive to reconstruct. The kernel does not interpret them in this slice.
- The framebuffer is the portable visible output. COM1 on x86_64 and PL011 on
  EDK/QEMU AArch64 provide deterministic automated observation.
- The console uses fixed arrays: six 96-byte output rows and one 64-byte command
  buffer. There is no kernel heap.
- AArch64 keyboard and pointer input remain temporary UEFI bridges until native xHCI/HID.
- Destructive installation is implemented only for x86_64 UEFI QEMU with a
  legacy ATA test disk. AArch64 and x86 installation remain unsupported.
# Milestone 7 identity boundary

Installed boot now continues from System Generation validation into the same
Runtime Core used by the live environment, then loads the native identity state.
Identity, Authentication, Session, Settings, Onboarding, and Shell are modular
dependency-ordered services above kernel mechanism. The first shell consumes
typed service state and explicit session capabilities; it does not introduce
users, home directories, root, PIDs, or ambient filesystem authority. See
`IDENTITY.md`, `SESSIONS.md`, and `GUI_CLI_PARITY.md`.
