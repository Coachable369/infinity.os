# Sculpted window controls v1

Gap: the kit had no reusable normal/hover/pressed window-button plates.
The generated `window-controls-v1.png` supplies all three states, left to right.
`tools/build-window-controls.py` packs three 96×96 RGBA cells, preserving alpha.

Use a top-right horizontal bar: minimize, maximize/restore, close. Shared native
geometry is 26 logical pixels square, 8 pixel gutters, 12 pixel right inset,
vertically centered within the title bar. Browser controls retain their native
layout hit boxes and center the same square material within them.

Neutral authored glass is tinted using the current primary and accent colors;
close hover/press uses the semantic danger tint. White native minus, square,
overlapping restore squares and X remain crisp at every display scale. Normal
is quiet, hover lifts the rim, pressed darkens the inset. No idle animation.
Do not add decorative traffic lights alongside these functional controls.

Hover is determined by the topmost visible owner, never by covered windows.
Repaint only the previous/current control bounds. Minimize preserves app state;
close observes unsaved-document guards; maximize restores the previous bounds.
Fresh profiles anchor the running-app drawer on the right; saved user positions
remain user choices.

Asset provenance: built-in imagegen, 2026-10-15. Prompt: three front-on neutral
grayscale smoked-glass rounded-square blank plates, normal/hover/pressed, thin
silver rim, transparent background, no text/glyphs, no icy texture. Runtime
overlays semantic native glyphs rather than resampling glyph artwork. Glyphs
switch to dark ink on light plates to retain contrast across theme colors.

## Verification status — 2026-10-15

- Focused behavioral geometry, interaction-state, shelf, native UI and production
  painter checks passed: `builds/manifests/20261015T102059936160Z-3932.json`.
- ARM64 native-browser kernel type check passed after the final contrast change:
  `builds/manifests/20261015T102323485173Z-4546.json`.
- Generated normal/hover/pressed production-painter evidence was visually reviewed
  in `builds/validation/window-control-states-proof.png`.
- Installed lifecycle harness: `tools/window-controls-installed-test.py`;
  not yet executed. Installed behavior on either architecture is not claimed.
- Release packaging remains pending. A concurrent compiler-header change blocked
  the installed-kernel build with undefined errno constants in
  `sdk/compiler/platform.c`; subsequent verification was blocked by another
  build-kit owner. Existing installer ISOs do not contain this change.
