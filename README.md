# InfinityOS

InfinityOS is a freestanding, multi-architecture operating-system foundation.
Milestone 2 boots through the existing driver and cinematic display path into a
bounded startup selector, interactive Infinity Console, and typed Intent Runtime.

## Build and test

On macOS with Homebrew:

```sh
brew install rust llvm lld nasm mtools xorriso qemu
./build.sh
make test
```

`build.sh` cleans the intermediate build, compiles every supported architecture,
validates each ISO boot catalog, and writes release artifacts and checksums to
`builds/`:

- `InfinityOS-x86.iso` — 32-bit BIOS boot for Intel/AMD hosts;
- `InfinityOS-x86_64.iso` — x86_64 UEFI for Intel/AMD hosts;
- `InfinityOS-aarch64.iso` — ARM64 UEFI for VirtualBox on Apple Silicon.

### VirtualBox on this Mac

Create a new VM with type `Other/Unknown (ARM 64-bit)`, 512 MB or more memory,
EFI firmware, VMSVGA graphics, xHCI enabled, and a USB Tablet. Attach
`builds/InfinityOS-aarch64.iso` as the VirtioSCSI optical disk. Put the writable
virtual hard disk on a separate NVMe controller: VirtualBox ARM currently marks
multi-target VirtioSCSI boot support as unimplemented, so a hard disk beside the
DVD on the same controller may not be published to UEFI Block I/O. This setup
has been booted, input-tested, and disk-discovery-tested locally with VirtualBox
7.2.16 on Apple M2 Max.

VirtualBox currently defaults a new ARM64 guest to a PS/2 mouse and keyboard.
That combination does not deliver pointer motion on its ARM machine. With the
VM fully powered off, apply the required input profile before first boot:

```sh
builds/configure-virtualbox-arm64.sh "<VM name>"
```

The helper verifies `USB Tablet`, `USB Keyboard`, and `xHCI` rather than merely
assuming that VirtualBox accepted the settings.

For normal use, start the VM through the guarded launcher:

```sh
builds/start-virtualbox-arm64.sh "<VM name>"
```

It repairs a powered-off VM before launch and refuses to start an incompatible
running configuration. `build.sh` also audits every registered ARM64 VM already
attached to an InfinityOS ISO, so rebuilding cannot silently leave one on PS/2.

Apple Silicon cannot run the x86 or x86_64 images through hardware
virtualization. Those artifacts are provided for Intel/AMD machines.

## Milestone 1 input and display slice

The UEFI targets now render generated high-resolution eclipse and transparent
glass-light emblem assets with a 120-step kernel-animated reveal, traveling pulse,
three persistent star/orb streams with luminous trails that trace the infinity
ribbon at 60 Hz, a gray loading-status bar, and a seven-row interactive command
surface with a mouse-driven menu to its right.
Typing `hello` produces `Hello there!`. x86_64 uses bounded PS/2 input. AArch64
currently uses typed UEFI keyboard and pointer bridges; native xHCI ownership
remains the next driver step.

## Milestone 2 console and intent foundation

After drivers initialize, InfinityOS presents mouse-hoverable Console and
InfinityOS Installer cards to the right of the CLI. The console supports bounded editing, exact commands, a small built-in
natural-language resolver, typed operations, an explicit future policy boundary,
serial-mirrored output, and a provider-neutral future AI interface. The startup
prompt uses a framebuffer-drawn infinity mark followed by `->`.

## Milestone 3A installation and storage

The x86_64 QEMU target can discover a real ATA virtual disk, explain the Infinity
Device/Container/Pool/Space hierarchy, build a typed whole-disk plan, require the
exact confirmation `ERASE`, create verified GPT and EFI structures, initialize
versioned Infinity metadata, install the System kernel, and boot from that disk
without installation media. Its graphical controls support mouse hover/click,
visible focus, Tab/Shift+Tab, all four arrow keys, Enter, and Escape.

```sh
make reset-test-disk
make install-test
make boot-installed
```

See [docs/INSTALLER.md](docs/INSTALLER.md), [docs/STORAGE.md](docs/STORAGE.md),
and [docs/STORAGE_FORMAT.md](docs/STORAGE_FORMAT.md).

See [docs/CONSOLE.md](docs/CONSOLE.md),
[docs/INTENT_RUNTIME.md](docs/INTENT_RUNTIME.md), and [docs/AI.md](docs/AI.md).

Individual developer targets remain available as `make x86`, `make x86_64`,
and `make aarch64`. See [docs/BOOT.md](docs/BOOT.md) for the exact sequence and
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the milestone boundary.
