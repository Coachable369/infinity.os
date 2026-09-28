# Browser and conversational voice — September 28, 2026

## Verified changes

- Installed ARM, booted without installer media: `https://www.google.com/`
  completes and renders its homepage. Entering `www.google.com` through the
  omnibox also completes. The prior native HTTPS failure was response capacity:
  128 KiB was smaller than the engine's existing 1 MiB resource limit.
- HTTPS response storage now has an exclusive, erased-on-release loan rather
  than putting a 1 MiB response on the kernel stack. Behavioral tests check
  512 KiB byte preservation, oversized response rejection, exclusive ownership,
  cancellation, and handle release with Internet access disabled.
- Speech dispatch uses word-aligned native-graph-sized chunks and the existing
  synthesis/playback overlap. Authorized conversational microphone capture
  continues while thinking/speaking. Voice onset stops DMA immediately and is
  retained for recognition; typed replies do not turn on a disabled microphone.
- Optional ARM FEAT_FHM tiles preserve baseline PCM hashes for all seven
  successful native probe cases. Warm short speech: 3.72 to 2.31 seconds;
  11.6-second paragraph: 36.83 to 22.02 seconds. Unsupported CPUs retain the
  baseline implementation; application logic remains shared with x86_64.

## Local-boot follow-up

- Browser startup, resize, and the software renderer now share the same
  area-bounded viewport validation. The previous 2048-pixel dimension checks
  rejected wide windows even when they fit the existing frame storage. The
  native ABI regression now renders and verifies every pixel at 2560 x 1440,
  then completes close/reopen, input, navigation, and tab lifecycle checks.
- Continuous voice reacquires authorized microphone capture after a completed
  capture window or DMA overrun. It discards incomplete input while preserving
  the conversational response/state. Permission denial and device loss do not
  auto-reopen. The behavioral regression covers an overrun during the first
  inference, spoken completion, and successful recognition of a second utterance.
- These checks establish the corrected failure paths, not the exact cause of
  the reported VirtualBox startup error. The full local error panel remains
  needed to distinguish viewport rejection from worker/startup failures.
- The inspected `infinityos-4` VM boots disk before DVD. Updating its attached
  ISO does not replace the already-installed System Generation. Do not erase
  or reprovision that disk just to verify a new ISO; use a separate test disk
  or the supported system-update workflow.

## Acceptance still open

- Google search results (`https://www.google.com/search?q=infinityos`) returned
  HTTP 200 but remained blank/loading in the installed ARM check. The observed
  state was one completed network request, no network/page error, and a running
  engine with frames. The cause is not yet confirmed; homepage success must not
  be presented as general website compatibility.
- Synthesis remains slower than real-time. There is no intentional phrase wait,
  but gap-free playback and installed text-to-first-audio latency are not proven.
- Echo handling subtracts a correlated direct playback path, not arbitrary room
  reverberation. Real speaker/microphone interruption remains unverified.
- Native ARM waveform correctness, host behavioral tests, ISO packaging parity,
  and installed browser tests are separate proof levels. They do not establish
  installed x86_64 browsing or audio acceptance.

## Reproduce focused checks

All commands run from the repository root through the build kit:

```sh
./build-kit run cargo run --manifest-path tools/behavior-harness/Cargo.toml --bin https-service-test
./build-kit run cargo test --manifest-path tools/behavior-harness/Cargo.toml --bin https-service-test browser_network::tests
./build-kit run cargo test --manifest-path tools/behavior-harness/Cargo.toml --bin voice-toggle-test
./build-kit run make voice-output-test
./build-kit run python3 tools/voice-kokoro/probe/run.py
```

For an isolated installed ARM test disk, the existing desktop harness accepts
`--url https://www.google.com/` and checks terminal engine/network state and
bare-hostname omnibox navigation. Use `--reuse-installed` only with the intended
disposable fixture; `--update-kernel` changes that fixture's installed payload.
