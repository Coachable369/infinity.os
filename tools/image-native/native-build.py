#!/usr/bin/env python3
"""Cross-compile the image backend; never reuse host-compiled libraries."""
from pathlib import Path
import hashlib
import json
import os
import shlex
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from voice_target import ARCH, TRIPLE, FLAGS
from prepare import ROOT, WORK, SOURCE, verify, REVISION, GGML_REVISION

LLVM = Path("/opt/homebrew/opt/llvm/bin")


# ------------------------=
# FUNC: main
# DESC: Compile pinned CPU translation units against native newlib and libc++ only.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/image-native/native-build.py")
    verify(SOURCE, REVISION)
    verify(SOURCE / "ggml", GGML_REVISION)
    output = WORK / ARCH
    output.mkdir(parents=True, exist_ok=True)
    newlib = ROOT / "build" / ("voice-newlib-" + ARCH) / TRIPLE / "newlib"
    cxx = WORK / ("cxx-" + ARCH) / "include/c++/v1"
    if not (newlib / "libc.a").is_file() or not cxx.is_dir():
        raise RuntimeError("Prepare the pinned native C/C++ runtime first")
    base = ["--target=" + TRIPLE, "-O2", *FLAGS, "-ffreestanding", "-fno-builtin",
            "-fno-stack-protector", "-ffunction-sections", "-fdata-sections",
            "-include", str(ROOT / "sdk/compiler/target.h"),
            "-I" + str(ROOT / "sdk/compiler/include"),
            "-isystem", str(newlib / "targ-include"),
            "-isystem", str(ROOT / "build/newlib-4.6.0.20260123/newlib/libc/include")]
    cpp = ["-std=c++17", "-nostdinc++", "-femulated-tls", "-fexceptions",
           "-fignore-exceptions", "-frtti", "-I" + str(cxx)]
    commands = json.loads((WORK / "reference/compile_commands.json").read_text())
    objects = []
    seen = set()
    for row in commands:
        source = Path(row["file"])
        if "/amx/" in str(source):
            continue
        if ARCH == "x86_64" and "/arch/arm/" in str(source):
            source = Path(str(source).replace("/arch/arm/", "/arch/x86/"))
        if source in seen:
            continue
        seen.add(source)
        original = shlex.split(row["command"])
        includes = [arg for arg in original if arg.startswith("-I")]
        if source == SOURCE / "src/core/util.cpp":
            # This unused include has no corresponding ioctl call. Keep the
            # vendor tree immutable and remove it only in the native copy.
            content = source.read_text()
            if content.count("#include <sys/ioctl.h>") != 1:
                raise RuntimeError("Unreviewed image utility include boundary")
            native_source = output / "util-native.cpp"
            native_source.write_text(content.replace("#include <sys/ioctl.h>", ""))
            source = native_source
        definitions = [arg for arg in original if arg.startswith("-D") and
                       not any(skip in arg for skip in ("DARWIN", "XOPEN", "METAL", "__ARM_"))]
        if source == SOURCE / "thirdparty/zip.c":
            # Reading immutable model archives never needs to change file times.
            definitions.append("-DMINIZ_NO_TIME")
        name = hashlib.sha256(str(source).encode()).hexdigest()[:12] + "-" + source.name + ".o"
        obj = output / name
        language = cpp if source.suffix == ".cpp" else ["-std=gnu11"]
        compiler = LLVM / ("clang++" if source.suffix == ".cpp" else "clang")
        invocation = [str(compiler), *base, *language, *includes, *definitions,
                      "-c", str(source), "-o", str(obj)]
        # Reuse only an object with identical flags and unchanged source/header
        # inputs; vendor trees are pinned, while the native SDK remains mutable.
        headers = list((ROOT / "sdk/compiler/include").rglob("*.h"))
        headers += [ROOT / "sdk/compiler/target.h", Path(__file__)]
        digest = hashlib.sha256(json.dumps(invocation).encode() + source.read_bytes())
        for header in sorted(headers):
            digest.update(str(header).encode())
            digest.update(header.read_bytes())
        signature = obj.with_suffix(".sha256")
        expected = digest.hexdigest()
        if not (obj.is_file() and signature.is_file() and signature.read_text() == expected):
            print("Native image:", source.name, flush=True)
            subprocess.run(invocation, cwd=ROOT, check=True)
            signature.write_text(expected)
        objects.append(str(obj))
    (output / "objects.json").write_text(json.dumps(objects, indent=2) + "\n")
    # Object compilation is not a runnable backend. Linking, native callback
    # validation, and inference acceptance are separate gates.


if __name__ == "__main__":
    main()
