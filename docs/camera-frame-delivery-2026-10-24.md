# Camera preview delivery correction

## Report and findings

The installed setup screen reported camera failure (104) after briefly starting
the VirtualBox camera, without a visible preview. This report does not prove a
recognition-model failure: the worker returns 104 before JPEG decoding.

Two source-level delivery faults were corrected:

- Metadata selected a sequence but did not retain its frame. Every camera poll
  could overwrite the single slot while the isolated worker yielded between
  4 KiB reads. This could repeatedly invalidate delivery. Metadata now pins one
  bounded snapshot; incoming frames are dropped until its final page is copied.
  Disconnect, cancellation, expiration and revocation invalidate it.
- Capture authority expired after five seconds, before the worker's ten-second
  capture budget. The service now permits a maximum fifteen-second lease; the
  worker retains its shorter ten-second capture watchdog. No unbounded streaming
  or new camera authority is introduced.

An expired read now reports timeout rather than generic disconnect. Exceptional
delivery errors have bounded diagnostic messages without image or identity data.
The startup check intentionally closes the stream after obtaining one snapshot;
subsequent enrollment captures start their own bounded streams. It is not a
continuous live-preview implementation.

## Behavioral evidence

`tools/camera-service-test.rs` delivers the existing real guest JPEG through native
user-memory checks while injecting a different frame before each page read. It
asserts byte-identical delivery, resumption of publication after delivery,
revocation while pinned, and availability beyond the old five-second cutoff.
The full existing camera service/lifecycle harness passed via build-kit:
`builds/manifests/20261024T130536430177Z-29728.json`.

These are service-boundary tests, not a new installed-camera capture. The exact
cause of the user's failed attempt cannot be established from its generic 104
status alone. Installed preview/capture verification remains required.
