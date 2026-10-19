# Infinity Browser welcome page

Reference: the approved navy browser mockup, generated October 18, 2026.
This replaces the charcoal welcome-page treatment; native text is never baked
into the illustration. Existing glyphs and button interaction recipes are reused.

Palette: background #121f33 to #0d1a29, cards #1c2a3e with #394d6a edges,
primary text #f1f5ff, secondary #a6bade, brand #b7c7ed. Wide layout places
copy left and transparent sculpture right, with 16px card gutters. Narrow
layout stacks artwork and cards without moving their hit targets independently.
Headline uses native Inter at 42px (28px compact), labels and descriptions 14px.
The two existing keyboard-accessible actions remain functional. The three cards
describe available features, not cross-device sync or other unimplemented services.

## Artwork

`browser-welcome-hero-v1.png` is the replacement generated master;
`browser-welcome-hero-v1.bmp` is the bounded native RGBA derivative produced by
`tools/build-browser-welcome-artwork.py` through build-kit. Both are included
in the default kit and its installed-System-Generation parity assertion.

Built-in image generation, background-extraction prompt:
“Extract/recreate ONLY the large blue glass infinity sculpture from the right
side of this approved Infinity Browser mockup, including its soft subtle blue
reflection beneath it. Transparent background. Preserve exactly the horizontal
infinity silhouette, polished rounded glass material, blue periwinkle silver
palette, highlights, perspective and proportions from reference. Asset for native
browser welcome page. Center sculpture with tight but safe padding, landscape
3:2 composition. No browser chrome, text, cards, wallpaper, floor rectangle,
stars or new decorations. Actual alpha transparency around the sculpture and
soft reflection, smooth antialiased clean edges.”

Source generation: exec-e4acbd53-1645-4352-a5c4-1926bf339b7f.png.
