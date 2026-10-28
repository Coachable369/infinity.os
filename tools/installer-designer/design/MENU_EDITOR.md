# Desktop menu editor

Uses the existing Studio IDesign Kit, SF Symbols for authoring controls and
Eclipse Precision for native runtime menus and window chrome. No new artwork
or alternate frame recipe is required.

Select Infinity Menu and open Edit Menu. Keep the collection sidebar and global
save/undo toolbar visible. The center becomes an indented item outline with
compact add, duplicate, reorder, indent/outdent and delete controls. The right
inspector edits the selected item's label, enabled/checked state and typed
destination. Use native pickers, checkboxes, text fields and icon tooltips.
Submenus and separators have distinct icons, not decorative cards. Selection
uses the existing cyan treatment. Long names truncate in the outline but remain
fully editable in the inspector. A collapsible menu preview uses saved item
order and disabled states. No OS actions execute on the host during preview.

New Screen / New Window requires a real name. Creation adds an editable desktop
canvas and links the item atomically. Existing core desktop scenes cannot be
deleted or renumbered. Removing a linked authored screen is rejected until its
references are reassigned. Menu changes participate in global undo/redo, dirty
state, source/runtime export, import and save. Runtime targets are a bounded
allowlist; no shell strings, filesystem paths or pre-authentication routes.

Runtime presentation uses the existing native window host: Window opens its
resizable layout and Screen opens the same layout maximized. One authored
destination is presented at a time. This is layout authoring, not an application
code generator; decorative elements do not acquire executable actions. Existing
native actions retain session authorization and power-action confirmations.
