# Native voice identity

Configuration screen 10 follows facial enrollment and precedes Ready. Three
explicit recordings create an account-specific speaker template. The third
recording must agree with both previous recordings. Skip preserves password
login. Captured PCM is not persisted.

The login button starts a 90-second account-bound voice attempt. Only a fresh
recording of “Infinity Hello” or “Hello Infinity” must first pass local speech
recognition and then match the enrolled speaker before starting the camera.
The phrase and speaker check use the same captured PCM; the words alone never
authorize login. Unrelated speech returns to capture without starting the camera. The face result must
be issued after speaker verification and belong to the same selected account;
only then can the combined proof create or unlock a session. Account changes,
cancellation, missing enrollment, mismatch and timeout fail closed. Password
login remains available. Desktop assistant capture, wake detection, recognition,
synthesis and interruption are unchanged.

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
pre-attempt recordings and pre-voice face proofs, expiry, encrypted roundtrip and corruption rejection.
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

## Enrollment retry lifetime (2026-11-19)

Enrollment reuses one microphone stream for up to four minutes, including
analysis and manual retry intervals. Inter-sample audio is drained and discarded;
each Record action discards queued DMA data and resets private PCM/resampler
state before collecting its new sample. Completion, Skip, Back, account change,
lease expiry and device failure close this workflow's stream. This changes only
voice identity enrollment, not the desktop assistant.

The user's frozen VirtualBox process was sampled before any VM changes. CPU0
was blocked in the emulated device lock, CPU7 in an audio timer/mixer lock, and
MixAIO-2 was spinning. The corresponding host capture restart events and sample
are preserved under `builds/validation/voice-enrollment-retry/`. This matches the
previously documented host-side zero-progress capture deadlock. Removing repeated
guest reopen calls reduces that trigger but cannot repair a stuck host library.
A rebuilt ISO is not proof that this host defect is fixed.

## Login greeting admission (2026-11-24)

Automatic biometric sign-in and the explicit biometric button use the login-only
capture owner. The prompt asks for either greeting above. Existing recognition
APIs are called without modifying desktop conversation behavior, speech assets,
or the speaker matching thresholds. Account/page changes cancel pending phrase
recognition before releasing its microphone capability. Password remains the
fallback. The existing bounded capture lifetime and retry behavior still apply.

The authentication harness checks both word orders, punctuation/case handling,
rejection of unrelated speech and phrase-only login, capture substitution,
account substitution, and fresh voice-before-face proof ordering. Physical
microphone acceptance of short greetings remains a VM testing requirement.

## Short-greeting enrollment and automatic recovery (2026-11-24)

Enrollment prompts alternate Hello Infinity / Infinity Hello / Hello Infinity,
matching the length and phonetic content used at sign-in instead of truncating a
long unrelated sentence. Each additional enrollment must agree with all earlier
recordings at 0.45, including the third sample. Existing encrypted templates remain
readable; re-enrollment is recommended for templates captured with the old sentence.

Combined login uses a 0.50 centroid floor and agreement of at least two samples at
0.45. The standalone SDK matching policy retains its 0.60 floor. These are
experimental tolerances, not a calibrated security/error-rate claim; the face
threshold is unchanged and a fresh same-account face proof remains mandatory.
VoiceIdentity logs record agreement count and centroid score, never PCM or vectors.

Auto sign-in remains enabled by default for new accounts. Recoverable voice/face
mismatch, quiet/clipped capture, and timeout rearm after two seconds, once the
worker is fully reclaimed. Missing enrollment, authorization, storage or device
errors do not create a retry storm. Explicit disable and password entry suppress
automatic attempts. No desktop conversation pipeline changes are included.
