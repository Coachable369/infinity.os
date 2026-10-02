#!/usr/bin/env python3
"""Cross-compile and privately namespace the pinned Whisper CPU engine."""
from pathlib import Path
import hashlib
import json
import shlex
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from voice_target import ARCH, TRIPLE, FLAGS

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "build/voice-whisper-src"
WORK = ROOT / "build/voice-whisper"
LLVM = Path("/opt/homebrew/opt/llvm/bin")


# ------------------------=
# FUNC: run
# DESC: Executes one checked native Whisper compilation or packaging operation.
# ------------------=
def run(*args):
    subprocess.run([str(arg) for arg in args], cwd=ROOT, check=True)


# ------------------------=
# FUNC: freestanding_whisper_source
# DESC: Removes unreachable host-file and debug-output paths while retaining buffer-backed model loading.
# ------------------=
def freestanding_whisper_source(source, output):
    content = source.read_text().replace("#include <fstream>\n", "")
    debug_start = '    // Dump log_mel_spectrogram\n    if (debug) {'
    debug_end = '    }\n\n    return true;'
    start = content.index(debug_start)
    end = content.index(debug_end, start)
    content = content[:start] + '    (void) debug;\n\n    return true;' + content[end + len(debug_end):]
    file_start = 'struct whisper_context * whisper_init_from_file_with_params_no_state('
    file_end = 'struct whisper_context * whisper_init_from_buffer_with_params_no_state('
    start = content.index(file_start)
    end = content.index(file_end, start)
    signature = content[start:content.index('{', start) + 1]
    content = content[:start] + signature + '\n    (void) path_model; (void) params; return nullptr;\n}\n\n' + content[end:]
    vad_start = 'struct whisper_vad_context * whisper_vad_init_from_file_with_params('
    vad_end = 'struct whisper_vad_context * whisper_vad_init_with_params('
    start = content.index(vad_start)
    end = content.index(vad_end, start)
    signature = content[start:content.index('{', start) + 1]
    content = content[:start] + signature + '\n    (void) path_model; (void) params; return nullptr;\n}\n\n' + content[end:]
    output.write_text(content)
    return output


# ------------------------=
# FUNC: freestanding_ggml_source
# DESC: Disables the host terminate-handler override while retaining GGML computation and lazy backend registration.
# ------------------=
def freestanding_ggml_source(source, output):
    content = source.read_text()
    start = content.index("static bool ggml_uncaught_exception_init = []{")
    end = content.index("}();", start) + 4
    content = content[:start] + "static bool ggml_uncaught_exception_init = false;" + content[end:]
    output.write_text(content)
    return output


