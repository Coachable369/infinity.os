# Editor schemes — implementation specification

Reuse `assets/design/infinity-editor-ai-idesign-kit-v1.png`, the default kit's
native menus, Inter labels, 8px rhythm, cyan focus and attached assistant tab.
No new icons or decorative raster artwork: existing assets cover these controls.
Window border, title bar and assistant stay unchanged. Only the document canvas,
gutter, current line, selection, caret and syntax palette follow the scheme.

Ten schemes: Infinity, Charcoal, Monokai Night, Dracula Night, Nord, Solar Dark,
Solar Light, Paper, Ocean, Forest. Light schemes must use dark readable text.
View offers separate Color scheme, Editor font and Text size menus; current
choices have check indicators. Long menus use bounded scrolling rows, with keyboard
navigation revealing the focused item. Preferences save per authenticated user.

Use real bundled OFL monospace fonts: JetBrains Mono, Fira Code, IBM Plex Mono,
Inconsolata. Rasterize each at 8/10/12/14/16/18/20/22/24px, not stretched UI labels.
Renderer, caret, scrolling and pointer selection must use identical cell metrics.
Use the font's measured monospace advance with no added tracking; preserve ink
overhang independently instead of widening every character cell. Control/Command
plus or equals zooms in, minus zooms out, and zero restores 14px. Both directions
have multiple steps from the default. Version 1 saved size indices migrate to
their original pixel size; version 2 retains all nine sizes per authenticated user.
Line numbers use the same font and size as the document. The bundled editor atlas
stores each font/size record as advance, glyph height, pixel size, ink width, then
95 ASCII glyph masks in a 24×32-cell grid; glyph overhang never changes the caret
advance. Selection backgrounds are painted before glyphs to preserve that overhang.

Edit > Text actions exposes select word/line, delete line, move lines up/down,
join lines, sort lines, trim trailing spaces, upper/lowercase, toggle comments,
indent/outdent. Mutations are atomic, capacity checked and one-step undoable.
Existing save/compile, unsaved guards, clipboard and AI behavior remain intact.

Additional lexical modes: Ruby, Go, Java, Swift, Kotlin, C#, SQL, Lua, YAML, TOML.
These are lexical highlighting modes, not parsers or language servers.
