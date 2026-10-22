# Facial login first-pass implementation gates

Date: 2026-10-22

The requested delivery remains incomplete. Do not advertise facial enrollment,
login, or unlock as installed functionality. Password authentication is unchanged.
The user requested no repeated end-to-end tests between implementation steps;
compilation checks are allowed and full pipeline acceptance is deferred.

## Concrete findings

1. Installed dispatch/backend/client are now connected in source; see
   `CAMERA-TRANSPORT-STATUS.md`. The decoded JPEG remains firmware-probe evidence;
   installed capture has not been exercised in this pass.
2. `boot/common/uefi_loader.c` retains boot services on ARM64 but exits them on
   x86_64. Passing the diagnostic firmware USB pointer into an installed client
   is not an acceptable shared transport implementation.
3. `kernel/runtime/crypto.rs::NodeCrypto::seal` is ChaCha20-Poly1305 payload
   encryption with a supplied key, not TPM sealing. No native TPM seal/unseal or
   measured-boot key-release implementation was found in the kernel. Storing a
   random encryption key next to encrypted templates is not protected pre-login
   key custody. Password-derived protection cannot enable passwordless cold login.
4. The identified camera provides RGB MJPEG. No presentation-attack detection
   implementation or evaluated anti-spoofing evidence exists. Recognition scores
   alone must not grant login authority.
5. No inference runtime/model has been adopted. Pinned YuNet/SFace candidates,
   published licenses and pre-login key design are recorded in
   `FACIAL-LOGIN-NATIVE-AUDIT.md`.

## New camera diagnostic code

The probe now sends a bounded CLEAR_FEATURE(ENDPOINT_HALT) request to endpoint
0x82 when ending capture, records EFI status, USB status and elapsed milliseconds
as three little-endian u64 values in `shutdown.bin`, and wipes capture buffers.
The runner extracts the binary result with the other evidence.

This is a shutdown request, not proof that the sensor has powered down. Its
VirtualBox behavior, camera LED, cancellation and reconnect must be checked on
the subsequent pipeline run. No new camera test was run in this pass.

## Required implementation order

- Complete installed USB transport, authenticated dispatch integration and the
  isolated capture client. No host-camera or host-inference substitutions.
- Implement protected pre-login key custody with explicit recovery and update
  policy, or retain a required second factor; do not silently downgrade security.
- Pin and audit compatible Rust/C inference and redistributable weights.
- Add bounded detection, one-face verification and presentation-attack handling
  in an isolated process. A positive match is evidence, not unlock authority.
- Add encrypted per-account named enrollments, password-authorized management,
  revocation, rate limits, and single-use account-bound authentication attempts.
- Integrate actual capture/enrollment into the reusable wizard UI, not a
  placeholder enrollment screen. Publish incremental ARM64 test media before
  manual pipeline review; verify x86_64 independently before claiming parity.

## Reference evidence

- UEFI USB protocols: https://uefi.org/specs/UEFI/2.11/17_Protocols_USB_Support.html
- UVC bulk shutdown precedent (primary driver source):
  https://gbmc.googlesource.com/linux/+/refs/heads/linux-5.18.y/drivers/media/usb/uvc/uvc_video.c
- InsightFace model licensing:
  https://github.com/deepinsight/insightface/blob/master/python-package/docs/model_zoo.md
- Facial presentation-attack guidance (not a certification claim):
  https://pages.nist.gov/800-63-4/sp800-63b.html
