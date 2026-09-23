# Gravity Wall, Worldshift, and floating app drawer

## Implemented interaction contract

- Gravity Well is renamed Gravity Wall in native navigation.
- Add idea creates bounded editable text in the selected category. Read / edit
  opens the stored text, Enter saves, and Escape cancels.
- New category creates a named ring. Names are unique ignoring ASCII case.
- Sub-idea (or I) attaches a child to the selected idea and expands its parent.
  Expand / fold (or Space) hides or reveals descendants without deleting them.
  Parent relationships and expansion state survive a saved-session reload.
- Remove idea asks for Enter confirmation; Escape cancels. Removal includes all
  descendants and never deletes referenced source files. Reassigning a branch
  to another category moves its descendants too; moving a child to another ring
  detaches that child from its old parent. Ordinary repositioning preserves parents.
- Select a category button or click its ellipse; 1–4 also select rings.
- Wheel and +/- change the selected ring's zoom independently.
- Drag an idea onto an existing category button to preview reassignment.
  Confirmation changes metadata only, not source files.
- Capacity remains explicit: four rings, sixteen shared references/ideas,
  192 printable characters per idea, 24 per category name. Large idea sets can
  become visually dense; this is not an unlimited canvas or rich-text editor.

The app drawer can float at an interior drop position. Drag its header toward
either screen edge and release inside the docking zone to anchor it. A normal
interior release does not snap. Menus open inward; position is saved per user.

Clicking a Worldshift tile starts the existing cancellable world transition.
The destination uses its packaged wallpaper, icon family, skin, accent and
primary palette, saved window layout, navigator location and editor snapshot.
Leaving a world captures its current appearance and layout. Save layout also
saves appearance. Active-world appearance is restored on authenticated login.
Existing unsaved editor/navigation in-session snapshots remain preserved.

## Persistence and installation

Existing 8 KiB spatial records retain their object identity and checksum.
The extension marker supports legacy zero-extension records and the new bounded
category, zoom and appearance fields. Legacy references retain their source
identity and get category labels during migration. Floating drawer coordinates
use previously reserved desktop-store trailing bytes; older records default to
edge anchoring. Spatial layouts also retain floating coordinates.

No new runtime asset dependency is introduced. World wallpapers already ship
in the installed kernel. The generated design reference is documentation only;
all actionable UI is rendered natively, not baked into that image.

## Design reference

`gravity-wall-reference.png` was generated with the built-in image tool.
Prompt: “InfinityOS Gravity Wall interaction design kit, midnight navy glass,
restrained cyan illumination, readable white typography, four labeled category
ellipses with attached idea cards, Add idea and New category controls, selected
ring zoom, floating app drawer and edge docking, Worldshift preview tiles,
generous spacing, selected/hover/drag states.”

The image includes illustrative search and example content not implemented as
features. It is a visual reference, not acceptance evidence or a native screenshot.

## Verification boundary

Behavior tests cover category capacity/uniqueness, assignment, zoom geometry,
hierarchy visibility, subtree removal, malformed ancestry rejection,
appearance round-trip, legacy migration, floating/drop state transitions and
per-user layout reconstruction. Production drawer pixel tests compare partial
repaints with full composition after floating and left/right placement.
Live installed-VM interaction and the Gravity Wall visual review remain pending.

2026-09-23 local checks: 16 spatial state tests, 5 drawer interaction tests,
11 production painter tests, the desktop session reconstruction executable,
and `make input-regression-test` passed. AArch64 cargo check passed with existing
warnings. These do not constitute installed-VM or final visual acceptance.

Final release validation: `make aarch64 x86_64`, `sh tools/build-hermes.sh`,
`python3 tools/installed-kernel-parity-test.py`, and the UI install parity
artifact check all exited successfully. Both linked installers contain the
byte-identical installed kernel and the four Worldshift wallpaper assets.
The model-enabled ARM64 ISO and x86_64 ISO are published under `builds/`.
