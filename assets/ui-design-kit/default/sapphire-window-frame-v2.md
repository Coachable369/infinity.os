# Sapphire Window Frame v2

AI-generated reference: `sapphire-window-frame-v2-reference.png` (OpenAI image generation).
Prompt direction: production window-chrome board, active and inactive sapphire bevel,
substantial continuous 3 px rim, soft inward highlight, no teal/purple/noise/corner
brackets. Existing circular window controls, title typography, fixed header heights
and AI adorner remain unchanged.

## Shared implementation contract

The compositor owns all outer application frames. Retain rectangular client geometry;
use continuous mitered rails at all four corners, three opaque logical pixels plus a
two-pixel inward falloff. Do not sample decorative corner brackets. Raster profiles
are median-extracted from the generated reference to eliminate stretched grain.
Active and inactive states retain identical weight. UIKit border overrides tint both
states; absent an override, use the theme outline tint with the generated sapphire/slate artwork. Density
scales the rim once; resizing never stretches its thickness.

`tools/build-window-frame-artwork.py` changes only frame fill/edge cells 0 and 1 in
the shared atlas. All other cells must remain byte-identical. The native compositor
and Studio use the same atlas and renderer rules. Global Window Frame in Studio
exposes the shared border and window background recipes, undo/reset and live states.
Saving UIKit and updating the ISO includes these recipes for live and fresh install.

## Acceptance

Compare active/inactive native frames, small and maximized app renderings, tint and
opacity, continuous corner joins, transparent interiors and scale 1/2/3. Compare
Studio pixels against executable native samples. Do not claim literal reference
pixel parity: the generated board illustrates rounded corners while the compositor
retains its rectangular client clipping contract.
