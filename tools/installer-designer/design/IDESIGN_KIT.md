# InfinityOS Installer Studio — IDesign Kit

## Product character

Installer Studio is a native macOS production tool: dark, precise, spatial,
and unmistakably InfinityOS. The chrome stays quiet so the installation screen
remains the brightest object. Cyan is reserved for selection, snapping, and
save state; warm amber marks validation warnings. The generated infinity-grid
icon is the application identity.

## Window topology

- Toolbar: screen picker, undo/redo, grid controls, preview, validate, and save.
- Left rail (248 pt): independently expandable Installation Screens and OS
  Configuration Screens collections. Every row is selectable and shows its
  flow-local number, title, and layer count. The selected collection owns all
  add, duplicate, reorder, rename, remove, reset, and canvas operations.
- Center stage: scalable 1000 × 1000 normalized artboard on a graphite pasteboard.
- Right inspector (300 pt): element identity, geometry, content, appearance,
  image source, lock state, and layer ordering.
- Bottom status bar: zoom, selected element geometry, snap state, validation,
  and output path.

## Canvas language

- Grid: 10-unit minor lines, 50-unit major lines, emphasized center axes.
- Selection: 2 pt cyan outline, eight white/cyan handles, translucent bounds.
- Snapping: live cyan guides and a compact coordinate badge.
- Panels: deep blue-black glass with a cool hairline.
- Console: darker monospaced surface with inset rim.
- Images: aspect-fit preview over a checker-free obsidian backing.
- Text: direct double-click editing plus full inspector editing.
- Buttons: visible gold-standard navigation controls rendered with a lock badge;
  they can be selected for inspection but never moved, resized, deleted, hidden,
  restyled, or edited.

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
