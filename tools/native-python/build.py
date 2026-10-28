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
import zipfile
from module_inventory import module_inventory
from dependencies import bzip2_sources, timezone_wheel, lzma_library
from tcl_sources import sources as tcl_sources
from tk_build import python_support
from ffi_build import python_support as ffi_support
from bundled_packages import freeze_packages, native_sources, license_notices, packages

ROOT = Path(__file__).resolve().parents[2]
VERSION = "3.13.16"
SHA = "f4b1bfb3c79b5bb11b8d228a12504163b4c0dab4d679828d8f5f26b6cb6ab35d"
LLVM = Path(os.environ.get("LLVM_BIN", "/opt/homebrew/opt/llvm/bin"))
LLVM_REVISION = "ea7d852a70e8bdfaf601d6626a760f9771b2c4b4"
ZLIB_CRATE_VERSION = "1.1.29"
ZLIB_CRATE_SHA = "85bc9657773828b90eeb625adff10eeac83cc21bbfd8e23a03eaa8a33c9e28d9"

# ------------------------=
# FUNC: compression_sources
# DESC: Reuses the browser's pinned zlib source crate while retaining a clean-build independent archive.
# ------------------=
def compression_sources():
    archive = ROOT / f"third_party/python/libz-sys-{ZLIB_CRATE_VERSION}.crate"
    if not archive.exists():
        cached = sorted((ROOT / "build/servo-cargo-home/registry/cache").glob(f"*/{archive.name}"))
        if cached:
            shutil.copyfile(cached[0], archive)
        else:
            with urllib.request.urlopen(f"https://static.crates.io/crates/libz-sys/{archive.name}", timeout=60) as response:
                archive.write_bytes(response.read())
    if hashlib.sha256(archive.read_bytes()).hexdigest() != ZLIB_CRATE_SHA:
        raise SystemExit("zlib source crate checksum mismatch")
    destination = archive.parent / f"libz-sys-{ZLIB_CRATE_VERSION}"
    if not destination.exists():
        with tarfile.open(archive) as bundle:
            bundle.extractall(archive.parent, filter="data")
    source = destination / "src/zlib"
    units = "adler32 compress crc32 deflate infback inffast inflate inftrees trees uncompr zutil".split()
    return source, [source / f"{unit}.c" for unit in units]

