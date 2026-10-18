# Browser tabs — Tron Lightrail v3

Reference: `browser-tabs-tron-v3-reference.png` (built-in image generation).
The existing kit lacked clipped-corner, luminous browser tabs. This reusable
native recipe fills that gap; Downloads retains its existing native design.

## Geometry and states

- 36px height, maximum 224px width, 8px gaps and clipped 8px upper corners.
- Flat lower attachment, smoked navy gradient, thin cyan diagonal edges.
- Active: two-pixel cyan top rail with a tightly bounded internal light falloff.
- Hover: lifted surface and brighter edges; active-hover is the brightest state.
- Inactive: quiet blue outline and darker fill. No idle animation or exterior
  bloom; all pixels remain inside the hit/damage rectangle.
- Existing native Inter 14px labels, 16px document glyph, 24px close hit target.
  Close-hover and new-tab surfaces reuse the same clipped geometry and cyan tones.
- Crowded tabs retain independent close targets. Scale geometry and strokes
  together at 1–4x; rendering and pointer ownership share the same silhouette.

`sdk/infinity-browser-core/tab_style.rs` is the runtime authority, shared by both
architectures and live/installed boots. The reference PNG is not a runtime sprite.
The existing byte-for-byte design-kit parity test covers this document and board.

## Generation prompt

Use case: ui-mockup. Create a polished InfinityOS IDesign Kit reference board for
a modern futuristic Tron-inspired native browser TAB STRIP. Dark smoked navy
#08111E backdrop. Orthographic pixel-clean product UI, no perspective. Main
example across top: four 224x36 proportion tabs, clipped 8px diagonal upper
corners and flat lower attachment, 8px gaps, restrained cyan edge light and
narrow electric cyan upper rail on selected tab, dark navy glossy gradient
interior. Active tab brighter, inactive quiet, hover luminous but not blurry.
Labels in clean Inter sans serif, favicon left, clear separate X close target
right. Below show four enlarged specimens labeled INACTIVE, HOVER, ACTIVE,
ACTIVE + HOVER; show a compact crowded strip and plus-tab/close-hover controls.
Reuse thin geometric browser glyphs. Design-token swatches, 8px spacing grid,
14px typography. Elegant, usable, premium cybernetic precision, not a game HUD.
No excessive neon bloom, no illegible tiny text, no unrelated artwork. Title
'InfinityOS / Browser tabs' subtitle 'TRON LIGHTRAIL'. Focus on shippable native
UI geometry and consistent state family.
