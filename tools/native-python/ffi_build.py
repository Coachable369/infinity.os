"""Pinned upstream libffi outbound ABI support, without executable closures."""
from pathlib import Path
import hashlib
import os
import re
import subprocess
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
VERSION = "3.4.8"
SHA256 = "bc9842a18898bfacb0ed1252c4febcc7e78fa139fd27fdc7a3e30d9d9356119b"


# ------------------------=
# FUNC: source_tree
# DESC: Retrieves the exact official libffi release into the repository and verifies its immutable source digest.
# ------------------=
def source_tree():
    parent = ROOT / "third_party/python"
    parent.mkdir(parents=True, exist_ok=True)
    archive = parent / f"libffi-{VERSION}.tar.gz"
    if not archive.exists():
        url = f"https://github.com/libffi/libffi/releases/download/v{VERSION}/{archive.name}"
        with urllib.request.urlopen(url, timeout=60) as response:
            body = response.read(2 * 1024 * 1024 + 1)
        if len(body) > 2 * 1024 * 1024 or hashlib.sha256(body).hexdigest() != SHA256:
            raise RuntimeError("Official libffi source checksum mismatch")
        temporary = archive.with_suffix(archive.suffix + ".partial")
        temporary.write_bytes(body)
        temporary.replace(archive)
    if hashlib.sha256(archive.read_bytes()).hexdigest() != SHA256:
        raise RuntimeError("Cached libffi source checksum mismatch")
    source = parent / f"libffi-{VERSION}"
    if not source.exists():
        with tarfile.open(archive) as bundle:
            bundle.extractall(parent, filter="data")
    return source


# ------------------------=
# FUNC: reject_body
# DESC: Replaces exactly one pinned upstream closure constructor with an explicit unsupported result while preserving all outbound ABI code.
# ------------------=
def reject_body(text, pattern, body):
    expression = re.compile(r"(" + pattern + r"\s*\([^;]*?\)\s*\n\{).*?\n\}", re.S)
    result, count = expression.subn(lambda match: match[1] + "\n" + body + "\n}", text)
    if count != 1:
        raise RuntimeError("Pinned upstream closure boundary changed: " + pattern)
    return result


# ------------------------=
# FUNC: python_support
# DESC: Builds genuine ARM64 or x86-64 libffi ABI marshaling and upstream ctypes, without mapping writable code or exposing callback trampolines.
# ------------------=
def python_support(arch, llvm, newlib, python_source):
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise RuntimeError("Native libffi builds require build-kit")
    if arch not in ("aarch64", "x86_64"):
        raise ValueError("Unsupported native libffi architecture")
    source = source_tree()
    out = ROOT / "build/native-python" / arch / "libffi"
    out.mkdir(parents=True, exist_ok=True)
    machine = source / "src" / ("aarch64" if arch == "aarch64" else "x86")
    header = (source / "include/ffi.h.in").read_text()
    for key, value in {"VERSION": VERSION, "TARGET": "AARCH64" if arch == "aarch64" else "X86_64",
                       "HAVE_LONG_DOUBLE": "1", "FFI_EXEC_TRAMPOLINE_TABLE": "0"}.items():
        header = header.replace("@" + key + "@", value)
    if re.search(r"@[A-Z_]+@", header):
        raise RuntimeError("Unresolved libffi header configuration")
    (out / "ffi.h").write_text(header)
    (out / "ffitarget.h").write_bytes((machine / "ffitarget.h").read_bytes())
    config = (source / "fficonfig.h.in").read_text()
    values = {name: "1" for name in (
        "HAVE_ALLOCA_H", "HAVE_AS_CFI_PSEUDO_OP", "HAVE_AS_X86_PCREL",
        "HAVE_HIDDEN_VISIBILITY_ATTRIBUTE", "HAVE_INTTYPES_H", "HAVE_LONG_DOUBLE",
        "HAVE_MEMCPY", "HAVE_RO_EH_FRAME", "HAVE_STDINT_H", "HAVE_STDIO_H",
        "HAVE_STDLIB_H", "HAVE_STRINGS_H", "HAVE_STRING_H", "HAVE_SYS_TYPES_H",
        "STDC_HEADERS")}
    values.update(SIZEOF_DOUBLE="8", SIZEOF_LONG_DOUBLE="16", SIZEOF_SIZE_T="8", EH_FRAME_FLAGS='"a"')
    for name, value in values.items():
        config = config.replace("#undef " + name + "\n", "#define " + name + " " + value + "\n")
    (out / "fficonfig.h").write_text(config)
    units = [source / "src/prep_cif.c", source / "src/types.c"]
    units += [machine / name for name in (("ffi.c", "sysv.S") if arch == "aarch64"
              else ("ffi64.c", "ffiw64.c", "unix64.S", "win64.S"))]
    includes = [out, source / "include", machine, newlib / "targ-include", newlib,
                ROOT / "build/newlib-4.6.0.20260123/newlib/libc/include"]
    flags = [f"--target={arch}-none-elf", "-O2", "-fPIE", "-ffunction-sections", "-fdata-sections",
             "-DNDEBUG", *["-I" + str(path) for path in includes]]
    flags += ["-mno-red-zone"] if arch == "x86_64" else ["-mno-outline-atomics", "-mstrict-align"]
    objects = []
    for path in units:
        if path.name in ("ffi.c", "ffi64.c", "ffiw64.c"):
            constructor = r"EFI64\(ffi_prep_closure_loc\)" if path.name == "ffiw64.c" else "ffi_prep_closure_loc"
            text = reject_body(path.read_text(), constructor, "  return FFI_BAD_ABI;")
            generated = out / path.name
            generated.write_text(text)
            path = generated
        obj = out / (path.stem + ".o")
        result = subprocess.run([str(llvm / "clang"), *flags, "-c", str(path), "-o", str(obj)],
                                cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        obj.with_suffix(".log").write_bytes(result.stdout)
        if result.returncode:
            raise RuntimeError(f"Native libffi compilation failed: {obj.with_suffix('.log')}")
        objects.append(obj)
    archive = out / "libffi.a"
    subprocess.run([str(llvm / "llvm-ar"), "rcs", str(archive), *map(str, objects)], check=True, cwd=ROOT)
    callbacks = python_source / "Modules/_ctypes/callbacks.c"
    rejected = reject_body(callbacks.read_text(), r"CThunkObject \*_ctypes_alloc_callback",
        '    PyErr_SetString(PyExc_NotImplementedError, "InfinityOS ctypes executable callbacks are unavailable; writable executable memory is forbidden");\n    return NULL;')
    bridge = out / "callbacks.c"
    bridge.write_text(rejected)
    return {"includes": [out, python_source / "Modules/_ctypes"], "archive": archive,
            "sources": [python_source / "Modules/_ctypes" / (name + ".c")
                        for name in ("_ctypes", "cfield", "callproc", "stgdict")] +
                       [bridge, ROOT / "sdk/python/ctypes_policy.c"],
            "license": source / "LICENSE"}
