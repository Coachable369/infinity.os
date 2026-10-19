#!/usr/bin/env python3
"""Verify synthetic wake commands after production VAD using actual native Whisper."""
from pathlib import Path
import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "builds/validation/wake-capture/native-audio"
WORDS = "Infinity create a simple graphical calculator app."


# ------------------------=
# FUNC: run
# DESC: Runs one bounded child inside the existing build-kit authority without host speech recognition or outside services.
# ------------------=
def run(*arguments, capture=False):
    return subprocess.run([str(argument) for argument in arguments], cwd=ROOT, check=True,
                          timeout=360, capture_output=capture, text=capture)


# ------------------------=
# FUNC: main
# DESC: Preserves synthetic fixture hashes, typed VAD receipts and real freestanding ARM recognition results without claiming live microphone accuracy.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE") == "1", "Use ./build-kit run"
    parser = argparse.ArgumentParser()
    parser.add_argument("--variant", choices=("normal", "quiet", "soft-onset"))
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    source = OUT / "source.wav"
    if not source.exists():
        run(ROOT / "build/voice-kokoro/reference/kokopop_say", "--model",
            ROOT / "model-cache/kokoro-v1_0.gguf", "--backend", "cpu", "--threads", "1",
            "--voice", "af_heart", "--speed", "1.0", "--text", WORDS, "--out", source)
    raw = OUT / "source.raw"
    run("ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", source,
        "-ar", "16000", "-ac", "1", "-f", "s16le", raw)
    executable = ROOT / "build/tools/voice-wake-pcm-probe"
    executable.parent.mkdir(parents=True, exist_ok=True)
    run("rustc", "--edition=2021", "-O", "tools/voice-wake-pcm-probe.rs", "-o", executable)
    segmented = run(executable, raw, OUT, capture=True)
    rows = [json.loads(line) for line in segmented.stdout.splitlines()]
    if args.variant:
        rows = [row for row in rows if row["variant"] == args.variant]
    for row in rows:
        name = row["variant"]
        fixture = OUT / f"{name}.wav"
        run("ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-f", "s16le",
            "-ar", "16000", "-ac", "1", "-i", OUT / f"{name}.raw", fixture)
        completed = subprocess.run([sys.executable, str(ROOT / "tools/voice-whisper/probe/run.py"),
            "--benchmark-only", "--fixture", str(fixture), "--expected", WORDS],
            cwd=ROOT, timeout=360)
        evidence = ROOT / "build/voice-whisper/aarch64/verified-hvf.bin"
        if evidence.exists():
            shutil.copyfile(evidence, OUT / f"{name}-recognition.bin")
        row["probe_exit_code"] = completed.returncode
        row["fixture_sha256"] = hashlib.sha256(fixture.read_bytes()).hexdigest()
        structured = evidence.with_suffix(".json")
        if completed.returncode == 0:
            row["recognition"] = json.loads(structured.read_text())
    result = dict(source="synthetic Kokoro fixture through production calibrated VAD",
                  source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
                  native_engine_sha256=hashlib.sha256((ROOT / "build/voice-kokoro/aarch64/private-native.o").read_bytes()).hexdigest(),
                  microphone_accuracy_verified=False, installed_verified=False, cases=rows)
    name = f"evidence-{args.variant}.json" if args.variant else "evidence.json"
    (OUT / name).write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))
    assert all(row["probe_exit_code"] == 0 for row in rows), "Native wake recognition failed; inspect structured receipts"


if __name__ == "__main__":
    main()
