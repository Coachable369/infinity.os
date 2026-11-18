# Facial enrollment — wave glass design kit

Generated reference: `reference.png`. Reusable transparent illustration:
`face-sprite.png`, mirrored into `assets/boot/installer-assets/Face-Enrollment-Scan-v2.png`
for authored Studio image loading. `pose-guides.png` provides front/left/right empty-state glyphs; accepted camera photos replace them. All artwork uses original built-in image-generator outputs.

## Composition

Reference viewport 1672 × 941. The full-screen background is the existing
`installer-assets/Glossy-Blue-Infinity-Wave-Wallpaper-2.png`, unchanged.
One Precision glass panel occupies normalized (45,65,910,870). Left content
begins at x=85; eyebrow y=92, title y=130, subtitle y=200. Camera card
(85,270,300,310), guidance to its right. Three equal review cards begin at
(85,600), (255,600), (425,600), each 158 × 200 normalized. Face illustration
(600,85,340,700); bottom navigation y=835. Positions use the Studio 1000-unit
canvas. Inter typography and Precision button/field states are authoritative.

The identity-name field remains editable beside the live preview. It also
provides the existing Retake action during review. This functional field is
an explicit addition to the generated reference, not a claim of pixel identity.

## Reusable scanning-face component

The generated transparent face sprite is composited once into the cached base.
Three feathered cyan horizontal bands travel over it on a 6.4-second loop.
Restore the cached rectangle before each frame; never accumulate scan trails.
Animation is bounded to the illustration rectangle at 30 Hz. Reduced motion
uses a fixed scan position. The illustration is decorative, not measured data.
Legacy authored screens without `faceSprite` retain their canonical mesh renderer.

## Data and control contract

Front, Left and Right cards display only the corresponding actual enrollment
photo, aspect-fitted without distortion. Pending, Checking and Accepted are
runtime states. Rejection must preserve already accepted photos and show the
real error. The live preview cannot overwrite accepted captures. Back, Skip,
Capture, Verify/save and Continue retain the existing enrollment dispatch.
All geometry, labels and image layers remain editable in Infinity Studio.

## Generated prompts

Reference: high fidelity InfinityOS facial configuration, exact supplied wave
wallpaper, one midnight navy glass panel, silver face with cyan mesh and scan
lines on right; left live camera, capture guidance, three equal Front/Left/Right
cards; Inter typography; Back, Skip for now and Capture front view glass buttons.
No simulated human photographs, no device frame, complete visible content.

Sprite: extract only reference right-side silver face, preserve materials and
cyan connectors/nodes, remove scan bands for runtime animation, true alpha around
face and eye openings, no wallpaper, controls, text, neck or body.

## Acceptance

Compare production-native rendering to reference at identical viewport size.
Check geometry, readability, imagery, controls, camera aspect, capture state
publication, bounded scan damage and reduced motion. Render evidence and any
remaining visual differences must be reported; generation or compilation alone
is not visual acceptance. No recognition thresholds or audio behavior change.

### Visual comparison notes

The native composition uses the supplied wallpaper, generated face, three capture
cards, hierarchy and navigation positions from the reference. Native UIKit owns
glass borders and button states; its shading is darker and thinner than the
AI-generated reference. The native font rasterizer also differs from generated
lettering. These are remaining visual differences, not pixel-identical acceptance.
The editable identity field and truthful RGB limitation are retained functional
additions. No fake camera portraits are used.
