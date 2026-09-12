# Bootstrap light reveal

Motion design kit: retain the existing eclipse background, illuminated infinity
artwork, typography and progress indicator. No new raster assets or UI controls.
The emblem emerges from darkness through a broad soft exposure front, rather
than a hard split. Cubic easing gives the center a quiet ignition, accelerates
through the ribbon, then slows toward the outside edges. The last frame is the
unaltered original emblem. Keep particle animation for the subsequent menu;
it must not restore fully exposed artwork ahead of the reveal.

Render only columns whose exposure changed, restoring their original background
before blending. Do not allocate frame images or repaint the full scene each
frame. Target a shorter 2.4-second sequence plus actual rendering cost.

Acceptance: symmetric monotonic exposure, soft partially transparent edges,
no accumulating alpha, exact final artwork, and unchanged startup menu handoff.
Visual ISO review remains required before claiming cinematic-quality acceptance.
