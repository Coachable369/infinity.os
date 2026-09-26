#!/usr/bin/env python3
"""Build the shared native speech-recognition provider for the selected target."""
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
REVISION = "511126b492dcb267cf30d49d631946d7b61a9530"
NEWLIB = "newlib-4.6.0.20260123"
DIGEST = "6ff27e3bf022666f43f7802255be680eeff722ac181b1725d21e2e8318604ee3"
LLVM = Path("/opt/homebrew/opt/llvm/bin")

# ------------------------=
# FUNC: run
# DESC: Executes a bounded build command, failing rather than using a stale artifact on errors.
# ------------------=
def run(*args, cwd=ROOT):
    subprocess.run([str(a) for a in args], cwd=cwd, check=True)

# ------------------------=
# FUNC: main
# DESC: Verifies source pins and builds private native libc, decoder, and immutable model objects.
# ------------------=
def main():
    arch = ARCH
    triple = TRIPLE
    machine_flags = " ".join(FLAGS)
    build = ROOT / "build"
    build.mkdir(exist_ok=True)
    source = build / "voice-pocketsphinx-src"
    if not source.exists():
        run("git", "clone", "--depth", "1", "--branch", "v5.1.1",
            "https://github.com/cmusphinx/pocketsphinx.git", source)
    revision = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
    if revision != REVISION or subprocess.check_output(["git", "-C", str(source), "status", "--porcelain", "--untracked-files=no"]):
        raise SystemExit("Unreviewed recognizer source; refusing build")
    archive = build / "newlib-voice.tar.gz"
    if not archive.exists():
        with urllib.request.urlopen(f"https://sourceware.org/pub/newlib/{NEWLIB}.tar.gz", timeout=90) as response:
            archive.write_bytes(response.read())
    if hashlib.sha256(archive.read_bytes()).hexdigest() != DIGEST:
        raise SystemExit("Native C library checksum mismatch")
    c_source = build / NEWLIB
    if not c_source.exists():
        with tarfile.open(archive) as tar:
            tar.extractall(build, filter="data")
    native = build / ("voice-newlib-" + arch)
    native.mkdir(exist_ok=True)
    if not (native / "Makefile").exists():
        run(c_source / "configure", "--target=" + triple, "--disable-newlib-supplied-syscalls",
            "--disable-libgloss", "--disable-multilib", "--disable-newlib-multithread",
            f"CC_FOR_TARGET={LLVM / 'clang'} --target={triple}",
            f"AR_FOR_TARGET={LLVM / 'llvm-ar'}", f"RANLIB_FOR_TARGET={LLVM / 'llvm-ranlib'}",
            f"CFLAGS_FOR_TARGET=-O2 {machine_flags} -ffunction-sections -fdata-sections", cwd=native)
    run("make", "-j4", "all-target-newlib", cwd=native)
    output = build / ("voice-pocketsphinx-arm" if arch == "aarch64" else "voice-pocketsphinx-x86_64")
    includes = [ROOT / "tools/voice-pocketsphinx/include", native / triple / "newlib/targ-include",
                c_source / "newlib/libc/include"]
    flags = machine_flags + " -ffunction-sections -fdata-sections " + " ".join("-I" + str(p) for p in includes)
    run("cmake", "-S", source, "-B", output, "-DCMAKE_SYSTEM_NAME=Generic",
        "-DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY", f"-DCMAKE_C_COMPILER={LLVM / 'clang'}",
        "-DCMAKE_C_COMPILER_TARGET=" + triple, f"-DCMAKE_AR={LLVM / 'llvm-ar'}",
        f"-DCMAKE_RANLIB={LLVM / 'llvm-ranlib'}", f"-DCMAKE_C_FLAGS={flags}",
        "-DPS_THREAD_LOCAL_RNG=OFF", "-DBUILD_TESTING=OFF", "-DCMAKE_BUILD_TYPE=Release")
    run("cmake", "--build", output, "--target", "pocketsphinx", "-j4")
    run(sys.executable, ROOT / "tools/voice-pocketsphinx/link-probe.py")

if __name__ == "__main__":
    main()
