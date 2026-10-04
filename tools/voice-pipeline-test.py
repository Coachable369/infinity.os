#!/usr/bin/env python3
"""Behavioral voice gates shared by focused verification and every ISO build."""
from pathlib import Path
import os
import subprocess

ROOT = Path(__file__).resolve().parents[1]


# ------------------------=
# FUNC: run
# DESC: Runs one behavioral harness under the enclosing serialized build-kit authority.
# ------------------=
def run(*command):
    subprocess.run(command, cwd=ROOT, check=True)


# ------------------------=
# FUNC: main
# DESC: Exercises the production turn controller, timing, PCM, VAD and output state machines without log-text oracles.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/voice-pipeline-test.py")
    output = ROOT / "build/behavior-tests"
    output.mkdir(parents=True, exist_ok=True)
    run("cargo", "test", "--quiet", "--manifest-path", "tools/behavior-harness/Cargo.toml",
        "--bin", "voice-toggle-test")
    run("rustc", "--edition=2021", "--test", "kernel/runtime/ai/voice_timing.rs",
        "-o", str(output / "voice-timing-test"))
    run(str(output / "voice-timing-test"))
    run("clang", "-O3", "-ffp-contract=off", "-c", "kernel/runtime/ai/qwen/cpu_math.c",
        "-o", str(output / "qwen-worker-math.o"))
    run("rustc", "--edition=2021", "--test", "kernel/runtime/ai/qwen/workers.rs",
        "-C", "link-arg=" + str(output / "qwen-worker-math.o"),
        "-o", str(output / "qwen-workers-test"))
    run(str(output / "qwen-workers-test"))
    run("python3", "tools/voice-kokoro/test_pcm_boundary.py")
    run("python3", "-m", "unittest", "discover", "-s", "tools/voice-kokoro",
        "-p", "test_latency*.py")
    run("make", "voice-output-test", "voice-pcm-test", "voice-vad-test", "audio-test")


if __name__ == "__main__":
    main()
