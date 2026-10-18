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