# ------------------------=
# FUNC: frozen_library
# DESC: Freezes upstream pip and its standard-library dependencies with exact bundled resources and native target metadata.
# ------------------=
def frozen_library(source, arch):
    library = source / "Lib"
    names = {
        "_frozen_importlib": ("importlib/_bootstrap.py", (library / "importlib/_bootstrap.py").read_bytes()),
        "_frozen_importlib_external": ("importlib/_bootstrap_external.py", (library / "importlib/_bootstrap_external.py").read_bytes()),
        "zipimport": ("zipimport.py", (library / "zipimport.py").read_bytes()),
    }
    excluded = {"test", "tests", "idlelib", "turtledemo", "ensurepip", "venv", "_pyrepl"}
    for path in sorted(library.rglob("*.py")):
        relative = path.relative_to(library)
        if any(part in excluded for part in relative.parts):
            continue
        name = relative.with_suffix("").as_posix().replace("/__init__", "").replace("/", ".")
        body = path.read_bytes()
        if name == "subprocess":
            before = b'{"emscripten", "wasi", "ios", "tvos", "watchos"}'
            if body.count(before) != 1:
                raise SystemExit("CPython subprocess platform contract changed")
            body = body.replace(before, b'{"emscripten", "wasi", "ios", "tvos", "watchos", "infinityos"}')
        if name == "http.client":
            before = b"    if http_version == 11:\n        context.set_alpn_protocols(['http/1.1'])"
            if body.count(before) != 1:
                raise SystemExit("CPython HTTPS ALPN feature contract changed")
            body = body.replace(before, b"    if http_version == 11 and ssl.HAS_ALPN:\n        context.set_alpn_protocols(['http/1.1'])")
        names[name] = (relative.as_posix(), body)
    for name, filename in (("_socket", "socket.py"), ("_ssl", "ssl.py"), ("select", "select_unavailable.py"),
                           ("_infinity_modules", "_infinity_modules.py"),
                           ("_infinity_network", "network.py"),
                           ("_infinity_subprocess", "subprocess.py"),
                           ("_infinity_child_entry", "child_entry.py"),
                           ("_infinity_generated_run", "generated_run.py"),
                           ("infinity", "infinity.py"),
                           ("_infinity_platform", "platform.py")):
        names[name] = (name + ".py", (ROOT / "sdk/python" / filename).read_bytes())
    resources, metadata = {}, {}
    wheel = library / "ensurepip/_bundled/pip-26.2.1-py3-none-any.whl"
    with zipfile.ZipFile(wheel) as bundle:
        for entry in bundle.infolist():
            if entry.is_dir():
                continue
            body = bundle.read(entry)
            if entry.filename.endswith(".py"):
                if entry.filename == "pip/_vendor/platformdirs/unix.py":
                    # InfinityOS grants capability-scoped namespaces, not a
                    # Unix root UID. Keep upstream's ordinary per-user paths
                    # without inventing a numeric user identity.
                    before = b"    from os import getuid"
                    root_check = b"self.use_site_for_root and getuid() == 0"
                    if body.count(before) != 1 or body.count(root_check) != 1:
                        raise SystemExit("pip platformdirs UID contract changed")
                    body = body.replace(before, b'    getuid = getattr(os, "getuid", None)')
                    body = body.replace(root_check, b"self.use_site_for_root and getuid is not None and getuid() == 0")
                    body = body.replace(b'        if sys.platform.startswith("openbsd"):',
                        b'        if sys.platform == "infinityos":\n            path = f"{gettempdir()}/runtime"\n        elif sys.platform.startswith("openbsd"):')
                    body = body.replace(b'path = f"{gettempdir()}/runtime-{getuid()}"',
                        b'path = f"{gettempdir()}/runtime" if getuid is None else f"{gettempdir()}/runtime-{getuid()}"')
                name = entry.filename[:-3].replace("/__init__", "").replace("/", ".")
                names[name] = (entry.filename, body)
            else:
                resources[entry.filename] = body
                if entry.filename.startswith("pip-26.2.1.dist-info/"):
                    metadata[entry.filename.split("/", 1)[1]] = body
            # Upstream pyproject-hooks launches this real Python file in a
            # separate native child. Its bytes are immutable package resources.
            if entry.filename == "pip/_vendor/pyproject_hooks/_in_process/_in_process.py":
                resources[entry.filename] = body
    distributions = {"pip": metadata}
    timezone_metadata = {}
    with zipfile.ZipFile(timezone_wheel()) as bundle:
        for entry in bundle.infolist():
            if entry.is_dir():
                continue
            body = bundle.read(entry)
            if entry.filename.endswith(".py"):
                name = entry.filename[:-3].replace("/__init__", "").replace("/", ".")
                names[name] = (entry.filename, body)
            else:
                resources[entry.filename] = body
                if ".dist-info/" in entry.filename:
                    timezone_metadata[entry.filename.split("/", 1)[1]] = body
    distributions["tzdata"] = timezone_metadata
    original_modules = set(names)
    freeze_packages(names, resources, distributions)
    public_roots = sorted({name.split(".")[0] for name in set(names) - original_modules})
    resource_code = f"RESOURCES = {resources!r}\nDISTRIBUTION_METADATA = {distributions!r}\nPUBLIC_PACKAGE_ROOTS = {public_roots!r}\n"
    names["_infinity_resources"] = ("_infinity_resources.py", resource_code.encode())
    native_config = dict(SOABI=f"cpython-313-{arch}-infinityos", EXT_SUFFIX=".so",
        MULTIARCH=f"{arch}-infinityos", Py_GIL_DISABLED=0, Py_DEBUG=0, WITH_PYMALLOC=1,
        ABIFLAGS="", VERSION="3.13", LDVERSION="3.13", py_version_short="3.13",
        prefix="/system/python", exec_prefix="/system/python", LIBDIR="/system/python/lib",
        CC="clang", CFLAGS="", LDSHARED="", SHLIB_SUFFIX=".so", EXE="", SIZEOF_VOID_P=8)
    name = f"_sysconfigdata__infinityos_{arch}-infinityos"
    names[name] = (name + ".py", ("build_time_vars = " + repr(native_config) + "\n").encode())
    return names, wheel

