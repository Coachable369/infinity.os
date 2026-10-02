#!/usr/bin/env python3
"""Behavioral freestanding-guest test for the native Whisper provider."""
from pathlib import Path
import argparse
import json
import os
import re
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[3]
EXPECTED_WORDS = "and so my fellow americans ask not what your country can do for you ask what you can do for your country".split()


# ------------------------=
# FUNC: normalized_words
# DESC: Normalizes recognizer API output for punctuation-insensitive recorded-speech comparison.
# ------------------=
def normalized_words(value):
    return re.findall(r"[a-z]+", value.lower())


# ------------------------=
# FUNC: main
# DESC: Executes genuine ARM Whisper inference and validates typed results from five API state transitions.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--accel", choices=("tcg", "hvf"), default="hvf")
    parser.add_argument("--fixture", type=Path)
    parser.add_argument("--expected")
    args = parser.parse_args()
    output = ROOT / "build/voice-whisper/aarch64"
    source = args.fixture or ROOT / "build/voice-whisper-src/samples/jfk.wav"
    expected_words = normalized_words(args.expected) if args.expected else EXPECTED_WORDS
    if args.fixture and not args.expected:
        raise SystemExit("--fixture requires --expected")
    fixture = output / ("wake-word.raw" if args.fixture else "jfk-10s.raw")
    output.mkdir(parents=True, exist_ok=True)
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i",
                    str(source), "-t", "10",
                    "-ar", "16000", "-ac", "1", "-f", "s16le", str(fixture)], check=True)
    assert 0 < fixture.stat().st_size <= 320000
    target = ROOT / "build/voice-whisper-probe-target"
    env = dict(os.environ, RUSTC_BOOTSTRAP="1", CARGO_TARGET_DIR=str(target),
               INFINITY_STT_FIXTURE=str(fixture))
    subprocess.run(["cargo", "build", "--manifest-path", "tools/voice-whisper/probe/Cargo.toml",
                    "--release", "-Z", "build-std=core", "--target", "aarch64-unknown-none-softfloat"],
                   cwd=ROOT, env=env, check=True)
    image = output / "probe.elf"
    subprocess.run(["/opt/homebrew/opt/lld/bin/ld.lld", "--gc-sections", "-nostdlib", "-T",
                    str(ROOT / "tools/attention-softfloat-probe.ld"), "-o", str(image),
                    str(target / "aarch64-unknown-none-softfloat/release/libinfinity_whisper_probe.a"),
                    str(ROOT / "build/voice-kokoro/aarch64/private-native.o")], check=True)
    result = output / ("verified-" + args.accel + ".bin")
    result.unlink(missing_ok=True)
    subprocess.run(["qemu-system-aarch64", "-machine", "virt", "-accel", args.accel,
                    "-cpu", "host" if args.accel == "hvf" else "max", "-m", "2G",
                    "-display", "none", "-serial", "file:" + str(result), "-monitor", "none",
                    "-kernel", str(image)], check=True, timeout=360)
    data = result.read_bytes()
    offset = 0
    rows = []
    for case in range(5):
        assert len(data) - offset >= 160, "Guest exception, panic, or incomplete result"
        version, index, code, length, memory, ticks, frequency, stack = struct.unpack_from("<8Q", data, offset)
        offset += 64
        diagnostics = struct.unpack_from("<12Q", data, offset)
        offset += 96
        assert version == 1 and index == case and frequency > 0
        assert code == [0, 0, 6, 5, 2][case]
        assert 0 < memory <= 1536 * 1024 * 1024 and 0 < stack < 8 * 1024 * 1024
        transcript = data[offset:offset + length].decode("utf-8")
        offset += length
        if case < 2:
            assert normalized_words(transcript) == expected_words
        else:
            assert transcript == ""
        rows.append(dict(case=case, result=code, transcript=transcript,
                         seconds=ticks / frequency, heap_committed_bytes=memory, stack_bytes=stack,
                         diagnostics=diagnostics))
    assert offset == len(data)
    # Synthesis must not evict the prepared recognizer. Case 1 performs Kokoro
    # immediately before the timed Whisper call; it must remain a warm decode.
    assert rows[1]["seconds"] <= rows[0]["seconds"] * 1.5 + 2.0
    evidence = dict(engine="whisper.cpp tiny.en", execution="freestanding ARM64 QEMU " + args.accel,
                    fixture=str(source), cases=rows)
    (output / ("verified-" + args.accel + ".json")).write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence, indent=2))


if __name__ == "__main__":
    main()
