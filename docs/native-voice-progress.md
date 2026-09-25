# Native voice implementation status

## Current acceptance status — September 25, native recognizer integration

**In progress, not end-to-end accepted.** The sections below are chronological;
this section supersedes their earlier statements that STT is absent.

Media built September 25 at 02:38 CDT:
`builds/InfinityOS-aarch64.iso`, 8,590,336,000 bytes,
SHA-256 `15d30a9689d971bde58b210410426cfbffaef3b48cb739c3361462bdb69e5682`.
The build's binary parity check passed for the 249,854,016-byte installed
kernel, 24,576-byte loader, and 33,722,129 bytes of checked acoustic/LM resources.
Hermes and Ministral payload parity also passed. Other architecture/test ISOs
were not refreshed by this build.

Implemented for AArch64:

- PocketSphinx 5.1.1 at `511126b492dcb267cf30d49d631946d7b61a9530`, with its
  bundled English acoustic model, dictionary, and language model linked as
  immutable native resources. Newlib supplies a private, symbol-prefixed C
  runtime; there is no Linux, Python, cloud, or host recognizer at runtime.
- One cancellable AP recognition job, 16 kHz mono PCM, maximum ten-second
  utterance, bounded 192 MiB decoder heap and 512-byte transcript. Private
  allocations and PCM are erased on release; a caught decoder fatal error
  quarantines that provider until reboot rather than reusing corrupted state.
- `SpeechRecognize` typed IOP validation, caller/capability checks, deadline,
  output ownership, revocation, and one-time result delivery. Native audio
  capture renews its existing stream with fresh short-lived authority instead
  of restarting DMA. The two-second HDA ring replaces the overrun-prone 100 ms
  capture ring; playback remains independent.
- `voice listen` / `voice conversation start` connect capture, resampling, VAD,
  native STT, the selected local chat model (Hermes by default), and native
  Flite reply chunks. `voice stop`, `voice status`, `voice devices`, and
  `voice say` expose the same underlying state. Explicit listening during a
  reply requests cancellation before restarting capture after worker release.
- A desktop chat-header listening control and bounded audio-reactive sine
  waveform; chat damage now incorporates voice state so a foreground app
  does not leave a stale listening label. Closing/hiding chat or locking the
  owning session stops capture. Reboot does not resume microphone access.

Evidence collected:

- A blank 16 GiB private VirtualBox disk completed the actual GUI installation
  from the September 25 ISO. With the ISO detached, it cold-booted into first-run
  configuration. Read-only extraction validated the disk structures and all
  eleven kernel references; its 249,854,016-byte kernel exactly matches the
  packaged artifact, SHA-256
  `e1f6df185aa8f16d0873eec6baff5dc03b395730b85d7eeab1ccbda3a2a42a2a`.
  The same verifier rejects a different valid ELF (exit 1) and conflicting
  verification/write flags (exit 2). This is installation/boot evidence, not
  completed first-run setup or voice-conversation evidence.
- Six freestanding AArch64 decoder cases pass with QEMU HVF and TCG: repeated
  real recorded speech, mid-utterance cancellation, output bounds, silence,
  and pre-start cancellation. These are native bare-metal probes, **not**
  installed desktop conversation proof.
- HVF recorded-fixture runs took 0.8364 s and 0.8057 s including decoder/model
  setup. Committed heap was 108,351,616 bytes, with 1,316 retained bytes stable
  across requests. This is committed decoder heap, not total OS peak RAM.
- The fixture says “go forward ten meters”; both the native guest and the
  identical upstream host reference decode “go forward ten years”. That is a
  known accuracy failure, not a successful exact-transcript acceptance test.
- The installed private VirtualBox disk boots without an ISO, opens continuous
  HDA capture, survives the former short-ring overrun interval, and displays
  LISTEN with the waveform. Console status remains accessible. Longer-run
  VirtualBox virtual-time lag was observed, so no installed latency or desktop
  responsiveness acceptance is claimed from that run.
- The acoustic speaker-fixture attempt produced no transcript in that session.
  VirtualBox microphone permission was already enabled. Subsequently even
  debugger/poweroff calls stalled; the private test instance was terminated.
  This does not establish the root cause or prove a microphone hardware fault.
- Behavioral host tests cover PCM conversion, VAD, stream capability renewal,
  speech IOP validation, waveform bounds, and existing AI security/chat behavior.
  AArch64 installed kernel links with both speech backends.

Remaining acceptance / limitations:

1. A live microphone utterance producing the expected transcript, a real
   Hermes response, and an audible synthesized reply in the installed desktop
   has **not yet been demonstrated**. Earlier audible tone confirmation is not
   speech confirmation.
2. Cold installation and no-ISO first-run boot pass. Completing that fresh
   system's account setup, installed voice retest, GUI microphone toggle
   reliability, and whole-turn cancellation/restart still need verification.
