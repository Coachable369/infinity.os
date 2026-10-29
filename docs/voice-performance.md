# AI audio input → LLM → speech performance trace

The native ARM64 and x86-64 live and installed kernels emit content-free,
versioned numeric telemetry over their existing serial diagnostics channel.
No prompt, transcript, response text, identity, or PCM is included in these
records. Other existing serial diagnostics may contain application data: treat
the complete serial log as private.

## Capture and inspect

For VirtualBox, shut down the test VM yourself, then run:

```sh
sh tools/enable-voice-metrics.sh 'infinityos-4'
```

This enables a serial log in `builds/voice-logs/<VM UUID>/serial-<UTC time>.log`,
outside disposable `build/`. It refuses to alter a running VM and does not
launch it. Boot the updated ISO/installation, speak a few requests, then run:

```sh
python3 tools/voice-performance-report.py /absolute/path/to/serial-log
```

The JSON report can be read by Codex through the filesystem. Existing serial
logs also work; old kernels will not contain the new records. A clean build
does not update an installed VM. The shell helper only configures VM hardware;
it is not a host dependency of the OS voice implementation.

## Measurements

- Capture onset through utterance endpoint (includes the endpoint silence tail).
- Endpoint to recognition result observed; transcription admission wait and
  actual recognizer calls are separate worker spans.
- Recognition to accepted prompt; routing/submission to the model or OS action.
- Intent routing, personal-memory update, memory-context selection, and prompt
  construction/tokenization have separate work spans (including early returns).
- First generated text, first presented reply frame, and generation completion.
- Inference counters: time to first token, BSP prefill/decode work, maximum
  service slice/pump, reused/input/output token counts. Model-load time is
  resident-model context, **not** charged again to every turn.
- Phrase buffering, speech worker admission, native synthesis, PCM conversion,
  and first successful hardware playback start.
- Driver-observed playback intervals, inter-batch gaps, final drain, and
  microphone rearm. Driver observation is not an acoustic loopback measurement.
- Owner-thread service and presentation call count, cumulative wall time and
  worst call. Service work includes all services advanced in that pass and CPU
  work to ingest mic buffers; it excludes waiting while the user speaks or the
  audio device plays. It can overlap asynchronous inference/synthesis.

**Do not add overlapping intervals into a total.** BSP work is not summed CPU
time across all worker cores. Queue/admission wait includes native-engine busy
retries; repeated wait spans are cumulative, not additive. A missing milestone
is `null`, not zero or success. Failed synthesis retains its numeric error code.
Hidden/minimized widgets do not satisfy the response-presentation checkpoint.

## Wire schema and overhead

All fields are hexadecimal integers, times are monotonic nanoseconds. Each
record is emitted in one serialized write; no allocation occurs in kernel
instrumentation. Records are bounded to 24 fields. Stage snapshots occur on
milestone changes and at most once per five seconds otherwise; no token or
frame-by-frame log is emitted. Serial output itself has nonzero overhead.

- `VOICE TIMING`: legacy version-1 seven-stage record retained for active tools.
- `VOICE PHASES`: version, turn, presence mask, eleven timestamps (endpoint,
  transcript, submitted, first text, speech queued, first audio, drained,
  capture started, recognition observed, first display, generation done),
  rearm timestamp/presence, service count/total/max, render count/total/max.
- `VOICE LINKS`: version, turn, recognition job timestamp, chat turn,
  speech response timestamp, controller state. IDs are boot-local, not user IDs.
- `VOICE LLM`: version, voice turn, chat turn, load/first-token/first-visible/
  response/prefill/decode/max-slice/max-pump nanoseconds, reused/prefill/output
  tokens. These counters do not assert a model was used for a fast OS action.
- `VOICE SPAN`: version, phase, job, part, start, end, units, error/status,
  valid-clock flag. Phases 1–8 are recognition wait, recognition work, speech
  wait, synthesis, PCM conversion, observed playback, observed gap, warmup.
  Phases 9–12 are intent routing, personal-memory update, memory-context
  selection, and prompt construction/tokenization; their job is the voice turn
  ID and units are zero. Scope completion alone does not assert action success.
  Units are input samples for recognition, text bytes for speech admission,
  output frames for synthesis (24 kHz), interleaved samples for PCM conversion,
  zero for playback/gaps, and allocated bytes for engine warmup.

Logs can grow with usage. The helper creates a new timestamped file when run;
archive or remove old host logs explicitly. No automatic disk retention policy
or upload is enabled. Use separate logs per boot because turn IDs restart.

Behavioral verification: `./build-kit run python3 tools/voice-pipeline-test.py`.
Real microphone, speaker, and installed-system performance remain device tests.
