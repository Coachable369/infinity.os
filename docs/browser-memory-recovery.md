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
after relocation and before component initialization. Restart storage is bounded.

A fatal restart creates a fresh active page. Other open tabs, unsaved web forms,
and ephemeral engine session state cannot be recovered from a failed heap.
Favorites, browser preferences, the desktop and native network policy persist.
Page/TLS failures remain distinct from engine memory or panic failures. Hardware
exceptions or noncooperative infinite loops are not recovered by this boundary.

Verification entrypoints:
- `./build-kit browser-core-tests`: state/error projections and private-image restore.
- `./build-kit run python3 tools/servo-platform-probe/run-component-guest.py --recovery`:
  actual component allocation aborts, two fresh starts, navigation and pixels.
- `./build-kit run python3 tools/servo-platform-probe/run-memory-guest.py --std-probe`:
  native allocation accounting, alignment, realloc failure/preservation and reclamation.
- `./build-kit run make https-service-test`: cancellation/loan draining and stale-handle isolation.
- Installed browser harness `--memory-recovery`: real oversized canvas allocations,
  followed by new-address navigation and rendering on a detached installed system.

These entrypoints describe coverage; individual execution receipts determine what
has actually passed. ISO packaging alone is not installed-system evidence.
