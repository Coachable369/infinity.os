# Buffered speech alongside visible responses

## Implementation

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
