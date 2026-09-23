# Holographic carousel interaction

App-bar entry: replace the former Network slot (seventh item) with Spatial
Desktop, retaining the existing dock spacing and selected-pack desktop/monitor
artwork (role 19). Its action opens the native spatial overlay, not Settings.
Network remains available in Settings, the launcher and the top status bar.

The app-bar route calls `spatial_open` and returns immediately, preventing the
ordinary desktop click repaint from covering the new overlay. UI regression
checks exercise the seventh slot's hit target and SpatialDesktop action at
1280×720, 1920×1080 and 2560×1440, and retain the Network launcher action.

Use the existing Spatial IDesign Kit's layered glass window cards and live
application surfaces. No new raster artwork is needed for this motion change.

- Selecting a side card or using arrows moves that exact window to the front
  over 320 ms. Cards follow a shallow curved path, changing size and depth.
- Re-targeting starts at the current visible geometry, without snapping.
- Clicking the settled front card activates its existing app/window and item.
  A second click during travel does not accidentally activate it.
- Hit testing uses the same animated geometry and depth order as painting.
- Reduced motion selects immediately. Escape cancels the overlay normally.
- Rasterize card chrome and previews once per travel; animation reuses cached
  surfaces and clips composition to the card stage. No application repaint or
  service query belongs in a carousel frame.

Verification: geometry endpoints/interruption, depth-hit agreement, activation
decision, bounded cache sizes, native compilation, installed mouse/keyboard QA.

## Implementation and verification, 2026-09-23

`OverviewFrame` supplies the same curved geometry and depth ordering to painting,
retargeting and hit testing. The controller animates for 320 ms and preserves the
selected File Navigator identity, editor document, command history and Settings
section when activating an already visible window.

The carousel caches card chrome and real retained app previews in a bounded
48 MiB atlas with a 31.64 MiB scratch surface. No per-frame allocation, application
render, namespace query or service call is introduced by the carousel renderer.
Composition is clipped to the card stage; the settled frame restores full-quality
text rendering. The cache is invalidated when desktop content refreshes or the
overlay closes. This is additional resident BSS, not additional ISO artwork.

Behavioral checks: all 12 spatial state tests and both retained-surface pixel
tests pass, including all 1–10 window focus pairs, intermediate retargeting,
frontmost hit testing, second-click activation, 4K cache bounds, alpha preservation
and clipping. The input regression suite passes. These checks do not establish
guest animation frame rate; installed visual and timing verification is pending.

Both `make x86_64` and `make aarch64` pass. The installed-kernel binary parity
check passes for both architectures; the UI artifact parity check passes for
their live images and installed ESPs. The model-inclusive ARM packaging path also
compares its reassembled installed-kernel shards byte-for-byte before packaging.

An updated private installed disk is attached to `infinityos-carousel-qa`, with
no ISO attached. It is paused pending GUI verification: desktop automation resolves
the user's already-running `infinityos-4` process instead of the second process.
Do not treat starting the QA VM as proof of successful installed interaction.
The user's running VM and disk were not modified by this pass.

`sh tools/build-hermes.sh` completed successfully, including Ministral and Hermes
install-parity checks and the streamed-kernel payload comparison. Release artifacts
are `builds/InfinityOS-aarch64.iso` (model-inclusive) and
`builds/InfinityOS-x86_64.iso`; `builds/SHA256SUMS` records their updated hashes.
