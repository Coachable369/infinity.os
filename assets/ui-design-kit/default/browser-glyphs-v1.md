# Browser glyph family v1

Generated master: `browser-glyphs-v1.png`. Sixteen transparent silver-blue
glyphs share optical weight and front-on perspective. No boxed backgrounds,
red close glyph, neon halos, or runtime line-drawn substitutes.

Atlas order (four columns): back, forward, reload, stop; download, lock, more,
minimize; maximize, close, document, new tab; bookmark, settings, search, tabs.
Bookmark role 16 aliases role 12. Webpage favicons remain website-owned content.

`tools/build-browser-glyphs.py` fits each generated cell into a 64px RGBA cell
without stretching. Runtime uses alpha-weighted area filtering at native UI
scale. Disabled glyphs retain their material at 38% opacity. Hover and pressed
feedback reuse existing browser surfaces; geometry and hit targets do not change.
The whole default kit is copied into both installed System Generations and
checked byte-for-byte by `tools/design-kit-parity.py`.

Built-in imagegen prompt: exact transparent 4x4 browser glyph sheet in the above
order; thin rounded contemporary UI strokes, satin white with faint cool silver
edge, consistent 2px optical weight at 24px, no labels, backgrounds, glow or
puffy bevels. Refined from the first generated sheet to reduce stroke thickness.

This asset is a design-kit component, not installed-runtime acceptance evidence.

Verification: glyph pixel/state tests passed at 16, 20, 24, 32, 48 and 64px
(`20261019T031737874711Z-61269`). Both ARM64 and x86_64 native-browser kernel
checks passed (`20261019T031952849864Z-61788`). Packed preview visually reviewed.
Installed screenshot review and clean release parity remain pending.
