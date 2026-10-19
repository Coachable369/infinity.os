#!/usr/bin/env python3
"""Pinned CPython native port; host Python only prepares immutable frozen code."""
from pathlib import Path
import concurrent.futures
import hashlib
import json
import marshal
import os
import re
import shutil
import subprocess
import sys
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
VERSION = "3.13.16"
SHA = "f4b1bfb3c79b5bb11b8d228a12504163b4c0dab4d679828d8f5f26b6cb6ab35d"
LLVM = Path(os.environ.get("LLVM_BIN", "/opt/homebrew/opt/llvm/bin"))
LLVM_REVISION = "ea7d852a70e8bdfaf601d6626a760f9771b2c4b4"

# ------------------------=
# FUNC: arithmetic_sources
# DESC: Retrieves only the pinned LLVM compiler-runtime conversion helper and its local headers.
# ------------------=
def arithmetic_sources():
    destination = ROOT / "third_party/python/compiler-rt" / LLVM_REVISION
    destination.mkdir(parents=True, exist_ok=True)
    pending = ["trunctfdf2.c"]
    visited = set()
    while pending:
        name = pending.pop()
        if name in visited:
            continue
        if not re.fullmatch(r"[A-Za-z0-9_.-]+", name):
            raise ValueError("Invalid compiler-runtime include")
        visited.add(name)
        path = destination / name
        if not path.exists():
            url = f"https://raw.githubusercontent.com/llvm/llvm-project/{LLVM_REVISION}/compiler-rt/lib/builtins/{name}"
            with urllib.request.urlopen(url, timeout=60) as response:
                data = response.read()
            temporary = path.with_suffix(path.suffix + ".partial")
            temporary.write_bytes(data)
            temporary.replace(path)
        pending += re.findall(r'^\s*#\s*include\s+"([^"/]+)"', path.read_text(), re.M)
    license_path = destination / "LICENSE.TXT"
    if not license_path.exists():
        with urllib.request.urlopen(f"https://raw.githubusercontent.com/llvm/llvm-project/{LLVM_REVISION}/compiler-rt/LICENSE.TXT", timeout=60) as response:
            license_path.write_bytes(response.read())
    return [destination / "trunctfdf2.c"], license_path

# ------------------------=
# FUNC: run
# DESC: Executes a checked build subprocess inside the repository.
# ------------------=
def run(*args, **kwargs):
    kwargs.setdefault("cwd", ROOT)
    subprocess.run([str(x) for x in args], check=True, **kwargs)

# ------------------------=
# FUNC: record
# DESC: Retains port diagnostics outside disposable build scratch without claiming runtime acceptance.
# ------------------=
def record(arch, stage, contents):
    destination = ROOT / "builds/validation/native-python" / arch
    destination.mkdir(parents=True, exist_ok=True)
    (destination / f"{stage}.log").write_bytes(contents)

