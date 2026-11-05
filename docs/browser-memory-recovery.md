# Browser memory failure and recovery

The reported failure was native engine allocation exhaustion: failure 4,
496930624 peak reserved bytes, a failed 16777280-byte request, and no native
network failure. The old fatal callback spun forever and every subsequent
navigation queued against that stopped worker.

Large allocations (at least 1 MiB) now use a bounded 256-entry side table for
allocation metadata. A power-of-two surface therefore consumes its payload size
instead of twice that size due to an inline header. Small allocations retain the
existing path, and side-table saturation falls back to the inline representation.
The browser keeps its existing 512 MiB grant; it does not take memory from speech.

Fatal callbacks abandon the component's cooperative stacks through the shared
native context adapter. The supervisor publishes stopped state only after it
regains control. An explicit Reload or new address rechecks identity/network
permissions and time/entropy requirements. It waits for cancelled native network
loans to drain before restoring the complete private writable image, zeroing BSS
and heap, and starting a new engine. Failed continuations are never resumed.

The partial component linker preserves separate relocation sections. Packaging
classifies all writable ELF sections, rejects unsupported state, and renames them
for the final kernel link. ARM64 (both machine layouts) and x86_64 use the same
supervisor and include the same reset ranges in live and installed kernels.
Constructor addresses are immutable and excluded; initialized data is checkpointed
after relocation and before component initialization. Restart storage is bounded:
a shared 2 MiB entry stack, plus a 2 MiB checkpoint on ARM or 8 MiB on x86.
The measured initialized state is 600856 bytes on ARM and 5245872 bytes on x86;
the x86 component contains substantially larger initialized relocation tables.
Final-link assertions prevent either image from exceeding its reservation.

A fatal restart creates a fresh active page. Other open tabs, unsaved web forms,
and ephemeral engine session state cannot be recovered from a failed heap.
Favorites, browser preferences, the desktop and native network policy persist.
Page/TLS failures remain distinct from engine memory or panic failures. Hardware
exceptions or noncooperative infinite loops are not recovered by this boundary.

Verification entrypoints:
- `./build-kit browser-core-tests`: state/error projections and private-image restore.
- `./build-kit run python3 tools/servo-platform-probe/run-component-guest.py --recovery`:
  actual component allocation aborts, two fresh starts, navigation and pixels.
- `./build-kit browser-native-memory-tests`:
  native allocation accounting, alignment, realloc failure/preservation and reclamation.
- `./build-kit https-service-tests`: cancellation/loan draining and stale-handle isolation.
- Installed browser harness `--memory-recovery`: real oversized canvas allocations,
  followed by new-address navigation and rendering on a detached installed system.

These entrypoints describe coverage; individual execution receipts determine what
has actually passed. ISO packaging alone is not installed-system evidence.

Recorded results (2026-11-04):
- Core behavioral checks passed: `builds/manifests/20261105T001446843298Z-84263.json`.
- ARM native component recovery passed: `builds/manifests/20261105T002603924161Z-85907.json`;
  complete receipt in `builds/validation/browser-component-recovery-03.log`.
  Two real allocation aborts returned failure 4, each followed by a fresh engine.
  The third engine navigated to a different rendered page and shut down normally.
  Reserved memory did not grow across the two failed generations.
- The separate broad component scenario stopped at its tab-selection-after-download
  expectation before reaching memory recovery. It is not passing regression evidence.
- ARM freestanding allocation checks passed:
  `builds/manifests/20261105T005501137613Z-36725.json`. Alignment, exact large-block
  reservation, realloc preservation/failure, and complete reclamation were exercised.
- Native network actor lifecycle checks passed:
  `builds/manifests/20261105T005556421577Z-36996.json`. Cancelled loans drain before
  restart and stale handles cannot cancel a subsequent request.
- Installed ARM QEMU verification completed an unmodified installation, detached
  boot, account configuration/login, and real HTTPS rendering. Offline full-kernel
  comparisons were intentionally skipped. Preserved receipts and screenshots:
  `builds/validation/browser-recovery/`.
- Installed memory recovery remains **unverified**. The first two attempts did not
  navigate away from the initial page; their engine state remained healthy with
  no allocation failure. A final attempt to submit the stress URL through Console
  timed out opening the launcher while the browser retained focus. These failed
  preconditions do not demonstrate recovery or establish its failure.
  Attempt manifests: `20261105T015131009583Z-24407`,
  `20261105T020315843323Z-35017`, `20261105T021347446531Z-47134`.
  Failed fixture retry experiments were archived and removed rather than being
  represented as working acceptance coverage. Correction loops stopped here.
- x86 installed execution and the user's VirtualBox reproduction were not verified.

Installer handoff:
- `builds/InfinityOS-aarch64.iso` (10435837952 bytes), SHA-256
  `b217ed530b9e9e498b43a28a2307d2d0e529295a8afb9124aa49010d48dd1685`.
  ARM packaging, payload parity and checksum verification completed in
  `builds/manifests/20261105T005808555389Z-38216.json`; that combined run subsequently
  failed at the x86 checkpoint bound, so its overall result is correctly failed.
- `builds/InfinityOS-x86_64.iso` (10368731136 bytes), SHA-256
  `54e4213644ecd9478fa85f062475dd48ad315aecceb27a19676cb9ccf1508449`.
  After the x86-only reservation correction, packaging, parity and checksums passed:
  `builds/manifests/20261105T012244210067Z-81601.json`.
- Both images retain compiled products from the clean build attempt. Broad regression
  tests and redundant full-image comparisons were omitted; this is an incremental
  handoff, not a claim that the full clean release profile passed.
