# Eclipse Precision — OS surface contract

`eclipse-precision-v1-reference.png` is the approved standard, not an optional skin
example. All OS panels use the shared image-backed renderer. Applications retain
content/layout ownership, never ownership of their outer frame geometry.

- Windows: straight rectangular edges; integrated bold L corners; bright active
  and subdued inactive frames. No rounded outer frame beneath square brackets.
- Panels: midnight navy glass, restrained icy-silver edge, optically softened
  corners. Primary controls use electric blue; fields use navy inset surfaces.
- Tokens: background #090F26, surface #101B3A, elevated #1A2E63, primary #3E6FFF,
  border #86C7FF, text #FFFFFF. Desktop theme overrides tint these same assets.
- States: normal, focused/hovered, pressed, inactive and error. Existing 140 ms
  focus easing and reduced-motion behavior remain authoritative.
- Type: packaged Inter; 32/40 display, 20/28 title, 16/24 body, 14/20 caption,
  12/16 label at logical scale. Authored wizard font sizes remain WYSIWYG, with
  coverage-filtered resizing rather than nearest-neighbor enlargement.
- Background opacity does not fade borders, controls, glyphs or text.
- Reuse the packaged Eclipse monochrome glyph family and native window controls.

## Assets and integration

Gap in the earlier kit: no image-backed shared rectangular frame/panel family.
The reference and blank component source were generated with the built-in image
generator. Prompt: flat Eclipse Precision production controls, transparent canvas,
rectangular frame with L corners, navy panel/field, electric-blue button, current
and inactive progress dots; exact tokens above; no text, chrome bevels or scenery.
The approved reference supplies native-resolution 18px frame corners with 4px L
strokes, panel normal/focus states, primary normal/hover/pressed states, and field
normal/focus states. Border and inward-glow strips are preserved directly; blank
inner-edge samples replace example labels with the authored surface gradient. Only
the clean progress dots still use the component sheet. Sheet gutters are not
runtime artwork.

`./build-kit run python3 tools/build-precision-artwork.py` deterministically crops
and packs the native RGBA atlas. `kernel/ui/panel_recipe.rs` owns sampling,
nine-slice geometry, theme tint, alpha and states. The atlas has eleven state
cells in two layers: independently translucent fill and authored edge/glow.
Corners are packed at native resolution rather than downsampling a whole window.
Focus interpolates those premultiplied state pixels; the bounded inward light
remains visible at zero background opacity. Default tokens preserve the board's
full contrast, and custom themes apply their color delta without flattening it.
`panel_view.rs` owns clipped
framebuffer composition. Shared glass, application cards and access surfaces use
this path. The desktop compositor supplies every window's frame independently
of its application renderer, including generated native applications.

The kit assets are included in installer/fresh-install parity and kernel binary
asset verification. Visual proofs use production sampling, not a second mockup.
