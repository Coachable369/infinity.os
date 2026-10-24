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

## Release

Clean release passed through `./build-kit full --skip-tests`, including kernel
currentness, live/installed model and artwork parity, and checksum verification.
Manifest: `builds/manifests/20261024T130606592353Z-29756.json`.
Broad regressions were deferred; the focused camera harness above passed.

| Installer | SHA-256 |
| --- | --- |
| `builds/InfinityOS-aarch64.iso` | `5a4df5d64a754d2215071c584539fdc8643e3f55fc9b727870ca9dc63097afe6` |
| `builds/InfinityOS-x86_64.iso` | `95c0b29f85194d38e3e8de85e3d93b6f55b9a6902955ca62e359a4abdd9040c0` |

Implementation commit: `942816ee` on mainline. The running VM was not modified.
