# Kokoro shared native integration

`reference.py` builds and runs an isolated **host CPU reference**. It is not
linked into InfinityOS, not a host service used by the OS, and not installed
acceptance. ARM64 and x86-64 live and installed kernels link the private native
Kokoro backend, including architecture-selected streamed installer builds.
The x86-64 integration and its still-open installed acceptance gates are tracked
in `docs/x86-native-speech-status.md`. Recognition uses the privately linked,
CPU-only Whisper engine and pinned English model; Kokoro remains the synthesis
engine. Installed playback and perceptual quality remain separate gates.

The production worker uses 24 kHz PCM, band-limited device-rate conversion,
single-owner cancellation, a bounded 90-second asynchronous synthesis deadline,
and a 130-second output lease covering synthesis plus finite playback. This
provider-specific budget replaces the Flite-only two-second deadline; it is not
a speed improvement. The UI thread never performs inference. Invalid/empty
provider results cannot reach DMA. The private speech heap remains fixed and bounded.
Long replies are synthesized in at most 80-character, word-boundary phrases;
duplicated model-edge silence is bounded before the PCM is crossfaded and
concatenated for playback. A single word exceeding this
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
private heap is capped at 1.5 GiB so the measured 752 MiB Whisper context and
539 MiB Kokoro peak can remain resident together without per-turn model reloads.
The original prototype took roughly 15 seconds for 1.25 seconds of
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
ARM CPUs exposing FEAT_FHM additionally use checked 16-byte-aligned widening
half multiply-accumulate tiles; other CPUs and alignments retain the existing
fallback. September 28 ARM HVF probes preserved the full PCM hashes for all
seven successful cases. At the 1.15 conversational rate, warm `Hi.` synthesis
measures 2.10 seconds and the paragraph case measures 18.57 seconds. The
paragraph previously took 20.47 seconds at the 1.10 rate. These are native probe measurements,
not installed end-to-end response latency or a real-time throughput pass.
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

The conversation controller submits word-aligned spans from cumulative visible
LLM output. The native engine retains up to 80 characters per coherent inference
phrase, preferring word boundaries, and removes duplicated edge silence. This matches
one native graph rather than waiting for a 160-character multi-graph batch.
The first word-aligned span is released after roughly 24 visible characters so
synthesis overlaps ongoing LLM generation. Completed spans enter one continuous
resident DMA response, without scheduler-timed stops between spans. There is no
intentional inter-phrase wait.

With conversation input explicitly enabled, authorized microphone capture stays
open during thinking and speaking. Three voiced 20-ms frames interrupt playback
and cancel the old response, preserving the utterance onset for recognition.
Typed replies do not silently enable a disabled microphone. Logout, lock, mute,
and cancellation close capture and erase its private buffers.

A bounded playback-reference correlator subtracts delayed direct-path speaker
echo before VAD, retaining independent near-end speech. Recent reference samples
remain available for a 200-ms acoustic tail instead of discarding microphone
input. This is not a full room-adaptive acoustic echo canceller: reverberant
speakers and real microphone interruption remain installed-device acceptance
gates. Behavioral tests cover delayed scaled echo, mixed human speech, onset
preservation, immediate DMA cancellation, and voice off/on lifecycle handling.

Unfinished upstream modifications are preserved as reviewable patches in
`third_party/patches/voice-kokoro`. Generated clones, dependency checkouts and
objects remain disposable under `build/` and may be removed before every full
build.
