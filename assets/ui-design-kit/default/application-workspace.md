# Native application workspace standard

This is the required application layout contract for new and updated InfinityOS
apps. The Eclipse Precision compositor owns the title bar, frame, adornment and
window controls. Do not render a second app title or scale a screenshot of the
application when its window changes size.

## Fixed chrome, flexible content

| Region | Logical geometry | Resize behavior |
| --- | --- | --- |
| Title and window controls | Shared compositor geometry | Remain at system scale |
| Menu | 36 px high | Fill width, never stretch height |
| Toolbar | 48 px high | Fill width; overflow commands when necessary |
| Workspace tabs | 40 px base | Fixed shoulders; stretch only flat center |
| Toolbox | 210 px wide | Left anchored; vertical content scrolls |
| Inspector | 260 px wide | Right anchored; vertical content scrolls |
| Work area | Remaining width and height | Reflow; never stretch controls |
| Designer form | Saved logical dimensions | Center when it fits; scroll otherwise |

Apply display density once, then any explicit content zoom. Resizing the window
must not change font size, menu height, toolbar height, icon size, border thickness
or control height. A workspace may support Cmd/Ctrl +, − and 0; zoom affects its
content and internal chrome deliberately, never the compositor title bar.
Do not change saved form coordinates, event bindings or source code when zooming.

## Visual and interaction contract

Use Inter 18 px for readable workspace text, Inter 20 px semibold for section
headings, and IBM Plex Mono 18 px for source. Use 8 px spacing increments and
12 px panel insets. Keep labels aligned with their fields. Use existing UIKit
input, button, toggle, scrollbar and menu controls. No extra outer app border.
Content background #090F26; pane #101B3A; foreground #E6EAF2. Text and controls
remain legible under the selected theme. Focus must be visible without shifting
layout. Help anchors at the right end of the menu row.

The [workspace tab reference](infinitycode-tabs-reference.png) and
[tab specification](infinitycode-tabs.md) define the reusable soft-shoulder
normal, selected, hover and keyboard-focus states. Permanent workspace modes
have no close glyph. Document tabs may use the existing browser-tab close target.
Never use decorative borders around every control or change hit bounds on hover.

## Reuse from Python

`infinity_uikit` ships in the native interpreter. Import `MENU_HEIGHT`,
`TOOLBAR_HEIGHT`, `workspace_tabs(root, style)`, and
`workspace_controls(root, style)`. Keep the returned image list
alive while the notebook exists. Use style `Workspace.TNotebook`; this selects
the reusable image-backed tab element. InfinityCode is the reference consumer.

Use `workspace_menu(button, menu)` for menu-bar dropdowns. A completed click
posts the menu below its button; the opening mouse release cannot activate an
item or dismiss the newly opened menu. Disabled buttons do not open menus, and
releasing outside a pressed button cancels the click. Keyboard traversal remains
owned by the native toolkit. Keep menu commands on the same application action
handlers used by toolbar and keyboard shortcuts.

## Acceptance

Capture the real native application at normal, maximized and restored sizes,
and at supported zoom levels. Compare with the design reference: menu/toolbar
heights, text baselines, tab shoulders and state colors, sidebar alignment,
content padding and absence of black/unpainted regions. Exercise menus, tabs,
keyboard focus, zoom and form interaction. Host screenshots and successful
compilation do not replace installed native pixel checks. Record any remaining
mismatch explicitly; do not claim exact visual parity while defects remain.

## Native Python control adapter

The reusable button and input adapter uses production-rendered Precision pixels,
not hand-drawn substitutes. `tools/native-python/uikit-artwork.py` compiles the
OS surface sampler and exports normal, focused/hovered, and pressed states for
secondary buttons, primary buttons, and fields. Native Python builds regenerate
these states from the current kit. The SDK ships the same artwork module.
Inspectors must scroll when explicit zoom or a smaller viewport reduces space.
