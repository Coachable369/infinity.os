# Kokoro shared native integration

`reference.py` builds and runs an isolated **host CPU reference**. It is not
linked into InfinityOS, not a host service used by the OS, and not installed
acceptance. ARM64 and x86-64 live and installed kernels link the private native
Kokoro backend, including architecture-selected streamed installer builds.
The x86-64 integration and its still-open installed acceptance gates are tracked
in `docs/x86-native-speech-status.md`. Recognition remains PocketSphinx; Kokoro does not implement speech
recognition. Installed playback and perceptual quality remain separate gates.

The production worker uses 24 kHz PCM, band-limited device-rate conversion,
single-owner cancellation, a bounded 90-second asynchronous synthesis deadline,
and a 130-second output lease covering synthesis plus finite playback. This
provider-specific budget replaces the Flite-only two-second deadline; it is not
a speed improvement. The UI thread never performs inference. Invalid/empty
provider results cannot reach DMA. The existing 1 GiB private heap is unchanged.
Long replies are synthesized in at most 44-character, word-boundary phrases;
the bounded PCM is concatenated before playback. A single word exceeding this
bound is rejected, not cut into unrelated pronunciations. This avoids a proven
paragraph-sized graph allocation failure (a 259,560,528-byte request with
952,663,520 bytes already committed). A 158-character paragraph now produces
278,400 frames (11.6 seconds) in about 31 seconds using 904,048,096 committed bytes.

`build.py --build-only` prepares and links dependencies without running host or
guest synthesis. `install-parity.py` checks the actual loadable model, phonemizer,
notices, and constructor data in both installed and live ELF images. The ISO
packager additionally verifies byte-identical installed kernel payload shards.

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

## Native boundary and verification

The private freestanding ARM64 libc++, libc++abi, GGML CPU, phonemizer, and
Kokoro objects now compile and link into the native guest probe. The only
remaining external symbols in the intermediate object are the two quad-float
conversion helpers supplied by Rust compiler-builtins during final probe link.
No host runtime library is linked. Native synthesis now passes in a freestanding
ARM64 guest; this is not installed-system acceptance.

The probe links `private-native.o`, with all library symbols prefixed except the
two compiler-builtins above. Its private constructor table is initialized once
by the single speech worker; it does not depend on the kernel constructor list.
Full dependency notices are embedded as an immutable resource and compared
byte-for-byte with the build inputs through the running guest API.

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

For operation-level native timing, use
`INFINITY_KOKORO_PROFILE=1 ./build-kit run python3 tools/voice-kokoro/build.py`.
The guest evidence includes numeric GGML operation IDs, call counts, and elapsed
seconds accumulated across the probe cases. This instrumentation is opt-in and
does not record text or tensor contents. Rebuild without that environment
variable before measuring release latency. The source-signature cache and fresh
archive construction keep instrumented objects out of the normal engine.

`port.c` exposes only immutable packaged resources, a bounded private heap, and
a single-worker synchronization contract. `mapping.cpp` borrows resource bytes;
`registry.cpp` refuses dynamic backend loading. The guest probe checks actual
PCM and structured return values for repeat synthesis, cancellation and input
bounds. The native guest uses 2 GiB RAM for this initial bounded prototype; its
private heap is capped at 1 GiB and measured committed allocation reaches about
539 MiB. The original prototype took roughly 15 seconds for 1.25 seconds of
audio. September 26 measurements isolated matrix multiplication as the dominant
cost: checked alignment specialization reduced a same-session 12.3-second warm
run to 4.3 seconds, and four-row activation reuse reduced it further to about
3.3 seconds, preserving the full PCM hash. These are `Hi.` probe measurements,
not full-response or installed-desktop latency claims. x86-64 uses the same
native build with architecture-selected workers and SIMD. Baseline SSE2 and
feature-detected F16C tiles each pass 205 numerical/guard cases. The separately
bounded x86 correctness probe completes 30,000 PCM samples and matches the
independent reference at 0.998685 correlation. It takes 795.68 seconds under
software emulation and does not pass the unchanged 90-second production
deadline. Use `build.py --target x86_64 --x86-probe-mode correctness` through
the build kit for this diagnostic; default verification still enforces the
production deadline. QEMU HVF requires an ARM64 Mac for the ARM test harness, not for
the native synthesis implementation.

The matrix changes preserve upstream FP32 accumulation and reduction order.
They use wider reads only after checking alignment and retain upstream handling
for unsupported types, lengths, and boundary rows. The guest additionally checks
8,448 aligned/fallback dot cases and 165 four-row tile/rejection cases against
the single-row implementation. A separate experiment with the upstream tinyBLAS
backend provided no measurable speedup and was removed.

Remaining acceptance gates:

1. Complete distribution materials before release integration. GPLv3-compatible
   distribution was explicitly approved on September 25, 2026. The current
   prototype statically links GPLv3-licensed eSpeak NG. Kokopop's MIT license
   does not remove dependency obligations. Dependency notices are embedded.
   `./build-kit run python3 tools/voice-kokoro/source-bundle.py` preserves the
   dependency sources, current tracked OS integration/build source and artwork
   under `builds/voice-kokoro/infinityos-kokoro-sources.tar.gz`. Model attribution
   and the full Apache 2.0 text are embedded with dependency notices. This source
   archive is release preparation, not a legal certification of whole-OS source
   completeness. Approval alone is not packaging verification.
2. Verify production worker execution and actual audio on an installed node,
   including cancellation, UI responsiveness and voice off/on recovery.
3. Complete native x86-64 latency and hardware playback acceptance for the linked backend.
4. Verify audio on a cold-installed node with ISO detached before declaring
   installed acceptance complete.

There is no silent Flite fallback on ARM64. A synthesis error is surfaced as a
failed native speech job; it does not substitute a different voice.

## Conversational phrase playback and microphone interruption

The conversation controller now submits one phrase of at most 44 characters,
preferring a sentence boundary. It does not wait for all 160 characters to be
synthesized before beginning playback. This is phrase-wise playback after LLM
generation, not simultaneous synthesis/playback or token-streamed generation.

After each completed phrase the controller opens fresh authorized capture,
discards 200 ms of samples for the speaker's acoustic tail, then checks 600 ms
of actual microphone samples before allowing another phrase. Confirmed VAD
speech discards the remaining assistant reply and preserves the utterance for
recognition. Missing microphone samples do not count as silence; a three-second
capture watchdog stops the reply. Logout, mute and cancellation close capture.
The final phrase also gets the acoustic-tail guard before normal listening.

This is a half-duplex boundary check, not acoustic echo cancellation: speech
during playback or the 200 ms tail guard is not captured. Long room reverberation
may still trigger VAD. Installed-device microphone/speaker testing is required
before claiming acoustic interruption quality. The behavioral controller test
uses real resampling/VAD with deterministic audio and playback seams.

Unfinished upstream modifications are preserved as reviewable patches in
`third_party/patches/voice-kokoro`. Generated clones, dependency checkouts and
objects remain disposable under `build/` and may be removed before every full
build.
