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
# FUNC: main
# DESC: Checks waveform agreement while allowing the reference API's documented trailing-silence trim.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", choices=("aarch64", "x86_64"), default="aarch64")
    args = parser.parse_args()
    output = ROOT / "build/voice-kokoro" / ("x86-runtime" if args.target == "x86_64" else "aarch64")
    reference = read_pcm(ROOT / "build/voice-kokoro/reference/hello.wav")
    native = read_pcm(output / ("native-hi.wav" if args.target == "x86_64" else "native-0.wav"))
    assert len(reference) <= len(native) <= len(reference) + 2400
    compared = native[:len(reference)]
    energy = sum(v*v for v in reference)
    native_energy = sum(v*v for v in compared)
    assert energy > 0 and native_energy > 0
    correlation = sum(a*b for a,b in zip(reference,compared))/math.sqrt(energy*native_energy)
    relative_error = math.sqrt(sum((a-b)**2 for a,b in zip(reference,compared))/energy)
    evidence = dict(correlation=correlation,relative_rms_error=relative_error,
                    reference_frames=len(reference),native_frames=len(native),installed_verified=False,
                    target=args.target, native_latency_verified=False)
    print(json.dumps(evidence,indent=2))
    assert correlation > 0.98 and relative_error < 0.15, evidence
    (output / "reference-comparison.json").write_text(json.dumps(evidence,indent=2)+"\n")


if __name__ == "__main__":
    main()
