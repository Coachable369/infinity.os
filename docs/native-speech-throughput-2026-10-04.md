# Native speech throughput — October 4, 2026

## Scope and measured result

The same Kokoro model, voice, 1.15 rate, 24 kHz PCM and complete-response playback
contract are retained. No host speech service, generic pthread graph execution,
reduced precision or discarded audio is introduced.

Matched measurements use the production ARM firmware loader and worker adapter
in an isolated eight-vCPU HVF guest, with the preserved original engine as the
baseline. They are **not installed desktop or end-to-end microphone latency**.

| Fixture | Original synthesis | Final synthesis | Audio duration | Reduction |
|---|---:|---:|---:|---:|
| Warm short reply, median of 3 | 1.831 s | 0.909 s | 0.761 s | 50.4% |
| Greeting | 4.824 s | 2.354 s | 2.539 s | 51.2% |
| Longer reply, median of 2 | 16.023 s | 7.836 s | 8.846 s | 51.1% |

Every successful output sample matches the original engine byte for byte.
Greeting and longer-reply real-time factors are 0.927 and 0.886. The short-reply
factor is still 1.194: this is not universal real-time synthesis or an Alexa
latency acceptance claim. Complete-response buffering remains unchanged, so
long responses can still have noticeable preparation latency. Installed audio
quality and responsiveness require the user's ISO retest.

## Implementation boundaries

- Exact two-column matrix tiles reuse data without changing FP32 accumulation
  or reduction order. Unsupported shapes and alignments retain upstream math.
- Validated matrix chunks and convolution output rows use an atomic work queue
  and borrow at most three idle APs. The owner also computes. Busy workers are
  not waited on; every accepted callback is joined before buffer reuse.
- The first online AP is excluded from borrowing. The BSP retains input/services;
  the browser and model jobs retain exclusive mailbox ownership.
- CPU capabilities are checked on the executing CPU once per matrix chunk, not
  inside every tile or in a cross-CPU global cache.
- Elementwise hot loops use O3 without fast-math. Model loading, shared native
  heap ownership and cancellation decisions remain serialized on the admitted
  engine owner. Callbacks do not allocate, use TLS, unwind or dispatch recursively.
- ARM64 and x86-64 use the same math and scheduler code with small ISA adapters.

## Behavioral evidence

Artifacts remain under `build/voice-kokoro/native-latency/`:

- `original-eight/`: preserved pre-change object on the same firmware/CPU count.
- `final/`: ten full-PCM cases; cancellation requires joined helper progress,
  followed by exact recovery output. 7,221 helper callbacks completed.
- `final-comparison.json`: complete-byte equality, bounded heap and timer ratios.
- `final-lifetime/`: two real Whisper decodes alternating with twelve Kokoro
  replies; identical repeated full-sample hashes; peak committed native heap
  1,343,364,576 bytes with no repeat growth. Injected invalid metadata clears all
  720,000 output samples and quarantines subsequent calls.

Native math checks include 392 matrix layout/tile cases and convolution
conversion/guard/partition checks. Host behavioral tests also cover no helpers,
all-busy fallback, browser + speech + one helper, reservation of the first AP,
concurrent model jobs, cancellation and repeated buffer reuse.

Shared x86-64 native compilation also passed. Both the baseline SSE2 guest and
the optional F16C guest passed 205 existing dot/rejection cases plus 392 matrix
cases (597 each). These arithmetic checks do not establish x86 synthesis speed,
installed playback or a new x86 ISO.

Final ARM native object SHA-256:
`ed1c71fa19aaf6f32745d20115a86068724b46a13d210c8ada69b26cc07dc392`.

Relevant build-kit manifests:

- `20261004T070535726508Z-46258.json`: original native baseline.
- `20261004T070943986681Z-46379.json`: voice behavioral gates.
- `20261004T071420957745Z-47837.json`: final PCM and active cancellation.
- `20261004T071524446487Z-47953.json`: mixed-engine lifetime and quarantine.
- `20261004T071620632361Z-47995.json`: original/final comparison.
- `20261004T071643670752Z-48008.json`: shared native x86 build.
- `20261004T071800901477Z-49164.json`: baseline x86 numerical checks.
- `20261004T071834580941Z-49241.json`: F16C numerical checks.

The native runner and reproducible commands are documented in
`tools/voice-kokoro/README.md`. Diagnostic profile builds are explicitly rejected
by release latency comparison. ISO production remains exclusively through
`./build.sh --target aarch64` and its full-bundle/parity gates.
