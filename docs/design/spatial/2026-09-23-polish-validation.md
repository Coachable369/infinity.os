# Spatial and desktop polish — 2026-09-23

Visual reference: `idesign-kit.png` in this directory. Preserve its cool-blue
translucency, layered edge highlights and restrained typography. Reuse native
glass and installed icon artwork; do not bake controls into images.

## Changes

- Desktop shortcut moves hide their stationary source, rather than rendering two
  icons. Cancellation restores the source. Moving an existing shortcut has no
  copy badge. Labels and icons share the tile centre; long labels use ellipsis.
- Carousel neighbours remain circular across selection wraparound. Cached card
  sprites include the animation origin geometry, preventing stale sprites when
  a rotation is interrupted and returns to the same destination.
- Cards use a continuous reflection falloff, highlighted name pills and a subtle
  selected-card light instead of a hard reflection band.
- New running-app drawers start left-anchored. Saved user placement is preserved.
  Dragging into an anchoring zone highlights the corresponding drawer edge.

## Verification boundary

Host behavioral coverage exercises shortcut composition against complete frames,
including cancellation; circular ordering; interrupted carousel cache parity;
and existing spatial persistence/geometry tests. The native card renderer emits
`build/holographic-glass-proof.ppm` for visual inspection.

Worldshift's reported no-op is **not resolved or verified** by these changes.
Its controller, storage commit and installed guest interaction still need an
end-to-end reproduction. Do not treat state serialization tests as proof that
clicking a world changes the installed desktop.

## Installer publication — September 23

Rebuilt the ARM64 model-enabled installer with `sh tools/build-hermes.sh` and
the x86_64 installer with `make x86_64`. Published both to `builds/` and verified
byte identity against their build outputs. `builds/SHA256SUMS` identifies these
artifacts. The default re-provision path also uses the newly rebuilt ARM64 ISO
under `build/hermes/`.

The ARM64 build passed installed-kernel payload byte parity and both Hermes and
Ministral installed-payload harnesses. The first packaging attempt exhausted disk
space; the incomplete image and disposable packaging intermediates were removed
before a successful retry. No source, model-cache files or VM disks were removed.

Repeated behavioral checks: active painter 15 passed, spatial state 18 passed,
drawer interactions 5 passed, launcher interactions passed. Drawer tests now
explicitly assert left-anchored initial geometry and inward menu placement, while
still exercising deliberate right anchoring. Inspected the freshly rendered
shortcut-drag frame: one moving icon and a centered, elided label.

Installed-guest interaction and fresh-install GUI verification remain pending;
replacing an ISO does not update an already installed guest. Existing saved
drawer placement is deliberately preserved. No Hermes latency improvement is
claimed by this desktop packaging change.
