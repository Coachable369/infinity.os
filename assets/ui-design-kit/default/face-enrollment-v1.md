# Facial setup — verification before login activation

Status: native source integration added; not yet compiled or production-rendered.
Reuse Eclipse Access typography, spacing, glyphs, wallpaper and progress markers;
reuse Eclipse Precision image-backed panels, fields and button states.
Kit gap: no mask illustration existed. Added original `face-mask-v1.png`, generated
with the built-in image generation tool, with true alpha outside the mask and in
the eye openings. It is an illustration, never a substitute for camera preview.

## Entry and authority

Offer optional setup after password/account creation. Explain that the camera
will capture three views and then perform a separate comparison. Entering this
optional page starts the password-authorized live preview. Only the Capture
button records an enrollment photo; Skip leaves password configuration unchanged.
Do not enable facial login as a side effect of setup or successful verification.
Leaving the page immediately revokes capture authority and clears preview pixels.

## Content and states

### Enrollment persistence extension (2026-10-26)

Reuse the existing generated alpha mask, Precision surfaces and Access font roles;
no replacement icon family or alternate window chrome is needed. Increase the
live preview to 208 height-normalized pixels wide, retaining 4:3 geometry. Active
scanning adds a feathered, counter-traveling illumination band and brighter node
cores; idle remains subdued, and reduced motion remains static. The mesh is
illustrative, not tracked landmarks. This is the visual implementation target;
production screenshot acceptance is still required.

The explicit review action is **Verify & save identity**. Save three matching
128-dimensional vectors and three aligned 56x56 grayscale portraits (front,
angled left, angled right), encrypted with account-scoped ChaCha20-Poly1305.
Use the protected native node key service and fresh boot-seeded entropy, never
public keys or password verifiers as encryption keys. Bind account, pinned model,
label and record version as authenticated metadata. Publish saved status only
after atomic native-object commit and authenticated readback. Never store raw
camera frames or unencrypted biometric data. Cancel erases temporary buffers;
it does not remove a previously committed enrollment.

This recognizer requires visible eye landmarks: full 90-degree profiles are not
supported. Guided angled side views must not be advertised as full-profile
recognition. Enrollment does not enable facial login or certify liveness. Disk
protection inherits the native node-key custody threat model, not a claimed TPM
or secure-enclave guarantee. Identity management/revocation and installed-camera
acceptance are separate remaining gates.

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
   facial landmarks. Entry checks a real camera frame before loading models;
   after preparation, preview frames continue while awaiting Capture. Frames
   publish atomically; partial or superseded frames never overwrite the display.
   Preview frames are not enrollment photos. Bounded camera leases are reopened
   during preview; Review and verification do not leave an idle stream open.
3. Capture: front, slight left, slight right. Show 1/3, 2/3, 3/3 and one clear
   instruction at a time. Every capture must have one sufficiently large face.
   Explain low light, overexposure, blur, multiple faces and incorrect pose with
   actionable guidance. A rejected sample does not fill its progress marker.
4. Review: allow Retake or Verify. Release capture while reviewing. Do not leave
   a camera session open while waiting indefinitely for user input.
5. Verify: request a separate capture and ask the person to face the camera.
   Require three spaced, fresh comparisons against every accepted view.
   Show a mismatch as “Not verified—try again,” never a successful enrollment.
6. Result: show saved only after encrypted durable readback; a write/readback
   failure is an explicit failure, never successful enrollment. Password login
   remains enabled. This is not a liveness/security certification.

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

### Capture review feedback

Reuse the existing Field surface for three 4:3 review thumbnails beside the live
preview, labeled Front, Left and Right. Only an accepted explicit capture fills
its slot; live preview and rejected captures must never replace earlier accepted
photos. Show Accepted/Pending beneath each slot and Checking photo while the
capture command is pending. Transient review images are scrubbed on cancellation,
failure, restart and teardown. Old/too-soon frames allow a retry within the
existing session deadline without losing accepted views. These review images
are not additional persistent biometric records.

Do not wire this specification into a selectable configuration page before the
isolated camera decoder, detector and embedding worker can supply real results.
Review production screenshots with real frames and all error/focus states.
Never use numerical policy-unit-test vectors as visual or recognition evidence.
