# Spatial completion QA — September 22

This pass is **not full installed acceptance**. Do not infer completion from the
successful build or the host state-machine tests.

## Changes retained

- Continue polling an authoritative raw tablet button source even when firmware
  supplies a fresh coordinate. Previously the polling gate could starve the only
  accepted button source after ownership transferred.
- Ignore the stale relative firmware button mirror after an actual raw relative
  report; canceling an asynchronous endpoint that never reported does not claim
  ownership. Independent raw USB button sources remain independent.
- Add explicit, read-only `inputdiag` and `uiperf` console commands. No per-event
  logging is introduced. Spatial presents now enter the existing measured frame
  history.
- Delete the model packer's private temporary staging tree on exit. Published
  images and cached model files are retained. Repeated staging trees had exhausted
  host disk space during QA.

## Installed observations

Environment: disk-only AArch64 VirtualBox, 2560×1440 guest framebuffer, 7 vCPUs,
20 GiB RAM. Both the default USB-mouse profile and temporary USB-tablet profile
were exercised. No optical image was attached during the desktop checks.

On the tablet path, login, Search → Spatial, tab selection, keyboard overview
activation and a linked-node drag worked. A subsequent drag was missed. Compact
diagnostics after an input failure showed zero held buttons in the shell and all
eleven transport slots. This does not establish where the missing edge was lost.

Ctrl+Shift+K failed through both direct keyboard automation and VirtualBox's soft
keyboard. The installed EFI loader and rebuilt loader differed only in PE
timestamp bytes, not executable code; replacing the loader is not an established
fix.

A candidate that stopped polling firmware mirrors after raw USB ownership
regressed installed pointer delivery. That candidate was reverted. It must not be
reintroduced merely because a simulated single-report-queue test passes.

One measured, mixed desktop/spatial history contained 61 timed frames:

| Metric | Observed |
| --- | ---: |
| Average | 40.007 ms |
| P95 | 209.925 ms |
| Worst | 238.985 ms |
| Latest damaged pixels | 1,529,600 of 3,686,400 |

These are not an animation-only benchmark or before/after speedup. Initial
backdrop capture precedes the spatial present timer. The sample is insufficient
to claim smooth animation or isolate its dominant cost.

## Remaining gates

- Reliable consecutive clicks and drags on the installed pointer transport.
- Working reserved spatial keyboard chord.
- Complete Worldshift unsaved-document round trip.
- Confirmed and canceled cross-application drops.
- Animation-specific frame timing and responsive installed presentation.

The retained candidate is `infinityos-4-completion-v4.vdi`; revision five was the
reverted input experiment. Original installation disks were preserved. Temporary
raw copies, superseded test clones and completed build staging directories created
in this pass were removed to recover space; they are reproducible, not backups.

## Final build verification

- The earlier full `sh build.sh` run passed. Subsequent diagnostic/input changes
  were checked with final `make x86_64`, `make aarch64`, and
  `make input-regression-test` runs; all exited successfully.
- Final installed-kernel and EFI-loader binary parity passed for both architectures.
- Final `sh tools/build-hermes.sh` passed, including streamed-kernel byte parity
  and the executable Hermes/Ministral installed-payload checks. Its private staging
  directory was removed automatically on successful exit.
- Focused Rust formatting and `git diff --check` passed.
- The final ISO contains the retained revision-four behavior, not the regressed
  revision-five firmware-polling experiment.

The restored revision-four VM could open Text Editor and enter an unsaved test
document, but subsequent Search clicks did not reliably open the menu. The
Worldshift round trip therefore remains unverified. No further speculative input
rewrite was retained. Build and binary parity are not substitutes for the remaining
installed interaction and animation acceptance gates above.
