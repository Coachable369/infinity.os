# InfinityOS Native Desktop Apps — IDesign Kit

## Window recipe

- Text Editor and Command Window are native desktop windows, composed over the installed desktop wallpaper, top bar, widgets, Home window, and dock.
- The active app uses deep transparent navy glass, a restrained cyan edge, a soft top highlight, and the same upper-right minimize, maximize, and close controls as Home and Settings.
- App title and icon share the title-bar vertical center. The content area begins below a compact toolbar and never exposes bootstrap artwork or replaces the desktop shell.
- Windows are movable from their title bars, maximize and restore in place, minimize back to the desktop, and close without changing the authenticated session.

## Application identities

- Text Editor uses the colorful dimensional new-document icon from the installed icon-theme family. It provides a writable multiline page, visible insertion caret, New and Save actions, and an explicit saved/modified state.
- Command Window uses the colorful dimensional terminal icon from the installed icon-theme family. It preserves the native typed Infinity Console language and history inside a desktop window.
- Both applications are launcher entries. Command Window is intentionally absent from the dock; the dock remains a compact launcher plus Files, Settings, system utilities, and Trash.

## Interaction contract

- Launcher activation opens the selected application above the existing desktop without destroying or replacing other desktop state.
- Text input, Backspace, and Enter edit the Text Editor document. The Save button commits the current in-session document state; New clears it.
- Command input, history, execution, clear, and help continue to use the native console pipeline. Escape, `exit`, minimize, or close returns to the unchanged desktop.
- Pointer hit geometry comes from the same shared layout used for rendering, with bounded margins around window controls.
