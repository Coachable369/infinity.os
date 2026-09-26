"""Check the pinned engine; keep source/cache/logs inside the repository."""
import json
import argparse
import os
from pathlib import Path
import subprocess
import shlex


# ------------------------=
# FUNC: main
# DESC: Records an actual minimal-feature Servo compiler attempt without runtime claims.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run via ./build-kit run python3 tools/servo-platform-probe/check-servo.py")
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", choices=("servo", "fontsan", "aws-lc-sys"), default="servo")
    parser.add_argument("--arch", choices=("aarch64", "x86_64"), default="aarch64")
    options = parser.parse_args()
    arch = options.arch
    target = "aarch64-unknown-none-softfloat" if arch == "aarch64" else "x86_64-unknown-none"
    target_key = target.replace("-", "_")
    source = root / "build/servo-port-audit"
    revision = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
    if revision != "d05154e2b4def11a9fefe412898a0a6c8925a9cd":
        raise SystemExit("Unreviewed Servo revision")
    output = root / "build/servo-platform-probe"
    output.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    environment.update({"RUSTC_BOOTSTRAP": "1", "CARGO_HOME": str(root / "build/servo-cargo-home"),
                        "__CARGO_TESTS_ONLY_SRC_ROOT": str(root / "build/servo-rust-src/library"),
                        "RUSTFLAGS": "--cfg infinity_native"})
    # Use actual freestanding target headers already built by the native toolchain,
    # never macOS headers. This compiler check does not link the serial voice libc.
    includes = [root / f"build/voice-newlib-{arch}/{arch}-none-elf/newlib/targ-include",
                root / "build/newlib-4.6.0.20260123/newlib/libc/include"]
    if not all(path.is_dir() for path in includes):
        raise SystemExit("Native target C headers unavailable; prepare the native toolchain through build-kit")
    environment[f"CC_{target_key}"] = "/opt/homebrew/opt/llvm/bin/clang"
    cflags = (
        f"--target={arch}-none-elf -ffreestanding " +
        ("-mstrict-align " if arch == "aarch64" else "-mno-red-zone ") +
        "-I" + str(root / "sdk/servo-std/include") + " " +
        "-include " + str(root / "sdk/servo-std/c-target.h") + " " +
        " ".join("-isystem " + str(path) for path in includes))
    environment[f"CFLAGS_{target_key}"] = cflags
    cxx_headers = root / f"build/voice-kokoro/cxx-{arch}/include/c++/v1"
    if not (cxx_headers / "__config_site").is_file():
        raise SystemExit("Native libc++ headers unavailable; prepare native toolchain through build-kit")
    environment[f"CXX_{target_key}"] = "/opt/homebrew/opt/llvm/bin/clang++"
    environment[f"CXXFLAGS_{target_key}"] = (
        "-nostdinc++ -isystem " + str(cxx_headers) + " " + cflags)
    subprocess.run([environment[f"CC_{target_key}"],
                    *shlex.split(cflags), "-std=c11", "-fsyntax-only",
                    str(Path(__file__).with_name("c-abi-probe.c"))], check=True, env=environment)
    command = ["cargo", "check", "-j", "4", "-Z", "build-std=std,panic_abort",
               "--target", target, "--manifest-path",
               str(source / "components/servo/Cargo.toml"), "--locked"]
    native_libc = root / "build/servo-native-deps/libc-0.2.189"
    if native_libc.is_dir():
        command += ["--config", 'patch.crates-io.libc.path="' + str(native_libc) + '"']
    if options.package == "servo":
        command += ["--no-default-features", "--features", "bundled"]
    else:
        command += ["-p", options.package]
    prefix = options.package + "-" + arch + "-check"
    with (output / (prefix + ".log")).open("w") as log:
        result = subprocess.run(command, cwd=root, env=environment, stdout=log,
                                stderr=subprocess.STDOUT, check=False)
    report = {"servo_revision": revision, "package": options.package, "target": target, "command": command,
              "compiler_exit_status": result.returncode, "executed": False}
    (output / (prefix + ".json")).write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
