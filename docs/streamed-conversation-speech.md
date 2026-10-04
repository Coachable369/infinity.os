# Buffered speech alongside visible responses

## Current conversation contract — October 4, 2026

The shared controller uses continuous, bounded microphone capture and Whisper
to identify the configured wake phrase. It is **not** a separate acoustic
keyword-spotting model. An addressed utterance can contain both “Infinity” and
the command; a wake-only utterance opens a ten-second command-onset window.
Capture continues while that wake utterance is decoded, preserving one following
command and the remainder of its input chunk. A 400-ms pause ends an utterance.
Continuous speech reaching the ten-second capacity is rejected and erased,
then drained until silence; it is never submitted as a truncated request.

The recognized command is normalized atomically into the widget's ASCII
composer, without replacing typed drafts. Its owner, model, wake setting,
exact contents, and submission deadline remain bound together. One accepted
turn owns model generation and speech playback until completion. Microphone
energy cannot interrupt its response or enqueue self-heard speech. Logout,
explicit stop, permission loss, and model failure still cancel promptly.

Synthesis begins as text arrives, but playback seals a prepared resident batch
to avoid gaps when inference is slower than real time. Every nonzero PCM sample
is preserved at phrase joins. This prioritizes complete output; it does **not**
prove Alexa-level first-audio latency or real-microphone recognition accuracy.
The older September evidence below describes earlier buffering work, not
current end-to-end installed acceptance.

### Verification and timing

`./build-kit run python3 tools/voice-pipeline-test.py` exercises the production
controller, VAD, resampling, worker recovery, and output state machine through
typed seams. The same gate runs inside every `build.sh` ISO workflow. Native
Whisper and Kokoro guest probes separately exercise the linked inference code.
Neither kind of test substitutes for installed microphone and speaker proof.

Content-free `[VOICE TIMING]` records contain ten fixed-width hexadecimal
integers: schema version, utterance sequence, presence bitmask, then monotonic
nanoseconds for endpoint, transcript accepted, model submitted, first text,
first speech queued, first device playback start, and complete drain. Missing
milestones remain absent, not zero-latency successes. First playback is the
successful device-start acknowledgement, not a measurement at the speaker.
The typed `timing_snapshot()` API exposes the same fields. Cancellation keeps
an incomplete record; it cannot masquerade as a fully spoken turn.

### Reference patterns

The bounded pre-roll and lossless handoff follow the source-stream indexing
pattern in [Alexa AudioInputProcessor](https://github.com/alexa/avs-device-sdk/blob/master/CapabilityAgents/AIP/src/AudioInputProcessor.cpp).
Whisper's [stream example](https://github.com/ggml-org/whisper.cpp/blob/v1.8.2/examples/stream/stream.cpp)
provides explicit audio-context sizing and retained overlap. These informed the
native implementation; no host audio service or external conversation framework
is required by InfinityOS.

## Historical implementation — September 28

- Removed the per-phrase microphone reopen and 800-ms echo/interrupt check.
- Buffer cumulative visible assistant text into complete sentences, with a
  160-byte word-aligned upper bound and terminal-fragment flush.
- Begin typed and spoken response synthesis while generation is still running.
  Preserve a single consumed-text cursor to avoid repeating streamed text.
- Double-buffer resident PCM: hardware reads one immutable buffer while one
  background Kokoro job prepares the next. No concurrent Kokoro invocations.
- Keep a bounded queued job when inference temporarily occupies the APs;
  cancellation and the existing synthesis deadline still apply.
- Resume continuous capture only after the full reply drains, discarding the
  existing 200-ms acoustic tail once. No full-duplex echo cancellation or
  voice interruption during output is claimed; explicit stop still cancels.
- Cancel queued speech on logout, cancellation, failed generation, or a new
  turn. Preserve typed output permissions and direction-specific capabilities.

The additional resident buffer costs 6,144,000 bytes. These shared kernel paths
apply to ARM64 and x86_64, live and installed, without separate packaging assets.

## Evidence and limits

`20260928T011833792997Z-16003.json`: six conversation/chunk/resampling tests,
one standalone chunk test, and both native-browser architecture compile checks
passed. The conversation fixture proves partial text waits, the first complete
sentence starts while generation is Running, final text flushes without repeats,
and capture does not restart between response sentences.

`20260928T011925036317Z-16570.json`: five output/resampling tests and both
architecture checks passed after the busy-worker retry change.
The output harness exercises the actual production speech state machine with
deterministic provider/hardware seams: playback buffer immutability while the
next sentence synthesizes, bounded lookahead, immediate ready-buffer handoff,
authority revocation, deadlines, cancellation, and busy-worker retry.

This removes intentional gaps; it does not guarantee synthesis faster than
real time. Visible text can lead the voice by synthesis latency. No updated ISO
or installed-VM listening test was performed in this pass.
