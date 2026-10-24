# Facial setup — verification before login activation

Status: interaction specification; not a rendered or implemented wizard screen.
Reuse Eclipse Access typography, spacing, glyphs, wallpaper and progress markers;
reuse Eclipse Precision image-backed panels, fields and button states. No new
illustration is needed. A generated face must never stand in for camera preview.

## Entry and authority

Offer optional setup after password/account creation. Explain that the camera
will capture three views and then perform a separate comparison. An explicit
Start action authorizes capture; Skip leaves password configuration unchanged.
Do not enable facial login as a side effect of setup or successful verification.
Never start the camera merely by displaying this page.

## Content and states

Use the current configuration content frame, eight-pixel rhythm and authored
Inter sizes. Place a live, aspect-fitted 4:3 preview on the right, guidance and
named enrollment field on the left. Keep Back/Skip and the primary action in
the existing navigation gutter, not overlapping the preview.

1. Explain and name: “Check facial recognition”; nonempty label, maximum 64 UTF-8
   bytes. Explain temporary capture and the fact that password login stays on.
2. Prepare: show actual camera/model readiness. Retry a recoverable device failure;
   unsupported camera or missing model must show the specific unavailable state.
   Never display a fake live preview, fake scan, or endless loading animation.
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
reduced-motion settings suppress decorative motion. Only real preview damage
and changed status regions repaint. No permanent idle glow or scene-wide blur
recomputation is introduced.

## Completion gate

Do not wire this specification into a selectable configuration page before the
isolated camera decoder, detector and embedding worker can supply real results.
Review production screenshots with real frames and all error/focus states.
Never use numerical policy-unit-test vectors as visual or recognition evidence.
