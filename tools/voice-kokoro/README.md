# Kokoro native migration: incomplete

`reference.py` builds and runs an isolated **host CPU reference**. It is not
linked into InfinityOS, not a host service used by the OS, and not installed
acceptance. The existing installed default remains Flite/KAL16. Recognition
remains PocketSphinx; Kokoro does not implement speech recognition.

Run:

```sh
python3 tools/voice-kokoro/reference.py
python3 -m unittest discover -s tools/voice-kokoro -p 'test_*.py'
```

The reference pins kokopop revision
`2c6989ac800f624ea984210215e1b76be42eea81` and validates the complete
Kokoro v1.0 GGUF SHA-256 before loading it. Its upstream build fetches pinned
GGML and eSpeak-ng dependencies into the private build tree. It does not
install packages globally. The English `af_heart` voice is inside the model.
The model download is approximately 168 MiB.

Outputs under `build/voice-kokoro/reference`:

- `hello.wav`: real Kokoro 24-kHz, signed 16-bit mono reference speech.
- `evidence.json`: actual PCM duration, peak, clipping count and hash, explicitly
  marking native execution, installed execution, perceptual quality and
  recognition as unverified.

On September 25, the default phrase produced 98,280 WAV frames (4.095 seconds),
peak 12,556, and no saturated samples. This establishes synthesis and container
behavior only, not intelligibility or native execution.

## Native boundary still outstanding

An attempted freestanding AArch64 compile of upstream `src/audio/istft.cpp`
against the existing ARM64 newlib and host libc++ headers failed in libc++
platform configuration (`availability.h` and missing thread API). Host C++
headers/libraries cannot simply be linked into the OS. This is the first
observed compile failure, not proof that later dependencies work.

Next implementation gates remain:

1. Build a compatible freestanding ARM64 C++ runtime and native GGML execution;
   preserve bounded allocation, worker cancellation, and deadlines.
2. Replace model/data filesystem mapping and phonemizer resource access with
   native immutable resources. Do not mount a host filesystem or call a host
   synthesizer. Include redistribution notices and required source material.
3. Run actual Kokoro synthesis in a freestanding guest and compare its audio
   against the reference; then connect its 24-kHz output to the Audio Service.
4. Package the backend, model, voice and phonemizer in the installed System
   Generation. Verify audio on a cold-installed node with ISO detached before
   declaring the replacement complete.

No Kokoro availability flag, fake provider or silent Flite fallback has been
added. The reference workflow is a migration prerequisite, not the requested
completed native replacement.
