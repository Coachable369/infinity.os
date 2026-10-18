#!/usr/bin/env python3
"""Prepare pinned CPU image inference sources without touching installed models."""
from pathlib import Path
import os
import subprocess

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "build/image-native-src"
WORK = ROOT / "build/image-native"
REVISION = "b13a48219d5cdf408edcb93da6cd69b612ed03d3"
GGML_REVISION = "dc998771e700d49bbdf9a39cede22bb5b3311767"


# ------------------------=
# FUNC: run
# DESC: Execute a checked source preparation operation inside the build kit.
# ------------------=
def run(*args):
    subprocess.run([str(arg) for arg in args], cwd=ROOT, check=True)


# ------------------------=
# FUNC: verify
# DESC: Reject unreviewed revisions and modified tracked inference sources.
# ------------------=
def verify(path, revision):
    head = subprocess.check_output(["git", "-C", str(path), "rev-parse", "HEAD"], text=True).strip()
    if head != revision:
        raise RuntimeError(f"Unreviewed image dependency revision: {path}: {head}")
    run("git", "-C", path, "diff", "--exit-code", "HEAD", "--", ".")


# ------------------------=
# FUNC: main
# DESC: Pin the independent engine and generate CPU-only compilation metadata.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/image-native/prepare.py")
    WORK.mkdir(parents=True, exist_ok=True)
    if not SOURCE.exists():
        run("git", "init", SOURCE)
        run("git", "-C", SOURCE, "remote", "add", "origin", "https://github.com/leejet/stable-diffusion.cpp.git")
        run("git", "-C", SOURCE, "fetch", "--depth=1", "origin", REVISION)
        run("git", "-C", SOURCE, "checkout", "--detach", REVISION)
    verify(SOURCE, REVISION)
    run("git", "-C", SOURCE, "submodule", "update", "--init", "--depth=1", "ggml")
    verify(SOURCE / "ggml", GGML_REVISION)
    # This host configure step emits source lists only. Its libraries must never
    # be linked into InfinityOS; native-build.py cross-compiles every object.
    run("cmake", "-S", SOURCE, "-B", WORK / "reference", "-G", "Ninja",
        "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON", "-DCMAKE_BUILD_TYPE=Release",
        "-DSD_BUILD_EXAMPLES=OFF", "-DSD_BUILD_SHARED_LIBS=OFF",
        "-DSD_BUILD_SHARED_GGML_LIB=OFF", "-DSD_WEBP=OFF", "-DSD_WEBM=OFF",
        "-DSD_CUDA=OFF", "-DSD_HIPBLAS=OFF", "-DSD_METAL=OFF",
        "-DSD_VULKAN=OFF", "-DSD_OPENCL=OFF", "-DSD_SYCL=OFF", "-DSD_MUSA=OFF",
        "-DGGML_NATIVE=OFF", "-DGGML_OPENMP=OFF", "-DGGML_ACCELERATE=OFF",
        "-DGGML_BLAS=OFF", "-DGGML_METAL=OFF", "-DGGML_CUDA=OFF",
        "-DGGML_VULKAN=OFF", "-DSD_RPC=OFF", "-DGGML_RPC=OFF",
        "-DGGML_CPU_ALL_VARIANTS=OFF", "-DGGML_BACKEND_DL=OFF")


if __name__ == "__main__":
    main()