3. Current conversation is turn-taking: capture pauses during recognition,
   thinking, and playback and resumes between replies. There is no acoustic
   echo cancellation, spoken barge-in, or wake word. Speech begins after the
   completed chat reply, not on token streaming. Explicit restart is supported.
4. English-only recognizer, ASCII/compact diphone voice, ten-second utterances;
   recognition quality needs live testing. Decoder setup occurs per utterance;
   cancellation checkpoints begin after model initialization.
5. Full installed stage latencies, Hermes throughput regression, total peak
   RAM/CPU, and a spoken capability-gated OS action remain unmeasured/unproven.
6. Native STT/controller integration is AArch64-only; do not advertise the
   x86_64 ISO as having the same voice-conversation capability.

Reproduction: `make voice-recognition-test`, `make voice-indicator-test`,
`make audio-test voice-vad-test voice-pcm-test`, `sh tools/ai-test.sh`.
Native binary-result reports live under `build/voice-pocketsphinx-arm/`.
`tools/audio-install-parity.py` checks actual packaged kernel/model/loader bytes,
not source strings or human-readable diagnostics. Packaging parity alone does
not establish installed conversation acceptance.

## Verified audio prerequisite

See `native-audio-substrate.md` and `evidence/audio-virtualbox-installed.json`.
VirtualBox playback was audible to the user. Installed microphone capture,
cancellation, and idle cold-reboot state were exercised without an attached ISO.
The September 24, 23:58 CDT ARM64 ISO contains those audio fixes.

## September 25: speech segmentation and service boundary

`kernel/runtime/ai/voice_vad.rs` provides allocation-free 16 kHz mono energy VAD:

- 20 ms frames; three consecutive voiced frames trigger speech.
- 200 ms preroll, 600 ms end silence, 100 ms retained tail.
- Ten-second absolute sample limit; silence and overlong speech are distinct failures.
- Fixed utterance capacity: 160,000 signed 16-bit samples (320,000 bytes).
- Completion exposes sample bounds, not a transcript. Cancellation/reset erase PCM.
- Chunk boundaries do not change detection or trimming.

The existing VoiceService accepts at most 100 ms of PCM per call and validates
session identity, caller ownership, format, deadline, and current microphone
capability. It transitions Listening → Recognizing only on completed speech,
rejects concurrent session replacement and rejects cancellation by another owner.

Evidence:

- `make voice-vad-test`: executable PCM tests cover silence, clipped integer
  samples, transient rejection, sample bounds, chunk invariance, capacity,
  cancellation and reset. A private-storage unit test checks actual PCM erasure.
- `sh tools/ai-test.sh`: exercises wrong-owner/stale-session/oversized/wrong-rate
  rejection, real PCM-driven state transition, active-session exclusion and
  capability revocation with the actual capability manager.

These are host behavioral tests of native Rust logic, **not an installed voice
conversation demonstration**. Energy VAD can mistake background noise or music
for speech; it does not recognize words. Threshold calibration, proper capture
rate conversion and driver-to-voice-service polling are still required.

## Remaining acceptance (not implemented)

1. Real native offline STT model/runtime, bounded asynchronous requests and text
   results; the current speech provider honestly returns ProviderUnavailable.
2. Installed-desktop validation of native TTS and incremental playback (implementation below).
3. Capture/resampling/VAD/STT → existing AI service → safe typed actions → TTS
   integration, shared Console/GUI session state, barge-in and resource cleanup.
4. Audio-reactive chat-header waveform and all requested voice Console commands.
5. End-to-end latency, Hermes throughput regression checks and installed-system
   proof, including fresh-install payload parity for speech models/providers.

## September 25: native capture-format conversion

`kernel/runtime/ai/voice_pcm.rs` adds bounded 44.1/48 kHz mono → 16 kHz
conversion, with exact 16 kHz passthrough. A 96-tap, 160-phase normalized
Blackman-windowed sinc filter suppresses aliasing before decimation. Filter
coefficients are prepared only when the input rate changes, not per audio chunk.
The converter has fixed storage, explicit input/output counts for backpressure,
and erases retained sample history on reset. The filter delay is approximately
one millisecond; it does not silently flush fabricated tail samples.

`make voice-pcm-test` exercises the compiled native implementation and asserts:

- One second at either HDA rate produces exactly 16,000 output samples.
- Input chunks from one sample to 4,800 samples and output capacities down to one
  sample produce identical PCM, without loss at chunk boundaries.
- 1 kHz gain error is below 1%; fractional-phase error is below 3 PCM units RMS.
- A 12 kHz source is attenuated by more than 60 dB relative to the 1 kHz fixture.
- Full-scale DC, silence, passthrough, empty output, invalid rate and privacy reset
  behave as specified.
- Capture-rate PCM passed through the real converter and utterance segmenter
  produces a completed, bounded speech segment; cancellation removes access.

