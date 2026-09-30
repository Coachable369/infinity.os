#!/usr/bin/env python3
"""Prepare pinned whisper.cpp source, model, and host compile metadata."""
from pathlib import Path
import hashlib
import os
import subprocess
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "build/voice-whisper-src"
REFERENCE = ROOT / "build/voice-whisper/reference"
MODEL = ROOT / "model-cache/whisper-tiny.en.bin"
REVISION = "4979e04f5dcaccb36057e059bbaed8a2f5288315"
MODEL_SHA256 = "921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f"
MODEL_URL = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin"


# ------------------------=
# FUNC: run
# DESC: Executes one checked source preparation command inside the repository workspace.
# ------------------=
def run(*args):
    subprocess.run([str(arg) for arg in args], cwd=ROOT, check=True)


# ------------------------=
# FUNC: checksum
# DESC: Returns the complete SHA-256 of one immutable dependency artifact.
# ------------------=
def checksum(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


# ------------------------=
# FUNC: main
# DESC: Pins official source and model bytes and emits CPU-only compile commands for cross-compilation.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/voice-whisper/prepare.py")
    if not SOURCE.exists():
        run("git", "clone", "--depth", "1", "--branch", "v1.8.2",
            "https://github.com/ggml-org/whisper.cpp.git", SOURCE)
    revision = subprocess.check_output(["git", "-C", str(SOURCE), "rev-parse", "HEAD"], text=True).strip()
    dirty = subprocess.check_output(["git", "-C", str(SOURCE), "status", "--porcelain", "--untracked-files=no"])
    if revision != REVISION or dirty:
        raise RuntimeError("Unreviewed whisper.cpp source; refusing build")
    MODEL.parent.mkdir(exist_ok=True)
    if not MODEL.exists():
        pending = MODEL.with_suffix(".partial")
        pending.unlink(missing_ok=True)
        with urllib.request.urlopen(MODEL_URL, timeout=180) as response:
            pending.write_bytes(response.read())
        if checksum(pending) != MODEL_SHA256:
            pending.unlink()
            raise RuntimeError("Unverified Whisper model download")
        pending.replace(MODEL)
    if checksum(MODEL) != MODEL_SHA256:
        raise RuntimeError("Unverified Whisper model")
    run("cmake", "-S", SOURCE, "-B", REFERENCE, "-DCMAKE_BUILD_TYPE=Release",
        "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON", "-DBUILD_SHARED_LIBS=OFF",
        "-DWHISPER_BUILD_TESTS=OFF", "-DWHISPER_BUILD_EXAMPLES=OFF",
        "-DWHISPER_BUILD_SERVER=OFF", "-DGGML_NATIVE=OFF", "-DGGML_OPENMP=OFF",
        "-DGGML_ACCELERATE=OFF", "-DGGML_BLAS=OFF", "-DGGML_CPU_ALL_VARIANTS=OFF")


if __name__ == "__main__":
    main()
