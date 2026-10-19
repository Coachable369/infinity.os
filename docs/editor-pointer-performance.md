# Editor pointer and CPU follow-up (2026-09-12)

## Changes

- Active-editor caret ticks no longer enter the global content-hash invalidation
  and retained-window repaint path. The clock and visible caret strips are
  updated directly. Menus, dialogs, search fields and chat keep their existing
  rendering path. A soft-wrap boundary may have two visible caret strips, matching
  the existing editor renderer.
- The code painter skips clipped-out rows before text measurement and skips
  clipped-out glyphs before their pixel loops. Syntax classification still scans
  the bounded document; it is not claimed to be cached by this change.
- Hover handling in editor menus and per-window assistant panels now presents
  the pointer even when those controls consume the event without a click.
- ARM native inference can request up to 64 secondary workers instead of four.
  MP Services reserves only the BSP; PSCI discovery already excludes the BSP and
  now uses every discovered eligible secondary CPU, subject to that capacity and
  successful firmware startup. Idle workers retain their existing wait path.
  On a six-vCPU guest the intended split is one UI/service CPU plus five workers.
  This is not a general-purpose multicore UI scheduler or new x86 SMP support.
  Static mailboxes grow from about 256 KiB to 4 MiB; private AP stacks are reserved
  only for discovered CPUs. The 12 GiB target remains unchanged.

## Verification and deployment

- Production layout tests verify caret-strip coordinates, damage area, wrapping,
  offscreen cursors, empty documents, scrolling and scales 1–3.
- Six concurrent native math workers pass exact-result, cancellation, multi-batch
  and tail tests. Editor/assistant and AI behavioral suites pass.
- ARM loader/installed kernel and streamed ISO build pass; extracted installed
  payload parity and x86 kernel compile check pass.
- The preserved installed QA clone received the kernel and both EFI loader paths.
  Its separate VM was started without an ISO. The GUI control provider selected
  the original VM instead, so login, live pointer latency and actual five-worker
  startup could not be verified. The QA VM was powered off afterward; its disk is
  retained. The user's running infinityos-4 was not updated or restarted.

Built media: `build/qwen/InfinityOS-Qwen3-8B-aarch64.iso`. Full fresh-install UI
interaction and measured installed mouse responsiveness remain pending; passing
geometry tests are not a claim of measured input latency.

## Generated-source repaint follow-up (2026-10-19 UTC)

The editor clips each glyph once, composites its unchanged coverage mask through
the existing pixel blender, and submits one conservative damage rectangle for
the glyph instead of one rectangle per ink pixel. Geometry, colors, alpha,
font advance, source buffering, undo, and cancellation are unchanged.

The production editor-window painter was measured after three warmup frames,
using nine timed frames per case. For a 16 KiB document at 2× scale, the host
median decreased from 18.801 ms to 17.298 ms (8.0%); damage submissions decreased
from 295,403 to 53,882 (81.8%). At 1×, the median decreased from 4.832 ms to
4.619 ms. These are **host repaint measurements, not token-generation speed or
installed input latency**. The host fixture also excludes the kernel damage
region merger. Appending the entire 16 KiB source in 32-byte chunks, including
two wrap scans per chunk, took about 16 ms total; the document buffer was not
rewritten without evidence of a bottleneck.

All nine full-window zoom captures remained pixel-identical. Behavioral glyph
checks compare the old per-pixel composition with batching across four font
families, nine sizes, two scales, pixel formats, clipping and retained-alpha
surfaces. Each painted glyph now produces exactly one damage submission.

Reproduce through the build authority: build `active-painter-test` with
`./build-kit run make active-painter-test`, then run the resulting executable
through `./build-kit run` with `--editor-streaming-benchmark=before` or `=after`.
Use separately preserved binaries for meaningful before/after comparisons.
`--editor-typography-only` exercises pixel parity. Evidence is retained in
`builds/validation/editor/streaming-{before,after}.json`; manifests are
`20261019T025852266450Z-57890`, `20261019T030010960403Z-57995` and
`20261019T030057532668Z-58169`.
