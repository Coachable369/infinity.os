#!/usr/bin/env python3
"""Build an independent freestanding image C++ runtime with file stream support."""
from pathlib import Path
import os
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from voice_target import ARCH, TRIPLE, FLAGS
from prepare import ROOT, WORK


# ------------------------=
# FUNC: main
# DESC: Build image-private standard libraries without changing the speech runtime.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/image-native/prepare-runtime.py")
    source = ROOT / "build/voice-kokoro/llvm-src"
    revision = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
    if revision != "ea7d852a70e8bdfaf601d6626a760f9771b2c4b4":
        raise RuntimeError("Prepare the pinned native LLVM runtime source first")
    newlib = ROOT / "build" / ("voice-newlib-" + ARCH) / TRIPLE / "newlib"
    if not (newlib / "libc.a").is_file():
        raise RuntimeError("Prepare target newlib first")
    llvm = Path("/opt/homebrew/opt/llvm/bin")
    flags = " ".join(["-O2 -ffunction-sections -fdata-sections", *FLAGS,
        "-include " + str(ROOT / "sdk/compiler/target.h"),
        "-include " + str(ROOT / "tools/image-native/runtime-compat.h"),
        "-I" + str(ROOT / "sdk/compiler/include"),
        "-isystem " + str(newlib / "targ-include"),
        "-isystem " + str(ROOT / "build/newlib-4.6.0.20260123/newlib/libc/include")])
    output = WORK / ("cxx-" + ARCH)
    args = ["cmake", "-G", "Ninja", "-S", source / "runtimes", "-B", output,
        "-C", ROOT / "sdk/compiler/cmake/runtime-options.cmake",
        "-DLIBCXX_ENABLE_FILESYSTEM=ON", "-DLIBCXX_ENABLE_RTTI=ON", "-DLIBCXXABI_ENABLE_RTTI=ON",
        "-DLLVM_ENABLE_RUNTIMES=libcxx;libcxxabi",
        "-DLIBCXXABI_USE_LLVM_UNWINDER=OFF", "-DLLVM_DEFAULT_TARGET_TRIPLE=" + TRIPLE,
        "-DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY", "-DCMAKE_BUILD_TYPE=Release",
        "-DCMAKE_SYSTEM_NAME=InfinityOS", "-DCMAKE_MODULE_PATH=" + str(ROOT / "sdk/compiler/cmake"),
        "-DCMAKE_C_COMPILER=" + str(llvm / "clang"), "-DCMAKE_CXX_COMPILER=" + str(llvm / "clang++"),
        "-DCMAKE_ASM_COMPILER=" + str(llvm / "clang"), "-DCMAKE_C_COMPILER_TARGET=" + TRIPLE,
        "-DCMAKE_CXX_COMPILER_TARGET=" + TRIPLE, "-DCMAKE_ASM_COMPILER_TARGET=" + TRIPLE,
        "-DCMAKE_AR=" + str(llvm / "llvm-ar"), "-DCMAKE_RANLIB=" + str(llvm / "llvm-ranlib"),
        "-DCMAKE_LINKER=/opt/homebrew/opt/lld/bin/ld.lld",
        "-DCMAKE_INSTALL_PREFIX=" + str(output / "sysroot"),
        "-DCMAKE_C_FLAGS=" + flags, "-DCMAKE_CXX_FLAGS=" + flags]
    subprocess.run([str(arg) for arg in args], cwd=ROOT, check=True)
    subprocess.run(["ninja", "-C", str(output), "-j4", "cxx", "cxxabi"], cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
