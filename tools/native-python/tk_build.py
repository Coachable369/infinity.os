"""Build gate for the genuine upstream Tk native platform implementation.

This gate compiles the real widget engine and native backend separately. A
successful object build is not tkinter acceptance; publication remains gated on
the real window, widget, input and Python bridge acceptance executable.
"""
from pathlib import Path
import concurrent.futures
import hashlib
import os
import re
import subprocess
import sys

from tcl_sources import sources

ROOT = Path(__file__).resolve().parents[2]
LLVM = Path(os.environ.get("LLVM_BIN", "/opt/homebrew/opt/llvm/bin"))


# ------------------------=
# FUNC: library_sources
# DESC: Selects actual upstream Tk widgets and reusable platform drawing code, never platform dummy implementations.
# ------------------=
def library_sources(upstream):
    tk = upstream["tk"]
    make = (tk / "unix/Makefile.in").read_text()
    units = []
    for group in ("GENERIC_OBJS", "WIDG_OBJS", "CANV_OBJS", "IMAGE_OBJS", "TEXT_OBJS", "STUB_OBJS", "TTK_OBJS"):
        match = re.search(r"^" + group + r"\s*=\s*(.*?)(?=\n\n)", make, re.M | re.S)
        if not match:
            raise RuntimeError("Pinned Tk object contract changed: " + group)
        directory = tk / "generic" / ("ttk" if group == "TTK_OBJS" else "")
        units.extend(directory / (name + ".c") for name in re.findall(r"\b(\w+)\.o", match[1]))
    units += [tk / "unix" / (name + ".c") for name in
              ("tkUnix3d", "tkUnixButton", "tkUnixColor", "tkUnixConfig", "tkUnixDraw",
               "tkUnixMenu", "tkUnixMenubu", "tkUnixScale", "tkUnixScrlbr")]
    units += [tk / "xlib" / (name + ".c") for name in ("xcolors", "xdraw", "ximage", "xgc")]
    units += sorted((ROOT / "sdk/python/tk").glob("*.c"))
    return units


