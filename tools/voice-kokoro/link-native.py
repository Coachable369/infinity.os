#!/usr/bin/env python3
"""Link immutable Kokoro resources and a private native target execution image."""
from pathlib import Path
import json
import subprocess
from reference import checksum, MODEL_SHA256
from notices import assemble
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from voice_target import ARCH, TRIPLE, FLAGS, syscall_aliases

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / "build/voice-kokoro"
LLVM = Path("/opt/homebrew/opt/llvm/bin")


# ------------------------=
# FUNC: run
# DESC: Rejects failed compilation and linking without publishing a stale native object.
# ------------------=
def run(*args):
    subprocess.run([str(arg) for arg in args], cwd=ROOT, check=True)


# ------------------------=
# FUNC: main
# DESC: Embeds exact read-only resources and links native C/C++ libraries without any host runtime.
# ------------------=
def main():
    output = WORK / ARCH
    output.mkdir(parents=True, exist_ok=True)
    newlib = ROOT / "build" / ("voice-newlib-" + ARCH) / TRIPLE / "newlib"
    source = ROOT / "build/kokopop-port-audit"
    deps = WORK / "reference/_deps"
    model = ROOT / "model-cache/kokoro-v1_0.gguf"
    if checksum(model) != MODEL_SHA256:
        raise RuntimeError("Unverified Kokoro model")
    data = deps / "espeak-build/espeak-ng-data"
    if not (data / "phontab").is_file():
        raise RuntimeError("Missing pinned phonemizer resources")
    notice = assemble(ROOT, output)
    resources = [("/kokoro.gguf", model), ("/licenses/kokoro-dependencies.txt", notice), ("/espeak-ng-data", None)]
    resources += [("/espeak-ng-data/" + str(p.relative_to(data)), p if p.is_file() else None)
                  for p in sorted(data.rglob("*"))]
    assembly = ['.section .rodata.native_models,"a"\n']
    definitions = ['#include <stddef.h>\nstruct model_file {const char *name;const unsigned char *data;size_t length;};\n']
    rows = []
    for i, (name, path) in enumerate(resources):
        if path is not None:
            assembly.append(f'.balign 4096\n.global native_model_{i}\nnative_model_{i}:\n.incbin {json.dumps(str(path))}\n')
            definitions.append(f'extern const unsigned char native_model_{i}[];\n')
            rows.append('{' + json.dumps(name) + f',native_model_{i},{path.stat().st_size}' + '}')
        else:
            rows.append('{' + json.dumps(name) + ',NULL,0}')
    definitions += ['const struct model_file native_model_files[]={' + ','.join(rows) + '};\n',
                    f'const size_t native_model_file_count={len(rows)};\n']
    (output / "resources.S").write_text(''.join(assembly))
    (output / "resources.c").write_text(''.join(definitions))
    flags = ["--target=" + TRIPLE, "-O2", *FLAGS, "-fno-builtin", "-ffreestanding",
             "-ffunction-sections", "-fdata-sections", "-fno-stack-protector",
             "-include", str(ROOT / "sdk/compiler/target.h"),
             "-I" + str(ROOT / "sdk/compiler/include"), "-I" + str(source / "include"),
             "-I" + str(source / "src"), "-I" + str(deps / "ggml-src/include"),
             "-isystem", str(newlib / "targ-include"),
             "-isystem", str(ROOT / "build/newlib-4.6.0.20260123/newlib/libc/include")]
    objects = []
    paths = [ROOT / "tools/voice-kokoro" / name for name in ("entry.cpp", "mapping.cpp", "registry.cpp", "port.c", "memory.c", "profile.c", "dot4.cpp")]
    paths += [ROOT / "sdk/compiler" / name for name in ("pthread.c", "serial_sync.c", "serial_tls.c")]
    paths += [output / "resources.S", output / "resources.c"]
    for path in paths:
        obj = output / (path.name + ".o")
        cxx = path.suffix == ".cpp"
        extra = ["-std=c++17", "-nostdinc++", "-fno-exceptions", "-fno-rtti", "-femulated-tls",
                 "-I" + str(WORK / ("cxx-" + ARCH) / "include/c++/v1")] if cxx else []
        run(LLVM / ("clang++" if cxx else "clang"), *flags, *extra, "-c", path, "-o", obj)
        objects.append(obj)
        if ARCH == "aarch64" and path.name == "dot4.cpp":
            fast = output / "dot4-fhm.o"
            run(LLVM / "clang++", *flags, *extra, "-march=armv8.2-a+fp16fml",
                "-DNATIVE_FAST_FHM", "-c", path, "-o", fast)
            objects.append(fast)
        if ARCH == "x86_64" and path.name == "dot4.cpp":
            fast = output / "dot4-f16c.o"
            # Compile the same arithmetic source with optional ISA flags. The
            # baseline entry checks CPUID and XCR0 before entering this object.
            run(LLVM / "clang++", *flags, *extra, "-mavx", "-mf16c",
                "-DNATIVE_FAST_F16C", "-c", path, "-o", fast)
            objects.append(fast)
    string_source = ROOT / "build/newlib-4.6.0.20260123/newlib/libc/string"
    for name in ("strlen", "strnlen", "strcmp", "strncmp", "strcpy", "strncpy", "stpcpy", "stpncpy",
                 "strchr", "strrchr", "strchrnul", "memcmp", "memchr", "memrchr", "strcat", "strncat"):
        obj = output / ("safe-" + name + ".o")
        run(LLVM / "clang", *flags, "-Os", "-DPREFER_SIZE_OVER_SPEED", "-c", string_source / (name + ".c"), "-o", obj)
        objects.append(obj)
    native = output / "native.o"
    whisper = ROOT / "build/voice-whisper" / ARCH / "private-engine.o"
    if not whisper.is_file():
        raise RuntimeError("Missing native Whisper engine")
    run("/opt/homebrew/opt/lld/bin/ld.lld", "-r", "--gc-sections", "--undefined=native_synthesize", "--undefined=native_recognize", "--undefined=native_prepare_recognition", "--undefined=native_diagnostics",
        "--undefined=native_profile_read", "--undefined=native_verify_im2col1d",
        "-T", ROOT / "tools/voice-kokoro/private.ld",
        *syscall_aliases("close", "fstat", "getpid", "gettimeofday", "isatty", "kill",
                         "lseek", "open", "read", "sbrk", "stat", "unlink", "write"),
        "--wrap=_malloc_r", "--wrap=_calloc_r", "--wrap=_realloc_r", "--wrap=_free_r",
        "-o", native, *objects, whisper, "--start-group", output / "libkokoro-engine.a",
        WORK / ("cxx-" + ARCH) / "lib/libc++.a", WORK / ("cxx-" + ARCH) / "lib/libc++abi.a",
        newlib / "libm.a", newlib / "libc.a", "--end-group")
    run(LLVM / "llvm-nm", "--undefined-only", native)
    private = output / "private-native.o"
    run(LLVM / "llvm-objcopy", "--prefix-symbols=infinity_kokoro_", native, private)
    run(LLVM / "llvm-objcopy",
        "--redefine-sym", "infinity_kokoro_infinity_speech_clock_ns=infinity_speech_clock_ns",
        "--redefine-sym", "infinity_kokoro___extenddftf2=__extenddftf2",
        "--redefine-sym", "infinity_kokoro___trunctfdf2=__trunctfdf2", private)


if __name__ == "__main__":
    main()
