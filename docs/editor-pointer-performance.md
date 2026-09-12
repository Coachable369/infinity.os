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
