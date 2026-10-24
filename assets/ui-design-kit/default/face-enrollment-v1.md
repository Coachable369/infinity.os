# Facial setup — verification before login activation

Status: native source integration added; not yet compiled or production-rendered.
Reuse Eclipse Access typography, spacing, glyphs, wallpaper and progress markers;
reuse Eclipse Precision image-backed panels, fields and button states.
Kit gap: no mask illustration existed. Added original `face-mask-v1.png`, generated
with the built-in image generation tool, with true alpha outside the mask and in
the eye openings. It is an illustration, never a substitute for camera preview.

## Entry and authority

Offer optional setup after password/account creation. Explain that the camera
will capture three views and then perform a separate comparison. An explicit
Start action authorizes capture; Skip leaves password configuration unchanged.
Do not enable facial login as a side effect of setup or successful verification.
Never start the camera merely by displaying this page.

## Content and states

Use the current configuration content frame, eight-pixel rhythm and authored
Inter sizes. Place the medium-large mask on the right (maximum 480 square physical
pixels). Place the real 4:3 capture thumbnail below the guidance/name on the left,
explicitly labeled Camera capture. Keep Back/Skip and the primary action in
the existing navigation gutter, not overlapping the preview.

1. Explain and name: “Check facial recognition”; nonempty label, maximum 64 UTF-8
   bytes. Explain temporary capture and the fact that password login stays on.
2. Prepare: show actual camera/model readiness. Retry a recoverable device failure;
   unsupported camera or missing model must show the specific unavailable state.
   Never display a fake live preview or claim the decorative mesh is measured
   facial landmarks. The mesh runs only during real pending capture/comparison.
3. Capture: front, slight left, slight right. Show 1/3, 2/3, 3/3 and one clear
   instruction at a time. Every capture must have one sufficiently large face.
   Explain low light, overexposure, blur, multiple faces and incorrect pose with
   actionable guidance. A rejected sample does not fill its progress marker.
4. Review: allow Retake or Verify. Release capture while reviewing. Do not leave
   a camera session open while waiting indefinitely for user input.
5. Verify: request a separate capture and ask the person to face the camera.
   Require three spaced, fresh comparisons against every accepted view.
   Show a mismatch as “Not verified—try again,” never a successful enrollment.
6. Result: successful comparison reads “Camera comparison passed” and
   “Facial login remains off.” This is not a liveness/security certification.
   Until protected persistence exists, disclose that the check is temporary;
   do not label a button Save or claim the enrollment survives reboot.

The close/cancel/Back actions revoke capture and discard temporary biometric
buffers. Timeout, disconnect, permission loss and worker exit do the same.
Keyboard focus order follows the visual order; Escape cancels capture. Native
reduced-motion settings suppress the traveling scan (static illumination only).
The 187-node connector mesh follows the mask silhouette and excludes its eye
holes. A blue-white band sweeps down and up every 3.2 seconds, lighting nearby
connectors and nodes. Updates are time-based, capped at 30 Hz, and repaint only
the cached mask rectangle; no scene-wide blur, allocation or image decoding.
Idle, loading, review, failure, cancellation and completion do not animate.

## Generated artwork prompt

Original front-facing symmetrical blank human face mask; matte white porcelain
and silver shading, sculpted nose, subtle closed lips, hollow almond eye openings.
Orthographic, neutral, no neck/ears/pedestal. Soft cool blue edge illumination,
bright on dark navy. Entire mask with padding. True alpha outside silhouette and
eye openings. No background, text, watermark, checkerboard, dots or baked-in mesh.
The supplied watermarked stock example was not edited or shipped.

`tools/build-face-mask-artwork.py` packs a 480×480 BGRA BMP derivative through the
build kit. Both targets embed it; the System Generation design-kit copy includes
the master, derivative and specification. The PNG master remains unchanged.

## Completion gate

Do not wire this specification into a selectable configuration page before the
isolated camera decoder, detector and embedding worker can supply real results.
Review production screenshots with real frames and all error/focus states.
Never use numerical policy-unit-test vectors as visual or recognition evidence.
