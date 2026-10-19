#!/usr/bin/env python3
"""Behavioral native-loader checks; never claims guest CPython acceptance."""
from pathlib import Path
import os
import subprocess

ROOT = Path(__file__).resolve().parents[2]

# ------------------------=
# FUNC: run
# DESC: Runs checked compiler and executable fixtures inside the active build-kit lease.
# ------------------=
def run(*arguments):
    subprocess.run([str(value) for value in arguments], cwd=ROOT, check=True)

# ------------------------=
# FUNC: main
# DESC: Cross-compiles real shared libraries and exercises both page-table implementations and owned argv.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/native-python/verify-loader.py")
    output = ROOT / "build/native-python/loader-tests"
    output.mkdir(parents=True, exist_ok=True)
    libraries = []
    for arch in ("aarch64", "x86_64"):
        obj = output / f"{arch}.o"
        library = output / f"{arch}.so"
        run("/opt/homebrew/opt/llvm/bin/clang", f"--target={arch}-none-elf", "-fPIC", "-O0",
            "-c", ROOT / "tools/native-python/library.c", "-o", obj)
        run("/opt/homebrew/opt/lld/bin/ld.lld", "-shared", "--hash-style=sysv", "-z", "max-page-size=4096",
            obj, "-o", library)
        libraries.append(library)
    extensions = [ROOT / "builds/toolchains" / arch / "infinity_native_test.so" for arch in ("aarch64", "x86_64")]
    for test, args in (("arguments", []), ("scope", []), ("extension-mapping", libraries), ("extension-image", extensions)):
        executable = output / test
        run("rustc", "--edition=2021", ROOT / f"tools/native-python/{test}-test.rs", "-o", executable)
        run(executable, *args)
    storage = output / "storage"
    run("rustc", "--edition=2021", "-A", "warnings", "-O", ROOT / "tools/native-python/storage-test.rs", "-o", storage)
    for arch in ("aarch64", "x86_64"):
        package = ROOT / "builds/toolchains" / arch
        run(storage, package / "infinity_native_test.so", package / "python-license.txt")
    packages = output / "package-storage"
    run("rustc", "--edition=2021", "-A", "warnings", "-O", ROOT / "tools/native-python/package-storage-test.rs", "-o", packages)
    run(packages)
    run("python3", ROOT / "tools/native-python/package-tests.py")

if __name__ == "__main__":
    main()
