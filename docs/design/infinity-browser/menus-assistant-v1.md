# Browser menu and assistant integration

Reuse the default kit's generated 28 x 104 attached AI tab and the browser's
native sapphire surfaces, Inter typography, cyan focus and 8px spacing rhythm.
No new artwork is required.

Add a 32px menu row below the tabs: File followed immediately by Settings.
File provides New tab, Close tab and Close window; Settings provides Browser
settings. Both the menu item and URL-row gear open the same persistent page.
Place the gear immediately after the address field, followed by Go and Downloads.
Dropdowns overlay page content, consume their own input, dismiss on Escape or
outside click, and support keyboard navigation. Retain the shared assistant's
edge-aware attachment, independent conversation state and explicit Apply actions.

## Verification

The browser-core behavior suite and shared editor/assistant tests passed through
the build kit. An ARM HVF disposable installation, booted from its installed disk
without installer media after a kernel update, verified the Settings dropdown,
the URL-adjacent gear, File menu dismissal, and AI expansion/collapse. Assertions
use native state and changed/restored pixels, not rendered strings.

Reviewed `installed-menus-v1.png` and `installed-assistant-v1.png`: menu headings
are adjacent, the gear follows the address field, and the generated attached tab
opens the shared panel without clipping its composer. Existing desktop top-bar
label overlap is outside this browser change.

The first AI-toggle attempt exposed stale pointer diagnostics on cursor-only
updates; the cursor had actually reached the tab. A bounded pointer snapshot
update fixes observation without forcing redraws or querying services per move.
The retry passed: build manifest `20260928T082413671997Z-18275.json`.

This is installed updated-kernel evidence, not an unchanged fresh-ISO installation
claim. The ordinary ARM ISO packages the same shared runtime changes. No x86
runtime acceptance was performed for this change.
