# Spatial retained-scene and kit alignment pass

## Measured installed-system evidence

Environment: `infinityos-4`, native ARM64 VirtualBox on M2 Max, 7 vCPUs,
20 GiB RAM, 2560×1440 guest, 3D disabled, installed disk boot without ISO.
The same saved Editor/Navigator workspace and Command Window were used.
`spatial` resets bounded timing samples, opens the overlay; after settling,
Escape closes it and `spatialperf` reports compositor timing.

| Candidate | P95 / worst µs | Mean µs | Capture µs | Paint µs | Fade µs | Present µs |
|---|---:|---:|---:|---:|---:|---:|
| Size-optimized baseline | 213754 | not captured | 44303 | 117986 | 5979 | 3618 |
| Release opt-level 3 | 99164 | not captured | 6741 | 55298 | 6562 | 2977 |
| Retained scene, 10 frames | 119343 | 49957 | 5302 | 30171 | 8197 | 6285 |

Phase columns are averages. Cold scene preparation is included in these samples;
these are not steady-state-only FPS claims. The first two console renderings
clipped the count/mean rows; those values are deliberately not reconstructed.
The compact four-row diagnostic fixes that presentation issue.

**This early candidate failed the <33ms P95 target.** Retention reduced mean
work, but was not sufficient. The follow-up results below supersede this early
status for the measured reveal/dismissal sequence. No GPU offload is claimed.

Further inspection found queued application refreshes could invalidate the
retained scene during its finite opening/closing transition. The follow-up
defers those refreshes until motion settles, preserving the queued request.
Behavioral tests cover this scheduling rule; installed timing for that follow-up
must be recorded separately before claiming a gain.

### Follow-up installed baseline

The queued-refresh deferral was subsequently measured on the same installed
ARM64 fixture, without an ISO attached: 19 opening/dismissal samples, mean
23,541 us, P95/worst 119,946 us. Phase averages were capture 1,929 us,
paint 5,611 us, fade 7,185 us, scanout 8,814 us. This is a measured improvement
over the earlier mean, but the cold opening still prevents a P95 pass.
VirtualBox USB Tablet was used temporarily for deterministic GUI coordinates;
the production USB Mouse configuration is restored after verification.

The next candidate combines retained translation and backdrop blending into
one bounded pass. Pixel tests compare every opacity and offset to independent
per-channel reference arithmetic, including untouched padding and rejected
invalid bounds. This test is correctness evidence, not a performance claim.

First fused candidate, installed with the same fixture: 21 opening/dismissal
samples, mean 19,344 us, P95 21,457 us, worst 120,116 us. Phase averages:
capture 1,954 us, paint 3,359 us, fused composition 6,872 us, scanout 7,158 us.
The phase historically labeled "fade" now includes translation as well; its
cost must not be compared to the old fade phase in isolation.

A second sequence traversed all five tabs: 74 samples, mean 17,398 us,
P95 34,057 us, worst 114,270 us; capture 525 us, paint 2,312 us,
composition 6,737 us, scanout 7,822 us. Opening/closing passes the P95 target,
but this broader sequence narrowly misses it. The cold entry path still used
restore/translate/fade separately in that candidate; the next change shares the
fused operation with entry and world-arrival frames too.

Final fused-entry candidate, identical five-tab sequence: **75 samples,
mean 16,726 us, P95 25,082 us, worst 110,590 us**. Phase averages:
capture 488 us, paint 2,243 us, fused composition 6,373 us, scanout 7,621 us.
This meets the <33 ms P95 target for the tested opening, five-tab reveal and
dismissal sequence. The 110.6 ms one-time cold preparation is still present;
this is not a claim that every frame or every OS animation is below 33 ms.
No host microbenchmark was substituted for these installed-VM measurements.
The dominant recurring cost has moved to scanout, followed by composition.
No GPU acceleration, reduced resolution, or lower-quality artwork was used.

`make input-regression-test` passes with the fused helper. The backdrop harness
also exercises the actual DisplayDevice wrapper, clipped lift/blend output,
stride padding and invalidated-cache rejection. The helper's exhaustive
opacity/offset test uses varying alpha as well as RGB channels. Moving the
helper under the backdrop module resolves standalone harness compilation
without adding a fake display implementation to production.

### Release verification

The full `sh build.sh` run passed its compile and regression stages, but its
first ARM ISO write failed for insufficient temporary disk space. Removed only
the disposable first-candidate VDI and intermediate RAW conversion file; the
original and tested VM disks remain. Refreshed `make x86_64` after the final
fallback safety edit, then resumed `sh tools/build-hermes.sh` successfully.
Both final images were copied into `builds/`; the ARM copy was compared byte
for byte with the packaged image. Extracted both EFI boot loaders from the
published ISOs and compared them against the build outputs.

Final `installed-kernel-parity-test.py`, `ui-install-parity-test.sh`, and the
VirtualBox ARM input-profile guard passed. Hermes and Ministral payload parity
also passed as part of the successful packaging command. This is a resumed
successful release, not a claim that the initial `build.sh` invocation exited 0.
The no-ISO installed VM measurements above cover the rendering implementation;
the final helper relocation and invalid-backdrop fallback were compiled and
regression-tested afterward, rather than re-benchmarked as a new performance
candidate. The VM is left powered off with the tested candidate attached.

## Implementation

- Release builds optimize for throughput (`opt-level=3`).
- Retain the settled spatial scene in the existing mutually exclusive arrival
  buffer; translate/fade retained pixels instead of repainting window previews,
  glass, fonts and icons on each reveal frame. No extra 32 MiB buffer was added.
- Start the reveal clock after initial scene preparation. Cursor-only movement
  does not invalidate the retained scene. Real edits/cancellations do.
- Use the kit's layered window stack for up to five windows; preserve a bounded
  accessible filmstrip for larger sessions. Geometry tests verify exposed hit
  areas at all three tested zoom levels for every focus ordering.
- Contain preview titles and notices; derive label spacing from font metrics.
- Replace generic Worldshift placeholders with generated landscape imagery,
  while retaining real saved-window indicators and live controls.

## Generated Worldshift artwork

Built-in image generation produced `assets/desktop/spatial-worlds-v1.png` as a
2×2 atlas: alpine lake, ocean island, futuristic city and planet/moon. Prompt
direction: cinematic midnight-blue environments matching the Spatial IDesign
Kit, polished landscape cards without labels or fabricated application UI.
Four 480×320 bitmap crops (`spatial-world-0.bmp` through `spatial-world-3.bmp`)
are embedded in both installed kernels. Installed-kernel parity verifies the
exact bitmap bytes and installer payload bytes.

The icon material correction, generation specification and paths are documented
separately in `mixed-material-icons.md`.

## Packaging

The new asset sources exceeded the old fixed 256 MiB installed ESP. Its size now
rounds measured staging usage up to a 64 MiB boundary, adds 64 MiB headroom, and
keeps a 256 MiB minimum on both architectures. Existing byte-for-byte container
parity checks remain mandatory. Asset validation alone is not fresh-install proof.
