"""Expose unresolved native engine dependencies; this is not a runnable browser."""
import os
from pathlib import Path
import subprocess
import json
import argparse
import tempfile

# ------------------------=
# FUNC: main
# DESC: Links actual Servo initialization without supplying fake native service definitions.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arch", choices=("aarch64", "x86_64"), default="aarch64")
    parser.add_argument("--boot-probe", action="store_true")
    parser.add_argument("--swgl-probe", action="store_true")
    parser.add_argument("--page-probe", action="store_true")
    parser.add_argument("--network-probe", action="store_true")
    parser.add_argument("--component", action="store_true")
    parser.add_argument("--component-trace", action="store_true")
    options = parser.parse_args()
    arch = options.arch
    if options.boot_probe and arch != "aarch64":
        raise SystemExit("Engine boot fixture currently supports AArch64 only")
    triple = "aarch64-unknown-none" if arch == "aarch64" else "x86_64-unknown-none"
    target = root / "build/cargo" / triple / "debug"
    output = root / "build/servo-platform-probe"
    codegen = json.loads((output / ("servo-" + arch + "-codegen.json")).read_text())
    if codegen["compiler_exit_status"] != 0 or codegen["target"] != triple:
        raise SystemExit("Successful matching native code generation required")
    native_search = ["-L", "native=" + str(root / ("build/voice-kokoro/cxx-" + arch + "/lib"))]
    if options.swgl_probe:
        if not options.boot_probe:
            raise SystemExit("SWGL execution requires the native boot fixture")
        raster = json.loads((output / ("swgl-" + arch + "-codegen.json")).read_text())
        if raster["compiler_exit_status"] != 0:
            raise SystemExit("Successful native SWGL code generation required")
        codegen["native_search_paths"] += raster["native_search_paths"]
    for entry in codegen["native_search_paths"]:
        path = Path(entry.removeprefix("native=")).resolve()
        # Cargo reports host build-script paths too; never link a host archive.
        if path.is_relative_to(target.resolve()) and path.is_dir():
            native_search += ["-L", "native=" + str(path)]
    flags = ["--cfg", "infinity_native", "--check-cfg=cfg(infinity_native)",
             "--check-cfg=cfg(infinity_certificate_test)"]
    native_externs = []
    for name in ("core", "panic_abort", "compiler_builtins", *(("std",) if options.boot_probe or options.component else ())):
        recorded = codegen.get("native_archives", {}).get(name)
        if recorded and Path(recorded).is_file():
            native_externs += ["--extern", name + "=" + recorded]
            continue
        archives = []
        for fingerprint in (target / ".fingerprint").glob(name + "-*/lib-" + name + ".json"):
            archive = target / "deps" / ("lib" + fingerprint.parent.name + ".rlib")
            if json.loads(fingerprint.read_text())["rustflags"] == flags and archive.exists():
                archives.append(archive)
        if len(archives) != 1:
            raise SystemExit("Expected one native code-generated " + name + " archive")
        native_externs += ["--extern", name + "=" + str(archives[0])]
    if options.page_probe or options.component:
        archives = list((target / "deps").glob("libhttp-*.rlib"))
        if len(archives) != 1:
            raise SystemExit("Expected one native HTTP type archive")
        native_externs += ["--extern", "http=" + str(archives[0])]
    if options.network_probe:
        network = json.loads((output / "network.json").read_text())
        native_externs += ["--extern", "infinity_browser_native_network=" + network["archive"]]
    runtime = output / ("libnative_runtime_" + arch + ".rlib")
    newlib = root / ("build/voice-newlib-" + arch) / (arch + "-none-elf/newlib")
    c_objects = []
    sqlite = root / "build/servo-cargo-home/registry/src/index.crates.io-1949cf8c6b5b557f/libsqlite3-sys-0.36.0/sqlite3"
    for name in ("c-reentrant-allocator", "c-thread-time", "c-sync", "c-thread", "c-memory", "c-system", "c-sqlite"):
        obj = output / (name + "-" + arch + ".o")
        subprocess.run(["/opt/homebrew/opt/llvm/bin/clang", "--target=" + arch + "-none-elf",
                    "-mstrict-align" if arch == "aarch64" else "-mno-red-zone",
                    "-ffreestanding", "-fno-builtin", "-I", str(sqlite), "-isystem", str(newlib / "targ-include"),
                    "-isystem", str(root / "build/newlib-4.6.0.20260123/newlib/libc/include"),
                    "-include", str(root / "sdk/servo-std/c-target.h"),
                    "-c", str(root / "sdk/servo-std" / (name + ".c")), "-o", str(obj)], check=True)
        c_objects += ["-C", "link-arg=" + str(obj)]
    if options.boot_probe:
        fatal = output / "guest-fatal.o"
        subprocess.run(["/opt/homebrew/opt/llvm/bin/clang", "--target=" + arch + "-none-elf",
                        "-isystem", str(newlib / "targ-include"),
                        "-isystem", str(root / "build/newlib-4.6.0.20260123/newlib/libc/include"),
                        "-ffreestanding", "-c", str(Path(__file__).with_name("guest") / "fatal.c"),
                        "-o", str(fatal)], check=True)
        c_objects += ["-C", "link-arg=" + str(fatal), "-C", "link-arg=--wrap=abort",
                      "-C", "link-arg=--wrap=__assert_func", "-C", "link-arg=--wrap=fprintf",
                      "-C", "link-arg=--wrap=fputs", "-C", "link-arg=--wrap=fwrite"]
    subprocess.run(["rustc", "--edition=2021", "--target", triple,
                    "--crate-name", "infinity_servo_runtime_primitives", "--crate-type", "rlib",
                    "--cfg", 'feature="native-abi"', "--cfg", 'feature="c-allocator-abi"', "-C", "panic=abort",
                    *(["--cfg", "infinity_component_trace"] if options.component_trace else []),
                    *native_externs, "-L", "dependency=" + str(target / "deps"),
                    str(root / "sdk/servo-runtime-primitives/lib.rs"), "-o", str(runtime)], check=True)
    if options.component:
        if options.component_trace:
            archives = list((target / "deps").glob("liblog-*.rlib"))
            if len(archives) != 1:
                raise SystemExit("Expected one native log archive")
            native_externs += ["--extern", "log=" + str(archives[0]), "--cfg", "infinity_component_trace"]
            base = list((target / "deps").glob("libservo_base-*.rlib"))
            if len(base) != 1:
                raise SystemExit("Expected one native Servo base archive")
            native_externs += ["--extern", "servo_base=" + str(base[0])]
        archive = output / ("browser-component-" + arch + ".a")
        component = root / "sdk/infinity-browser-servo"
        command = ["rustc", "--edition=2021", "--target", triple, "--crate-type", "staticlib",
                   "--cfg", "infinity_native", "-C", "panic=abort", "-l", "static=c++abi",
                   *native_search, *native_externs,
                   "--extern", "servo=" + str(target / "libservo.rlib"),
                   "--extern", "infinity_servo_runtime_primitives=" + str(runtime),
                   "-L", "dependency=" + str(target / "deps"),
                   "-L", "dependency=" + str(root / "build/cargo/debug/deps"),
                   str(component / "component.rs"), "-o", str(archive)]
        with (output / "component-link.log").open("w") as log:
            result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
            if result.returncode:
                return result.returncode
            # rustc's executable link retains Rust crate objects (and their
            # inventory constructors), but lazily selects bundled C archives.
            # Reproduce that distinction: whole-archiving all bundled C pulls
            # duplicate implementations such as SpiderMonkey/fontsan LZ4.
            ar = "/opt/homebrew/opt/llvm/bin/llvm-ar"
            members = subprocess.check_output([ar, "t", str(archive)], text=True).splitlines()
            rust_members = [member for member in members if member.endswith(".rcgu.o")]
            if not rust_members or len(rust_members) != len(set(rust_members)):
                raise SystemExit("Ambiguous native Rust archive members")
            rust_archive = output / ("browser-rust-" + arch + ".a")
            rust_archive.unlink(missing_ok=True)
            with tempfile.TemporaryDirectory(prefix="browser-rust-", dir=output) as directory:
                subprocess.run([ar, "x", str(archive), *rust_members], cwd=directory, check=True)
                subprocess.run([ar, "rc", str(rust_archive), *rust_members], cwd=directory, check=True)
            native = output / ("browser-native-" + arch + ".o")
            objects = [argument.removeprefix("link-arg=") for argument in c_objects if argument.startswith("link-arg=")]
            result = subprocess.run(["/opt/homebrew/opt/lld/bin/ld.lld", "-r", "--gc-sections", "--strip-debug",
                    "--undefined=infinity_browser_run", "--wrap=abort", "--defsym=__wrap_abort=infinity_browser_abort",
                    "-T", str(component / "private.ld"), "-o", str(native), *objects,
                    "--start-group", "--whole-archive", str(rust_archive), "--no-whole-archive", str(archive),
                    str(root / ("build/voice-kokoro/cxx-" + arch + "/lib/libc++.a")),
                    str(root / ("build/voice-kokoro/cxx-" + arch + "/lib/libc++abi.a")),
                    str(newlib / "libc.a"), str(newlib / "libm.a"),
                    "--end-group"], stdout=log, stderr=subprocess.STDOUT)
            if result.returncode:
                return result.returncode
            private = output / ("browser-private-" + arch + ".o")
            subprocess.run(["/opt/homebrew/opt/llvm/bin/llvm-objcopy",
                "--prefix-symbols=infinity_browser_private_", str(native), str(private)], check=True)
        undefined = subprocess.check_output(["/opt/homebrew/opt/llvm/bin/llvm-nm", "--undefined-only", str(private)], text=True)
        report = {"target": triple, "object": str(private), "undefined": undefined, "executed": False}
        (output / ("component-" + arch + ".json")).write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(report))
        return 1 if any(line.split()[0] == "U" for line in undefined.splitlines() if line.split()) else 0
    command = ["rustc", "--edition=2021", "--target", triple,
               "--cfg", "infinity_native", "-C", "panic=abort",
               *(["--cfg", "infinity_page_probe"] if options.page_probe else []),
               *(["--cfg", "infinity_network_probe"] if options.network_probe else []),
               *(["--cfg", "infinity_swgl_probe", "--extern",
                  "infinity_swgl_probe=" + str(target / "libinfinity_swgl_probe.rlib")]
                 if options.swgl_probe else []),
               "-C", "linker=/opt/homebrew/opt/lld/bin/ld.lld",
               "-C", "link-arg=--entry=" + ("_start" if options.boot_probe else "infinity_browser_link_probe"),
               *(["-C", "link-arg=-T" + str(Path(__file__).with_name("engine-boot.ld"))] if options.boot_probe else []),
               "-C", "link-arg=--error-limit=0",
               *native_search,
               "-l", "static=c++abi",
               *c_objects,
               # Provider objects and engine archives must precede fallback libc.
               "-C", "link-arg=" + str(newlib / "libc.a"),
               "-C", "link-arg=" + str(newlib / "libm.a"),
               "--extern", "servo=" + str(target / "libservo.rlib"),
               "--extern", "infinity_servo_runtime_primitives=" + str(runtime),
               *native_externs,
               "-L", "dependency=" + str(target / "deps"),
               "-L", "dependency=" + str(root / "build/cargo/debug/deps"),
               str(Path(__file__).with_name("engine-boot.rs" if options.boot_probe else "engine-link.rs")),
               "-o", str(output / ("engine-boot.elf" if options.boot_probe else "engine-link-only.elf"))]
    with (output / "engine-link.log").open("w") as log:
        result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
    report = {"link_exit_status": result.returncode, "executed": False,
              "purpose": "native boot fixture; execution is a separate gate" if options.boot_probe else
                         "link diagnostics only; native providers are not initialized"}
    (output / "engine-link.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report))
    return result.returncode

if __name__ == "__main__":
    raise SystemExit(main())
