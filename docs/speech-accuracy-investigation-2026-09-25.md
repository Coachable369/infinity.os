# Speech accuracy investigation

Reported failures: “Hello” -> “no sugar”; “What is your name” -> “Gervais”.
These are unresolved live-input failures, not accepted recognition quality.

## Evidence

The native probe now accepts a bounded 16 kHz signed little-endian mono PCM
recording and its independent expected transcript. It embeds that recording
into the freestanding ARM64 binary; no host decoder supplies guest output.

Two diagnostic fixtures were synthesized locally with macOS Samantha, converted
to 16 kHz mono, then passed through both the host reference and native QEMU TCG
decoder. Both returned exactly `hello` and `what is your name`. Each native run
also passed repeated recognition, cancellation, small output buffer, silence,
and immediate cancellation assertions. Evidence is in
`build/voice-accuracy/hello-native.json` and `name-native.json`.

`make voice-pcm-test` also passed sample-rate, timing, anti-aliasing,
chunk-invariance, VAD integration, and private-buffer erasure checks.

These synthetic fixtures bypass VirtualBox capture and do not represent the
user's voice, room, microphone, or actual captured PCM. They cannot establish
that live recognition is fixed, nor distinguish capture distortion/truncation
from acoustic-model limitations. No runtime tuning or word replacement was
introduced on the strength of these results. No new ISO is needed for these
test-only changes.

## Reproduce against a real recording

```sh
python3 tools/voice-pocketsphinx/probe/run.py \
  --pcm /absolute/path/hello-16k-mono.raw --expected hello
```

Next required evidence is the failing microphone recording, preferably the
guest's actual post-capture PCM. Compare listening quality, duration, clipping,
and word boundaries, then replay identical bytes through the host and native
decoders. Do not infer a hardware or model fix from the synthetic tests.
