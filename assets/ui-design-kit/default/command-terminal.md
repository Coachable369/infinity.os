# Command terminal

Extends Eclipse Precision, not the outer window frame. Reuses generated role 25
(terminal), role 27 (search), the shared menu/button recipes, Inter labels and
the packaged InfinityEditor monospace library. No new icon or artwork is needed.

## Visual contract

- Compositor-owned active/inactive frame and existing window controls unchanged.
- Unframed terminal interior, default Charcoal, fourteen-pixel IBM Plex Mono.
  IBM Plex Mono, JetBrains Mono, Fira Code and Inconsolata are installed by
  default with their OFL licenses. Nine exact raster sizes and ten existing schemes.
- Eighteen logical pixels of horizontal breathing room, twelve above the first
  row. Font advance and actual glyph height plus four pixels determine the grid.
- The namespace prompt and editable command are the final logical transcript
  row, wrapping on the same grid as output. No bottom input field or nested card.
- Title bar: File, Preferences and Performance in order. Preferences opens a
  Precision dropdown directly below its label. The transcript uses the space
  beneath the title bar. A
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

## Terminal editor (`edit`)

Reuse the existing Command Window frame and selected packaged editor monospace
font. The document is an unframed text grid in the content rectangle, with a
filename/modified indicator above and two compact status/shortcut rows below.
Selection and caret use the same editor scheme colors. Search and go-to-line
prompts replace the status row; unsaved exit presents Save/Discard/Cancel there.
No new imagery, icon family, outer border or separate command input is introduced.
The viewport wraps and follows the document caret when the window resizes.

## Preferences and remote terminals

Preferences uses the existing Precision menu surface. Up/Down selects the row;
Left/Right adjusts, and Escape closes. Rows/Columns accept numeric typing and
Enter to commit (0 restores automatic sizing). Background cycles between the scheme color and named dark surfaces independently
of the font, scheme and opacity. Background opacity never fades
text or compositor controls. Saved settings are isolated per user.

SSH requests a VT100 pseudo-terminal, forwards keys immediately, and updates its
window size. Ctrl+] disconnects; Escape, Ctrl+C, arrows and Tab go to the remote
process. Remote ANSI cursor, erase, scroll-region, rendition and alternate-screen
operations stay inside the terminal grid. No remote escape may alter OS settings.
SF Mono Terminal Regular is not bundled: Apple distributes it under restrictions
that do not grant redistribution in InfinityOS. No substitute is labeled SF Mono.

## Disk explorer overview

Reuse the terminal's packaged monospace font and sixteen ANSI colors. The disk
outline is a text-cell diagram, with physical LBA positions (including gaps), a
bright selected partition, and a separate measured pool allocation bar. Partition
capacity is not filesystem utilization. Unknown filesystem usage stays unknown.
Arrow selection and Enter drill-down retain the existing explorer controls;
small windows collapse the diagram to the keyboard-navigable partition list.
