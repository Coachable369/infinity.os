# Holographic Desktop icon

Generated using the built-in image-generation tool. Source:
`assets/apps/infinity-holographic-desktop-icon-v1-source.png` (1254 × 1254 RGBA).
Native bitmap: `assets/apps/infinity-holographic-desktop-icon-v1.bmp`.

## Generation prompt

Use case: stylized-concept. Asset type: premium InfinityOS Holographic Desktop application icon, one isolated object on a truly transparent background. Create three beautifully crafted floating desktop window panels in a shallow curved carousel, the central panel larger and forward, two panels receding behind it. A compact sculpted brushed titanium crescent beneath them suggests a holographic projector. Panels have dark midnight sapphire glass faces, fine satin silver edges, restrained luminous cyan and violet rims, and simple broad light shapes suggesting desktop windows without text or tiny UI. Strong recognizable unified silhouette, sophisticated tactile materials, NOT ice, NOT a generic monitor, no stand. Near frontal three-quarter 3D product rendering, carefully controlled bright highlights, dark blue and silver with violet accent. Centered square composition with about 10 percent transparent margin; highly readable at dock sizes, exquisite at 256px. Crisp clean alpha edges. No background scenery, floor, cast shadow outside the object, checkerboard, words, lettering, watermark, rounded-square enclosing tile, or extra objects.

## Integration

Replaces the procedural three-panel placeholder in the shared spatial identity
painter, including launcher callers and other uses of the Holographic Desktop
identity. The existing application-asset packaging includes both files on ARM64
and x86_64. The parity checker requires exact source and bitmap bytes from both
installed ESP images after rebuilding. No image resampling or alpha alteration
is applied during native BMP conversion.

ISO rebuild and installed-screen verification remain pending.

