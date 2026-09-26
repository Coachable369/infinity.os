"""Expose unresolved native engine dependencies; this is not a runnable browser."""
import os
from pathlib import Path
import subprocess
import json
import argparse

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
    arch = parser.parse_args().arch
    triple = "aarch64-unknown-none-softfloat" if arch == "aarch64" else "x86_64-unknown-none"
    target = root / "build/cargo" / triple / "debug"
    output = root / "build/servo-platform-probe"
    codegen = json.loads((output / ("servo-" + arch + "-codegen.json")).read_text())
    if codegen["compiler_exit_status"] != 0 or codegen["target"] != triple:
        raise SystemExit("Successful matching native code generation required")
    native_search = ["-L", "native=" + str(root / ("build/voice-kokoro/cxx-" + arch + "/lib"))]
    for entry in codegen["native_search_paths"]:
        path = Path(entry.removeprefix("native=")).resolve()
        # Cargo reports host build-script paths too; never link a host archive.
        if path.is_relative_to(target.resolve()) and path.is_dir():
            native_search += ["-L", "native=" + str(path)]
    flags = ["--cfg", "infinity_native", "--check-cfg=cfg(infinity_native)",
             "--check-cfg=cfg(infinity_certificate_test)"]
    native_externs = []
    for name in ("core", "panic_abort", "compiler_builtins"):
        archives = []
        for fingerprint in (target / ".fingerprint").glob(name + "-*/lib-" + name + ".json"):
            archive = target / "deps" / ("lib" + fingerprint.parent.name + ".rlib")
            if json.loads(fingerprint.read_text())["rustflags"] == flags and archive.exists():
                archives.append(archive)
        if len(archives) != 1:
            raise SystemExit("Expected one native code-generated " + name + " archive")
        native_externs += ["--extern", name + "=" + str(archives[0])]
    runtime = output / ("libnative_runtime_" + arch + ".rlib")
    newlib = root / ("build/voice-newlib-" + arch) / (arch + "-none-elf/newlib")
    c_objects = []
    for name in ("c-reentrant-allocator", "c-thread-time", "c-sync", "c-thread"):
        obj = output / (name + "-" + arch + ".o")
        subprocess.run(["/opt/homebrew/opt/llvm/bin/clang", "--target=" + arch + "-none-elf",
                    "-mstrict-align" if arch == "aarch64" else "-mno-red-zone",
                    "-ffreestanding", "-fno-builtin", "-isystem", str(newlib / "targ-include"),
                    "-isystem", str(root / "build/newlib-4.6.0.20260123/newlib/libc/include"),
                    "-include", str(root / "sdk/servo-std/c-target.h"),
                    "-c", str(root / "sdk/servo-std" / (name + ".c")), "-o", str(obj)], check=True)
        c_objects += ["-C", "link-arg=" + str(obj)]
    subprocess.run(["rustc", "--edition=2021", "--target", triple,
                    "--crate-name", "infinity_servo_runtime_primitives", "--crate-type", "rlib",
                    "--cfg", 'feature="native-abi"', "--cfg", 'feature="c-allocator-abi"', "-C", "panic=abort",
                    *native_externs, "-L", "dependency=" + str(target / "deps"),
                    str(root / "sdk/servo-runtime-primitives/lib.rs"), "-o", str(runtime)], check=True)
    command = ["rustc", "--edition=2021", "--target", triple,
               "--cfg", "infinity_native", "-C", "panic=abort",
               "-C", "linker=/opt/homebrew/opt/lld/bin/ld.lld",
               "-C", "link-arg=--entry=infinity_browser_link_probe",
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
               str(Path(__file__).with_name("engine-link.rs")),
               "-o", str(output / "engine-link-only.elf")]
    with (output / "engine-link.log").open("w") as log:
        result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
    report = {"link_exit_status": result.returncode, "executed": False,
              "purpose": "link diagnostics only; native providers are not initialized"}
    (output / "engine-link.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report))
    return result.returncode

if __name__ == "__main__":
    raise SystemExit(main())