# ------------------------=
# FUNC: main
# DESC: Freezes the supported library and builds one self-contained native CPython executable.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/native-python/build.py ARCH")
    if sys.version_info[:2] != (3, 13):
        raise SystemExit("Frozen bytecode generation requires host Python 3.13")
    arch = sys.argv[1] if len(sys.argv) > 1 else "aarch64"
    if arch not in ("aarch64", "x86_64"):
        raise SystemExit("Unsupported architecture")
    archive = ROOT / f"third_party/python/Python-{VERSION}.tar.xz"
    archive.parent.mkdir(parents=True, exist_ok=True)
    if not archive.exists():
        with urllib.request.urlopen(f"https://www.python.org/ftp/python/{VERSION}/Python-{VERSION}.tar.xz", timeout=60) as response:
            archive.write_bytes(response.read())
    if hashlib.sha256(archive.read_bytes()).hexdigest() != SHA:
        raise SystemExit("CPython source checksum mismatch")
    source = archive.parent / f"Python-{VERSION}"
    if not source.exists():
        with tarfile.open(archive) as bundle:
            bundle.extractall(archive.parent, filter="data")
    port_patch = ROOT / "sdk/python/cpython-infinity.patch"
    patched = subprocess.run(["patch", "-f", "--dry-run", "-R", "-p1", "-d", str(source), "-i", str(port_patch)],
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if patched.returncode:
        run("patch", "-f", "-p1", "-d", source, "-i", port_patch)
    out = ROOT / "build/native-python" / arch
    out.mkdir(parents=True, exist_ok=True)
    (out / "pyconfig.h").write_bytes((ROOT / "sdk/python/pyconfig.h").read_bytes())
    newlib = ROOT / f"build/voice-newlib-{arch}/{arch}-none-elf/newlib"
    if not (newlib / "libc.a").exists():
        env = dict(os.environ, INFINITY_VOICE_TARGET=arch)
        run(sys.executable, "tools/voice-native-runtime/newlib.py", env=env)
    if arch == "x86_64":
        # Existing speech archives use low-address absolute relocations. Native
        # user processes require position-independent code above 4 GiB.
        native_lib = out / "newlib-pie"
        native_lib.mkdir(exist_ok=True)
        configuration = native_lib / "configure.log"
        with configuration.open("wb") as log:
            if not (native_lib / "Makefile").exists():
                run(ROOT / "build/newlib-4.6.0.20260123/configure", f"--target={arch}-none-elf",
                    "--disable-newlib-supplied-syscalls", "--disable-libgloss", "--disable-multilib",
                    "--disable-newlib-multithread", f"CC_FOR_TARGET={LLVM / 'clang'} --target={arch}-none-elf",
                    f"AR_FOR_TARGET={LLVM / 'llvm-ar'}", f"RANLIB_FOR_TARGET={LLVM / 'llvm-ranlib'}",
                    "CFLAGS_FOR_TARGET=-O2 -fPIE -mno-red-zone -ffunction-sections -fdata-sections",
                    cwd=native_lib, stdout=log, stderr=subprocess.STDOUT)
        with (native_lib / "build.log").open("wb") as log:
            run("make", "-j4", "all-target-newlib", cwd=native_lib, stdout=log, stderr=subprocess.STDOUT)
        newlib = native_lib / f"{arch}-none-elf/newlib"
    includes = [out, ROOT / "sdk/python/include", source / "Include", source / "Include/internal", source,
                ROOT / "sdk/compiler/include", newlib / "targ-include", newlib,
                ROOT / "build/newlib-4.6.0.20260123/newlib/libc/include"]
    flags = [f"--target={arch}-none-elf", "-std=c11", "-O2", "-fPIE", "-ffunction-sections", "-fdata-sections",
             "-DPy_BUILD_CORE", "-DNDEBUG", "-include", str(ROOT / "sdk/compiler/target.h"),
             '-DPREFIX="/system/python"', '-DEXEC_PREFIX="/system/python"', '-DVERSION="3.13"',
             '-DVPATH=""', '-DPLATLIBDIR="lib"', '-DPYTHONPATH=""']
    flags += ["-mno-red-zone"] if arch == "x86_64" else ["-mno-outline-atomics", "-mstrict-align"]
    flags += [f"-I{x}" for x in includes]
    flags += ["-idirafter", str(ROOT / "sdk/c/include")]
    extension_flags = [flag for flag in flags if flag not in ("-DPy_BUILD_CORE", "-fPIE")]
    run(LLVM / "clang", *extension_flags, "-fPIC", "-c", ROOT / "tools/native-python/extension.c", "-o", out / "extension.o")
    run("/opt/homebrew/opt/lld/bin/ld.lld", "-shared", "--hash-style=sysv", "-z", "max-page-size=4096",
        out / "extension.o", "-o", out / "infinity_native_test.so")
    run(LLVM / "clang", *extension_flags, "-fPIC", "-O0", "-c", ROOT / "tools/native-python/library.c", "-o", out / "library.o")
    run("/opt/homebrew/opt/lld/bin/ld.lld", "-shared", "--hash-style=sysv", "-z", "max-page-size=4096",
        out / "library.o", "-o", out / "library.so")
    make = (source / "Makefile.pre.in").read_text()
    sources = []
    for group in ("PEGEN_OBJS", "TOKENIZER_OBJS", "POBJS", "PYTHON_OBJS", "OBJECT_OBJS", "MODULE_OBJS"):
        match = re.search(r"^" + group + r"=\s*(.*?)(?=\n\n)", make, re.M | re.S)
        sources += [source / (x[:-2] + ".c") for x in re.findall(r"(?:Parser|Python|Objects|Modules)/[\w/.-]+\.o", match[1])]
    sources = [p for p in sources if p.name != "config.c"]
    sources += [source / p for p in ("Parser/myreadline.c", "Python/dynload_shlib.c", "Modules/getbuildinfo.c", "Modules/getpath.c")]
    bootstrap = (source / "Modules/Setup.bootstrap.in").read_text()
    modules = []
    for line in bootstrap.splitlines():
        if not line or line[0] in "#*@":
            continue
        fields = line.split()
        modules.append(fields[0])
        sources += [source / "Modules" / x for x in fields[1:] if x.endswith(".c")]
    config = (source / "Modules/config.c.in").read_text()
    config = config.replace("/* -- ADDMODULE MARKER 1 -- */", "\n".join(f"extern PyObject *PyInit_{m}(void);" for m in modules))
    config = config.replace("/* -- ADDMODULE MARKER 2 -- */", "\n".join(f'{{"{m}", PyInit_{m}}},' for m in modules))
    (out / "config.c").write_text(config)
    frozen = ['#include "Python.h"', '#include "pycore_import.h"']
    entries = [[], []]
    names = {"_frozen_importlib": "importlib/_bootstrap.py", "_frozen_importlib_external": "importlib/_bootstrap_external.py", "zipimport": "zipimport.py"}
    # CPython 3.13 registers -c source through linecache before running it;
    # this is needed by native syntax validation, not just traceback display.
    stdlib = "abc codecs io _collections_abc _sitebuiltins genericpath ntpath posixpath os site stat keyword types reprlib functools operator collections enum copyreg warnings re json importlib linecache".split()
    for name in stdlib:
        path = source / "Lib" / name
        if path.is_dir():
            for item in sorted(path.rglob("*.py")):
                relative = item.relative_to(source / "Lib").with_suffix("").as_posix()
                names[relative.replace("/__init__", "").replace("/", ".")] = item.relative_to(source / "Lib").as_posix()
        else:
            names[name] = name + ".py"
    for item in sorted((source / "Lib/encodings").glob("*.py")):
        name = "encodings" if item.stem == "__init__" else "encodings." + item.stem
        names[name] = "encodings/" + item.name
    for index, (name, filename) in enumerate(names.items()):
        data = marshal.dumps(compile((source / "Lib" / filename).read_bytes(), f"<frozen {name}>", "exec", optimize=0))
        frozen.append(f"static const unsigned char data_{index}[]={{" + ",".join(map(str, data)) + "};")
        entries[0 if index < 3 else 1].append(f'{{"{name}",data_{index},{len(data)},{int(filename.endswith("/__init__.py"))}}},')
    for index, suffix in enumerate(("Bootstrap", "Stdlib")):
        frozen += [f"static const struct _frozen table_{index}[]={{", *entries[index], "{0,0,0,0}};", f"const struct _frozen *_PyImport_Frozen{suffix}=table_{index};"]
    frozen += ["static const struct _frozen empty[]={ {0,0,0,0} };", "const struct _frozen *_PyImport_FrozenTest=empty;", "const struct _frozen *PyImport_FrozenModules=0;",
               'static const struct _module_alias aliases[]={{"_frozen_importlib","importlib._bootstrap"},{"_frozen_importlib_external","importlib._bootstrap_external"},{0,0}};',
               "const struct _module_alias *_PyImport_FrozenAliases=aliases;"]
    (out / "frozen.c").write_text("\n".join(frozen))
    data = marshal.dumps(compile((source / "Modules/getpath.py").read_bytes(), "<frozen getpath>", "exec", optimize=0))
    frozen_headers = out / "Python/frozen_modules"
    frozen_headers.mkdir(parents=True, exist_ok=True)
    (frozen_headers / "getpath.h").write_text("static const unsigned char _Py_M__getpath[]={" + ",".join(map(str, data)) + "};")
    sources += [out / "config.c", out / "frozen.c", ROOT / "sdk/python/main.c", ROOT / "sdk/python/posix.c", ROOT / "sdk/python/dlfcn.c"]
    sources += [ROOT / f"sdk/compiler/{unit}.c" for unit in "start entry isolated platform files heap time serial_sync serial_tls pthread namespace process newlib".split()]
    arithmetic, arithmetic_license = arithmetic_sources()
    sources += arithmetic

    # ------------------------=
    # FUNC: compile_unit
    # DESC: Compiles a source with explicit native includes and retains actionable diagnostics.
    # ------------------=
    def compile_unit(path):
        obj = out / (hashlib.sha256(str(path).encode()).hexdigest()[:12] + "-" + path.stem + ".o")
        command = [str(LLVM / "clang"), *flags, "-c", str(path), "-o", str(obj)]
        result = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (obj.with_suffix(".log")).write_bytes(result.stdout)
        if result.returncode:
            return str(path) + "\n" + result.stdout.decode(errors="replace")
        return obj
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
        results = list(pool.map(compile_unit, dict.fromkeys(sources)))
    failures = [x for x in results if isinstance(x, str)]
    if failures:
        (out / "errors.log").write_text("\n".join(failures))
        record(arch, "compile", "\n".join(failures).encode())
        print("\n".join(failures[:2])[:12000])
        raise SystemExit(f"{len(failures)} failed units; see {out / 'errors.log'}")
    link = ["/opt/homebrew/opt/lld/bin/ld.lld", "-static", "--error-limit=0", "--image-base=0x100000000", "-z", "max-page-size=4096",
        "-e", "infinity_compiler_entry", *results, "--start-group", newlib / "libc.a", newlib / "libm.a", "--end-group", "-o", out / "python.elf"]
    result = subprocess.run([str(x) for x in link], cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (out / "link.log").write_bytes(result.stdout)
    record(arch, "link", result.stdout)
    if result.returncode:
        print(result.stdout.decode(errors="replace")[:16000])
        raise SystemExit(f"Native Python link failed; see {out / 'link.log'}")
    evidence = dict(architecture=arch, cpython_version=VERSION, source_archive_sha256=SHA,
        executable_sha256=hashlib.sha256((out / "python.elf").read_bytes()).hexdigest(),
        compiled=True, runtime_verified=False, installed_verified=False)
    package = ROOT / "builds/toolchains" / arch
    package.mkdir(parents=True, exist_ok=True)
    for name in ("python.elf", "infinity_native_test.so"):
        temporary = package / (name + ".partial")
        shutil.copyfile(out / name, temporary)
        temporary.replace(package / name)
    (package / "python-license.txt").write_bytes((source / "LICENSE").read_bytes() + b"\n\nLLVM compiler runtime:\n" + arithmetic_license.read_bytes())
    (ROOT / "builds/validation/native-python" / arch / "build.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(out / "python.elf")

if __name__ == "__main__":
    main()
