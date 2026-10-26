# Local facial enrollment

The optional configuration page starts a password-authorized live camera preview.
Capture records one front view, then angled left and right views. Recoverable
quality/pose failures preserve previous captures. Full 90-degree profiles are not
supported by the current five-landmark detector.

**Verify & save identity** explicitly authorizes persistence. Three fresh frontal
comparisons must match every enrolled view. Only then can the fixed isolated
worker export three 128-float embeddings and three aligned 56x56 grayscale
portraits. Transfers are sequential, command-bound and capped at 4096 bytes.
General applications cannot select this worker or invoke its private dispatcher.

The kernel rechecks the exact password-authorized account/worker binding, encrypts
the record, atomically commits it to native storage, reads it back and authenticates
the recovered payload before displaying saved status. Failures never display
successful enrollment. Temporary plaintext is wiped on failure, cancellation and
successful save. No raw camera frame is persisted.

## Record v1

Account-specific path: `/system/identity/face/<32 lowercase hexadecimal digits>`.
The account is the full native StableId, not the worker's opaque account handle.

| Bytes | Value |
| --- | --- |
| 0–7 | `INFFACE1` |
| 8–23 | Account ID |
| 24–55 | Pinned recognition-model digest |
| 56–119 | UTF-8 label, zero-padded |
| 120 | Label byte length |
| 121–127 | Reserved zero |
| 128–139 | Fresh 96-bit nonce |
| 140–147 | Capture-session monotonic nanoseconds, not wall time |
| 148–159 | Reserved zero |
| 160–11103 | Encrypted vectors and portraits |
| 11104–11119 | Authentication tag |

The exact size is defined by `face_vault::BYTES`; all header bytes are authenticated
associated data. Encryption uses the existing ChaCha20-Poly1305 implementation.
HKDF-SHA256 derives an account-specific key from the native node seed with a face
enrollment domain. Nonces use the existing boot-seeded native entropy service.

Protection inherits native node-key custody. This is not a TPM/secure-enclave or
encrypted-disk claim. A saved enrollment is **not an active login credential**;
password authentication stays unchanged. Liveness/anti-spoofing, security accuracy
qualification, enrollment management/revocation UI and facial login remain gates.

## Verification status

Focused policy, transfer, authenticated-encryption and native-pixel checks pass.
The host rendering fixture has no camera input and is not recognition evidence.
ARM64 and x86_64 installed kernels compiled with the final painter refactor.
The full release attempt stopped at an unrelated voice-toggle harness compilation
failure. No new release ISO or installed-runtime acceptance is claimed.

Remaining device acceptance: attach a supported camera; install; capture the same
person from all three guided angles; retry a rejected side view; verify/save;
restart without ISO; authenticate and inspect the saved record through the native
vault boundary. Separately exercise cancellation, disconnect, wrong-person,
storage failure and authorization revocation. Do not enable face-only login until
the separate authentication/liveness gates are met.
