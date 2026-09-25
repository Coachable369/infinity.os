"""Link the experimental ARM64 recognizer against private native C libraries.

This is not an installer integration or a passing speech acceptance test.
The libraries are cross-built, never run as a host recognition service.
"""
from pathlib import Path
import subprocess
import json

ROOT = Path(__file__).resolve().parents[2]

# ------------------------=
# FUNC: main
# DESC: Packages exact immutable model resources and privately namespaces the bounded recognition provider.
# ------------------=
def main():
    source = ROOT / "build/voice-pocketsphinx-src"
    revision = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
    assert revision == "511126b492dcb267cf30d49d631946d7b61a9530"
    output = ROOT / "build/voice-pocketsphinx-arm"
    llvm = Path("/opt/homebrew/opt/llvm/bin")
    newlib = ROOT / "build/voice-newlib-aarch64/aarch64-none-elf/newlib"
    flags = ["--target=aarch64-none-elf", "-O2", "-mstrict-align", "-fno-builtin",
             "-ffunction-sections", "-fdata-sections", "-fno-stack-protector",
             "-I" + str(source / "include"), "-I" + str(output / "include"),
             "-I" + str(newlib / "targ-include"),
             "-I" + str(ROOT / "build/newlib-4.6.0.20260123/newlib/libc/include")]
    resources = sorted(p for p in (source / "model/en-us").rglob("*") if p.is_file())
    assembly = [".section .rodata.native_models,\"a\"\n"]
    definitions = ["#include <stddef.h>\nstruct model_file {const char *name; const unsigned char *data; size_t length;};\n"]
    rows = []
    for i, path in enumerate(resources):
        assembly += [f".balign 4096\n.global native_model_{i}\nnative_model_{i}:\n.incbin {json.dumps(str(path))}\n"]
        definitions += [f"extern const unsigned char native_model_{i}[];\n"]
        name = "/model/" + str(path.relative_to(source / "model"))
        rows.append("{" + json.dumps(name) + f",native_model_{i},{path.stat().st_size}" + "}")
    definitions += ["const struct model_file native_model_files[]={" + ",".join(rows) + "};\n",
                    f"const size_t native_model_file_count={len(rows)};\n"]
    # Keep complete third-party notices in the same self-contained native object.
    for index, notice in enumerate([source / "LICENSE", ROOT / "build/newlib-4.6.0.20260123/COPYING.NEWLIB"]):
        assembly += [f".global native_notice_{index}\nnative_notice_{index}:\n.incbin {json.dumps(str(notice))}\n.byte 0\n"]
    (output / "models.S").write_text("".join(assembly))
    (output / "models.c").write_text("".join(definitions))
    objects = []
    for path in [ROOT / "tools/voice-pocketsphinx/port.c", output / "models.S", output / "models.c"]:
        obj = output / (path.name + ".o")
        subprocess.run([str(llvm / "clang"), *flags, "-c", str(path), "-o", str(obj)], check=True)
        objects.append(str(obj))
    # ARM's optimized newlib assembly assumes normal memory and performs
    # unaligned vector accesses. Use the upstream size-oriented C variants in
    # InfinityOS's early MMU-off environment; keep them privately namespaced.
    string_source = ROOT / "build/newlib-4.6.0.20260123/newlib/libc/string"
    for name in ["strlen", "strnlen", "strcmp", "strncmp", "strcpy", "strncpy", "stpcpy", "stpncpy",
                 "strchr", "strrchr", "strchrnul", "memcmp", "memchr", "memrchr", "strcat", "strncat"]:
        path = string_source / (name + ".c")
        obj = output / ("safe-" + name + ".o")
        subprocess.run([str(llvm / "clang"), *flags, "-Os", "-DPREFER_SIZE_OVER_SPEED", "-c", str(path), "-o", str(obj)], check=True)
        objects.append(str(obj))
    native = output / "native.o"
    subprocess.run(["/opt/homebrew/opt/lld/bin/ld.lld", "-r", "--gc-sections", "--undefined=native_recognize", "--undefined=native_memory_state",
                    "--wrap=_malloc_r", "--wrap=_calloc_r", "--wrap=_realloc_r", "--wrap=_free_r",
                    "-o", str(native), *objects, "--start-group", str(output / "libpocketsphinx.a"),
                    str(newlib / "libm.a"), str(newlib / "libc.a"), "--end-group"], check=True)
    subprocess.run([str(llvm / "llvm-objcopy"), "--prefix-symbols=infinity_stt_", str(native), str(output / "private-native.o")], check=True)
    subprocess.run([str(llvm / "llvm-objcopy"),
                    "--redefine-sym", "infinity_stt___extenddftf2=__extenddftf2",
                    "--redefine-sym", "infinity_stt___trunctfdf2=__trunctfdf2",
                    str(output / "private-native.o")], check=True)
    subprocess.run([str(llvm / "llvm-nm"), "--undefined-only", str(output / "private-native.o")], check=True)

if __name__ == "__main__":
    main()
