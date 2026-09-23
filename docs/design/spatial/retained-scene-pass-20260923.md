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

**The 30 FPS / <33ms P95 acceptance remains open.** Retention reduced mean work,
but the measured candidate still fails the target. No GPU offload is claimed.

Further inspection found queued application refreshes could invalidate the
retained scene during its finite opening/closing transition. The follow-up
defers those refreshes until motion settles, preserving the queued request.
Behavioral tests cover this scheduling rule; installed timing for that follow-up
must be recorded separately before claiming a gain.

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
