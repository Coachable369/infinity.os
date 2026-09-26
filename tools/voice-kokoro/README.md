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

The private freestanding ARM64 libc++, libc++abi, GGML CPU, phonemizer, and
Kokoro objects now compile and link into the native guest probe. The only
remaining external symbols in the intermediate object are the two quad-float
conversion helpers supplied by Rust compiler-builtins during final probe link.
No host runtime library is linked. This is build evidence, not synthesis or
installed-system acceptance.

The first HVF guest run failed on `ldxr` in `ggml_graph_next_uid`, with
ESR `0x96000035`, before producing PCM. The probe had left the MMU disabled,
which does not give RAM the normal-memory attributes required for exclusive
accesses. A probe-only identity mapping of RAM as normal memory has been added.
Its retry is **unverified**: the shared `build/` tree was removed while Cargo
was compiling, causing a missing temporary-directory error. Do not report the
mapping change as a successful fix or enable the provider from this evidence.

Rebuild sequence (requires exclusive use of the build tree; no concurrent clean):

```sh
python3 tools/voice-pocketsphinx/build.py
python3 tools/voice-kokoro/reference.py
python3 tools/voice-kokoro/prepare-native.py
python3 tools/voice-kokoro/native-build.py
python3 tools/voice-kokoro/link-native.py
python3 tools/voice-kokoro/probe/run.py
```

`port.c` exposes only immutable packaged resources, a bounded private heap, and
a single-worker synchronization contract. `mapping.cpp` borrows resource bytes;
`registry.cpp` refuses dynamic backend loading. The guest probe checks actual
PCM and structured return values for repeat synthesis, cancellation and input
bounds. These new runtime paths still require successful execution, cleanup
and cancellation validation. The native guest needs 2 GiB RAM for this initial
bounded prototype; its private heap is capped at 1 GiB, not a measured product
memory requirement. x86-64 is not implemented by these scripts.

Next implementation gates remain:

1. Pass native ARM64 GGML execution; verify bounded allocation, cleanup,
   worker cancellation and deadlines. Validate the production memory mapping,
   not only the disposable probe's mapping.
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

Unfinished upstream modifications are preserved as reviewable patches in
`third_party/patches/voice-kokoro`. Generated clones, dependency checkouts and
objects remain disposable under `build/` and may be removed before every full
build.
