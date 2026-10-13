# Fresh-install microphone calibration

## Observed failure

The installed VM serial trace retained at
`builds/validation/voice-startup-20261013/serial-before.log` shows recognizer warmup,
model readiness, desktop autostart, captured samples and completed calibration.
Each subsequent speech onset reaches the ten-second VAD limit without submitting
an utterance. Capture authority continues rotating afterward. This establishes a
segmentation stall, not failure to load Whisper or the model. The trace does not
establish an automatic transition to Off, nor does it contain microphone levels.

## Reproduced defect and change

The baseline used the minimum energy of 25 frames. Just two cold zero frames
followed by 23 ambient frames at amplitude 500 produce threshold 300 instead of
1000. Sustained ambient sound then prevents the required end pause indefinitely.
The production-controller regression failed on this threshold before the fix.

Use median frame energy instead, retaining the half-second sample-based
calibration, fixed memory, threshold bounds and existing wake-word policy.
The controller regression checks idle noise produces no recognition request,
then speech followed by the same ambient noise produces one recognition request.
Separate tests vary chunk boundaries, startup zeros and clipped impulses.

Calibration and VAD transitions now emit `[VOICE CAPTURE]` with five hexadecimal
values: monotonic nanoseconds, state, threshold, mean-square PCM energy and peak.
States are 0 calibration, 1 waiting, 2 speech, 3 complete, 4 no speech, 5 limit,
6 cancelled. No PCM or transcripts are logged. Explicit widget start/stop is
also logged to distinguish user toggles from capture failure.

## Verification boundary

The full voice behavioral gate passed, including 29 controller/calibration tests.
This reproduces and fixes a real calibration failure matching the observed stall;
the old trace cannot prove that startup zeros caused this particular microphone's
baseline. Fresh installed microphone acceptance remains required, using the new
numeric trace to distinguish ambient noise, clipping and actual pauses. No VM was
restarted or reprovisioned during diagnosis.
