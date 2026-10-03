#!/usr/bin/env python3
"""Compare actual native PCM with the independently synthesized 'Hi.' reference."""
from pathlib import Path
import array
import argparse
import json
import math
import wave

ROOT = Path(__file__).resolve().parents[2]


# ------------------------=
# FUNC: read_pcm
# DESC: Reads actual mono 24-kHz signed PCM and rejects mismatched formats.
# ------------------=
def read_pcm(path):
    with wave.open(str(path), "rb") as stream:
        assert (stream.getnchannels(), stream.getsampwidth(), stream.getframerate()) == (1, 2, 24000)
        return array.array("h", stream.readframes(stream.getnframes()))


# ------------------------=
# FUNC: active_pcm
# DESC: Aligns only bit-exact edge silence without trimming quiet speech, changing internal pauses or resampling.
# ------------------=
def active_pcm(samples):
    start = 0
    while start < len(samples) and samples[start] == 0:
        start += 1
    end = len(samples)
    while end > start and samples[end - 1] == 0:
        end -= 1
    return samples[start:end]


# ------------------------=
# FUNC: compare_pcm
# DESC: Compares the entire active waveform at unchanged tolerances and rejects missing quiet edges or internal timing changes.
# ------------------=
def compare_pcm(reference, native):
    reference_active = active_pcm(reference)
    native_active = active_pcm(native)
    assert len(reference_active) == len(native_active) and reference_active
    energy = sum(v*v for v in reference_active)
    native_energy = sum(v*v for v in native_active)
    assert energy > 0 and native_energy > 0
    correlation = sum(a*b for a,b in zip(reference_active,native_active))/math.sqrt(energy*native_energy)
    relative_error = math.sqrt(sum((a-b)**2 for a,b in zip(reference_active,native_active))/energy)
    evidence = dict(correlation=correlation,relative_rms_error=relative_error,
                    reference_frames=len(reference),native_frames=len(native),
                    active_frames=len(reference_active),installed_verified=False)
    assert correlation > 0.98 and relative_error < 0.15, evidence
    return evidence


# ------------------------=
# FUNC: main
# DESC: Checks complete waveform agreement while allowing only exact-zero model or API edge padding differences.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", choices=("aarch64", "x86_64"), default="aarch64")
    args = parser.parse_args()
    output = ROOT / "build/voice-kokoro" / ("x86-runtime" if args.target == "x86_64" else "aarch64")
    reference = read_pcm(ROOT / "build/voice-kokoro/reference/hello.wav")
    native = read_pcm(output / ("native-hi.wav" if args.target == "x86_64" else "native-0.wav"))
    evidence = dict(compare_pcm(reference, native), target=args.target, native_latency_verified=False)
    print(json.dumps(evidence,indent=2))
    (output / "reference-comparison.json").write_text(json.dumps(evidence,indent=2)+"\n")


if __name__ == "__main__":
    main()
