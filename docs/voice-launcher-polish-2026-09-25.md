# Speech input and launcher polish — 2026-09-25

## Speech input

The superseded native recognizer treated VAD-completed PCM as a complete
utterance rather than restarting streaming channel normalization for each
recording. There are no forced word substitutions or transcript corrections.
Digital silence returns no transcript without invoking the decoder.

`make voice-recognition-test` passed six freestanding ARM64 QEMU cases:
recorded speech, repeat speech, cancellation, insufficient output capacity,
digital silence, and immediate cancellation. The recorded fixture now returns
`go forward ten meters`, rather than the previous `go forward ten years`.
Repeat runs retained 1,316 bytes, with 108,306,560 committed heap bytes. Transcript
bytes, result codes, memory state, and erasure counters are asserted directly.
This historical evidence was superseded by the native Whisper behavioral probe.

Recognition remains background work. Cancellation is checked before and after
the bounded, at-most-ten-second recording decode, before publication; it cannot
interrupt the upstream batch call itself. This increases cancellation latency
relative to chunked processing. QEMU TCG fixture decoding took 6.46–6.57 seconds;
these are not installed-desktop latency measurements.

This does **not** establish that the user's microphone recording of “Hello” is
correct. A real recording and installed microphone round-trip remain required.
The historical small English acoustic model had accuracy limitations and has
been removed from the runtime and build.

## Three desktop UX enhancements

All reuse the existing launcher visuals and animated closing path.

1. Up/Down follow grid rows, including partial rows and category boundaries.
   Tab/Shift+Tab traverse sequentially; Shift+Tab no longer unexpectedly opens
   the app switcher while the launcher owns focus.
2. Enter from search opens its first matching app. Empty results are a no-op,
   not an unrelated category launch.
3. Escape clears a populated search and resets focus/scroll first. A second
   Escape closes the launcher.

`make app-launcher-interaction-test` passed the two new navigation tests and
existing interaction harness. The new functions are wired into the native
console launcher input path; these are behavioral tests, not screenshot QA.

## Packaging

The native recognizer and launcher are linked into both live and installed
kernels through the existing model-inclusive build. No new optional payload
component or host inference dependency was introduced. `sh tools/build-hermes.sh`
completed successfully after these changes. The fresh installer is
`builds/InfinityOS-aarch64.iso` (5,503,328,256 bytes), SHA-256
`8f1affe853f78668c1a9264ca469c3c233f5f004d444013931d6bf7cb2cfc3c5`.
Binary installed-kernel, loader, and speech-model parity passed. Installed VM
interaction is not yet verified for this revision. Other architecture ISOs
were not rebuilt.
