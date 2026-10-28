#!/usr/bin/env python3
"""Behavioral native-loader checks; never claims guest CPython acceptance."""
from pathlib import Path
import os
import subprocess
from tcl_sources import SOURCES

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
    errno_test = output / "network-errno"
    run("cc", "-std=c11", ROOT / "tools/native-python/network-errno-test.c", "-o", errno_test)
    run(errno_test)
    tk_events = output / "tk-events"
    run("cc", "-std=c11", "-I", ROOT / "third_party/python/tcl-tk" / SOURCES["tk"][2] / "xlib",
        ROOT / "tools/native-python/tk-events-test.c", "-o", tk_events)
    run(tk_events)
    sqlite_test = output / "sqlite-path"
    sqlite_source = ROOT / "third_party/python/tcl-tk/tcl8.6.18/pkgs/sqlite3.53.0/compat/sqlite3"
    run("cc", "-std=c11", "-DSQLITE_OS_OTHER=1", "-DSQLITE_THREADSAFE=0",
        "-Dsqlite3_os_init=infinity_sqlite_os_init", "-Dsqlite3_os_end=infinity_sqlite_os_end",
        "-I", sqlite_source, "-I", ROOT / "sdk/compiler/include",
        ROOT / "tools/native-python/sqlite-path-test.c", sqlite_source / "sqlite3.c",
        "-o", sqlite_test)
    run(sqlite_test)
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
    for test, args in (("arguments", []), ("scope", []), ("library-catalog", []), ("extension-mapping", libraries), ("extension-image", extensions)):
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
    package_extensions = output / "package-extensions"
    run("rustc", "--edition=2021", "-A", "warnings", "-O", ROOT / "tools/native-python/package-extension-test.rs", "-o", package_extensions)
    package_inputs = [ROOT / "builds/toolchains" / arch / name
                      for arch in ("aarch64", "x86_64") for name in ("python.elf", "infinity_native_test.so")]
    run(package_extensions, *package_inputs)
    run("python3", ROOT / "tools/native-python/package-tests.py")

if __name__ == "__main__":
    main()
