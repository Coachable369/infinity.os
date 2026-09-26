#!/usr/bin/env python3
"""Cross-compile the pinned Kokoro CPU engine; no host objects enter the archive."""
from pathlib import Path
import hashlib
import json
import os
import shlex
import subprocess

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / "build/voice-kokoro"
LLVM = Path("/opt/homebrew/opt/llvm/bin")


# ------------------------=
# FUNC: compile_signature
# DESC: Invalidates native objects when their command, source or included headers change.
# ------------------=
def compile_signature(arguments, source, dependencies):
    digest = hashlib.sha256(json.dumps(arguments).encode())
    paths = {source}
    if dependencies.exists():
        content = dependencies.read_text().replace("\\\n", "")
        paths.update(Path(name) for name in shlex.split(content.split(":", 1)[1]))
    for path in sorted(paths):
        digest.update(str(path).encode())
        if not path.is_file():
            return None
        digest.update(path.read_bytes())
    return digest.hexdigest()


# ------------------------=
# FUNC: main
# DESC: Reuses upstream source selection but replaces all platform flags with the native ARM64 ABI.
# ------------------=
def main():
    output = WORK / "aarch64"
    output.mkdir(parents=True, exist_ok=True)
    include = ROOT / "tools/voice-kokoro/include"
    base = ["--target=aarch64-none-elf", "-O2", "-mstrict-align", "-ffreestanding",
            "-fno-stack-protector", "-ffunction-sections", "-fdata-sections", "-fno-builtin",
            "-include", str(ROOT / "sdk/compiler/target.h"), "-I" + str(include),
            "-I" + str(ROOT / "sdk/compiler/include"),
            "-isystem", str(ROOT / "build/voice-newlib-aarch64/aarch64-none-elf/newlib/targ-include"),
            "-isystem", str(ROOT / "build/newlib-4.6.0.20260123/newlib/libc/include")]
    cxx = ["-std=c++17", "-nostdinc++", "-fno-exceptions", "-fignore-exceptions", "-fno-rtti", "-femulated-tls", "-include", "cstdlib",
           "-I" + str(WORK / "cxx-aarch64/include/c++/v1")]
    commands = json.loads((WORK / "reference/compile_commands.json").read_text())
    objects = []
    failures = []
    seen = set()
    for row in commands:
        source = Path(row["file"])
        command = shlex.split(row["command"])
        if not any(part in row["command"] for part in (
                "CMakeFiles/kokopop.dir/", "CMakeFiles/ggml-base.dir/", "CMakeFiles/ggml-cpu.dir/",
                "CMakeFiles/ggml.dir/", "CMakeFiles/espeak-ng.dir/", "CMakeFiles/ucd.dir/", "CMakeFiles/yyjson.dir/")):
            continue
        if source in seen or source.suffix not in (".c", ".cpp"):
            continue
        seen.add(source)
        if "CMakeFiles/kokopop.dir/" in row["command"]:
            selected = {"backend_names.cpp", "error.cpp", "replace.cpp", "utf8.cpp", "model.cpp",
                        "arch.cpp", "registry.cpp", "cpu.cpp", "phonemizer.cpp", "zh_g2p.cpp",
                        "synth.cpp", "istft.cpp"}
            if source.name not in selected and "/arch/kokoro/" not in str(source):
                continue
        if source.name in ("ggml-backend-reg.cpp", "file_mapping.cpp") or "/playback/" in str(source):
            continue
        flags = ["-I" + str(source.parent)] + [item for item in command if item.startswith("-I")]
        flags += ["-DGGML_USE_CPU", "-DGGML_USE_CPU_REPACK", "-DGGML_SCHED_MAX_COPIES=4",
                  '-DGGML_VERSION="0.22.0"', '-DGGML_COMMIT="36da5713"',
                  '-DKOKOPOP_ESPEAK_BUILD_DATA_DIR="/espeak-ng-data"',
                  '-DKOKOPOP_ESPEAK_INSTALL_DATA_DIR="/espeak-ng-data"',
                  '-DPATH_ESPEAK_DATA="/espeak-ng-data"', "-DNDEBUG"]
        for item in command:
            if item.startswith("-D") and any(name in item for name in ("LIBESPEAK_NG_EXPORT", "HAVE_CONFIG_H")):
                flags.append(item)
        replacement = ROOT / "tools/voice-kokoro/overrides" / source.name
        if replacement.exists():
            source = replacement
        if source.name == "ggml-cpu.c" and os.environ.get("INFINITY_KOKORO_PROFILE") == "1":
            original = source.read_text()
            begin = "        const int n_fused = ggml_cpu_try_fuse_ops(cgraph, node_n, &params, cplan);"
            end = "        if (state->ith == 0 && cplan->abort_callback &&"
            if original.count(begin) != 1 or original.count(end) != 1:
                raise RuntimeError("Unreviewed GGML profiling boundary")
            original = original.replace(begin, "        const uint64_t native_start = native_profile_clock();\n" + begin)
            original = original.replace(end, "        if (state->ith == 0) native_profile_record(node->op, native_start);\n\n" + end)
            source = output / "profiled-ggml-cpu.c"
            source.write_text('#include <stdint.h>\nextern uint64_t native_profile_clock(void);\n'
                              'extern void native_profile_record(unsigned, uint64_t);\n' + original)
        if source.name == "cpu.cpp" and "CMakeFiles/kokopop.dir/" in row["command"]:
            original = source.read_text()
            anchor = "            ggml_backend_cpu_set_n_threads(backend_, std::max<int32_t>(1, n_threads));"
            if original.count(anchor) != 1:
                raise RuntimeError("Unreviewed CPU cancellation boundary")
            capacity = "            backend_graph_capacity(graph));"
            if original.count(capacity) != 1:
                raise RuntimeError("Unreviewed CPU graph capacity boundary")
            original = original.replace(capacity,
                "            static_cast<size_t>(graph->n_nodes) + static_cast<size_t>(graph->n_leafs) + 1024);")
            flags.append("-I" + str(WORK / "reference/_deps/ggml-src/src"))
            source = output / "cpu.cpp"
            source.write_text('#include <ggml-impl.h>\nextern "C" bool native_abort_callback(void *);\n' + original.replace(anchor,
                anchor + "\n            ggml_backend_cpu_set_abort_callback(backend_, native_abort_callback, nullptr);"))
        if source.name == "audio_utils.cpp":
            original = source.read_text()
            start = "    try {\n        mem = arena.data(mem_size);\n    } catch (const std::bad_alloc &) {"
            end = '        error = std::string("failed to allocate ggml ") + label + " scratch memory";\n        return nullptr;\n    }'
            if original.count(start) != 1 or original.count(end) != 1:
                raise RuntimeError("Unreviewed scratch allocation boundary")
            source = output / "audio_utils.cpp"
            source.write_text(original.replace(start, "    mem = arena.data(mem_size);\n    if (mem == nullptr) {"))
        name = hashlib.sha256(str(source).encode()).hexdigest()[:12] + "-" + source.name + ".o"
        obj = output / name
        dependencies = obj.with_suffix(".d")
        compiler = LLVM / ("clang++" if source.suffix == ".cpp" else "clang")
        # GGUF catches allocation/length failures. The private native fatal
        # boundary quarantines these instead of unwinding across the Rust ABI.
        exception_flags = ["-fexceptions", "-fignore-exceptions", "-include", "cerrno"] if source.name == "gguf.cpp" else []
        arguments = [str(compiler), *base, *(cxx if source.suffix == ".cpp" else ["-std=gnu11"]),
                     *flags, *exception_flags, "-MD", "-MF", str(dependencies), "-c", str(source), "-o", str(obj)]
        signature = compile_signature(arguments, source, dependencies)
        stamp = obj.with_suffix(".stamp")
        if not (obj.exists() and dependencies.exists() and stamp.exists() and signature and stamp.read_text() == signature):
            print(source, flush=True)
            result = subprocess.run(arguments)
            if result.returncode:
                failures.append(str(source))
                continue
            stamp.write_text(compile_signature(arguments, source, dependencies))
        objects.append(str(obj))
    if failures:
        raise RuntimeError("Native compile failures: " + ", ".join(failures))
    (output / "objects.json").write_text(json.dumps(objects, indent=2) + "\n")
    archive = output / "libkokoro-engine.a"
    archive.unlink(missing_ok=True)
    subprocess.run([str(LLVM / "llvm-ar"), "rcs", str(archive), *objects], check=True)


if __name__ == "__main__":
    main()
