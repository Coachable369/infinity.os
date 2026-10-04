#!/usr/bin/env python3
"""Compare actual native recognition of immutable real and synthetic fixtures."""
from pathlib import Path
import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "build/voice-whisper/conversation-performance"
FIXTURES = [("jfk", None), ("hello", "Hello."), ("wake", "Infinity."),
            ("command", "Infinity, what time is it?"),
            ("unaddressed", "The weather is sunny today."),
            ("computer", "Computer, hello.")]


# ------------------------=
# FUNC: run
# DESC: Executes one bounded fixture preparation or serialized native guest measurement.
# ------------------=
def run(*args):
    subprocess.run([str(value) for value in args], cwd=ROOT, check=True, timeout=360)


# ------------------------=
# FUNC: main
# DESC: Retains paired engine artifacts, exact recognition outcomes and first/warm native timing without claiming microphone or installed proof.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE") == "1", "Use ./build-kit run"
    parser = argparse.ArgumentParser()
    parser.add_argument("--label", required=True, choices=("baseline", "candidate"))
    args = parser.parse_args()
    output = OUT / args.label
    output.mkdir(parents=True, exist_ok=True)
    fixture_dir = OUT / "fixtures"
    fixture_dir.mkdir(parents=True, exist_ok=True)
    engine = ROOT / "build/voice-kokoro/aarch64/private-native.o"
    # Copies are test artifacts, retained separately from subsequent release rebuilds.
    shutil.copyfile(engine, output / "private-native.o")
    rows = []
    for name, text in FIXTURES:
        arguments = []
        if text is not None:
            audio = fixture_dir / (name + ".wav")
            if not audio.exists():
                assert args.label == "baseline", "Candidate must reuse baseline fixture bytes"
                run(ROOT / "build/voice-kokoro/reference/kokopop_say", "--model",
                    ROOT / "model-cache/kokoro-v1_0.gguf", "--backend", "cpu", "--threads", "1",
                    "--voice", "af_heart", "--speed", "1.0", "--text", text, "--out", audio)
            arguments = ["--fixture", audio, "--expected", text]
        else:
            audio = ROOT / "build/voice-whisper-src/samples/jfk.wav"
        run(sys.executable, ROOT / "tools/voice-whisper/probe/run.py", "--benchmark-only", *arguments)
        evidence = ROOT / "build/voice-whisper/aarch64/verified-hvf.json"
        data = json.loads(evidence.read_text())
        shutil.copyfile(evidence, output / (name + ".json"))
        shutil.copyfile(evidence.with_suffix(".bin"), output / (name + ".bin"))
        rows.append(dict(name=name, source="real recording" if text is None else "synthetic fixture only",
                         fixture_sha256=hashlib.sha256(audio.read_bytes()).hexdigest(),
                         first_seconds=data["cases"][0]["seconds"],
                         warm_seconds=data["cases"][1]["seconds"],
                         transcript=data["cases"][0]["transcript"]))
    result = dict(environment="freestanding ARM64 QEMU HVF", installed_verified=False,
                  microphone_accuracy_verified=False,
                  engine_sha256=hashlib.sha256(engine.read_bytes()).hexdigest(), cases=rows)
    if args.label == "candidate":
        baseline = json.loads((OUT / "baseline/results.json").read_text())
        for old, new in zip(baseline["cases"], rows):
            assert old["name"] == new["name"] and old["fixture_sha256"] == new["fixture_sha256"]
            # Both runs independently assert exact decoded words against the fixture's source text.
            new["warm_reduction_percent"] = 100 * (1 - new["warm_seconds"] / old["warm_seconds"])
    (output / "results.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2), flush=True)


if __name__ == "__main__":
    main()
