# Spatial rendering cost pass — September 22

## Retained changes

- `launcher_backdrop::capture_stage`: reuse vertical sample sums in a 17-column
  ring. Packed 16-bit channel lanes replace repeated nine-tap channel extraction.
  Scratch storage is 136 bytes per row, with no heap allocation or new full-screen
  cache. Clamped edges, vignette, tint and alpha are unchanged.
- `launcher_backdrop::fade`: blend red/blue in separate packed lanes and green
  independently. Integer rounding is unchanged for all 256 opacities.
- `spatial_view::present`: restore the cached stage directly; no preceding desktop
  copy that is immediately overwritten. Retain the original backdrop for Shelf
  and as a fallback when the stage cache is unavailable.
- `DisplayDevice::soft_stroke`: preserve subpixel glow pixels while batching damage
  once per segment, rather than inserting a damage rectangle for every pixel.
- Frame timing now includes initial stage generation and Worldshift arrival
  composition, which were excluded from the earlier frame history.

## Measurement

Host AArch64, actual native backdrop module, Rust `-C opt-level=z`, 2560×1440,
same buffer/opacity and five iterations per binary. Baseline executable was built
before edits from `090dda2`; optimized executable uses this change. Timed scopes
exclude allocation, application painting, device scanout and hypervisor overhead.
Final paired run occurred after the first architecture build sequence completed.

| Stage | Baseline median | Optimized median | Reduction |
| --- | ---: | ---: | ---: |
| Capture blur/tint | 235.552 ms | 57.391 ms | 75.6% |
| Crossfade | 11.219 ms | 6.776 ms | 39.6% |

Raw capture microseconds: baseline 235552, 228615, 230722, 246622, 240302;
optimized 74756, 56746, 57287, 57391, 58961.
Raw fade microseconds: baseline 11219, 11191, 11257, 11227, 11207;
optimized 6873, 6776, 6769, 6878, 6755.

The stroke regression observes more than 1,000 damage submissions reduced to one
per tested segment. This is an operation-count improvement, not a measured GPU or
installed frame-rate speedup. The host fixture counts submissions; it does not
emulate production damage merging cost.

## Behavioral verification

- Optimized stage output matches the original scalar equations pixel-for-pixel on
  widths 1, 8, 17, 37, 257 and 513, padded strides, varied channels/alpha, clamped
  edges and vignette boundaries.
- Fade output matches the scalar equations at every opacity, preserving pixels
  outside the damage clip and row padding.
- Batched strokes match per-pixel rendering with and without clipping and retained
  surface alpha. Nine active-painter test cases passed.
- The final existing input regression suite passed, including the new backdrop
  and stroke cases. Focused backdrop formatting and repository diff checks passed.
- `make aarch64`, `make x86_64` and installed-kernel/loader parity checks passed.
  `sh tools/build-hermes.sh` passed with streamed-kernel binary parity and
  Hermes/Ministral installed-payload checks. These are artifact checks, not a new
  installed VM animation benchmark.

Benchmark command (explicit opt-in; timing is not a flaky test threshold):

```sh
rustc --edition=2021 -C opt-level=z --test tools/spatial-backdrop-test.rs -o /tmp/spatial-backdrop-bench
/tmp/spatial-backdrop-bench --ignored --nocapture --test-threads=1
```

## GPU request — not implemented by these CPU optimizations

The active AArch64 VirtualBox log reports VMSVGA3, `3DEnabled=0`,
`VMSVGA3dEnabled=0`, and zero vertex/fragment shader capabilities. The repository's
native SVGA II backend binds on x86_64 and submits UPDATE notifications, not
shader work. AArch64 currently uses UEFI GOP scanout.

Real offload requires a native ARM SVGA3 PCI/MMIO transport, capability negotiation,
GPU-owned surfaces, upload and lifetime management, command submission and fences,
and shader/compositor integration with a tested CPU fallback. Enabling the host
3D option alone would not supply that missing guest implementation. No GPU claim,
fake acceleration flag, or speculative hardware command was added.

Installed smooth-frame acceptance and GPU offload remain open. The existing VM
input-delivery defect was not changed in this performance pass. These microbenchmarks
must not be substituted for an installed animation P95 or FPS measurement.
