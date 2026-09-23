# Launcher / desktop app dragging

## Interaction contract

- Press an app tile and move in either axis to reorder. Crossing a row preserves
  catalog identity and inserts the tile instead of swapping/duplicating entries.
- Drag outside the launcher to dismiss it and carry a copy-badged app preview.
  Release on the desktop to create a shortcut; the launcher entry remains.
- Drag that desktop shortcut over the dock launcher button while holding the
  button to spring open the launcher. Drop on a tile to insert the existing app
  at that position. The desktop copy remains; no duplicate launcher app is made.
- A desktop shortcut click launches its typed application action. Escape cancels
  a captured cross-surface drag without changing its saved position or ordering.
- Desktop shortcut positions and launcher order are saved on release in a
  versioned, checksummed, user-ID-specific Personal-space object. No storage I/O
  is performed during drag motion. Missing records retain fresh-install defaults.
  Writes require the active owner session and replace the checkpoint without
  allocating an unbounded version history.

## Gravity Wall idea editor

The editor exposes Cancel, Save, and Save & add another using the same geometry
for painting and pointer dispatch. Save & add another is available for idea
creation/editing, not category renaming or deletion. Failed saves retain the
draft and display the failure; a new draft opens only after a successful save.
Enter and Escape remain available. The 17 spatial-state tests include button
hit regions and exclusion of inappropriate actions. Guest visual verification
is still pending.

## Rendering and installation

The existing installed theme atlases and font rasterizer supply the artwork;
there is no separate icon family or generated mock desktop. Old and new shortcut
damage bounds cover the label as well as the art. A single copy badge follows the
pointer, including while the launcher is open. The native installed kernel and
live installer kernel compile the same implementation; no live-only assets exist.

## Verification

- `make app-launcher-interaction-test`: click slop, all-axis reorder, catalog
  uniqueness, copy semantics, durable-state roundtrip and corruption rejection.
- `active-painter-unit-test --test-threads=1`: production icon/font pixels and
  partial-vs-complete composition for movement, release and cancellation.
- `build/shortcut-drag-proof.png`: host framebuffer rendering inspection, not
  installed-VM proof.
- Installed VM pointer replay remains required before claiming end-to-end guest
  acceptance. This change concerns application shortcuts, not importing arbitrary
  documents into the app catalog.
