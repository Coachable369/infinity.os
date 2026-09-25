# Kokoro migration and desktop chat

## Implemented

- Chat baseline spacing now follows the actual scale-one font metrics rather
  than the desktop geometry scale. At geometry scale two, the previous code
  advanced 58 pixels for a 28-pixel glyph cell. The new advance is 30 pixels.
- Native response publication requests bottom-following before layout measures
  the new message. Scrolling history manually still works between updates.
- The audio converter can now accept 24 kHz mono PCM without changing pitch or
  duration. Existing 16 kHz Flite callers retain their behavior.

Behavioral verification: `make ai-test`; three tests from
`rustc --test kernel/drivers/speech_pcm.rs`. The tests cover response-following
after history scrolling, extent growth, baseline metrics, 24 kHz sample
positions, stereo duplication, chunk continuity, silence padding, and invalid
format rejection. These are not installed desktop screenshot or listening tests.

## Kokoro is not yet the installed voice

The current default remains Flite/KAL16. No Kokoro availability claim, model-only
registry entry, host speech service, or renamed fallback has been added.

Upstream Kokoro requires an ALBERT text encoder, recurrent prosody/duration
prediction, convolutional synthesis, and iSTFT waveform generation, plus
compatible text normalization/phonemization and voice-style tensors. Its
reference implementation uses PyTorch:
https://github.com/hexgrad/kokoro/blob/main/kokoro/model.py

A lightweight alternative audited was kokopop revision
`2c6989ac800f624ea984210215e1b76be42eea81`:
https://github.com/tterrasson/kokopop

That implementation uses C++17, GGML, eSpeak-ng, file mapping/stdio, and host
threads. It is not a freestanding ARM64 drop-in. InfinityOS's existing native
C++ toolchain build targets x86_64; the ARM64 speech path currently links bounded
C libraries into the kernel. The host source audit is in
`build/kokopop-port-audit`; it is not a shipped dependency.

## Remaining implementation acceptance

1. Port a narrowly scoped CPU synthesis backend and phonemizer to the ARM64
   native runtime; retain capability checks, deadlines, cancellation and worker
   isolation. Replace host file/thread dependencies with native facilities.
2. Pin and validate Kokoro-82M weights, one English voice, phonemizer data and
   all required redistribution notices in the installed System Generation.
3. Connect actual 24 kHz PCM synthesis to the existing Audio Service. Adjust
   the current 160-character/two-second job limits based on measured synthesis
   behavior, not arbitrary timeout increases.
4. Compare real generated speech with a reference waveform, then listen to
   punctuation, numbers and multiple sentences. Naturalness is not established
   by nonzero samples or compilation.
5. Verify cold-installed VM speech without the ISO or host runtime attached,
   measure time-to-first-audio and memory, then replace the default voice.

The 24 kHz converter is preparatory plumbing only; the requested voice swap is
not complete.
