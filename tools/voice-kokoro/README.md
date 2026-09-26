# Kokoro native migration: incomplete

`reference.py` builds and runs an isolated **host CPU reference**. It is not
linked into InfinityOS, not a host service used by the OS, and not installed
acceptance. The existing installed default remains Flite/KAL16. Recognition
remains PocketSphinx; Kokoro does not implement speech recognition.

Run:

```sh
./build-kit run python3 tools/voice-kokoro/build.py
./build-kit run python3 -m unittest discover -s tools/voice-kokoro -p 'test_*.py'
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

The default reference phrase is `Hi.`. The reference uses baseline ARMv8-A
instructions on ARM64 and one CPU thread, matching the native prototype. Its
evidence records the input and output metrics. This is not a listening test.

## Native boundary still outstanding

The private freestanding ARM64 libc++, libc++abi, GGML CPU, phonemizer, and
Kokoro objects now compile and link into the native guest probe. The only
remaining external symbols in the intermediate object are the two quad-float
conversion helpers supplied by Rust compiler-builtins during final probe link.
No host runtime library is linked. Native synthesis now passes in a freestanding
ARM64 guest; this is not installed-system acceptance.

The first HVF guest run failed on `ldxr` in `ggml_graph_next_uid`, with
ESR `0x96000035`, before producing PCM. The probe had left the MMU disabled,
which does not give RAM the normal-memory attributes required for exclusive
accesses. A probe-only identity mapping of RAM as normal memory fixes that gate.
A subsequent allocation failure came from sizing the GGML scheduler using
reserved graph capacity rather than populated nodes and leaves. The native CPU
backend now uses populated counts with the existing margin and default floor.
The heap limit and timeout were not increased to make these tests pass.

Six behavioral cases now pass: real synthesis, repeated synthesis, immediate
cancellation, empty-input rejection, cancellation during inference, and synthesis
after cancellation. All successful cases produce identical 30,000-frame, 24-kHz
mono PCM with SHA-256
`1b4fcddf4574e017ba17be8b56972b666319f8bc54c9987e796738880b9d0694`.
Comparison against the independently synthesized reference, accounting for its
trailing-silence trim, gives correlation 0.9996 and relative RMS error 0.02785.
Evidence and WAV files are under `build/voice-kokoro/aarch64`. These measurements
do not establish perceptual quality or installed-system behavior.

Rebuild and native-probe verification through the repository build lock:

```sh
./build-kit run python3 tools/voice-kokoro/build.py
```

The lock spans dependency preparation through guest verification. Stage logs
are written under `build/logs/kokoro-*.log`. Do not bypass a live build lock.

`port.c` exposes only immutable packaged resources, a bounded private heap, and
a single-worker synchronization contract. `mapping.cpp` borrows resource bytes;
`registry.cpp` refuses dynamic backend loading. The guest probe checks actual
PCM and structured return values for repeat synthesis, cancellation and input
bounds. The native guest uses 2 GiB RAM for this initial bounded prototype; its
private heap is capped at 1 GiB and measured committed allocation reaches about
539 MiB. Synthesis currently takes roughly 15 seconds for 1.25 seconds of audio,
incompatible with the production two-second deadline. x86-64 is not implemented
by these scripts. QEMU HVF requires an ARM64 Mac for this test harness, not for
the native synthesis implementation.

Next implementation gates remain:

1. Resolve distribution licensing before release integration. The current
   prototype statically links GPLv3-licensed eSpeak NG. Kokopop's MIT license
   does not remove dependency obligations. Approval of a licensing/packaging
   approach, notices, and required corresponding source remain outstanding.
2. Reduce synthesis latency and integrate the production worker, including
   initialization, symbol isolation, cancellation, deadlines, and memory budget.
   Validate production memory mapping, not only the disposable probe mapping.
3. Connect real 24-kHz output to Audio Service and verify voice off/on recovery.
   Add the x86-64 native backend.
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
