# Circular glass window controls v2

Extends Eclipse Precision and replaces only the generated window-button material.
The existing kit has rounded-square plates but no circular three-state family.

Reuse the compositor-owned 26 logical pixel control bounds, 8 pixel gutters,
12 pixel inset, semantic minus/maximize/restore/close glyphs, and shared hit tests.
Normal is quiet smoked glass; hover illuminates the rim; press recesses the
center. Do not change control positions or glyph positions between states.
Preserve transparent corners, theme tinting, high-DPI filtering, and topmost-owner
hover routing. No idle animation or special destructive Close coloring.

The three equal cells in window-controls-circular-v2.png are normal, hover,
and pressed. Generate the runtime atlas through the build kit; source artwork
is not runtime verification. Studio overrides remain authoritative.

Provenance: built-in imagegen. Prompt: three equal, front-facing circular smoked
glass plates on transparent alpha, normal / illuminated icy-blue hover / dark
recessed pressed, thin silver rims, reflective crescents, no labels or glyphs.

Acceptance: production-rendered states at 1x/2x, unchanged hit geometry,
transparent corners, visibly distinct states, and installed window lifecycle.
