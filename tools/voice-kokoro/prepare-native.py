#!/usr/bin/env python3
"""Build a private freestanding C++ runtime from pinned sources, never host libraries."""
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / "build/voice-kokoro"
LLVM = Path("/opt/homebrew/opt/llvm/bin")
REVISION = "ea7d852a70e8bdfaf601d6626a760f9771b2c4b4"


# ------------------------=
# FUNC: run
# DESC: Stops dependency preparation at the first failed build operation.
# ------------------=
def run(*args):
    subprocess.run([str(arg) for arg in args], cwd=ROOT, check=True)


# ------------------------=
# FUNC: main
# DESC: Reconstructs the native C++ runtime after a clean build directory removal.
# ------------------=
def main():
    source = WORK / "llvm-src"
    WORK.mkdir(parents=True, exist_ok=True)
    if not source.exists():
        run("git", "init", source)
        run("git", "-C", source, "remote", "add", "origin", "https://github.com/llvm/llvm-project.git")
        run("git", "-C", source, "config", "remote.origin.promisor", "true")
        run("git", "-C", source, "config", "remote.origin.partialclonefilter", "blob:none")
        run("git", "-C", source, "fetch", "--depth=1", "--filter=blob:none", "origin", REVISION)
    run("git", "-C", source, "sparse-checkout", "set", "libcxx", "libcxxabi", "runtimes",
        "cmake", "llvm/cmake", "llvm/utils/llvm-lit", "llvm/utils/lit", "libc")
    head = subprocess.run(["git", "-C", str(source), "rev-parse", "--verify", "HEAD"],
                          text=True, capture_output=True).stdout.strip()
    if head != REVISION:
        if subprocess.check_output(["git", "-C", str(source), "diff", "--name-only"]):
            raise RuntimeError("Refusing to replace modified runtime sources")
        run("git", "-C", source, "checkout", "--detach", REVISION)
    patch = ROOT / "sdk/compiler/libcxx-infinity.patch"
    if subprocess.run(["git", "-C", str(source), "apply", "--reverse", "--check", str(patch)],
                      stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode:
        run("git", "-C", source, "apply", patch)
    newlib = ROOT / "build/voice-newlib-aarch64/aarch64-none-elf/newlib"
    if not (newlib / "libc.a").exists():
        raise RuntimeError("Build pinned newlib first with tools/voice-pocketsphinx/build.py")
    flags = " ".join(["-O2 -mstrict-align -ffunction-sections -fdata-sections",
        "-include " + str(ROOT / "sdk/compiler/target.h"),
        "-I" + str(ROOT / "sdk/compiler/include"),
        "-isystem " + str(newlib / "targ-include"),
        "-isystem " + str(ROOT / "build/newlib-4.6.0.20260123/newlib/libc/include")])
    output = WORK / "cxx-aarch64"
    run("cmake", "-G", "Ninja", "-S", source / "runtimes", "-B", output,
        "-C", ROOT / "sdk/compiler/cmake/runtime-options.cmake",
        "-DLLVM_ENABLE_RUNTIMES=libcxx;libcxxabi", "-DLIBCXXABI_USE_LLVM_UNWINDER=OFF",
        "-DLLVM_DEFAULT_TARGET_TRIPLE=aarch64-none-elf", "-DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY",
        "-DCMAKE_BUILD_TYPE=Release", "-DCMAKE_SYSTEM_NAME=InfinityOS",
        "-DCMAKE_MODULE_PATH=" + str(ROOT / "sdk/compiler/cmake"),
        "-DCMAKE_C_COMPILER=" + str(LLVM / "clang"), "-DCMAKE_CXX_COMPILER=" + str(LLVM / "clang++"),
        "-DCMAKE_ASM_COMPILER=" + str(LLVM / "clang"),
        "-DCMAKE_C_COMPILER_TARGET=aarch64-none-elf", "-DCMAKE_CXX_COMPILER_TARGET=aarch64-none-elf",
        "-DCMAKE_ASM_COMPILER_TARGET=aarch64-none-elf", "-DCMAKE_AR=" + str(LLVM / "llvm-ar"),
        "-DCMAKE_RANLIB=" + str(LLVM / "llvm-ranlib"),
        "-DCMAKE_LINKER=/opt/homebrew/opt/lld/bin/ld.lld",
        "-DCMAKE_INSTALL_PREFIX=" + str(WORK / "sysroot"),
        "-DCMAKE_C_FLAGS=" + flags, "-DCMAKE_CXX_FLAGS=" + flags)
    run("ninja", "-C", output, "-j4", "cxx", "cxxabi")


if __name__ == "__main__":
    main()
