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
- Clean release manifest `20261013T154836743563Z-35188.json` passed on
  revision `6b427dbc1cc481e04ba746b9477acdb5126563b1` in 5066.764 seconds.
  Both canonical ISOs in `builds/` now contain the repair; full-bundle checks
  validated their live and installed kernel load ranges and loader placement.
- Independent SHA-256 verification passed for both files, recorded in
  manifest `20261013T171342305905Z-41691.json`.
- The exact canonical `builds/InfinityOS-aarch64.iso` booted to the graphical
  installer menu in the separate 12 GiB VirtualBox test VM. Screenshot:
  `builds/validation/kernel-layout/release-boot.png`. The test VM was powered
  off afterward. This remains ISO boot proof, not a new installation test.

Release SHA-256 values:

| ISO | SHA-256 |
| --- | --- |
| `InfinityOS-aarch64.iso` | `fe682249fa4c40e4a4a8dffb068d6df6d46b5aac86536835bea69e7c7b6287ea` |
| `InfinityOS-x86_64.iso` | `f3ea15d203fd51375d3eb081c74ef71cbda451d9c10aa64af39ed6e292161000` |

## Release probe compatibility

Full-build manifest `20261013T093417554877Z-44682.json` failed at
`native-https-arm-test`. That minimal probe reuses `linker/aarch64-qemu.ld`
without linking the speech runtime. The QEMU linker now permits an absent
speech arena, but still rejects any nonzero arena other than 1.5 GiB. The
canonical ARM linker retains its mandatory 1.5 GiB assertion; release artifact
aperture checks remain unchanged.

Focused manifest `20261013T144204408818Z-44117.json` successfully linked and
ran the ARM native HTTPS probe (guest exit 0, DHCP configured). It does not
establish public-site HTTPS or installed-system acceptance. The subsequent
clean release and canonical ARM ISO boot check are recorded above.
