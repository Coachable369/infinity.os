# Command terminal

Extends Eclipse Precision, not the outer window frame. Reuses generated role 25
(terminal), role 27 (search), the shared menu/button recipes, Inter labels and
the packaged InfinityEditor monospace library. No new icon or artwork is needed.

## Visual contract

- Compositor-owned active/inactive frame and existing window controls unchanged.
- Unframed terminal interior, default Charcoal, fourteen-pixel JetBrains Mono.
  Four packaged monospace faces, nine exact raster sizes and ten existing schemes.
- Eighteen logical pixels of horizontal breathing room, twelve above the first
  row. Font advance and actual glyph height plus four pixels determine the grid.
- The namespace prompt and editable command are the final logical transcript
  row, wrapping on the same grid as output. No bottom input field or nested card.
- Quiet toolbar: appearance menu, generated search icon, clipboard menu. A
  restrained footer reports namespace and scroll position, never fake status.
- Selection uses the selected scheme's selection color; search hits use its
  accent. Caret is a narrow high-contrast bar at the actual insertion position.
- Menus use shared Precision surfaces, visible selection, keyboard navigation,
  Escape and click-away dismissal. Narrow windows retain nonoverlapping targets.

## Behavior contract

Bounded ring scrollback; soft wrapping without modifying copied text; tail follow
unless the user scrolls away; pointer selection, word/line selection, scrollbar,
search next/previous, guarded clipboard, history, completion, font zoom and
durable user-specific appearance. Clear removes transcript; it never cancels a
program. Program input/cancellation stays with the existing execution controller.
Lock/logout clears transcript, selection and search, with no cross-user history.
Live and installed systems use the same embedded implementation and font pixels.

## Verification

Exercise typed state transitions, bounded geometry, production-rendered pixels
at compact/normal/HiDPI sizes, native execution, and fresh-install packaging.
Do not present screenshots alone as command-execution or installed-VM proof.
