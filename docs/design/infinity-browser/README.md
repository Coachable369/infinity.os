# Infinity Browser v0.1 IDesign Kit

`idesign-kit-v1.png` is a generated design reference, **not a running browser**.
Generated with the built-in image generation tool. Native implementation should
reuse InfinityUI fonts, controls and compositor rather than rasterizing UI text.

## Measurable shell recipe

- Midnight blue native chrome; white web content belongs to the page, not shell.
- One title bar, upper-right native window controls; no tabs for v0.1.
- Toolbar: back, forward, reload/stop, inset address field, Go. 8px gaps,
  16px side gutters, 44px control height. Address field takes remaining width.
- Content excludes title/toolbar/status bounds. Pointer mapping uses that origin.
- One-pixel soft cyan edge, modest bevel, no animated full-window glass blur.
- Normal, focused, disabled, hover and pressed control states are distinct.
- Security state must follow verified TLS results, not merely an HTTPS scheme.
- Loading uses an indeterminate line unless actual progress is known. Do not
  implement the illustrative 60% label as synthetic network progress.
- Error panel uses a concise native message and safe retry/back action. No
  raw certificate/engine diagnostic text and no ignore-certificate button.
- Download completion is shown only after a durable native object commit.
- 16/24 body typography, 14/20 metadata, 24/32 headings; native pixel rendering.
- Minimum useful window width 640px; prefer 1040x760 initially. Fit content on
  resize; keep control targets separate, and clip page rendering to its viewport.

## Generation prompt

Use case: ui-mockup. Create a polished high fidelity Infinity Browser v0.1
IDesign Kit reference board for a native operating system browser. Dark midnight
blue glass surfaces, restrained cyan edge highlights, soft bevels, crisp readable
typography, generous 8px grid. A large one-window browser mockup occupies upper
two thirds, with native title bar Infinity Browser, upper right minimize maximize
close, one toolbar back forward reload, rounded inset address field showing
https://example.com, Go button. No tabs. White readable webpage content area with
simple typographic Example Domain page and blue link, isolated from dark native
chrome. Bottom third shows labeled reusable components: focused address field,
disabled back button, loading progress line, TLS error card without raw engine
errors, completed download row, button hover and pressed states, typography roles
and 8/16/24 spacing. High-end functional desktop product design, precise alignment,
no giant decorative space art, no generic wireframes, no mobile phone. This is a
design reference board, not an OS screenshot.

Review: composition separates native controls and page content clearly. Generated
marketing copy and auxiliary link/button illustrations are not requirements.
No browser icon family is introduced: use the existing selected OS icon set.
