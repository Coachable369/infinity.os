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
