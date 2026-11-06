# Yahoo resource loading and modern page compatibility

The 2026-11-06 native baseline received Yahoo's document and ten stylesheets
successfully. During its 180-second emulated observation window, the serialized
transport continued dispatching JavaScript without reaching fonts or images.
This establishes resource starvation; it does not establish packet loss.

Changes:

- Replace parking_lot's bare-metal spin-only parker with the browser executor's
  existing atomic compare/park/wake service. A stalled native Yahoo run was
  sampled repeatedly in `parking_lot::RawMutex::lock_slow`; its waiter could
  monopolize the cooperative executor and prevent the lock owner from running.
  This adapter is selected only in the browser dependency overlay on native
  targets. The kernel and direct audio pipeline are unchanged.
- Keep document/CSS precedence and active-transfer ownership. After three
  script/API dispatches, service a queued visual resource. Alternate fonts and
  images when both are waiting; retain FIFO within each category.
- Use one shared browser preference constructor in production and native
  fixtures. Enable implemented IntersectionObserver, FontFace and variable
  fonts. Device APIs and transport authorization remain
  unchanged.
- Optimize the browser's Rust engine and adapter at level 2. The previous
  native component linked unoptimized development archives. Preserve development
  safety checks and established archive paths; omit debug information.
- Capture actual document readiness, font readiness, image decode counts and
  viewport/content dimensions in native public-site diagnostics.

Verification entry points:

```
./build-kit browser-core-tests
./build-kit browser-modern-web-tests
```

The modern-web fixture runs actual native Servo: an intersection callback starts
an image load, decoded image dimensions are checked, a FontFace loads from the
packaged catalog, and variable font axes change measured glyph widths.
It also forces real parking_lot mutex contention, owner handoff, and timed-lock
expiry on native executor threads.
It requires prepared native engine archives.

CSS container queries remain a known engine gap. The pinned Servo implementation
of `ServoDangerousStyleElement::query_container_size` is `todo!()`. Enabling the
experimental preference is not an implementation and must not be shipped as a
fix. Yahoo uses container queries; full responsive-layout parity remains an
acceptance item independent from font and image loading.

Public-site evidence is under `builds/evidence/yahoo-20261106-*`. A received HTTP
200, successful compilation, or a styled header alone is not full-page acceptance.
The direct AI audio pipeline is outside this change.

## Verification outcome

- Browser core: 64 behavioral tests pass; manifest
  `20261106T191745231995Z-73599.json`.
- Native ARM fixture: mutex contention/handoff/timeout, FontFace, variable font
  metrics and IntersectionObserver-triggered decoding pass; manifest
  `20261106T193637158011Z-74571.json`.
- Live Yahoo at 1536x864 in disposable ARM TCG: acceptance fails after 180 seconds.
  Five of 38 image elements decoded, zero reported broken images, document
  remained interactive; screenshot still contains overlapping content.
  Manifest `20261106T193743942200Z-74631.json`, evidence
  `builds/evidence/yahoo-20261106-native-parking-tcg/`.
  The previous lock stall did not recur in this run.
- Maximum observed engine/network/paint slices were 49.311/174.701/681.691 ms.
  Earlier unoptimized 800x600 baseline paint maximum was 3095.733 ms.
  These different-viewport emulation samples are diagnostic, not an installed
  VM performance guarantee or a controlled whole-page speedup measurement.
- A native virtualization fixture attempt could not obtain hardware entropy;
  its unsupported acceleration option was removed. No entropy fallback was added.

Remaining acceptance: finish Yahoo loading, implement native CSS container sizing
and its layout/restyle lifecycle, and verify responsive card placement and
images in the installed OS. Do not enable the unfinished container-query flag.
No updated ISO has been produced or published by this browser investigation.
