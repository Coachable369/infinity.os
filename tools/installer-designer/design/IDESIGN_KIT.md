# InfinityOS Installer Studio — IDesign Kit

## Product character

Installer Studio is a native macOS production tool: dark, precise, spatial,
and unmistakably InfinityOS. The chrome stays quiet so the installation screen
remains the brightest object. Cyan is reserved for selection, snapping, and
save state; warm amber marks validation warnings. The generated infinity-grid
icon is the application identity.

## Canonical UIKit tokens

InfinityStudio and every installer or post-install configuration template use
the same semantic Default Dark tokens. Raw one-off colors, arbitrary radii,
and per-screen control dimensions are not permitted.

- Canvas `#02070F`; panel `#0A121D` at 90%; raised panel `#121D2A` at 94%.
- Border `#33475B`; primary text `#F1F5FA`; secondary text `#AEB8C6`.
- Accent `#20BFFF`; bright accent `#9CE8FF`; focus `#FFFFFF`.
- Primary action base `#083654` with `#28B5E7` edge; secondary action base
  `#050F1B` with `#2A455B` edge.
- Field base `#020A14`, idle edge `#28485F`, placeholder `#778595`.
- Spacing scale: 4, 8, 12, 16, 20, 24, 32, 48. Panel content uses a 32 px
  outer inset and a 16 px control gutter at the 1536 x 1024 reference size.
- Corner radii: 4 px detail, 10 px control, 16 px panel, 24 px hero.
- Control heights: 34 px toolbar, 40 px compact, 48 px standard, 56 px hero.
- Typography: Inter for body and controls, Inter Semibold for headings and
  actions, JetBrains Mono only for technical values. Body text never drops
  below 16 px at the reference size.

Buttons are role-driven components rather than freely styled rectangles.
Primary and secondary actions have equal height, centered labels, a 12 px gap,
and at least a 40 px interactive target. Text fields share the standard action
height, control radius, and 16 px internal leading gutter. Focus changes the
edge and highlight, never the geometry.

## Window topology

- Toolbar (64 pt): identity, undo/redo, insertion, grid controls, validation,
  and save. Every labeled action is 32 pt high; icon-only actions are square.
- Left rail (248 pt): independently expandable Installation Screens and OS
  Configuration Screens collections. Every row is selectable and shows its
  flow-local number, title, and layer count. The selected collection owns all
  add, duplicate, reorder, rename, remove, reset, and canvas operations.
- Center stage: scalable 1000 × 1000 normalized artboard on a graphite pasteboard.
- Right inspector (300 pt): element identity, geometry, content, appearance,
  image source, lock state, and layer ordering.
- Bottom status bar (34 pt): zoom, selected element geometry, snap state, validation,
  and output path.

## Canvas language

- Grid: 10-unit minor lines, 50-unit major lines, emphasized center axes.
- Selection: 2 pt cyan outline, eight white/cyan handles, translucent bounds.
- Snapping: live cyan guides and a compact coordinate badge.
- Panels: semantic Default Dark glass with the canonical 16 px radius and
  border; nested surfaces use the raised-panel token.
- Console: darker monospaced surface with inset rim.
- Images: aspect-fit preview over a checker-free obsidian backing.
- Text: direct double-click editing plus full inspector editing.
- Buttons: visible gold-standard primary and secondary controls; lock state is
  shown by the selection overlay rather than altering button contents;
  they can be selected for inspection but never moved, resized, deleted, hidden,
  restyled, or edited.

## Post-install composition

The 1536 x 1024 reference composition uses a 521 x 754 px configuration card,
32 px card insets, 16 px nested content gutters, 48 px fields and actions, and
12 px between adjacent actions. All eight screens retain the same card, header,
progress, title, body, content, and navigation anchors. Dynamic input and
network state overlays exactly replace their design-time preview surfaces.

## Interaction contract

- Click selects; Shift-click toggles selection.
- Drag moves; eight handles resize; arrow keys nudge; Shift-arrow moves by grid.
- Grid snapping is on by default and can be disabled from the toolbar.
- Command-Z/Shift-Command-Z undo and redo all document mutations.
- Command-S validates and atomically saves editable and runtime templates for
  both the installer and post-install OS configuration flow.
- Delete removes editable elements only. Command-D duplicates editable elements.
- New Panel, Text, Image, and Console commands create real elements at the
  current viewport center.

## Template contract

The installer editable document is versioned JSON
(`installer-screens.infinityui`) and its compact runtime export is
`installer-screens.iuit`. The OS configuration collection uses the parallel
`configuration-screens.infinityui` and `configuration-screens.iuit` artifacts.
Both formats retain normalized integer geometry, bounded UTF-8 strings, stable
element IDs, kind/role enums, z-order, appearance tokens, image asset
identifiers, crop state, and lock flags. The installer contains eleven factory
screens; OS configuration contains eight factory screens matching the actual
first-boot state machine. Every screen retains one locked Back action and one
locked Primary action; all non-button layers remain unlockable and editable.

The InfinityOS kernel parses the embedded runtime template through a typed API.
Invalid data, an unknown version, missing screens, malformed rectangles, or a
changed button record rejects the saved template as a unit and selects the
compiled gold-standard fallback.

## Accessibility

All toolbar actions have labels and keyboard equivalents. Inspector controls use
native labels, fields, steppers, toggles, and color wells. Selection is conveyed
by outline and label, not color alone. Minimum hit targets are 28 pt and canvas
handles are at least 10 pt onscreen.
