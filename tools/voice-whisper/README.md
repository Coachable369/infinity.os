# Native Whisper recognition

InfinityOS uses the pinned CPU-only `whisper.cpp` v1.8.2 engine and the official
`tiny.en` model for offline English recognition. The engine, complete model, and
upstream license are immutable bytes in the live and installed kernels. Runtime
recognition does not use Linux, macOS, a cloud API, a host process, dynamic
libraries, or a filesystem.

Both aarch64 and x86_64 compile this directory from the same sources. The
architecture selection comes from `tools/voice_target.py`; application logic is
shared. The private engine has its own symbol namespace so it cannot resolve
against Kokoro's independently pinned GGML version.

The provider accepts at most ten seconds of signed 16 kHz mono PCM and publishes
at most 511 UTF-8 bytes. It executes on the existing speech worker. Whisper and
Kokoro serialize inference through one speech worker while both model contexts
remain resident in the existing bounded native heap. Both engines warm during
boot, so the readiness cover represents the complete conversation path and no
turn pays a model rebuild or first-response synthesis-model load.

Recognition uses Whisper's supported audio-context override instead of encoding
the default 30 seconds for every wake word. The context covers all admitted PCM
plus at least one second, rounded to 64 encoder positions, with a conservative
minimum of 512 positions (10.24 seconds). No captured input is truncated. Decode
strategy, language, model, and single-worker ownership remain unchanged.
The override is part of the pinned upstream
[Whisper streaming example](https://github.com/ggml-org/whisper.cpp/blob/v1.8.2/examples/stream/stream.cpp).

An October 4 paired native ARM64 guest measurement decoded the same exact words
in six fixtures, first and after Kokoro synthesis. Short-fixture warm decoding
fell from 7.54–7.73 seconds to 1.99–2.10 seconds (73% less time); the real 10-second
JFK recording fell from 7.88 to 2.55 seconds (68% less time). Five short fixtures
are synthetic test speech, not evidence of microphone accuracy. A smaller
5.12-second minimum failed the combined-command test and is not used.

To preserve baseline and candidate binaries, immutable fixtures, recognition
results, and paired timings around a native engine change:

```sh
./build-kit run python3 tools/voice-whisper/conversation-benchmark.py --label baseline
# Rebuild the changed native engine through build-kit.
./build-kit run python3 tools/voice-whisper/conversation-benchmark.py --label candidate
# Reuse a generated immutable fixture to alternate context sizes in one engine.
./build-kit run python3 tools/voice-whisper/probe/run.py --alternate-fixture build/voice-whisper/conversation-performance/fixtures/wake.wav --alternate-expected Infinity
```

These are isolated inference measurements, not installed end-to-end latency or
an assertion that arbitrary acoustic conditions have unchanged accuracy.
The resident-context test alternates the real JFK recording and the wake-word
fixture eight times, verifies every decoded word, and requires a stable heap
high-water mark across the final four turns.

Behavioral verification uses real recorded speech inside a freestanding ARM64
guest and checks decoding, repeat recognition after Kokoro synthesis, output
bounds, silence, cancellation, and private heap limits:

```sh
./build-kit run make voice-recognition-test
```

`tools/audio-install-parity.py` verifies that the complete model bytes occur in
the installed kernel and that the ISO payload reconstructs that kernel exactly.
This artifact proof does not replace an installed microphone word-error-rate
test.
