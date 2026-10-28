"""Builds the genuine Tcl core for the native Tk platform port.

This is a development gate, not a declaration that Tk is implemented. The
standalone executable exercises real interpreter, resource, timer and error
semantics before the larger Tk backend is linked into CPython.
"""
from pathlib import Path
import concurrent.futures
import hashlib
import json
import os
import re
import subprocess
import sys

from tcl_sources import sources as upstream_sources
from build import compression_sources, arithmetic_sources

ROOT = Path(__file__).resolve().parents[2]
LLVM = Path(os.environ.get("LLVM_BIN", "/opt/homebrew/opt/llvm/bin"))


# ------------------------=
# FUNC: resource_table
# DESC: Embeds exact upstream Tcl/Tk runtime resources so fresh installs do not depend on build-host library paths.
# ------------------=
def resource_table(upstream, out):
    entries = []
    declarations = []
    for component in ("tcl", "tk"):
        library = upstream[component] / "library"
        for path in sorted(library.rglob("*")):
            if not path.is_file() or path.suffix in (".html", ".man"):
                continue
            relative = path.relative_to(library).as_posix()
            body = path.read_bytes()
            symbol = "library_" + str(len(entries))
            declarations.append("static const unsigned char " + symbol + "[]={" +
                ",".join(map(str, body)) + ("" if body else "0") + "};")
            entries.append((f"/system/python/{component}8.6/{relative}", symbol, len(body)))
    declarations.append("static const BundledResource resources[]={")
    declarations += ["{" + json.dumps(path) + "," + symbol + "," + str(size) + "},"
                     for path, symbol, size in sorted(entries)]
    declarations.append("};")
    (out / "tcl-resources.inc").write_text("\n".join(declarations))


# ------------------------=
# FUNC: library_sources
# DESC: Reuses the pinned upstream core object list with explicit native replacements for platform-only code.
# ------------------=
def library_sources(upstream):
    source = upstream["tcl"]
    make = (source / "unix/Makefile.in").read_text()
    units = []
    for group, directory in (("GENERIC_OBJS", "generic"), ("OO_OBJS", "generic"),
                              ("TOMMATH_OBJS", "libtommath")):
        match = re.search(r"^" + group + r"\s*=\s*(.*?)(?=\n\n)", make, re.M | re.S)
        if match is None:
            raise RuntimeError("Pinned Tcl object contract changed: " + group)
        units += [source / directory / (name + ".c")
                  for name in re.findall(r"\b(\w+)\.o", match[1]) if name != "tclIOSock"]
    units += [source / "unix" / (name + ".c") for name in
              ("tclUnixChan", "tclUnixFile", "tclUnixFCmd", "tclUnixTime", "tclUnixThrd")]
    units.append(source / "generic/tclLoadNone.c")
    units += [ROOT / "sdk/python/tcl" / (name + ".c")
              for name in ("platform", "notifier", "unavailable", "resources")]
    return units