`make voice-vad-test`, `sh tools/ai-test.sh`, and the AArch64 installed-kernel
build also pass. These remain host tests and a cross-build, **not installed
STT/TTS or voice-conversation acceptance**. The converter is not yet connected
to live driver polling. No refreshed ISO is claimed for this increment.

## September 25: real native speech synthesis and streamed playback

Flite v2.2 (pinned commit `e9e2e37c329dbe98bfeb27a1828ef9a71fa84f88`) now
cross-compiles as a freestanding compact English diphone voice. The native port
excludes upstream audio, files, sockets, and dynamic model loaders. Its complete
license is retained in `tools/voice-flite/COPYING` and linked into the kernel.
It is a compact intelligible voice, not a neural/naturalness claim.

The C-only failure boundary uses an 8 MiB fixed arena, checks cancellation and
deadline at allocator/free checkpoints, and erases private storage after every
request. Input is currently limited to 160 printable ASCII characters; output
is bounded to 30 seconds of 8 kHz mono PCM. There is no host synthesis dependency.

`kernel/runtime/ai/voice_output.rs` queues synthesis on one available AP using
the existing speech-provider interface. Busy/missing workers fail explicitly;
there is no synchronous UI fallback. Matrix jobs exclude the reserved worker.
`voice say TEXT`, `voice stop`, and `voice status` expose output testing through
authenticated Console sessions. These commands do not enable microphone capture.

`AudioPlaybackStart` uses typed IOP and AudioOutput authority with a bounded
deadline. An initial 50 ms refill implementation exposed a real installed-VM
underrun: desktop polling sometimes takes longer than a DMA half. The bounded
reply path now prepares negotiated-rate PCM on the AP into a resident 6,144,000
byte maximum buffer, with a silence tail. HDA reads that directly, independently
of UI frame timing. The BSP observes hardware progress to finish and releases
the buffer only after DMA stops. Cancellation/revocation stop playback. The
short-ring refill API remains available, but is not the desktop reply path.
Both live and installed kernel link rules include the same voice archive.

Evidence (not full conversation acceptance):

- Host C tests execute real synthesis, deterministic repeated requests, small
  arena/output failures, cancellation, and actual private-arena erasure.
- Two Rust PCM tests verify chunk invariance, extreme samples, stereo equality,
  tail silence, and rejection without output mutation.
- Audio behavioral harness verifies output-only authority, 35-second playback
  deadline bounds, expiry, and revocation.
- Hermes `concurrent_rows_and_cancellation` test passes while a real worker is
  held by a background job; remaining workers still compute quantized matrices.
- Freestanding AArch64 QEMU **TCG** probe synthesizes 37,507 frames (4.688375 s)
  in 74.487 ms, peak arena 445,360 bytes, and plays native HDA PCM while deliberately
  polling only every 120 ms (39 polls), beyond the former 50 ms refill deadline.
  The WAV sink contains real stereo speech PCM. This is neither Linux nor a
  host synthesis service, but also **not an installed InfinityOS desktop test**.
- QEMU/HVF probe encounters a QEMU assertion in `hvf_handle_exception`; it is
  not reported as passing or used as a performance measurement.
- AArch64 installed-kernel cross-build passes. No refreshed ISO or complete
  fresh-install voice acceptance is claimed for this increment.
- Initial installed desktop synthesis of `Hello from Infinity` generated 12,992
  samples in 1 ms with 160,176 arena bytes, but that first playback attempt
  underrran. It is not counted as successful installed playback evidence.
- The corrected resident-buffer path additionally uses **two** descriptors:
  VirtualBox rejects LVI=0 (one descriptor), although QEMU accepts it. The hardware
  probe now asserts LVI=1. Installed VirtualBox retest reached Complete (state 7),
  error 0, 12,992 speech frames, 2 ms synthesis including output preparation,
  160,176 peak arena bytes. HDA advanced to byte 286,728 at 44.1 kHz stereo and
  cleared RUN. A second request followed by `voice stop` reached Cancelled (state
  6), error 0. These were live UI/state inspections, not prose-matching tests.
  The private installed disk booted without an ISO. Human confirmation of speech
  audibility is pending; the earlier confirmed 440 Hz tone is separate evidence.

Reproduce with `make voice-synthesis-test`, `make audio-test`, and
`make voice-synthesis-hardware-test`. Native metrics and WAV artifacts are in
`build/voice-flite/aarch64/`.

The user's updated acceptance is **always-listening interactive desktop voice**,
not push-to-talk-only. Real offline STT, continuous capture integration, echo
suppression/barge-in, Hermes conversation orchestration, waveform widget states,
and fresh-installed end-to-end proof remain unfinished. Listening must remain
visible and stoppable, and must end on session lock/revocation.

No host speech command, cloud API or fabricated transcript substitutes for these
requirements. The VAD change alone does not complete the voice milestone.
