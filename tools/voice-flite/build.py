#!/usr/bin/env python3
"""Compile pinned, file/network-free Flite synthesis objects for InfinityOS."""
import argparse
import json
import re
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
REVISION = "e9e2e37c329dbe98bfeb27a1828ef9a71fa84f88"

# ------------------------=
# FUNC: main
# DESC: Builds a pinned compact voice, excluding upstream host audio, sockets, and dynamic voice loaders.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", choices=("aarch64", "x86_64", "host"), default="aarch64")
    args = parser.parse_args()
    source = ROOT / "build/voice-flite-src"
    if not source.exists():
        subprocess.run(["git", "clone", "--depth", "1", "--branch", "v2.2",
                        "https://github.com/festvox/flite.git", str(source)], check=True)
    revision = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
    if revision != REVISION:
        raise SystemExit("Flite source revision does not match the reviewed pin")
    if subprocess.check_output(["git", "-C", str(source), "status", "--porcelain", "--untracked-files=no"]):
        raise SystemExit("Flite source has modifications outside the reviewed native port")
    license_text = (source / "COPYING").read_text()
    if license_text.strip() != (ROOT / "tools/voice-flite/COPYING").read_text().strip():
        raise SystemExit("Flite license does not match the distributed notice")
    output = ROOT / "build/voice-flite" / args.target
    output.mkdir(parents=True, exist_ok=True)
    compiler = "/opt/homebrew/opt/llvm/bin/clang"
    flags = ["-O2", "-std=c11", "-ffunction-sections", "-fdata-sections", "-fno-stack-protector",
             "-fno-unwind-tables", "-fno-asynchronous-unwind-tables", "-fno-builtin",
             "-Wno-incompatible-pointer-types", "-Wno-unused-command-line-argument"]
    if args.target != "host":
        flags += [f"--target={args.target}-none-elf", "-ffreestanding", "-nostdlib"]
        if args.target == "aarch64":
            flags += ["-mno-outline-atomics", "-mstrict-align"]
        else:
            flags += ["-mno-red-zone"]
    flags += ["-I" + str(ROOT / "tools/voice-flite/include"),
              "-I" + str(source / "include"), "-I" + str(source / "lang/usenglish"),
              "-I" + str(source / "lang/cmulex")]
    flags += ["-include", str(ROOT / "tools/voice-flite/include/prefix.h")]
    sources = []
    directories = [source / "src" / d for d in ["hrg", "stats", "lexicon", "regex", "synth", "speech", "wavesynth", "utils"]]
    directories += [source / "lang" / d for d in ["cmu_us_kal", "cmulex", "usenglish"]]
    for directory in directories:
        makefile = (directory / "Makefile").read_text().replace("\\\n", " ")
        names = re.search(r"^SRCS\s*=\s*(.*)$", makefile, re.MULTILINE).group(1).split()
        sources.extend(directory / name for name in names)
    excluded = {"cst_alloc.c", "cst_error.c", "cst_args.c", "cst_socket.c", "cst_url.c",
                "cst_ssml.c", "cst_wave_io.c", "cst_track_io.c", "cst_wchar.c"}
    objects = []
    failures = []
    for path in sorted(sources):
        if path.name in excluded or path.name.startswith(("cst_mmap_", "cst_file_")):
            continue
        obj = output / ("_".join(path.relative_to(source).parts) + ".o")
        result = subprocess.run([compiler, *flags, "-c", str(path), "-o", str(obj)])
        if result.returncode:
            failures.append(str(path))
        objects.append(str(obj))
    if failures:
        raise SystemExit("Compilation failed: " + ", ".join(failures))
    port = output / "port.o"
    subprocess.run([compiler, *flags, "-c", str(ROOT / "tools/voice-flite/port.c"), "-o", str(port)], check=True)
    objects.append(str(port))
    jump = output / "jump.o"
    subprocess.run([compiler, *flags, "-c", str(ROOT / "tools/voice-flite/jump.S"), "-o", str(jump)], check=True)
    objects.append(str(jump))
    notice = output / "notice.c"
    notice.write_text("const char infinity_flite_license[] = " + json.dumps(license_text) + ";\n")
    notice_object = output / "notice.o"
    subprocess.run([compiler, *flags, "-c", str(notice), "-o", str(notice_object)], check=True)
    objects.append(str(notice_object))
    if args.target != "host":
        compact = output / "native-voice.o"
        subprocess.run(["/opt/homebrew/opt/lld/bin/ld.lld", "-r", "--gc-sections",
                        "--undefined=infinity_flite_synthesize", "--undefined=infinity_flite_clean",
                        "--undefined=infinity_flite_license", "-o", str(compact), *objects], check=True)
        objects = [str(compact)]
    archive = output / "libflite.a"
    archive.unlink(missing_ok=True)
    subprocess.run(["/opt/homebrew/opt/llvm/bin/llvm-ar", "rcs", str(archive), *objects], check=True)

if __name__ == "__main__":
    main()
