# Native voice identity

Configuration screen 10 follows facial enrollment and precedes Ready. Three
explicit recordings create an account-specific speaker template. The third
recording must agree with both previous recordings. Skip preserves password
login. Captured PCM is not persisted.

The login button first runs the existing face verifier. Its one-use proof is
consumed by a 90-second account-bound voice attempt. A fresh voice capture is
required after that face proof; account changes, cancellation, unavailable
hardware, missing enrollment, mismatches and timeouts cannot create a session.
The previous login greeting listener is no longer activated. Desktop assistant
capture, wake detection, recognition, synthesis and interruption are unchanged.

## Native implementation

`infinity-speaker-core` implements 16 kHz, 80-band Kaldi-compatible features and
executes pinned WeSpeaker English VoxCeleb ResNet34 weights using the existing
Rust tensor executor. `infinity-speaker-worker` runs in isolated native memory
on ARM64 and x86_64. Neither a Linux service nor a host inference runtime is
required. ONNX Runtime and Kaldi are development-only numerical references.

Each enrollment stores three normalized 256-float vectors. Matching requires
cosine >= 0.60 against their normalized centroid and >= 0.45 against at least
two individual recordings. These thresholds are experimental, not a calibrated
false-accept-rate guarantee. Voice matching is text independent: the displayed
phrase guides recording but its words are not an authentication secret.

Templates use authenticated encryption with a dedicated node-derived key,
account and model identifiers as authenticated metadata, and fresh nonces.
They persist at `/system/identity/voice/<account-id>` in the object store.
The model, head and target-specific worker are embedded in both live and
installed kernels. `tools/installed-kernel-parity-test.py` checks their bytes.
Model attribution ships in `/System/Notices/WeSpeaker.txt`.

## Acceptance evidence and outstanding limits

Numerical checks compare actual native features and embeddings to Kaldi/ONNX
outputs. Authentication tests exercise stale proofs, account substitution,
pre-face recordings, expiry, encrypted roundtrip and corruption rejection.
A held-out genuine recording and two other speakers exercise the production
matching policy. Native screenshot captures cover idle, recording, comparing,
retry and saved presentation.

These checks do not establish a production biometric error rate, liveness,
resistance to recorded/cloned speech, physical microphone operation, or
successful combined biometric login. RGB face matching plus speaker matching is
vulnerable to presentation attacks. A disposable ARM VirtualBox guest completed
installation, booted without the ISO, displayed voice enrollment after facial
setup, handled unavailable microphone input with retry/skip, completed setup,
and returned to the remembered account at login after disk-only reboot. Actual
microphone enrollment, successful combined login and failed-speaker login remain
acceptance items until recorded evidence exists. Never describe model similarity alone as replay-resistant authentication.

Visual review: the native screen uses the shared configuration artwork and
controls. Exact reference parity remains open: the generated reference and
native render differ in font weight, panel edging, microphone scale, and
waveform shape. Screenshot coverage is not a claim of pixel-identical parity.

## Enrollment responsiveness corrections (2026-11-18)

Camera admission uses monotonic session tickets independent of hardware frame
counters that reset on stream reopen; hardware freshness and exposure checks
remain required. Authored face/voice hit geometry is cached per viewport. These
pages suppress full-scene decorative repaints between their bounded animation
frames. Recording activity remains visible before speech; comparison has its
own pulse. Retry preserves accepted samples after capture timeout/unavailability.
Quiet, clipped, capture timeout, analysis timeout and worker failures are distinct;
failures and worker terminal state are written to `/Logs/VoiceIdentity.log`.

The native ARM speaker worker completed with real EL0 isolation under QEMU HVF:
24.43 ms preparation, 2841.43 ms inference, 8.87 ms scrub/reclaim, 704 visits,
all private memory reclaimed. This is a recorded-PCM worker measurement, not
physical microphone or installed end-to-end latency. Camera restart regression,
authentication policy and native render checks pass. User-reported physical
capture/retry behavior remains an installed-runtime acceptance item.
