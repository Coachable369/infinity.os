# Modern surface collection

Approved visual direction: `buttons-v2-reference.png`. The earlier neon/bevel
button proposal was rejected and is not part of this collection.
`window-families-reference.png` defines the ten compositor frame treatments.

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
Tinted Glass and Text Action, respectively. IDs 15–24 identify Sapphire Seam,
Floating Shadow, Frosted Silver, Obsidian Bevel, Pearl Ceramic, Brushed Alloy,
Aurora Edge, Double Rail, Inset Slate and Cobalt Ribbon.

Button logical radii: 10, capsule (32 bounded by height), 12, 8, 10, 5, 14, 8,
12, 8, 8. Normal/hover/pressed geometry is identical. Hover interpolates shared
pixels with the existing timing; pressed changes illumination without moving
hit bounds. Outline and Text Action have transparent idle interiors. Porcelain
uses a dark caption. Disabled states retain the family at reduced opacity.
Frames keep the compositor's rectangular client geometry and active/inactive
band widths. No rounded-frame clipping is implied.

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
