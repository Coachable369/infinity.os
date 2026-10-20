# Browser chrome — approved full-window reference

Target: `browser-chrome-v5-reference.png`, approved generated preview. Reuse
soft-shoulder tabs, native Inter, stroke icons and the existing assistant star.
URL text, tab text, and page content are explicitly outside this change.

Correction checklist:
- Plain minimize/maximize/close strokes, not boxed desktop control artwork.
- Continuous dark themed tab/menu/navigation surfaces with subtle separators.
- Smooth antialiased pill URL field with subtle inset gradient and focus border.
- Back, forward, reload, address, Downloads, Settings in that order. Navigation
  submits through Enter; no redundant standalone Go arrow.
- Quiet border and focus colors derived from OS Primary and Accent.
- Favorites hidden by default; existing saved visibility preference preserved.
- External circular assistant star near the browser's top-right, with matching
  input/damage bounds; other applications keep their established assistant fin.
- Preserve all actual menu actions, URL editing/selection, history, tab controls,
  downloads, saved settings and page pixels. No new framework or raster chrome.

The shared installed design-kit packaging and byte-parity assertion include this
reference and specification on both architectures. Generated reference is not VM
acceptance evidence. Review the native component proof separately from release
and installed-runtime validation.

## Optical alignment correction — October 20, 2026

Reuse the generated browser glyph atlas and the approved soft-shoulder reference;
no replacement artwork is needed. Keep existing hit-target sizes and tab geometry;
center the new-tab and window controls vertically against the tab glyphs.
Navigation glyphs use a centered 20px optical box, tab document and close glyphs
16px, new-tab 20px, and window controls 16px. This separates comfortable pointer
targets from icon ink size. The document icon starts 20px inside the tab shoulder;
its 16px box and the close glyph share the same vertical center. Labels remain
native Inter 14px and their baseline is centered against those glyphs.

Welcome copy uses native Inter 28px at ordinary window widths and 42px only when
there is room. Headline line spacing is 1.25 times its actual authored size,
not a fixed 54px at every width. Brand, headline, supporting text and actions form
one compact vertical column; the existing unmodified hero remains aspect-fit.
Use the browser's smooth native pill recipe for welcome action buttons, avoiding
the two-piece half-height stripe currently visible in the screenshot.

Proof must render the production glyph pixels at their production geometry,
not hand-drawn approximations. Check native dimensions, optical-center alignment,
anti-aliased edge coverage, compact/wide welcome geometry, and clipping.
