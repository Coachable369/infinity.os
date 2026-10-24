# Native facial setup: first implementation pass

## Scope and status

Optional configuration-time camera comparison, **not facial authentication**.
Password login is unchanged. No biometric credential is persisted, no login
switch is enabled, and no worker result can unlock a session. The page explicitly
discloses temporary capture and deletion. This deliberately avoids claiming that
a conventional RGB camera supplies presentation-attack detection.

Source implementation is present; compilation and execution were deferred at the
user's request until the source pass was wired. The first ARM64 build attempt
then stopped at build-kit serialization: `20261024T040336525080Z-7368` owned the
active lock, so no compilation ran. No recognition accuracy, installed-system success, performance,
updated ISO, or security qualification is claimed by this document.

## Implementation map

- `sdk/infinity-face-core`: fixed float32 operator runtime, bounded YuNet output
  decoding, five-landmark similarity alignment, SFace embeddings, quality and
  enrollment policy, private worker protocol, C-only MJPEG decoder.
- `sdk/infinity-face-worker`: native user-mode executable; no host API table,
  C++ runtime, Python runtime, filesystem model loading, or kernel inference.
- `kernel/runtime/face_setup.rs`: ordered capture/review/verification state,
  per-command deadlines, atomic real-thumbnail publication and cancellation.
- `kernel/runtime/face_setup_dispatch.rs`: checked private-memory copies and
  fixed model reads, bound to the launcher's process identity. No unlock opcode.
- `kernel/runtime/native_face_job.rs`: bounded process visits, revocation and
  cooperative private-page erasure before reclamation.
- `kernel/runtime/face_setup_host.rs`: fixed packaged worker/models and setup
  bridge; shared application logic on ARM64 and x86_64.
- `kernel/drivers/camera.rs`: separate short-lived password-authorized setup
  authority; onboarding completion, deactivation and expiry revoke access.
- Configuration console plus `face_setup_view.rs` / `face_setup_layout.rs`:
  optional named front/left/right captures and a separate fresh-camera check,
  shared kit controls, keyboard/pointer actions and actionable failure messages.

The existing x86_64 native streaming backend remains unavailable; the page must
say so and offer Skip. Sharing the worker does not establish camera support on
that architecture.

## Pinned dependencies and licensing record

OpenCV Zoo revision: `48b10fa505aeb51fbd127fc60a5c660040e823b7`.

| Asset | SHA-256 | Upstream notice |
| --- | --- | --- |
| YuNet 2023mar ONNX | `8f2383e4dd3cfbb4553ea8718107fc0423210dc964f9f4280604804ed2552fa4` | model-directory MIT license, Shiqi Yu |
| SFace 2021dec ONNX | `0ba9fbfa01b5270c96627c4ef784da859931e02f04419c829e83484087c34e79` | model-directory Apache-2.0 license |
| stb_image.h | `594c2fe35d49488b4382dbfaec8f98366defca819d916ac95becf3e75f4200b3` | MIT option from dual-license notice |

stb revision: `003501f6d7a8668ce8b6d875e34dd97ad0d5d5b5`.
Only JPEG is compiled; stdio, HDR, linear conversion, SIMD intrinsics and thread
locals are disabled. The decoder uses a fixed private arena. The original notice
is retained. Rust dependencies are the local face-core crate and compiler/core
support only; no ONNX Runtime, OpenCV runtime, tract, C++ inference library, or
transitive ML framework is adopted. Build-time conversion uses Python's standard
library, not a guest Python dependency. The source packer accepts only the two
pinned input hashes and rejects unsupported graph operators.

This records the upstream distribution notices, not independent assurances about
training-data provenance or suitability for security authentication. Public or
security-qualified deployment still needs a documented model/security review.

## Bounds

- Camera: 640×480 MJPEG, encoded frame at most 1,228,800 bytes.
- Decoder scratch: 8 MiB; RGB buffer: 921,600 bytes.
- Thumbnail: 160×120 RGB, 57,600 bytes; sequential writes at most 4 KiB.
- Worker heap: fixed 192 MiB; ordinary app heap limits are unchanged.
- Model: each package at most 48 MiB; activation arena at most 128 MiB.
- Inference: cooperative callbacks between bounded output-element visits,
  plus existing hardware process preemption.
- Setup: four-minute outer lifetime; one-minute command limit. Core enrollment
  retains its stricter two-minute policy deadline.
- Private worker image/heap/stack: scrubbed in at most 64 KiB visits before free.
- Recognition policy cosine threshold 0.6 is experimental, not a calibrated
  false-accept guarantee. Three fresh matches are not liveness proof.

## Build integration and deferred checks

`native-face` is a default feature. Both installed and live kernel prerequisites
build the same worker and embed the same model bytes. Named focused profiles are
`./build-kit face-aarch64` and `./build-kit face-x86_64`; neither has run yet.
`tools/face-package-manifest.py` records exact output hashes and explicitly marks
installed/security verification false. It is inventory, not acceptance proof.

`tools/face-setup-test.rs` exercises ordered progress, replay rejection, thumbnail
publication and timeout behavior. Existing face-core tests exercise numerical
and policy transitions. These are authored but unexecuted; synthetic fixtures
cannot substitute for real-camera evidence.

Next validation must compile the whole pass once, fix surfaced errors, and then
exercise the complete installed pipeline with the installer detached. Record
actual capture, three views, fresh comparison, wrong-person rejection, camera
loss, cancellation, permission revocation and continued desktop responsiveness.
Inspect production-rendered geometry against the kit. Protected persistence,
reboot enrollment restoration, PAD/liveness qualification and login activation
remain separate security gates—not hidden behind a successful comparison label.
