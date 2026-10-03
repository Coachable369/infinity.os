#!/usr/bin/env python3
"""Cross-compile the pinned Kokoro CPU engine; no host objects enter the archive."""
from pathlib import Path
import hashlib
import json
import os
import shlex
import subprocess
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from voice_target import ARCH, TRIPLE, FLAGS

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
# DESC: Reuses upstream sources with the selected native target ABI and CPU kernels.
# ------------------=
def main():
    output = WORK / ARCH
    output.mkdir(parents=True, exist_ok=True)
    include = ROOT / "tools/voice-kokoro/include"
    base = ["--target=" + TRIPLE, "-O2", *FLAGS, "-ffreestanding",
            "-fno-stack-protector", "-ffunction-sections", "-fdata-sections", "-fno-builtin",
            "-include", str(ROOT / "sdk/compiler/target.h"), "-I" + str(include),
            "-I" + str(ROOT / "sdk/compiler/include"),
            "-isystem", str(ROOT / "build" / ("voice-newlib-" + ARCH) / TRIPLE / "newlib/targ-include"),
            "-isystem", str(ROOT / "build/newlib-4.6.0.20260123/newlib/libc/include")]
    cxx = ["-std=c++17", "-nostdinc++", "-fno-exceptions", "-fignore-exceptions", "-fno-rtti", "-femulated-tls", "-include", "cstdlib",
           "-I" + str(WORK / ("cxx-" + ARCH) / "include/c++/v1")]
    commands = json.loads((WORK / "reference/compile_commands.json").read_text())
    objects = []
    failures = []
    seen = set()
    for row in commands:
        source = Path(row["file"])
        if ARCH == "x86_64" and "/ggml-cpu/arch/arm/" in str(source):
            source = Path(str(source).replace("/arch/arm/", "/arch/x86/"))
            if not source.exists():
                raise RuntimeError("Missing x86 CPU source counterpart")
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
        # Native profiling identifies matrix products and im2col as the dominant
        # synthesis cost. Optimize these loop kernels without fast-math or any
        # change to their floating-point contract; PCM parity gates the result.
        if source.name in ("ops.cpp", "ggml-cpu.c"):
            flags.append("-O3")
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
        if source.name == "ops.cpp":
            original = source.read_text()
            anchor = "static void ggml_compute_forward_im2col_f16("
            if original.count(anchor) != 1:
                raise RuntimeError("Unreviewed one-dimensional convolution boundary")
            original = original.replace(anchor,
                '#include "' + str(ROOT / "tools/voice-kokoro/im2col1d.h") + '"\n\n' + anchor)
            dispatch = "                ggml_compute_forward_im2col_f16(params, dst);"
            if original.count(dispatch) != 1:
                raise RuntimeError("Unreviewed half convolution dispatch boundary")
            original = original.replace(dispatch,
                "                if (!native_im2col1d_f16(params, dst))\n" + dispatch)
            source = output / "ops.cpp"
            source.write_text(original + '\n#include "' + str(ROOT / "tools/voice-kokoro/im2col1d-test.h") + '"\n')
        if source.name == "ggml-cpu.c":
            original = source.read_text()
            anchor = "                    vec_dot(ne00, &tmp[ir0 - iir0], (num_rows_per_vec_dot > 1 ? 16 : 0),"
            if original.count(anchor) != 1:
                raise RuntimeError("Unreviewed matrix tile boundary")
            original = original.replace(anchor,
                "                    if (num_rows_per_vec_dot == 1 && ir0 + 4 <= MIN(iir0 + blck_0, ir0_end) &&\n"
                "                        native_dot4(type, ne00, &tmp[ir0 - iir0], src0_row + ir0 * nb01, nb01, src1_col)) {\n"
                "                        ir0 += 3; continue;\n                    }\n" + anchor)
            source = output / "ggml-cpu.c"
            source.write_text('#include <stddef.h>\nextern int native_dot4(int, int, float *, const void *, size_t, const void *);\n' + original)
        if source.name == "vec.cpp" and ARCH == "x86_64":
            # The upstream SSE path only needs SSE3 for horizontal addition.
            # Supply its exact pairwise operation using baseline SSE2 shuffles.
            mappings = (source.parent / "simd-mappings.h").read_text()
            anchor = "#elif defined(__SSE3__)"
            if mappings.count(anchor) != 1:
                raise RuntimeError("Unreviewed SSE mapping boundary")
            mappings = mappings.replace(anchor, "#elif defined(__SSE2__)")
            mappings = mappings.replace("_mm_hadd_ps(", "native_sse2_hadd(")
            preamble = ('#include <xmmintrin.h>\n'
                '// ------------------------=\n// FUNC: native_sse2_hadd\n'
                '// DESC: Preserves SSE3 pairwise addition order on every baseline x86-64 CPU.\n'
                '// ------------------=\n'
                'static inline __m128 native_sse2_hadd(__m128 a, __m128 b) {\n'
                ' return _mm_add_ps(_mm_shuffle_ps(a,b,_MM_SHUFFLE(2,0,2,0)),\n'
                '                   _mm_shuffle_ps(a,b,_MM_SHUFFLE(3,1,3,1)));\n}\n')
            (output / "simd-mappings.h").write_text(preamble + mappings)
            (output / "vec.h").write_text((source.parent / "vec.h").read_text())
            original = source.read_text()
            source = output / "vec.cpp"
            source.write_text(original)
        if source.name == "vec.cpp" and ARCH == "aarch64":
            original = source.read_text()
            for kind, alignment in (("f16", 8), ("f32", 16)):
                name = "ggml_vec_dot_" + kind
                start = original.index("void " + name + "(")
                body = original.index("{", start)
                end = original.index("\n}\n", body) + 3
                signature = original[start:body]
                implementation = signature.replace("void " + name, "static inline __attribute__((always_inline)) void native_" + name)
                implementation = ("// ------------------------=\n// FUNC: native_" + name +
                    "\n// DESC: Specializes the upstream dot product for checked row alignment without changing arithmetic.\n// ------------------=\n"
                    "template<bool Aligned>\n" + implementation + "{\n"
                    f"    if constexpr (Aligned) {{ x = (__typeof__(x))__builtin_assume_aligned(x, {alignment}); "
                    f"y = (__typeof__(y))__builtin_assume_aligned(y, {alignment}); }}\n" + original[body+1:end])
                wrapper = ("\n// ------------------------=\n// FUNC: " + name +
                    "\n// DESC: Uses aligned vector loads only when both pointers satisfy the contract, retaining the general fallback.\n// ------------------=\n" +
                    signature + "{\n" + f"    if ((((uintptr_t)x | (uintptr_t)y) & {alignment-1}) == 0) " +
                    f"native_{name}<true>(n, s, bs, x, bx, y, by, nrc);\n" +
                    f"    else native_{name}<false>(n, s, bs, x, bx, y, by, nrc);\n}}\n")
                original = original[:start] + implementation + wrapper + original[end:]
            source = output / "aligned-vec.cpp"
            source.write_text(original)
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
        if source.name == "ggml-backend-meta.cpp":
            original = source.read_text()
            anchor = "bool ggml_backend_buffer_is_meta(ggml_backend_buffer_t buf) {"
            if original.count(anchor) != 1:
                raise RuntimeError("Unreviewed private backend pointer boundary")
            # Every native backend buffer object is constructed with C++ new
            # in ggml_backend_buffer_init. Do not dereference out-of-arena
            # objects: preserve the exact fault address and quarantine instead
            # of entering the firmware exception loop indefinitely.
            original = original.replace(anchor,
                '// ------------------------=\n// FUNC: ggml_backend_buffer_is_meta\n'
                '// DESC: Checks private object ownership before inspecting the backend buffer interface.\n'
                '// ------------------=\n' + anchor +
                '\n    if (buf != nullptr) native_require_heap_extent(buf, sizeof(*buf));')
            source = output / "ggml-backend-meta.cpp"
            source.write_text('#include <stddef.h>\nextern "C" void native_require_heap_extent(const void *, size_t);\n' + original)
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
            accounting = "        (void) ctx;"
            if original.count(accounting) != 1:
                raise RuntimeError("Unreviewed CPU context accounting boundary")
            original = original.replace(accounting,
                "        native_context_record(ggml_get_mem_size(ctx), ggml_used_mem(ctx));")
            flags.append("-I" + str(WORK / "reference/_deps/ggml-src/src"))
            source = output / "cpu.cpp"
            source.write_text('#include <ggml-impl.h>\nextern "C" bool native_abort_callback(void *);\n'
                'extern "C" void native_context_record(size_t, size_t);\n' + original.replace(anchor,
                anchor + "\n            ggml_backend_cpu_set_abort_callback(backend_, native_abort_callback, nullptr);"))
        if source.name == "kokoro_arch.cpp":
            original = source.read_text()
            first = "size_t kokoro_generation_context_bytes("
            last = "// ---------------------------------------------------------------------------\n// KokoroArch"
            if original.count(first) != 1 or original.count(last) != 1:
                raise RuntimeError("Unreviewed Kokoro metadata sizing boundary")
            begin = original.index(first)
            end = original.index(last, begin)
            # All three native CPU graph contexts use no_alloc=true. Tensor
            # storage belongs to the backend allocator, not these arenas. The
            # upstream data-sized reservations retained >135 MiB here and
            # vector growth exhausted the shared ASR/TTS arena after two jobs.
            replacement = '''// ------------------------=
// FUNC: kokoro_generation_context_bytes
// DESC: Reserves bounded graph metadata; tensor data is separately allocated by the CPU scheduler.
// ------------------=
size_t kokoro_generation_context_bytes(const Backend & backend,
                                       int64_t total_frames, int64_t n_tokens) {
    const size_t nodes = generation_graph_size(total_frames, n_tokens);
    return backend.graph_context_bytes(nodes, nodes);
}

// ------------------------=
// FUNC: kokoro_generator_context_bytes
// DESC: Uses the graph's conservative object bound without duplicating its large convolution data buffers.
// ------------------=
size_t kokoro_generator_context_bytes(const Backend & backend,
                                      int64_t decoder_len) {
    const size_t nodes = generator_graph_size(decoder_len);
    return backend.graph_context_bytes(nodes, nodes);
}

'''
            source = output / "kokoro_arch.cpp"
            source.write_text(original[:begin] + replacement + original[end:])
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
