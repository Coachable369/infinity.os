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

No host speech command, cloud API or fabricated transcript substitutes for these
requirements. The VAD change alone does not complete the voice milestone.
