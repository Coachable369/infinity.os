"""Small pinned native dependencies for genuine CPython library backends."""
from pathlib import Path
import hashlib
import os
import shlex
import subprocess
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
BZ2_VERSION = "1.0.8"
BZ2_SHA256 = "ab5a03176ee106d3f0fa90e381da478ddae405918153cca248e682cd0c4a2269"
TZDATA_VERSION = "2026.5"
TZDATA_SHA256 = "b683bd1b6659ddcd810ff02ad09ba821d4bf1065072805063eb35c49617905ac"
TZDATA_URL = "https://files.pythonhosted.org/packages/94/21/1e5995a1c920cce14e4bffae20c665ec10e7ed03ab25e006cd741092b718/tzdata-2026.5-py2.py3-none-any.whl"
XZ_VERSION = "5.8.3"
XZ_SHA256 = "3d3a1b973af218114f4f889bbaa2f4c037deaae0c8e815eec381c3d546b974a0"


# ------------------------=
# FUNC: lzma_library
# DESC: Builds the pinned genuine liblzma for the native target without host threads, command-line tools or external dependencies.
# ------------------=
def lzma_library(arch, llvm, newlib):
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise RuntimeError("Native dependencies must be prepared through build-kit")
    parent = ROOT / "third_party/python"
    parent.mkdir(parents=True, exist_ok=True)
    archive = parent / f"xz-{XZ_VERSION}.tar.gz"
    if not archive.exists():
        url = f"https://github.com/tukaani-project/xz/releases/download/v{XZ_VERSION}/{archive.name}"
        with urllib.request.urlopen(url, timeout=60) as response:
            body = response.read(4 * 1024 * 1024 + 1)
        if len(body) > 4 * 1024 * 1024 or hashlib.sha256(body).hexdigest() != XZ_SHA256:
            raise RuntimeError("Official XZ source checksum mismatch")
        temporary = archive.with_suffix(archive.suffix + ".partial")
        temporary.write_bytes(body)
        temporary.replace(archive)
    if hashlib.sha256(archive.read_bytes()).hexdigest() != XZ_SHA256:
        raise RuntimeError("Cached XZ source checksum mismatch")
    source = parent / f"xz-{XZ_VERSION}"
    if not source.exists():
        with tarfile.open(archive) as bundle:
            bundle.extractall(parent, filter="data")
    output = ROOT / "build/native-python" / arch / "liblzma"
    output.mkdir(parents=True, exist_ok=True)
    flags = ["-O2", "-fPIE", "-ffunction-sections", "-fdata-sections",
             "-include", str(ROOT / "sdk/compiler/target.h"),
             "-I" + str(newlib / "targ-include"), "-I" + str(newlib),
             "-I" + str(ROOT / "build/newlib-4.6.0.20260123/newlib/libc/include")]
    flags += ["-mno-red-zone"] if arch == "x86_64" else ["-mno-outline-atomics", "-mstrict-align"]
    command = ["cmake", "-S", str(source), "-B", str(output), "-G", "Ninja",
        "-DCMAKE_SYSTEM_NAME=Generic", f"-DCMAKE_SYSTEM_PROCESSOR={arch}",
        f"-DCMAKE_C_COMPILER={llvm / 'clang'}", f"-DCMAKE_C_COMPILER_TARGET={arch}-none-elf",
        f"-DCMAKE_AR={llvm / 'llvm-ar'}", f"-DCMAKE_RANLIB={llvm / 'llvm-ranlib'}",
        "-DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY", "-DCMAKE_BUILD_TYPE=Release",
        "-DCMAKE_C_FLAGS=" + shlex.join(flags), "-DBUILD_SHARED_LIBS=OFF", "-DBUILD_TESTING=OFF",
        "-DXZ_THREADS=no", "-DXZ_NLS=OFF", "-DXZ_TOOL_XZ=OFF", "-DXZ_TOOL_XZDEC=OFF",
        "-DXZ_TOOL_LZMADEC=OFF", "-DXZ_TOOL_LZMAINFO=OFF", "-DXZ_TOOL_SCRIPTS=OFF",
        "-DXZ_CLMUL_CRC=OFF", "-DXZ_ARM64_CRC32=OFF"]
    with (output / "configure.log").open("wb") as log:
        subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
    with (output / "build.log").open("wb") as log:
        subprocess.run(["cmake", "--build", str(output), "--target", "liblzma", "-j4"],
                       cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
    return source / "src/liblzma/api", output / "liblzma.a", source / "COPYING"


# ------------------------=
# FUNC: timezone_wheel
# DESC: Retrieves the checksum-pinned PSF timezone database for offline standard ZoneInfo lookups on a fresh installation.
# ------------------=
def timezone_wheel():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise RuntimeError("Native dependencies must be prepared through build-kit")
    parent = ROOT / "third_party/python"
    parent.mkdir(parents=True, exist_ok=True)
    archive = parent / f"tzdata-{TZDATA_VERSION}-py2.py3-none-any.whl"
    if not archive.exists():
        with urllib.request.urlopen(TZDATA_URL, timeout=60) as response:
            body = response.read(1024 * 1024 + 1)
        if len(body) > 1024 * 1024 or hashlib.sha256(body).hexdigest() != TZDATA_SHA256:
            raise RuntimeError("PSF timezone database checksum mismatch")
        temporary = archive.with_suffix(archive.suffix + ".partial")
        temporary.write_bytes(body)
        temporary.replace(archive)
    if hashlib.sha256(archive.read_bytes()).hexdigest() != TZDATA_SHA256:
        raise RuntimeError("Cached timezone database checksum mismatch")
    return archive


# ------------------------=
# FUNC: bzip2_sources
# DESC: Retrieves and verifies the official small bzip2 library inside the repository without compiling a host replacement.
# ------------------=
def bzip2_sources():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise RuntimeError("Native dependencies must be prepared through build-kit")
    parent = ROOT / "third_party/python"
    parent.mkdir(parents=True, exist_ok=True)
    archive = parent / f"bzip2-{BZ2_VERSION}.tar.gz"
    if not archive.exists():
        temporary = archive.with_suffix(archive.suffix + ".partial")
        with urllib.request.urlopen(f"https://sourceware.org/pub/bzip2/{archive.name}", timeout=60) as response:
            body = response.read(2 * 1024 * 1024 + 1)
        if len(body) > 2 * 1024 * 1024 or hashlib.sha256(body).hexdigest() != BZ2_SHA256:
            raise RuntimeError("Official bzip2 source checksum mismatch")
        temporary.write_bytes(body)
        temporary.replace(archive)
    if hashlib.sha256(archive.read_bytes()).hexdigest() != BZ2_SHA256:
        raise RuntimeError("Cached bzip2 source checksum mismatch")
    source = parent / f"bzip2-{BZ2_VERSION}"
    if not source.exists():
        with tarfile.open(archive) as bundle:
            bundle.extractall(parent, filter="data")
    units = "blocksort huffman crctable randtable compress decompress bzlib".split()
    return source, [source / (name + ".c") for name in units]
