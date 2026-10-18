#!/usr/bin/env python3
"""Link-audit the CPU image engine against existing native OS service adapters."""
from pathlib import Path
import json
import os
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from voice_target import ARCH, TRIPLE, FLAGS
from prepare import ROOT, WORK


# ------------------------=
# FUNC: run
# DESC: Execute a checked native compilation or relocatable link operation.
# ------------------=
def run(*args):
    subprocess.run([str(arg) for arg in args], cwd=ROOT, check=True)


# ------------------------=
# FUNC: main
# DESC: Diagnose unresolved native dependencies without publishing a runnable provider.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/image-native/link-native.py")
    output = WORK / ARCH
    llvm = Path("/opt/homebrew/opt/llvm/bin")
    newlib = ROOT / "build" / ("voice-newlib-" + ARCH) / TRIPLE / "newlib"
    objects = json.loads((output / "objects.json").read_text())
    for name in ("platform", "dlfcn", "files", "heap", "time", "serial_sync",
                 "serial_tls", "pthread", "namespace", "process", "newlib"):
        obj = output / ("native-service-" + name + ".o")
        run(llvm / "clang", "--target=" + TRIPLE, "-O2", *FLAGS,
            "-ffreestanding", "-fno-builtin", "-fno-stack-protector",
            "-ffunction-sections", "-fdata-sections", "-std=gnu11",
            "-include", ROOT / "sdk/compiler/target.h",
            "-I" + str(ROOT / "sdk/compiler/include"),
            "-isystem", newlib / "targ-include",
            "-isystem", ROOT / "build/newlib-4.6.0.20260123/newlib/libc/include",
            "-c", ROOT / ("sdk/compiler/" + name + ".c"), "-o", obj)
        objects.append(str(obj))
    linked = output / "link-audit.o"
    run("/opt/homebrew/opt/lld/bin/ld.lld", "-r", "--gc-sections",
        "--undefined=new_sd_ctx", "--undefined=generate_image", "--undefined=free_sd_ctx",
        "--undefined=free_sd_images", "--undefined=sd_set_progress_callback",
        "--undefined=infinity_compiler_set_host", "-o", linked, *objects,
        "--start-group", WORK / ("cxx-" + ARCH) / "lib/libc++.a",
        WORK / ("cxx-" + ARCH) / "lib/libc++abi.a",
        newlib / "libm.a", newlib / "libc.a", "--end-group")
    # This is a linker diagnostic, not behavioral/inference acceptance.
    symbols = subprocess.check_output([str(llvm / "llvm-nm"), "--undefined-only", str(linked)], text=True)
    unresolved = sorted({line.split()[-1] for line in symbols.splitlines() if line.split()})
    (output / "link-audit.json").write_text(json.dumps({
        "architecture": ARCH, "unresolved": unresolved, "inference_verified": False,
    }, indent=2) + "\n")
    if unresolved:
        raise RuntimeError("Unresolved native image dependencies: " + ", ".join(unresolved))


if __name__ == "__main__":
    main()
