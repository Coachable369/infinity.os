# ARM kernel allocation repair — 2026-10-13

## Root causes

The former installed ARM ELF placed its 1.5 GiB speech arena in ordinary BSS.
Its BSS began at `0x403e3000` and occupied `0xa1e48940` bytes, extending into
VirtualBox's device aperture at `0xe0000000`. More configured RAM cannot turn
that device region into RAM.

Moving the arena above 4 GiB exposed a second conflict: the ARM EFI loader's
implicit PE preferred base was `0x140000000`, inside the arena. The firmware
map showed LoaderCode at that address.

## Repair

- Both ARM linker layouts give the existing speech arena its own zero-filled
  load segment at `0x100000000`, size `0x60000000`. No inference/runtime logic
  changes; the QEMU layout retains its different low kernel base.
- Explicit ELF program headers keep low data/BSS and the high arena separate.
  Link assertions prevent low sections growing into the reserved region.
- The ARM EFI loader prefers `0x08000000`, below the canonical kernel.
- Allocation-failure memory-map diagnostics now use the firmware console,
  making them available on VirtualBox rather than only QEMU's UART address.
- Release verification parses the actual live and installed ELF headers from
  each ISO and rejects ARM load ranges intersecting the device aperture. It
  also checks the ARM EFI loader's PE base and image extent.

## Evidence and boundaries

- Behavioral ELF tests reject the old exact BSS extent and one-byte aperture
  crossings, and accept separate low/high ranges. Loader reservation/rollback
  tests also pass.
- Build manifest `20261013T092854234994Z-43764.json` produced diagnostic media
  containing the real installed ARM kernel and corrected EFI loader.
- A separate 12 GiB VirtualBox VM (`infinity-kernel-layout-probe`) reached the
  graphical startup menu. Screenshot: `builds/validation/kernel-layout/boot.png`.
  Failure map: `loader-collision.log`; successful boot diagnostics:
  `successful-boot.log`, under that same directory.
- This test proves firmware allocation and kernel startup, not installation or
  an authenticated desktop session. The user's VM and disk were not modified.
- Replacement canonical release ISOs require a successful full build; until
  that completes, the existing ISOs still contain the previous broken layout.