# ------------------------=
# FUNC: build_library
# DESC: Compiles upstream Tcl with native task/namespace adapters under the build-kit lease and retains per-unit diagnostics.
# ------------------=
def build_library(arch, newlib, compression):
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/native-python/tcl_build.py ARCH")
    upstream = upstream_sources()
    out = ROOT / "build/native-tcl" / arch
    out.mkdir(parents=True, exist_ok=True)
    resource_table(upstream, out)
    source = upstream["tcl"]
    includes = [out, ROOT / "sdk/python/tcl", source / "generic", source / "libtommath",
                source / "unix", ROOT / "sdk/python/include", ROOT / "sdk/compiler/include",
                newlib / "targ-include", newlib,
                ROOT / "build/newlib-4.6.0.20260123/newlib/libc/include", compression]
    flags = [f"--target={arch}-none-elf", "-std=c11", "-O2", "-fPIE",
             "-ffunction-sections", "-fdata-sections", "-DHAVE_TCL_CONFIG_H", "-DBUILD_tcl",
             "-DMP_FIXED_CUTOFFS", "-DMP_NO_STDINT", "-DNDEBUG",
             '-DTCL_TEMPORARY_FILE_DIRECTORY="/work/.cache"',
             "-include", str(ROOT / "sdk/compiler/target.h")]
    flags += ["-mno-red-zone"] if arch == "x86_64" else ["-mno-outline-atomics", "-mstrict-align"]
    flags += [f"-I{path}" for path in includes]
    flags += ["-idirafter", str(ROOT / "sdk/c/include")]

    # ------------------------=
    # FUNC: compile_unit
    # DESC: Compiles one native Tcl unit and records errors without executing target code on the host.
    # ------------------=
    def compile_unit(path):
        obj = out / (hashlib.sha256(str(path).encode()).hexdigest()[:12] + "-" + path.stem + ".o")
        if path.name == "tclUnixChan.c":
            # The pinned file combines reusable native file channels with the
            # Unix select notifier. Keep the channels but replace that one
            # platform function with notifier.c's actual native readiness.
            text = path.read_text()
            marker = "int\nTclUnixWaitForFile("
            if text.count(marker) != 1:
                raise RuntimeError("Pinned Tcl channel layout changed")
            start = text.index(marker)
            opening = text.index("{", start)
            depth, end = 1, opening + 1
            while depth and end < len(text):
                depth += (text[end] == "{") - (text[end] == "}")
                end += 1
            if depth:
                raise RuntimeError("Pinned Tcl channel function is incomplete")
            generated = out / "tclUnixChan-native.c"
            generated.write_text(text[:start] + "/* Native notifier supplies TclUnixWaitForFile. */\n" + text[end:])
            path = generated
        result = subprocess.run([str(LLVM / "clang"), *flags, "-c", str(path), "-o", str(obj)],
                                cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        obj.with_suffix(".log").write_bytes(result.stdout)
        return obj if result.returncode == 0 else str(path) + "\n" + result.stdout.decode(errors="replace")

    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
        result = list(pool.map(compile_unit, library_sources(upstream)))
    failures = [item for item in result if isinstance(item, str)]
    if failures:
        (out / "errors.log").write_text("\n".join(failures))
        print("\n".join(failures[:3])[:12000])
        raise SystemExit(f"{len(failures)} failed Tcl units; see {out / 'errors.log'}")
    return result, compile_unit, out


# ------------------------=
# FUNC: main
# DESC: Links the standalone interpreter acceptance executable without publishing a Tk-enabled Python payload.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/native-python/tcl_build.py ARCH")
    arch = sys.argv[1] if len(sys.argv) > 1 else "aarch64"
    if arch not in ("aarch64", "x86_64"):
        raise SystemExit("Unsupported architecture")
    newlib = (ROOT / "build/native-python/x86_64/newlib-pie/x86_64-none-elf/newlib"
              if arch == "x86_64" else ROOT / "build/voice-newlib-aarch64/aarch64-none-elf/newlib")
    if not (newlib / "libc.a").exists():
        raise SystemExit("Build native Python for this architecture first; its Newlib is required")
    compression, compression_units = compression_sources()
    objects, compile_unit, out = build_library(arch, newlib, compression)
    adapters = [ROOT / f"sdk/compiler/{name}.c" for name in
                "start entry isolated platform files heap time serial_sync serial_tls pthread namespace process newlib".split()]
    adapters += [ROOT / "sdk/python/posix.c", ROOT / "tools/native-python/tcl-acceptance.c"]
    arithmetic, _ = arithmetic_sources()
    for source in adapters + compression_units + arithmetic:
        result = compile_unit(source)
        if isinstance(result, str):
            print(result); raise SystemExit("Native Tcl adapter compilation failed")
        objects.append(result)
    command = ["/opt/homebrew/opt/lld/bin/ld.lld", "-static", "--error-limit=0", "--image-base=0x100000000",
               "-z", "max-page-size=4096", "-e", "infinity_compiler_entry", *objects,
               "--start-group", newlib / "libc.a", newlib / "libm.a", "--end-group", "-o", out / "tcl.elf"]
    result = subprocess.run([str(item) for item in command], cwd=ROOT,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (out / "link.log").write_bytes(result.stdout)
    if result.returncode:
        print(result.stdout.decode(errors="replace")[:16000]); raise SystemExit("Native Tcl link failed")
    print(out / "tcl.elf")


if __name__ == "__main__":
    main()