# ------------------------=
# FUNC: arithmetic_sources
# DESC: Retrieves only the pinned LLVM compiler-runtime conversion helper and its local headers.
# ------------------=
def arithmetic_sources():
    destination = ROOT / "third_party/python/compiler-rt" / LLVM_REVISION
    destination.mkdir(parents=True, exist_ok=True)
    pending = ["trunctfdf2.c", "extenddftf2.c"]
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
    return [destination / name for name in ("trunctfdf2.c", "extenddftf2.c")], license_path

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
    compression, zlib_sources = compression_sources()
    bzip2, bzip2_units = bzip2_sources()
    lzma_include, lzma_archive, lzma_license = lzma_library(arch, LLVM, newlib)
    # Reuse the checksum-pinned Tcl distribution's complete SQLite amalgamation.
    sqlite = tcl_sources()["tcl"] / "pkgs/sqlite3.53.0/compat/sqlite3"
    tkinter = python_support(arch,newlib,compression,source)
    ctypes = ffi_support(arch, LLVM, newlib, source)
    includes = [out, bzip2, sqlite, lzma_include, ROOT / "sdk/python/include", source / "Include", source / "Include/internal", source,
                source / "Modules/_hacl/include", source / "Modules/expat", compression,
                ROOT / "sdk/compiler/include", newlib / "targ-include", newlib,
                ROOT / "build/newlib-4.6.0.20260123/newlib/libc/include", *tkinter["includes"], *ctypes["includes"]]
    flags = [f"--target={arch}-none-elf", "-std=c11", "-O2", "-fPIE", "-ffunction-sections", "-fdata-sections",
             "-DPy_BUILD_CORE", "-DNDEBUG", "-include", str(ROOT / "sdk/compiler/target.h"),
             '-DPREFIX="/system/python"', '-DEXEC_PREFIX="/system/python"', '-DVERSION="3.13"',
             '-DVPATH=""', '-DPLATLIBDIR="lib"', '-DPYTHONPATH=""', '-DHAVE_FFI_PREP_CIF_VAR=1']
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
    run(LLVM / "clang", *extension_flags, "-fPIC", "-c", ROOT / "tools/native-python/ffi-library.c", "-o", out / "ffi-library.o")
    run("/opt/homebrew/opt/lld/bin/ld.lld", "-shared", "--hash-style=sysv", "-z", "max-page-size=4096",
        out / "ffi-library.o", "-o", out / "native_ffi_test.so")
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
    extra_modules = {
        "_sqlite3": ["_sqlite/" + name + ".c" for name in
                     "blob connection cursor microprotocols module prepare_protocol row statement util".split()],
        "_lzma": ["_lzmamodule.c"],
        "mmap": ["mmapmodule.c"],
        "array": ["arraymodule.c"], "_bisect": ["_bisectmodule.c"], "_csv": ["_csv.c"],
        "_heapq": ["_heapqmodule.c"], "_json": ["_json.c"], "_opcode": ["_opcode.c"],
        "_contextvars": ["_contextvarsmodule.c"],
        "_queue": ["_queuemodule.c"], "_statistics": ["_statisticsmodule.c"],
        "_elementtree": ["_elementtree.c"], "_zoneinfo": ["_zoneinfo.c"],
        "_lsprof": ["_lsprof.c", "rotatingtree.c"],
        "_multibytecodec": ["cjkcodecs/multibytecodec.c"],
        "_codecs_cn": ["cjkcodecs/_codecs_cn.c"],
        "_codecs_hk": ["cjkcodecs/_codecs_hk.c"],
        "_codecs_iso2022": ["cjkcodecs/_codecs_iso2022.c"],
        "_codecs_jp": ["cjkcodecs/_codecs_jp.c"],
        "_codecs_kr": ["cjkcodecs/_codecs_kr.c"],
        "_codecs_tw": ["cjkcodecs/_codecs_tw.c"],
        "_pickle": ["_pickle.c"], "_random": ["_randommodule.c"], "_struct": ["_struct.c"],
        "math": ["mathmodule.c"], "cmath": ["cmathmodule.c"], "_datetime": ["_datetimemodule.c"],
        "binascii": ["binascii.c"], "zlib": ["zlibmodule.c"], "_bz2": ["_bz2module.c"], "unicodedata": ["unicodedata.c"],
        "pyexpat": ["pyexpat.c", "expat/xmlparse.c", "expat/xmlrole.c", "expat/xmltok.c"],
        "_md5": ["md5module.c", "_hacl/Hacl_Hash_MD5.c"],
        "_sha1": ["sha1module.c", "_hacl/Hacl_Hash_SHA1.c"],
        "_sha2": ["sha2module.c", "_hacl/Hacl_Hash_SHA2.c"],
        "_sha3": ["sha3module.c", "_hacl/Hacl_Hash_SHA3.c"],
        "_blake2": ["_blake2/blake2module.c", "_blake2/blake2b_impl.c", "_blake2/blake2s_impl.c"],
    }
    for name, units in extra_modules.items():
        modules.append(name)
        sources += [source / "Modules" / unit for unit in units]
    modules.append("_infinity_https")
    sources.append(ROOT / "sdk/python/https.c")
    modules.append("_infinity_app")
    sources.append(ROOT / "sdk/python/app.c")
    modules.append("_infinity_socket")
    sources.append(ROOT / "sdk/python/socket.c")
    modules.append("_infinity_process")
    sources.append(ROOT / "sdk/python/process.c")
    modules.append("_infinity_runtime")
    sources.append(ROOT / "sdk/python/runtime.c")
    modules.append(tkinter["module"])
    sources.append(tkinter["source"])
    modules.append("_ctypes")
    sources += ctypes["sources"]
    modules.append("markupsafe._speedups")
    sources += native_sources()
    sources += zlib_sources + bzip2_units + [sqlite / "sqlite3.c", ROOT / "sdk/python/sqlite.c"]
    config = (source / "Modules/config.c.in").read_text()
    config = config.replace("/* -- ADDMODULE MARKER 1 -- */", "\n".join(f"extern PyObject *PyInit_{m.rsplit('.', 1)[-1]}(void);" for m in modules))
    config = config.replace("/* -- ADDMODULE MARKER 2 -- */", "\n".join(f'{{"{m}", PyInit_{m.rsplit(".", 1)[-1]}}},' for m in modules))
    (out / "config.c").write_text(config)
    frozen = ['#include "Python.h"', '#include "pycore_import.h"']
    entries = [[], []]
    names, pip_wheel = frozen_library(source, arch)
    inventory = module_inventory(source, VERSION, arch, names, modules)
    inventory["bundled_distributions"] = {p["name"]: p["version"] for p in packages()}
    inventory["bundled_distributions"].update(pip="26.2.1", tzdata="2026.5")
    names["_infinity_inventory"] = ("_infinity_inventory.py",
        ("INVENTORY = " + repr(inventory) + "\n").encode())
    for index, (name, (filename, body)) in enumerate(names.items()):
        data = marshal.dumps(compile(body, f"/stdlib/{filename}", "exec", optimize=0))
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
        # CPython's installation PREFIX is not a third-party library macro;
        # Expat uses PREFIX as an XML namespace structure type.
        unit_flags = [*flags, "-UPREFIX"] if path.parent == source / "Modules/expat" else list(flags)
        if path == source / "Modules/mmapmodule.c":
            # This native port exposes real getpagesize(), not a fabricated
            # general POSIX sysconf service. Anonymous allocation granularity
            # is the same verified physical page size on both targets.
            unit_flags += ["-Dmy_getallocationgranularity=getpagesize"]
        if path.parent == sqlite or path == ROOT / "sdk/python/sqlite.c":
            unit_flags += ["-DSQLITE_OS_OTHER=1", "-DSQLITE_THREADSAFE=0", "-DSQLITE_TEMP_STORE=3",
                           "-DSQLITE_OMIT_LOAD_EXTENSION", "-DSQLITE_OMIT_WAL", "-DSQLITE_ENABLE_FTS5",
                           "-DSQLITE_ENABLE_RTREE", "-DSQLITE_ENABLE_MATH_FUNCTIONS"]
        if path.parent == source / "Modules/_sqlite":
            unit_flags += ["-DPY_SQLITE_HAVE_SERIALIZE"]
        command = [str(LLVM / "clang"), *unit_flags, "-c", str(path), "-o", str(obj)]
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
        "-e", "infinity_compiler_entry", *results, *tkinter["objects"], "--start-group", lzma_archive, ctypes["archive"],
        newlib / "libc.a", newlib / "libm.a", "--end-group", "-o", out / "python.elf"]
    result = subprocess.run([str(x) for x in link], cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (out / "link.log").write_bytes(result.stdout)
    record(arch, "link", result.stdout)
    if result.returncode:
        print(result.stdout.decode(errors="replace")[:16000])
        raise SystemExit(f"Native Python link failed; see {out / 'link.log'}")
    evidence = dict(architecture=arch, cpython_version=VERSION, source_archive_sha256=SHA,
        pip_version="26.2.1", pip_wheel_sha256=hashlib.sha256(pip_wheel.read_bytes()).hexdigest(),
        bundled_distributions=inventory["bundled_distributions"],
        frozen_module_count=len(names), builtin_modules=modules,
        executable_sha256=hashlib.sha256((out / "python.elf").read_bytes()).hexdigest(),
        compiled=True, runtime_verified=False, installed_verified=False)
    package = ROOT / "builds/toolchains" / arch
    package.mkdir(parents=True, exist_ok=True)
    for name in ("python.elf", "infinity_native_test.so"):
        temporary = package / (name + ".partial")
        shutil.copyfile(out / name, temporary)
        temporary.replace(package / name)
    notices = [(source / "LICENSE").read_bytes(), b"\n\nLLVM compiler runtime:\n", arithmetic_license.read_bytes(),
               b"\n\nzlib:\n", (compression / "zlib.h").read_bytes().split(b"#ifndef", 1)[0],
               b"\n\nBundled Expat:\n", (source / "Modules/expat/COPYING").read_bytes(),
               b"\n\nbzip2:\n", (bzip2 / "LICENSE").read_bytes(),
               b"\n\nXZ/liblzma:\n", lzma_license.read_bytes(),
               b"\n\nlibffi:\n", ctypes["license"].read_bytes(),
               b"\n\nSQLite (public domain):\n", (sqlite / "sqlite3.c").read_bytes().split(b"#define", 1)[0]]
    for component,path in tcl_sources().items():
        license_file=path / "license.terms"
        notices += [b"\n\n"+component.encode()+b":\n",license_file.read_bytes()]
    for wheel in (pip_wheel, timezone_wheel()):
        with zipfile.ZipFile(wheel) as bundle:
            for name in bundle.namelist():
                if "/licenses/" in name or name.endswith(("LICENSE", "LICENSE.txt", "COPYING")):
                    notices += [b"\n\n" + name.encode() + b":\n", bundle.read(name)]
    notices += license_notices()
    (package / "python-license.txt").write_bytes(b"".join(notices))
    (package / "python-modules.json").write_text(json.dumps(inventory, indent=2) + "\n")
    (ROOT / "builds/validation/native-python" / arch / "build.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(out / "python.elf")

if __name__ == "__main__":
    main()