# ------------------------=
# FUNC: build_objects
# DESC: Compiles the same native Tk backend for either architecture and preserves complete per-unit diagnostics under the build lease.
# ------------------=
def build_objects(arch):
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/native-python/tk_build.py ARCH")
    if arch not in ("aarch64", "x86_64"):
        raise SystemExit("Unsupported native architecture")
    upstream = sources()
    tk, tcl = upstream["tk"], upstream["tcl"]
    out = ROOT / "build/native-tk" / arch
    out.mkdir(parents=True, exist_ok=True)
    declarations = (tk / "generic/tkIntXlibDecls.h").read_text()
    declarations = declarations.replace("#if defined(_WIN32) || defined(__CYGWIN__) /* WIN */",
                                        "#if 1 /* Native backend uses the upstream X interface declarations. */", 1)
    # Native keyboard/text input is delivered through IEF, not XIM. Do not
    # advertise a server input-method API that this platform does not provide.
    (out / "tkIntXlibDecls.h").write_text(declarations + "\n#undef XNQueryInputStyle\n")
    # The official Git archive names the exact commit; unlike Fossil archives,
    # it has no manifest.uuid. Preserve that real source identity in Tk's API.
    (out / "tkUuid.h").write_text("#define TK_VERSION_UUID " + tk.name.removeprefix("tk-") + "\n")
    newlib = (ROOT / "build/native-python/x86_64/newlib-pie/x86_64-none-elf/newlib"
              if arch == "x86_64" else ROOT / "build/voice-newlib-aarch64/aarch64-none-elf/newlib")
    if not (newlib / "libc.a").is_file():
        raise SystemExit("Build native Python for this architecture first")
    includes = [out, ROOT / "sdk/python/tk", ROOT / "sdk/python/tcl", tk / "generic", tk / "generic/ttk",
                tk / "xlib", tk / "bitmaps", tk / "unix", tcl / "generic", tcl / "libtommath",
                tcl / "unix", ROOT / "sdk/python/include", ROOT / "sdk/compiler/include",
                newlib / "targ-include", newlib,
                ROOT / "build/newlib-4.6.0.20260123/newlib/libc/include"]
    flags = [f"--target={arch}-none-elf", "-std=c11", "-O2", "-fPIE", "-ffunction-sections", "-fdata-sections",
             "-DBUILD_tk", "-DHAVE_TCL_CONFIG_H", "-DNDEBUG", "-DSTATIC_BUILD", "-D_XOPEN_SOURCE=700",
             '-DTK_LIBRARY="/system/python/tk8.6"', '-DTK_PACKAGE_PATH="/system/python"',
             "-include", str(ROOT / "sdk/compiler/target.h")]
    flags += ["-mno-red-zone"] if arch == "x86_64" else ["-mno-outline-atomics", "-mstrict-align"]
    flags += [f"-I{path}" for path in includes]
    flags += ["-idirafter", str(ROOT / "sdk/c/include")]

    # ------------------------=
    # FUNC: compile_unit
    # DESC: Compiles one actual upstream or native platform unit without executing target code on the host.
    # ------------------=
    def compile_unit(path):
        obj = out / (hashlib.sha256(str(path).encode()).hexdigest()[:12] + "-" + path.stem + ".o")
        if path.name == "xgc.c":
            text = path.read_text()
            marker = "/*\n * Some additional dummy functions (hopefully implemented soon).\n */"
            if text.count(marker) != 1:
                raise RuntimeError("Pinned upstream GC boundary changed")
            # Upstream's real GC/clip implementation is reused. Its subsequent
            # platform dummy functions must never satisfy native symbols.
            text = text[:text.index(marker)]
            generated = out / "native-gc.c"
            generated.write_text(text + "\nint XSetRegion(Display *d, GC g, Region r) { return TkSetRegion(d, g, (TkRegion)r); }\n")
            path = generated
        result = subprocess.run([str(LLVM / "clang"), *flags, "-c", str(path), "-o", str(obj)],
                                cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        obj.with_suffix(".log").write_bytes(result.stdout)
        return obj if not result.returncode else str(path) + "\n" + result.stdout.decode(errors="replace")

    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
        results = list(pool.map(compile_unit, library_sources(upstream)))
    failures = [result for result in results if isinstance(result, str)]
    (out / "errors.log").write_text("\n".join(failures))
    if failures:
        print("\n".join(failures[:4])[:14000])
        raise SystemExit(f"{len(failures)} failed Tk units; see {out / 'errors.log'}")
    return results, compile_unit, out


# ------------------------=
# FUNC: python_support
# DESC: Builds genuine Tcl/Tk and creates only an upstream CPython module initializer wrapper for the immutable native library filesystem.
# ------------------=
def python_support(arch, newlib, compression, python_source):
    from tcl_build import build_library
    upstream = sources()
    tk_objects, _, out = build_objects(arch)
    tcl_objects, _, _ = build_library(arch, newlib, compression)
    bridge = out / "tkinter-native.c"
    implementation = python_source / "Modules/_tkinter.c"
    bridge.write_text(
        '#define select InfinityTkinterSelect\n'
        '#define PyInit__tkinter InfinityTkinterImplementationInit\n'
        '#include "' + str(implementation) + '"\n'
        '#undef PyInit__tkinter\n'
        '#undef select\n'
        '// ------------------------=\n'
        '// FUNC: InfinityTkinterSelect\n'
        '// DESC: Implements the upstream zero-descriptor sleep using the real native Tcl timer; readiness requests remain explicitly unsupported.\n'
        '// ------------------=\n'
        'int InfinityTkinterSelect(int n, fd_set *r, fd_set *w, fd_set *e, struct timeval *t) {\n'
        '    if (n || r || w || e || !t) { errno = ENOTSUP; return -1; }\n'
        '    if (t->tv_sec < 0 || t->tv_usec < 0 || t->tv_usec >= 1000000 || t->tv_sec > 2147482) { errno = EINVAL; return -1; }\n'
        '    Tcl_Sleep((int)(t->tv_sec * 1000 + (t->tv_usec + 999) / 1000));\n'
        '    t->tv_sec = t->tv_usec = 0;\n'
        '    return 0;\n'
        '}\n'
        'extern int InfinityTclRegisterResources(void);\n'
        '// ------------------------=\n'
        '// FUNC: PyInit__tkinter\n'
        '// DESC: Registers exact upstream Tcl/Tk resources after the real CPython module initializes Tcl, before any application interpreter is created.\n'
        '// ------------------=\n'
        'PyMODINIT_FUNC PyInit__tkinter(void) {\n'
        '    PyObject *module = InfinityTkinterImplementationInit();\n'
        '    if (!module) return NULL;\n'
        '    if (InfinityTclRegisterResources() != TCL_OK) {\n'
        '        Py_DECREF(module);\n'
        '        PyErr_SetString(PyExc_RuntimeError, "Native Tcl library registration failed");\n'
        '        return NULL;\n'
        '    }\n'
        '    return module;\n'
        '}\n')
    return {
        "objects": tk_objects + tcl_objects,
        "includes": [upstream["tcl"] / "generic", upstream["tk"] / "generic", upstream["tk"] / "xlib"],
        "source": bridge,
        "module": "_tkinter",
    }


# ------------------------=
# FUNC: main
# DESC: Produces the bounded native Tk object gate without publishing or claiming a usable tkinter payload.
# ------------------=
def main():
    arch = sys.argv[1] if len(sys.argv) > 1 else "aarch64"
    objects, tk_compile, out = build_objects(arch)
    (out / "objects.list").write_text("\n".join(str(obj) for obj in objects) + "\n")
    if "--link" in sys.argv:
        from tcl_build import build_library
        from build import compression_sources, arithmetic_sources
        compression, compression_units = compression_sources()
        newlib = (ROOT / "build/native-python/x86_64/newlib-pie/x86_64-none-elf/newlib"
                  if arch == "x86_64" else ROOT / "build/voice-newlib-aarch64/aarch64-none-elf/newlib")
        tcl, compile_unit, _ = build_library(arch, newlib, compression)
        objects += tcl
        for fixture in ("tk-image-acceptance.c", "tk-draw-acceptance.c"):
            pixel_test = tk_compile(ROOT / "tools/native-python" / fixture)
            if isinstance(pixel_test, str):
                print(pixel_test); raise SystemExit("Native Tk pixel acceptance compilation failed")
            objects.append(pixel_test)
        adapters = [ROOT / f"sdk/compiler/{name}.c" for name in
                    "start entry isolated platform files heap time serial_sync serial_tls pthread namespace process newlib".split()]
        adapters += [ROOT / "sdk/python/posix.c", ROOT / "tools/native-python/tk-acceptance.c"]
        arithmetic, _ = arithmetic_sources()
        for path in adapters + compression_units + arithmetic:
            result = compile_unit(path)
            if isinstance(result, str):
                print(result); raise SystemExit("Native Tk adapter compilation failed")
            objects.append(result)
        command = ["/opt/homebrew/opt/lld/bin/ld.lld", "-static", "--error-limit=0", "--image-base=0x100000000",
                   "-z", "max-page-size=4096", "-e", "infinity_compiler_entry", *objects,
                   "--start-group", newlib / "libc.a", newlib / "libm.a", "--end-group", "-o", out / "tk.elf"]
        result = subprocess.run([str(item) for item in command], cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (out / "link.log").write_bytes(result.stdout)
        if result.returncode:
            print(result.stdout.decode(errors="replace")[:16000]); raise SystemExit("Native Tk link failed")
        print(out / "tk.elf")
        return
    print(f"Compiled {len(objects)} upstream/native Tk objects; widget runtime acceptance is still required")


if __name__ == "__main__":
    main()
