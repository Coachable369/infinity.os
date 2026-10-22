# Native facial authentication audit — 2026-10-22

Camera integration is implemented in source. Facial authentication is **not
implemented or enabled**. This is not a security-qualified facial-login release.

## Pinned candidates, not adopted dependencies

OpenCV Zoo revision: `48b10fa505aeb51fbd127fc60a5c660040e823b7`.

| Asset | Git LFS SHA-256 | Bytes | Published directory license |
| --- | --- | ---: | --- |
| `face_detection_yunet_2023mar.onnx` | `8f2383e4dd3cfbb4553ea8718107fc0423210dc964f9f4280604804ed2552fa4` | 232589 | MIT |
| `face_recognition_sface_2021dec.onnx` | `0ba9fbfa01b5270c96627c4ef784da859931e02f04419c829e83484087c34e79` | 38696353 | Apache-2.0 |

These hashes were read from pinned LFS pointers, not calculated from downloaded
weights. Neither weight file is installed. The directory READMEs explicitly
apply their licenses to all directory files. Retain license/attribution files
if adopted. Sources:
- [YuNet](https://github.com/opencv/opencv_zoo/tree/48b10fa505aeb51fbd127fc60a5c660040e823b7/models/face_detection_yunet)
- [SFace](https://github.com/opencv/opencv_zoo/tree/48b10fa505aeb51fbd127fc60a5c660040e823b7/models/face_recognition_sface)

Rust candidate: tract 0.22.0, revision
`2f8f99bc6c22ec44eba810eea49ca9602255408f`, MIT OR Apache-2.0.
Its ONNX crate directly depends on memmap2, prost, tract-hir, tract-nnef and
tract-onnx-opl. Optional workspace backends are not necessarily linked.
The resolved feature-selected transitive dependency graph is **not yet audited**;
no C++-free native build is claimed. Do not add the workspace wholesale.
[Pinned manifest](https://github.com/sonos/tract/blob/2f8f99bc6c22ec44eba810eea49ca9602255408f/onnx/Cargo.toml).

## Concrete integration constraints

- Ordinary isolated app images have a 256 KiB limit in `native_c_image.rs`.
  Embedding 38.7 MB of weights cannot work. A separately budgeted trusted worker
  must load immutable model objects through scoped storage operations; do not
  raise all app limits. JPEG decoding also belongs outside the kernel.
- ARM uses a kernel-private retained-UEFI adapter. x86_64 exits boot services
  and has no native USB streaming backend. Shared framing compiling on both
  architectures is not x86 camera support.
- `boot/common/tpm_random.h` supplies random bytes, not seal/unseal, measured
  key release or rollback protection. `NodeCrypto::seal` encrypts with a supplied
  key; it is not TPM key custody.
- The camera supplies RGB MJPEG. No evaluated presentation-attack detection
  exists. An embedding match alone does not establish live presence.

## Required authentication boundary

Preserve password login/recovery. Never expose a public unlock IPC. The identity
service must own an expiring single-use challenge bound to account, session/boot,
enrollment revision, camera lease and trusted worker instance. Only that worker's
authenticated completion may consume it. Cancel, logout, device loss, timeout
and revocation invalidate both challenge and capture. Multiple faces or poor
quality fail closed. An arbitrary app must not submit a score to gain authority.

Named enrollment creation/rename/deletion requires password authorization.
Template AEAD associated data binds account, enrollment ID, schema, model hash
and revision. Nonces must be unique. Reuse reviewed AEAD; never store an
unprotected key beside encrypted templates.

Cold-login key release requires a native measured-boot-backed key provider with
tested update/recovery and rollback behavior. Inspect actual measurements before
choosing PCR policy. A password-derived key is an alternative only if password
entry stays mandatory; it does not provide passwordless cold facial login.

## Security gate remains closed

Enable face-only authentication only after the dependency/weight audit, real
isolated inference, protected key custody, and camera-specific anti-spoofing and
false-accept evaluation. No wizard controls pretending these missing services
exist have been added. A convenience recognition demo is not secure login.
