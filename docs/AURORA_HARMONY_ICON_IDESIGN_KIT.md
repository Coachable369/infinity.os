# Aurora Harmony Icon IDesign Kit

Aurora Harmony is InfinityOS's colorful, approachable icon family. It follows the same semantic contract as every installed family while offering stronger role recognition at a glance.

## Visual language

- Form: rounded app tiles, soft circles, and dimensional object silhouettes.
- Color: saturated cyan, blue, violet, coral, amber, emerald, and magenta.
- Material: polished glass and enamel with restrained highlights and soft depth.
- Detail: one dominant metaphor per icon, no embedded labels, and no brand imitation.
- Background: true RGBA transparency; the icon remains legible over dark glass and wallpaper.
- Geometry: centered artwork, consistent optical scale, generous clear space, and no clipped glow.

## Runtime states

- Normal: full color with native highlight and shadow.
- Hover: the compositor supplies a subtle accent halo; artwork is not replaced.
- Selected: the compositor supplies the active-theme selection plate.
- Disabled: the compositor lowers saturation and opacity without changing the semantic silhouette.

## Semantic and size contract

The family implements all 60 roles in `assets/icons/manifest.csv`: 45 system/object roles and 15 actions. Every role is generated at 24, 32, 48, 64, 96, 128, and 256 pixels. Runtime atlases preserve manifest ordering for deterministic lookup, while the launcher atlas carries the fourteen high-resolution launcher roles.

## Source layout

- `source-v1/base-00-14.png`, `base-15-29.png`, and `base-30-44.png`: generated 5x3 source sheets.
- `source-v1/actions-45-59.png`: generated 5x3 action sheet.
- `master-base-v1.png`: transparent 5x9 production master.
- `master-actions-v1.png`: transparent 5x3 production master.

These masters are source assets. `tools/build-icon-themes.sh` derives every packaged tier and runtime atlas from them so the live tree and fresh-installed System Generation cannot drift.
