#!/usr/bin/env python3
"""Behavioral native-guest decoder, cancellation, bounds, and repeat-use test."""
from pathlib import Path
import argparse
import json
import os
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[3]

# ------------------------=
# FUNC: main
# DESC: Executes actual guest recognition and asserts binary API results, not diagnostic log prose.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--accel", choices=("tcg", "hvf"), default="tcg")
    parser.add_argument("--pcm", type=Path, default=ROOT / "build/voice-pocketsphinx-src/test/data/goforward.raw",
                        help="Recorded signed little-endian 16 kHz mono PCM, at most ten seconds")
    parser.add_argument("--expected", default="go forward ten meters", help="Independent reference transcript")
    args = parser.parse_args()
    fixture = args.pcm.resolve()
    size = fixture.stat().st_size
    assert 0 < size <= 320000 and size % 2 == 0, "Fixture must contain complete bounded PCM samples"
    env = dict(os.environ, RUSTC_BOOTSTRAP="1", CARGO_TARGET_DIR=str(ROOT / "build/voice-pocketsphinx-probe-target"))
    env["INFINITY_STT_FIXTURE"] = str(fixture)
    subprocess.run(["cargo", "build", "--manifest-path", "tools/voice-pocketsphinx/probe/Cargo.toml",
                    "--release", "-Z", "build-std=core", "--target", "aarch64-unknown-none-softfloat"],
                   cwd=ROOT, env=env, check=True)
    output = ROOT / "build/voice-pocketsphinx-arm"
    subprocess.run(["/opt/homebrew/opt/lld/bin/ld.lld", "--gc-sections", "-nostdlib", "-T",
                    str(ROOT / "tools/attention-softfloat-probe.ld"), "-o", str(output / "probe.elf"),
                    str(ROOT / "build/voice-pocketsphinx-probe-target/aarch64-unknown-none-softfloat/release/libinfinity_stt_probe.a"),
                    str(output / "private-native.o")], check=True)
    result = output / f"verified-{args.accel}.bin"
    # A unique current invocation output prevents stale pass evidence.
    if result.exists():
        result.unlink()
    subprocess.run(["qemu-system-aarch64", "-machine", "virt", "-accel", args.accel, "-cpu",
                    "host" if args.accel == "hvf" else "max", "-m", "512M", "-display", "none",
                    "-serial", "file:" + str(result), "-monitor", "none", "-kernel", str(output / "probe.elf")],
                   check=True, timeout=120)
    data = result.read_bytes()
    rows = []
    offset = 0
    for case in range(6):
        assert len(data) - offset >= 80, "Guest exception, panic, or incomplete result"
        version, index, code, length, memory, ticks, frequency, live, erased, stack = struct.unpack_from("<10Q", data, offset)
        offset += 80
        assert version == 3 and index == case and frequency > 0
        assert 0 < stack < 1024 * 1024 - 4096, "Native worker exceeded its reserved stack"
        assert code == [0, 0, 2, 6, 5, 2][case]
        assert 0 < memory <= 192 * 1024 * 1024 and erased > 0
        assert offset + length <= len(data)
        text = data[offset:offset + length]
        offset += length
        # Transcript bytes are the recognizer's actual API output, not a log oracle.
        if case < 2:
            assert text == args.expected.encode(), "Completed recording must match its reference transcript"
        else:
            assert length == 0
        if rows:
            assert live == rows[0]["retained_bytes"] and memory == rows[0]["heap_committed_bytes"]
            assert erased >= rows[-1]["erased_bytes"]
            if case not in (4, 5):
                assert erased > rows[-1]["erased_bytes"]
        rows.append(dict(case=case, result=code, transcript=text.decode(), seconds=ticks/frequency,
                         heap_committed_bytes=memory, retained_bytes=live, erased_bytes=erased, stack_bytes=stack))
    assert offset == len(data)
    evidence = dict(environment=f"freestanding ARM64 QEMU {args.accel}; not installed InfinityOS",
                    fixture=str(fixture), expected=args.expected,
                    accuracy_scope="One recorded fixture; live microphone word error rate is not established",
                    cases=rows)
    (output / f"verified-{args.accel}.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence, indent=2))

if __name__ == "__main__":
    main()
