# Speech clarity follow-up

Status: targeted corrections verified in harnesses; human intelligibility and
installed microphone accuracy are not yet accepted.

## Changes

- Replaced the compiled Flite `kal` 8 kHz diphone data with its pinned `kal16`
  16 kHz voice. Both live and installed kernels link the same archive.
- Kept speech at 16 kHz through native PCM staging, duration accounting and
  conversion to 44.1/48 kHz stereo HDA. Doubled bounded mono storage to preserve
  the same 30-second limit; no host speech service or network dependency.
- Waiting silence now rolls out of the utterance buffer while preserving
  pre-roll and incomplete analysis frames. A late start no longer consumes most
  of the ten-second speech budget.
- Reaching the speech limit submits the retained bounded segment instead of
  discarding all of it. This is still a bounded, turn-taking interface, not
  unlimited continuous dictation.

## Evidence

`make voice-vad-test voice-output-test voice-synthesis-test` passed. Segmentation
tests include a long silent wait followed by a complete phrase, exact retained
speech samples, and preservation at the maximum buffer size. Output tests cover
cancellation/deadlines, PCM conversion, deterministic synthesis and cleanup.

`make voice-synthesis-hardware-test` passed using freestanding ARM64 QEMU and
native HDA: 74,266 synthesized mono frames at 16 kHz, 511,104-byte arena peak,
resident DMA with 120 ms polling. This verifies conversion/playback behavior,
not perceptual quality or VirtualBox microphone capture.

Listen to `build/voice-flite/host/native-voice.wav` for the new reference voice.
It remains a small diphone synthesizer, not a natural neural voice. Raising its
bandwidth does not establish that all users will find it intelligible.

PocketSphinx's acoustic/language models are unchanged. Word substitutions remain
an open limitation; these segmentation fixes do not establish a reduced word
error rate. The previously observed VirtualBox capture stall remains unresolved.
Do not present this change as complete end-to-end voice acceptance.
