# Rendering performance: measured foundation

Status: **PARTIAL**. This change does not complete the supplied Performance Monitor milestone.

## Implemented in this change

- Bounded, versioned completed-frame telemetry in `kernel/ui/performance.rs`: 3,600 records maximum, 60-second readable detail when timestamps exist, and 600 one-second aggregate buckets. Recording never allocates or waits for a reader. Contended records are counted as dropped.
- Frame/compose/present durations for the active desktop presenter, damage submissions/merges/pixels, and fallback accounting. Durations are optional: AArch64 uses CNTVCT/CNTFRQ; x86_64 requires architectural CPUID 0x15 calibration. No guessed frequency is reported. Host tests inject durations separately from real `Instant` benchmarks.
- Statistics distinguish absent measurements from zero. Average/p95/worst summarize retained measured durations. Snapshots sort bounded history on demand; monitor sampling overhead is not yet characterized.
- Actual framebuffer damage merging consumes transitive overlap chains. Whole-screen presentations now increment public fallback diagnostics, including non-forced full paints. Appearance and explicit recovery paths have reasons; remaining causes are honestly `Unclassified` pending audit.
- Retained software composition validates all visible surfaces before staging pixels, invalidates rejected transactions, samples negative-origin surfaces correctly, and uses a direct opaque-row path. Translucent pixel math remains unchanged.
- Short deferred-output buffers return `DeferredBufferTooSmall` before presenting anything; they neither discard damage nor override the presentation budget. Damage floods no longer manufacture priority 255.

## Evidence

`RUSTFLAGS=-Awarnings make performance-test` exercises the implementation and asserts pixels/state: 300 retained window moves, every framebuffer pixel against a scalar reference, unchanged surface generation, local damage, negative-origin sampling, rejected commits, short deferred buffers, alpha, transitive merging, 1,000-region floods, bounded retention, expiration, availability, and exact percentile values.

Host release fixture: average 138–145 microseconds before, approximately 30–34 microseconds after. Raw observations are in `performance-retained-drag-baseline.json`. This is **not** a measurement of native desktop or VM responsiveness. It times retained composition and copy only; pixel assertions run outside the timed interval.

Also passed: existing `tools/infinity-ui-test.rs` executable behavior suite; build-std cargo checks for x86_64 installer/installed, AArch64 softfloat installer/installed, and legacy i686; x86_64 ISO release build. QEMU/UEFI visually reached the startup menu (`build/performance-boot.png`). No desktop, heavy-load, or detached-media acceptance is claimed from this boot observation.

## Required status ledger

| Area | Status | Remaining evidence or implementation |
| --- | --- | --- |
| Damage-aware rendering | PARTIAL | Host pixel/flood tests pass; actual desktop motion needs VM validation. |
| Cursor-limited damage | PARTIAL | Existing cursor path; continuous guest movement not tested here. |
| Window surface reuse / drag | PARTIAL | Retained compositor fixture passes; native desktop still has clipped scene-rerender paths. |
| Resize | PARTIAL | Existing geometry; rapid-resize/slow-paint acceptance outstanding. |
| Glyph cache | PARTIAL | Existing bundled atlases; runtime reuse/cost counters outstanding. |
| Text layout cache | PARTIAL | No new cache or measurement-count instrumentation. |
| Image cache | PARTIAL | Existing asset paths; hot decode/scale audit outstanding. |
| Translucency optimization | PARTIAL | Opaque retained path optimized; native glass cache audit outstanding. |
| Async UI conversion | PARTIAL | Disk/network/AI/settings handler audit outstanding. |
| IOP latency | PARTIAL | Request/response duration instrumentation outstanding. |
| Compositor priority | PARTIAL | Existing policy; no new preemption or under-load guarantee. |
| Frame telemetry | PARTIAL | Core presenter timing/damage implemented; surface/cache/input/system metrics outstanding; uncalibrated x86 timers unavailable. |
| Performance Monitor | SCAFFOLDED | Visual recipe only; no application, chart UI, or launcher registration implemented. |
| Performance overlay | SCAFFOLDED | Requirement only; not implemented. |
| Settings controls / effects | PARTIAL | Existing quality policy; requested controls, persistence and measured adaptation not wired. |
| CLI parity | SCAFFOLDED | No performance commands or capability-authorized telemetry IOP service yet. |
| System Generation | PARTIAL | Core code builds in both installed/installer kernels; monitor/service/schema packaging and parity assertions outstanding. |
| Clean-install acceptance | PARTIAL | Blank-disk install, detached boot, authentication, workload, parity, reboot not run. |

The complete milestone must not be marked accepted until the outstanding native app, authority/integration, scheduling/cache/async work, and mandatory guest acceptance are implemented and tested. Existing descriptive shell checks are not behavioral proof.