# ------------------------=
# FUNC: main
# DESC: Builds upstream CPU objects for the selected architecture and namespaces every engine definition.
# ------------------=
def main():
    output = WORK / ARCH
    output.mkdir(parents=True, exist_ok=True)
    newlib = ROOT / "build" / ("voice-newlib-" + ARCH) / TRIPLE / "newlib"
    cxx_root = ROOT / "build/voice-kokoro" / ("cxx-" + ARCH)
    commands = json.loads((WORK / "reference/compile_commands.json").read_text())
    base = ["--target=" + TRIPLE, "-O3", *FLAGS, "-ffreestanding", "-fno-builtin",
            "-fno-stack-protector", "-ffunction-sections", "-fdata-sections",
            "-include", str(ROOT / "sdk/compiler/target.h"),
            "-I" + str(ROOT / "sdk/compiler/include"),
            "-isystem", str(newlib / "targ-include"),
            "-isystem", str(ROOT / "build/newlib-4.6.0.20260123/newlib/libc/include")]
    cxx = ["-std=c++17", "-nostdinc++", "-femulated-tls", "-fexceptions", "-fignore-exceptions",
           "-fno-rtti", "-include", "cstdlib", "-I" + str(cxx_root / "include/c++/v1")]
    selected = ("CMakeFiles/whisper.dir/", "CMakeFiles/ggml-base.dir/",
                "CMakeFiles/ggml-cpu.dir/", "CMakeFiles/ggml.dir/")
    objects = []
    seen = set()
    for row in commands:
        if not any(marker in row["command"] for marker in selected):
            continue
        source = Path(row["file"])
        if source.name == "ggml-backend-reg.cpp" or "/amx/" in str(source):
            continue
        if ARCH == "x86_64" and "/arch/arm/" in str(source):
            source = Path(str(source).replace("/arch/arm/", "/arch/x86/"))
        if source in seen or not source.exists():
            continue
        seen.add(source)
        original = shlex.split(row["command"])
        includes = [item for item in original if item.startswith("-I")]
        definitions = [item for item in original if item.startswith("-D") and
                       not any(skip in item for skip in ("DARWIN", "XOPEN", "METAL"))]
        definitions += ["-DGGML_USE_CPU", "-DGGML_SCHED_MAX_COPIES=4", "-DNDEBUG"]
        if source.name == "whisper.cpp":
            source = freestanding_whisper_source(source, output / "whisper-freestanding.cpp")
        elif source.name == "ggml.cpp":
            source = freestanding_ggml_source(source, output / "ggml-freestanding.cpp")
        name = hashlib.sha256(str(source).encode()).hexdigest()[:12] + "-" + source.name + ".o"
        obj = output / name
        compiler = LLVM / ("clang++" if source.suffix == ".cpp" else "clang")
        language = cxx if source.suffix == ".cpp" else ["-std=gnu11"]
        run(compiler, *base, *language, *includes, *definitions, "-c", source, "-o", obj)
        objects.append(obj)
    adapter = output / "entry.cpp.o"
    run(LLVM / "clang++", *base, *cxx,
        "-I" + str(SOURCE / "include"), "-I" + str(SOURCE / "ggml/include"),
        "-I" + str(SOURCE / "ggml/src/ggml-cpu"),
        "-c", ROOT / "tools/voice-whisper/entry.cpp", "-o", adapter)
    objects.append(adapter)
    model = ROOT / "model-cache/whisper-tiny.en.bin"
    license_file = SOURCE / "LICENSE"
    assembly = output / "model.S"
    assembly.write_text('.section .rodata.whisper_model,"a"\n.balign 4096\n.global whisper_model\n'
                        f'whisper_model:\n.incbin {json.dumps(str(model))}\n.global whisper_model_end\nwhisper_model_end:\n'
                        '.balign 8\n.global whisper_model_length\nwhisper_model_length:\n.quad whisper_model_end-whisper_model\n')
    with assembly.open("a") as target:
        target.write('.balign 8\n.global whisper_license\nwhisper_license:\n'
                     f'.incbin {json.dumps(str(license_file))}\n.global whisper_license_end\nwhisper_license_end:\n')
    for source in (assembly,):
        obj = output / (source.name + ".o")
        run(LLVM / "clang", *base, "-c", source, "-o", obj)
        objects.append(obj)
    engine = output / "engine.o"
    run("/opt/homebrew/opt/lld/bin/ld.lld", "-r", "--gc-sections", "--undefined=native_transcribe",
        "--undefined=native_prepare",
        "-o", engine, *objects)
    defined = set()
    undefined = set()
    for line in subprocess.check_output([str(LLVM / "llvm-nm"), "-g", str(engine)], text=True).splitlines():
        parts = line.split()
        if len(parts) >= 2 and parts[-2] == "U":
            undefined.add(parts[-1])
        elif len(parts) >= 3:
            defined.add(parts[-1])
    leaked = sorted(name for name in undefined - defined if name.startswith(("ggml_", "whisper_")))
    if leaked:
        raise RuntimeError("Incomplete private Whisper engine: " + ", ".join(leaked))
    private = output / "private-engine.o"
    run(LLVM / "llvm-objcopy", "--prefix-symbols=whisper_private_", engine, private)
    arguments = []
    for name in sorted(undefined - defined):
        arguments += ["--redefine-sym", f"whisper_private_{name}={name}"]
    run(LLVM / "llvm-objcopy", *arguments, private)
    (output / "objects.json").write_text(json.dumps([str(path) for path in objects], indent=2) + "\n")


if __name__ == "__main__":
    main()
