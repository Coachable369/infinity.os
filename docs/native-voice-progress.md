# Native voice implementation status

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
2. Real native TTS and a general incremental playback stream (not diagnostic tone).
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

Flite v2.2 (upstream commit `e9e2e37c329dbe98bfeb27a1828ef9a71fa84f88`)
was fetched into the ignored build cache for native TTS feasibility inspection.
It is not linked, registered, installed, or a verified speech backend. Its C
runtime dependencies still need a bounded native integration; host synthesis
would not satisfy the requirement.

No host speech command, cloud API or fabricated transcript substitutes for these
requirements. The VAD change alone does not complete the voice milestone.
