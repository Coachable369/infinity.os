# Modern surface collection

Approved visual direction: `buttons-v2-reference.png`. The earlier neon/bevel
button proposal was rejected and is not part of this collection.
`precision-window-edges-reference.png` defines the ten compositor frame treatments.

Gap: previous presets changed colors over one texture. These families introduce
independent artwork, silhouettes, opacity, state lighting and border treatments.
Legacy presets remain active compatibility choices for saved user designs.

## Shared implementation

`tools/build-style-collection.py` compiles reusable RGBA material artwork into
`modern-buttons-v1.rgba` (11 families × 3 states × 256 × 64) and
`modern-frames-v1.rgba` (10 families × 3 states × 64 × 64). Both Studio and native
UIKit consume identical bytes. Button corners use the shared antialiased shape
sampler; frames preserve their fixed-width bands and transparent client region.
All artwork is packaged into the installed DesignKit. No application-specific
outer border is introduced.

Material IDs 4–14 identify Optical Glass, Liquid Pill, Frosted Slate, Solid
Sapphire, Porcelain, Carbon, Aurora Silk, Clear Outline, Floating Surface,
Tinted Glass and Text Action, respectively. IDs 15–24 identify the precision edge collection documented below.

Button logical radii: 10, capsule (32 bounded by height), 12, 8, 10, 5, 14, 8,
12, 8, 8. Normal/hover/pressed geometry is identical. Hover interpolates shared
pixels with the existing timing; pressed changes illumination without moving
hit bounds. Outline and Text Action have transparent idle interiors. Porcelain
uses a dark caption. Disabled states retain the family at reduced opacity.
Frames preserve active/inactive geometry with independently selectable square or rounded corners.

Global Buttons applies primary and secondary together, with undo. Per-button
Appearance overrides retain identity, position, caption and action. Global
Window Frame applies to compositor-owned chrome. Saving a document alone does
not change a running OS: use Save & Update ISO and boot/install that image.
Existing installed systems are not silently modified.

Global surface records retain their existing binary structure. IUIT 14 adds a
full material byte after each element flags byte; IUIT 15 combines that extension
with the existing navigation menu payload. Documents without new materials
continue to emit IUIT 12/13. Readers retain support for versions 6–13.

## Verification

Compare native and Studio pixels across states, sizes, density and alpha;
exercise binary/JSON round trips, inheritance and undo. Inspect the rendered
selection board against the approved references before packaging. Compilation
alone does not establish visual or installed-runtime acceptance.


## Precision window edges — revised November 20

`precision-window-edges-reference.png` replaces the former bulky frame board as the visual target.
The ten materials retain IDs 15–24: Sapphire Hairline, Soft Shadow, Frosted Rim,
Graphite Edge, Porcelain Edge, Satin Silver, Aurora Trace, Horizon Light,
Inset Shadow, Cobalt Accent. The reference shows square and 12 px rounded specimens.
Each uses one continuous optical edge with a 1 px core and at most 2 px quiet falloff.
No bevels, brackets, double rails or thick ribbons. Inactive state lowers alpha to
60% and luminance by 12; geometry never changes with activation.

Global Window Frame exposes Square / Rounded; radius remains editable 0–40 px.
Changing material preserves corner geometry. Compositor clips the entire window
against the same antialiased shape, including blur and cached content. Studio
and native OS use identical RGBA assets and integer sampling. Application header
sizes, controls, interior content, and all button themes remain unchanged.

The generated reference is the design target; native rendered comparisons and
Studio/native byte parity are separate acceptance checks. Do not claim exact
visual acceptance from compilation or parity alone.

### Rendered review evidence

The November 20 implementation exports all ten native frame families in square
and 12 px rounded forms, with active and inactive states, through
`tools/style-collection-proof.rs`. The comparison board is retained at
`builds/validation/precision-edges/native-frame-board.png`; a complete native
Settings render is `builds/validation/precision-edges/settings-rounded.png`.
Studio save/reload/undo tests cover rounded frame persistence. Native corner
tests cover opaque client content and partly offscreen windows.

Review checks: continuous single edge, fixed thickness, antialiased curvature,
independent family colors/falloff, no bevel rails, and unchanged client layout.
The reference board is AI artwork, not a pixel-identical raster golden: its
labels, overlapping sample arrangement, interior lighting, and title glyphs
are not a window-frame asset. Exact byte parity applies to shared Studio/native
frame sampling, not to the entire reference illustration.
