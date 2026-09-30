#!/usr/bin/env python3
"""Build the shared freestanding C runtime used by native speech providers."""
from pathlib import Path
import hashlib
import os
import subprocess
import sys
import tarfile
import urllib.request

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from voice_target import ARCH, TRIPLE, FLAGS

ROOT = Path(__file__).resolve().parents[2]
NEWLIB = "newlib-4.6.0.20260123"
DIGEST = "6ff27e3bf022666f43f7802255be680eeff722ac181b1725d21e2e8318604ee3"
LLVM = Path("/opt/homebrew/opt/llvm/bin")


# ------------------------=
# FUNC: run
# DESC: Executes one bounded native-runtime preparation command.
# ------------------=
def run(*args, cwd=ROOT):
    subprocess.run([str(arg) for arg in args], cwd=cwd, check=True)


# ------------------------=
# FUNC: main
# DESC: Reconstructs the pinned target newlib without retaining a deprecated recognizer dependency.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/voice-native-runtime/newlib.py")
    build = ROOT / "build"
    build.mkdir(exist_ok=True)
    archive = build / "newlib-voice.tar.gz"
    if not archive.exists():
        with urllib.request.urlopen(f"https://sourceware.org/pub/newlib/{NEWLIB}.tar.gz", timeout=90) as response:
            archive.write_bytes(response.read())
    if hashlib.sha256(archive.read_bytes()).hexdigest() != DIGEST:
        raise SystemExit("Native C library checksum mismatch")
    source = build / NEWLIB
    if not source.exists():
        with tarfile.open(archive) as bundle:
            bundle.extractall(build, filter="data")
    output = build / ("voice-newlib-" + ARCH)
    output.mkdir(exist_ok=True)
    if not (output / "Makefile").exists():
        run(source / "configure", "--target=" + TRIPLE, "--disable-newlib-supplied-syscalls",
            "--disable-libgloss", "--disable-multilib", "--disable-newlib-multithread",
            f"CC_FOR_TARGET={LLVM / 'clang'} --target={TRIPLE}",
            f"AR_FOR_TARGET={LLVM / 'llvm-ar'}", f"RANLIB_FOR_TARGET={LLVM / 'llvm-ranlib'}",
            f"CFLAGS_FOR_TARGET=-O2 {' '.join(FLAGS)} -ffunction-sections -fdata-sections", cwd=output)
    run("make", "-j4", "all-target-newlib", cwd=output)


if __name__ == "__main__":
    main()
